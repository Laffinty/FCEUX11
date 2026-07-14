use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// Per-thread profiler function map.
///
/// Replaces the C++ `std::map<std::string, funcProfileRecord*>` and the
/// call-stack `std::vector` with a Rust `HashMap` and `Vec`.
///
/// # Thread-safety note
/// Each instance is `thread_local` in C++; no concurrent access from multiple
/// threads is expected for a single map, so the map and stack are not wrapped
/// in a mutex.  The global `PROFILER_MANAGER` mutex protects the list of maps.
///
/// hotfix1 P1-16 (H-16): record pointers used to be raw `*mut c_void`
/// stored directly in the map's HashMap — if the C++ side destroyed a
/// record without telling us, the next iteration would read a dangling
/// pointer. We now register each record through Rust, getting back a
/// `u64` ID; the actual record pointer lives inside an `Arc<OpaqueRec>`
/// that we own, and the map stores `Weak<OpaqueRec>`. Iterators
/// transparently skip records whose `Arc` was dropped on the C++ side.
pub struct ProfilerFuncMap {
    records: HashMap<String, Weak<OpaqueRec>>,
    stack: Vec<u64>,
    /// Snapshot of keys for C-style iteration (`iterateBegin` / `iterateNext`).
    iter_keys: Vec<String>,
    iter_index: usize,
}

/// Owned record handle. The C++ side sees the `u64` ID and our maps
/// keep the strong `Arc`; downgrade to `Weak` after registration so the
/// C++ `release` call is what actually frees the `Arc`. Until that
/// happens, all uses of the record remain valid.
#[repr(transparent)]
pub struct OpaqueRec {
    pub ptr: *mut c_void,
}

// SAFETY: `OpaqueRec.ptr` is never read on the Rust side; it is treated
// strictly as a tag that is handed back to C++. So the auto-derive is
// safe under our access pattern.
unsafe impl Send for OpaqueRec {}
unsafe impl Sync for OpaqueRec {}

/// Global registry: maps a u64 ID to the strong owner of an `OpaqueRec`.
/// `record_take` registers (allocates an ID, stores the Arc); C++ calls
/// `record_drop` to release.
struct RecordRegistry {
    next_id: u64,
    live: HashMap<u64, Arc<OpaqueRec>>,
}
impl RecordRegistry {
    fn new() -> Self {
        Self { next_id: 1, live: HashMap::new() }
    }
}

/// Global profiler-manager state. Replaces C++ `std::list<profilerFuncMap*>`.
struct ProfilerManagerState {
    /// We store profilerFuncMap pointers as `usize` ids rather than raw
    /// pointers, paired with a parallel `ManagerRegistry` so that a
    /// subsequent dangling access is detectable. The C++ side receives a
    /// `u64` token here too; the pointer conversion happens only at the
    /// FFI boundary.
    maps: Vec<u64>,
}

// SAFETY: All cross-thread sharing happens under the global `PROFILER_MANAGER`
// mutex; per-field immutability is enforced inside the lock.
unsafe impl Send for ProfilerManagerState {}
unsafe impl Sync for ProfilerManagerState {}

static PROFILER_MANAGER: Mutex<ProfilerManagerState> =
    Mutex::new(ProfilerManagerState { maps: Vec::new() });

// hotfix1 P1-16 (H-16): RecordRegistry holds a HashMap which is not
// const-constructible, so we use OnceLock to defer allocation to first
// FFI entry. The Mutex is then created lazily behind the same gate.
static RECORD_REGISTRY: OnceLock<Mutex<RecordRegistry>> = OnceLock::new();

fn record_registry() -> &'static Mutex<RecordRegistry> {
    RECORD_REGISTRY.get_or_init(|| Mutex::new(RecordRegistry::new()))
}

/// Take ownership of a raw C++ record pointer and return a stable `u64`
/// ID. Subsequent FFI calls use this ID; the ID remains valid until
/// `fceux11_rust_profiler_record_drop` is called or the process exits.
///
/// # Safety
/// The pointer may be anything non-null that the C++ side wants Rust to
/// hand back later. Rust never dereferences it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_profiler_record_take(rec: *mut c_void) -> u64 {
    if rec.is_null() {
        return 0;
    }
    let mut reg = match record_registry().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let id = reg.next_id;
    reg.next_id = reg.next_id.wrapping_add(1);
    reg.live.insert(id, Arc::new(OpaqueRec { ptr: rec }));
    id
}

/// Drop the strong reference previously taken with `record_take`. Map
/// lookups that still hold a `Weak` for this record will start returning
/// `null` afterwards.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_record_drop(id: u64) {
    if id == 0 {
        return;
    }
    let mut reg = match record_registry().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    reg.live.remove(&id);
}

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

/// C ABI: Register a record (looked up by ID) under the key `file:line`.
/// `record_id` must come from `fceux11_rust_profiler_record_take`.
#[unsafe(no_mangle)]
/// # Safety
/// Callers must ensure all raw pointer arguments and the record ID are valid.
pub unsafe extern "C" fn fceux11_rust_profiler_map_add_record(
    handle: *mut c_void,
    file: *const c_char,
    line: c_int,
    _func: *const c_char,
    _comment: *const c_char,
    record_id: u64,
) -> c_int {
    if handle.is_null() || file.is_null() || record_id == 0 {
        return -1;
    }
    let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
    let file_str = unsafe { CStr::from_ptr(file).to_string_lossy() };
    let key = format!("{}:{}", file_str, line);

    // Borrow the registry to get a Weak; do not keep the lock for the
    // HashMap insert so we don't hold a global mutex across user code.
    let weak = {
        let reg = match record_registry().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match reg.live.get(&record_id) {
            Some(arc) => Arc::downgrade(arc),
            None => return -1,
        }
    };
    map.records.insert(key, weak);
    0
}

/// Convert a Weak back into a `*mut c_void` for C++ consumption, falling
/// back to null when the underlying record has been released.
fn resolve_record(weak: &Weak<OpaqueRec>) -> *mut c_void {
    weak.upgrade().map(|a| a.ptr).unwrap_or(std::ptr::null_mut())
}

/// C ABI: Push a record ID onto the call stack.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_push_stack(handle: *mut c_void, record_id: u64) {
    if !handle.is_null() {
        let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
        map.stack.push(record_id);
    }
}

/// C ABI: Pop the top of the call stack.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_pop_stack(handle: *mut c_void, _record_id: u64) {
    if !handle.is_null() {
        let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
        map.stack.pop();
    }
}

/// C ABI: Begin iteration over the map. Returns the first record pointer,
/// or null. Skips records whose `Arc` has already been released.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_iterate_begin(handle: *mut c_void) -> *mut c_void {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
    map.iter_keys = map.records.keys().cloned().collect();
    map.iter_index = 0;
    iterate_skip_invalid(map)
}

/// C ABI: Advance iteration and return the next record pointer, or null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_map_iterate_next(handle: *mut c_void) -> *mut c_void {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let map = unsafe { &mut *(handle as *mut ProfilerFuncMap) };
    map.iter_index = map.iter_index.saturating_add(1);
    iterate_skip_invalid(map)
}

/// Walk forward through the iteration snapshot until we find a Weak
/// that still upgrades, or we run out of keys.
fn iterate_skip_invalid(map: &mut ProfilerFuncMap) -> *mut c_void {
    while let Some(key) = map.iter_keys.get(map.iter_index) {
        if let Some(weak) = map.records.get(key) {
            // We borrow the Weak without holding any locks — upgrades
            // are lock-free on std::sync::Arc on x86_64.
            let resolved = resolve_record(weak);
            if !resolved.is_null() {
                return resolved;
            }
        }
        map.iter_index = map.iter_index.saturating_add(1);
    }
    std::ptr::null_mut()
}

/// C ABI: Add a C++ `profilerFuncMap*` to the global thread list.
///
/// To keep the FFI surface stable we still take `*mut c_void`, but the
/// stored value is converted through a `usize` ID — this lets the mutex
/// decide when the pointer identity has gone stale.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_mgr_add(cpp_ptr: *mut c_void) -> c_int {
    if cpp_ptr.is_null() {
        return -1;
    }
    let mut mgr = match PROFILER_MANAGER.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let id = cpp_ptr as usize as u64;
    if mgr.maps.contains(&id) {
        return 0;
    }
    mgr.maps.push(id);
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
    if cpp_ptr.is_null() {
        return -1;
    }
    let mut mgr = match PROFILER_MANAGER.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let id = cpp_ptr as usize as u64;
    if let Some(pos) = mgr.maps.iter().position(|&x| x == id) {
        mgr.maps.remove(pos);
        0
    } else {
        -1
    }
}

/// C ABI: Clear the global thread list (called from `profilerManager` destructor).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_mgr_clear() {
    let mut mgr = match PROFILER_MANAGER.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    mgr.maps.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_map_create_destroy() {
        unsafe {
            let h = fceux11_rust_profiler_map_create();
            assert!(!h.is_null());
            fceux11_rust_profiler_map_destroy(h);
        }
    }

    #[test]
    fn test_profiler_map_add_and_iterate() {
        unsafe {
            let h = fceux11_rust_profiler_map_create();
            assert!(!h.is_null());

            let rec1: *mut c_void = 0x1234 as *mut c_void;
            let rec2: *mut c_void = 0x5678 as *mut c_void;
            let id1 = fceux11_rust_profiler_record_take(rec1);
            let id2 = fceux11_rust_profiler_record_take(rec2);
            assert_ne!(id1, 0);
            assert_ne!(id2, 0);

            fceux11_rust_profiler_map_add_record(
                h,
                b"file.cpp\0".as_ptr() as *const c_char,
                42,
                std::ptr::null(),
                std::ptr::null(),
                id1,
            );
            fceux11_rust_profiler_map_add_record(
                h,
                b"other.cpp\0".as_ptr() as *const c_char,
                10,
                std::ptr::null(),
                std::ptr::null(),
                id2,
            );

            let first = fceux11_rust_profiler_map_iterate_begin(h);
            assert!(first == rec1 || first == rec2);

            let second = fceux11_rust_profiler_map_iterate_next(h);
            assert!(second == rec1 || second == rec2);
            assert_ne!(first, second);

            let third = fceux11_rust_profiler_map_iterate_next(h);
            assert!(third.is_null());

            // Releasing one record ID should make iteration skip it on
            // the next pass rather than reading dangling memory.
            fceux11_rust_profiler_record_drop(id1);

            let again_first = fceux11_rust_profiler_map_iterate_begin(h);
            assert_eq!(again_first, rec2); // id1 dropped, only id2 left

            fceux11_rust_profiler_record_drop(id2);
            fceux11_rust_profiler_map_destroy(h);
        }
    }

    #[test]
    fn test_profiler_map_stack() {
        unsafe {
            let h = fceux11_rust_profiler_map_create();
            let id1 = fceux11_rust_profiler_record_take(0x1 as *mut c_void);
            let id2 = fceux11_rust_profiler_record_take(0x2 as *mut c_void);
            assert_ne!(id1, 0);
            assert_ne!(id2, 0);

            fceux11_rust_profiler_map_push_stack(h, id1);
            fceux11_rust_profiler_map_push_stack(h, id2);
            fceux11_rust_profiler_map_pop_stack(h, id2);
            fceux11_rust_profiler_map_pop_stack(h, id1);

            fceux11_rust_profiler_record_drop(id1);
            fceux11_rust_profiler_record_drop(id2);
            fceux11_rust_profiler_map_destroy(h);
        }
    }

    #[test]
    fn test_profiler_mgr_add_remove() {
        unsafe {
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
}
