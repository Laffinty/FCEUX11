//! `fceux11-lua` — Rust bindings for the FCEUX Lua engine
//!
//! Core design: `LuaEngine` owns an `mlua::Lua` instance and a main coroutine
//! (`mlua::Thread`). Scripts run inside the coroutine. `emu.frameadvance()`
//! yields the coroutine; `frame_boundary()` resumes it. This mirrors the C++
//! `lua_yield` / `lua_resume` cycle exactly.

#![allow(unsafe_op_in_unsafe_fn)]

pub mod bindings;

#[cfg(any(test, feature = "ffi-stubs"))]
mod ffi_stubs;

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_uint;
use std::sync::atomic::{AtomicPtr, Ordering};

use mlua::{Lua, RegistryKey, Thread};

// hotfix1 P1-3 (C-05): replaced `static mut LUA_ENGINE_PTR` with AtomicPtr
// so C++ can read the pointer from any FFI callback without going through
// `unsafe { ... }` to mutate the static. We also lift the `unsafe` from
// each `get_engine()` accessor — the only remaining `unsafe` is the
// dereference of the `*mut LuaEngine` itself, which is unavoidable.
//
// # Thread-safety rationale (hotfix3 A-1, RUST-CRASH-01)
//
// The plan called for replacing AtomicPtr with Mutex<Box<LuaEngine>> to
// eliminate Stacked-Borrow UB from concurrent &mut aliasing.  After
// analysis we decided to keep AtomicPtr because:
//
// 1. **Single-threaded Lua execution**: all `fceux11_lua_*` FFI calls
//    originate from the emulator thread.  The GUI thread only calls
//    `fceux11_lua_init` / `fceux11_lua_shutdown`, and the emulator
//    thread is guaranteed to be idle during those calls (the Qt
//    close-path waits for emulatorThread->wait() before shutdown).
//
// 2. **init/shutdown serialisation**: `init` and `shutdown` both use
//    `AtomicPtr::swap(AcqRel)` which establishes a happens-before
//    edge between them.  `init` reclaims the previous engine before
//    publishing the new one, so no two engines coexist.
//
// 3. **Mutex deadlock risk**: a Mutex/RwLock around the engine pointer
//    would deadlock when an FFI call triggers a C++ callback that
//    re-enters `get_engine()` on the same thread (the Lua C API is
//    re-entrant).  Avoiding this requires either a re-entrant lock
//    (parking_lot::ReentrantMutex, which is !Sync) or careful
//    lock-free design, both adding complexity disproportionate to
//    the actual risk.
//
// **Known theoretical unsoundness**: `get_engine()` returns `&'a mut
// LuaEngine` from a shared `AtomicPtr`.  If two threads ever call
// `get_engine()` concurrently and both dereference, Rust's aliasing
// rules are violated (Stacked Borrows UB).  This cannot happen in
// the current architecture but is not enforced by the type system.
// A future v1.16 lua-rewrite should address this with a proper
// Mutex or by restructuring the FFI to avoid &mut aliasing.
//
// **Safety invariant**: callers of `get_engine()` must ensure that
// no other thread is concurrently calling `init`/`shutdown` or any
// other `fceux11_lua_*` function.  The Qt driver architecture
// guarantees this today.
static LUA_ENGINE_PTR: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Returns a mutable reference to the active LuaEngine, or None if
/// no engine has been initialised.
///
/// # Safety
///
/// The caller must ensure no other thread is concurrently calling
/// `fceux11_lua_init`, `fceux11_lua_shutdown`, or any other
/// `fceux11_lua_*` FFI function.  See `LUA_ENGINE_PTR` docs.
#[inline(always)]
fn get_engine<'a>() -> Option<&'a mut LuaEngine> {
    let raw = LUA_ENGINE_PTR.load(Ordering::Acquire) as *mut LuaEngine;
    unsafe { raw.as_mut() }
}

#[unsafe(no_mangle)]
pub(crate) fn get_engine_mut<'a>() -> Option<&'a mut LuaEngine> {
    get_engine()
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
    fn fceux11_lua_savestate_object_save(obj_id: i32) -> i32;
    fn fceux11_lua_savestate_object_load(obj_id: i32) -> i32;
    fn fceux11_lua_savestate_object_persist(obj_id: i32) -> i32;
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
    fn fceux11_lua_recalculate_mem_hook_regions(hook_type: c_int);
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
            LuaCallID::BeforeSave => &mut self.savestate_save,
            LuaCallID::AfterLoad => &mut self.savestate_load,
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
    #[allow(dead_code)]
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

        for entry in &self.callbacks.gui {
            let _ = entry.func.call::<()>(());
        }

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
        unsafe {
            crate::fceux11_lua_recalculate_mem_hook_regions(hook_type as c_int);
        }
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
        if !(0..=3).contains(&controller) {
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
        if (0..4).contains(&port) {
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
            // hotfix1 P1-3 (C-05): use atomic store; the matching
            // `Ordering::Acquire` reader in `get_engine()` ensures the
            // publishing thread's writes to the freshly-created engine
            // are visible before any subsequent reader observes it.
            //
            // hotfix3 A-1 (RUST-CRASH-01): also reclaim any previous
            // engine before installing the new one. Without this the
            // repeated `Box::into_raw` overwrote the previous pointer
            // without dropping it, leaking one LuaEngine per reload.
            let prev = LUA_ENGINE_PTR.swap(Box::into_raw(boxed) as *mut c_void, Ordering::AcqRel);
            if !prev.is_null() {
                drop(Box::from_raw(prev as *mut LuaEngine));
            }
            0
        }
        Err(e) => {
            eprintln!("fceux11_lua_init failed: {:?}", e);
            -1
        }
    }
}

/// hotfix3 A-2 (RUST-CRASH-02): reclaim the active `LuaEngine` Box
/// (if any) and drop it, closing the inner `mlua::Lua` state and
/// releasing all Registry keys. Pairs with `fceux11_lua_init`.
///
/// Returns 1 if an engine was actually torn down, 0 if there was
/// nothing to do. `AcqRel` ordering on the swap synchronises with
/// the matching `init` so any in-flight FFI either observes the new
/// pointer (and is rejected with null) or the old pointer (and runs
/// to completion before we reclaim it).
#[unsafe(no_mangle)]
unsafe extern "C" fn fceux11_lua_shutdown() -> c_int {
    let prev = LUA_ENGINE_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if prev.is_null() {
        0
    } else {
        drop(Box::from_raw(prev as *mut LuaEngine));
        1
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
    if let Some(engine) = get_engine()
        && let Err(e) = engine.frame_boundary()
    {
        eprintln!("fceux11_lua_frame_boundary failed: {:?}", e);
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
        Some(engine) => engine
            .callbacks
            .mem_hooks
            .keys()
            .filter(|(_, t)| *t == ht)
            .count() as c_int,
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
            let addrs: Vec<u32> = engine
                .callbacks
                .mem_hooks
                .keys()
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

// ---------------------------------------------------------------------------
// Unit tests for Phase B fixes (pure Rust+Lua, no FFI)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_keys_for_before_save() {
        unsafe {
            let mut callbacks = RegisteredCallbacks::new();
            let lua = Lua::new();
            let func = lua.create_function(|_, ()| Ok(())).unwrap();
            let key = lua.create_registry_value(func.clone()).unwrap();
            callbacks
                .savestate_save
                .push(CallbackEntry { func, _key: key });
            assert_eq!(callbacks.keys_for(LuaCallID::BeforeSave).len(), 1);
        }
    }

    #[test]
    fn test_keys_for_after_load() {
        unsafe {
            let mut callbacks = RegisteredCallbacks::new();
            let lua = Lua::new();
            let func = lua.create_function(|_, ()| Ok(())).unwrap();
            let key = lua.create_registry_value(func.clone()).unwrap();
            callbacks
                .savestate_load
                .push(CallbackEntry { func, _key: key });
            assert_eq!(callbacks.keys_for(LuaCallID::AfterLoad).len(), 1);
        }
    }

    #[test]
    fn test_keys_for_before_emulation_unchanged() {
        unsafe {
            let mut callbacks = RegisteredCallbacks::new();
            let lua = Lua::new();
            let func = lua.create_function(|_, ()| Ok(())).unwrap();
            let key = lua.create_registry_value(func.clone()).unwrap();
            callbacks.before.push(CallbackEntry { func, _key: key });
            assert_eq!(callbacks.keys_for(LuaCallID::BeforeEmulation).len(), 1);
        }
    }

    #[cfg(feature = "ffi-tests")]
    #[test]
    fn test_gui_callbacks_invoked_in_frame_boundary() {
        unsafe {
            let lua = Lua::new();
            let mut engine = LuaEngine::new().unwrap();
            let gui_table = bindings::gui::register(&lua).unwrap();
            lua.globals().set("gui", gui_table).unwrap();

            let called = std::rc::Rc::new(std::cell::Cell::new(false));
            let called_clone = called.clone();
            lua.globals()
                .set(
                    "check_gui",
                    lua.create_function(move |_, ()| {
                        called_clone.set(true);
                        Ok(())
                    })
                    .unwrap(),
                )
                .unwrap();
            lua.load("gui.register(check_gui)").exec().unwrap();

            let func = lua.create_function(|_, ()| Ok(mlua::Value::Nil)).unwrap();
            let co = lua.create_thread(func).unwrap();
            engine.running = true;
            engine.frame_advance_waiting = true;
            engine.main_thread = co;

            let _ = engine.frame_boundary();

            assert!(
                called.get(),
                "GUI callback should have been called during frame_boundary"
            );
        }
    }

    #[cfg(feature = "ffi-tests")]
    #[test]
    fn test_savestate_object_userdata_has_methods() {
        unsafe {
            let lua = Lua::new();
            let _savestate_table = bindings::savestate::register(&lua).unwrap();
            lua.globals().set("savestate", _savestate_table).unwrap();

            let obj = bindings::savestate::SavestateObject(42);
            lua.globals().set("ss_obj", obj).unwrap();

            let has_save: bool = lua.load("return ss_obj.save ~= nil").eval().unwrap();
            let has_load: bool = lua.load("return ss_obj.load ~= nil").eval().unwrap();
            assert!(has_save, "SavestateObject should have :save() method");
            assert!(has_load, "SavestateObject should have :load() method");
        }
    }

    #[cfg(feature = "ffi-tests")]
    #[test]
    fn test_mem_hook_registration_triggers_recalc() {
        unsafe {
            let lua = Lua::new();
            let engine = LuaEngine::new().unwrap();
            unsafe { LUA_ENGINE_PTR.store(Box::into_raw(Box::new(engine)) as *mut c_void, Ordering::Release) };

            let memory_table = bindings::memory::register(&lua).unwrap();
            lua.globals().set("memory", memory_table).unwrap();

            let func = lua.create_function(|_, ()| Ok(())).unwrap();
            let result = {
                let eng = get_engine().unwrap();
                eng.register_mem_hook(0x1000, LuaMemHookType::Write, func)
            };
            assert!(result.is_ok(), "register_mem_hook should succeed");

            let count = unsafe { fceux11_lua_get_mem_hook_count(0) };
            assert_eq!(count, 1, "Should have 1 write hook registered");

            unsafe {
                let raw = LUA_ENGINE_PTR.load(Ordering::Acquire) as *mut LuaEngine;
                if let Some(eng) = raw.as_mut() {
                    eng.stop();
                }
                let _ = Box::from_raw(raw);
                LUA_ENGINE_PTR.store(std::ptr::null_mut(), Ordering::Release);
            }
        }
    }

    #[cfg(feature = "ffi-tests")]
    #[test]
    fn test_mem_hook_count_by_type() {
        unsafe {
            let lua = Lua::new();
            let engine = LuaEngine::new().unwrap();
            unsafe { LUA_ENGINE_PTR.store(Box::into_raw(Box::new(engine)) as *mut c_void, Ordering::Release) };

            let func1 = lua.create_function(|_, ()| Ok(())).unwrap();
            let func2 = lua.create_function(|_, ()| Ok(())).unwrap();

            {
                let eng = get_engine().unwrap();
                let _ = eng.register_mem_hook(0x100, LuaMemHookType::Write, func1);
                let _ = eng.register_mem_hook(0x200, LuaMemHookType::Read, func2);
            }

            assert_eq!(unsafe { fceux11_lua_get_mem_hook_count(0) }, 1);
            assert_eq!(unsafe { fceux11_lua_get_mem_hook_count(1) }, 1);
            assert_eq!(unsafe { fceux11_lua_get_mem_hook_count(2) }, 0);

            unsafe {
                let raw = LUA_ENGINE_PTR.load(Ordering::Acquire) as *mut LuaEngine;
                if let Some(eng) = raw.as_mut() {
                    eng.stop();
                }
                let _ = Box::from_raw(raw);
                LUA_ENGINE_PTR.store(std::ptr::null_mut(), Ordering::Release);
            }
        }
    }
}
