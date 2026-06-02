//! `joypad` library binding
//!
//! Provides joypad input access: `get`, `set`.
//!
//! FFI bridge: reads joypad state via C++ and sets override masks.

use mlua::{Lua, Result, Table};

const JOY_A: u32 = 0x01;
const JOY_B: u32 = 0x02;
const JOY_SELECT: u32 = 0x04;
const JOY_START: u32 = 0x08;
const JOY_UP: u32 = 0x10;
const JOY_DOWN: u32 = 0x20;
const JOY_LEFT: u32 = 0x40;
const JOY_RIGHT: u32 = 0x80;

const BUTTON_MAP: [(&str, u32); 8] = [
    ("A", JOY_A),
    ("B", JOY_B),
    ("select", JOY_SELECT),
    ("start", JOY_START),
    ("up", JOY_UP),
    ("down", JOY_DOWN),
    ("left", JOY_LEFT),
    ("right", JOY_RIGHT),
];

pub fn bitmask_to_table(lua: &Lua, state: u32) -> Result<Table> {
    let table = lua.create_table()?;
    for (name, bit) in BUTTON_MAP {
        table.set(name, (state & bit) != 0)?;
    }
    Ok(table)
}

/// Register the `joypad` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let joypad = lua.create_table()?;

    joypad.set(
        "get",
        lua.create_function(|lua, port: i32| {
            let state = unsafe { crate::fceux11_lua_GetJoypadState(port) };
            bitmask_to_table(lua, state)
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

            for (name, bit) in BUTTON_MAP {
                match table.get::<mlua::Value>(name) {
                    Ok(val) => {
                        if val.is_nil() {
                            continue;
                        }
                        // In Lua, only nil and false are falsy
                        let is_truthy =
                            !matches!(val, mlua::Value::Nil | mlua::Value::Boolean(false));
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

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_bitmask_to_table_all_pressed() {
        let lua = Lua::new();
        let table = bitmask_to_table(&lua, 0xFF).unwrap();
        for (name, _bit) in BUTTON_MAP {
            let pressed: bool = table.get(name).unwrap();
            assert!(pressed, "button {} should be pressed", name);
        }
    }

    #[test]
    fn test_bitmask_to_table_none_pressed() {
        let lua = Lua::new();
        let table = bitmask_to_table(&lua, 0x00).unwrap();
        for (name, _bit) in BUTTON_MAP {
            let pressed: bool = table.get(name).unwrap();
            assert!(!pressed, "button {} should not be pressed", name);
        }
    }

    #[test]
    fn test_bitmask_to_table_a_and_start() {
        let lua = Lua::new();
        let state = JOY_A | JOY_START;
        let table = bitmask_to_table(&lua, state).unwrap();
        assert_eq!(table.get::<bool>("A").unwrap(), true);
        assert_eq!(table.get::<bool>("start").unwrap(), true);
        assert_eq!(table.get::<bool>("B").unwrap(), false);
        assert_eq!(table.get::<bool>("select").unwrap(), false);
        assert_eq!(table.get::<bool>("up").unwrap(), false);
        assert_eq!(table.get::<bool>("down").unwrap(), false);
        assert_eq!(table.get::<bool>("left").unwrap(), false);
        assert_eq!(table.get::<bool>("right").unwrap(), false);
    }

    #[test]
    fn test_bitmask_to_table_directional() {
        let lua = Lua::new();
        let state = JOY_UP | JOY_LEFT;
        let table = bitmask_to_table(&lua, state).unwrap();
        assert_eq!(table.get::<bool>("up").unwrap(), true);
        assert_eq!(table.get::<bool>("left").unwrap(), true);
        assert_eq!(table.get::<bool>("down").unwrap(), false);
        assert_eq!(table.get::<bool>("right").unwrap(), false);
    }

    #[test]
    fn test_bitmask_to_table_lua_roundtrip() {
        let lua = Lua::new();
        lua.globals().set("bitmask_to_table", lua.create_function(|lua, state: u32| {
            bitmask_to_table(lua, state)
        }).unwrap()).unwrap();

        let result: mlua::Table = lua.load("bitmask_to_table(0x11)").eval().unwrap();
        assert_eq!(result.get::<bool>("A").unwrap(), true);
        assert_eq!(result.get::<bool>("up").unwrap(), true);
        assert_eq!(result.get::<bool>("B").unwrap(), false);
    }

    #[test]
    fn test_bitmask_to_table_type_is_boolean() {
        let lua = Lua::new();
        let table = bitmask_to_table(&lua, JOY_A).unwrap();
        let val: mlua::Value = table.get("A").unwrap();
        assert!(matches!(val, mlua::Value::Boolean(true)));

        let val: mlua::Value = table.get("B").unwrap();
        assert!(matches!(val, mlua::Value::Boolean(false)));
    }
}
