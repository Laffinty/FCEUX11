//! `debugger` library binding
//!
//! Provides debugger functions: `hitbreakpoint`, `getcyclescount`,
//! `getinstructionscount`, `resetcyclescount`, `resetinstructionscount`,
//! `getsymboloffset`.
//!
//! FFI bridge: reads debug counters and state from C++ debug.cpp.

use mlua::{Lua, Result, Table};

/// Register the `debugger` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let debugger = lua.create_table()?;

    debugger.set(
        "hitbreakpoint",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_debugger_hitbreakpoint() };
            Ok(())
        })?,
    )?;

    debugger.set(
        "getcyclescount",
        lua.create_function(|_, ()| {
            let count = unsafe { crate::fceux11_lua_debugger_get_cycles_count() };
            Ok(count as i64)
        })?,
    )?;

    debugger.set(
        "getinstructionscount",
        lua.create_function(|_, ()| {
            let count = unsafe { crate::fceux11_lua_debugger_get_instructions_count() };
            Ok(count as i64)
        })?,
    )?;

    debugger.set(
        "resetcyclescount",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_debugger_reset_cycles_count() };
            Ok(())
        })?,
    )?;

    debugger.set(
        "resetinstructionscount",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_debugger_reset_instructions_count() };
            Ok(())
        })?,
    )?;

    debugger.set(
        "getsymboloffset",
        lua.create_function(|_, name: mlua::String| {
            // Pass string directly as C string pointer to FFI
            let bytes = name.as_bytes_with_nul();
            let offset = unsafe {
                crate::fceux11_lua_debugger_get_symbol_offset(bytes.as_ptr() as *const i8)
            };
            Ok(offset)
        })?,
    )?;

    Ok(debugger)
}
