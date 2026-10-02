//! The cartridge real-time clock (v2.0 S2-b3).
//!
//! # The chip is already there
//!
//! The vendored core implements the S3511 in full: `cpu/hardware/rtc.rs` decodes
//! the bit-banged command byte, streams BCD date/time back, and `InternalMemory`
//! wires it to the GPIO pins at ROM offsets `0xC4`–`0xC9` — writing any of them
//! marks the cartridge as having the chip, and reading `0xC4` hands back
//! whatever the chip is driving on SIO. **To a game, none of that needed doing,
//! and none of it needed us.** The plan's original phrasing ("RTC needs
//! exposing") described a missing feature; what was actually missing is the
//! ability to *verify* it.
//!
//! # What was missing: a time to report
//!
//! `current_unix_secs()` is a private free function reading `SystemTime::now()`.
//! Two consequences, and both of them are about evidence rather than
//! behaviour:
//!
//! - The core's own RTC test compares what the chip streams against
//!   `datetime_bytes(current_unix_secs())` — the host clock read *at the same
//!   moment*. A self-comparison cannot fail for a wrong date conversion, and it
//!   only reads the year byte, so the weekday, the hour, the minute and the
//!   second have no coverage at all.
//! - The GA gate's "RTC verified on a real game" has no answer without a moment
//!   the test controls, and falls through to "or recorded as a known
//!   limitation".
//!
//! So the patch this side needed from the core is one field and two accessors —
//! see `ATTRIBUTION.md` §3.2 entries 10-14. Everything else is ours.
//!
//! # Frozen, not offset
//!
//! A pinned clock **does not advance**. That is the behaviour both callers of
//! this feature want: a lock test needs a date it can state, and someone
//! debugging a game's clock wants the date they picked. A game that wants
//! elapsed time has the host clock, which is what it gets by default. The cost
//! is recorded as a known limitation rather than hidden: while the clock is
//! pinned, two reads of it are equal, so a game that derives elapsed time from
//! the RTC sees none.
//!
//! # Why there is no "does this cart have an RTC" query
//!
//! The core's `gpio_present` flag says the *game* has touched a GPIO register,
//! not that the cartridge carries the chip — those are different claims, and a
//! query named after the second would be answering with the first. Games that
//! need it poke the port themselves. Exposing it would invite a caller to
//! display "RTC" off a flag that means something else.

/// The moment the RTC reports right now, in seconds since the Unix epoch.
///
/// Matches what the core's own clock source would return for the same wall
/// clock, and what a pinned clock replaces.
#[must_use]
pub fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// Pin a machine's clock to `unix_secs`.
///
/// Any instant, including the Unix epoch itself — an earlier draft used `0` as
/// "release the pin", which quietly made the one instant the core's own tests
/// lean on untestable, and a sentinel that collides with a legal value is not a
/// sentinel. Use [`clear_time_override`] to release.
pub fn set_time_override(machine: &mut gba_core::gba::Gba, unix_secs: i64) {
    machine
        .cpu
        .bus
        .internal_memory
        .rtc_mut()
        .set_time_override(Some(unix_secs));
}

/// Release the pin, so the clock follows the host again.
pub fn clear_time_override(machine: &mut gba_core::gba::Gba) {
    machine
        .cpu
        .bus
        .internal_memory
        .rtc_mut()
        .set_time_override(None);
}

/// The pinned instant, or `None` when the machine is following the host clock.
#[must_use]
pub fn time_override(machine: &gba_core::gba::Gba) -> Option<i64> {
    machine.cpu.bus.internal_memory.rtc().time_override()
}

#[cfg(test)]
mod tests {
    use gba_core::gba::Gba;

    use super::{clear_time_override, now_unix_secs, set_time_override, time_override};

    /// A fresh machine with the stub BIOS and a minimal cartridge.
    fn machine() -> Box<Gba> {
        Box::new(Gba::new([0u8; 0x4000], &[0u8; 0x200]))
    }

    /// A new machine follows the host clock.
    #[test]
    fn a_new_machine_follows_the_host_clock() {
        let gba = machine();
        assert_eq!(
            time_override(&gba),
            None,
            "a fresh machine must not come up with the clock pinned"
        );
        // And the value the core would compute is a plausible present, not a
        // constant: 2020-01-01 was 1_577_836_800.
        let now = now_unix_secs();
        assert!(now > 1_577_836_800, "the host clock reads {now}");
    }

    /// Pinning and releasing both take effect, and release restores the host.
    #[test]
    fn pinning_and_releasing_the_clock() {
        let mut gba = machine();
        set_time_override(&mut gba, 1_612_325_106);
        assert_eq!(time_override(&gba), Some(1_612_325_106));

        clear_time_override(&mut gba);
        assert_eq!(time_override(&gba), None, "releasing did not take");
    }

    /// A pin survives a move of the machine.
    ///
    /// Not a hypothetical: `apply_state` in `frame.rs` moves a decoded `Arm7tdmi`
    /// into a freshly built `Gba`, so a pin that lived anywhere but the
    /// serialized state would be dropped by every savestate load.
    #[test]
    fn a_pin_travels_with_the_machine() {
        let mut first = machine();
        set_time_override(&mut first, 1_612_325_106);
        let moved = first;
        assert_eq!(
            time_override(&moved),
            Some(1_612_325_106),
            "the pin was dropped when the machine moved"
        );
    }

    // ---- the chip itself, driven over GPIO ---------------------------------
    //
    // The tests above are about our glue. These are about the S3511: they drive
    // the real bit-banged protocol through the public `Bus` and assert the date
    // the chip streams back. That is the only way the RTC is *verified* rather
    // than merely present, and it is the coverage the core's own RTC test does
    // not have -- that one compares the chip against the host clock read at the
    // same moment, so a wrong conversion cannot fail it, and it reads one byte
    // of the seven.

    /// S3511 pinout, as the core defines it (`rtc.rs`).
    const PIN_SCK: u8 = 1 << 0;
    const PIN_SIO: u8 = 1 << 1;
    const PIN_CS: u8 = 1 << 2;

    /// GPIO registers, at the ROM offsets the core maps at `0xC4`-`0xC9`.
    const GPIO_DATA: usize = 0x0800_00C4;
    const GPIO_DIRECTION: usize = 0x0800_00C6;

    /// 2021-02-03 04:05:06 UTC, a Wednesday. The same instant the core's own
    /// test uses, so the two can be compared rather than both being trusted.
    const PINNED_INSTANT: i64 = 1_612_325_106;

    fn gpio_write(gba: &mut Gba, address: usize, value: u8) {
        gba.cpu.bus.write_byte(address, value);
    }

    fn gpio_read(gba: &mut Gba, address: usize) -> u8 {
        gba.cpu.bus.read_byte(address)
    }

    /// Clock one bit into the chip: the game drives SIO, SCK's rising edge
    /// samples it, CS stays high.
    fn clock_in(gba: &mut Gba, bit: bool) {
        let base = PIN_CS | if bit { PIN_SIO } else { 0 };
        gpio_write(gba, GPIO_DIRECTION, PIN_SIO);
        gpio_write(gba, GPIO_DATA, base);
        gpio_write(gba, GPIO_DATA, base | PIN_SCK);
    }

    /// Clock one bit out: the chip drives SIO, and the bit is read *before* the
    /// rising edge that advances to the next one.
    fn clock_out(gba: &mut Gba) -> bool {
        gpio_write(gba, GPIO_DIRECTION, 0);
        gpio_write(gba, GPIO_DATA, PIN_CS);
        let bit = gpio_read(gba, GPIO_DATA) & PIN_SIO != 0;
        gpio_write(gba, GPIO_DATA, PIN_CS | PIN_SCK);
        bit
    }

    /// Select the chip and clock in a command byte, MSB first.
    ///
    /// Laid out as the core decodes it (`rtc.rs`): bits 7-4 are the fixed `0110`
    /// that identifies the chip, bits 3-1 are the command, and bit 0 is read
    /// versus write. The fixed part is a *nibble*, so it is `0x60` -- writing
    /// `0b0110_000` puts it in bits 5-2 instead and the chip rejects the whole
    /// byte as "not addressed to me", which it does by returning zeros with no
    /// error anywhere.
    fn command(gba: &mut Gba, command: u8, is_read: bool) {
        gpio_write(gba, GPIO_DIRECTION, PIN_SIO);
        gpio_write(gba, GPIO_DATA, PIN_CS);
        for index in (0..8).rev() {
            let bit = (0b0110_0000 | (command << 1) | u8::from(is_read)) >> index & 1 != 0;
            clock_in(gba, bit);
        }
    }

    /// Read `count` bytes out, LSB first within each byte.
    fn read_bytes(gba: &mut Gba, count: usize) -> Vec<u8> {
        (0..count)
            .map(|_| {
                let mut byte = 0u8;
                for bit in 0..8 {
                    byte |= u8::from(clock_out(gba)) << bit;
                }
                byte
            })
            .collect()
    }

    /// A machine whose clock is pinned to [`PINNED_INSTANT`].
    fn pinned() -> Box<Gba> {
        let mut gba = machine();
        set_time_override(&mut gba, PINNED_INSTANT);
        gba
    }

    /// The chip streams the pinned date, all seven of its bytes.
    ///
    /// The weekday is the byte worth having: it is the one with a convention
    /// behind it rather than a value, and it is exactly what the core's own test
    /// never reads. `0x03` is Wednesday under the S3511's `0 = Sunday` mapping,
    /// which 2021-02-03 satisfies.
    #[test]
    fn the_chip_streams_the_pinned_date() {
        let mut gba = pinned();
        command(&mut gba, 2, true); // date and time, read
        assert_eq!(
            read_bytes(&mut gba, 7),
            vec![0x21, 0x02, 0x03, 0x03, 0x04, 0x05, 0x06],
            "year, month, day, weekday, hour, minute, second"
        );
    }

    /// The time-only command returns three bytes, not seven.
    ///
    /// A second read path through the same conversion, and the one that slices
    /// `[4..7]` -- so it covers the half a date-only assertion would leave
    /// alone.
    #[test]
    fn the_chip_streams_the_pinned_time_alone() {
        let mut gba = pinned();
        command(&mut gba, 3, true); // time only, read
        assert_eq!(read_bytes(&mut gba, 3), vec![0x04, 0x05, 0x06]);
    }

    /// The weekday convention is `0 = Sunday`.
    ///
    /// A dedicated test for the one value that has a convention behind it,
    /// because it is the byte a conversion can get right by accident: 1970-01-01
    /// was a Thursday, and "day 0" and "day 4" are both defensible readings of
    /// that depending on which day you call the first.
    #[test]
    fn the_epoch_reads_as_a_thursday() {
        let mut gba = machine();
        set_time_override(&mut gba, 0);
        command(&mut gba, 2, true);
        let bytes = read_bytes(&mut gba, 7);
        assert_eq!(bytes[0], 0x70, "1970");
        assert_eq!(bytes[1], 0x01, "January");
        assert_eq!(bytes[2], 0x01, "the 1st");
        assert_eq!(bytes[3], 0x04, "Thursday, so Sunday is 0");
    }

    /// A released clock follows the host again.
    ///
    /// The chip, not our getter: this is the path a player is on, and it is the
    /// one a pin left in place would silently break.
    #[test]
    fn a_released_clock_follows_the_host() {
        let mut gba = pinned();
        clear_time_override(&mut gba);
        command(&mut gba, 2, true);
        let bytes = read_bytes(&mut gba, 7);
        let year = (bytes[0] >> 4) * 10 + (bytes[0] & 0x0F);
        assert!(
            year >= 24,
            "the chip still reports 20{year:02} after the pin was released"
        );
    }

    /// A pin survives a savestate round trip.
    ///
    /// The field is serialized, so this is what makes "pin the clock" survive
    /// loading a state -- without it a user who pinned a date would watch it
    /// revert on the first load, and a test that pinned a date would get a
    /// different answer after a round trip.
    #[test]
    fn a_pin_survives_a_savestate_round_trip() {
        let mut gba = pinned();
        let state = crate::gba::save::save(
            &gba.cpu,
            &crate::gba::save::RomFingerprint::of(&gba),
            crate::gba::save::AudioState::capture(&crate::gba::audio::AudioOut::new(44_100)),
            None,
        )
        .expect("serialize");
        let bytes: Vec<u8> = state
            .header
            .iter()
            .chain(state.payload.iter())
            .copied()
            .collect();

        let wire = crate::gba::save::load_on_worker(bytes)
            .expect("the worker started")
            .expect("decode");
        assert_eq!(
            wire.cpu.bus.internal_memory.rtc().time_override(),
            Some(PINNED_INSTANT),
            "the pin did not survive the round trip"
        );
    }
}
