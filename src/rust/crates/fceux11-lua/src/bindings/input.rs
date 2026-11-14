//! `input` library binding
//!
//! Provides input state and dialog functions: `get`, `popup`, `openfilepopup`, `savefilepopup`.
//!
//! FFI bridge: reads keyboard/mouse state via C++ FFI functions.

use mlua::{Lua, Result, Table};

// v2.0.4 V25: this table maps SDL scancodes (the index domain of the driver's
// g_keyState array, see src/drivers/Qt/input.cpp) to the Lua-facing field
// names of input.get(). The old table was keyed by Windows VK codes, which
// misaligned every name (VK 0x14 "capslock" read scancode 20 = Q, VK 0x08
// "backspace" read scancode 8 = E). Scancode values are taken from
// SDL_scancode.h of the vendored SDL2 (SDL_NUM_SCANCODES = 512); entries are
// limited to scancodes < 256 because the FFI bridge copies only indices
// 0..255 (fceWrapper_GetKeyboardState in src/drivers/Qt/fceu_callbacks.cpp).
// Lua-facing names keep the upstream FCEUX spelling; left/right modifier
// scancodes deliberately share one name ("shift"/"control"/"alt"), matching
// the old VK behaviour where L/R were conflated. Mouse buttons are not part
// of this table: they come from the click word (left 0x01, right 0x02,
// middle 0x04) below. Scancodes >= 256 (MODE, audio/media keys) are outside
// the FFI window and are not representable.
const KEY_TO_NAME: [(usize, &str); 99] = [
    (4, "A"), // SDL_SCANCODE_A
    (5, "B"), // SDL_SCANCODE_B
    (6, "C"), // SDL_SCANCODE_C
    (7, "D"), // SDL_SCANCODE_D
    (8, "E"), // SDL_SCANCODE_E
    (9, "F"), // SDL_SCANCODE_F
    (10, "G"), // SDL_SCANCODE_G
    (11, "H"), // SDL_SCANCODE_H
    (12, "I"), // SDL_SCANCODE_I
    (13, "J"), // SDL_SCANCODE_J
    (14, "K"), // SDL_SCANCODE_K
    (15, "L"), // SDL_SCANCODE_L
    (16, "M"), // SDL_SCANCODE_M
    (17, "N"), // SDL_SCANCODE_N
    (18, "O"), // SDL_SCANCODE_O
    (19, "P"), // SDL_SCANCODE_P
    (20, "Q"), // SDL_SCANCODE_Q
    (21, "R"), // SDL_SCANCODE_R
    (22, "S"), // SDL_SCANCODE_S
    (23, "T"), // SDL_SCANCODE_T
    (24, "U"), // SDL_SCANCODE_U
    (25, "V"), // SDL_SCANCODE_V
    (26, "W"), // SDL_SCANCODE_W
    (27, "X"), // SDL_SCANCODE_X
    (28, "Y"), // SDL_SCANCODE_Y
    (29, "Z"), // SDL_SCANCODE_Z
    (30, "1"), // SDL_SCANCODE_1
    (31, "2"), // SDL_SCANCODE_2
    (32, "3"), // SDL_SCANCODE_3
    (33, "4"), // SDL_SCANCODE_4
    (34, "5"), // SDL_SCANCODE_5
    (35, "6"), // SDL_SCANCODE_6
    (36, "7"), // SDL_SCANCODE_7
    (37, "8"), // SDL_SCANCODE_8
    (38, "9"), // SDL_SCANCODE_9
    (39, "0"), // SDL_SCANCODE_0
    (40, "enter"), // SDL_SCANCODE_RETURN
    (41, "escape"), // SDL_SCANCODE_ESCAPE
    (42, "backspace"), // SDL_SCANCODE_BACKSPACE
    (43, "tab"), // SDL_SCANCODE_TAB
    (44, "space"), // SDL_SCANCODE_SPACE
    (45, "minus"), // SDL_SCANCODE_MINUS
    (46, "plus"), // SDL_SCANCODE_EQUALS (the "+/=" key, VK_OEM_PLUS)
    (47, "leftbracket"), // SDL_SCANCODE_LEFTBRACKET
    (48, "rightbracket"), // SDL_SCANCODE_RIGHTBRACKET
    (49, "backslash"), // SDL_SCANCODE_BACKSLASH
    (51, "semicolon"), // SDL_SCANCODE_SEMICOLON
    (52, "quote"), // SDL_SCANCODE_APOSTROPHE
    (53, "tilde"), // SDL_SCANCODE_GRAVE
    (54, "comma"), // SDL_SCANCODE_COMMA
    (55, "period"), // SDL_SCANCODE_PERIOD
    (56, "slash"), // SDL_SCANCODE_SLASH
    (57, "capslock"), // SDL_SCANCODE_CAPSLOCK
    (58, "F1"), // SDL_SCANCODE_F1
    (59, "F2"), // SDL_SCANCODE_F2
    (60, "F3"), // SDL_SCANCODE_F3
    (61, "F4"), // SDL_SCANCODE_F4
    (62, "F5"), // SDL_SCANCODE_F5
    (63, "F6"), // SDL_SCANCODE_F6
    (64, "F7"), // SDL_SCANCODE_F7
    (65, "F8"), // SDL_SCANCODE_F8
    (66, "F9"), // SDL_SCANCODE_F9
    (67, "F10"), // SDL_SCANCODE_F10
    (68, "F11"), // SDL_SCANCODE_F11
    (69, "F12"), // SDL_SCANCODE_F12
    (71, "scrolllock"), // SDL_SCANCODE_SCROLLLOCK
    (72, "pause"), // SDL_SCANCODE_PAUSE
    (73, "insert"), // SDL_SCANCODE_INSERT
    (74, "home"), // SDL_SCANCODE_HOME
    (75, "pageup"), // SDL_SCANCODE_PAGEUP
    (76, "delete"), // SDL_SCANCODE_DELETE
    (77, "end"), // SDL_SCANCODE_END
    (78, "pagedown"), // SDL_SCANCODE_PAGEDOWN
    (79, "right"), // SDL_SCANCODE_RIGHT
    (80, "left"), // SDL_SCANCODE_LEFT
    (81, "down"), // SDL_SCANCODE_DOWN
    (82, "up"), // SDL_SCANCODE_UP
    (83, "numlock"), // SDL_SCANCODE_NUMLOCKCLEAR
    (84, "numpad/"), // SDL_SCANCODE_KP_DIVIDE
    (85, "numpad*"), // SDL_SCANCODE_KP_MULTIPLY
    (86, "numpad-"), // SDL_SCANCODE_KP_MINUS
    (87, "numpad+"), // SDL_SCANCODE_KP_PLUS
    (89, "numpad1"), // SDL_SCANCODE_KP_1
    (90, "numpad2"), // SDL_SCANCODE_KP_2
    (91, "numpad3"), // SDL_SCANCODE_KP_3
    (92, "numpad4"), // SDL_SCANCODE_KP_4
    (93, "numpad5"), // SDL_SCANCODE_KP_5
    (94, "numpad6"), // SDL_SCANCODE_KP_6
    (95, "numpad7"), // SDL_SCANCODE_KP_7
    (96, "numpad8"), // SDL_SCANCODE_KP_8
    (97, "numpad9"), // SDL_SCANCODE_KP_9
    (98, "numpad0"), // SDL_SCANCODE_KP_0
    (99, "numpad."), // SDL_SCANCODE_KP_PERIOD
    (224, "control"), // SDL_SCANCODE_LCTRL
    (225, "shift"), // SDL_SCANCODE_LSHIFT
    (226, "alt"), // SDL_SCANCODE_LALT
    (228, "control"), // SDL_SCANCODE_RCTRL
    (229, "shift"), // SDL_SCANCODE_RSHIFT
    (230, "alt"), // SDL_SCANCODE_RALT
];

pub fn build_input_table(
    lua: &Lua,
    keys: &[u8; 256],
    xmouse: i32,
    ymouse: i32,
    click: i32,
) -> Result<Table> {
    let table = lua.create_table()?;
    // v2.0.4 V25: keys[] holds raw key state (g_keyState is 0/1 from SDL
    // events; the FFI bridge may normalize any pressed key to 0x80), so the
    // pressed test is "value != 0". The old per-key 0x80/0x01 mask rule was
    // built for the VK toggle-state convention and does not apply here.
    for (sc, name) in KEY_TO_NAME {
        if keys.get(sc).map(|&v| v != 0).unwrap_or(false) {
            table.set(name, true)?;
        }
    }
    if xmouse >= 0 {
        table.set("xmouse", xmouse)?;
    }
    if ymouse >= 0 {
        table.set("ymouse", ymouse)?;
    }
    // v2.0.4 V22: GetMouseData OR-s the pressed buttons into a bit word
    // (left 0x01, right 0x02, middle 0x04), so buttons must be tested by bit;
    // the old `click == 1 / == 2` equality dropped both buttons when left and
    // right were held together (click == 3) and had no middle button at all.
    // The numeric "click" field (raw bit word, 0 when idle) matches upstream
    // FCEUX lua-engine.cpp, which lua_pushinteger's it unconditionally.
    table.set("click", click)?;
    if click & 0x01 != 0 {
        table.set("leftclick", true)?;
    }
    if click & 0x02 != 0 {
        table.set("rightclick", true)?;
    }
    if click & 0x04 != 0 {
        table.set("middleclick", true)?;
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
                return lua.create_table();
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
        lua.create_function(|lua, _options: Table| lua.create_table())?,
    )?;

    input.set(
        "savefilepopup",
        lua.create_function(|_, _options: Table| Ok(String::new()))?,
    )?;

    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::{Lua, Table};

    /// Boolean field reader: an absent field counts as "not pressed".
    fn get_bool(table: &Table, name: &str) -> bool {
        table.get::<bool>(name).unwrap_or(false)
    }

    #[test]
    fn test_build_input_table_empty() {
        unsafe {
            let lua = Lua::new();
            let keys = [0u8; 256];
            let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
            let count: i64 = table.len().unwrap();
            assert_eq!(count, 0);
        }
    }

    // v2.0.4 V25: the keys buffer is indexed by SDL scancode (g_keyState is
    // filled at input.cpp by `g_keyState[event.key.keysym.scancode]`), so all
    // test literals below are SDL scancodes taken from SDL_scancode.h
    // (vcpkg SDL2, SDL_NUM_SCANCODES = 512; the FFI window copies indices
    // 0..255, see fceWrapper_GetKeyboardState). The old VK literals
    // (0x10 = shift, 0x14 = capslock, ...) were the V25 defect: VK codes were
    // used to index a scancode array, misnaming every key.
    #[test]
    fn test_build_input_table_shift_pressed() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[225] = 1; // SDL_SCANCODE_LSHIFT
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(table.get::<bool>("shift").unwrap(), true);
        assert_eq!(get_bool(&table, "control"), false);
    }

    #[test]
    fn test_build_input_table_shift_right_side() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[229] = 1; // SDL_SCANCODE_RSHIFT
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(get_bool(&table, "shift"), true);
    }

    #[test]
    fn test_build_input_table_capslock_toggle() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[57] = 1; // SDL_SCANCODE_CAPSLOCK
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(table.get::<bool>("capslock").unwrap(), true);
    }

    #[test]
    fn test_build_input_table_mouse_coords() {
        unsafe {
            let lua = Lua::new();
            let keys = [0u8; 256];
            let table = build_input_table(&lua, &keys, 100, 200, 0).unwrap();
            assert_eq!(table.get::<i32>("xmouse").unwrap(), 100);
            assert_eq!(table.get::<i32>("ymouse").unwrap(), 200);
        }
    }

    #[test]
    fn test_build_input_table_left_click() {
        unsafe {
            let lua = Lua::new();
            let keys = [0u8; 256];
            let table = build_input_table(&lua, &keys, 50, 60, 1).unwrap();
            assert_eq!(table.get::<bool>("leftclick").unwrap(), true);
            assert_eq!(table.get::<i32>("xmouse").unwrap(), 50);
            assert_eq!(table.get::<i32>("ymouse").unwrap(), 60);
        }
    }

    #[test]
    fn test_build_input_table_right_click() {
        unsafe {
            let lua = Lua::new();
            let keys = [0u8; 256];
            let table = build_input_table(&lua, &keys, 10, 20, 2).unwrap();
            assert_eq!(table.get::<bool>("rightclick").unwrap(), true);
        }
    }

    #[test]
    fn test_build_input_table_multiple_keys() {
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[26] = 0x80; // SDL_SCANCODE_W
        keys[4] = 0x80; // SDL_SCANCODE_A
        keys[225] = 0x80; // SDL_SCANCODE_LSHIFT
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(table.get::<bool>("W").unwrap(), true);
        assert_eq!(table.get::<bool>("A").unwrap(), true);
        assert_eq!(table.get::<bool>("shift").unwrap(), true);
    }

    #[test]
    fn test_build_input_table_no_mouse_on_negative() {
        unsafe {
            let lua = Lua::new();
            let keys = [0u8; 256];
            let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
            assert!(table.get::<i32>("xmouse").is_err());
            assert!(table.get::<i32>("ymouse").is_err());
        }
    }

    #[test]
    fn test_key_to_name_coverage() {
        // Production-normalized form: fceWrapper_GetKeyboardState maps every
        // pressed key to 0x80. Every entry of the table must report true.
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        for (sc, _) in KEY_TO_NAME {
            assert!(sc < 256, "scancode {} is outside the 256-byte FFI window", sc);
            keys[sc] = 0x80;
        }
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        for (_, name) in KEY_TO_NAME {
            assert!(get_bool(&table, name), "key {} should be detected as pressed", name);
        }
    }

    #[test]
    fn test_key_to_name_coverage_raw_event_values() {
        // v2.0.4 V25 mask leg: g_keyState itself holds 0/1 (input.cpp:
        // `g_keyState[event.key.keysym.scancode] = (event.type == SDL_KEYDOWN)
        // ? 1 : 0`). A pressed key must be detected from the raw event value
        // too, so the check has to be "value != 0", not a 0x80 bitmask.
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        for (sc, _) in KEY_TO_NAME {
            keys[sc] = 1;
        }
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        for (_, name) in KEY_TO_NAME {
            assert!(
                get_bool(&table, name),
                "key {} should be detected with raw value 1",
                name
            );
        }
    }

    #[test]
    fn test_scancode_spot_checks_against_sdl_header() {
        // Independent expectations keyed to SDL_scancode.h (vcpkg SDL2), not
        // derived from KEY_TO_NAME: CAPSLOCK=57, BACKSPACE=42, RIGHT=79,
        // LEFT=80, DOWN=81, UP=82, F1=58, F12=69, LCTRL=224, RCTRL=228,
        // LALT=226, RALT=230. Q=20 and E=8 are the scancodes the old VK-based
        // table landed on (VK 0x14 capslock -> scancode 20 = Q, VK 0x08
        // backspace -> scancode 8 = E); they must NOT report the old names.
        let cases: [(usize, &str); 10] = [
            (57, "capslock"),
            (42, "backspace"),
            (79, "right"),
            (80, "left"),
            (81, "down"),
            (82, "up"),
            (58, "F1"),
            (69, "F12"),
            (224, "control"),
            (226, "alt"),
        ];
        for (sc, name) in cases {
            let lua = Lua::new();
            let mut keys = [0u8; 256];
            keys[sc] = 1;
            let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
            assert!(
                get_bool(&table, name),
                "scancode {} should report as {}",
                sc,
                name
            );
        }
        // Right-hand variants of the modifiers share the Lua-facing name.
        for (sc, name) in [(228usize, "control"), (230usize, "alt"), (229usize, "shift")] {
            let lua = Lua::new();
            let mut keys = [0u8; 256];
            keys[sc] = 1;
            let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
            assert!(get_bool(&table, name), "scancode {} should report as {}", sc, name);
        }
    }

    #[test]
    fn test_old_vk_indices_do_not_report_old_names() {
        // v2.0.4 V25 domain leg: the old table indexed the scancode array with
        // Windows VK codes, so "capslock" was read at scancode 20 (Q) and
        // "backspace" at scancode 8 (E). Lock the corrected alignment from
        // both sides.
        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[20] = 1; // SDL_SCANCODE_Q (old "capslock" read position)
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(get_bool(&table, "capslock"), false);
        assert_eq!(table.get::<bool>("Q").unwrap(), true);

        let lua = Lua::new();
        let mut keys = [0u8; 256];
        keys[8] = 1; // SDL_SCANCODE_E (old "backspace" read position)
        let table = build_input_table(&lua, &keys, -1, -1, 0).unwrap();
        assert_eq!(get_bool(&table, "backspace"), false);
        assert_eq!(table.get::<bool>("E").unwrap(), true);
    }

    #[test]
    fn test_mouse_click_bitmask_combinations() {
        // v2.0.4 V22: GetMouseData builds the button word by OR-ing bits
        // (left 0x01, right 0x02, middle 0x04), so the Lua side must test
        // bits, not equality; the numeric "click" field carries the raw word
        // (upstream lua-engine.cpp parity).
        struct Case {
            click: i32,
            left: bool,
            right: bool,
            middle: bool,
        }
        let cases = [
            Case { click: 0, left: false, right: false, middle: false },
            Case { click: 1, left: true, right: false, middle: false },
            Case { click: 2, left: false, right: true, middle: false },
            Case { click: 3, left: true, right: true, middle: false },
            Case { click: 4, left: false, right: false, middle: true },
            Case { click: 5, left: true, right: false, middle: true },
            Case { click: 6, left: false, right: true, middle: true },
            Case { click: 7, left: true, right: true, middle: true },
        ];
        for c in cases {
            let lua = Lua::new();
            let keys = [0u8; 256];
            let table = build_input_table(&lua, &keys, -1, -1, c.click).unwrap();
            assert_eq!(get_bool(&table, "leftclick"), c.left, "click={}", c.click);
            assert_eq!(get_bool(&table, "rightclick"), c.right, "click={}", c.click);
            assert_eq!(get_bool(&table, "middleclick"), c.middle, "click={}", c.click);
            let raw: i32 = table.get("click").unwrap();
            assert_eq!(raw, c.click, "numeric click field for click={}", c.click);
        }
    }
}
