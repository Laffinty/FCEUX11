//! `savestate` library binding
//!
//! `save`/`load` delegate to C++ FFI. `registersave`/`registerload` store
//! callbacks persistently in the engine's `RegisteredCallbacks` via `RegistryKey`.

use mlua::{Function, Lua, Result, String, Table, UserData, UserDataMethods};

pub struct SavestateObject(pub i32);

impl UserData for SavestateObject {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("save", |_, obj, ()| {
            let r = unsafe { crate::fceux11_lua_savestate_object_save(obj.0) };
            if r != 0 {
                return Err(mlua::Error::RuntimeError("savestate:save failed".into()));
            }
            Ok(())
        });
        methods.add_method("load", |_, obj, ()| {
            let r = unsafe { crate::fceux11_lua_savestate_object_load(obj.0) };
            if r != 0 {
                return Err(mlua::Error::RuntimeError("savestate:load failed".into()));
            }
            Ok(())
        });
    }
}

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
        lua.create_function(|_lua, (which, anonymous): (Option<i32>, Option<bool>)| {
            let slot = which.unwrap_or(-1);
            let anon = anonymous.unwrap_or(false) as i32;
            let id =
                unsafe { crate::fceux11_lua_savestate_create_object(std::ptr::null(), slot, anon) };
            if id < 0 {
                return Err(mlua::Error::RuntimeError("savestate.create failed".into()));
            }
            Ok(SavestateObject(id))
        })?,
    )?;

    savestate.set(
        "object",
        lua.create_function(|_lua, path: String| {
            let path_bytes = path.as_bytes_with_nul();
            let id = unsafe {
                crate::fceux11_lua_savestate_create_object(path_bytes.as_ptr() as *const i8, -1, 0)
            };
            if id < 0 {
                return Err(mlua::Error::RuntimeError("savestate.object failed".into()));
            }
            Ok(SavestateObject(id))
        })?,
    )?;

    savestate.set(
        "persist",
        lua.create_function(|_, ud: mlua::AnyUserData| {
            let obj = ud.borrow::<SavestateObject>()?;
            let r = unsafe { crate::fceux11_lua_savestate_object_persist(obj.0) };
            if r != 0 {
                return Err(mlua::Error::RuntimeError("savestate.persist failed".into()));
            }
            Ok(())
        })?,
    )?;

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
