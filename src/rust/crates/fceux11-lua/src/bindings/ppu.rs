//! `ppu` library binding
//!
//! Provides PPU (Picture Processing Unit) memory access: `readbyte`, `readbyterange`.
//!
//! FFI bridge: calls C++ `FFCEUX_PPURead` from `ppu.cpp`.

use mlua::{Lua, Result, Table};

/// Register the `ppu` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let ppu = lua.create_table()?;

    ppu.set(
        "readbyte",
        lua.create_function(|_, addr: u32| {
            let val = unsafe { crate::fceux11_lua_PPURead(addr) };
            Ok(val as i32)
        })?,
    )?;

    ppu.set(
        "readbyterange",
        lua.create_function(|_, (addr, len): (u32, u32)| {
            let mut result = Vec::with_capacity(len as usize);
            for i in 0..len {
                let val = unsafe { crate::fceux11_lua_PPURead(addr + i) };
                result.push(val as i32);
            }
            Ok(result)
        })?,
    )?;

    Ok(ppu)
}
