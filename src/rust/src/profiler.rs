use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Mutex;

/// Per-thread profiler function map.
///
/// Replaces the C++ `std::map<std::string, funcProfileRecord*>` and the
/// call-stack `std::vector` with a Rust `HashMap` and `Vec`.
///
/// # Thread-safety note
/// Each instance is `thread_local` in C++; no concurrent access from multiple
/// threads is expected for a single map, so the map and stack are not wrapped
/// in a mutex.  The global `PROFILER_MANAGER` mutex protects the list of maps.
pub struct ProfilerFuncMap {
    records: HashMap<String, *mut c_void>,
    stack: Vec<*mut c_void>,
    /// Snapshot of keys for C-style iteration (`iterateBegin` / `iterateNext`).
    iter_keys: Vec<String>,
    iter_index: usize,
}

/// Global profiler-manager state. Replaces C++ `std::list<profilerFuncMap*>`.
struct ProfilerManagerState {
    maps: Vec<*mut c_void>,
}

// SAFETY: The pointers stored in `maps` are opaque C++ `this` pointers.
// They are never dereferenced or sent to other threads for mutation.
unsafe impl Send for ProfilerManagerState {}
unsafe impl Sync for ProfilerManagerState {}

static PROFILER_MANAGER: Mutex<ProfilerManagerState> = Mutex::new(ProfilerManagerState {
    maps: Vec::new(),
});

/// C ABI: Create a new `ProfilerFuncMap` and return an opaque handle.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_create() -> *mut c_void {
    let map = Box::new(ProfilerFuncMap {
        records: HashMap::new(),
        stack: Vec::new(),
        iter_keys: Vec::new(),
        iter_index: 0,
    });
    Box::into_raw(map) as *mut c_void
}

/// C ABI: Destroy a `ProfilerFuncMap` handle.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_destroy(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle as *mut ProfilerFuncMap);
        }
    }
}

/// C ABI: Register a `funcProfileRecord*` under the key `file:line`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_add_record(
    handle: *mut c_void,
    file: *const c_char,
    line: c_int,
    _func: *const c_char,
    _comment: *const c_char,
    rec: *mut c_void,
) -> c_int {
    if handle.is_null() || file.is_null() {
        return -1;
    }
    let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
    let file_str = unsafe { CStr::from_ptr(file).to_string_lossy() };
    let key = format!("{}:{}", file_str, line);
    map.records.insert(key, rec);
    0
}

/// C ABI: Push a record pointer onto the call stack.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_push_stack(handle: *mut c_void, rec: *mut c_void) {
    if !handle.is_null() {
        let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
        map.stack.push(rec);
    }
}

/// C ABI: Pop the top of the call stack.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_pop_stack(handle: *mut c_void, _rec: *mut c_void) {
    if !handle.is_null() {
        let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
        map.stack.pop();
    }
}

/// C ABI: Begin iteration over the map. Returns the first record, or null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_iterate_begin(handle: *mut c_void) -> *mut c_void {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
    map.iter_keys = map.records.keys().cloned().collect();
    map.iter_index = 0;
    if let Some(key) = map.iter_keys.get(0) {
        map.records.get(key).copied().unwrap_or(std::ptr::null_mut())
    } else {
        std::ptr::null_mut()
    }
}

/// C ABI: Advance iteration and return the next record, or null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_iterate_next(handle: *mut c_void) -> *mut c_void {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
    map.iter_index = map.iter_index.saturating_add(1);
    if let Some(key) = map.iter_keys.get(map.iter_index) {
        map.records.get(key).copied().unwrap_or(std::ptr::null_mut())
    } else {
        std::ptr::null_mut()
    }
}

/// C ABI: Add a C++ `profilerFuncMap*` to the global thread list.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_mgr_add(cpp_ptr: *mut c_void) -> c_int {
    let mut mgr = PROFILER_MANAGER.lock().unwrap();
    mgr.maps.push(cpp_ptr);
    0
}

/// C ABI: Remove a C++ `profilerFuncMap*` from the global thread list.
///
/// `should_destroy` is ignored; destruction is handled by C++.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_mgr_remove(
    cpp_ptr: *mut c_void,
    _should_destroy: c_int,
) -> c_int {
    let mut mgr = PROFILER_MANAGER.lock().unwrap();
    if let Some(pos) = mgr.maps.iter().position(|&x| x == cpp_ptr) {
        mgr.maps.remove(pos);
        0
    } else {
        -1
    }
}

/// C ABI: Clear the global thread list (called from `profilerManager` destructor).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_mgr_clear() {
    let mut mgr = PROFILER_MANAGER.lock().unwrap();
    mgr.maps.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_map_create_destroy() {
        let h = fceux11_rust_profiler_map_create();
        assert!(!h.is_null());
        fceux11_rust_profiler_map_destroy(h);
    }

    #[test]
    fn test_profiler_map_add_and_iterate() {
        let h = fceux11_rust_profiler_map_create();
        assert!(!h.is_null());

        let rec1: *mut c_void = 0x1234 as *mut c_void;
        let rec2: *mut c_void = 0x5678 as *mut c_void;

        fceux11_rust_profiler_map_add_record(
            h,
            b"file.cpp\0".as_ptr() as *const c_char,
            42,
            std::ptr::null(),
            std::ptr::null(),
            rec1,
        );
        fceux11_rust_profiler_map_add_record(
            h,
            b"other.cpp\0".as_ptr() as *const c_char,
            10,
            std::ptr::null(),
            std::ptr::null(),
            rec2,
        );

        let first = fceux11_rust_profiler_map_iterate_begin(h);
        assert!(first == rec1 || first == rec2);

        let second = fceux11_rust_profiler_map_iterate_next(h);
        assert!(second == rec1 || second == rec2);
        assert_ne!(first, second);

        let third = fceux11_rust_profiler_map_iterate_next(h);
        assert!(third.is_null());

        fceux11_rust_profiler_map_destroy(h);
    }

    #[test]
    fn test_profiler_map_stack() {
        let h = fceux11_rust_profiler_map_create();
        let rec1: *mut c_void = 0x1 as *mut c_void;
        let rec2: *mut c_void = 0x2 as *mut c_void;

        fceux11_rust_profiler_map_push_stack(h, rec1);
        fceux11_rust_profiler_map_push_stack(h, rec2);
        fceux11_rust_profiler_map_pop_stack(h, rec2);
        fceux11_rust_profiler_map_pop_stack(h, rec1);

        fceux11_rust_profiler_map_destroy(h);
    }

    #[test]
    fn test_profiler_mgr_add_remove() {
        let ptr1: *mut c_void = 0xABCD as *mut c_void;
        let ptr2: *mut c_void = 0xEF01 as *mut c_void;

        assert_eq!(fceux11_rust_profiler_mgr_add(ptr1), 0);
        assert_eq!(fceux11_rust_profiler_mgr_add(ptr2), 0);
        assert_eq!(fceux11_rust_profiler_mgr_remove(ptr1, 0), 0);
        assert_eq!(fceux11_rust_profiler_mgr_remove(ptr2, 0), 0);
        assert_eq!(fceux11_rust_profiler_mgr_remove(ptr1, 0), -1);

        fceux11_rust_profiler_mgr_clear();
    }
}
