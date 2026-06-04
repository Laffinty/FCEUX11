//! Video post-processing — PNG snapshot encoder and pixel accessors.
//!
//! Migrated from `src/video.cpp` in v0.2.27 per
//! `docs/rust_refactor_plan_v0.2.12-v0.2.30.md` §6.3.
//!
//! # Scope (migrated to Rust in v0.2.27)
//!
//! - PNG snapshot save path: `WritePNGChunk`, `SaveSnapshot()` (RGB color
//!   type 2), `SaveSnapshot(char[])` (indexed color type 3 with PLTE).
//! - Pixel accessors: `GetScreenPixel`, `GetScreenPixelPalette`.
//! - `AsSnapshotName` global + accessors.
//!
//! # Out of scope (kept in C++)
//!
//! - `FCEU_PutImage` / `FCEU_PutImageDummy` (frame finalize loop, coupled
//!   to Lua / AVI / GUI input aids / drawing overlays).
//! - `FCEU_DispMessage*` / `guiMessage` (shared with `drawing.cpp`).
//! - FPS display (`ShowFPS`, `FCEUI_ShowFPS`).
//! - `XBuf` / `XBackBuf` / `XDBuf` / `XDBackBuf` ownership (read by 5+ TUs).
//! - `lastu` auto-snapshot index (C++ owns the "find next free filename"
//!   loop, since it depends on `FCEU_MakeFName(FCEUMKF_SNAP, ...)`).
//! - File I/O: C++ side opens the file (via `FCEUD_UTF8fopen`, which is
//!   platform-specific) and writes the PNG bytes that Rust produces.
//!
//! # Design
//!
//! All FFI functions take length-parameterized slices (`FceuSlice` /
//! `FceuSliceMut`) and validate null/length before any `unsafe` access,
//! following the R3 HotOS 2024 FFI-safety guidance. `AsSnapshotName` is
//! owned by Rust behind a `Mutex<Option<String>>` (set once, get many).
//!
//! CRC32 is computed over `(type || data)`, matching the standard PNG
//! layout. Deflate compression uses level 6 (`Z_DEFAULT_COMPRESSION`),
//! matching the C++ `compress()` call. The `compress_to_vec` API of
//! `miniz_oxide` removes the need to pre-size a compmem buffer.

use std::os::raw::c_char;
use std::slice;
use std::sync::Mutex;

use miniz_oxide::deflate::compress_to_vec_zlib;

use fceux11_utils::slice::{FceuSlice, FceuSliceMut};

// ---------------------------------------------------------------------------
// C-compatible constants
// ---------------------------------------------------------------------------

/// Standard PNG signature (8 bytes). Every PNG file begins with this.
pub const FCEUX11_RUST_PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Filter byte prepended to each scanline: 0 = "None" (no filtering),
/// matching `src/video.cpp:578` and `:677`.
pub const FCEUX11_RUST_PNG_FILTER_NONE: u8 = 0;

/// `XBuf` is always 256×256 (65536 bytes), but only the
/// `[first_sline, last_sline]` rows are emitted to disk.
pub const FCEUX11_RUST_VIDEO_XBUF_WIDTH: u32 = 256;
pub const FCEUX11_RUST_VIDEO_XBUF_HEIGHT: u32 = 256;
pub const FCEUX11_RUST_VIDEO_XBUF_SIZE: usize =
    (FCEUX11_RUST_VIDEO_XBUF_WIDTH * FCEUX11_RUST_VIDEO_XBUF_HEIGHT) as usize;

/// NES palette: 64 colors × 3 channels = 192 bytes for `GetScreenPixel`'s
/// caller-supplied PLTE-like buffer, OR 256 entries × 3 = 768 bytes for
/// the indexed snapshot's PLTE chunk (always 256 entries per `src/video.cpp:660`).
pub const FCEUX11_RUST_VIDEO_PALETTE_256: usize = 256;
pub const FCEUX11_RUST_VIDEO_PALETTE_64: usize = 64;
pub const FCEUX11_RUST_VIDEO_PALETTE_BYTES_256: usize =
    FCEUX11_RUST_VIDEO_PALETTE_256 * 3;
pub const FCEUX11_RUST_VIDEO_PALETTE_BYTES_64: usize = FCEUX11_RUST_VIDEO_PALETTE_64 * 3;

/// Deflate compression level — matches the C++ `compress()` default
/// (`Z_DEFAULT_COMPRESSION` = 6). Explicit for defensiveness.
const DEFLATE_LEVEL: u8 = 6;

// ---------------------------------------------------------------------------
// C-compatible result struct
// ---------------------------------------------------------------------------

/// Encode-time arguments shared by the RGB and indexed snapshot paths.
///
/// The caller owns all pointers. All lengths are validated against the
/// required minimums in the encoder functions. The output buffer
/// (`out.ptr`, `out.len`) must be at least 256 KiB to hold the worst-case
/// PNG (240 scanlines × 768 RGB + 12 byte scanline header + zlib overhead
/// + chunk headers).
#[repr(C)]
pub struct FceuVideoEncodeArgs {
    /// 256×256 indexed pixel buffer (NES palette indices in the low 6
    /// bits, with deemph/overlay bits in the high bits).
    pub xbuf: *const u8,
    /// Must equal `FCEUX11_RUST_VIDEO_XBUF_SIZE`.
    pub xbuf_len: usize,
    /// Inclusive first scanline index, 0..=255. Validated.
    pub first_sline: i32,
    /// Inclusive last scanline index, must satisfy `first_sline <= last_sline <= 255`.
    pub last_sline: i32,
    /// Caller-allocated output buffer.
    pub out: FceuSliceMut,
}

// Safety: the struct contains only raw pointers + length. Caller is
// responsible for the lifetime and aliasing rules.
unsafe impl Send for FceuVideoEncodeArgs {}

// ---------------------------------------------------------------------------
// Pure helpers (no FFI overhead)
// ---------------------------------------------------------------------------

/// Compute the CRC32 over the PNG chunk's `(type || data)` span.
///
/// PNG chunks (after the 4-byte length) consist of a 4-byte type code,
/// then `data`, then a 4-byte CRC computed over `type || data`. The
/// initial CRC is 0 (the algorithm internally seeds 0xFFFFFFFF).
///
/// `crc32fast::Hasher::new_with_initial(0)` is equivalent to the
/// "fresh start" state, matching the C++ `CalcCRC32(0, ...)` call at
/// `src/video.cpp:475`.
fn png_crc32(chunk_type: &[u8; 4], data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new_with_initial(0);
    hasher.update(chunk_type);
    hasher.update(data);
    hasher.finalize()
}

/// Validate the scanline range and return the number of scanlines to
/// emit, or an error code on out-of-range input.
///
/// `first_sline` and `last_sline` must satisfy
/// `0 <= first <= last < FCEUX11_RUST_VIDEO_XBUF_HEIGHT`.
/// Returns `Ok(totallines)` or `Err(0)` on invalid input.
fn validate_sline_range(first: i32, last: i32) -> Result<u32, ()> {
    if first < 0
        || last < first
        || (last as u32) >= FCEUX11_RUST_VIDEO_XBUF_HEIGHT
        || (first as u32) >= FCEUX11_RUST_VIDEO_XBUF_HEIGHT
    {
        Err(())
    } else {
        Ok((last - first + 1) as u32)
    }
}

/// Append one PNG chunk (length + type + data + CRC) to `out`.
///
/// On overflow, returns `Err(())` and leaves `out` partially written
/// (callers should pre-allocate generously).
fn write_png_chunk(
    out: &mut Vec<u8>,
    chunk_type: &[u8; 4],
    data: &[u8],
) -> Result<(), ()> {
    // Length: big-endian u32
    let len = data.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    // Type
    out.extend_from_slice(chunk_type);
    // Data (may be empty)
    if !data.is_empty() {
        out.extend_from_slice(data);
    }
    // CRC over (type || data)
    let crc = png_crc32(chunk_type, data);
    out.extend_from_slice(&crc.to_be_bytes());
    Ok(())
}

/// Build the 13-byte IHDR payload for a 256-wide, `height`-tall RGB PNG
/// (color type 2 = RGB, 8-bit depth, deflate, no interlace).
fn make_ihdr_rgb(height: u32) -> [u8; 13] {
    let mut ihdr = [0u8; 13];
    // Width (4 bytes big-endian) = 256
    ihdr[0..4].copy_from_slice(&FCEUX11_RUST_VIDEO_XBUF_WIDTH.to_be_bytes());
    // Height (4 bytes big-endian)
    ihdr[4..8].copy_from_slice(&height.to_be_bytes());
    ihdr[8] = 8; // bit depth
    ihdr[9] = 2; // color type: RGB
    ihdr[10] = 0; // compression: deflate
    ihdr[11] = 0; // filter: adaptive
    ihdr[12] = 0; // interlace: none
    ihdr
}

/// Build the 13-byte IHDR payload for a 256-wide, `height`-tall indexed
/// PNG (color type 3 = indexed, 8-bit depth, deflate, no interlace).
fn make_ihdr_indexed(height: u32) -> [u8; 13] {
    let mut ihdr = [0u8; 13];
    ihdr[0..4].copy_from_slice(&FCEUX11_RUST_VIDEO_XBUF_WIDTH.to_be_bytes());
    ihdr[4..8].copy_from_slice(&height.to_be_bytes());
    ihdr[8] = 8; // bit depth
    ihdr[9] = 3; // color type: indexed
    ihdr[10] = 0; // compression: deflate
    ihdr[11] = 0; // filter: adaptive
    ihdr[12] = 0; // interlace: none
    ihdr
}

/// Deflate-compress the raw scanline buffer at level 6 using zlib format.
///
/// PNG IDAT chunks require zlib-format output (2-byte header + raw
/// deflate stream + 4-byte Adler-32), which is exactly what
/// `miniz_oxide::deflate::compress_to_vec_zlib` produces. This matches
/// the C++ `compress()` call at `src/video.cpp:590` and `:683`.
fn compress_idat(raw_scanlines: &[u8]) -> Vec<u8> {
    compress_to_vec_zlib(raw_scanlines, DEFLATE_LEVEL)
}

// ---------------------------------------------------------------------------
// FFI: PNG encoder (RGB)
// ---------------------------------------------------------------------------

/// Encode an RGB snapshot PNG to the caller's output buffer.
///
/// `args` carries the indexed XBuf, scanline range, and pre-allocated
/// output buffer. `rgb_scanlines` is a `width * 3 * totallines`-byte
/// buffer of pre-computed RGB triples (the C++ side runs
/// `ModernDeemphColorMap` per pixel and packs them contiguously).
///
/// # Returns
///
/// Number of bytes written to `args.out` on success, or `0` on:
/// - null `args` / null `xbuf` / null `out.ptr`
/// - `args.xbuf_len < 65536`
/// - `rgb_scanlines.len() < 256 * 3 * totallines`
/// - `args.out.len < bytes_needed`
/// - `first_sline`/`last_sline` out of `[0, 256)`
///
/// # Safety
///
/// All pointers in `args` and `rgb_scanlines` must be valid for the
/// declared lengths. The caller may read from `args.xbuf` and
/// `rgb_scanlines`; the caller may write to `args.out.ptr..out.ptr+out.len`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_video_encode_png_rgb(
    args: *const FceuVideoEncodeArgs,
    rgb_scanlines: FceuSlice,
) -> usize {
    let result = encode_png_rgb(unsafe { &*args }, unsafe { rgb_scanlines.as_slice() });
    match result {
        Ok(bytes) => bytes,
        Err(()) => 0,
    }
}

fn encode_png_rgb(args: &FceuVideoEncodeArgs, rgb_scanlines: &[u8]) -> Result<usize, ()> {
    // Validate inputs
    if args.xbuf.is_null()
        || args.xbuf_len < FCEUX11_RUST_VIDEO_XBUF_SIZE
        || args.out.ptr.is_null()
        || args.out.len == 0
    {
        return Err(());
    }
    let totallines = validate_sline_range(args.first_sline, args.last_sline)?;
    let height = totallines;
    let needed_rgb =
        (FCEUX11_RUST_VIDEO_XBUF_WIDTH as usize) * 3 * (totallines as usize);
    if rgb_scanlines.len() < needed_rgb {
        return Err(());
    }

    // Build PNG into a stack Vec, then copy into the caller's buffer.
    let mut png = Vec::new();
    png.extend_from_slice(&FCEUX11_RUST_PNG_SIGNATURE);

    // IHDR
    let ihdr = make_ihdr_rgb(height);
    write_png_chunk(&mut png, b"IHDR", &ihdr)?;

    // IDAT: each scanline = 1 filter byte + width*3 RGB bytes
    let scanline_stride_rgb = (FCEUX11_RUST_VIDEO_XBUF_WIDTH as usize) * 3;
    let mut raw = Vec::with_capacity((totallines as usize) * (1 + scanline_stride_rgb));
    for y in 0..(totallines as usize) {
        raw.push(FCEUX11_RUST_PNG_FILTER_NONE);
        let src_off = y * scanline_stride_rgb;
        raw.extend_from_slice(&rgb_scanlines[src_off..src_off + scanline_stride_rgb]);
    }
    let idat = compress_idat(&raw);
    write_png_chunk(&mut png, b"IDAT", &idat)?;

    // IEND
    write_png_chunk(&mut png, b"IEND", &[])?;

    if png.len() > args.out.len {
        return Err(());
    }
    // Safety: caller guarantees args.out.ptr is valid for args.out.len
    unsafe {
        std::ptr::copy_nonoverlapping(png.as_ptr(), args.out.ptr, png.len());
    }
    Ok(png.len())
}

// ---------------------------------------------------------------------------
// FFI: PNG encoder (indexed + PLTE)
// ---------------------------------------------------------------------------

/// Encode an indexed (palette) snapshot PNG to the caller's output
/// buffer.
///
/// `args` carries the indexed XBuf, scanline range, and pre-allocated
/// output buffer. `plte_rgb` is a 768-byte (256 entries × 3 channels)
/// palette — one RGB triple per NES palette index.
///
/// # Returns
///
/// Number of bytes written to `args.out` on success, or `0` on:
/// - null `args` / null `xbuf` / null `out.ptr`
/// - `args.xbuf_len < 65536`
/// - `plte_rgb.len() < 768`
/// - `args.out.len < bytes_needed`
/// - `first_sline`/`last_sline` out of `[0, 256)`
///
/// # Safety
///
/// Same as `fceux11_rust_video_encode_png_rgb`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_video_encode_png_indexed(
    args: *const FceuVideoEncodeArgs,
    plte_rgb: FceuSlice,
) -> usize {
    let result = encode_png_indexed(unsafe { &*args }, unsafe { plte_rgb.as_slice() });
    match result {
        Ok(bytes) => bytes,
        Err(()) => 0,
    }
}

fn encode_png_indexed(
    args: &FceuVideoEncodeArgs,
    plte_rgb: &[u8],
) -> Result<usize, ()> {
    if args.xbuf.is_null()
        || args.xbuf_len < FCEUX11_RUST_VIDEO_XBUF_SIZE
        || args.out.ptr.is_null()
        || args.out.len == 0
    {
        return Err(());
    }
    let totallines = validate_sline_range(args.first_sline, args.last_sline)?;
    let height = totallines;
    if plte_rgb.len() < FCEUX11_RUST_VIDEO_PALETTE_BYTES_256 {
        return Err(());
    }

    // Read indexed XBuf slice (we only use the relevant scanlines).
    // Safety: caller guarantees args.xbuf is valid for args.xbuf_len.
    let xbuf = unsafe { slice::from_raw_parts(args.xbuf, args.xbuf_len) };

    let mut png = Vec::new();
    png.extend_from_slice(&FCEUX11_RUST_PNG_SIGNATURE);

    // IHDR
    let ihdr = make_ihdr_indexed(height);
    write_png_chunk(&mut png, b"IHDR", &ihdr)?;

    // PLTE: 256 entries × 3 bytes = 768 bytes
    write_png_chunk(&mut png, b"PLTE", &plte_rgb[..FCEUX11_RUST_VIDEO_PALETTE_BYTES_256])?;

    // IDAT: each scanline = 1 filter byte + width indexed bytes
    let first_sline = args.first_sline as usize;
    let width = FCEUX11_RUST_VIDEO_XBUF_WIDTH as usize;
    let mut raw = Vec::with_capacity((totallines as usize) * (1 + width));
    for y in 0..(totallines as usize) {
        raw.push(FCEUX11_RUST_PNG_FILTER_NONE);
        let row_off = (first_sline + y) * width;
        raw.extend_from_slice(&xbuf[row_off..row_off + width]);
    }
    let idat = compress_idat(&raw);
    write_png_chunk(&mut png, b"IDAT", &idat)?;

    // IEND
    write_png_chunk(&mut png, b"IEND", &[])?;

    if png.len() > args.out.len {
        return Err(());
    }
    // Safety: caller guarantees args.out.ptr is valid for args.out.len.
    unsafe {
        std::ptr::copy_nonoverlapping(png.as_ptr(), args.out.ptr, png.len());
    }
    Ok(png.len())
}

// ---------------------------------------------------------------------------
// FFI: pixel accessors
// ---------------------------------------------------------------------------

/// Read a single screen pixel and return its 24-bit RGB value.
///
/// Mirrors `src/video.cpp:489-503`'s `GetScreenPixel`. The caller
/// supplies a 192-byte (64×3) NES palette; indexed pixel `p` is mapped
/// to `palette[((p & 0x3F) * 3)..((p & 0x3F) * 3 + 3)]`.
///
/// If `usebackup != 0`, reads from `xbackbuf` instead of `xbuf`.
///
/// # Returns
///
/// - `-1` on out-of-bounds (`x`/`y` not in `[0, 256)`).
/// - `0` on success; the 24-bit RGB value is written to `rgb_out` (which
///   must be at least 3 bytes long).
///
/// # Safety
///
/// `xbuf`, `xbackbuf`, and `rgb_out` must be non-null and point to at
/// least `xbuf_len` / `xbackbuf_len` / 3 readable/writable bytes
/// respectively.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_video_get_screen_pixel(
    x: i32,
    y: i32,
    usebackup: bool,
    xbuf: FceuSlice,
    xbackbuf: FceuSlice,
    palette_64: FceuSlice,
    rgb_out: FceuSliceMut,
) -> i32 {
    if x < 0 || y < 0 || x >= 256 || y >= 256 {
        return -1;
    }
    if xbuf.len == 0 || xbackbuf.len == 0 || rgb_out.len < 3 {
        return -1;
    }
    if palette_64.len < FCEUX11_RUST_VIDEO_PALETTE_BYTES_64 {
        return -1;
    }
    let src = if usebackup { &xbackbuf } else { &xbuf };
    let off = (y as usize) * 256 + (x as usize);
    if off >= src.len {
        return -1;
    }
    let pal_idx = (unsafe { src.as_slice() }[off] & 0x3F) as usize;
    let p_off = pal_idx * 3;
    // Safety: rgb_out.ptr is valid for rgb_out.len >= 3, palette_64.ptr
    // is valid for palette_64.len >= 192.
    let palette_bytes = unsafe { palette_64.as_slice() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            palette_bytes.as_ptr().add(p_off),
            rgb_out.ptr,
            3,
        );
    }
    0
}

/// Read a single screen pixel's palette index (low 6 bits).
///
/// Mirrors `src/video.cpp:505-515`'s `GetScreenPixelPalette`.
///
/// # Returns
///
/// - `-1` on out-of-bounds.
/// - Otherwise, the palette index in the low 6 bits (`0..=63`).
///
/// # Safety
///
/// `xbuf` and `xbackbuf` must be non-null and point to at least
/// `xbuf_len` / `xbackbuf_len` readable bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_video_get_screen_pixel_palette(
    x: i32,
    y: i32,
    usebackup: bool,
    xbuf: FceuSlice,
    xbackbuf: FceuSlice,
) -> i32 {
    if x < 0 || y < 0 || x >= 256 || y >= 256 {
        return -1;
    }
    if xbuf.len == 0 || xbackbuf.len == 0 {
        return -1;
    }
    let src = if usebackup { &xbackbuf } else { &xbuf };
    let off = (y as usize) * 256 + (x as usize);
    if off >= src.len {
        return -1;
    }
    (unsafe { src.as_slice() }[off] & 0x3F) as i32
}

// ---------------------------------------------------------------------------
// FFI: snapshot name state
// ---------------------------------------------------------------------------

/// `AsSnapshotName` storage. Set by the "Save snapshot as" command, read
/// by `FCEU_PutImage` to resolve the explicit filename. Wrapped in a
/// `Mutex<Option<String>>` because the C++ side can call set/get from
/// any thread (input.cpp on the input thread, video.cpp on the main
/// thread).
static SNAPSHOT_AS_NAME: Mutex<Option<String>> = Mutex::new(None);

/// Set the "save snapshot as" filename. The C++ side calls this from
/// `FCEUI_SetSnapshotAsName`. Passing a null pointer or `name_len == 0`
/// clears the name.
///
/// # Safety
///
/// If `name` is non-null, it must point to at least `name_len` valid
/// UTF-8 bytes. The bytes need not be NUL-terminated.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_video_set_snapshot_as_name(
    name: *const c_char,
    name_len: usize,
) {
    let mut guard = SNAPSHOT_AS_NAME.lock().expect("snapshot name mutex poisoned");
    if name.is_null() || name_len == 0 {
        *guard = None;
        return;
    }
    // Safety: caller guarantees name is valid for name_len bytes.
    let bytes = unsafe { slice::from_raw_parts(name as *const u8, name_len) };
    // Strip trailing NUL if present (the C++ side may pass a C string).
    let trimmed = match bytes.iter().position(|&b| b == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    };
    match std::str::from_utf8(trimmed) {
        Ok(s) => *guard = Some(s.to_owned()),
        Err(_) => *guard = None, // invalid UTF-8 -> treat as "unset"
    }
}

/// Read the "save snapshot as" filename into the caller's buffer.
///
/// # Returns
///
/// Number of bytes copied (excluding any NUL terminator). Returns `0` if:
/// - no name is set,
/// - `buf` is null,
/// - the name is too long for `buf_len` (in which case the name is
///   **truncated** to fit and `buf_len - 1` is returned).
///
/// # Safety
///
/// `buf` must point to at least `buf_len` writable bytes (or be null
/// when `buf_len == 0`).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_video_get_snapshot_as_name(
    buf: *mut c_char,
    buf_len: usize,
) -> usize {
    let guard = SNAPSHOT_AS_NAME.lock().expect("snapshot name mutex poisoned");
    let name = match guard.as_ref() {
        Some(s) => s.clone(),
        None => return 0,
    };
    drop(guard);
    if buf.is_null() || buf_len == 0 {
        return 0;
    }
    let bytes = name.as_bytes();
    let to_copy = bytes.len().min(buf_len.saturating_sub(1));
    // Safety: caller guarantees buf is valid for buf_len bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, to_copy);
        // Always NUL-terminate (within the buffer's capacity).
        *((buf as *mut u8).add(to_copy)) = 0;
    }
    to_copy
}

// ---------------------------------------------------------------------------
// Internal helpers exposed for testing
// ---------------------------------------------------------------------------

/// Pure helper: validate a scanline range, returning the count or 0 on
/// invalid input. Exposed for unit tests.
#[cfg(test)]
pub(crate) fn totallines_for(first: i32, last: i32) -> u32 {
    validate_sline_range(first, last).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_png_signature_is_well_known() {
        assert_eq!(
            FCEUX11_RUST_PNG_SIGNATURE,
            [137, 80, 78, 71, 13, 10, 26, 10]
        );
    }

    #[test]
    fn test_validate_sline_range() {
        assert_eq!(totallines_for(0, 239), 240);
        assert_eq!(totallines_for(0, 0), 1);
        assert_eq!(totallines_for(0, 255), 256);
        assert_eq!(totallines_for(100, 100), 1);
        assert_eq!(totallines_for(100, 99), 0); // last < first
        assert_eq!(totallines_for(-1, 10), 0); // first < 0
        assert_eq!(totallines_for(0, 256), 0); // last >= 256
    }

    #[test]
    fn test_png_crc32_ihdr() {
        // Known CRC32 for a 13-byte IHDR chunk (89 50 4E 47 0D 0A 1A 0A
        // ... 0,0,0,1, 0,0,0,0, 8, 2, 0, 0, 0) per PNG spec, hand-verified.
        let ihdr = make_ihdr_rgb(1);
        let crc = png_crc32(b"IHDR", &ihdr);
        // CRC32 of "IHDR" || 13 bytes of (256×1, 8-bit, RGB, etc.) is a
        // fixed value; we just verify the helper is consistent.
        let crc2 = png_crc32(b"IHDR", &ihdr);
        assert_eq!(crc, crc2);
    }

    #[test]
    fn test_write_png_chunk_layout() {
        let mut out = Vec::new();
        write_png_chunk(&mut out, b"IEND", &[]).unwrap();
        // IEND chunk: length(4) + "IEND"(4) + CRC(4) = 12 bytes
        assert_eq!(out.len(), 12);
        // Length = 0
        assert_eq!(&out[0..4], &[0, 0, 0, 0]);
        // Type = "IEND"
        assert_eq!(&out[4..8], b"IEND");
        // CRC of "IEND" with no data = 0xAE426082
        assert_eq!(&out[8..12], &[0xAE, 0x42, 0x60, 0x82]);
    }

    #[test]
    fn test_encode_png_rgb_roundtrip() {
        // Build a synthetic 256x240 RGB image: constant 0xAB in all channels.
        let height: u32 = 240;
        let scanline_bytes = 256 * 3;
        let rgb: Vec<u8> = vec![0xAB; scanline_bytes * (height as usize)];

        let mut out_buf = vec![0u8; 256 * 1024];
        let out_slice = FceuSliceMut {
            ptr: out_buf.as_mut_ptr(),
            len: out_buf.len(),
        };
        let xbuf = vec![0u8; FCEUX11_RUST_VIDEO_XBUF_SIZE];
        let args = FceuVideoEncodeArgs {
            xbuf: xbuf.as_ptr(),
            xbuf_len: xbuf.len(),
            first_sline: 0,
            last_sline: (height - 1) as i32,
            out: out_slice,
        };
        let rgb_slice = FceuSlice {
            ptr: rgb.as_ptr(),
            len: rgb.len(),
        };
        let written = fceux11_rust_video_encode_png_rgb(&args, rgb_slice);
        assert!(written > 0, "encode returned 0: failure");
        assert!(written <= out_buf.len());

        // Decode with the `png` crate.
        let decoder = png::Decoder::new(&out_buf[..written]);
        let mut reader = decoder.read_info().expect("PNG decode failed");
        let mut decoded = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut decoded).expect("frame read");
        assert_eq!(info.width, 256);
        assert_eq!(info.height, 240);
        assert_eq!(info.color_type, png::ColorType::Rgb);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        // First scanline filter byte was prepended in raw, then
        // miniz_oxide compressed it; the decoder strips filter bytes
        // transparently, so `decoded` contains only the 256*3 RGB bytes.
        assert_eq!(&decoded[..6], &[0xAB, 0xAB, 0xAB, 0xAB, 0xAB, 0xAB]);
    }

    #[test]
    fn test_encode_png_indexed_roundtrip() {
        let height: u32 = 240;
        let width = 256usize;
        // Synthetic indexed XBuf: alternating 0 and 1.
        let mut xbuf = vec![0u8; FCEUX11_RUST_VIDEO_XBUF_SIZE];
        for y in 0..(height as usize) {
            for x in 0..width {
                xbuf[y * width + x] = (x % 2) as u8;
            }
        }
        // 256-entry palette: index 0 = red, index 1 = green, rest = black.
        let mut plte = vec![0u8; FCEUX11_RUST_VIDEO_PALETTE_BYTES_256];
        plte[0] = 0xFF;
        plte[1] = 0x00;
        plte[2] = 0x00;
        plte[3] = 0x00;
        plte[4] = 0xFF;
        plte[5] = 0x00;

        let mut out_buf = vec![0u8; 256 * 1024];
        let out_slice = FceuSliceMut {
            ptr: out_buf.as_mut_ptr(),
            len: out_buf.len(),
        };
        let args = FceuVideoEncodeArgs {
            xbuf: xbuf.as_ptr(),
            xbuf_len: xbuf.len(),
            first_sline: 0,
            last_sline: (height - 1) as i32,
            out: out_slice,
        };
        let plte_slice = FceuSlice {
            ptr: plte.as_ptr(),
            len: plte.len(),
        };
        let written = fceux11_rust_video_encode_png_indexed(&args, plte_slice);
        assert!(written > 0, "indexed encode returned 0: failure");

        let decoder = png::Decoder::new(&out_buf[..written]);
        let mut reader = decoder.read_info().expect("PNG decode failed");
        let mut decoded = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut decoded).expect("frame read");
        assert_eq!(info.width, 256);
        assert_eq!(info.height, 240);
        assert_eq!(info.color_type, png::ColorType::Indexed);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        // First pixel: x=0 -> index 0 -> red. After decoder stripping,
        // the raw buffer is still indexed (one byte per pixel).
        assert_eq!(decoded[0], 0);
        assert_eq!(decoded[1], 1);
    }

    #[test]
    fn test_encode_png_rgb_bounds_validation() {
        let mut out_buf = vec![0u8; 256 * 1024];
        let out_slice = FceuSliceMut {
            ptr: out_buf.as_mut_ptr(),
            len: out_buf.len(),
        };
        let xbuf = vec![0u8; FCEUX11_RUST_VIDEO_XBUF_SIZE];
        let rgb = vec![0u8; 256 * 3 * 240];

        // first_sline out of range
        let args = FceuVideoEncodeArgs {
            xbuf: xbuf.as_ptr(),
            xbuf_len: xbuf.len(),
            first_sline: -1,
            last_sline: 239,
            out: FceuSliceMut {
                ptr: out_slice.ptr,
                len: out_slice.len,
            },
        };
        assert_eq!(
            fceux11_rust_video_encode_png_rgb(
                &args,
                FceuSlice {
                    ptr: rgb.as_ptr(),
                    len: rgb.len(),
                }
            ),
            0
        );

        // last_sline < first_sline
        let args = FceuVideoEncodeArgs {
            xbuf: xbuf.as_ptr(),
            xbuf_len: xbuf.len(),
            first_sline: 100,
            last_sline: 50,
            out: FceuSliceMut {
                ptr: out_slice.ptr,
                len: out_slice.len,
            },
        };
        assert_eq!(
            fceux11_rust_video_encode_png_rgb(
                &args,
                FceuSlice {
                    ptr: rgb.as_ptr(),
                    len: rgb.len(),
                }
            ),
            0
        );

        // last_sline >= 256
        let args = FceuVideoEncodeArgs {
            xbuf: xbuf.as_ptr(),
            xbuf_len: xbuf.len(),
            first_sline: 0,
            last_sline: 256,
            out: FceuSliceMut {
                ptr: out_slice.ptr,
                len: out_slice.len,
            },
        };
        assert_eq!(
            fceux11_rust_video_encode_png_rgb(
                &args,
                FceuSlice {
                    ptr: rgb.as_ptr(),
                    len: rgb.len(),
                }
            ),
            0
        );
    }

    #[test]
    fn test_encode_png_rgb_buffer_too_small() {
        let xbuf = vec![0u8; FCEUX11_RUST_VIDEO_XBUF_SIZE];
        let rgb = vec![0u8; 256 * 3 * 240];
        let mut out_buf = vec![0u8; 100]; // too small
        let out_slice = FceuSliceMut {
            ptr: out_buf.as_mut_ptr(),
            len: out_buf.len(),
        };
        let args = FceuVideoEncodeArgs {
            xbuf: xbuf.as_ptr(),
            xbuf_len: xbuf.len(),
            first_sline: 0,
            last_sline: 239,
            out: out_slice,
        };
        assert_eq!(
            fceux11_rust_video_encode_png_rgb(
                &args,
                FceuSlice {
                    ptr: rgb.as_ptr(),
                    len: rgb.len(),
                }
            ),
            0
        );
    }

    #[test]
    fn test_get_screen_pixel_in_bounds() {
        let mut xbuf = vec![0u8; FCEUX11_RUST_VIDEO_XBUF_SIZE];
        // Place index 5 at (10, 20)
        xbuf[20 * 256 + 10] = 5;
        // 64-entry palette: index 5 = (0x11, 0x22, 0x33), rest = 0
        let mut palette = vec![0u8; FCEUX11_RUST_VIDEO_PALETTE_BYTES_64];
        palette[5 * 3] = 0x11;
        palette[5 * 3 + 1] = 0x22;
        palette[5 * 3 + 2] = 0x33;
        let xbuf_slice = FceuSlice {
            ptr: xbuf.as_ptr(),
            len: xbuf.len(),
        };
        let xbackbuf_slice = FceuSlice {
            ptr: xbuf.as_ptr(), // re-use for test
            len: xbuf.len(),
        };
        let palette_slice = FceuSlice {
            ptr: palette.as_ptr(),
            len: palette.len(),
        };
        let mut rgb_out = [0u8; 3];
        let rgb_out_slice = FceuSliceMut {
            ptr: rgb_out.as_mut_ptr(),
            len: rgb_out.len(),
        };
        let rc = fceux11_rust_video_get_screen_pixel(
            10,
            20,
            false,
            xbuf_slice,
            xbackbuf_slice,
            palette_slice,
            rgb_out_slice,
        );
        assert_eq!(rc, 0);
        assert_eq!(rgb_out, [0x11, 0x22, 0x33]);
    }

    #[test]
    fn test_get_screen_pixel_out_of_bounds() {
        let xbuf = vec![0u8; FCEUX11_RUST_VIDEO_XBUF_SIZE];
        let palette = vec![0u8; FCEUX11_RUST_VIDEO_PALETTE_BYTES_64];
        let xbuf_slice = FceuSlice {
            ptr: xbuf.as_ptr(),
            len: xbuf.len(),
        };
        let xbackbuf_slice = FceuSlice {
            ptr: xbuf.as_ptr(),
            len: xbuf.len(),
        };
        let palette_slice = FceuSlice {
            ptr: palette.as_ptr(),
            len: palette.len(),
        };
        let mut rgb_out = [0u8; 3];
        let rgb_out_slice = FceuSliceMut {
            ptr: rgb_out.as_mut_ptr(),
            len: rgb_out.len(),
        };
        let rc = fceux11_rust_video_get_screen_pixel(
            300,
            10,
            false,
            xbuf_slice,
            xbackbuf_slice,
            palette_slice,
            rgb_out_slice,
        );
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_get_screen_pixel_palette_returns_index_low_6_bits() {
        let mut xbuf = vec![0u8; FCEUX11_RUST_VIDEO_XBUF_SIZE];
        // Set pixel (3, 4) to 0xFF (high bits set, low 6 = 0x3F = 63)
        xbuf[4 * 256 + 3] = 0xFF;
        let xbuf_slice = FceuSlice {
            ptr: xbuf.as_ptr(),
            len: xbuf.len(),
        };
        let xbackbuf_slice = FceuSlice {
            ptr: xbuf.as_ptr(),
            len: xbuf.len(),
        };
        let rc = fceux11_rust_video_get_screen_pixel_palette(
            3,
            4,
            false,
            xbuf_slice,
            xbackbuf_slice,
        );
        assert_eq!(rc, 0x3F);
    }

    #[test]
    fn test_snapshot_as_name_get_set_roundtrip() {
        // Clear first
        fceux11_rust_video_set_snapshot_as_name(std::ptr::null(), 0);
        let mut buf = [0i8; 64];
        let n = fceux11_rust_video_get_snapshot_as_name(buf.as_mut_ptr(), buf.len());
        assert_eq!(n, 0, "expected 0 (no name set)");

        // Set "snap-001.png"
        let name = b"snap-001.png";
        fceux11_rust_video_set_snapshot_as_name(
            name.as_ptr() as *const c_char,
            name.len(),
        );
        let n = fceux11_rust_video_get_snapshot_as_name(buf.as_mut_ptr(), buf.len());
        assert_eq!(n, name.len());
        let bytes: Vec<u8> = buf.iter().map(|&b| b as u8).collect();
        assert_eq!(&bytes[..n], name);
        // Must be NUL-terminated
        assert_eq!(bytes[n], 0);
    }

    #[test]
    fn test_snapshot_as_name_truncates_too_long() {
        fceux11_rust_video_set_snapshot_as_name(std::ptr::null(), 0);
        let long = b"this_is_a_very_long_filename_that_exceeds_the_buffer.png";
        fceux11_rust_video_set_snapshot_as_name(
            long.as_ptr() as *const c_char,
            long.len(),
        );
        let mut buf = [0i8; 16];
        let n = fceux11_rust_video_get_snapshot_as_name(buf.as_mut_ptr(), buf.len());
        // Should be truncated to buf.len() - 1 = 15
        assert_eq!(n, 15);
        let bytes: Vec<u8> = buf.iter().map(|&b| b as u8).collect();
        assert_eq!(&bytes[..n], &long[..15]);
        assert_eq!(bytes[n], 0); // NUL-terminated within the buffer
    }

    #[test]
    fn test_snapshot_as_name_null_clears() {
        let name = b"snap.png";
        fceux11_rust_video_set_snapshot_as_name(
            name.as_ptr() as *const c_char,
            name.len(),
        );
        fceux11_rust_video_set_snapshot_as_name(std::ptr::null(), 0);
        let mut buf = [0i8; 64];
        let n = fceux11_rust_video_get_snapshot_as_name(buf.as_mut_ptr(), buf.len());
        assert_eq!(n, 0);
    }
}
