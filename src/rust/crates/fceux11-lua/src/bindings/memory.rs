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

    // v2.0.2 B3: readbyterange — length bytes starting at addr as a Lua
    // string (matches upstream FCEUX semantics: negative length returns
    // nothing, zero length returns the empty string; no clamping here —
    // address handling follows fceux11_lua_GetMem, which masks to 16 bit).
    memory.set(
        "readbyterange",
        lua.create_function(|lua, (addr, len): (u32, i64)| {
            if len < 0 {
                return Ok(mlua::Value::Nil);
            }
            let mut buf = Vec::with_capacity(len as usize);
            for i in 0..len {
                let a = addr.wrapping_add(i as u32);
                buf.push(unsafe { crate::fceux11_lua_GetMem(a) });
            }
            Ok(mlua::Value::String(lua.create_string(&buf)?))
        })?,
    )?;

    // v2.0.2 B3: writebyterange — write each byte of the string in order,
    // byte by byte, through the same BWrite path as writebyte (so the
    // out-of-range semantics are identical: ROM/upper addresses are
    // silently ignored, no crash).
    memory.set(
        "writebyterange",
        lua.create_function(|_, (addr, data): (u32, mlua::String)| {
            for (i, &b) in data.as_bytes().iter().enumerate() {
                let a = addr.wrapping_add(i as u32);
                unsafe { crate::fceux11_lua_BWrite(a, b) };
            }
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

// ---------------------------------------------------------------------------
// v2.0.2 B3 — readbyterange / writebyterange unit tests
//
// These run under `cargo test -p fceux11-lua` where the FFI layer is the
// test-only stub (`ffi_stubs.rs`: GetMem -> 0, BWrite -> no-op). They pin
// the shape/edge semantics of the range exports (string result, negative
// length -> nil, zero length -> empty string, write path does not panic);
// value-level round-trip is covered by tests/lua_scripts/test_memory.lua
// against the real bus (kgmqa-046).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_table(lua: &Lua) -> Table {
        register(lua).unwrap()
    }

    #[test]
    fn readbyterange_returns_string_of_requested_length() {
        let lua = Lua::new();
        let memory = memory_table(&lua);
        let f: Function = memory.get("readbyterange").unwrap();
        let s: mlua::String = f.call((0u32, 8i64)).unwrap();
        assert_eq!(s.as_bytes().len(), 8);
    }

    #[test]
    fn readbyterange_zero_length_is_empty_string() {
        let lua = Lua::new();
        let memory = memory_table(&lua);
        let f: Function = memory.get("readbyterange").unwrap();
        let s: mlua::String = f.call((0u32, 0i64)).unwrap();
        assert_eq!(s.as_bytes().len(), 0);
    }

    #[test]
    fn readbyterange_negative_length_returns_nil() {
        let lua = Lua::new();
        let memory = memory_table(&lua);
        let f: Function = memory.get("readbyterange").unwrap();
        let v: mlua::Value = f.call((0u32, -1i64)).unwrap();
        assert!(matches!(v, mlua::Value::Nil));
    }

    #[test]
    fn writebyterange_accepts_string_and_does_not_panic() {
        let lua = Lua::new();
        let memory = memory_table(&lua);
        let w: Function = memory.get("writebyterange").unwrap();
        let r: Function = memory.get("readbyterange").unwrap();
        w.call::<()>((0x100u32, "abc")).unwrap();
        // Stub BWrite is a no-op and stub GetMem returns 0, so the write
        // must not have panicked and the read must still be a string.
        let s: mlua::String = r.call((0x100u32, 3i64)).unwrap();
        assert_eq!(s.as_bytes().len(), 3);
    }

    #[test]
    fn writebyterange_empty_string_is_accepted() {
        let lua = Lua::new();
        let memory = memory_table(&lua);
        let w: Function = memory.get("writebyterange").unwrap();
        w.call::<()>((0u32, "")).unwrap();
    }
}
