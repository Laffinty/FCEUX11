//! `bit` library binding
//!
//! Provides Lua 5.1 compatible bitwise operations: `tobit`, `bnot`, `band`,
//! `bor`, `bxor`, `lshift`, `rshift`, `arshift`, `rol`, `ror`, `bswap`, `tohex`.

use mlua::{Lua, Result, Table};

/// Register the `bit` table into the Lua global namespace
pub fn register(lua: &Lua) -> Result<Table> {
    let bit = lua.create_table()?;

    bit.set("tobit", lua.create_function(|_, x: i64| Ok(x as i32))?)?;
    bit.set("bnot", lua.create_function(|_, x: i32| Ok(!x))?)?;
    bit.set(
        "band",
        lua.create_function(|_, (a, b): (i32, i32)| Ok(a & b))?,
    )?;
    bit.set(
        "bor",
        lua.create_function(|_, (a, b): (i32, i32)| Ok(a | b))?,
    )?;
    bit.set(
        "bxor",
        lua.create_function(|_, (a, b): (i32, i32)| Ok(a ^ b))?,
    )?;
    bit.set(
        "lshift",
        lua.create_function(|_, (x, n): (i32, i32)| {
            Ok(if n < 0 || n >= 32 { 0 } else { x.wrapping_shl(n as u32) })
        })?,
    )?;
    bit.set(
        "rshift",
        lua.create_function(|_, (x, n): (i32, i32)| Ok(x.wrapping_shr(n as u32)))?,
    )?;
    bit.set(
        "arshift",
        lua.create_function(|_, (x, n): (i32, i32)| Ok((x as i64 >> n) as i32))?,
    )?;
    bit.set(
        "rol",
        lua.create_function(|_, (x, n): (i32, i32)| Ok(x.rotate_left(n as u32)))?,
    )?;
    bit.set(
        "ror",
        lua.create_function(|_, (x, n): (i32, i32)| Ok(x.rotate_right(n as u32)))?,
    )?;
    bit.set(
        "bswap",
        lua.create_function(|_, x: i32| Ok(x.swap_bytes()))?,
    )?;
    bit.set(
        "tohex",
        lua.create_function(|_, (x, n): (i32, Option<i32>)| {
            let digits = n.unwrap_or(8);
            let width = digits as usize;
            let mask = if width >= 8 {
                u32::MAX
            } else {
                (1u32 << (width * 4)).wrapping_sub(1)
            };
            Ok(format!("{:0>width$x}", (x as u32) & mask, width = width))
        })?,
    )?;

    Ok(bit)
}

// ---------------------------------------------------------------------------
// Tests (pure Lua, no FFI needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn setup_lua() -> Lua {
        let lua = Lua::new();
        let bit = register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        lua
    }

    #[test]
    fn test_bit_bor() {
        let lua = setup_lua();
        let result: i32 = lua.load("bit.bor(0xFF, 0x00)").eval().unwrap();
        assert_eq!(result, 0xFF);
    }

    #[test]
    fn test_bit_band() {
        let lua = setup_lua();
        let result: i32 = lua.load("bit.band(0xFF, 0x0F)").eval().unwrap();
        assert_eq!(result, 0x0F);
    }

    #[test]
    fn test_bit_bxor() {
        let lua = setup_lua();
        let result: i32 = lua.load("bit.bxor(0xFF, 0x0F)").eval().unwrap();
        assert_eq!(result, 0xF0);
    }

    #[test]
    fn test_bit_bnot() {
        let lua = setup_lua();
        let result: i32 = lua.load("bit.bnot(0)").eval().unwrap();
        assert_eq!(result, -1);
    }

    #[test]
    fn test_bit_lshift() {
        let lua = setup_lua();
        assert_eq!(1_i32, lua.load("bit.lshift(1, 0)").eval().unwrap());
        assert_eq!(256, lua.load("bit.lshift(1, 8)").eval::<i32>().unwrap());
        assert_eq!(0, lua.load("bit.lshift(1, 32)").eval::<i32>().unwrap()); // wrap
    }

    #[test]
    fn test_bit_rshift() {
        let lua = setup_lua();
        assert_eq!(1, lua.load("bit.rshift(256, 8)").eval::<i32>().unwrap());
        assert_eq!(0, lua.load("bit.rshift(1, 1)").eval::<i32>().unwrap());
    }

    #[test]
    fn test_bit_arshift() {
        let lua = setup_lua();
        assert_eq!(-1, lua.load("bit.arshift(-1, 0)").eval::<i32>().unwrap());
        assert_eq!(-4, lua.load("bit.arshift(-16, 2)").eval::<i32>().unwrap());
    }

    #[test]
    fn test_bit_rol() {
        let lua = setup_lua();
        assert_eq!(0x80, lua.load("bit.rol(0x01, 7)").eval::<i32>().unwrap());
    }

    #[test]
    fn test_bit_ror() {
        let lua = setup_lua();
        assert_eq!(0x02, lua.load("bit.ror(0x10, 3)").eval::<i32>().unwrap());
    }

    #[test]
    fn test_bit_bswap() {
        let lua = setup_lua();
        let result: i32 = lua.load("bit.bswap(0x12345678)").eval().unwrap();
        assert_eq!(result, 0x78563412);
    }

    #[test]
    fn test_bit_tohex() {
        let lua = setup_lua();
        assert_eq!(
            "000000ff",
            lua.load(r#"bit.tohex(255)"#).eval::<String>().unwrap()
        );
        assert_eq!(
            "ff",
            lua.load(r#"bit.tohex(255, 2)"#).eval::<String>().unwrap()
        );
        assert_eq!(
            "ffff",
            lua.load(r#"bit.tohex(-1, 4)"#).eval::<String>().unwrap()
        );
    }

    #[test]
    fn test_bit_tobit() {
        let lua = setup_lua();
        assert_eq!(1, lua.load("bit.tobit(1.0)").eval::<i32>().unwrap());
        assert_eq!(-1, lua.load("bit.tobit(-1.0)").eval::<i32>().unwrap());
    }
}
