//! `joypad` library binding
//!
//! Provides joypad input access: `get`, `set`.
//!
//! FFI bridge: reads joypad state via C++ and sets override masks.

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
        lua.create_function(|_, (port, table): (i32, Table)| {
            // Parse button table into mask1 (pass-through bits) and mask2 (force-on bits)
            // mask1: 0 = block pass-through, 1 = allow pass-through (default all 1)
            // mask2: 1 = force this button on
            let mut mask1: u32 = 0xFF; // All pass-through by default
            let mut mask2: u32 = 0x00; // Nothing forced on by default

            let buttons = [
                ("A", JOY_A),
                ("B", JOY_B),
                ("select", JOY_SELECT),
                ("start", JOY_START),
                ("up", JOY_UP),
                ("down", JOY_DOWN),
                ("left", JOY_LEFT),
                ("right", JOY_RIGHT),
            ];

            for (name, bit) in buttons {
                match table.get::<mlua::Value>(name) {
                    Ok(val) => {
                        if val.is_nil() {
                            continue;
                        }
                        // In Lua, only nil and false are falsy
                        let is_truthy = !matches!(val, mlua::Value::Nil | mlua::Value::Boolean(false));
                        let is_string = matches!(&val, mlua::Value::String(_));

                        if is_truthy {
                            // Truthy → force this button on
                            mask2 |= bit;
                        }
                        if !is_truthy || is_string {
                            // False or string → block pass-through
                            mask1 &= !bit;
                        }
                    }
                    Err(_) => {}
                }
            }

            // Safety: FFI call into C++ input system. `SetJoypadOverride`
            // stores mask1/mask2 in global luajoypads1/luajoypads2 arrays.
            unsafe { crate::fceux11_lua_SetJoypadOverride(port, mask1, mask2) };
            Ok(())
        })?,
    )?;

    Ok(joypad)
}

