//! `fceux11-lua` — Rust bindings for the FCEUX Lua engine
//!
//! Core design: `LuaEngine` owns an `mlua::Lua` instance and a main coroutine
//! (`mlua::Thread`). Scripts run inside the coroutine. `emu.frameadvance()`
//! yields the coroutine; `frame_boundary()` resumes it. This mirrors the C++
//! `lua_yield` / `lua_resume` cycle exactly.

#![allow(unsafe_op_in_unsafe_fn)]

pub mod bindings;

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_uint;

use mlua::{Lua, RegistryKey, Thread};

static mut LUA_ENGINE_PTR: *mut c_void = std::ptr::null_mut();

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
    fn fceux11_lua_GetRomMD5(buf: *mut u8) -> i32;
    fn fceux11_lua_ReadRomByte(addr: u32) -> u8;
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    fn fceux11_lua_sound_get_dmc_rate() -> i32;
    fn fceux11_lua_sound_get_dmc_regs() -> i32;
    fn fceux11_lua_sound_get_dmc_frequency() -> f64;
    fn fceux11_lua_sound_get_dmc_midikey() -> f64;
    fn fceux11_lua_sound_get_dmc_address() -> i32;
    fn fceux11_lua_sound_get_dmc_size() -> i32;
    fn fceux11_lua_sound_get_dmc_loop() -> i32;
    fn fceux11_lua_sound_get_dmc_seed() -> i32;
    #[allow(dead_code)]
    fn fceux11_lua_sound_get_frame_sequencer() -> i32;
    fn fceux11_lua_sound_get_sample_rate() -> i32;
    fn fceux11_lua_sound_get_length_count() -> i32;

    fn fceux11_lua_zapper_get_x() -> i32;
    fn fceux11_lua_zapper_get_y() -> i32;
    fn fceux11_lua_zapper_get_click() -> i32;
    fn fceux11_lua_zapper_set(x: i32, y: i32, fire: i32);

    fn fceux11_lua_debugger_hitbreakpoint();
    fn fceux11_lua_debugger_get_cycles_count() -> u64;
    fn fceux11_lua_debugger_get_instructions_count() -> u64;
    fn fceux11_lua_debugger_reset_cycles_count();
    fn fceux11_lua_debugger_reset_instructions_count();
    fn fceux11_lua_debugger_get_symbol_offset(name: *const c_char) -> i64;

    fn fceux11_lua_GetKeyboardState(keys: *mut u8) -> i32;
    fn fceux11_lua_GetMouseState(x: *mut i32, y: *mut i32, click: *mut i32);
}

// ---------------------------------------------------------------------------
// LuaCallID / LuaMemHookType
// ---------------------------------------------------------------------------

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum LuaCallID {
    BeforeEmulation = 0,
    AfterEmulation = 1,
    BeforeExit = 2,
    BeforeSave = 3,
    AfterLoad = 4,
    TaseditorAuto = 5,
    TaseditorManual = 6,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LuaMemHookType {
    Write = 0,
    Read = 1,
    Exec = 2,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SpeedMode {
    #[default]
    Normal,
    NoThrottle,
    Turbo,
    Maximum,
}

#[derive(Debug, Clone, Default)]
pub struct JoypadOverride {
    pub mask1: [u32; 4],
    pub mask2: [u32; 4],
}

// ---------------------------------------------------------------------------
// RegisteredCallbacks — persist RegistryKey across yields
// ---------------------------------------------------------------------------

pub struct CallbackEntry {
    pub func: mlua::Function,
    pub _key: RegistryKey,
}

pub struct RegisteredCallbacks {
    before: Vec<CallbackEntry>,
    after: Vec<CallbackEntry>,
    exit: Vec<CallbackEntry>,
    gui: Vec<CallbackEntry>,
    savestate_save: Vec<CallbackEntry>,
    savestate_load: Vec<CallbackEntry>,
    mem_hooks: HashMap<(u32, LuaMemHookType), Vec<CallbackEntry>>,
}

impl RegisteredCallbacks {
    fn new() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
            exit: Vec::new(),
            gui: Vec::new(),
            savestate_save: Vec::new(),
            savestate_load: Vec::new(),
            mem_hooks: HashMap::new(),
        }
    }

    fn keys_for(&mut self, call_id: LuaCallID) -> &mut Vec<CallbackEntry> {
        match call_id {
            LuaCallID::BeforeEmulation => &mut self.before,
            LuaCallID::AfterEmulation => &mut self.after,
            LuaCallID::BeforeExit => &mut self.exit,
            _ => &mut self.before,
        }
    }
}

// ---------------------------------------------------------------------------
// LuaEngine
// ---------------------------------------------------------------------------

pub struct LuaEngine {
    lua: Lua,
    main_thread: Thread,
    gui_data: Vec<u8>,
    joypad_state: JoypadOverride,
    speed_mode: SpeedMode,
    transparency_modifier: u8,
    running: bool,
    frame_advance_waiting: bool,
    callbacks: RegisteredCallbacks,
    script_name: Option<String>,
}

const FRAME_ADVANCE_THREAD: &str = "FCEUX11.FrameAdvance";
const GUI_WIDTH: usize = 256;
const GUI_HEIGHT: usize = 240;

impl LuaEngine {
    pub fn new() -> mlua::Result<Self> {
        let lua = Lua::new();

        let main_thread = lua.create_thread(lua.create_function(|_, ()| Ok(()))?)?;

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
            main_thread,
            gui_data: vec![0u8; GUI_WIDTH * GUI_HEIGHT * 4],
            joypad_state: JoypadOverride::default(),
            speed_mode: SpeedMode::Normal,
            transparency_modifier: 255,
            running: false,
            frame_advance_waiting: false,
            callbacks: RegisteredCallbacks::new(),
            script_name: None,
        })
    }

    pub fn load_script(&mut self, path: &str, arg: Option<&str>) -> mlua::Result<()> {
        let script = std::fs::read_to_string(path).map_err(mlua::Error::external)?;

        let arg_table = self.lua.create_table()?;
        if let Some(s) = arg {
            arg_table.set(1, s)?;
        }
        self.lua.globals().set("arg", arg_table)?;

        let func = self.lua.load(&script).into_function()?;

        self.main_thread = self.lua.create_thread(func)?;

        self.lua
            .set_named_registry_value(FRAME_ADVANCE_THREAD, self.main_thread.clone())?;

        self.callbacks = RegisteredCallbacks::new();
        self.clear_gui();
        self.frame_advance_waiting = false;
        self.running = true;
        self.script_name = Some(path.to_string());

        let first_result = self.main_thread.resume::<()>(());
        match first_result {
            Ok(()) => {}
            Err(mlua::Error::CoroutineUnresumable) => {}
            Err(e) => {
                eprintln!("Lua script load error: {:?}", e);
                self.running = false;
                return Err(e);
            }
        }

        if self.main_thread.status() == mlua::ThreadStatus::Resumable {
            self.frame_advance_waiting = true;
        } else {
            self.on_script_end();
        }

        Ok(())
    }

    pub fn frame_boundary(&mut self) -> mlua::Result<()> {
        if !self.running {
            return Ok(());
        }

        self.call_registered(LuaCallID::BeforeEmulation).ok();

        if self.main_thread.status() != mlua::ThreadStatus::Resumable {
            self.on_script_end();
            return Ok(());
        }

        self.frame_advance_waiting = false;

        let result = self.main_thread.resume::<()>(());
        match result {
            Ok(()) => {}
            Err(mlua::Error::CoroutineUnresumable) => {}
            Err(e) => {
                eprintln!("Lua frame_boundary error: {:?}", e);
                self.on_script_end();
                return Ok(());
            }
        }

        match self.main_thread.status() {
            mlua::ThreadStatus::Resumable => {
                self.frame_advance_waiting = true;
            }
            mlua::ThreadStatus::Finished
            | mlua::ThreadStatus::Error
            | mlua::ThreadStatus::Running => {
                self.on_script_end();
                return Ok(());
            }
        }

        self.call_registered(LuaCallID::AfterEmulation).ok();

        Ok(())
    }

    fn on_script_end(&mut self) {
        self.call_registered(LuaCallID::BeforeExit).ok();
        self.running = false;
        self.frame_advance_waiting = false;
        self.clear_gui();
    }

    pub fn stop(&mut self) {
        if self.running {
            self.on_script_end();
        }
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn call_registered(&mut self, call_id: LuaCallID) -> mlua::Result<()> {
        let entries = self.callbacks.keys_for(call_id);
        for entry in entries {
            let _ = entry.func.call::<()>(());
        }
        Ok(())
    }

    pub fn register_callback(
        &mut self,
        call_id: LuaCallID,
        func: mlua::Function,
    ) -> mlua::Result<()> {
        let key = self.lua.create_registry_value(func.clone())?;
        self.callbacks
            .keys_for(call_id)
            .push(CallbackEntry { func, _key: key });
        Ok(())
    }

    pub fn register_mem_hook(
        &mut self,
        addr: u32,
        hook_type: LuaMemHookType,
        func: mlua::Function,
    ) -> mlua::Result<()> {
        let key = self.lua.create_registry_value(func.clone())?;
        self.callbacks
            .mem_hooks
            .entry((addr, hook_type))
            .or_default()
            .push(CallbackEntry { func, _key: key });
        Ok(())
    }

    pub fn call_mem_hook(&mut self, addr: u32, _size: i32, _value: u32, hook_type: LuaMemHookType) {
        if let Some(entries) = self.callbacks.mem_hooks.get(&(addr, hook_type)) {
            for entry in entries {
                let _ = entry.func.call::<()>(());
            }
        }
    }

    pub fn gui_overlay(&self, xbuf: &mut [u8], width: i32, height: i32) {
        if width != GUI_WIDTH as i32 || height != GUI_HEIGHT as i32 {
            return;
        }
        let src = &self.gui_data;
        let dst_len = (width as usize) * (height as usize);
        if dst_len > xbuf.len() {
            return;
        }
        for (dst_i, src_i) in (0..dst_len).zip((0..src.len()).step_by(4)) {
            if src_i + 3 < src.len() {
                let a = src[src_i + 3];
                if a == 0 {
                    continue;
                }
                if a >= 250 {
                    xbuf[dst_i] = src[src_i];
                }
            }
        }
    }

    pub fn read_joypad(&self, controller: i32, original: u8) -> u8 {
        if controller < 0 || controller > 3 {
            return original;
        }
        let i = controller as usize;
        let mask1 = self.joypad_state.mask1[i] as u8;
        let mask2 = self.joypad_state.mask2[i] as u8;
        (original & mask1) | mask2
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) -> mlua::Result<()> {
        if x < 0 || x >= GUI_WIDTH as i32 || y < 0 || y >= GUI_HEIGHT as i32 {
            return Ok(());
        }
        let idx = ((y as usize * GUI_WIDTH) + x as usize) * 4;
        if idx + 3 < self.gui_data.len() {
            let rgba = color.to_le_bytes();
            self.gui_data[idx] = rgba[0];
            self.gui_data[idx + 1] = rgba[1];
            self.gui_data[idx + 2] = rgba[2];
            self.gui_data[idx + 3] = rgba[3];
        }
        Ok(())
    }

    fn clear_gui(&mut self) {
        for b in &mut self.gui_data {
            *b = 0;
        }
    }

    #[inline]
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    #[inline]
    pub fn main_thread(&self) -> &Thread {
        &self.main_thread
    }

    #[inline]
    pub fn frame_advance_waiting(&self) -> bool {
        self.frame_advance_waiting
    }

    pub fn set_frame_advance_waiting(&mut self, waiting: bool) {
        self.frame_advance_waiting = waiting;
    }

    pub fn set_joypad_override(&mut self, port: i32, mask1: u32, mask2: u32) {
        if port >= 0 && port < 4 {
            self.joypad_state.mask1[port as usize] = mask1;
            self.joypad_state.mask2[port as usize] = mask2;
        }
    }

    pub fn callbacks(&mut self) -> &mut RegisteredCallbacks {
        &mut self.callbacks
    }
}

// ---------------------------------------------------------------------------
// FFI bridge — called from C++
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_init() -> c_int {
    match LuaEngine::new() {
        Ok(engine) => {
            let boxed = Box::new(engine);
            LUA_ENGINE_PTR = Box::into_raw(boxed) as *mut c_void;
            0
        }
        Err(e) => {
            eprintln!("fceux11_lua_init failed: {:?}", e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_load_script(path: *const c_char, arg: *const c_char) -> c_int {
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
    let total = (width as usize) * (height as usize);
    if total == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(xbuf, total);
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
        0 => LuaCallID::BeforeEmulation,
        1 => LuaCallID::AfterEmulation,
        2 => LuaCallID::BeforeExit,
        3 => LuaCallID::BeforeSave,
        4 => LuaCallID::AfterLoad,
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
        0 => LuaMemHookType::Write,
        1 => LuaMemHookType::Read,
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

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_get_mem_hook_count(hook_type: c_int) -> c_int {
    let ht = match hook_type {
        0 => LuaMemHookType::Write,
        1 => LuaMemHookType::Read,
        2 => LuaMemHookType::Exec,
        _ => return 0,
    };
    match get_engine() {
        Some(engine) => engine.callbacks.mem_hooks.keys().filter(|( _, t)| *t == ht).count() as c_int,
        None => 0,
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_get_mem_hook_address(hook_type: c_int, index: c_int) -> u32 {
    let ht = match hook_type {
        0 => LuaMemHookType::Write,
        1 => LuaMemHookType::Read,
        2 => LuaMemHookType::Exec,
        _ => return 0,
    };
    match get_engine() {
        Some(engine) => {
            let addrs: Vec<u32> = engine.callbacks.mem_hooks.keys()
                .filter(|(_, t)| *t == ht)
                .map(|(addr, _)| *addr)
                .collect();
            if index >= 0 && (index as usize) < addrs.len() {
                addrs[index as usize]
            } else {
                0
            }
        }
        None => 0,
    }
}
