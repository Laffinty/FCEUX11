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
    /// A5 G2: true iff every installed coefficient satisfies |c| < 2^16,
    /// the precondition of the AVX2 bias trick (see `fir_pair_avx2`).
    /// Computed once per `fceux11_rust_filter_make`; a future coefficient
    /// table that breaks the bound silently keeps the scalar path.
    coeff_simd_safe: bool,
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
            coeff_simd_safe: true,
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
    // A5 G2: the AVX2 bias trick requires |c| < 2^16 for every tap. The
    // shipped tables' extremes are +50983 / −10839, well inside the bound;
    // verify against the actual table instead of trusting that fact.
    state.coeff_simd_safe = tmp.iter().all(|&v| (v as i64).abs() < (1i64 << 16));
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

/// A5 G1 — FIR pair for one resampler output position (pure scalar).
///
/// Computes the two adjacent FIR chains of `NeoFilterSound` at fixed-point
/// position `base`:
///
/// ```text
/// acc  = Σ_{c=0..nc-1} ( in[base-nc+c]   · coeffs[c] ) >> 6   (per tap)
/// acc2 = Σ_{c=0..nc-1} ( in[base-nc+c+1] · coeffs[c] ) >> 6   (per tap)
/// ```
///
/// The window is `nc + 1` consecutive samples starting at `base - nc`; the
/// second chain is the same window shifted by one element.
///
/// SEMANTIC LOCK: the `>> 6` truncation happens PER TAP, before the
/// additions. Σ⌊x/64⌋ ≠ ⌊Σx/64⌋ — hoisting the shift out of the loop
/// changes results and is forbidden. The G1 refactor only loads each
/// coefficient once to feed both chains; additions stay in ascending-c
/// order, so the result is bit-identical to the pre-refactor double loop.
#[inline]
fn fir_pair_scalar(in_slice: &[i32], coeffs: &[i32], base: usize) -> (i64, i64) {
    let nc = coeffs.len();
    let start = base - nc;
    let mut acc = 0i64;
    let mut acc2 = 0i64;
    for c in 0..nc {
        let coeff = coeffs[c] as i64;
        acc += ((in_slice[start + c] as i64) * coeff) >> 6;
        acc2 += ((in_slice[start + c + 1] as i64) * coeff) >> 6;
    }
    (acc, acc2)
}

/// A5 G2 — AVX2 twin of [`fir_pair_scalar`]. Bit-exact by construction.
///
/// Domain facts (established for the real `NeoFilterSound` input, see
/// `FlushEmulateSound` in sound.cpp):
///   * `s ∈ [0, ~82024)` — WaveHi holds `(b & 65535) + wlookup2[..] +
///     wlookup1[..]`, three non-negative terms; the extremes are
///     65535 + 12167 + 4322 = 82024 < 2^17.
///   * every FIR coefficient satisfies |c| < 2^16 (extremes of the shipped
///     tables: +50983 / −10839); `fceux11_rust_filter_make` verifies this
///     and keeps `coeff_simd_safe = false` (scalar-only) otherwise.
///
/// Bias trick — why the result is bit-identical to the scalar path:
///   * bias `c' = c + 2^16 ∈ [0, 2^17)`, so the u32×u32→u64 `vpmuludq`
///     product `p' = s·c'` is exact;
///   * `s·2^16 ≡ 0 (mod 64)`, therefore
///     `p' >> 6 = ⌊s·c/64⌋ + 1024·s` (adding a multiple of 64 commutes
///     with the floor);
///   * `t = p' & !63` floors `p'` to a multiple of 64, so
///     `t = 64·(p' >> 6)` and `T = Σt` stays an exact integer with
///     `T < 2^34 · nc ≤ 2^44` — no u64 lane overflow;
///   * `Σs < 2^17 · nc ≤ 2^27` — the i32 `vpaddd` sums cannot wrap;
///   * scalar close per output sample: `acc = (T >> 6) − 1024·Σs`, which
///     is exactly `Σ⌊s·c/64⌋` because the `1024·s` terms cancel.
/// The per-tap `>> 6` truncation is thereby preserved (it lives inside
/// each `t`), and integer addition is associative, so lane splitting and
/// reordering cannot change the value.
///
/// SAFETY: caller must have confirmed AVX2 support at runtime
/// (`std::arch::is_x86_feature_detected!("avx2")`).
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn fir_pair_avx2(in_slice: &[i32], coeffs: &[i32], base: usize) -> (i64, i64) {
    use std::arch::x86_64::*;

    let nc = coeffs.len();
    let start = base - nc;
    // Whole 8-tap blocks; the tail (nc % 8 taps — 4 for nc = 484, 0 for
    // nc = 1024) is closed in scalar below, same per-tap truncation.
    let full = nc & !7;

    let mask_low6 = _mm256_set1_epi64x(!63i64);
    let bias = _mm256_set1_epi32(1 << 16);
    let mut t_a = _mm256_setzero_si256(); // u64 lane accumulator, acc chain
    let mut t_b = _mm256_setzero_si256(); // u64 lane accumulator, acc2 chain
    let mut s_a = _mm256_setzero_si256(); // i32 Σs, acc chain
    let mut s_b = _mm256_setzero_si256(); // i32 Σs, acc2 chain

    let mut blk = 0usize;
    while blk < full {
        // Window A: in[start+blk .. start+blk+8]  (acc chain)
        // Window B: the same window shifted by one element (acc2 chain).
        // Coefficients are loaded once and feed both chains.
        let sa = _mm256_loadu_si256(in_slice.as_ptr().add(start + blk) as *const __m256i);
        let sb = _mm256_loadu_si256(in_slice.as_ptr().add(start + blk + 1) as *const __m256i);
        let cb = _mm256_add_epi32(
            _mm256_loadu_si256(coeffs.as_ptr().add(blk) as *const __m256i),
            bias,
        );

        // vpmuludq multiplies the even 32-bit lanes into u64; the odd lanes
        // need both operands pre-shifted right by 32 bits.
        let p_a_e = _mm256_mul_epu32(sa, cb);
        let p_b_e = _mm256_mul_epu32(sb, cb);
        let sa_hi = _mm256_srli_epi64(sa, 32);
        let sb_hi = _mm256_srli_epi64(sb, 32);
        let cb_hi = _mm256_srli_epi64(cb, 32);
        let p_a_o = _mm256_mul_epu32(sa_hi, cb_hi);
        let p_b_o = _mm256_mul_epu32(sb_hi, cb_hi);

        t_a = _mm256_add_epi64(t_a, _mm256_and_si256(p_a_e, mask_low6));
        t_a = _mm256_add_epi64(t_a, _mm256_and_si256(p_a_o, mask_low6));
        t_b = _mm256_add_epi64(t_b, _mm256_and_si256(p_b_e, mask_low6));
        t_b = _mm256_add_epi64(t_b, _mm256_and_si256(p_b_o, mask_low6));

        s_a = _mm256_add_epi32(s_a, sa);
        s_b = _mm256_add_epi32(s_b, sb);
        blk += 8;
    }

    // Horizontal reduction. vpmuludq leaves u64 in lanes 0/2 only, but the
    // even+odd pairs fill all four lanes, so summing the stored lanes is
    // exact (unused lanes are zero).
    let mut lanes = [0u64; 4];
    _mm256_storeu_si256(lanes.as_mut_ptr() as *mut __m256i, t_a);
    let t_a_total = (lanes[0] + lanes[1]) + (lanes[2] + lanes[3]);
    _mm256_storeu_si256(lanes.as_mut_ptr() as *mut __m256i, t_b);
    let t_b_total = (lanes[0] + lanes[1]) + (lanes[2] + lanes[3]);

    let mut sums = [0i32; 8];
    _mm256_storeu_si256(sums.as_mut_ptr() as *mut __m256i, s_a);
    let sum_a = sums.iter().fold(0i64, |a, &v| a + v as i64);
    _mm256_storeu_si256(sums.as_mut_ptr() as *mut __m256i, s_b);
    let sum_b = sums.iter().fold(0i64, |a, &v| a + v as i64);

    let mut acc = (t_a_total >> 6) as i64 - 1024 * sum_a;
    let mut acc2 = (t_b_total >> 6) as i64 - 1024 * sum_b;

    // Scalar tail taps (full..nc), same arithmetic as fir_pair_scalar.
    for c in full..nc {
        let coeff = coeffs[c] as i64;
        acc += ((in_slice[start + c] as i64) * coeff) >> 6;
        acc2 += ((in_slice[start + c + 1] as i64) * coeff) >> 6;
    }
    (acc, acc2)
}

/// A5 G2 domain defense: the bias arithmetic is exact only for non-negative
/// samples (a negative i32 reinterpreted as u32 in `vpmuludq` breaks the
/// `p' >> 6 = ⌊s·c/64⌋ + 1024·s` identity). Scan the FIR window region
/// touched by this call — O(n); a single negative falls the WHOLE frame
/// back to the scalar path. In the real pipeline WaveHi can never go
/// negative, so this is a pure safety net.
///
/// Over-scan note: the windows span [first_base − nc, last_base] with
/// last_base ≤ inlen − 2, so scanning `[lo, inlen)` covers every index the
/// FIR will read (at most ~nc+1 over-scanned elements).
#[cfg(target_arch = "x86_64")]
fn simd_window_has_negative(in_slice: &[i32], first_base: usize, nc: usize) -> bool {
    let lo = first_base.saturating_sub(nc);
    in_slice[lo..].iter().any(|&v| v < 0)
}

/// A5 G1/G2 — shared resampler core. The two original duplicated loops
/// (soundq == 2 vs else) differed only in coefficient table and tap count,
/// so they are unified here; arithmetic and index order are unchanged.
///
/// `simd_coeff_safe` comes from `FilterState::coeff_simd_safe`. On x86_64
/// the AVX2 kernel is selected when the flag holds, the CPU supports AVX2
/// (runtime detection — NOT the `__AVX2__` compile-time macro, which is
/// unreliable under MSVC per v2.0.2 r6), and the domain defense passes;
/// otherwise the scalar path runs unchanged, which is the bit-exact
/// fallback by construction.
///
/// Returns `(new mrindex, number of output samples written)`.
fn resample(
    mut x: u32,
    mrratio: u32,
    coeffs: &[i32],
    in_slice: &[i32],
    out_slice: &mut [i32],
    simd_coeff_safe: bool,
) -> (u32, usize) {
    let nc = coeffs.len();
    let inlen = in_slice.len() as u32;
    let max = (inlen - 1) << 16;
    let mut out_idx = 0usize;

    let _ = simd_coeff_safe; // non-x86_64 builds never select the SIMD path

    #[cfg(target_arch = "x86_64")]
    let mut simd_ok = simd_coeff_safe && std::arch::is_x86_feature_detected!("avx2");
    #[cfg(not(target_arch = "x86_64"))]
    let simd_ok = false;
    #[cfg(not(target_arch = "x86_64"))]
    let _ = simd_ok;

    #[cfg(target_arch = "x86_64")]
    if simd_ok && x < max {
        // Domain defense: one negative anywhere in the touched window
        // region -> whole frame falls back to scalar.
        if simd_window_has_negative(in_slice, (x >> 16) as usize, nc) {
            simd_ok = false;
        }
    }

    while x < max {
        let base = (x >> 16) as usize;
        let frac = (x & 65535) as i64;
        #[cfg(target_arch = "x86_64")]
        let (acc, acc2) = if simd_ok {
            // SAFETY: AVX2 support confirmed by is_x86_feature_detected
            // above; window indices are in-bounds exactly as in the scalar
            // path (same index expressions, bounded by base ≤ inlen − 2).
            unsafe { fir_pair_avx2(in_slice, coeffs, base) }
        } else {
            fir_pair_scalar(in_slice, coeffs, base)
        };
        #[cfg(not(target_arch = "x86_64"))]
        let (acc, acc2) = fir_pair_scalar(in_slice, coeffs, base);
        let acc = (acc * (65536 - frac) + acc2 * frac) >> (16 + 11);
        out_slice[out_idx] = acc as i32;
        out_idx += 1;
        x += mrratio;
    }

    (x - max + (nc as u32) * 65536, out_idx)
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

    let (new_mrindex, out_idx) = if soundq == 2 {
        let coeffs = &state.sq2coeffs;
        resample(
            state.mrindex,
            state.mrratio,
            coeffs,
            in_slice,
            out_slice,
            state.coeff_simd_safe,
        )
    } else {
        let coeffs = &state.coeffs;
        resample(
            state.mrindex,
            state.mrratio,
            coeffs,
            in_slice,
            out_slice,
            state.coeff_simd_safe,
        )
    };
    state.mrindex = new_mrindex;

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

    // -----------------------------------------------------------------
    // A5 lock tests — scalar ⇔ SIMD equivalence + domain fallback.
    //
    // Discipline note (AGENTS.md #3): each assertion below must fail when
    // the property it names is broken. Candidate mutation points are
    // listed on each test so the red runs can be reproduced.
    // -----------------------------------------------------------------

    /// Deterministic PRNG (xorshift32) — no floating point, no external
    /// crates, identical stream on every host.
    #[cfg(target_arch = "x86_64")]
    struct Xorshift32(u32);
    #[cfg(target_arch = "x86_64")]
    impl Xorshift32 {
        fn next(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            x
        }
        /// Domain sample: the NeoFilterSound input satisfies
        /// s ∈ [0, 82024) — see the WaveHi dequant comment on
        /// `fir_pair_avx2`.
        fn next_sample(&mut self) -> i32 {
            (self.next() % 82024) as i32
        }
    }

    /// Mirror a half coefficient table the way
    /// `fceux11_rust_filter_make` does.
    #[cfg(target_arch = "x86_64")]
    fn mirror_table(half: &[i32], nc: usize) -> Vec<i32> {
        let mut t = vec![0i32; nc];
        for (x, &v) in half.iter().take(nc >> 1).enumerate() {
            t[x] = v;
            t[nc - 1 - x] = v;
        }
        t
    }

    #[cfg(target_arch = "x86_64")]
    fn real_coeffs_484() -> Vec<i32> {
        mirror_table(&C48000NTSC, NCOEFFS)
    }

    #[cfg(target_arch = "x86_64")]
    fn real_coeffs_1024() -> Vec<i32> {
        mirror_table(&SQ2C48000NTSC, SQ2NCOEFFS)
    }

    /// Boundary coefficient table: the extreme magnitudes observed across
    /// the shipped tables (±50983), the other-sign extreme (±10839), the
    /// largest values that still satisfy the SIMD precondition
    /// (±65535 = 2^16 − 1), plus 0 and −1, tiled to length nc.
    #[cfg(target_arch = "x86_64")]
    fn boundary_coeffs(nc: usize) -> Vec<i32> {
        let seeds = [50983i32, -50983, 0, -1, 10839, -10839, 65535, -65535];
        (0..nc).map(|i| seeds[i % seeds.len()]).collect()
    }

    #[cfg(target_arch = "x86_64")]
    fn domain_input(len: usize, seed: u32) -> Vec<i32> {
        let mut rng = Xorshift32(seed);
        let mut v = vec![0i32; len];
        for s in v.iter_mut() {
            *s = rng.next_sample();
        }
        // Pin the domain boundary values explicitly.
        v[0] = 0;
        v[1] = 82023;
        v[2] = 65535;
        v[3] = 1;
        v
    }

    /// Lock test 2a — per-position FIR pair equality, scalar vs AVX2.
    ///
    /// Mutation points caught (A5 G3):
    ///   * M-A bias constant `(1 << 16)` in `fir_pair_avx2` changed
    ///     (equivalent to a per-tap coefficient offset on the SIMD side
    ///     only) — pair drift;
    ///   * M-B scalar close `(T >> 6) − 1024·Σs` losing the `−1024·Σs`
    ///     term — pair drift on every position with Σs ≠ 0;
    ///   * M-C `& !63` → `& !31` in `fir_pair_avx2` — truncation drift;
    ///   * M-E tail loop `for c in full..nc` skipped/shortened — drift on
    ///     tables whose tap count is not a multiple of 8 (484).
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn lock_fir_pair_scalar_simd_bit_exact() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            panic!(
                "AVX2 not detected on this host: the A5 equivalence lock \
                 must run where the SIMD path is live"
            );
        }
        let len = 4096usize;
        let input = domain_input(len, 0xA5_11_06);

        let tables = [
            real_coeffs_484(),
            real_coeffs_1024(),
            boundary_coeffs(484),
            boundary_coeffs(1024),
            boundary_coeffs(12),  // 8-tap full block + 4-tap scalar tail
            boundary_coeffs(20),  // 2 blocks + 4-tap tail
        ];
        for coeffs in &tables {
            let nc = coeffs.len();
            // Sweep base over the valid range with an odd stride so both
            // even and odd 32-bit lanes of every window are exercised.
            let mut base = nc + 1;
            let mut checked = 0usize;
            while base <= len - 2 {
                let scalar = fir_pair_scalar(&input, coeffs, base);
                let simd = unsafe { fir_pair_avx2(&input, coeffs, base) };
                assert_eq!(
                    scalar, simd,
                    "FIR pair drift: nc={nc} base={base}"
                );
                checked += 1;
                base += 7;
            }
            assert!(checked > 100, "sweep too short: nc={nc}");
        }
    }

    /// Lock test 2b — end-to-end `resample` equality (FIR pair + frac
    /// interpolation + mrindex bookkeeping) for both tap counts.
    ///
    /// SCOPE — what this lock does and does NOT cover (A5 G3, measured):
    ///   * It compares the scalar run against the SIMD run, i.e. it is a
    ///     RELATIVE comparison between two paths that SHARE the code
    ///     under test. It therefore catches asymmetries (a defect that
    ///     moves only the SIMD side, e.g. M-A / M-B / M-C / M-E) and
    ///     the mrindex bookkeeping.
    ///   * It does NOT cover the fractional interpolation weights
    ///     themselves. The interpolation lives in `resample` AFTER the
    ///     scalar/SIMD fork (see `resample`), so both sides of this
    ///     assertion use the same interpolation expression. Flipping
    ///     them (`65536 - frac` → `frac`, mutation M-D) moves both
    ///     sides identically and this lock stays GREEN — measured:
    ///     M-D leaves 2a, 2b and lock 3 all green (51/51).
    ///   * The interpolation weight direction is covered instead by
    ///     `lock_resample_interpolation_golden_vector`, which pins
    ///     absolute expected values derived from the definition of
    ///     linear interpolation.
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn lock_resample_end_to_end_scalar_simd_bit_exact() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            panic!(
                "AVX2 not detected on this host: the A5 equivalence lock \
                 must run where the SIMD path is live"
            );
        }
        let len = 16384usize;
        let input = domain_input(len, 0xBEEF_01);

        // mrratio is a 16.16 fixed-point INPUT SAMPLES PER OUTPUT value;
        // the real NTSC-48k ratio is ≈ 2444076 (37.28 samples per output).
        // Values: real NTSC-48k, exactly 1 sample/output (frac pinned to
        // the entry phase), a 1.5x downsample, a 2x downsample, and a
        // 256x downsample (few-output corner).
        let ratios = [2_444_076u32, 65536, 98_304, 131_072, 16_777_216];
        // Entry phases: frac ≈ 0, ≈ 65535, and in between.
        let phase_offsets = [0u32, 1, 65534, 65535];

        let tables = [real_coeffs_484(), real_coeffs_1024(), boundary_coeffs(484)];
        for coeffs in &tables {
            let nc = coeffs.len();
            let x0 = ((nc as u32) + 1) << 16;
            for &ratio in &ratios {
                for &off in &phase_offsets {
                    let x = x0.wrapping_add(off);
                    let mut out_scalar = vec![0i32; len];
                    let mut out_simd = vec![0i32; len];
                    let (mr_s, n_s) =
                        resample(x, ratio, coeffs, &input, &mut out_scalar, false);
                    let (mr_v, n_v) =
                        resample(x, ratio, coeffs, &input, &mut out_simd, true);
                    assert_eq!(n_s, n_v, "output count drift nc={nc} ratio={ratio}");
                    assert!(n_s > 0, "no outputs: nc={nc} ratio={ratio}");
                    assert_eq!(mr_s, mr_v, "mrindex drift nc={nc} ratio={ratio}");
                    assert_eq!(
                        out_scalar[..n_s],
                        out_simd[..n_s],
                        "output drift nc={nc} ratio={ratio} off={off}"
                    );
                }
            }
        }
    }

    /// Lock test 3 — domain fallback: a single negative sample anywhere in
    /// the touched window region must (a) be detected by the gate, (b) make
    /// the whole frame take the scalar path, and (c) yield results
    /// identical to the scalar path — while the raw AVX2 kernel on the
    /// same input demonstrably diverges (the gate is load-bearing).
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn lock_domain_fallback_on_negative_sample() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            panic!(
                "AVX2 not detected on this host: the A5 fallback lock \
                 must run where the SIMD path is live"
            );
        }
        let len = 8192usize;
        let coeffs = real_coeffs_484();
        let nc = coeffs.len();

        // (a) gate detects a negative anywhere in [first_base − nc, inlen).
        let mut input = domain_input(len, 0xFEED_BAC);
        let first_base = nc + 1;
        assert!(!simd_window_has_negative(&input, first_base, nc));
        // Negative just past the earliest window start …
        input[first_base - nc] = -1;
        assert!(simd_window_has_negative(&input, first_base, nc));
        // … and near the end of the touched region.
        let mut input2 = domain_input(len, 0xFEED_BAC);
        input2[len - 2] = -7; // last index a window can reach (base ≤ len−2)
        assert!(simd_window_has_negative(&input2, first_base, nc));

        // (b) whole-frame fallback: with simd enabled the result must be
        // byte-identical to the forced-scalar run.
        let ratio = 2_444_076u32; // real NTSC-48k mrratio
        let x0 = ((nc as u32) + 1) << 16;
        let mut out_scalar = vec![0i32; len];
        let mut out_dispatch = vec![0i32; len];
        let (mr_s, n_s) = resample(x0, ratio, &coeffs, &input, &mut out_scalar, false);
        let (mr_d, n_d) = resample(x0, ratio, &coeffs, &input, &mut out_dispatch, true);
        assert_eq!(n_s, n_d);
        assert!(n_s > 0);
        assert_eq!(mr_s, mr_d);
        assert_eq!(out_scalar[..n_s], out_dispatch[..n_s]);

        // (c) the gate is load-bearing: feeding the negative input straight
        // into the AVX2 kernel diverges from the scalar path.
        let base = first_base;
        let scalar = fir_pair_scalar(&input, &coeffs, base);
        let raw_simd = unsafe { fir_pair_avx2(&input, &coeffs, base) };
        assert_ne!(
            scalar, raw_simd,
            "negative input must diverge between scalar and raw SIMD \
             (otherwise this fallback lock proves nothing)"
        );
    }

    // -----------------------------------------------------------------
    // A5 G3 follow-up — ABSOLUTE golden-vector lock for the fractional
    // interpolation of `resample`.
    //
    // Why this test exists (A5 G3, measured):
    //   * Lock 2a (`fir_pair` scalar vs AVX2), lock 2b (`resample`
    //     scalar vs AVX2) and lock 3 (SIMD-gated vs forced scalar) are
    //     all RELATIVE comparisons: each one compares two paths that
    //     share the code under test.
    //   * The fractional interpolation lives in `resample` AFTER the
    //     scalar/SIMD fork, so BOTH sides of every relative lock use
    //     the same interpolation expression. Flipping the weights
    //     (`65536 - frac` -> `frac`, mutation M-D) therefore moves the
    //     real output (probe: input amplitudes 300 / 1000 / 4000 give
    //     different output hashes and different per-sample values)
    //     while every one of those relative locks stays green.
    //   * => the relative locks cannot see the interpolation at all.
    //     (The doc comment on lock 2b claiming it catches M-D is
    //     wrong; measured M-D leaves 2b green.)
    //
    // What this test does instead: it pins ABSOLUTE expected values
    // produced by a reference implementation that lives in this test
    // module and derives the result from the DEFINITION of linear
    // interpolation (see `golden_resample`), so the reason the test
    // passes is the very property under test — which of the two
    // adjacent FIR windows carries which weight. Nothing here compares
    // two implementations of the interpolation, and the expectation
    // never calls the code under test.
    //
    // The AVX2 dispatch is pinned to the same absolute values, so the
    // SIMD-side mutations (M-A bias constant, M-B missing `-1024*Σs`,
    // M-C `!63` -> `!31`, M-E skipped tail taps) are caught here too,
    // on top of lock 2a.
    // -----------------------------------------------------------------

    /// Golden reference for ONE FIR evaluation — the documented spec of
    /// `fir_pair_scalar`:
    ///
    /// ```text
    /// acc = Σ_{c=0..nc-1} ⌊ in[base-nc+c] · coeffs[c] / 64 ⌋
    /// ```
    ///
    /// (floor per tap, then sum — `⌊Σx⌋ ≠ Σ⌊x⌋`, the per-tap
    /// truncation is part of the spec). Re-derived here on purpose: the
    /// reference must not share code with the artifact it judges.
    #[cfg(target_arch = "x86_64")]
    fn golden_fir(input: &[i32], coeffs: &[i32], base: usize) -> i64 {
        let nc = coeffs.len();
        let start = base - nc;
        let mut acc = 0i64;
        for c in 0..nc {
            let tap = (input[start + c] as i64) * (coeffs[c] as i64);
            acc += tap >> 6;
        }
        acc
    }

    /// One golden output sample: the interpolated value plus the two
    /// FIR endpoints it was blended from (kept for the direction
    /// anchors and for failure diagnostics).
    #[cfg(target_arch = "x86_64")]
    #[derive(Clone, Copy, Debug)]
    struct GoldenSample {
        x: u32,
        acc: i64,
        acc2: i64,
        value: i32,
    }

    /// Independent reference implementation of the `resample` output
    /// stream.
    ///
    /// Ground truth — the resampler reads the input at the 16.16 fixed
    /// point position `x`, i.e. at the real position `base + t` with
    /// `base = x >> 16` and `t = frac / 65536`, `frac = x & 0xFFFF`.
    /// The FIR is evaluated at the two adjacent window starts and the
    /// result is linearly interpolated:
    ///
    /// ```text
    /// f(base + t) = (1 - t)·f(base) + t·f(base + 1)
    ///             = f(base) + (f(base + 1) - f(base))·t
    /// ```
    ///
    /// so `base` carries the weight `(1 - t)` and `base + 1` carries
    /// the weight `t`. The FIR result carries a fixed 2^-11 output
    /// scale, so the integer output sample is `floor(f / 2^11)`.
    ///
    /// The fractional part is kept exact as a numerator over 2^16 and
    /// the output scale is applied by one floor division by
    /// 2^27 = 2^16 · 2^11 — the same value the implementation produces,
    /// but reached from the interpolation identity instead of copied
    /// from the implementation's expression. Flipping the weights
    /// (`acc·frac + acc2·(65536 - frac)`) is equivalent to
    /// interpolating at `base + (1 - t)` — the wrong position — and
    /// therefore disagrees with this reference wherever the two
    /// endpoints differ.
    #[cfg(target_arch = "x86_64")]
    fn golden_resample(
        input: &[i32],
        coeffs: &[i32],
        x0: u32,
        mrratio: u32,
    ) -> Vec<GoldenSample> {
        let max = ((input.len() as u32) - 1) << 16;
        let mut out = Vec::new();
        let mut x = x0;
        while x < max {
            let base = (x >> 16) as usize;
            let frac = (x & 0xFFFF) as i64;
            let acc = golden_fir(input, coeffs, base);
            let acc2 = golden_fir(input, coeffs, base + 1);
            // f = acc + (acc2 - acc)·t, t = frac/65536, carried exactly
            // as a numerator over 2^16; then the fixed 2^-11 output
            // scale (floor division by 2^27 = 2^16 · 2^11).
            let numer = (acc as i128) * 65536 + ((acc2 - acc) as i128) * (frac as i128);
            out.push(GoldenSample {
                x,
                acc,
                acc2,
                value: (numer >> 27) as i32,
            });
            x = x.wrapping_add(mrratio);
        }
        out
    }

    /// Run `resample` for a single output position and return that
    /// output sample. The ratio is large enough to leave the loop after
    /// one sample, so the first (and only) output corresponds to `x`.
    #[cfg(target_arch = "x86_64")]
    fn resample_first(input: &[i32], coeffs: &[i32], x: u32, simd: bool) -> i32 {
        let mut out = vec![0i32; input.len()];
        let (_mr, n) = resample(x, 1 << 30, coeffs, input, &mut out, simd);
        assert!(n >= 1, "resample produced no output for x={x}");
        out[0]
    }

    /// Structured probe input: a slow ramp plus a fast square wave plus
    /// dither, all inside the legal domain [0, 82024). The square wave
    /// makes adjacent FIR windows differ strongly, which is what makes
    /// the fractional interpolation observable at all — the output
    /// scale divides the FIR result by 2^11, so a small window-to-
    /// window delta would vanish in the output.
    #[cfg(target_arch = "x86_64")]
    fn structured_input(len: usize, seed: u32) -> Vec<i32> {
        let mut rng = Xorshift32(seed);
        let mut v = vec![0i32; len];
        for (i, s) in v.iter_mut().enumerate() {
            let ramp = ((i * 37) % 65536) as i32;
            let square = if (i / 3) % 2 == 0 { 8000 } else { 0 };
            let dither = (rng.next() % 2048) as i32;
            *s = (ramp + square + dither).min(82023);
        }
        v
    }

    /// Lock test 4 — absolute golden vector for the `resample`
    /// fractional interpolation.
    ///
    /// Covers the common-cause blind spot of locks 2a / 2b / 3: those
    /// compare two paths that share the interpolation expression, so a
    /// weight flip (M-D) moves both sides identically and they stay
    /// green. This test instead compares `resample` against an
    /// independently derived expectation (see `golden_resample`), so
    /// the "pass reason" IS the property under test — which of the two
    /// adjacent FIR windows carries which interpolation weight.
    ///
    /// Mutation points caught (A5 G3):
    ///   * M-D interpolation `65536 - frac` → `frac` — the golden value
    ///     is derived from `f(base + t) = f(base) + (f(base+1) -
    ///     f(base))·t`, so a flipped weight (interpolating at
    ///     `base + (1 - t)`) disagrees on essentially every output
    ///     sample, and the frac = 0 direction anchor fires at once;
    ///   * M-A bias constant, M-B missing `-1024·Σs`, M-C `!63` → `!31`,
    ///     M-E skipped tail taps — the AVX2 dispatch is pinned to the
    ///     same absolute golden values, so any SIMD-side drift shows up
    ///     here as a mismatch against an absolute number.
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn lock_resample_interpolation_golden_vector() {
        if !std::arch::is_x86_feature_detected!("avx2") {
            panic!(
                "AVX2 not detected on this host: the A5 interpolation golden \
                 lock must run where the SIMD path is live"
            );
        }
        let len = 2048usize;
        let inputs = [
            ("domain", domain_input(len, 0x5AFE_7A1E)),
            ("structured", structured_input(len, 0x0BAD_C0DE)),
        ];
        let tables = [
            ("real484", real_coeffs_484()),
            ("real1024", real_coeffs_1024()),
            ("boundary484", boundary_coeffs(484)),
        ];
        // mrratio is 16.16 fixed-point INPUT SAMPLES PER OUTPUT: the real
        // NTSC-48k ratio (frac steps by 19244 -> a rich frac sequence),
        // exactly 1 sample/output (frac pinned to the entry phase), a
        // 1.5x downsample (frac alternates over two values) and a 256x
        // downsample (few-output corner).
        let ratios = [2_444_076u32, 65536, 98_304, 16_777_216];
        // Entry phases: frac ≈ 0, ≈ 1, just below and just above the
        // degenerate mid-point (32768, where both weights are equal).
        let phases = [0u32, 1, 32_767, 65_535];

        for (tname, coeffs) in &tables {
            let nc = coeffs.len();
            for (iname, input) in &inputs {
                for &ratio in &ratios {
                    for &phase in &phases {
                        let x0 = ((nc as u32) + 1) << 16;
                        let x = x0.wrapping_add(phase);
                        let golden = golden_resample(input, coeffs, x, ratio);
                        assert!(
                            !golden.is_empty(),
                            "no golden samples: table={tname} input={iname} \
                             ratio={ratio} phase={phase}"
                        );

                        let mut out_scalar = vec![0i32; len];
                        let mut out_simd = vec![0i32; len];
                        let (mr_s, n_s) =
                            resample(x, ratio, coeffs, input, &mut out_scalar, false);
                        let (mr_v, n_v) =
                            resample(x, ratio, coeffs, input, &mut out_simd, true);
                        assert_eq!(
                            n_s, n_v,
                            "output count drift: table={tname} input={iname} \
                             ratio={ratio} phase={phase}"
                        );
                        assert_eq!(
                            mr_s, mr_v,
                            "mrindex drift: table={tname} input={iname} \
                             ratio={ratio} phase={phase}"
                        );
                        assert_eq!(
                            n_s,
                            golden.len(),
                            "golden/output count mismatch: table={tname} \
                             input={iname} ratio={ratio} phase={phase}"
                        );

                        for (i, g) in golden.iter().enumerate() {
                            let frac = (g.x & 0xFFFF) as i64;
                            // Diagnostic only: what a flipped-weight
                            // implementation would have produced here.
                            let flipped =
                                ((g.acc * frac) + (g.acc2 * (65536 - frac))) >> 27;
                            assert_eq!(
                                out_scalar[i],
                                g.value,
                                "golden-vector mismatch (interpolation weight \
                                 direction / FIR arithmetic): table={tname} \
                                 input={iname} ratio={ratio} phase={phase} \
                                 idx={i} x={} resample={} golden={} acc={} \
                                 acc2={} flipped-weight-would-give={flipped}",
                                g.x,
                                out_scalar[i],
                                g.value,
                                g.acc,
                                g.acc2,
                            );
                            assert_eq!(
                                out_simd[i],
                                g.value,
                                "golden-vector mismatch on the AVX2 dispatch \
                                 (interpolation weight direction / FIR \
                                 arithmetic): table={tname} input={iname} \
                                 ratio={ratio} phase={phase} idx={i} x={} \
                                 resample={} golden={} acc={} acc2={} \
                                 flipped-weight-would-give={flipped}",
                                g.x,
                                out_simd[i],
                                g.value,
                                g.acc,
                                g.acc2,
                            );
                        }
                    }
                }
            }
        }

        // ---- Direction anchors -------------------------------------
        // The interpolation must slide from the FIR at `base` towards
        // the FIR at `base + 1` as the fractional part grows:
        //   frac = 0     -> the output IS the FIR at `base`
        //   frac = 65535 -> the output HAS REACHED the FIR at `base + 1`
        // These state the semantics directly; they compare no two
        // implementations, so a weight flip is caught by construction.
        for (tname, coeffs) in &tables {
            let nc = coeffs.len();
            let base = nc + 1;
            for (iname, input) in &inputs {
                let acc = golden_fir(input, coeffs, base);
                let acc2 = golden_fir(input, coeffs, base + 1);
                assert_ne!(
                    acc, acc2,
                    "degenerate probe: the two FIR endpoints must differ \
                     (table={tname} input={iname})"
                );
                let rising = acc2 > acc;

                // frac = 0: the read position is exactly `base`, so the
                // output must be the FIR at `base` and nothing else.
                let x_zero = (base as u32) << 16;
                for &simd in &[false, true] {
                    let out = resample_first(input, coeffs, x_zero, simd);
                    assert_eq!(
                        out as i64,
                        acc >> 11,
                        "interpolation weight direction: at frac=0 the \
                         output must be the FIR at `base` ({acc} >> 11), \
                         not the FIR at `base + 1` ({acc2} >> 11) \
                         [simd={simd} table={tname} input={iname}]"
                    );
                }

                // frac = 65535: the read position is one LSB short of
                // `base + 1`, so the output must have reached the FIR
                // at `base + 1` (the residual is the truncated part of
                // the 1/65536 remainder, i.e. |acc - acc2| / 2^27).
                let x_max = ((base as u32) << 16) | 65_535;
                let slack = ((acc - acc2).abs() >> 27) + 1;
                for &simd in &[false, true] {
                    let out = resample_first(input, coeffs, x_max, simd) as i64;
                    assert!(
                        (out - (acc2 >> 11)).abs() <= slack,
                        "interpolation weight direction: at frac=65535 the \
                         output must have reached the FIR at `base + 1` \
                         ({} >> 11 = {}, slack={slack}), got {out} \
                         [simd={simd} table={tname} input={iname}]",
                        acc2,
                        acc2 >> 11,
                    );
                }

                // Monotonic slide: as frac grows the output must move
                // monotonically from one endpoint towards the other.
                let mut prev: Option<i64> = None;
                for &frac in &[0u32, 16_384, 32_768, 49_152, 65_535] {
                    let x = ((base as u32) << 16) | frac;
                    let v = resample_first(input, coeffs, x, false) as i64;
                    if let Some(p) = prev {
                        let ok = if rising { v >= p } else { v <= p };
                        assert!(
                            ok,
                            "interpolation weight direction: the output must \
                             move monotonically with frac (table={tname} \
                             input={iname} frac={frac} prev={p} now={v} \
                             acc={acc} acc2={acc2})"
                        );
                    }
                    prev = Some(v);
                }
            }
        }
    }
}
