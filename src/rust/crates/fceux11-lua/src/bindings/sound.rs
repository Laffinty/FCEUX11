//! `sound` library binding
//!
//! Provides sound/APU state queries: `get`.
//!
//! FFI bridge: reads sound state from C++ `sound.cpp` globals.

use mlua::{Lua, Table, Result};

/// Register the `sound` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let sound = lua.create_table()?;

    sound.set(
        "get",
        lua.create_function(|lua, ()| {
            // TODO (v0.2.22.4+): FFI call to get actual sound state
            // Return a table with basic info for now
            let state = lua.create_table()?;
            state.set("volume", 100)?;
            Ok(state)
        })?,
    )?;

    Ok(sound)
}