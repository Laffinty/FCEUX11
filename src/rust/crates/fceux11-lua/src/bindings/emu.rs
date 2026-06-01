//! `emu` library binding — core emulator functions
//!
//! Key design: `emu.frameadvance()` yields the main coroutine via
//! `mlua::Thread::yield`. The C++ emulator resumes it each frame via
//! `fceux11_lua_frame_boundary()` → `LuaEngine::frame_boundary()`.

use mlua::{Function, Lua, Result, String, Table};

const SPEED_NORMAL: i32 = 0;
const SPEED_NOTHROTTLE: i32 = 1;
const SPEED_TURBO: i32 = 2;
const SPEED_MAXIMUM: i32 = 3;

pub fn register(lua: &Lua) -> Result<Table> {
    let emu = lua.create_table()?;

    emu.set(
        "framecount",
        lua.create_function(|_, ()| Ok(unsafe { crate::fceux11_lua_emu_get_framecount() }))?,
    )?;

    emu.set(
        "lagcount",
        lua.create_function(|_, ()| Ok(unsafe { crate::fceux11_lua_emu_get_lagcount() }))?,
    )?;

    emu.set(
        "lagged",
        lua.create_function(|_, ()| Ok(unsafe { crate::fceux11_lua_emu_get_lagcount() } > 0))?,
    )?;

    emu.set("emulating", lua.create_function(|_, ()| Ok(true))?)?;

    emu.set(
        "paused",
        lua.create_function(|_, ()| Ok(unsafe { crate::fceux11_lua_emu_is_paused() } != 0))?,
    )?;

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
        lua.create_function(|_, args: mlua::Variadic<mlua::Value>| {
            let mut parts = Vec::new();
            for v in args {
                let s = match v {
                    mlua::Value::String(s) => s.to_str()?.to_owned(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Integer(n) => n.to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    mlua::Value::Nil => "nil".to_string(),
                    _ => format!("{:?}", v),
                };
                parts.push(s);
            }
            println!("{}", parts.join("\t"));
            Ok(())
        })?,
    )?;

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

    emu.set(
        "frameadvance",
        lua.create_function(|lua, ()| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.set_frame_advance_waiting(true);
            }
            let co: mlua::Table = lua.globals().get("coroutine")?;
            let yield_fn: mlua::Function = co.get("yield")?;
            yield_fn.call::<()>(())
        })?,
    )?;

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
                        )))
                    }
                },
                Err(_) => return Err(mlua::Error::RuntimeError("invalid string encoding".into())),
            };
            unsafe { crate::fceux11_lua_emu_set_speedmode(m) };
            Ok(())
        })?,
    )?;

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

    emu.set(
        "registerbefore",
        lua.create_function(|_lua, cb: Function| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.register_callback(crate::LuaCallID::BeforeEmulation, cb)?;
            }
            Ok(())
        })?,
    )?;

    emu.set(
        "registerafter",
        lua.create_function(|_lua, cb: Function| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.register_callback(crate::LuaCallID::AfterEmulation, cb)?;
            }
            Ok(())
        })?,
    )?;

    emu.set(
        "registerexit",
        lua.create_function(|_lua, cb: Function| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.register_callback(crate::LuaCallID::BeforeExit, cb)?;
            }
            Ok(())
        })?,
    )?;

    Ok(emu)
}
