//! Test-only FFI stubs for fceux11-lua.
//!
//! These satisfy the linker when running `cargo test` without the full C++
//! executable. They are gated by `#[cfg(any(test, feature = "ffi-stubs"))]`
//! and never compiled into the production rlib/staticlib.

use std::ffi::{c_char, c_int};

#[allow(unused_variables)]
#[allow(dead_code)]
#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_GetMem(_addr: u32) -> u8 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_BWrite(_addr: u32, _val: u8) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_GetRegister(_reg_id: i32) -> u16 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_GetJoypadState(_port: i32) -> u32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_SetJoypadOverride(_port: i32, _mask1: u32, _mask2: u32) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_GetRomMD5(_buf: *mut u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_ReadRomByte(_addr: u32) -> u8 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_WriteRomByte(_addr: u32, _val: u8) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_PPURead(_addr: u32) -> u8 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_get_mode() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_get_rerecordcount() -> i64 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_get_length() -> i64 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_stop() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_get_readonly() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_set_readonly(_val: i32) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_is_poweron() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_is_from_savestate() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_get_name() -> *const c_char {
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_movie_get_filename() -> *const c_char {
    std::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_savestate_save_slot(_slot: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_savestate_load_slot(_slot: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_get_framecount() -> i64 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_get_lagcount() -> i64 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_is_paused() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_set_speedmode(_mode: i32) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_poweron() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_softreset() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_message(_msg: *const c_char) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_pause() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_emu_unpause() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_savestate_create_object(
    _path: *const c_char,
    _which: i32,
    _anonymous: i32,
) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_savestate_delete_object(_obj_id: i32) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_savestate_object_save(_obj_id: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_savestate_object_load(_obj_id: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_savestate_object_persist(_obj_id: i32) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_gui_popup(_msg: *const c_char) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_gui_savescreenshot(_filename: *const c_char) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square1_volume() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square1_frequency() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square1_midikey() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square1_duty() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square1_regs() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square2_volume() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square2_frequency() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square2_midikey() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square2_duty() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_square2_regs() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_triangle_volume() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_triangle_linear() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_triangle_frequency() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_triangle_midikey() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_noise_volume() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_noise_mode() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_noise_regs() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_noise_frequency() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_noise_midikey() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_volume() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_rate() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_regs() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_frequency() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_midikey() -> f64 {
    0.0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_address() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_size() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_loop() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_dmc_seed() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_frame_sequencer() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_sample_rate() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_sound_get_length_count() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_zapper_get_x() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_zapper_get_y() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_zapper_get_click() -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_zapper_set(_x: i32, _y: i32, _fire: i32) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_debugger_hitbreakpoint() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_debugger_get_cycles_count() -> u64 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_debugger_get_instructions_count() -> u64 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_debugger_reset_cycles_count() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_debugger_reset_instructions_count() {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_debugger_get_symbol_offset(_name: *const c_char) -> i64 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_GetKeyboardState(_keys: *mut u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_GetMouseState(_x: *mut i32, _y: *mut i32, _click: *mut i32) {}

#[unsafe(no_mangle)]
pub(crate) extern "C" fn fceux11_lua_recalculate_mem_hook_regions(_hook_type: c_int) {}
