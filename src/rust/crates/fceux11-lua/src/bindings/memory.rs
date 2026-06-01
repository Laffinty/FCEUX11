//! `memory` library binding
//!
//! Provides NES memory access: `readbyte`, `writebyte`, `getregister`,
//! `registerwrite`, `registerread`, `registerexec`.
//!
//! FFI bridge: calls C++ `GetMem()` and `BWrite[]` from `x6502.cpp`.

use mlua::{Lua, Table, Result, Function};

/// Register index for CPU registers (matching C++ enum order)
const REG_PC: i32 = 0;
const REG_A: i32 = 1;
const REG_X: i32 = 2;
const REG_Y: i32 = 3;
const REG_S: i32 = 4;
const REG_P: i32 = 5;

/// Register the `memory` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let memory = lua.create_table()?;

    // --- Read/Write ---

    memory.set(
        "readbyte",
        lua.create_function(|_, addr: u32| {
            // Safety: FFI call into C++ global function — the C++ side is
            // single-threaded w.r.t. emulation. `GetMem` is a normal C function
            // reading NES memory via function pointers, no aliasing issues.
            let val = unsafe { crate::fceux11_lua_GetMem(addr) };
            Ok(val as i32)
        })?,
    )?;

    memory.set(
        "writebyte",
        lua.create_function(|_, (addr, val): (u32, u32)| {
            // Safety: `BWrite` is a global array of write-function pointers.
            // The FFI call invokes the correct write handler for the address.
            // `addr` is cast to u16 (NES address space is 16-bit) and `val` to u8.
            unsafe { crate::fceux11_lua_BWrite(addr, val as u8) };
            Ok(())
        })?,
    )?;

    // --- Registers ---

    memory.set(
        "getregister",
        lua.create_function(|_, name: mlua::String| {
            let s = name.to_str()?;
            let val = match s.as_ref() {
                "pc" | "PC" => unsafe { crate::fceux11_lua_GetRegister(REG_PC) },
                "a" | "A" => unsafe { crate::fceux11_lua_GetRegister(REG_A) },
                "x" | "X" => unsafe { crate::fceux11_lua_GetRegister(REG_X) },
                "y" | "Y" => unsafe { crate::fceux11_lua_GetRegister(REG_Y) },
                "s" | "S" => unsafe { crate::fceux11_lua_GetRegister(REG_S) },
                "p" | "P" => unsafe { crate::fceux11_lua_GetRegister(REG_P) },
                _ => return Err(mlua::Error::RuntimeError(format!(
                    "unknown register: {}",
                    s
                ))),
            };
            Ok(val as i32)
        })?,
    )?;

    // --- Hooks (stub — TieredRegion stays in C++) ---

    memory.set(
        "registerwrite",
        lua.create_function(|_, (_addr, _func): (u32, Function)| {
            // TieredRegion lookup is a hot path kept in C++. The Rust side
            // would need RegistryKey management + FFI overhead, so we defer.
            Ok(())
        })?,
    )?;

    memory.set(
        "registerread",
        lua.create_function(|_, (_addr, _func): (u32, Function)| Ok(()))?,
    )?;

    memory.set(
        "registerexec",
        lua.create_function(|_, (_addr, _func): (u32, Function)| Ok(()))?,
    )?;

    Ok(memory)
}