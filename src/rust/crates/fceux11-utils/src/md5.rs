use std::ffi::c_char;
use std::slice;

/// C-compatible MD5 context layout. Must match `struct md5_context` in C++ exactly.
#[repr(C)]
pub struct Md5Context {
    pub total: [u32; 2],
    pub state: [u32; 4],
    pub buffer: [u8; 64],
}

macro_rules! get_uint32_le {
    ($b:expr, $i:expr) => {
        (($b[$i + 3] as u32) << 24)
            | (($b[$i + 2] as u32) << 16)
            | (($b[$i + 1] as u32) << 8)
            | ($b[$i] as u32)
    };
}

macro_rules! put_uint32_le {
    ($n:expr, $b:expr, $i:expr) => {
        $b[$i] = ($n) as u8;
        $b[$i + 1] = (($n) >> 8) as u8;
        $b[$i + 2] = (($n) >> 16) as u8;
        $b[$i + 3] = (($n) >> 24) as u8;
    };
}

fn f(x: u32, y: u32, z: u32) -> u32 {
    z ^ (x & (y ^ z))
}
fn g(x: u32, y: u32, z: u32) -> u32 {
    y ^ (z & (x ^ y))
}
fn h(x: u32, y: u32, z: u32) -> u32 {
    x ^ y ^ z
}
fn i(x: u32, y: u32, z: u32) -> u32 {
    y ^ (x | !z)
}

fn rotate_left(x: u32, n: u32) -> u32 {
    (x << n) | ((x & 0xFFFFFFFF) >> (32 - n))
}

fn md5_process(ctx: &mut Md5Context, data: &[u8; 64]) {
    let mut x = [0u32; 16];
    for j in 0..16 {
        x[j] = get_uint32_le!(data, j * 4);
    }

    let mut a = ctx.state[0];
    let mut b = ctx.state[1];
    let mut c = ctx.state[2];
    let mut d = ctx.state[3];

    // Round 1
    let step = |f: fn(u32, u32, u32) -> u32, a: &mut u32, b: u32, c: u32, d: u32, k: usize, s: u32, t: u32| {
        *a = b.wrapping_add(rotate_left(a.wrapping_add(f(b, c, d)).wrapping_add(x[k]).wrapping_add(t), s));
    };

    step(f, &mut a, b, c, d, 0, 7, 0xD76AA478);
    step(f, &mut d, a, b, c, 1, 12, 0xE8C7B756);
    step(f, &mut c, d, a, b, 2, 17, 0x242070DB);
    step(f, &mut b, c, d, a, 3, 22, 0xC1BDCEEE);
    step(f, &mut a, b, c, d, 4, 7, 0xF57C0FAF);
    step(f, &mut d, a, b, c, 5, 12, 0x4787C62A);
    step(f, &mut c, d, a, b, 6, 17, 0xA8304613);
    step(f, &mut b, c, d, a, 7, 22, 0xFD469501);
    step(f, &mut a, b, c, d, 8, 7, 0x698098D8);
    step(f, &mut d, a, b, c, 9, 12, 0x8B44F7AF);
    step(f, &mut c, d, a, b, 10, 17, 0xFFFF5BB1);
    step(f, &mut b, c, d, a, 11, 22, 0x895CD7BE);
    step(f, &mut a, b, c, d, 12, 7, 0x6B901122);
    step(f, &mut d, a, b, c, 13, 12, 0xFD987193);
    step(f, &mut c, d, a, b, 14, 17, 0xA679438E);
    step(f, &mut b, c, d, a, 15, 22, 0x49B40821);

    // Round 2
    step(g, &mut a, b, c, d, 1, 5, 0xF61E2562);
    step(g, &mut d, a, b, c, 6, 9, 0xC040B340);
    step(g, &mut c, d, a, b, 11, 14, 0x265E5A51);
    step(g, &mut b, c, d, a, 0, 20, 0xE9B6C7AA);
    step(g, &mut a, b, c, d, 5, 5, 0xD62F105D);
    step(g, &mut d, a, b, c, 10, 9, 0x02441453);
    step(g, &mut c, d, a, b, 15, 14, 0xD8A1E681);
    step(g, &mut b, c, d, a, 4, 20, 0xE7D3FBC8);
    step(g, &mut a, b, c, d, 9, 5, 0x21E1CDE6);
    step(g, &mut d, a, b, c, 14, 9, 0xC33707D6);
    step(g, &mut c, d, a, b, 3, 14, 0xF4D50D87);
    step(g, &mut b, c, d, a, 8, 20, 0x455A14ED);
    step(g, &mut a, b, c, d, 13, 5, 0xA9E3E905);
    step(g, &mut d, a, b, c, 2, 9, 0xFCEFA3F8);
    step(g, &mut c, d, a, b, 7, 14, 0x676F02D9);
    step(g, &mut b, c, d, a, 12, 20, 0x8D2A4C8A);

    // Round 3
    step(h, &mut a, b, c, d, 5, 4, 0xFFFA3942);
    step(h, &mut d, a, b, c, 8, 11, 0x8771F681);
    step(h, &mut c, d, a, b, 11, 16, 0x6D9D6122);
    step(h, &mut b, c, d, a, 14, 23, 0xFDE5380C);
    step(h, &mut a, b, c, d, 1, 4, 0xA4BEEA44);
    step(h, &mut d, a, b, c, 4, 11, 0x4BDECFA9);
    step(h, &mut c, d, a, b, 7, 16, 0xF6BB4B60);
    step(h, &mut b, c, d, a, 10, 23, 0xBEBFBC70);
    step(h, &mut a, b, c, d, 13, 4, 0x289B7EC6);
    step(h, &mut d, a, b, c, 0, 11, 0xEAA127FA);
    step(h, &mut c, d, a, b, 3, 16, 0xD4EF3085);
    step(h, &mut b, c, d, a, 6, 23, 0x04881D05);
    step(h, &mut a, b, c, d, 9, 4, 0xD9D4D039);
    step(h, &mut d, a, b, c, 12, 11, 0xE6DB99E5);
    step(h, &mut c, d, a, b, 15, 16, 0x1FA27CF8);
    step(h, &mut b, c, d, a, 2, 23, 0xC4AC5665);

    // Round 4
    step(i, &mut a, b, c, d, 0, 6, 0xF4292244);
    step(i, &mut d, a, b, c, 7, 10, 0x432AFF97);
    step(i, &mut c, d, a, b, 14, 15, 0xAB9423A7);
    step(i, &mut b, c, d, a, 5, 21, 0xFC93A039);
    step(i, &mut a, b, c, d, 12, 6, 0x655B59C3);
    step(i, &mut d, a, b, c, 3, 10, 0x8F0CCC92);
    step(i, &mut c, d, a, b, 10, 15, 0xFFEFF47D);
    step(i, &mut b, c, d, a, 1, 21, 0x85845DD1);
    step(i, &mut a, b, c, d, 8, 6, 0x6FA87E4F);
    step(i, &mut d, a, b, c, 15, 10, 0xFE2CE6E0);
    step(i, &mut c, d, a, b, 6, 15, 0xA3014314);
    step(i, &mut b, c, d, a, 13, 21, 0x4E0811A1);
    step(i, &mut a, b, c, d, 4, 6, 0xF7537E82);
    step(i, &mut d, a, b, c, 11, 10, 0xBD3AF235);
    step(i, &mut c, d, a, b, 2, 15, 0x2AD7D2BB);
    step(i, &mut b, c, d, a, 9, 21, 0xEB86D391);

    ctx.state[0] = ctx.state[0].wrapping_add(a);
    ctx.state[1] = ctx.state[1].wrapping_add(b);
    ctx.state[2] = ctx.state[2].wrapping_add(c);
    ctx.state[3] = ctx.state[3].wrapping_add(d);
}

const MD5_PADDING: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// # Safety
/// `ctx` must point to a valid, writable `Md5Context`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_md5_starts(ctx: *mut Md5Context) {
    let ctx = unsafe { &mut *ctx };
    ctx.total = [0, 0];
    ctx.state = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476];
    ctx.buffer = [0u8; 64];
}

/// # Safety
/// `ctx` must point to a valid, writable `Md5Context`.
/// `input` must point to at least `length` valid bytes, or be NULL when `length` is 0.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_md5_update(ctx: *mut Md5Context, input: *const u8, length: u32) {
    if input.is_null() || length == 0 {
        return;
    }
    let ctx = unsafe { &mut *ctx };
    let input = unsafe { slice::from_raw_parts(input, length as usize) };
    let mut length = length as u32;
    let mut input_offset = 0usize;

    let mut left = ((ctx.total[0] >> 3) & 0x3F) as usize;
    let fill = 64 - left;

    ctx.total[0] = ctx.total[0].wrapping_add(length << 3);
    if ctx.total[0] < (length << 3) {
        ctx.total[1] = ctx.total[1].wrapping_add(1);
    }
    ctx.total[1] = ctx.total[1].wrapping_add(length >> 29);

    if left != 0 && length >= fill as u32 {
        ctx.buffer[left..left + fill].copy_from_slice(&input[input_offset..input_offset + fill]);
        let block = ctx.buffer;
        md5_process(ctx, &block);
        length -= fill as u32;
        input_offset += fill;
        left = 0;
    }

    while length >= 64 {
        let mut block = [0u8; 64];
        block.copy_from_slice(&input[input_offset..input_offset + 64]);
        md5_process(ctx, &block);
        length -= 64;
        input_offset += 64;
    }

    if length > 0 {
        let len = length as usize;
        ctx.buffer[left..left + len].copy_from_slice(&input[input_offset..input_offset + len]);
    }
}

/// # Safety
/// `ctx` must point to a valid, writable `Md5Context`.
/// `digest` must point to at least 16 writable bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_md5_finish(ctx: *mut Md5Context, digest: *mut u8) {
    let ctx = unsafe { &mut *ctx };
    let digest = unsafe { slice::from_raw_parts_mut(digest, 16) };

    let last = ((ctx.total[0] >> 3) & 0x3F) as usize;
    let padn = if last < 56 { 56 - last } else { 120 - last };

    let mut msglen = [0u8; 8];
    put_uint32_le!(ctx.total[0], msglen, 0);
    put_uint32_le!(ctx.total[1], msglen, 4);

    fceux11_rust_md5_update(ctx, MD5_PADDING.as_ptr(), padn as u32);
    fceux11_rust_md5_update(ctx, msglen.as_ptr(), 8);

    put_uint32_le!(ctx.state[0], digest, 0);
    put_uint32_le!(ctx.state[1], digest, 4);
    put_uint32_le!(ctx.state[2], digest, 8);
    put_uint32_le!(ctx.state[3], digest, 12);
}

// Thread-local static buffer, matching original C++ behavior.
thread_local! {
    static MD5_ASCII_STR: std::cell::RefCell<[c_char; 33]> = const { std::cell::RefCell::new([0; 33]) };
}

/// # Safety
/// `md5` must point to a valid 16-byte MD5 digest.
/// Returns a pointer to a thread-local static buffer. Caller should copy immediately.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_md5_asciistr(md5: *const u8) -> *const c_char {
    if md5.is_null() {
        return std::ptr::null();
    }
    let md5 = unsafe { slice::from_raw_parts(md5, 16) };
    let trans: [u8; 16] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f'];

    MD5_ASCII_STR.with(|s| {
        let mut s = s.borrow_mut();
        for x in 0..16 {
            s[x * 2] = trans[(md5[x] >> 4) as usize] as c_char;
            s[x * 2 + 1] = trans[(md5[x] & 0x0F) as usize] as c_char;
        }
        s[32] = 0;
        s.as_ptr()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use md5::Md5;
    use digest::Digest;

    fn rust_md5_compute(data: &[u8]) -> [u8; 16] {
        let mut ctx = Md5Context {
            total: [0, 0],
            state: [0, 0, 0, 0],
            buffer: [0; 64],
        };
        let mut digest = [0u8; 16];
        fceux11_rust_md5_starts(&mut ctx);
        fceux11_rust_md5_update(&mut ctx, data.as_ptr(), data.len() as u32);
        fceux11_rust_md5_finish(&mut ctx, digest.as_mut_ptr());
        digest
    }

    fn crate_md5_compute(data: &[u8]) -> [u8; 16] {
        let mut hasher = Md5::new();
        Digest::update(&mut hasher, data);
        hasher.finalize().into()
    }

    #[test]
    fn test_md5_empty() {
        let digest = rust_md5_compute(b"");
        let expected = crate_md5_compute(b"");
        assert_eq!(digest, expected);
    }

    #[test]
    fn test_md5_the_quick_brown_fox() {
        let digest = rust_md5_compute(b"The quick brown fox jumps over the lazy dog");
        let expected = crate_md5_compute(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(digest, expected);
    }

    #[test]
    fn test_md5_asciistr() {
        let digest: [u8; 16] = [
            0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04,
            0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8, 0x42, 0x7e,
        ];
        let ptr = fceux11_rust_md5_asciistr(digest.as_ptr());
        let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
        assert_eq!(cstr.to_str().unwrap(), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_md5_multi_update() {
        let mut ctx = Md5Context {
            total: [0, 0],
            state: [0, 0, 0, 0],
            buffer: [0; 64],
        };
        fceux11_rust_md5_starts(&mut ctx);
        fceux11_rust_md5_update(&mut ctx, b"The quick ".as_ptr(), 10);
        fceux11_rust_md5_update(&mut ctx, b"brown fox jumps over the lazy dog".as_ptr(), 33);
        let mut digest = [0u8; 16];
        fceux11_rust_md5_finish(&mut ctx, digest.as_mut_ptr());
        let expected = crate_md5_compute(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(digest, expected);
    }

    #[test]
    fn test_md5_large_input() {
        // 1 MiB of zeros — stresses multi-block path
        let data = vec![0u8; 1024 * 1024];
        let digest = rust_md5_compute(&data);
        let expected = crate_md5_compute(&data);
        assert_eq!(digest, expected);
    }

    #[test]
    fn test_md5_chunked_update() {
        // Deterministic pseudo-random data using LCG (no external crate)
        let mut data = vec![0u8; 4096];
        let mut seed: u32 = 12345;
        for b in data.iter_mut() {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            *b = (seed >> 16) as u8;
        }

        let expected = crate_md5_compute(&data);

        // Incremental updates with varying chunk sizes
        let mut ctx = Md5Context {
            total: [0, 0],
            state: [0, 0, 0, 0],
            buffer: [0; 64],
        };
        fceux11_rust_md5_starts(&mut ctx);
        let chunk_sizes: [usize; 12] = [1, 3, 7, 15, 31, 63, 127, 255, 511, 1023, 2047, 4096];
        for &chunk_size in &chunk_sizes {
            let mut offset = 0usize;
            let mut ctx2 = Md5Context {
                total: [0, 0],
                state: [0, 0, 0, 0],
                buffer: [0; 64],
            };
            fceux11_rust_md5_starts(&mut ctx2);
            while offset < data.len() {
                let end = (offset + chunk_size).min(data.len());
                fceux11_rust_md5_update(&mut ctx2, data[offset..end].as_ptr(), (end - offset) as u32);
                offset = end;
            }
            let mut digest = [0u8; 16];
            fceux11_rust_md5_finish(&mut ctx2, digest.as_mut_ptr());
            assert_eq!(digest, expected, "mismatch for chunk_size={}", chunk_size);
        }
    }
}
