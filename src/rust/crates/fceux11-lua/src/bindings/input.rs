//! `input` library binding
//!
//! Provides input dialog and file popup functions: `get`, `popup`, `openfilepopup`, `savefilepopup`.
//!
//! Note: Most functions are UI dialogs that require C++ integration.

use mlua::{Lua, Result, Table};

/// Register the `input` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let input = lua.create_table()?;

    // input.get() — returns current input state as a table
    input.set(
        "get",
        lua.create_function(|lua, ()| {
            // TODO (v0.2.22.3+): FFI call to get actual input state
            // For now return an empty table
            Ok(lua.create_table()?)
        })?,
    )?;

    // input.popup(string msg) — show a message popup
    input.set(
        "popup",
        lua.create_function(|_, msg: String| {
            // TODO (v0.2.22.4+): FFI call to C++ driver for popup
            println!("[input.popup] {}", msg);
            Ok(())
        })?,
    )?;

    // input.openfilepopup(table options) — show open file dialog
    input.set(
        "openfilepopup",
        lua.create_function(|_, _options: Table| {
            // TODO (v0.2.22.4+): FFI call to C++ driver for file dialog
            Ok(String::new())
        })?,
    )?;

    // input.savefilepopup(table options) — show save file dialog
    input.set(
        "savefilepopup",
        lua.create_function(|_, _options: Table| {
            // TODO (v0.2.22.4+): FFI call to C++ driver for file dialog
            Ok(String::new())
        })?,
    )?;

    Ok(input)
}
