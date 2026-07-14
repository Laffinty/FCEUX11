use std::ffi::c_char;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Mutex;

struct WaveState {
    file: Option<File>,
    data_size: u32,
}

static WAVE_STATE: Mutex<WaveState> = Mutex::new(WaveState {
    file: None,
    data_size: 0,
});

/// Build a standard 44-byte PCM WAV header.
fn build_wav_header(sample_rate: u32, data_size: u32) -> [u8; 44] {
    let byte_rate = sample_rate * 2; // mono, 16-bit = 2 bytes per sample
    let riff_chunk_size = data_size + 36;

    let mut h = [0u8; 44];

    // RIFF chunk descriptor
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&riff_chunk_size.to_le_bytes());
    h[8..12].copy_from_slice(b"WAVE");

    // fmt sub-chunk
    h[12..16].copy_from_slice(b"fmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes()); // SubChunk1Size (PCM)
    h[20..22].copy_from_slice(&1u16.to_le_bytes()); // AudioFormat (PCM)
    h[22..24].copy_from_slice(&1u16.to_le_bytes()); // NumChannels (Mono)
    h[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    h[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    h[32..34].copy_from_slice(&2u16.to_le_bytes()); // BlockAlign
    h[34..36].copy_from_slice(&16u16.to_le_bytes()); // BitsPerSample

    // data sub-chunk
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_size.to_le_bytes());

    h
}

/// C ABI: Begin recording a wave file.
///
/// Opens (or truncates) `path` and writes a 44-byte PCM WAV header.
/// `sample_rate` is passed explicitly from C++ to avoid reading global state.
///
/// # Safety
/// `path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_wave_begin(path: *const c_char, sample_rate: u32) -> bool {
    if path.is_null() {
        return false;
    }
    let path_str = unsafe { std::ffi::CStr::from_ptr(path) }.to_string_lossy();

    let file = match File::create(&*path_str) {
        Ok(f) => f,
        Err(_) => return false,
    };

    // hotfix1 P1-14 (H-14): swallow lock poisoning on FFI entry — the
    // mutex can only be poisoned if a prior C++ caller panics inside a
    // Rust call, in which case we still want subsequent FFI requests to
    // make progress rather than abort the process.
    let mut state = match WAVE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.file = Some(file);
    state.data_size = 0;

    let header = build_wav_header(sample_rate, 0);
    if let Some(ref mut f) = state.file
        && f.write_all(&header).is_err()
    {
        state.file = None;
        return false;
    }

    true
}

/// C ABI: Returns whether a wave recording is currently active.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_wave_running() -> bool {
    match WAVE_STATE.lock() {
        Ok(g) => g.file.is_some(),
        Err(poisoned) => poisoned.into_inner().file.is_some(),
    }
}

/// C ABI: Write audio samples to the active wave file.
///
/// `buffer` points to `count` little-endian signed 16-bit samples.
/// Returns the number of bytes written, or 0 if no file is open.
///
/// # Safety
/// `buffer` must point to at least `count` valid `i16` samples, or be NULL when `count` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_wave_write(buffer: *const i16, count: i32) -> i64 {
    if count <= 0 || buffer.is_null() {
        return 0;
    }

    let mut state = match WAVE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let file = match state.file.as_mut() {
        Some(f) => f,
        None => return 0,
    };

    let samples = unsafe { std::slice::from_raw_parts(buffer, count as usize) };
    // Safe transmute: &[i16] -> &[u8] because i16 has no padding/invalid bit patterns for I/O
    let bytes = unsafe {
        std::slice::from_raw_parts(
            samples.as_ptr() as *const u8,
            std::mem::size_of_val(samples),
        )
    };

    match file.write_all(bytes) {
        Ok(()) => {
            state.data_size += bytes.len() as u32;
            bytes.len() as i64
        }
        Err(_) => -1,
    }
}

/// C ABI: Finalize and close the active wave file.
///
/// Seeks back to the header and updates the RIFF chunk size and data chunk size.
/// Returns 1 on success, 0 if no file was open.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_wave_end() -> i32 {
    let mut state = match WAVE_STATE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut file = match state.file.take() {
        Some(f) => f,
        None => return 0,
    };

    let data_size = state.data_size;
    let riff_chunk_size = data_size + 36;

    // Update RIFF chunk size at offset 4
    if file.seek(SeekFrom::Start(4)).is_err() {
        return 0;
    }
    if file.write_all(&riff_chunk_size.to_le_bytes()).is_err() {
        return 0;
    }

    // Update data chunk size at offset 40
    if file.seek(SeekFrom::Start(40)).is_err() {
        return 0;
    }
    if file.write_all(&data_size.to_le_bytes()).is_err() {
        return 0;
    }

    // File is implicitly closed when dropped
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::fs;

    #[test]
    fn test_wave_roundtrip() {
        unsafe {
            let tmp = std::env::temp_dir().join("fceux11_rust_wave_test.wav");
            let _ = fs::remove_file(&tmp);

            let path = CString::new(tmp.to_str().unwrap()).unwrap();
            assert!(fceux11_rust_wave_begin(path.as_ptr(), 48000));
            assert!(fceux11_rust_wave_running());

            let samples: [i16; 4] = [0, 16384, -16384, 32767];
            assert_eq!(
                fceux11_rust_wave_write(samples.as_ptr(), samples.len() as i32),
                8
            );

            assert_eq!(fceux11_rust_wave_end(), 1);
            assert!(!fceux11_rust_wave_running());

            let data = fs::read(&tmp).unwrap();
            assert_eq!(data.len(), 44 + 8);

            // Verify header fields
            assert_eq!(&data[0..4], b"RIFF");
            let riff_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            assert_eq!(riff_size, 36 + 8);
            assert_eq!(&data[8..12], b"WAVE");
            assert_eq!(&data[12..16], b"fmt ");
            let fmt_size = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
            assert_eq!(fmt_size, 16);
            let audio_format = u16::from_le_bytes([data[20], data[21]]);
            assert_eq!(audio_format, 1); // PCM
            let channels = u16::from_le_bytes([data[22], data[23]]);
            assert_eq!(channels, 1); // mono
            let rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
            assert_eq!(rate, 48000);
            let byte_rate = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);
            assert_eq!(byte_rate, 48000 * 2);
            let block_align = u16::from_le_bytes([data[32], data[33]]);
            assert_eq!(block_align, 2);
            let bits = u16::from_le_bytes([data[34], data[35]]);
            assert_eq!(bits, 16);
            assert_eq!(&data[36..40], b"data");
            let data_size = u32::from_le_bytes([data[40], data[41], data[42], data[43]]);
            assert_eq!(data_size, 8);

            // Verify sample bytes are little-endian
            assert_eq!(data[44..46], [0x00, 0x00]); // 0
            assert_eq!(data[46..48], [0x00, 0x40]); // 16384 = 0x4000
            assert_eq!(data[48..50], [0x00, 0xC0]); // -16384 = 0xC000
            assert_eq!(data[50..52], [0xFF, 0x7F]); // 32767 = 0x7FFF

            let _ = fs::remove_file(&tmp);
        }
    }

    #[test]
    fn test_wave_end_without_begin() {
        unsafe {
            // Ensure calling end without begin is safe and returns 0
            assert_eq!(fceux11_rust_wave_end(), 0);
        }
    }

    #[test]
    fn test_wave_write_without_begin() {
        unsafe {
            let samples: [i16; 2] = [100, 200];
            assert_eq!(fceux11_rust_wave_write(samples.as_ptr(), 2), 0);
        }
    }
}
