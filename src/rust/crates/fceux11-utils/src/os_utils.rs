use std::ffi::c_char;
use std::time::Duration;

/// C ABI: Create a single directory.
///
/// Returns 0 on success, -1 on failure.
///
/// # Safety
/// `path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_mkdir(path: *const c_char) -> i32 {
    if path.is_null() {
        return -1;
    }
    let path_str = unsafe { std::ffi::CStr::from_ptr(path) }.to_string_lossy();
    match std::fs::create_dir(&*path_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// C ABI: Create a directory and all its parents.
///
/// Returns 0 on success, -1 on failure.
///
/// # Safety
/// `path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_mkpath(path: *const c_char) -> i32 {
    if path.is_null() {
        return -1;
    }
    let path_str = unsafe { std::ffi::CStr::from_ptr(path) }.to_string_lossy();
    match std::fs::create_dir_all(&*path_str) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// C ABI: Check whether a regular file exists.
///
/// Returns 1 if the path exists and is a file, 0 otherwise.
///
/// # Safety
/// `filepath` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_file_exists(filepath: *const c_char) -> i32 {
    if filepath.is_null() {
        return 0;
    }
    let path_str = unsafe { std::ffi::CStr::from_ptr(filepath) }.to_string_lossy();
    match std::fs::metadata(&*path_str) {
        Ok(m) => {
            if m.is_file() {
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

/// C ABI: Sleep for `ms` milliseconds.
///
/// Returns 0.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_msleep(ms: i32) -> i32 {
    if ms > 0 {
        std::thread::sleep(Duration::from_millis(ms as u64));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::fs;

    #[test]
    fn test_mkdir_and_file_exists() {
        let tmp = std::env::temp_dir().join("fceux11_rust_os_utils_test_dir");
        let _ = fs::remove_dir(&tmp);

        let path = CString::new(tmp.to_str().unwrap()).unwrap();
        assert_eq!(fceux11_rust_file_exists(path.as_ptr()), 0);
        assert_eq!(fceux11_rust_mkdir(path.as_ptr()), 0);
        // Directory is not a file
        assert_eq!(fceux11_rust_file_exists(path.as_ptr()), 0);
        let _ = fs::remove_dir(&tmp);
    }

    #[test]
    fn test_mkpath() {
        let base = std::env::temp_dir().join("fceux11_rust_os_utils_mkpath");
        let deep = base.join("a").join("b").join("c");
        let _ = fs::remove_dir_all(&base);

        let path = CString::new(deep.to_str().unwrap()).unwrap();
        assert_eq!(fceux11_rust_mkpath(path.as_ptr()), 0);
        assert!(deep.exists());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_mkpath_already_exists() {
        let base = std::env::temp_dir().join("fceux11_rust_os_utils_mkpath2");
        let _ = fs::remove_dir_all(&base);
        let _ = fs::create_dir_all(&base);

        let path = CString::new(base.to_str().unwrap()).unwrap();
        assert_eq!(fceux11_rust_mkpath(path.as_ptr()), 0);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_msleep() {
        let start = std::time::Instant::now();
        assert_eq!(fceux11_rust_msleep(50), 0);
        assert!(start.elapsed() >= std::time::Duration::from_millis(50));
    }

    #[test]
    fn test_null_safety() {
        assert_eq!(fceux11_rust_mkdir(std::ptr::null()), -1);
        assert_eq!(fceux11_rust_mkpath(std::ptr::null()), -1);
        assert_eq!(fceux11_rust_file_exists(std::ptr::null()), 0);
        assert_eq!(fceux11_rust_msleep(0), 0);
    }
}
