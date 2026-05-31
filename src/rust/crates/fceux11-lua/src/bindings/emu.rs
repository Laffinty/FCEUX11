//! `emu` library binding
//!
//! Provides core emulator functions: `framecount`, `lagcount`, `paused`,
//! `message`, `print`, `frameadvance`, `speedmode`, etc.

use mlua::{Lua, Table, Result, Function};

/// Register the `emu` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let emu = lua.create_table()?;

    // --- FFI-free functions (v0.2.22.1) ---

    emu.set("framecount", lua.create_function(|_, ()| Ok(0i64))?)?; // TODO: FFI read
    emu.set("lagcount", lua.create_function(|_, ()| Ok(0i64))?)?; // TODO: FFI read
    emu.set("paused", lua.create_function(|_, ()| Ok(false))?)?; // TODO: FFI read

    emu.set(
        "message",
        lua.create_function(|_, msg: String| {
            println!("[emu.message] {}", msg);
            Ok(())
        })?,
    )?;

    emu.set(
        "print",
        lua.create_function(|_, msg: String| {
            println!("[emu.print] {}", msg);
            Ok(())
        })?,
    )?;

    // --- FFI-dependent functions (v0.2.22.2+) ---

    emu.set("poweron", lua.create_function(|_, ()| Ok(()))?)?; // TODO: FFI
    emu.set("softreset", lua.create_function(|_, ()| Ok(()))?)?; // TODO: FFI

    // frameadvance: in mlua without async, we signal yield via CoroutineUnresumable
    // The frame_boundary() caller detects this and handles frame advancement externally.
    emu.set("frameadvance", lua.create_function(|_, ()| {
        Err::<(), _>(mlua::Error::CoroutineUnresumable)
    })?)?;

    emu.set("speedmode", lua.create_function(|_, mode: String| {
        println!("[emu.speedmode] {}", mode);
        Ok(())
    })?)?;

    emu.set("registerbefore", lua.create_function(|_, _cb: Function| Ok(()))?)?;
    emu.set("registerafter", lua.create_function(|_, _cb: Function| Ok(()))?)?;
    emu.set("registerexit", lua.create_function(|_, _cb: Function| Ok(()))?)?;

    Ok(emu)
}