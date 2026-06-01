//! `emu` library binding
//!
//! Provides core emulator functions: `framecount`, `lagcount`, `paused`,
//! `message`, `print`, `frameadvance`, `speedmode`, etc.

use mlua::{Lua, Table, Result, Function, String};

/// Speed mode constants (matching C++ speedmode enum)
const SPEED_NORMAL: i32 = 0;
const SPEED_NOTHROTTLE: i32 = 1;
const SPEED_TURBO: i32 = 2;
const SPEED_MAXIMUM: i32 = 3;

/// Register the `emu` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let emu = lua.create_table()?;

    // --- State accessors (FFI complete) ---

    emu.set(
        "framecount",
        lua.create_function(|_, ()| {
            let count = unsafe { crate::fceux11_lua_emu_get_framecount() };
            Ok(count)
        })?,
    )?;

    emu.set(
        "lagcount",
        lua.create_function(|_, ()| {
            let count = unsafe { crate::fceux11_lua_emu_get_lagcount() };
            Ok(count)
        })?,
    )?;

    emu.set(
        "paused",
        lua.create_function(|_, ()| {
            let is_paused = unsafe { crate::fceux11_lua_emu_is_paused() };
            Ok(is_paused != 0)
        })?,
    )?;

    // --- Display functions (FFI complete) ---

    emu.set(
        "message",
        lua.create_function(|_, msg: String| {
            let c_str = msg.as_bytes_with_nul();
            unsafe { crate::fceux11_lua_emu_message(c_str.as_ptr() as *const i8) };
            Ok(())
        })?,
    )?;

    emu.set(
        "print",
        lua.create_function(|_, msg: String| {
            // Match C++ behavior: print goes to console
            let s = msg.display();
            println!("{}", s);
            Ok(())
        })?,
    )?;

    // --- Reset functions (FFI complete) ---

    emu.set(
        "poweron",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_emu_poweron() };
            Ok(())
        })?,
    )?;

    emu.set(
        "softreset",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_emu_softreset() };
            Ok(())
        })?,
    )?;

    // --- Frame advance (coroutine yield) ---
    // Signal CoroutineUnresumable to indicate frame boundary.
    // The C++ caller detects this via lua_resume return and handles frame advancement.
    emu.set(
        "frameadvance",
        lua.create_function(|_, ()| {
            Err::<(), _>(mlua::Error::CoroutineUnresumable)
        })?,
    )?;

    // --- Speed mode (FFI complete) ---

    emu.set(
        "speedmode",
        lua.create_function(|_, mode: String| {
            let m = match mode.to_str() {
                Ok(s) => match s.to_lowercase().as_str() {
                    "normal" => SPEED_NORMAL,
                    "nothrottle" => SPEED_NOTHROTTLE,
                    "turbo" => SPEED_TURBO,
                    "maximum" => SPEED_MAXIMUM,
                    v => {
                        return Err(mlua::Error::RuntimeError(format!(
                            "invalid mode '{}' (valid: normal, nothrottle, turbo, maximum)",
                            v
                        )));
                    }
                },
                Err(_) => {
                    return Err(mlua::Error::RuntimeError(
                        "invalid string encoding".to_string(),
                    ));
                }
            };
            unsafe { crate::fceux11_lua_emu_set_speedmode(m) };
            Ok(())
        })?,
    )?;

    // --- Pause/Unpause (FFI complete) ---

    emu.set(
        "pause",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_emu_pause() };
            Ok(())
        })?,
    )?;

    emu.set(
        "unpause",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_emu_unpause() };
            Ok(())
        })?,
    )?;

    // --- Callback registration ---
    // Callbacks are stored in Lua's registry using create_registry_value.
    // C++ side invokes them via fceux11_lua_call_registered(BEFORE/AFTER/EXIT)
    // which retrieves from the registry in LuaEngine::call_registered.

    emu.set(
        "registerbefore",
        lua.create_function(|lua, cb: Function| {
            // Store in Lua registry, discarding the RegistryKey
            // (value lives until Lua engine is destroyed)
            let _key = lua.create_registry_value(cb)?;
            Ok(())
        })?,
    )?;

    emu.set(
        "registerafter",
        lua.create_function(|lua, cb: Function| {
            let _key = lua.create_registry_value(cb)?;
            Ok(())
        })?,
    )?;

    emu.set(
        "registerexit",
        lua.create_function(|lua, cb: Function| {
            let _key = lua.create_registry_value(cb)?;
            Ok(())
        })?,
    )?;

    Ok(emu)
}