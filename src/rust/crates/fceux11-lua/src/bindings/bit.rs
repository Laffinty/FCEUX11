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
            Ok(if !(0..32).contains(&n) {
                0
            } else {
                x.wrapping_shl(n as u32)
            })
        })?,
    )?;
    bit.set(
        "rshift",
        // Stage-2 §六 B-1: LuaBitOp 1.0.2 spec requires `rshift` to be a
        // LOGICAL right shift (zero-fill). i32::wrapping_shr is arithmetic
        // (sign-extending), so for negative x it produced -1 where the
        // spec demands the high bits to be cleared. Reinterpret x as u32
        // for the shift, then return as i32 to preserve Lua's signed
        // return convention.
        //
        // Stage-2 §六 B-4: also clamp n to 0..32 to match the lshift
        // binding's behavior — `u32::wrapping_shr(32)` masks n to 0
        // (no-op), which silently violates the spec's "rotate modulo 32"
        // expectation. lshift already returns 0 for out-of-range n; rshift
        // must do the same so both functions behave symmetrically.
        lua.create_function(|_, (x, n): (i32, i32)| {
            Ok(if !(0..32).contains(&n) {
                0
            } else {
                (x as u32).wrapping_shr(n as u32) as i32
            })
        })?,
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
        // Stage-2 §六 B-3: LuaBitOp 1.0.2 §tohex says n < 0 produces
        // UPPERCASE hex, n >= 0 produces lowercase. abs(n) is the digit
        // count; the value mask is identical for both signs.
        lua.create_function(|_, (x, n): (i32, Option<i32>)| {
            let digits = n.unwrap_or(8);
            let abs_digits = if digits < 0 {
                (-digits) as usize
            } else {
                digits as usize
            };
            let mask = if abs_digits >= 8 {
                u32::MAX
            } else {
                (1u32 << (abs_digits * 4)).wrapping_sub(1)
            };
            let masked = (x as u32) & mask;
            if digits < 0 {
                Ok(format!("{:0>abs_digits$X}", masked, abs_digits = abs_digits))
            } else {
                Ok(format!("{:0>abs_digits$x}", masked, abs_digits = abs_digits))
            }
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
        unsafe {
            let lua = setup_lua();
            let result: i32 = lua.load("bit.bor(0xFF, 0x00)").eval().unwrap();
            assert_eq!(result, 0xFF);
        }
    }

    #[test]
    fn test_bit_band() {
        unsafe {
            let lua = setup_lua();
            let result: i32 = lua.load("bit.band(0xFF, 0x0F)").eval().unwrap();
            assert_eq!(result, 0x0F);
        }
    }

    #[test]
    fn test_bit_bxor() {
        unsafe {
            let lua = setup_lua();
            let result: i32 = lua.load("bit.bxor(0xFF, 0x0F)").eval().unwrap();
            assert_eq!(result, 0xF0);
        }
    }

    #[test]
    fn test_bit_bnot() {
        unsafe {
            let lua = setup_lua();
            let result: i32 = lua.load("bit.bnot(0)").eval().unwrap();
            assert_eq!(result, -1);
        }
    }

    #[test]
    fn test_bit_lshift() {
        unsafe {
            let lua = setup_lua();
            assert_eq!(1_i32, lua.load("bit.lshift(1, 0)").eval().unwrap());
            assert_eq!(256, lua.load("bit.lshift(1, 8)").eval::<i32>().unwrap());
            assert_eq!(0, lua.load("bit.lshift(1, 32)").eval::<i32>().unwrap()); // wrap
        }
    }

    #[test]
    fn test_bit_rshift() {
        unsafe {
            let lua = setup_lua();
            assert_eq!(1, lua.load("bit.rshift(256, 8)").eval::<i32>().unwrap());
            assert_eq!(0, lua.load("bit.rshift(1, 1)").eval::<i32>().unwrap());
            // Stage-2 §六 B-1: negative x must logical-shift (zero-fill),
            // not arithmetic-shift (sign-extend). Spec: rshift(-1, 31) == 1.
            assert_eq!(1, lua.load("bit.rshift(-1, 31)").eval::<i32>().unwrap());
            // -256 as i32 is 0xFFFFFF00; logical shift by 8 → 0x00FFFFFF = 16777215.
            assert_eq!(16777215, lua.load("bit.rshift(-256, 8)").eval::<i32>().unwrap());
        }
    }

    #[test]
    fn test_bit_arshift() {
        unsafe {
            let lua = setup_lua();
            assert_eq!(-1, lua.load("bit.arshift(-1, 0)").eval::<i32>().unwrap());
            assert_eq!(-4, lua.load("bit.arshift(-16, 2)").eval::<i32>().unwrap());
        }
    }

    #[test]
    fn test_bit_rol() {
        unsafe {
            let lua = setup_lua();
            assert_eq!(0x80, lua.load("bit.rol(0x01, 7)").eval::<i32>().unwrap());
        }
    }

    #[test]
    fn test_bit_ror() {
        unsafe {
            let lua = setup_lua();
            assert_eq!(0x02, lua.load("bit.ror(0x10, 3)").eval::<i32>().unwrap());
        }
    }

    #[test]
    fn test_bit_bswap() {
        unsafe {
            let lua = setup_lua();
            let result: i32 = lua.load("bit.bswap(0x12345678)").eval().unwrap();
            assert_eq!(result, 0x78563412);
        }
    }

    #[test]
    fn test_bit_tohex() {
        unsafe {
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
            // Stage-2 §六 B-3: negative n → UPPERCASE (spec §tohex).
            assert_eq!(
                "FF",
                lua.load(r#"bit.tohex(255, -2)"#).eval::<String>().unwrap()
            );
            assert_eq!(
                "FFFFFFFF",
                lua.load(r#"bit.tohex(-1, -8)"#).eval::<String>().unwrap()
            );
        }
    }

    #[test]
    fn test_bit_tobit() {
        unsafe {
            let lua = setup_lua();
            assert_eq!(1, lua.load("bit.tobit(1.0)").eval::<i32>().unwrap());
            assert_eq!(-1, lua.load("bit.tobit(-1.0)").eval::<i32>().unwrap());
        }
    }

    // ----------------------------------------------------------------------
    // Stage-2 §六 B-4: full-API spec audit.
    //
    // Each function cross-checked against LuaBitOp 1.0.2
    // (https://github.com/LuaDist/luabitop, the FCEUX Lua-ecosystem reference).
    // Behavior locked down here so future audits don't re-flag
    // spec-conforming code as buggy, and vice versa.
    // ----------------------------------------------------------------------

    /// LuaBitOp §tobit: coerce to signed 32-bit. Out-of-range values are
    /// taken modulo 2^32 and re-interpreted as signed.
    #[test]
    fn spec_tobit_modular_wrap() {
        unsafe {
            let lua = setup_lua();
            // 2^32 = 4294967296; spec says reduce modulo 2^32 → 0.
            assert_eq!(0, lua.load("bit.tobit(4294967296)").eval::<i32>().unwrap());
            // 2^32 + 5 → 5.
            assert_eq!(5, lua.load("bit.tobit(4294967301)").eval::<i32>().unwrap());
            // Negative out-of-range: -4294967295 → 1.
            assert_eq!(
                1,
                lua.load("bit.tobit(-4294967295)").eval::<i32>().unwrap()
            );
        }
    }

    /// LuaBitOp §bnot: bitwise NOT over 32 bits, returns signed.
    /// Bit-pattern 0xAAAAAAAA = -1431655766 as i32 (top bit set, must
    /// use the signed form so mlua can convert to i32).
    #[test]
    fn spec_bnot_full_width() {
        unsafe {
            let lua = setup_lua();
            // !-1 = 0 (all bits clear; using -1 = 0xFFFFFFFF)
            assert_eq!(0, lua.load("bit.bnot(-1)").eval::<i32>().unwrap());
            // !0 = -1 (all bits set, signed)
            assert_eq!(-1, lua.load("bit.bnot(0)").eval::<i32>().unwrap());
            // !-1431655766 = 0x55555555 (top bit clear, positive)
            assert_eq!(
                0x55555555,
                lua.load("bit.bnot(-1431655766)").eval::<i32>().unwrap()
            );
        }
    }

    /// LuaBitOp §band / §bor / §bxor: straightforward bitwise ops.
    /// Coverage audit: identity element, commutativity-via-symmetry, mask.
    #[test]
    fn spec_bitwise_identity() {
        unsafe {
            let lua = setup_lua();
            // Identity: x | 0 == x; x & ~0 == x; x ^ x == 0.
            assert_eq!(0x12345678, lua.load("bit.bor(0x12345678, 0)").eval::<i32>().unwrap());
            assert_eq!(0x12345678, lua.load("bit.band(0x12345678, -1)").eval::<i32>().unwrap());
            assert_eq!(0, lua.load("bit.bxor(0x12345678, 0x12345678)").eval::<i32>().unwrap());
            // Identity: x | 0xFFFFFFFF == -1 (signed); x & 0 == 0.
            assert_eq!(-1, lua.load("bit.bor(0x12345678, -1)").eval::<i32>().unwrap());
            assert_eq!(0, lua.load("bit.band(0x12345678, 0)").eval::<i32>().unwrap());
        }
    }

    /// LuaBitOp §lshift: shift left, modulo 2^32 (wrap), 0 ≤ n ≤ 31
    /// (n outside that range returns 0 per FCEUX's binding).
    #[test]
    fn spec_lshift_wrap_and_clamp() {
        unsafe {
            let lua = setup_lua();
            // Wrap: 1 << 31 = 0x80000000 = -2147483648 (signed).
            assert_eq!(
                -2147483648,
                lua.load("bit.lshift(1, 31)").eval::<i32>().unwrap()
            );
            // Out-of-range clamp: returns 0 (matches rshift B-4 behavior).
            assert_eq!(0, lua.load("bit.lshift(1, 32)").eval::<i32>().unwrap());
            // i32::MIN << 1 wraps to 0.
            assert_eq!(
                0,
                lua.load("bit.lshift(-2147483648, 1)").eval::<i32>().unwrap()
            );
        }
    }

    /// LuaBitOp §rshift: LOGICAL right shift (zero-fill). B-1 fixed
    /// arithmetic→logical; this locks down several sign-flip edge cases.
    #[test]
    fn spec_rshift_logical_across_signs() {
        unsafe {
            let lua = setup_lua();
            // Positive: identical to arithmetic shift.
            assert_eq!(1, lua.load("bit.rshift(0x100, 8)").eval::<i32>().unwrap());
            // Negative: zero-fill; high bits become 0, then re-interpreted
            // as i32 give a positive number.
            assert_eq!(
                0x00FFFFFF,
                lua.load("bit.rshift(-256, 8)").eval::<i32>().unwrap()
            );
            assert_eq!(
                0x7FFFFFFF,
                lua.load("bit.rshift(-2, 1)").eval::<i32>().unwrap()
            );
            // Out-of-range clamp (matches lshift behavior).
            assert_eq!(0, lua.load("bit.rshift(1, 32)").eval::<i32>().unwrap());
        }
    }

    /// LuaBitOp §arshift: ARITHMETIC right shift (sign-extending).
    /// Distinct from §rshift: arshift(-1, 1) == -1, but rshift(-1, 1) == 0x7FFFFFFF.
    #[test]
    fn spec_arshift_sign_extending() {
        unsafe {
            let lua = setup_lua();
            assert_eq!(-1, lua.load("bit.arshift(-1, 1)").eval::<i32>().unwrap());
            assert_eq!(-1, lua.load("bit.arshift(-1, 31)").eval::<i32>().unwrap());
            assert_eq!(-4, lua.load("bit.arshift(-16, 2)").eval::<i32>().unwrap());
            // Positive: same as logical.
            assert_eq!(1, lua.load("bit.arshift(2, 1)").eval::<i32>().unwrap());
        }
    }

    /// LuaBitOp §rol / §ror: rotate within 32 bits, signed return.
    /// Rotations by multiples of 32 are identity; rotate of 0 is identity.
    #[test]
    fn spec_rotate_identity_and_full() {
        unsafe {
            let lua = setup_lua();
            // Identity: rotate by 0 and by 32.
            assert_eq!(
                0x12345678,
                lua.load("bit.rol(0x12345678, 0)").eval::<i32>().unwrap()
            );
            assert_eq!(
                0x12345678,
                lua.load("bit.rol(0x12345678, 32)").eval::<i32>().unwrap()
            );
            assert_eq!(
                0x12345678,
                lua.load("bit.ror(0x12345678, 0)").eval::<i32>().unwrap()
            );
            // Rol(1, 1) = 2; ror(2, 1) = 1 (round-trip).
            assert_eq!(2, lua.load("bit.rol(1, 1)").eval::<i32>().unwrap());
            assert_eq!(1, lua.load("bit.ror(2, 1)").eval::<i32>().unwrap());
            // ror(-2147483648, 1) = 0x40000000 (top bit rotated to bit 30).
            assert_eq!(
                0x40000000,
                lua.load("bit.ror(-2147483648, 1)").eval::<i32>().unwrap()
            );
        }
    }

    /// LuaBitOp §bswap: reverse byte order of a 32-bit integer.
    #[test]
    fn spec_bswap_known_vectors() {
        unsafe {
            let lua = setup_lua();
            // bswap(0) == 0; bswap(-1) == -1 (all bytes identical).
            assert_eq!(0, lua.load("bit.bswap(0)").eval::<i32>().unwrap());
            assert_eq!(-1, lua.load("bit.bswap(-1)").eval::<i32>().unwrap());
            // bswap is its own inverse.
            assert_eq!(
                0x12345678,
                lua.load("bit.bswap(bit.bswap(0x12345678))")
                    .eval::<i32>()
                    .unwrap()
            );
            // Asymmetric byte pattern.
            assert_eq!(
                0x78563412,
                lua.load("bit.bswap(0x12345678)").eval::<i32>().unwrap()
            );
        }
    }

    /// LuaBitOp §tohex: spec audit covering both case branches and mask width.
    /// Cross-cuts B-2 (lowercase default) and B-3 (uppercase for n < 0).
    #[test]
    fn spec_tohex_full_table() {
        unsafe {
            let lua = setup_lua();
            // Default width 8, lowercase.
            assert_eq!("00000000", lua.load("bit.tohex(0)").eval::<String>().unwrap());
            assert_eq!("000000ff", lua.load("bit.tohex(255)").eval::<String>().unwrap());
            assert_eq!("ffffffff", lua.load("bit.tohex(-1)").eval::<String>().unwrap());
            // n = 1..7: low n hex digits, masked, lowercase.
            assert_eq!("1", lua.load("bit.tohex(1, 1)").eval::<String>().unwrap());
            assert_eq!("ff", lua.load("bit.tohex(0xFF, 2)").eval::<String>().unwrap());
            // n < 0: same mask, uppercase.
            assert_eq!("FF", lua.load("bit.tohex(0xFF, -2)").eval::<String>().unwrap());
            assert_eq!(
                "FFFFFFFF",
                lua.load("bit.tohex(-1, -8)").eval::<String>().unwrap()
            );
            // n = 0 (edge case): empty mask → all zeros masked out → "0".
            assert_eq!("0", lua.load("bit.tohex(0x12345678, 0)").eval::<String>().unwrap());
        }
    }
}
