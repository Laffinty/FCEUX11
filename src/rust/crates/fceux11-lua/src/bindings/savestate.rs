//! `savestate` library binding
//!
//! `save`/`load` delegate to C++ FFI. `registersave`/`registerload` store
//! callbacks persistently in the engine's `RegisteredCallbacks` via `RegistryKey`.
//! `LuaSaveData` serialization stays in C++ (depends on Lua stack internals).

use mlua::{Function, Lua, Result, String, Table};

pub fn register(lua: &Lua) -> Result<Table> {
    let savestate = lua.create_table()?;

    savestate.set(
        "save",
        lua.create_function(|_, slot: i32| {
            if slot < 1 || slot > 10 {
                return Err(mlua::Error::RuntimeError(format!(
                    "invalid savestate slot {} (valid range 1-10)",
                    slot
                )));
            }
            let result = unsafe { crate::fceux11_lua_savestate_save_slot(slot - 1) };
            if result != 0 {
                return Err(mlua::Error::RuntimeError(format!(
                    "savestate.save failed with code {}",
                    result
                )));
            }
            Ok(())
        })?,
    )?;

    savestate.set(
        "load",
        lua.create_function(|_, slot: i32| {
            if slot < 1 || slot > 10 {
                return Err(mlua::Error::RuntimeError(format!(
                    "invalid savestate slot {} (valid range 1-10)",
                    slot
                )));
            }
            let result = unsafe { crate::fceux11_lua_savestate_load_slot(slot - 1) };
            if result != 0 {
                return Err(mlua::Error::RuntimeError(format!(
                    "savestate.load failed with code {}",
                    result
                )));
            }
            Ok(())
        })?,
    )?;

    savestate.set(
        "create",
        lua.create_function(|_, (which, anonymous): (Option<i32>, Option<bool>)| {
            let slot = which.unwrap_or(-1);
            let anon = anonymous.unwrap_or(false) as i32;
            let id =
                unsafe { crate::fceux11_lua_savestate_create_object(std::ptr::null(), slot, anon) };
            if id < 0 {
                return Err(mlua::Error::RuntimeError("savestate.create failed".into()));
            }
            Ok(id)
        })?,
    )?;

    savestate.set(
        "object",
        lua.create_function(|_, path: String| {
            let path_bytes = path.as_bytes_with_nul();
            let id = unsafe {
                crate::fceux11_lua_savestate_create_object(path_bytes.as_ptr() as *const i8, -1, 0)
            };
            if id < 0 {
                return Err(mlua::Error::RuntimeError("savestate.object failed".into()));
            }
            Ok(id)
        })?,
    )?;

    // persist: tell C++ to persist the savestate object to disk
    savestate.set(
        "persist",
        lua.create_function(|_, _obj_id: i32| {
            // TODO: FFI call to C++ savestate persist
            Ok(())
        })?,
    )?;

    // registersave: store callback persistently (not dropped after function returns)
    savestate.set(
        "registersave",
        lua.create_function(|lua, cb: Function| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                let key = lua.create_registry_value(cb.clone())?;
                eng.callbacks().savestate_save.push(crate::CallbackEntry {
                    func: cb,
                    _key: key,
                });
            }
            Ok(())
        })?,
    )?;

    // registerload: store callback persistently
    savestate.set(
        "registerload",
        lua.create_function(|lua, cb: Function| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                let key = lua.create_registry_value(cb.clone())?;
                eng.callbacks().savestate_load.push(crate::CallbackEntry {
                    func: cb,
                    _key: key,
                });
            }
            Ok(())
        })?,
    )?;

    Ok(savestate)
}
