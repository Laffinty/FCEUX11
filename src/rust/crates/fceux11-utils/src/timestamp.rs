use std::sync::OnceLock;
use std::time::Instant;

static BASE_INSTANT: OnceLock<Instant> = OnceLock::new();

/// Returns the base instant, lazily initialized on first call.
fn base_instant() -> &'static Instant {
    BASE_INSTANT.get_or_init(Instant::now)
}

/// C ABI: Return the current monotonic timestamp in nanoseconds.
///
/// The returned value is relative to an arbitrary epoch established
/// at program startup (or first call). It is guaranteed to be
/// monotonically increasing and suitable for high-precision delta
/// measurements.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_timestamp_now() -> u64 {
    let elapsed = Instant::now().duration_since(*base_instant());
    elapsed.as_nanos() as u64
}

/// C ABI: Return the timestamp frequency in Hz.
///
/// The Rust implementation uses nanosecond resolution, so the
/// frequency is always 1_000_000_000 Hz.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_timestamp_freq() -> u64 {
    1_000_000_000
}

/// C ABI: Initialize the timestamp module.
///
/// Ensures the base instant is captured. Returns 1 on success.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_timestamp_init() -> i32 {
    let _ = base_instant();
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_monotonic() {
        let t1 = fceux11_rust_timestamp_now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = fceux11_rust_timestamp_now();
        assert!(t2 > t1, "timestamp should be monotonically increasing");
    }

    #[test]
    fn test_timestamp_freq() {
        assert_eq!(fceux11_rust_timestamp_freq(), 1_000_000_000);
    }

    #[test]
    fn test_timestamp_init() {
        assert_eq!(fceux11_rust_timestamp_init(), 1);
    }

    #[test]
    fn test_timestamp_approximate_elapsed() {
        let start = fceux11_rust_timestamp_now();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let end = fceux11_rust_timestamp_now();
        let elapsed_ns = end - start;
        // Allow ±15 ms tolerance to account for scheduler jitter
        assert!(elapsed_ns >= 35_000_000, "elapsed too short: {} ns", elapsed_ns);
        assert!(elapsed_ns <= 200_000_000, "elapsed too long: {} ns", elapsed_ns);
    }
}
