//! `savestate` library binding
//!
//! Provides save state operations: `save`, `load`, `create`, `object`, `persist`.
//!
//! Note: `LuaSaveData` serialization (LuaStackToBinary/BinaryToLuaStack) stays in C++
//! as it depends on Lua 5.1 stack internal type tags.

use mlua::{Lua, Table, Result, Function, String};

/// Register the `savestate` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let savestate = lua.create_table()?;

    // savestate.save(slot) — save state to slot (1-10)
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

    // savestate.load(slot) — load state from slot (1-10)
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

    // savestate.create(which, anonymous) — create savestate object
    // which: slot number (1-10) or nil; anonymous: bool (default false)
    savestate.set(
        "create",
        lua.create_function(|_lua, (which, anonymous): (Option<i32>, Option<bool>)| {
            let slot = which.unwrap_or(-1);
            let anon = anonymous.unwrap_or(false) as i32;
            let id = unsafe {
                crate::fceux11_lua_savestate_create_object(std::ptr::null(), slot, anon)
            };
            if id < 0 {
                return Err(mlua::Error::RuntimeError(
                    "savestate.create failed".to_string(),
                ));
            }
            Ok(id)
        })?,
    )?;

    // savestate.object(path) — open existing savestate file by path
    savestate.set(
        "object",
        lua.create_function(|_lua, path: String| {
            let path_bytes = path.as_bytes_with_nul();
            let id = unsafe {
                crate::fceux11_lua_savestate_create_object(
                    path_bytes.as_ptr() as *const i8,
                    -1,
                    0,
                )
            };
            if id < 0 {
                return Err(mlua::Error::RuntimeError(
                    "savestate.object failed".to_string(),
                ));
            }
            Ok(id)
        })?,
    )?;

    // savestate.persist(obj_id) — persist object to its file
    savestate.set(
        "persist",
        lua.create_function(|_, _obj_id: i32| {
            // TODO (v0.2.22.5+): call C++ LuaSaveData serialization + file write
            // For now, note that this is deferred per plan
            Ok(())
        })?,
    )?;

    // savestate.registersave(func) — register automatic save callback
    savestate.set(
        "registersave",
        lua.create_function(|lua, cb: Function| {
            let _key = lua.create_registry_value(cb)?;
            Ok(())
        })?,
    )?;

    // savestate.registerload(func) — register automatic load callback
    savestate.set(
        "registerload",
        lua.create_function(|lua, cb: Function| {
            let _key = lua.create_registry_value(cb)?;
            Ok(())
        })?,
    )?;

    Ok(savestate)
}