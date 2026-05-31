//! `fceux11-lua` — Rust bindings for the FCEUX Lua engine

#![allow(unsafe_op_in_unsafe_fn)]

pub mod bindings;

use std::ffi::{c_char, c_int};
use std::os::raw::c_uint;

use mlua::Lua;

// ---------------------------------------------------------------------------
// Global engine pointer (raw pointer — Lua is !Sync)
// ---------------------------------------------------------------------------

static mut LUA_ENGINE_PTR: *mut std::ffi::c_void = std::ptr::null_mut();

#[inline(always)]
fn get_engine<'a>() -> Option<&'a mut LuaEngine> {
    unsafe { (LUA_ENGINE_PTR as *mut LuaEngine).as_mut() }
}

// ---------------------------------------------------------------------------
// LuaEngine
// ---------------------------------------------------------------------------

/// Lua call IDs for registered callbacks
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum LuaCallID {
    Before = 0,
    After = 1,
    Exit = 2,
}

/// Lua memory hook types
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum LuaMemHookType {
    Read = 0,
    Write = 1,
    Exec = 2,
}

/// Speed mode for emu.speedmode()
#[derive(Debug, Clone, Copy, Default)]
pub enum SpeedMode {
    #[default]
    Normal,
    NoThrottle,
    Turbo,
    Maximum,
}

/// Joypad override state
#[derive(Debug, Clone, Default)]
pub struct JoypadOverride {
    pub mask1: [u32; 4],
    pub mask2: [u32; 4],
}

/// The FCEUX Lua engine
pub struct LuaEngine {
    lua: Lua,
    gui_data: Vec<u8>,
    joypad_state: JoypadOverride,
    speed_mode: SpeedMode,
    transparency_modifier: u8,
    running: bool,
}

impl LuaEngine {
    pub fn new() -> mlua::Result<Self> {
        let lua = Lua::new();
        bindings::bit::register(&lua)?;
        bindings::emu::register(&lua)?;
        Ok(Self {
            lua,
            gui_data: vec![0u8; 256 * 240 * 4],
            joypad_state: JoypadOverride::default(),
            speed_mode: SpeedMode::Normal,
            transparency_modifier: 255,
            running: false,
        })
    }

    pub fn load_script(&mut self, path: &str, arg: Option<&str>) -> mlua::Result<()> {
        let script =
            std::fs::read_to_string(path).map_err(mlua::Error::external)?;

        let _arg_table: mlua::Table = match arg {
            Some(s) => {
                let t = self.lua.create_table()?;
                t.set(1, s)?;
                t
            }
            None => self.lua.create_table()?,
        };

        self.lua.load(&script).exec()?;
        self.running = true;
        Ok(())
    }

    pub fn frame_boundary(&mut self) -> mlua::Result<()> {
        // TODO (v0.2.22.2): resume co-routines that yielded via frameadvance
        self.running = false;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn call_registered(&mut self, _call_id: LuaCallID) -> mlua::Result<()> {
        Ok(())
    }

    pub fn call_mem_hook(
        &mut self,
        _addr: u32,
        _size: i32,
        _value: u32,
        _hook_type: LuaMemHookType,
    ) {
    }

    pub fn gui_overlay(&self, xbuf: &mut [u8], _width: i32, _height: i32) {
        let len = std::cmp::min(self.gui_data.len(), xbuf.len());
        xbuf[..len].copy_from_slice(&self.gui_data[..len]);
    }

    pub fn read_joypad(&self, _controller: i32, original: u8) -> u8 {
        original
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) -> mlua::Result<()> {
        if x < 0 || x >= 256 || y < 0 || y >= 240 {
            return Ok(());
        }
        let idx = ((y as usize * 256) + x as usize) * 4;
        if idx + 3 < self.gui_data.len() {
            let rgba = color.to_le_bytes();
            self.gui_data[idx] = rgba[0];
            self.gui_data[idx + 1] = rgba[1];
            self.gui_data[idx + 2] = rgba[2];
            self.gui_data[idx + 3] = rgba[3];
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FFI bridge — called from C++
// ---------------------------------------------------------------------------

// These functions are invoked from C++ via FFI. They manipulate a static
// mutable raw pointer, so each function body must be inside `unsafe {}`.

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_init() -> c_int {
    match LuaEngine::new() {
        Ok(engine) => {
            let boxed = Box::new(engine);
            LUA_ENGINE_PTR = Box::into_raw(boxed) as *mut std::ffi::c_void;
            0
        }
        Err(e) => {
            eprintln!("fceux11_lua_init failed: {:?}", e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_load_script(
    path: *const c_char,
    arg: *const c_char,
) -> c_int {
    let path = std::ffi::CStr::from_ptr(path)
        .to_string_lossy()
        .into_owned();
    let arg = if arg.is_null() {
        None
    } else {
        Some(std::ffi::CStr::from_ptr(arg).to_string_lossy().into_owned())
    };

    match get_engine() {
        Some(engine) => match engine.load_script(&path, arg.as_deref()) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("fceux11_lua_load_script failed: {:?}", e);
                -1
            }
        },
        None => -1,
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_frame_boundary() {
    if let Some(engine) = get_engine() {
        if let Err(e) = engine.frame_boundary() {
            eprintln!("fceux11_lua_frame_boundary failed: {:?}", e);
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_stop() {
    if let Some(engine) = get_engine() {
        engine.stop();
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_running() -> c_int {
    match get_engine() {
        Some(engine) if engine.is_running() => 1,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_gui(xbuf: *mut u8, width: c_int, height: c_int) {
    if xbuf.is_null() {
        return;
    }
    let slice =
        std::slice::from_raw_parts_mut(xbuf, (width as usize) * (height as usize) * 4);
    if let Some(engine) = get_engine() {
        engine.gui_overlay(slice, width, height);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_read_joypad(controller: c_int, original: u8) -> u8 {
    match get_engine() {
        Some(engine) => engine.read_joypad(controller, original),
        None => original,
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_call_registered(call_id: c_int) -> c_int {
    let id = match call_id {
        0 => LuaCallID::Before,
        1 => LuaCallID::After,
        2 => LuaCallID::Exit,
        _ => return -1,
    };
    match get_engine() {
        Some(engine) => match engine.call_registered(id) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("fceux11_lua_call_registered failed: {:?}", e);
                -1
            }
        },
        None => -1,
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_call_mem_hook(
    addr: c_uint,
    size: c_int,
    value: c_uint,
    hook_type: c_int,
) {
    let ht = match hook_type {
        0 => LuaMemHookType::Read,
        1 => LuaMemHookType::Write,
        2 => LuaMemHookType::Exec,
        _ => return,
    };
    if let Some(engine) = get_engine() {
        engine.call_mem_hook(addr, size, value, ht);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_gui_pixel(x: c_int, y: c_int, color: c_uint) -> c_int {
    match get_engine() {
        Some(engine) => match engine.set_pixel(x, y, color) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("fceux11_lua_gui_pixel failed: {:?}", e);
                -1
            }
        },
        None => -1,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_bit_bor() {
        let lua = Lua::new();
        let bit = bindings::bit::register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let result: i32 = lua.load("bit.bor(0xFF, 0x00)").eval().unwrap();
        assert_eq!(result, 0xFF);
    }

    #[test]
    fn test_bit_band() {
        let lua = Lua::new();
        let bit = bindings::bit::register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let result: i32 = lua.load("bit.band(0xFF, 0x0F)").eval().unwrap();
        assert_eq!(result, 0x0F);
    }

    #[test]
    fn test_bit_lshift() {
        let lua = Lua::new();
        let bit = bindings::bit::register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let result: i32 = lua.load("bit.lshift(1, 8)").eval().unwrap();
        assert_eq!(result, 256);
    }

    #[test]
    fn test_bit_rshift() {
        let lua = Lua::new();
        let bit = bindings::bit::register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let result: i32 = lua.load("bit.rshift(256, 8)").eval().unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_bit_tohex() {
        let lua = Lua::new();
        let bit = bindings::bit::register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let result: String = lua.load(r#"bit.tohex(255)"#).eval().unwrap();
        assert_eq!(result, "000000FF");
    }
}