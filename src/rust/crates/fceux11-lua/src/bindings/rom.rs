//! `rom` library binding
//!
//! Provides ROM data access: `gethash`, `readbyte`, `writebyte`.
//!
//! FFI bridge: calls `FCEU_ReadRomByte` / `FCEU_WriteRomByte` and MD5 from
//! `fceux11-utils` via C++ FFI.

use mlua::{Lua, Result, Table};

const HASH_MD5: i32 = 0;

pub fn compute_md5_hex(buf: &[u8]) -> String {
    let mut ctx = fceux11_utils::md5::Md5Context {
        total: [0, 0],
        state: [0, 0, 0, 0],
        buffer: [0; 64],
    };
    let mut digest = [0u8; 16];
    fceux11_utils::md5::fceux11_rust_md5_starts(&mut ctx);
    fceux11_utils::md5::fceux11_rust_md5_update(&mut ctx, buf.as_ptr(), buf.len() as u32);
    fceux11_utils::md5::fceux11_rust_md5_finish(&mut ctx, digest.as_mut_ptr());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Register the `rom` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let rom = lua.create_table()?;

    rom.set(
        "gethash",
        lua.create_function(|_, hash_type: Option<i32>| {
            let ht = hash_type.unwrap_or(HASH_MD5);
            if ht != HASH_MD5 {
                return Err(mlua::Error::RuntimeError(
                    "only MD5 hash is currently supported".to_string(),
                ));
            }
            let mut md5_buf = [0u8; 16];
            let ok = unsafe { crate::fceux11_lua_GetRomMD5(md5_buf.as_mut_ptr()) };
            if ok != 0 {
                return Err(mlua::Error::RuntimeError(
                    "failed to retrieve ROM MD5".to_string(),
                ));
            }
            Ok(md5_buf.iter().map(|b| format!("{:02x}", b)).collect::<String>())
        })?,
    )?;

    rom.set(
        "readbyte",
        lua.create_function(|_, addr: u32| {
            let val = unsafe { crate::fceux11_lua_ReadRomByte(addr) };
            Ok(val as i32)
        })?,
    )?;

    rom.set(
        "writebyte",
        lua.create_function(|_, (addr, val): (u32, u32)| {
            // Safety: FFI call into C++ ROM writing function.
            // `FCEU_WriteRomByte` handles bank-switching internally.
            unsafe { crate::fceux11_lua_WriteRomByte(addr, val as u8) };
            Ok(())
        })?,
    )?;

    Ok(rom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_md5_hex_empty() {
        let hash = compute_md5_hex(b"");
        assert_eq!(hash.len(), 32);
        assert_eq!(hash, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_compute_md5_hex_fceux() {
        let hash = compute_md5_hex(b"FCEUX");
        assert_eq!(hash.len(), 32);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_md5_hex_format_lowercase() {
        let hash = compute_md5_hex(b"test");
        assert_eq!(hash, hash.to_lowercase());
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_md5_hex_vs_direct() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let hash = compute_md5_hex(data);
        let mut ctx = fceux11_utils::md5::Md5Context {
            total: [0, 0],
            state: [0, 0, 0, 0],
            buffer: [0; 64],
        };
        let mut digest = [0u8; 16];
        fceux11_utils::md5::fceux11_rust_md5_starts(&mut ctx);
        fceux11_utils::md5::fceux11_rust_md5_update(&mut ctx, data.as_ptr(), data.len() as u32);
        fceux11_utils::md5::fceux11_rust_md5_finish(&mut ctx, digest.as_mut_ptr());
        let expected: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_md5_hex_32_chars() {
        let hash = compute_md5_hex(b"NES ROM data");
        assert_eq!(hash.len(), 32, "MD5 hex string must be exactly 32 characters");
    }
}
