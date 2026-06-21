//! `memory` library binding — NES memory access + hook registration
//!
//! `readbyte`/`writebyte`/`getregister` call C++ FFI directly.
//! `registerwrite`/`registerread`/`registerexec` store Lua callbacks
//! in the engine's persistent `RegisteredCallbacks` map via `RegistryKey`.

use mlua::{Function, Lua, Result, Table};

const REG_PC: i32 = 0;
const REG_A: i32 = 1;
const REG_X: i32 = 2;
const REG_Y: i32 = 3;
const REG_S: i32 = 4;
const REG_P: i32 = 5;

pub fn register(lua: &Lua) -> Result<Table> {
    let memory = lua.create_table()?;

    memory.set(
        "readbyte",
        lua.create_function(|_, addr: u32| Ok(unsafe { crate::fceux11_lua_GetMem(addr) } as i32))?,
    )?;

    memory.set(
        "readbytesigned",
        lua.create_function(|_, addr: u32| {
            let v = unsafe { crate::fceux11_lua_GetMem(addr) } as i8;
            Ok(v as i32)
        })?,
    )?;

    memory.set(
        "readword",
        lua.create_function(|_, addr: u32| {
            let lo = unsafe { crate::fceux11_lua_GetMem(addr) } as u16;
            let hi = unsafe { crate::fceux11_lua_GetMem(addr + 1) } as u16;
            Ok(((hi << 8) | lo) as i32)
        })?,
    )?;

    memory.set(
        "readwordsigned",
        lua.create_function(|_, addr: u32| {
            let lo = unsafe { crate::fceux11_lua_GetMem(addr) } as u16;
            let hi = unsafe { crate::fceux11_lua_GetMem(addr + 1) } as u16;
            Ok(((hi << 8) | lo) as i16 as i32)
        })?,
    )?;

    memory.set(
        "writebyte",
        lua.create_function(|_, (addr, val): (u32, u32)| {
            unsafe { crate::fceux11_lua_BWrite(addr, val as u8) };
            Ok(())
        })?,
    )?;

    memory.set(
        "getregister",
        lua.create_function(|_, name: mlua::String| {
            let s = name.to_str()?;
            let val = match s.as_ref() {
                "pc" | "PC" => unsafe { crate::fceux11_lua_GetRegister(REG_PC) },
                "a" | "A" => unsafe { crate::fceux11_lua_GetRegister(REG_A) },
                "x" | "X" => unsafe { crate::fceux11_lua_GetRegister(REG_X) },
                "y" | "Y" => unsafe { crate::fceux11_lua_GetRegister(REG_Y) },
                "s" | "S" => unsafe { crate::fceux11_lua_GetRegister(REG_S) },
                "p" | "P" => unsafe { crate::fceux11_lua_GetRegister(REG_P) },
                _ => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "unknown register: {}",
                        s
                    )));
                }
            };
            Ok(val as i32)
        })?,
    )?;

    // registerwrite: register a Lua callback for memory writes at address
    memory.set(
        "registerwrite",
        lua.create_function(|_lua, (addr, func): (u32, Function)| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.register_mem_hook(addr, crate::LuaMemHookType::Write, func)?;
            }
            Ok(())
        })?,
    )?;

    memory.set(
        "registerread",
        lua.create_function(|_lua, (addr, func): (u32, Function)| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.register_mem_hook(addr, crate::LuaMemHookType::Read, func)?;
            }
            Ok(())
        })?,
    )?;

    memory.set(
        "registerexec",
        lua.create_function(|_lua, (addr, func): (u32, Function)| {
            let engine = crate::get_engine_mut();
            if let Some(eng) = engine {
                eng.register_mem_hook(addr, crate::LuaMemHookType::Exec, func)?;
            }
            Ok(())
        })?,
    )?;

    Ok(memory)
}
