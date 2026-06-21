//! `movie` library binding
//!
//! Provides movie/recording state queries: `mode`, `length`, `rerecordcount`,
//! `getfilename`, `getname`, `stop`, `play`, `record`, `restart`, etc.
//!
//! FFI bridge: calls C++ `FCEUMOV_*` and `FCEUI_*` functions.

use mlua::{Lua, Result, Table};

/// Register the `movie` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let movie = lua.create_table()?;

    // movie.mode() — returns "record", "playback", "finished", or "none"
    movie.set(
        "mode",
        lua.create_function(|lua, ()| {
            let mode = unsafe { crate::fceux11_lua_movie_get_mode() };
            let s = match mode {
                0 => String::from("none"),
                1 => String::from("record"),
                2 => String::from("playback"),
                3 => String::from("finished"),
                _ => String::from("none"),
            };
            lua.create_string(&s)
        })?,
    )?;

    // movie.rerecordcount() — returns the rerecord count
    movie.set(
        "rerecordcount",
        lua.create_function(|_, ()| {
            let count = unsafe { crate::fceux11_lua_movie_get_rerecordcount() };
            Ok(count)
        })?,
    )?;

    // movie.length() — returns movie length in frames
    movie.set(
        "length",
        lua.create_function(|_, ()| {
            let len = unsafe { crate::fceux11_lua_movie_get_length() };
            Ok(len)
        })?,
    )?;

    // movie.stop() — stops movie playback/recording
    movie.set(
        "stop",
        lua.create_function(|_, ()| {
            unsafe { crate::fceux11_lua_movie_stop() };
            Ok(())
        })?,
    )?;

    // movie.getfilename() — returns movie filename (stripped of path)
    movie.set(
        "getfilename",
        lua.create_function(|lua, ()| {
            let ptr = unsafe { crate::fceux11_lua_movie_get_filename() };
            if ptr.is_null() {
                return lua.create_string("");
            }
            let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned();
            lua.create_string(&s)
        })?,
    )?;

    // movie.getname() — returns movie name (from header or filename)
    movie.set(
        "getname",
        lua.create_function(|lua, ()| {
            let ptr = unsafe { crate::fceux11_lua_movie_get_name() };
            if ptr.is_null() {
                return lua.create_string("");
            }
            let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned();
            lua.create_string(&s)
        })?,
    )?;

    // movie.getreadonly() — returns whether movie is read-only
    movie.set(
        "getreadonly",
        lua.create_function(|_, ()| {
            let ro = unsafe { crate::fceux11_lua_movie_get_readonly() };
            Ok(ro != 0)
        })?,
    )?;

    // movie.setreadonly(bool) — sets read-only mode
    movie.set(
        "setreadonly",
        lua.create_function(|_, ro: bool| {
            unsafe { crate::fceux11_lua_movie_set_readonly(if ro { 1 } else { 0 }) };
            Ok(())
        })?,
    )?;

    // movie.ispoweron() — returns whether movie starts from power-on
    movie.set(
        "ispoweron",
        lua.create_function(|_, ()| {
            let v = unsafe { crate::fceux11_lua_movie_is_poweron() };
            Ok(v != 0)
        })?,
    )?;

    // movie.isfromsavestate() — returns whether movie starts from savestate
    movie.set(
        "isfromsavestate",
        lua.create_function(|_, ()| {
            let v = unsafe { crate::fceux11_lua_movie_is_from_savestate() };
            Ok(v != 0)
        })?,
    )?;

    // movie.active() — returns whether a movie is active
    movie.set(
        "active",
        lua.create_function(|_, ()| {
            let mode = unsafe { crate::fceux11_lua_movie_get_mode() };
            Ok(mode != 0)
        })?,
    )?;

    // movie.recording() — returns whether recording
    movie.set(
        "recording",
        lua.create_function(|_, ()| {
            let mode = unsafe { crate::fceux11_lua_movie_get_mode() };
            Ok(mode == 1)
        })?,
    )?;

    // movie.playing() — returns whether playing back
    movie.set(
        "playing",
        lua.create_function(|_, ()| {
            let mode = unsafe { crate::fceux11_lua_movie_get_mode() };
            Ok(mode == 2)
        })?,
    )?;

    Ok(movie)
}
