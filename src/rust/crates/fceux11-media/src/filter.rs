use crate::fcoeffs::*;
use std::slice;

// hotfix1 P1-6 (C-04): the previous
// `#[repr(C)] pub struct FceuFilterState { _private: [u8; 0] }`
// generated a non-standard zero-length-array layout in C and made the
// type aggregate-init-able in C++. We replace it with a non-empty Rust
// struct holding an opaque `u8` placeholder; cbindgen emits a valid
// `typedef struct FceuFilterState { uint8_t _handle; } FceuFilterState;`
// which is C99-portable. C consumers only ever pass `FceuFilterState*`
// pointers (the runtime object behind the handle is a `Box<FilterState>`
// allocated by `fceux11_rust_filter_state_create`), so the placeholder
// field is read by no one — its only purpose is to give the struct a
// non-zero layout that survives a strict C compiler.
//
// The size is 1 byte at the Rust level but the FFI only passes pointers,
// so the on-stack cost on the C side is zero.
#[repr(C)]
pub struct FceuFilterState {
    _handle: u8,
}

/// Internal Rust state for audio filter.
/// Replaces the C++ `static` variables: `sq2coeffs`, `coeffs`, `mrindex`,
/// `mrratio`, and the `static` accumulators inside `SexyFilter` / `SexyFilter2`.
struct FilterState {
    sq2coeffs: [i32; SQ2NCOEFFS],
    coeffs: [i32; NCOEFFS],
    mrindex: u32,
    mrratio: u32,
    sexy_acc1: i64,
    sexy_acc2: i64,
    sexy2_acc: i64,
}

impl Default for FilterState {
    fn default() -> Self {
        Self {
            sq2coeffs: [0; SQ2NCOEFFS],
            coeffs: [0; NCOEFFS],
            mrindex: 0,
            mrratio: 0,
            sexy_acc1: 0,
            sexy_acc2: 0,
            sexy2_acc: 0,
        }
    }
}

/// C ABI: Create a new filter state and return an opaque handle.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_filter_state_create() -> *mut FceuFilterState {
    let state = Box::new(FilterState::default());
    Box::into_raw(state) as *mut FceuFilterState
}

/// C ABI: Destroy a filter state handle.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_filter_state_destroy(handle: *mut FceuFilterState) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle.cast::<FilterState>());
        }
    }
}

/// C ABI: Initialise FIR coefficients and resampling state.
///
/// `ntsc_cpu` and `pal_cpu` are the base CPU clock frequencies in Hz
/// (e.g. 1789772.727… for NTSC, 1662607.125 for PAL).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_filter_make(
    handle: *mut FceuFilterState,
    rate: i32,
    soundq: i32,
    is_pal: i32,
    ntsc_cpu: f64,
    pal_cpu: f64,
) {
    if handle.is_null() {
        return;
    }
    let state = unsafe { &mut *(handle.cast::<FilterState>()) };

    let nco = if soundq == 2 { SQ2NCOEFFS } else { NCOEFFS };

    state.mrindex = ((nco + 1) << 16) as u32;
    let cpu_hz = if is_pal != 0 { pal_cpu } else { ntsc_cpu };
    state.mrratio = ((cpu_hz * 65536.0) / rate as f64) as u32;

    let tmp: &[i32] = if soundq == 2 {
        let idx = (if is_pal != 0 { 1 } else { 0 })
            | (if rate == 48000 { 2 } else { 0 })
            | (if rate == 96000 { 4 } else { 0 });
        match idx {
            0 => &SQ2C44100NTSC,
            1 => &SQ2C44100PAL,
            2 => &SQ2C48000NTSC,
            3 => &SQ2C48000PAL,
            4 => &SQ2C96000NTSC,
            5 => &SQ2C96000PAL,
            _ => &SQ2C44100NTSC,
        }
    } else {
        let idx = (if is_pal != 0 { 1 } else { 0 })
            | (if rate == 48000 { 2 } else { 0 })
            | (if rate == 96000 { 4 } else { 0 });
        match idx {
            0 => &C44100NTSC,
            1 => &C44100PAL,
            2 => &C48000NTSC,
            3 => &C48000PAL,
            4 => &C96000NTSC,
            5 => &C96000PAL,
            _ => &C44100NTSC,
        }
    };

    let half = nco >> 1;
    if soundq == 2 {
        for (x, &v) in tmp.iter().take(half).enumerate() {
            state.sq2coeffs[x] = v;
            state.sq2coeffs[SQ2NCOEFFS - 1 - x] = v;
        }
    } else {
        for (x, &v) in tmp.iter().take(half).enumerate() {
            state.coeffs[x] = v;
            state.coeffs[NCOEFFS - 1 - x] = v;
        }
    }
}

/// Internal: `SexyFilter2` low-pass filter.
fn sexy_filter2(state: &mut FilterState, buf: &mut [i32]) {
    let mut acc = state.sexy2_acc;
    for sample in buf.iter_mut() {
        let dropcurrent = ((((*sample as u32) << 16) as i64) - acc) >> 3;
        acc += dropcurrent;
        *sample = (acc >> 16) as i32;
    }
    state.sexy2_acc = acc;
}

/// Internal: `SexyFilter` volume + simple IIR filter (in-place).
fn sexy_filter_in_place(
    state: &mut FilterState,
    buf: &mut [i32],
    snd_rate: i32,
    sound_volume: i32,
    soundq: i32,
) {
    let mul1 = ((94i64) << 16) / snd_rate as i64;
    let mul2 = ((24i64) << 16) / snd_rate as i64;
    let mut vmul = ((sound_volume as i64) << 16) * 3 / 4 / 100;

    if soundq != 0 {
        vmul /= 4;
    } else {
        vmul *= 2;
    }

    let mut acc1 = state.sexy_acc1;
    let mut acc2 = state.sexy_acc2;

    for sample in buf.iter_mut() {
        let ino64 = (*sample as i64) * vmul;
        acc1 += ((ino64 - acc1) * mul1) >> 16;
        acc2 += ((ino64 - acc1 - acc2) * mul2) >> 16;
        let mut t = ((acc1 - ino64 + acc2) >> 16) as i32;
        t = t.clamp(-32768, 32767);
        *sample = t;
    }

    state.sexy_acc1 = acc1;
    state.sexy_acc2 = acc2;
}

/// Internal: `SexyFilter` volume + simple IIR filter (out-of-place).
/// The input buffer is zeroed after reading, matching the original C++ side-effect.
fn sexy_filter_out_of_place(
    state: &mut FilterState,
    in_buf: &mut [i32],
    out_buf: &mut [i32],
    snd_rate: i32,
    sound_volume: i32,
    soundq: i32,
) {
    let mul1 = ((94i64) << 16) / snd_rate as i64;
    let mul2 = ((24i64) << 16) / snd_rate as i64;
    let mut vmul = ((sound_volume as i64) << 16) * 3 / 4 / 100;

    if soundq != 0 {
        vmul /= 4;
    } else {
        vmul *= 2;
    }

    let mut acc1 = state.sexy_acc1;
    let mut acc2 = state.sexy_acc2;

    for (in_sample, out_sample) in in_buf.iter_mut().zip(out_buf.iter_mut()) {
        let ino64 = (*in_sample as i64) * vmul;
        acc1 += ((ino64 - acc1) * mul1) >> 16;
        acc2 += ((ino64 - acc1 - acc2) * mul2) >> 16;
        let mut t = ((acc1 - ino64 + acc2) >> 16) as i32;
        t = t.clamp(-32768, 32767);
        *out_sample = t;
        *in_sample = 0;
    }

    state.sexy_acc1 = acc1;
    state.sexy_acc2 = acc2;
}

/// C ABI: `SexyFilter` wrapper.
///
/// `in_buf` and `out_buf` may point to the same memory.
///
/// # Safety
/// `handle` must be a valid filter state handle.
/// `in_buf` and `out_buf` must each point to at least `count` valid `i32`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_filter_sexy(
    handle: *mut FceuFilterState,
    in_buf: *mut i32,
    out_buf: *mut i32,
    count: i32,
    snd_rate: i32,
    sound_volume: i32,
    soundq: i32,
) {
    if handle.is_null() || in_buf.is_null() || out_buf.is_null() || count <= 0 {
        return;
    }
    let state = unsafe { &mut *(handle.cast::<FilterState>()) };
    let in_slice = unsafe { slice::from_raw_parts_mut(in_buf, count as usize) };
    let out_slice = unsafe { slice::from_raw_parts_mut(out_buf, count as usize) };
    if in_buf == out_buf {
        sexy_filter_in_place(state, in_slice, snd_rate, sound_volume, soundq);
    } else {
        sexy_filter_out_of_place(state, in_slice, out_slice, snd_rate, sound_volume, soundq);
    }
}

/// C ABI: `SexyFilter2` wrapper.
///
/// # Safety
/// `handle` must be a valid filter state handle.
/// `buf` must point to at least `count` valid `i32`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_filter_sexy2(
    handle: *mut FceuFilterState,
    buf: *mut i32,
    count: i32,
) {
    if handle.is_null() || buf.is_null() || count <= 0 {
        return;
    }
    let state = unsafe { &mut *(handle.cast::<FilterState>()) };
    let buf_slice = unsafe { slice::from_raw_parts_mut(buf, count as usize) };
    sexy_filter2(state, buf_slice);
}

/// C ABI: `NeoFilterSound` wrapper.
///
/// Returns the number of samples written to `out`.
/// `leftover` is set to the number of samples that must be copied from the
/// end of `in` to the beginning of `in` on the next call.
///
/// # Safety
/// `handle` must be a valid filter state handle.
/// `in_buf` and `out_buf` must each point to at least `inlen` valid `i32`s.
/// `leftover`, when non-null, must point to a writable `i32`.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fceux11_rust_filter_neo(
    handle: *mut FceuFilterState,
    in_buf: *mut i32,
    out_buf: *mut i32,
    inlen: u32,
    leftover: *mut i32,
    soundq: i32,
    lowpass: i32,
    neo_fill: Option<extern "C" fn(*mut i32, i32)>,
    snd_rate: i32,
    sound_volume: i32,
) -> i32 {
    if handle.is_null() || in_buf.is_null() || out_buf.is_null() || inlen == 0 {
        return 0;
    }
    let state = unsafe { &mut *(handle.cast::<FilterState>()) };
    let in_slice = unsafe { slice::from_raw_parts(in_buf, inlen as usize) };
    // out_buf must be at least as large as inlen; caller guarantees this.
    let out_slice = unsafe { slice::from_raw_parts_mut(out_buf, inlen as usize) };

    let max = (inlen - 1) << 16;
    let mut out_idx = 0usize;

    if soundq == 2 {
        let mut x = state.mrindex;
        while x < max {
            let mut acc = 0i64;
            let mut acc2 = 0i64;
            let base = (x >> 16) as usize;
            for c in 0..SQ2NCOEFFS {
                let s = in_slice[base + c - SQ2NCOEFFS];
                let coeff = state.sq2coeffs[c];
                acc += ((s as i64) * (coeff as i64)) >> 6;
                acc2 += ((in_slice[base + c + 1 - SQ2NCOEFFS] as i64) * (coeff as i64)) >> 6;
            }
            let frac = (x & 65535) as i64;
            acc = (acc * (65536 - frac) + acc2 * frac) >> (16 + 11);
            out_slice[out_idx] = acc as i32;
            out_idx += 1;
            x += state.mrratio;
        }
        state.mrindex = x - max;
        state.mrindex += (SQ2NCOEFFS * 65536) as u32;
    } else {
        let mut x = state.mrindex;
        while x < max {
            let mut acc = 0i64;
            let mut acc2 = 0i64;
            let base = (x >> 16) as usize;
            for c in 0..NCOEFFS {
                let s = in_slice[base + c - NCOEFFS];
                let coeff = state.coeffs[c];
                acc += ((s as i64) * (coeff as i64)) >> 6;
                acc2 += ((in_slice[base + c + 1 - NCOEFFS] as i64) * (coeff as i64)) >> 6;
            }
            let frac = (x & 65535) as i64;
            acc = (acc * (65536 - frac) + acc2 * frac) >> (16 + 11);
            out_slice[out_idx] = acc as i32;
            out_idx += 1;
            x += state.mrratio;
        }
        state.mrindex = x - max;
        state.mrindex += (NCOEFFS * 65536) as u32;
    }

    let left = if soundq == 2 {
        SQ2NCOEFFS + 1
    } else {
        NCOEFFS + 1
    };

    if let Some(fill) = neo_fill {
        fill(out_slice.as_mut_ptr(), out_idx as i32);
    }

    // Apply SexyFilter in-place on the output (in == out)
    let out_sub = &mut out_slice[..out_idx];
    sexy_filter_in_place(state, out_sub, snd_rate, sound_volume, soundq);

    if lowpass != 0 {
        sexy_filter2(state, out_sub);
    }

    if !leftover.is_null() {
        unsafe {
            *leftover = left as i32;
        }
    }
    out_idx as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_state_create_destroy() {
        unsafe {
            let h = fceux11_rust_filter_state_create();
            assert!(!h.is_null());
            fceux11_rust_filter_state_destroy(h);
        }
    }

    #[test]
    fn test_filter_make() {
        unsafe {
            let h = fceux11_rust_filter_state_create();
            fceux11_rust_filter_make(h, 44100, 0, 0, 1789772.727, 1662607.125);
            let state = unsafe { &*(h as *mut FilterState) };
            assert_ne!(state.mrratio, 0);
            assert_eq!(state.coeffs[0], 285);
            assert_eq!(state.coeffs[NCOEFFS - 1], 285);
            fceux11_rust_filter_state_destroy(h);
        }
    }

    #[test]
    fn test_filter_sexy() {
        unsafe {
            let h = fceux11_rust_filter_state_create();
            let mut buf = [1000i32; 4];
            let mut out = [0i32; 4];
            fceux11_rust_filter_sexy(h, buf.as_mut_ptr(), out.as_mut_ptr(), 4, 44100, 100, 0);
            // input should be zeroed because in != out
            assert_eq!(buf, [0, 0, 0, 0]);
            // output should be non-zero (clamped)
            assert!(out.iter().any(|&v| v != 0));
            fceux11_rust_filter_state_destroy(h);
        }
    }

    #[test]
    fn test_filter_sexy_same_ptr() {
        unsafe {
            let h = fceux11_rust_filter_state_create();
            let mut buf = [1000i32; 4];
            fceux11_rust_filter_sexy(h, buf.as_mut_ptr(), buf.as_mut_ptr(), 4, 44100, 100, 0);
            // input should NOT be zeroed because in == out
            assert!(buf.iter().any(|&v| v != 0));
            fceux11_rust_filter_state_destroy(h);
        }
    }

    #[test]
    fn test_filter_sexy2() {
        unsafe {
            let h = fceux11_rust_filter_state_create();
            let mut buf = [1000i32; 4];
            fceux11_rust_filter_sexy2(h, buf.as_mut_ptr(), 4);
            // output should be modified
            assert!(buf.iter().any(|&v| v != 1000));
            fceux11_rust_filter_state_destroy(h);
        }
    }

    #[test]
    fn test_filter_neo_basic() {
        unsafe {
            let h = fceux11_rust_filter_state_create();
            fceux11_rust_filter_make(h, 44100, 0, 0, 1789772.727, 1662607.125);
            let mut input = [0i32; 1024];
            input[500] = 10000;
            let mut output = [0i32; 1024];
            let mut leftover = 0i32;
            let count = fceux11_rust_filter_neo(
                h,
                input.as_mut_ptr(),
                output.as_mut_ptr(),
                1024,
                &mut leftover,
                0,
                0,
                None,
                44100,
                100,
            );
            assert!(count > 0);
            assert_eq!(leftover, (NCOEFFS + 1) as i32);
            fceux11_rust_filter_state_destroy(h);
        }
    }
}
