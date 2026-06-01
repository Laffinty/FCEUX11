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

#[unsafe(no_mangle)]
pub(crate) fn get_engine_mut<'a>() -> Option<&'a mut LuaEngine> {
    unsafe { (LUA_ENGINE_PTR as *mut LuaEngine).as_mut() }
}

// ---------------------------------------------------------------------------
// FFI declarations for C++ hooks
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fceux11_lua_GetMem(addr: u32) -> u8;
    fn fceux11_lua_BWrite(addr: u32, val: u8);
    fn fceux11_lua_GetRegister(reg_id: i32) -> u16;
    fn fceux11_lua_GetJoypadState(port: i32) -> u32;
    fn fceux11_lua_SetJoypadOverride(port: i32, mask1: u32, mask2: u32);
    #[allow(dead_code)]
    fn fceux11_lua_GetRomHash(which: i32) -> u32;
    fn fceux11_lua_ReadRomByte(addr: u32) -> u8;
    fn fceux11_lua_WriteRomByte(addr: u32, val: u8);
    fn fceux11_lua_PPURead(addr: u32) -> u8;
    fn fceux11_lua_movie_get_mode() -> i32;
    fn fceux11_lua_movie_get_rerecordcount() -> i64;
    fn fceux11_lua_movie_get_length() -> i64;
    fn fceux11_lua_movie_stop();
    fn fceux11_lua_movie_get_readonly() -> i32;
    fn fceux11_lua_movie_set_readonly(val: i32);
    fn fceux11_lua_movie_is_poweron() -> i32;
    fn fceux11_lua_movie_is_from_savestate() -> i32;
    fn fceux11_lua_movie_get_name() -> *const c_char;
    fn fceux11_lua_movie_get_filename() -> *const c_char;
    fn fceux11_lua_savestate_save_slot(slot: i32) -> i32;
    fn fceux11_lua_savestate_load_slot(slot: i32) -> i32;
    fn fceux11_lua_emu_get_framecount() -> i64;
    fn fceux11_lua_emu_get_lagcount() -> i64;
    fn fceux11_lua_emu_is_paused() -> i32;
    fn fceux11_lua_emu_set_speedmode(mode: i32);
    fn fceux11_lua_emu_poweron();
    fn fceux11_lua_emu_softreset();
    fn fceux11_lua_emu_message(msg: *const c_char);
    fn fceux11_lua_emu_pause();
    fn fceux11_lua_emu_unpause();
    fn fceux11_lua_savestate_create_object(path: *const c_char, which: i32, anonymous: i32) -> i32;
    #[allow(dead_code)]
    fn fceux11_lua_savestate_delete_object(obj_id: i32);
    fn fceux11_lua_gui_popup(msg: *const c_char);
    fn fceux11_lua_gui_savescreenshot(filename: *const c_char);

    // --- P3: Sound ---
    fn fceux11_lua_sound_get_square1_volume() -> f64;
    fn fceux11_lua_sound_get_square1_frequency() -> f64;
    fn fceux11_lua_sound_get_square1_midikey() -> f64;
    fn fceux11_lua_sound_get_square1_duty() -> i32;
    fn fceux11_lua_sound_get_square1_regs() -> i32;
    fn fceux11_lua_sound_get_square2_volume() -> f64;
    fn fceux11_lua_sound_get_square2_frequency() -> f64;
    fn fceux11_lua_sound_get_square2_midikey() -> f64;
    fn fceux11_lua_sound_get_square2_duty() -> i32;
    fn fceux11_lua_sound_get_square2_regs() -> i32;
    fn fceux11_lua_sound_get_triangle_volume() -> f64;
    fn fceux11_lua_sound_get_triangle_linear() -> i32;
    fn fceux11_lua_sound_get_triangle_frequency() -> f64;
    fn fceux11_lua_sound_get_triangle_midikey() -> f64;
    fn fceux11_lua_sound_get_noise_volume() -> f64;
    fn fceux11_lua_sound_get_noise_mode() -> i32;
    fn fceux11_lua_sound_get_noise_regs() -> i32;
    fn fceux11_lua_sound_get_noise_frequency() -> f64;
    fn fceux11_lua_sound_get_noise_midikey() -> f64;
    fn fceux11_lua_sound_get_dmc_volume() -> f64;
    fn fceux11_lua_sound_get_dmc_rate() -> i32;
    fn fceux11_lua_sound_get_dmc_regs() -> i32;
    fn fceux11_lua_sound_get_dmc_frequency() -> f64;
    fn fceux11_lua_sound_get_dmc_midikey() -> f64;
    fn fceux11_lua_sound_get_dmc_address() -> i32;
    fn fceux11_lua_sound_get_dmc_size() -> i32;
    fn fceux11_lua_sound_get_dmc_loop() -> i32;
    fn fceux11_lua_sound_get_dmc_seed() -> i32;
    fn fceux11_lua_sound_get_frame_sequencer() -> i32;
    fn fceux11_lua_sound_get_sample_rate() -> i32;
    fn fceux11_lua_sound_get_length_count() -> i32;

    // --- P3: Zapper ---
    fn fceux11_lua_zapper_get_x() -> i32;
    fn fceux11_lua_zapper_get_y() -> i32;
    fn fceux11_lua_zapper_get_click() -> i32;
    fn fceux11_lua_zapper_set(x: i32, y: i32, fire: i32);

    // --- P3: Debugger ---
    fn fceux11_lua_debugger_hitbreakpoint();
    fn fceux11_lua_debugger_get_cycles_count() -> u64;
    fn fceux11_lua_debugger_get_instructions_count() -> u64;
    fn fceux11_lua_debugger_reset_cycles_count();
    fn fceux11_lua_debugger_reset_instructions_count();
    fn fceux11_lua_debugger_get_symbol_offset(name: *const c_char) -> i64;
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
        bindings::memory::register(&lua)?;
        bindings::joypad::register(&lua)?;
        bindings::rom::register(&lua)?;
        bindings::ppu::register(&lua)?;
        bindings::input::register(&lua)?;
        bindings::sound::register(&lua)?;
        bindings::movie::register(&lua)?;
        bindings::savestate::register(&lua)?;
        bindings::gui::register(&lua)?;
        bindings::zapper::register(&lua)?;
        bindings::debugger::register(&lua)?;
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

// Tests for bit operations live in bindings/bit.rs alongside the implementation.
// All other bindings are tested via integration tests (tests/lua_scripts/).