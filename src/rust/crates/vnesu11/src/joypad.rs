//! Joypad module — standard NES joypad + VS coin extension.
//!
//! Phase 4 ships the standard joypad (8-bit shift register + strobe).
//! VS UniSystem coin support is a Phase 5 mapper concern; the data
//! model is ready (`coin` field, 2-pad allocation).
//!
//! Reference: `src/console_interface.cpp::FCEU_UpdateJoyPad` + NESdev wiki
//! "Standard controllers".

/// Standard NES button layout (8 bits, bit 0 = A).
pub const BUTTON_A: u8 = 0x01;
pub const BUTTON_B: u8 = 0x02;
pub const BUTTON_SELECT: u8 = 0x04;
pub const BUTTON_START: u8 = 0x08;
pub const BUTTON_UP: u8 = 0x10;
pub const BUTTON_DOWN: u8 = 0x20;
pub const BUTTON_LEFT: u8 = 0x40;
pub const BUTTON_RIGHT: u8 = 0x80;

/// Two joypads (standard NES) + 1 coin slot (VS UniSystem).
#[derive(Debug, Clone, Default)]
pub struct JoypadState {
    /// Pad 1 button state (8 bits).
    pub button_state: [u8; 2],
    /// Pad 1 shift register (8 bits, rotated on each read).
    pub shift_register: [u8; 2],
    /// Strobe ($4016 bit 0). 1 = refresh on each read.
    pub strobe: bool,
    /// VS Unisystem coin slot (1 = coin inserted).
    pub coin: u8,
    /// VS coin strobe (write-only).
    pub coin_strobe: bool,
}

impl JoypadState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a button (called by Qt input handler).
    pub fn set_button(&mut self, pad: usize, button: u8, pressed: bool) {
        if pad >= 2 {
            return;
        }
        if pressed {
            self.button_state[pad] |= button;
        } else {
            self.button_state[pad] &= !button;
        }
    }

    /// Read $4016/$4017.  Pad 0 = $4016, Pad 1 = $4017.
    ///
    /// In strobe mode, reads continuously return the current button state.
    /// Outside strobe, reads return the MSB of the shift register and
    /// shift left by 1.
    pub fn read(&mut self, addr: u16) -> u8 {
        let pad = (addr & 1) as usize;
        if pad >= 2 {
            return 0;
        }
        if self.strobe {
            // Strobe mode: return MSB of the button state directly.
            (self.button_state[pad] & 0x80) >> 7
        } else {
            let bit = (self.shift_register[pad] & 0x80) >> 7;
            self.shift_register[pad] <<= 1;
            // Open-bus behavior: bits 0-3 of the result are open bus.
            bit
        }
    }

    /// Write $4016 (strobe).
    pub fn write_strobe(&mut self, val: u8) {
        let strobe_bit = (val & 0x01) != 0;
        if strobe_bit {
            // Strobe rising edge: refresh shift register from button state.
            self.shift_register[0] = self.button_state[0];
            self.shift_register[1] = self.button_state[1];
        }
        self.strobe = strobe_bit;
        // VS coin handling: bit 1 is coin counter strobe.
        if (val & 0x02) != 0 {
            self.coin_strobe = true;
        }
    }

    /// Reset for power-cycle.
    pub fn reset(&mut self) {
        self.button_state = [0; 2];
        self.shift_register = [0; 2];
        self.strobe = false;
        self.coin = 0;
        self.coin_strobe = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strobe_latches_button_state() {
        let mut j = JoypadState::new();
        // Press button at MSB position so the high bit is correctly
        // reflected in the strobe-mode read.
        j.set_button(0, BUTTON_RIGHT, true); // bit 7
        assert_eq!(j.read(0x4016), 0); // not latched yet, shift = 0
        // First write strobe: refresh shift register from button state.
        j.write_strobe(0x01);
        // In strobe mode, read returns MSB of button state.
        assert_eq!(j.read(0x4016), 1);
    }

    #[test]
    fn strobe_off_returns_open_bus() {
        let mut j = JoypadState::new();
        j.set_button(0, BUTTON_A, true);
        j.write_strobe(0x01); // rising edge → refresh
        j.write_strobe(0x00); // strobe off
        // First read: MSB of shift register (was 0x01, MSB = 0).
        assert_eq!(j.read(0x4016), 0);
        // After 8 reads, the shift register is all zeros.
        for _ in 0..7 {
            j.read(0x4016);
        }
        assert_eq!(j.read(0x4016), 0);
    }

    #[test]
    fn pad_1_read_from_4017() {
        let mut j = JoypadState::new();
        j.set_button(1, BUTTON_B, true);
        j.write_strobe(0x01);
        assert_eq!(j.read(0x4017), 0); // bit 1 = B, MSB = 0
    }

    #[test]
    fn button_release_clears_bit() {
        let mut j = JoypadState::new();
        j.set_button(0, BUTTON_A, true);
        j.set_button(0, BUTTON_A, false);
        j.write_strobe(0x01);
        assert_eq!(j.read(0x4016), 0);
    }
}