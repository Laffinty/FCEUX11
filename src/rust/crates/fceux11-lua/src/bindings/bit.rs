//! `bit` library binding
//!
//! Provides Lua 5.1 compatible bitwise operations: `tobit`, `bnot`, `band`,
//! `bor`, `bxor`, `lshift`, `rshift`, `arshift`, `rol`, `ror`, `bswap`, `tohex`.

use mlua::{Lua, Table, Result};

/// Register the `bit` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let bit = lua.create_table()?;

    bit.set("tobit", lua.create_function(|_, x: i64| Ok(x as i32))?)?;
    bit.set("bnot", lua.create_function(|_, x: i32| Ok(!x))?)?;
    bit.set("band", lua.create_function(|_, (a, b): (i32, i32)| Ok(a & b))?)?;
    bit.set("bor",  lua.create_function(|_, (a, b): (i32, i32)| Ok(a | b))?)?;
    bit.set("bxor", lua.create_function(|_, (a, b): (i32, i32)| Ok(a ^ b))?)?;
    bit.set(
        "lshift",
        lua.create_function(|_, (x, n): (i32, i32)| Ok(x.wrapping_shl(n as u32)))?,
    )?;
    bit.set(
        "rshift",
        lua.create_function(|_, (x, n): (i32, i32)| Ok(x.wrapping_shr(n as u32)))?,
    )?;
    bit.set(
        "arshift",
        lua.create_function(|_, (x, n): (i32, i32)| {
            Ok((x as i64 >> n) as i32)
        })?,
    )?;
    bit.set(
        "rol",
        lua.create_function(|_, (x, n): (i32, i32)| Ok(x.rotate_left(n as u32)))?,
    )?;
    bit.set(
        "ror",
        lua.create_function(|_, (x, n): (i32, i32)| Ok(x.rotate_right(n as u32)))?,
    )?;
    bit.set("bswap", lua.create_function(|_, x: i32| Ok(x.swap_bytes()))?)?;
    bit.set(
        "tohex",
        lua.create_function(|_, (x, n): (i32, Option<i32>)| {
            let digits = n.unwrap_or(8);
            Ok(format!("{:0>width$X}", x as u32, width = digits as usize))
        })?,
    )?;

    Ok(bit)
}