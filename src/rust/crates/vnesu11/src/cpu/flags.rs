//! 6502 P register flag constants + helper methods.
//!
//! Bit positions are stable hardware constants; the masks here match
//! `src/x6502abbrev.h` (`x6502.h` v1.13 H) byte-for-byte:
//!
//! ```text
//!   bit 7 (0x80) = N (negative)
//!   bit 6 (0x40) = V (overflow)
//!   bit 5 (0x20) = U (unused; always 1)
//!   bit 4 (0x10) = B (break; pushed by BRK/IRQ/NMI)
//!   bit 3 (0x08) = D (decimal)
//!   bit 2 (0x04) = I (interrupt disable)
//!   bit 1 (0x02) = Z (zero)
//!   bit 0 (0x01) = C (carry)
//! ```

pub const N_FLAG: u8 = 0x80;
pub const V_FLAG: u8 = 0x40;
pub const U_FLAG: u8 = 0x20; // unused; always 1 in P
pub const B_FLAG: u8 = 0x10;
pub const D_FLAG: u8 = 0x08;
pub const I_FLAG: u8 = 0x04;
pub const Z_FLAG: u8 = 0x02;
pub const C_FLAG: u8 = 0x01;

/// Initial P value: U flag set, I flag set (interrupts disabled until SEI/CLI).
pub const INITIAL_P: u8 = U_FLAG | I_FLAG;

/// Pushed P during BRK/IRQ/NMI: B flag set, U flag set; B is only pushed (not in P itself).
pub const PUSHED_P_BRK: u8 = U_FLAG | B_FLAG;
pub const PUSHED_P_IRQ: u8 = U_FLAG;
pub const PUSHED_P_NMI: u8 = U_FLAG;

#[inline(always)]
pub const fn get_flag(p: u8, flag: u8) -> bool {
    (p & flag) != 0
}

#[inline(always)]
pub const fn set_flag(p: u8, flag: u8, on: bool) -> u8 {
    if on { p | flag } else { p & !flag }
}

/// Set Z and N flags from an 8-bit result.
#[inline(always)]
pub fn set_zn(p: u8, val: u8) -> u8 {
    let mut p = p;
    p = set_flag(p, Z_FLAG, val == 0);
    p = set_flag(p, N_FLAG, val & 0x80 != 0);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_constants_match_abbrev() {
        assert_eq!(N_FLAG, 0x80);
        assert_eq!(V_FLAG, 0x40);
        assert_eq!(U_FLAG, 0x20);
        assert_eq!(B_FLAG, 0x10);
        assert_eq!(D_FLAG, 0x08);
        assert_eq!(I_FLAG, 0x04);
        assert_eq!(Z_FLAG, 0x02);
        assert_eq!(C_FLAG, 0x01);
    }

    #[test]
    fn initial_p_has_u_and_i() {
        assert_eq!(INITIAL_P, U_FLAG | I_FLAG);
    }

    #[test]
    fn set_zn_z_flag() {
        assert_eq!(set_zn(0, 0) & Z_FLAG, Z_FLAG);
        assert_eq!(set_zn(Z_FLAG, 0) & Z_FLAG, Z_FLAG);
        assert_eq!(set_zn(Z_FLAG, 1) & Z_FLAG, 0);
    }

    #[test]
    fn set_zn_n_flag() {
        assert_eq!(set_zn(0, 0xFF) & N_FLAG, N_FLAG);
        assert_eq!(set_zn(0, 0x7F) & N_FLAG, 0);
    }
}
