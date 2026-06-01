//! `rom` library binding
//!
//! Provides ROM data access: `gethash`, `readbyte`, `writebyte`.
//!
//! FFI bridge: calls `FCEU_ReadRomByte` / `FCEU_WriteRomByte` and CRC32 from
//! `fceux11-utils`.

use mlua::{Lua, Table, Result};
use crc32fast::Hasher;

/// Hash type constants (matching C++ lua-engine.cpp)
const HASH_MD5: i32 = 0;
const HASH_SHA1: i32 = 1;

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
            // Read entire PRG ROM — 16 KB minimum, up to 512 KB
            // We read 128 KB as a reasonable upper bound for MD5
            const ROM_SIZE: usize = 128 * 1024;
            let mut buf = vec![0u8; ROM_SIZE];
            let mut total = 0usize;
            for i in 0..ROM_SIZE {
                let b = unsafe { crate::fceux11_lua_ReadRomByte(i as u32) };
                if b == 0 && i > 0 {
                    // Try to detect actual ROM size by looking for zeros
                    // Only break if we've seen non-zero data first
                    let non_zero = buf[..i].iter().any(|&v| v != 0);
                    if non_zero && total > 0 {
                        break;
                    }
                } else {
                    buf[i] = b;
                    total = i + 1;
                }
            }
            let mut hasher = Hasher::new();
            hasher.update(&buf[..total]);
            let hash = hasher.finalize();
            Ok(format!("{:08x}", hash))
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

// ---------------------------------------------------------------------------
// FFI declarations (mirrored in lib.rs)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fceux11_lua_ReadRomByte(addr: u32) -> u8;
    fn fceux11_lua_WriteRomByte(addr: u32, val: u8);
}