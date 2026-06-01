//! `joypad` library binding
//!
//! Provides joypad input access: `get`, `set`.
//!
//! FFI bridge: reads/writes joypad state via C++ input system.

use mlua::{Lua, Table, Result};

/// Button bitmask flags (matching FCEUX input.h)
const JOY_A: u32 = 0x01;
const JOY_B: u32 = 0x02;
const JOY_SELECT: u32 = 0x04;
const JOY_START: u32 = 0x08;
const JOY_UP: u32 = 0x10;
const JOY_DOWN: u32 = 0x20;
const JOY_LEFT: u32 = 0x40;
const JOY_RIGHT: u32 = 0x80;

/// Register the `joypad` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let joypad = lua.create_table()?;

    joypad.set(
        "get",
        lua.create_function(|_, port: i32| {
            let state = unsafe { crate::fceux11_lua_GetJoypadState(port) };
            Ok(state as i32)
        })?,
    )?;

    joypad.set(
        "set",
        lua.create_function(|_, (port, mask1, mask2): (i32, u32, u32)| {
            // Safety: FFI call into C++ input system. `SetJoypadOverride`
            // stores mask1/mask2 in a global JoypadOverride struct accessed
            // only during `FCEU_LuaReadJoypad` call in the input pipeline.
            unsafe { crate::fceux11_lua_SetJoypadOverride(port, mask1, mask2) };
            Ok(())
        })?,
    )?;

    Ok(joypad)
}

// ---------------------------------------------------------------------------
// FFI declarations (mirrored in lib.rs)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fceux11_lua_GetJoypadState(port: i32) -> u32;
    fn fceux11_lua_SetJoypadOverride(port: i32, mask1: u32, mask2: u32);
}