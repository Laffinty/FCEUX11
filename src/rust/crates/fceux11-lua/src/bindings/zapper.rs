//! `zapper` library binding
//!
//! Provides zapper (light gun) functions: `read`, `set`.
//!
//! FFI bridge: reads zapper state from C++ via FCEU_LuaReadZapper / movie data.

use mlua::{Lua, Result, Table};

/// Register the `zapper` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let zapper = lua.create_table()?;

    // zapper.read() — returns {x, y, click}
    zapper.set(
        "read",
        lua.create_function(|_lua: &Lua, ()| {
            let x = unsafe { crate::fceux11_lua_zapper_get_x() };
            let y = unsafe { crate::fceux11_lua_zapper_get_y() };
            let click = unsafe { crate::fceux11_lua_zapper_get_click() };
            let table = _lua.create_table()?;
            table.set("x", x)?;
            table.set("y", y)?;
            table.set("click", click)?;
            Ok(table)
        })?,
    )?;

    // zapper.set({x, y, fire}) — sets zapper override for next frame
    zapper.set(
        "set",
        lua.create_function(|_lua: &Lua, state: Table| {
            let x: Option<i32> = state.get("x").ok();
            let y: Option<i32> = state.get("y").ok();
            let fire: Option<bool> = state.get("fire").ok();
            unsafe {
                crate::fceux11_lua_zapper_set(
                    x.unwrap_or(-1),
                    y.unwrap_or(-1),
                    if fire.unwrap_or(false) { 1 } else { 0 },
                );
            }
            Ok(())
        })?,
    )?;

    Ok(zapper)
}
