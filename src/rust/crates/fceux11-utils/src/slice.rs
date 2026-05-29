use std::slice;

/// C-compatible slice descriptor for read-only buffers.
#[repr(C)]
pub struct FceuSlice {
    pub ptr: *const u8,
    pub len: usize,
}

/// C-compatible slice descriptor for mutable buffers.
#[repr(C)]
pub struct FceuSliceMut {
    pub ptr: *mut u8,
    pub len: usize,
}

impl FceuSlice {
    /// Convert to a safe Rust slice.
    ///
    /// # Safety
    /// `ptr` must be valid for `len` bytes or null.
    pub unsafe fn as_slice(&self) -> &[u8] {
        if self.ptr.is_null() || self.len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.ptr, self.len) }
        }
    }
}

impl FceuSliceMut {
    /// Convert to a safe mutable Rust slice.
    ///
    /// # Safety
    /// `ptr` must be valid for `len` bytes or null.
    pub unsafe fn as_mut_slice(&self) -> &mut [u8] {
        if self.ptr.is_null() || self.len == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fceu_slice() {
        let data = [1u8, 2, 3];
        let s = FceuSlice {
            ptr: data.as_ptr(),
            len: data.len(),
        };
        let sl = unsafe { s.as_slice() };
        assert_eq!(sl, &[1, 2, 3]);
    }

    #[test]
    fn test_fceu_slice_null() {
        let s = FceuSlice {
            ptr: std::ptr::null(),
            len: 0,
        };
        let sl = unsafe { s.as_slice() };
        assert!(sl.is_empty());
    }

    #[test]
    fn test_fceu_slice_mut() {
        let mut data = [1u8, 2, 3];
        let s = FceuSliceMut {
            ptr: data.as_mut_ptr(),
            len: data.len(),
        };
        let sl = unsafe { s.as_mut_slice() };
        sl[0] = 42;
        assert_eq!(data[0], 42);
    }
}
