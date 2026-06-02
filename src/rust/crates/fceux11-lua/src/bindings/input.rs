//! `input` library binding
//!
//! Provides input state and dialog functions: `get`, `popup`, `openfilepopup`, `savefilepopup`.
//!
//! FFI bridge: reads keyboard/mouse state via C++ FFI functions.

use mlua::{Lua, Result, Table};

const KEY_TO_NAME: [(usize, &str); 99] = [
    (0x01, "leftclick"),
    (0x02, "rightclick"),
    (0x04, "middleclick"),
    (0x08, "backspace"),
    (0x09, "tab"),
    (0x0D, "enter"),
    (0x10, "shift"),
    (0x11, "control"),
    (0x12, "alt"),
    (0x13, "pause"),
    (0x14, "capslock"),
    (0x1B, "escape"),
    (0x20, "space"),
    (0x21, "pageup"),
    (0x22, "pagedown"),
    (0x23, "end"),
    (0x24, "home"),
    (0x25, "left"),
    (0x26, "up"),
    (0x27, "right"),
    (0x28, "down"),
    (0x2D, "insert"),
    (0x2E, "delete"),
    (0x30, "0"),
    (0x31, "1"),
    (0x32, "2"),
    (0x33, "3"),
    (0x34, "4"),
    (0x35, "5"),
    (0x36, "6"),
    (0x37, "7"),
    (0x38, "8"),
    (0x39, "9"),
    (0x41, "A"),
    (0x42, "B"),
    (0x43, "C"),
    (0x44, "D"),
    (0x45, "E"),
    (0x46, "F"),
    (0x47, "G"),
    (0x48, "H"),
    (0x49, "I"),
    (0x4A, "J"),
    (0x4B, "K"),
    (0x4C, "L"),
    (0x4D, "M"),
    (0x4E, "N"),
    (0x4F, "O"),
    (0x50, "P"),
    (0x51, "Q"),
    (0x52, "R"),
    (0x53, "S"),
    (0x54, "T"),
    (0x55, "U"),
    (0x56, "V"),
    (0x57, "W"),
    (0x58, "X"),
    (0x59, "Y"),
    (0x5A, "Z"),
    (0x60, "numpad0"),
    (0x61, "numpad1"),
    (0x62, "numpad2"),
    (0x63, "numpad3"),
    (0x64, "numpad4"),
    (0x65, "numpad5"),
    (0x66, "numpad6"),
    (0x67, "numpad7"),
    (0x68, "numpad8"),
    (0x69, "numpad9"),
    (0x6A, "numpad*"),
    (0x6B, "numpad+"),
    (0x6D, "numpad-"),
    (0x6E, "numpad."),
    (0x6F, "numpad/"),
    (0x70, "F1"),
    (0x71, "F2"),
    (0x72, "F3"),
    (0x73, "F4"),
    (0x74, "F5"),
    (0x75, "F6"),
    (0x76, "F7"),
    (0x77, "F8"),
    (0x78, "F9"),
    (0x79, "F10"),
    (0x7A, "F11"),
    (0x7B, "F12"),
    (0x90, "numlock"),
    (0x91, "scrolllock"),
    (0xBA, "semicolon"),
    (0xBB, "plus"),
    (0xBC, "comma"),
    (0xBD, "minus"),
    (0xBE, "period"),
    (0xBF, "slash"),
    (0xC0, "tilde"),
    (0xDB, "leftbracket"),
    (0xDC, "backslash"),
    (0xDD, "rightbracket"),
    (0xDE, "quote"),
];

pub fn build_input_table(lua: &Lua, keys: &[u8; 256], xmouse: i32, ymouse: i32, click: i32) -> Result<Table> {
    let table = lua.create_table()?;
    for (vk, name) in KEY_TO_NAME {
        let mask = if vk == 0x14 || vk == 0x90 || vk == 0x91 {
            0x01
        } else {
            0x80
        };
        if keys.get(vk).map(|&v| (v & mask) != 0).unwrap_or(false) {
            table.set(name, true)?;
        }
    }
    if xmouse >= 0 {
        table.set("xmouse", xmouse)?;
    }
    if ymouse >= 0 {
        table.set("ymouse", ymouse)?;
    }
    if click != 0 {
        if click == 1 {
            table.set("leftclick", true)?;
        } else if click == 2 {
            table.set("rightclick", true)?;
        }
    }
    Ok(table)
}

/// Register the `input` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let input = lua.create_table()?;

    input.set(
        "get",
        lua.create_function(|lua, ()| {
            let mut keys = [0u8; 256];
            let ok = unsafe { crate::fceux11_lua_GetKeyboardState(keys.as_mut_ptr()) };
            let mut x: i32 = 0;
            let mut y: i32 = 0;
            let mut click: i32 = 0;
            unsafe { crate::fceux11_lua_GetMouseState(&mut x, &mut y, &mut click) };
            if ok < 0 {
                return Ok(lua.create_table()?);
            }
            build_input_table(lua, &keys, x, y, click)
        })?,
    )?;

    input.set(
        "popup",
        lua.create_function(|_, msg: String| {
            let c_msg = std::ffi::CString::new(msg).unwrap_or_default();
            unsafe { crate::fceux11_lua_gui_popup(c_msg.as_ptr()) };
            Ok(())
        })?,
    )?;

    input.set(
        "openfilepopup",
        lua.create_function(|lua, _options: Table| {
            Ok(lua.create_table()?)
        })?,
    )?;

    input.set(
        "savefilepopup",
        lua.create_function(|_, _options: Table| {
            Ok(String::new())
        })?,
    )?;

    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_build_input_table_empty() {
        let lua = Lua::new();
        let keys = [0u8; 256];
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        let pairs: Vec<(String, mlua::Value)> = table.sequence_values().filter_map(|v| v.ok()).collect();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_build_input_table_shift_pressed() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[0x10] = 0x80;
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(table.get::<bool>("shift").unwrap(), true);
        assert!(table.get::<bool>("control").is_err() || !table.get::<bool>("control").unwrap());
    }

    #[test]
    fn test_build_input_table_capslock_toggle() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[0x14] = 0x01;
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(table.get::<bool>("capslock").unwrap(), true);
    }

    #[test]
    fn test_build_input_table_mouse_coords() {
        let lua = Lua::new();
        let keys = [0u8; 256];
        let table = build_input_table(&lua, &keys, 100, 200, 0).unwrap();
        assert_eq!(table.get::<i32>("xmouse").unwrap(), 100);
        assert_eq!(table.get::<i32>("ymouse").unwrap(), 200);
    }

    #[test]
    fn test_build_input_table_left_click() {
        let lua = Lua::new();
        let keys = [0u8; 256];
        let table = build_input_table(&lua, &keys, 50, 60, 1).unwrap();
        assert_eq!(table.get::<bool>("leftclick").unwrap(), true);
        assert_eq!(table.get::<i32>("xmouse").unwrap(), 50);
        assert_eq!(table.get::<i32>("ymouse").unwrap(), 60);
    }

    #[test]
    fn test_build_input_table_right_click() {
        let lua = Lua::new();
        let keys = [0u8; 256];
        let table = build_input_table(&lua, &keys, 10, 20, 2).unwrap();
        assert_eq!(table.get::<bool>("rightclick").unwrap(), true);
    }

    #[test]
    fn test_build_input_table_multiple_keys() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[0x57] = 0x80;
        keys[0x41] = 0x80;
        keys[0x10] = 0x80;
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(table.get::<bool>("W").unwrap(), true);
        assert_eq!(table.get::<bool>("A").unwrap(), true);
        assert_eq!(table.get::<bool>("shift").unwrap(), true);
    }

    #[test]
    fn test_build_input_table_no_mouse_on_negative() {
        let lua = Lua::new();
        let keys = [0u8; 256];
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert!(table.get::<i32>("xmouse").is_err());
        assert!(table.get::<i32>("ymouse").is_err());
    }

    #[test]
    fn test_key_to_name_coverage() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        for (vk, _) in KEY_TO_NAME {
            keys[vk] = 0x80;
        }
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        for (_, name) in KEY_TO_NAME {
            let pressed: bool = table.get(name).unwrap();
            assert!(pressed, "key {} should be detected as pressed", name);
        }
    }
}
