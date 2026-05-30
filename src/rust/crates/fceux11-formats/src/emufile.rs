//! EmuFile - unified file I/O abstraction.
//!
//! Provides `EmuFileMem` (in-memory buffer) and `EmuFileFile` (disk file)
//! as two concrete `EmuFile` implementations. The Rust side owns the state;
//! C++ receives opaque handles and calls FFI functions to perform I/O.
//!
//! # FFI design
//! - Opaque pointer pattern: C++ never dereferences the handle, only passes
//!   it back to FFI functions.
//! - Memory allocations use Rust's `Vec<u8>` (growable buffer).
//! - No `failbit` in Rust; errors are expressed via `Result<T, E>` in internal
//!   Rust APIs and via the `fail()` return value in FFI.

/// In-memory file buffer. Corresponds to C++ `EMUFILE_MEMORY`.
pub struct EmuFileMem {
    buf: Vec<u8>,
    pos: usize,
    own_buf: bool,
}

impl EmuFileMem {
    /// Create with a caller-provided buffer (borrowed).
    pub fn from_buf(buf: &[u8]) -> Self {
        Self {
            buf: buf.to_vec(),
            pos: 0,
            own_buf: false,
        }
    }

    /// Create with a pre-allocated buffer.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buf = Vec::new();
        buf.reserve(capacity);
        buf.resize(capacity, 0);
        Self {
            buf,
            pos: 0,
            own_buf: true,
        }
    }

    /// Create an empty buffer with default reserve.
    pub fn new() -> Self {
        let mut buf = Vec::new();
        buf.reserve(1024);
        Self {
            buf,
            pos: 0,
            own_buf: true,
        }
    }

    /// Create from raw bytes (takes ownership of the data).
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            buf: data.to_vec(),
            pos: 0,
            own_buf: true,
        }
    }

    /// Returns the current position.
    pub fn tell(&self) -> usize {
        self.pos
    }

    /// Returns the current length of the underlying buffer.
    pub fn size(&self) -> usize {
        self.buf.len()
    }

    /// Returns true if the position is at or past EOF.
    pub fn eof(&self) -> bool {
        self.pos >= self.buf.len()
    }

    /// Returns a mutable slice of the internal buffer for reading/writing.
    fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buf
    }

    /// Read up to `bytes` into `ptr`. Returns bytes actually read.
    pub fn read_into(&mut self, ptr: *mut u8, bytes: usize) -> usize {
        if ptr.is_null() || bytes == 0 {
            return 0;
        }
        let Some(slice) = self.as_slice().get(self.pos..) else {
            return 0;
        };
        let remain = slice.len();
        let todo = std::cmp::min(remain, bytes);
        if todo <= 4 {
            let src = &self.buf[self.pos..self.pos + todo];
            let dst = unsafe { std::slice::from_raw_parts_mut(ptr, todo) };
            for (i, &b) in src.iter().enumerate() {
                dst[i] = b;
            }
        } else {
            let src = &self.buf[self.pos..self.pos + todo];
            let dst = unsafe { std::slice::from_raw_parts_mut(ptr, todo) };
            dst.copy_from_slice(src);
        }
        self.pos += todo;
        if todo < bytes {
            return todo;
        }
        todo
    }

    /// Write bytes from `ptr` into the buffer. Grows if needed.
    pub fn write_from(&mut self, ptr: *const u8, bytes: usize) -> usize {
        if ptr.is_null() || bytes == 0 {
            return 0;
        }
        let new_pos = self.pos.saturating_add(bytes);
        if new_pos > self.buf.len() {
            self.buf.resize(new_pos, 0);
        }
        let src = unsafe { std::slice::from_raw_parts(ptr, bytes) };
        let dst = &mut self.buf[self.pos..new_pos];
        dst.copy_from_slice(src);
        self.pos = new_pos;
        bytes
    }

    /// Seek to an absolute position.
    pub fn seek_set(&mut self, offset: isize) -> bool {
        let offset = offset as usize;
        if offset > self.buf.len() {
            self.buf.resize(offset, 0);
        }
        self.pos = offset;
        true
    }

    /// Seek relative to current position.
    pub fn seek_cur(&mut self, offset: isize) -> bool {
        let new_pos = (self.pos as isize).saturating_add(offset);
        if new_pos < 0 {
            return false;
        }
        self.pos = new_pos as usize;
        if self.pos > self.buf.len() {
            self.buf.resize(self.pos, 0);
        }
        true
    }

    /// Seek relative to end.
    pub fn seek_end(&mut self, offset: isize) -> bool {
        let new_pos = (self.buf.len() as isize).saturating_add(offset);
        if new_pos < 0 {
            return false;
        }
        self.pos = new_pos as usize;
        if self.pos > self.buf.len() {
            self.buf.resize(self.pos, 0);
        }
        true
    }

    /// Truncate the buffer to `length`.
    pub fn truncate(&mut self, length: usize) {
        self.buf.truncate(length);
        if self.pos > length {
            self.pos = length;
        }
    }

    /// Get pointer to buffer data (for read-only access).
    pub fn data_ptr(&self) -> *const u8 {
        self.buf.as_ptr()
    }

    /// Mutable pointer for FFI writes.
    pub fn data_ptr_mut(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr()
    }

    /// Get the internal buffer (clone).
    pub fn get_vec(&self) -> Vec<u8> {
        self.buf.clone()
    }

    /// Set the logical length of the buffer.
    pub fn set_len(&mut self, length: usize) {
        self.buf.truncate(length);
        if self.pos > length {
            self.pos = length;
        }
    }
}

/// Opaque handle for an in-memory file.
pub type EmuFileMemHandle = *mut EmuFileMem;

/// Create a new in-memory EmuFile.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_emufile_mem_create() -> EmuFileMemHandle {
    Box::into_raw(Box::new(EmuFileMem::new()))
}

/// Create a new in-memory EmuFile with pre-allocated capacity.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_emufile_mem_create_with_capacity(capacity: usize) -> EmuFileMemHandle {
    Box::into_raw(Box::new(EmuFileMem::with_capacity(capacity)))
}

/// Create a new in-memory EmuFile from raw bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_emufile_mem_create_from_bytes(ptr: *const u8, len: usize) -> EmuFileMemHandle {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    Box::into_raw(Box::new(EmuFileMem::from_bytes(data)))
}

/// Destroy an in-memory EmuFile handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_destroy(handle: EmuFileMemHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

/// Read bytes into C buffer. Returns bytes read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_fread(
    handle: EmuFileMemHandle,
    ptr: *mut u8,
    bytes: usize,
) -> usize {
    if handle.is_null() || ptr.is_null() || bytes == 0 {
        return 0;
    }
    let mem = unsafe { &mut *handle };
    mem.read_into(ptr, bytes)
}

/// Write bytes from C buffer. Returns bytes written.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_fwrite(
    handle: EmuFileMemHandle,
    ptr: *const u8,
    bytes: usize,
) -> usize {
    if handle.is_null() || ptr.is_null() || bytes == 0 {
        return 0;
    }
    let mem = unsafe { &mut *handle };
    mem.write_from(ptr, bytes)
}

/// Seek within the memory file.
/// origin: 0 = SEEK_SET, 1 = SEEK_CUR, 2 = SEEK_END
/// Returns 0 on success, -1 on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_fseek(
    handle: EmuFileMemHandle,
    offset: i32,
    origin: i32,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let mem = unsafe { &mut *handle };
    let ok = match origin {
        0 => mem.seek_set(offset as isize),
        1 => mem.seek_cur(offset as isize),
        2 => mem.seek_end(offset as isize),
        _ => return -1,
    };
    if ok {
        0
    } else {
        -1
    }
}

/// Get current position.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_ftell(handle: EmuFileMemHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let mem = unsafe { &*handle };
    mem.tell() as u32
}

/// Get size of memory buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_size(handle: EmuFileMemHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let mem = unsafe { &*handle };
    mem.size() as u32
}

/// Truncate the memory buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_truncate(handle: EmuFileMemHandle, length: u32) {
    if !handle.is_null() {
        let mem = unsafe { &mut *handle };
        mem.truncate(length as usize);
    }
}

/// Check fail bit (always false in Rust, kept for API parity).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_fail(handle: EmuFileMemHandle) -> i32 {
    if handle.is_null() {
        return 1;
    }
    0
}

/// Returns non-zero if at EOF.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_eof(handle: EmuFileMemHandle) -> i32 {
    if handle.is_null() {
        return 1;
    }
    let mem = unsafe { &*handle };
    if mem.eof() {
        1
    } else {
        0
    }
}

/// Get read-only data pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_data_ptr(handle: EmuFileMemHandle) -> *const u8 {
    if handle.is_null() {
        return std::ptr::null();
    }
    let mem = unsafe { &*handle };
    mem.data_ptr()
}

/// Ungetc: step position back by one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_unget(handle: EmuFileMemHandle) {
    if handle.is_null() {
        return;
    }
    let mem = unsafe { &mut *handle };
    if mem.tell() > 0 {
        mem.seek_set((mem.tell() - 1) as isize);
    }
}

// ============================================================
// Scalar read/write helpers (little-endian)
// ============================================================

/// Read a u8 value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_read8(handle: EmuFileMemHandle) -> u8 {
    if handle.is_null() {
        return 0;
    }
    let mem = unsafe { &mut *handle };
    if mem.eof() {
        return 0;
    }
    let val = mem.as_slice()[mem.tell()];
    mem.seek_set((mem.tell() + 1) as isize);
    val
}

/// Write a u8 value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_write8(handle: EmuFileMemHandle, val: u8) {
    if handle.is_null() {
        return;
    }
    let mem = unsafe { &mut *handle };
    mem.write_from(&val, 1);
}

/// Read a u16 LE.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_read16le(handle: EmuFileMemHandle) -> u16 {
    if handle.is_null() {
        return 0;
    }
    let mem = unsafe { &mut *handle };
    let pos = mem.tell();
    if pos + 2 > mem.size() {
        return 0;
    }
    let slice = mem.as_slice();
    let lo = slice[pos] as u16;
    let hi = slice[pos + 1] as u16;
    mem.seek_set((pos + 2) as isize);
    lo | (hi << 8)
}

/// Write a u16 LE.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_write16le(handle: EmuFileMemHandle, val: u16) {
    if handle.is_null() {
        return;
    }
    let mem = unsafe { &mut *handle };
    let pos = mem.tell();
    let new_pos = pos + 2;
    if new_pos > mem.size() {
        mem.buf.resize(new_pos, 0);
    }
    mem.as_mut_slice()[pos] = (val & 0xFF) as u8;
    mem.as_mut_slice()[pos + 1] = ((val >> 8) & 0xFF) as u8;
    mem.seek_set(new_pos as isize);
}

/// Read a u32 LE.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_read32le(handle: EmuFileMemHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let mem = unsafe { &mut *handle };
    let pos = mem.tell();
    if pos + 4 > mem.size() {
        return 0;
    }
    let slice = mem.as_slice();
    let b0 = slice[pos] as u32;
    let b1 = slice[pos + 1] as u32;
    let b2 = slice[pos + 2] as u32;
    let b3 = slice[pos + 3] as u32;
    mem.seek_set((pos + 4) as isize);
    b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
}

/// Write a u32 LE.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_write32le(handle: EmuFileMemHandle, val: u32) {
    if handle.is_null() {
        return;
    }
    let mem = unsafe { &mut *handle };
    let pos = mem.tell();
    let new_pos = pos + 4;
    if new_pos > mem.size() {
        mem.buf.resize(new_pos, 0);
    }
    let slice = mem.as_mut_slice();
    slice[pos] = (val & 0xFF) as u8;
    slice[pos + 1] = ((val >> 8) & 0xFF) as u8;
    slice[pos + 2] = ((val >> 16) & 0xFF) as u8;
    slice[pos + 3] = ((val >> 24) & 0xFF) as u8;
    mem.seek_set(new_pos as isize);
}

/// Read a u64 LE.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_read64le(handle: EmuFileMemHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    let mem = unsafe { &mut *handle };
    let pos = mem.tell();
    if pos + 8 > mem.size() {
        return 0;
    }
    let slice = mem.as_slice();
    let mut val: u64 = 0;
    for i in 0..8 {
        val |= (slice[pos + i] as u64) << (i * 8);
    }
    mem.seek_set((pos + 8) as isize);
    val
}

/// Write a u64 LE.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_emufile_mem_write64le(handle: EmuFileMemHandle, val: u64) {
    if handle.is_null() {
        return;
    }
    let mem = unsafe { &mut *handle };
    let pos = mem.tell();
    let new_pos = pos + 8;
    if new_pos > mem.size() {
        mem.buf.resize(new_pos, 0);
    }
    let slice = mem.as_mut_slice();
    for i in 0..8 {
        slice[pos + i] = ((val >> (i * 8)) & 0xFF) as u8;
    }
    mem.seek_set(new_pos as isize);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_destroy() {
        let h = fceux11_rust_emufile_mem_create();
        assert!(!h.is_null());
        unsafe { fceux11_rust_emufile_mem_destroy(h) };
    }

    #[test]
    fn test_write_read_bytes() {
        let h = fceux11_rust_emufile_mem_create();
        let data = [1u8, 2, 3, 4, 5];
        let wrote = unsafe { fceux11_rust_emufile_mem_fwrite(h, data.as_ptr(), data.len()) };
        assert_eq!(wrote, 5);

        unsafe { fceux11_rust_emufile_mem_fseek(h, 0, 0) };

        let mut buf = [0u8; 5];
        let read = unsafe { fceux11_rust_emufile_mem_fread(h, buf.as_mut_ptr(), 5) };
        assert_eq!(read, 5);
        assert_eq!(buf, data);

        unsafe { fceux11_rust_emufile_mem_destroy(h) };
    }

    #[test]
    fn test_eof() {
        let h = fceux11_rust_emufile_mem_create();
        assert!(unsafe { fceux11_rust_emufile_mem_eof(h) } != 0);

        let data = [42u8];
        unsafe { fceux11_rust_emufile_mem_fwrite(h, data.as_ptr(), 1) };

        unsafe { fceux11_rust_emufile_mem_fseek(h, 0, 0) };
        assert!(unsafe { fceux11_rust_emufile_mem_eof(h) } == 0);

        unsafe { fceux11_rust_emufile_mem_fseek(h, 0, 0) };
        let mut buf = [0u8];
        unsafe { fceux11_rust_emufile_mem_fread(h, buf.as_mut_ptr(), 1) };
        assert!(unsafe { fceux11_rust_emufile_mem_eof(h) } != 0);

        unsafe { fceux11_rust_emufile_mem_destroy(h) };
    }

    #[test]
    fn test_scalar_read_write() {
        let h = fceux11_rust_emufile_mem_create();

        unsafe { fceux11_rust_emufile_mem_write8(h, 0xAB) };
        unsafe { fceux11_rust_emufile_mem_write16le(h, 0x1234) };
        unsafe { fceux11_rust_emufile_mem_write32le(h, 0x56789ABC) };
        unsafe { fceux11_rust_emufile_mem_write64le(h, 0xDEADBEEF01234567) };

        unsafe { fceux11_rust_emufile_mem_fseek(h, 0, 0) };

        assert_eq!(unsafe { fceux11_rust_emufile_mem_read8(h) }, 0xAB);
        assert_eq!(unsafe { fceux11_rust_emufile_mem_read16le(h) }, 0x1234);
        assert_eq!(unsafe { fceux11_rust_emufile_mem_read32le(h) }, 0x56789ABC);
        assert_eq!(unsafe { fceux11_rust_emufile_mem_read64le(h) }, 0xDEADBEEF01234567);

        unsafe { fceux11_rust_emufile_mem_destroy(h) };
    }

    #[test]
    fn test_truncate() {
        let h = fceux11_rust_emufile_mem_create();
        let data = [1u8, 2, 3, 4, 5];
        unsafe { fceux11_rust_emufile_mem_fwrite(h, data.as_ptr(), data.len()) };

        unsafe { fceux11_rust_emufile_mem_truncate(h, 3) };
        assert_eq!(unsafe { fceux11_rust_emufile_mem_size(h) }, 3);

        unsafe { fceux11_rust_emufile_mem_destroy(h) };
    }

    #[test]
    fn test_null_safety() {
        // Operations on null handle return safe defaults
        assert_eq!(unsafe { fceux11_rust_emufile_mem_fread(std::ptr::null_mut(), std::ptr::null_mut(), 10) }, 0);
        assert_eq!(unsafe { fceux11_rust_emufile_mem_fwrite(std::ptr::null_mut(), std::ptr::null(), 10) }, 0);
        assert_eq!(unsafe { fceux11_rust_emufile_mem_ftell(std::ptr::null_mut()) }, 0);
        assert_eq!(unsafe { fceux11_rust_emufile_mem_size(std::ptr::null_mut()) }, 0);
        assert!(unsafe { fceux11_rust_emufile_mem_eof(std::ptr::null_mut()) } != 0);
    }

    #[test]
    fn test_determinism() {
        // Same data always produces same result
        let h1 = fceux11_rust_emufile_mem_create();
        let h2 = fceux11_rust_emufile_mem_create();
        let data = [0xDEu8, 0xAD, 0xBE, 0xEF];

        unsafe {
            fceux11_rust_emufile_mem_fwrite(h1, data.as_ptr(), data.len());
            fceux11_rust_emufile_mem_fwrite(h2, data.as_ptr(), data.len());
            fceux11_rust_emufile_mem_fseek(h1, 0, 0);
            fceux11_rust_emufile_mem_fseek(h2, 0, 0);

            let mut buf1 = [0u8; 4];
            let mut buf2 = [0u8; 4];
            fceux11_rust_emufile_mem_fread(h1, buf1.as_mut_ptr(), 4);
            fceux11_rust_emufile_mem_fread(h2, buf2.as_mut_ptr(), 4);

            assert_eq!(buf1, buf2);
            fceux11_rust_emufile_mem_destroy(h1);
            fceux11_rust_emufile_mem_destroy(h2);
        }
    }
}