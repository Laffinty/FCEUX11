use serde::{Deserialize, Serialize};

use crate::bitwise::Bits;

/// Timer overflow result indicating which IRQs to request
#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct TimerOverflowResult {
    pub timer0_overflow: bool,
    pub timer1_overflow: bool,
    pub timer2_overflow: bool,
    pub timer3_overflow: bool,
    /// Overflow of timers 0 and 1 regardless of their IRQ enable bit. The DMA
    /// sound engine is clocked by these and must react even when the timer IRQ
    /// is off.
    pub timer0_raw_overflow: bool,
    pub timer1_raw_overflow: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Timers {
    /// Timer 0 Counter (the actual running value read by games)
    pub tm0cnt_l: u16,
    /// Timer 0 Control
    pub tm0cnt_h: u16,
    /// Timer 0 Reload value (written to `tm0cnt_l`, loaded on overflow)
    pub(crate) tm0_reload: u16,
    /// Timer 0 prescaler counter
    tm0_prescaler_counter: u32,
    /// Timer 0 cycles still to elapse before counting starts
    tm0_start_delay: u8,

    /// Timer 1 Counter
    pub tm1cnt_l: u16,
    /// Timer 1 Control
    pub tm1cnt_h: u16,
    /// Timer 1 Reload value
    pub(crate) tm1_reload: u16,
    /// Timer 1 prescaler counter
    tm1_prescaler_counter: u32,
    /// Timer 1 cycles still to elapse before counting starts
    tm1_start_delay: u8,

    /// Timer 2 Counter
    pub tm2cnt_l: u16,
    /// Timer 2 Control
    pub tm2cnt_h: u16,
    /// Timer 2 Reload value
    pub(crate) tm2_reload: u16,
    /// Timer 2 prescaler counter
    tm2_prescaler_counter: u32,
    /// Timer 2 cycles still to elapse before counting starts
    tm2_start_delay: u8,

    /// Timer 3 Counter
    pub tm3cnt_l: u16,
    /// Timer 3 Control
    pub tm3cnt_h: u16,
    /// Timer 3 Reload value
    pub(crate) tm3_reload: u16,
    /// Timer 3 prescaler counter
    tm3_prescaler_counter: u32,
    /// Timer 3 cycles still to elapse before counting starts
    tm3_start_delay: u8,
}

impl Timers {
    /// Master cycles between the enable edge and the counter starting to move.
    ///
    /// The AGS `TIMER PRESCALER` routine enables TM0 and then measures 1024
    /// iterations of `SUBS` + `BNE` plus two `mov r0, r0`, a window of 4098
    /// master cycles, and expects the counter to read 4096 鈥?two cycles short of
    /// the window. The loop itself is exact (4092 cycles measured over 1023
    /// iterations, 4.00 per iteration), so the two cycles are the ones between
    /// the enable edge and the start of counting, not the loop.
    ///
    /// Calibrated against that anchor rather than derived: within this model a
    /// start delay and no start delay produce the same constant offset, and
    /// there is no internal reference for the constant. It is a fitted value,
    /// held only because a second anchor (`CONNECT`, plan r21 鈶? is required to
    /// agree before P4 counts as closed.
    const START_DELAY_CYCLES: u8 = 2;

    /// Get prescaler divider from control register bits 0-1
    fn get_prescaler(control: u16) -> u32 {
        match control & 0b11 {
            0 => 1,    // F/1
            1 => 64,   // F/64
            2 => 256,  // F/256
            3 => 1024, // F/1024
            _ => unreachable!(),
        }
    }

    /// Check if timer is enabled (bit 7)
    fn is_enabled(control: u16) -> bool {
        control.get_bit(7)
    }

    /// Check if timer uses count-up timing / cascade mode (bit 2)
    fn is_cascade(control: u16) -> bool {
        control.get_bit(2)
    }

    /// Check if timer IRQ is enabled (bit 6)
    fn is_irq_enabled(control: u16) -> bool {
        control.get_bit(6)
    }

    /// Called when writing to `TMxCNT_L`, sets the reload value
    pub const fn set_reload(&mut self, timer: usize, value: u16) {
        match timer {
            0 => self.tm0_reload = value,
            1 => self.tm1_reload = value,
            2 => self.tm2_reload = value,
            3 => self.tm3_reload = value,
            _ => {}
        }
    }

    /// Called when writing to `TMxCNT_H`, may start/restart timer
    pub fn set_control(&mut self, timer: usize, value: u16) {
        let (old_control, counter, reload) = match timer {
            0 => (self.tm0cnt_h, &mut self.tm0cnt_l, self.tm0_reload),
            1 => (self.tm1cnt_h, &mut self.tm1cnt_l, self.tm1_reload),
            2 => (self.tm2cnt_h, &mut self.tm2cnt_l, self.tm2_reload),
            3 => (self.tm3cnt_h, &mut self.tm3cnt_l, self.tm3_reload),
            _ => return,
        };

        // If timer is being enabled (bit 7: 0->1), reload the counter
        let was_enabled = Self::is_enabled(old_control);
        let now_enabled = Self::is_enabled(value);

        if !was_enabled && now_enabled {
            *counter = reload;
            // Counting does not begin on this write: the counter starts moving
            // `START_DELAY_CYCLES` later (see its doc comment).
            match timer {
                0 => {
                    self.tm0_prescaler_counter = 0;
                    self.tm0_start_delay = Self::START_DELAY_CYCLES;
                }
                1 => {
                    self.tm1_prescaler_counter = 0;
                    self.tm1_start_delay = Self::START_DELAY_CYCLES;
                }
                2 => {
                    self.tm2_prescaler_counter = 0;
                    self.tm2_start_delay = Self::START_DELAY_CYCLES;
                }
                3 => {
                    self.tm3_prescaler_counter = 0;
                    self.tm3_start_delay = Self::START_DELAY_CYCLES;
                }
                _ => {}
            }
        }

        // Store the new control value
        match timer {
            0 => self.tm0cnt_h = value,
            1 => self.tm1cnt_h = value,
            2 => self.tm2cnt_h = value,
            3 => self.tm3cnt_h = value,
            _ => {}
        }
    }

    /// Advance all timers by `cycles` CPU cycles. Returns which timer IRQs
    /// should be triggered (an overflow happened while that timer's IRQ is
    /// enabled).
    ///
    /// Cascade timers tick once per overflow of the timer below them, so the
    /// overflow counts are threaded down the chain independently of whether
    /// each timer's IRQ is enabled.
    pub fn step(&mut self, cycles: u64) -> TimerOverflowResult {
        // The cascade bit has no effect on timer 0: it still counts on its own
        // prescaler. Ignoring the bit is what the hardware does — a cartridge
        // that sets it (the AGS `TIMER CONNECT` routine writes `0x0084` to all
        // four control registers) gets a running timer 0, not a stopped one.
        let tm0 = if Self::is_enabled(self.tm0cnt_h) {
            Self::advance(
                &mut self.tm0cnt_l,
                self.tm0_reload,
                &mut self.tm0_prescaler_counter,
                &mut self.tm0_start_delay,
                Self::get_prescaler(self.tm0cnt_h),
                cycles,
            )
        } else {
            0
        };

        let tm1 = self.step_timer(self.tm1cnt_h, cycles, tm0, 1);
        let tm2 = self.step_timer(self.tm2cnt_h, cycles, tm1, 2);
        let tm3 = self.step_timer(self.tm3cnt_h, cycles, tm2, 3);

        TimerOverflowResult {
            timer0_overflow: tm0 > 0 && Self::is_irq_enabled(self.tm0cnt_h),
            timer1_overflow: tm1 > 0 && Self::is_irq_enabled(self.tm1cnt_h),
            timer2_overflow: tm2 > 0 && Self::is_irq_enabled(self.tm2cnt_h),
            timer3_overflow: tm3 > 0 && Self::is_irq_enabled(self.tm3cnt_h),
            timer0_raw_overflow: tm0 > 0,
            timer1_raw_overflow: tm1 > 0,
        }
    }

    /// Advance one of timers 1..=3 and return how many times it overflowed.
    /// In cascade mode it ticks once per overflow of the timer below it,
    /// otherwise it is driven by its own prescaler.
    fn step_timer(&mut self, control: u16, cycles: u64, prev_overflows: u32, timer: usize) -> u32 {
        if !Self::is_enabled(control) {
            return 0;
        }

        let (counter, reload, prescaler_counter, start_delay) = match timer {
            1 => (
                &mut self.tm1cnt_l,
                self.tm1_reload,
                &mut self.tm1_prescaler_counter,
                &mut self.tm1_start_delay,
            ),
            2 => (
                &mut self.tm2cnt_l,
                self.tm2_reload,
                &mut self.tm2_prescaler_counter,
                &mut self.tm2_start_delay,
            ),
            _ => (
                &mut self.tm3cnt_l,
                self.tm3_reload,
                &mut self.tm3_prescaler_counter,
                &mut self.tm3_start_delay,
            ),
        };

        if Self::is_cascade(control) {
            Self::apply_ticks(counter, reload, prev_overflows)
        } else {
            Self::advance(
                counter,
                reload,
                prescaler_counter,
                start_delay,
                Self::get_prescaler(control),
                cycles,
            )
        }
    }

    /// Advance a prescaler-driven timer by `cycles`, carrying the prescaler
    /// remainder across calls. Returns how many times the counter overflowed.
    ///
    /// `start_delay` is consumed first: those cycles elapse before the counter
    /// starts moving, so they neither advance the prescaler nor reach the
    /// counter.
    // The remainder is below the prescaler (<= 1024) and the tick count fits a
    // u32 for any realistic per-step cycle delta, so the casts cannot truncate.
    #[allow(clippy::cast_possible_truncation)]
    fn advance(
        counter: &mut u16,
        reload: u16,
        prescaler_counter: &mut u32,
        start_delay: &mut u8,
        prescaler: u32,
        cycles: u64,
    ) -> u32 {
        let delay = u64::from(*start_delay).min(cycles);
        *start_delay -= delay as u8;
        let cycles = cycles - delay;
        if cycles == 0 {
            return 0;
        }

        let total = u64::from(*prescaler_counter) + cycles;
        let ticks = total / u64::from(prescaler);
        *prescaler_counter = (total % u64::from(prescaler)) as u32;
        Self::apply_ticks(counter, reload, ticks as u32)
    }

    /// Add `ticks` to a timer counter, reloading on each overflow. Returns the
    /// number of overflows. After the first overflow the counter runs from
    /// `reload`, so the wrap period is `0x1_0000 - reload`.
    // The two `as u16` results are reduced modulo the wrap period, so they are
    // always below `0x1_0000` and cannot truncate.
    #[allow(clippy::cast_possible_truncation)]
    fn apply_ticks(counter: &mut u16, reload: u16, ticks: u32) -> u32 {
        if ticks == 0 {
            return 0;
        }

        let value = u32::from(*counter) + ticks;
        if value < 0x1_0000 {
            *counter = value as u16;
            return 0;
        }

        let period = 0x1_0000 - u32::from(reload);
        let past_first = value - 0x1_0000;
        let overflows = 1 + past_first / period;
        *counter = (u32::from(reload) + past_first % period) as u16;
        overflows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Control register bits: enable (7), irq (6), cascade (2), prescaler (0-1).
    const ENABLE: u16 = 1 << 7;
    const IRQ: u16 = 1 << 6;
    const CASCADE: u16 = 1 << 2;

    #[test]
    fn overflows_after_exactly_one_full_period() {
        let mut t = Timers::default();
        t.set_reload(0, 0);
        t.set_control(0, ENABLE | IRQ); // prescaler 1, counter reloads to 0

        // The enable edge spends START_DELAY_CYCLES before counting begins, so
        // the period is measured over the cycles that follow it.
        let delay = u64::from(Timers::START_DELAY_CYCLES);

        // One short of a full 16-bit period: no overflow yet.
        let r = t.step(0xFFFF + delay);
        assert!(!r.timer0_overflow);
        assert_eq!(t.tm0cnt_l, 0xFFFF);

        // The cycle that completes the period overflows and reloads to 0.
        let r = t.step(1);
        assert!(r.timer0_overflow);
        assert_eq!(t.tm0cnt_l, 0);
    }

    #[test]
    fn prescaler_divides_and_carries_remainder() {
        let mut t = Timers::default();
        t.set_reload(0, 0xFFFF); // wrap period of one tick
        t.set_control(0, ENABLE | IRQ | 0b01); // prescaler 64

        let delay = u64::from(Timers::START_DELAY_CYCLES);

        // 63 counted cycles: not enough for a single tick.
        assert!(!t.step(63 + delay).timer0_overflow);
        assert_eq!(t.tm0cnt_l, 0xFFFF);

        // The 64th counted cycle produces one tick, which overflows the
        // reloaded counter.
        let r = t.step(1);
        assert!(r.timer0_overflow);
        assert_eq!(t.tm0cnt_l, 0xFFFF);
    }

    /// AGS PRESCALER 鐨勭獥鍙ｅ舰鐘讹紙纭欢鏍″噯鍊硷級锛歍MxCNT_H bit0-1 閫夊垎棰戯紝
    /// 璁℃暟鍣ㄤ粠 0 璧锋暟锛?096 涓?IWRAM 闆剁瓑寰呭懆鏈熷悗璇绘暟 = 4096/鍒嗛銆?    ///
    /// 绐楀彛鏄?**4098** 涓富鍛ㄦ湡鑰屼笉鏄?4096锛欰GS 渚嬬▼鍦?1024 娆?`SUBS`+`BNE`
    /// 寰幆涔嬪悗銆佽璁℃暟鍣ㄤ箣鍓嶏紝杩樻墽琛屼袱鏉?`mov r0, r0`銆傞偅涓ゆ潯鎸囦护鐨?2 涓懆鏈?    /// 钀藉湪浣胯兘杈规部涓庤捣鏁颁箣闂达紝琚?`START_DELAY_CYCLES` 鍚告敹锛屾墍浠ヨ鏁板埌鐨勪粛
    /// 鏄?4096 鈥斺€?杩欐鏄湰娴嬭瘯鑳介拤浣忛偅涓父鏁扮殑鍘熷洜銆?    ///
    /// 鍙樺紓锛歚START_DELAY_CYCLES` 鏀瑰洖 0锛屆? 妗ｈ浆绾紙璇诲埌 4098锛夈€?    #[test]
    fn the_ags_prescaler_cases_hold_for_every_divider() {
        let mut t = Timers::default();
        let expected = [4096u32, 64, 16, 4];
        for (field, want) in expected.iter().enumerate() {
            // AGS 渚嬬▼鐨勪袱娈靛紡锛氬厛鍐?HI=0 澶辫兘锛屽啀鍐欎娇鑳?鍒嗛锛?2 浣嶅啓鐨勪袱涓崐瀛楋級銆?            // 涓嶅け鑳藉垯 0->1 杈规部涓嶆垚绔嬶紝璁℃暟鍣ㄤ笉娓呴浂锛堟湰娴嬭瘯绗竴鐗堝氨姝诲湪杩欙級銆?            t.set_control(0, 0);
            t.set_reload(0, 0);
            t.set_control(0, ENABLE | field as u16);
            t.step(4096 + u64::from(Timers::START_DELAY_CYCLES));
            assert_eq!(t.tm0cnt_l as u32, *want, "prescaler field {field}");
        }
    }

    #[test]
    fn multiple_overflows_in_a_single_step() {
        let mut t = Timers::default();
        t.set_reload(0, 0xFF00); // period of 0x100 ticks
        t.set_control(0, ENABLE | IRQ);

        // Two full periods plus a bit in one step.
        let r = t.step(0x100 * 2 + 5 + u64::from(Timers::START_DELAY_CYCLES));
        assert!(r.timer0_overflow);
        assert_eq!(t.tm0cnt_l, 0xFF05);
    }

    #[test]
    fn cascade_ticks_once_per_lower_overflow() {
        let mut t = Timers::default();
        // Timer 0 overflows every counted cycle (period of one tick).
        t.set_reload(0, 0xFFFF);
        t.set_control(0, ENABLE);

        // Timer 1 cascades, period of two ticks.
        t.set_reload(1, 0xFFFE);
        t.set_control(1, ENABLE | IRQ | CASCADE);

        // Three timer-0 overflows advance the cascade timer three ticks,
        // which is one and a half of its two-tick period: one overflow.
        let r = t.step(3 + u64::from(Timers::START_DELAY_CYCLES));
        assert!(r.timer1_overflow);
        assert_eq!(t.tm1cnt_l, 0xFFFF);
    }

    /// Pins `START_DELAY_CYCLES` against the cartridge's own expectation, with the
    /// window written as a **literal**.
    ///
    /// The other timer tests spend `4096 + START_DELAY_CYCLES`, which means they
    /// follow the constant and stay green when it changes 鈥?a gate written as a
    /// mirror of the thing it is supposed to police. This one does not: the AGS
    /// window is 4098 master cycles and the cartridge expects the counter to
    /// read 4096, and both numbers are written down here. Changing the constant
    /// to 0 turns this red (the counter reads 4098); changing it to 3 turns it
    /// red the other way (4095).
    #[test]
    fn the_ags_window_counts_4096_of_its_4098_cycles() {
        let mut t = Timers::default();
        t.set_control(0, 0);
        t.set_reload(0, 0);
        t.set_control(0, ENABLE); // divide by one

        t.step(4098); // 1024 iterations of SUBS+BNE, then two `mov r0, r0`

        assert_eq!(u32::from(t.tm0cnt_l), 4096);
    }

    /// Research-only probe: the AGS `TIMER CONNECT` case, modelled straight from
    /// `sub_8009294`. The cartridge expects **512**.
    ///
    /// The write order matters and is easy to get wrong. `sub_8009294` writes
    /// TM3 first and **still has reload 0 at that moment** — `0x0084_0000` — so
    /// TM3's counter is loaded with 0 and, having no reload until the end of the
    /// window, never wraps. Only afterwards are TM2, TM1 and TM0 written with
    /// `0x0084_FFFE`. Writing all four at `0xFFFE` (the first version of this
    /// probe) gives TM3 a wrap period of 2 ticks, so it can only ever read
    /// `0xFFFE` — a broken probe that looks exactly like a broken cascade chain.
    ///
    /// The enable order also sets the four start delays, and TM0 is enabled last,
    /// which is what fixes the window: 4096 cycles of `SUBS`/`BNE` plus the two
    /// `mov r0, r0`, minus the delay TM0 still has to work off.
    #[test]
    #[ignore = "prints a measurement to compare by hand"]
    fn measure_the_ags_timer_connect_case() {
        const CONTROL: u16 = 0x0084; // enable (bit 7) + cascade (bit 2)

        let mut t = Timers::default();
        // TM3 first, with reload still 0.
        t.set_reload(3, 0);
        t.set_control(3, CONTROL);
        // Then TM2, TM1 and TM0, all with reload 0xFFFE (period of two ticks).
        for timer in 0..3 {
            t.set_reload(timer, 0xFFFE);
            t.set_control(timer, CONTROL);
        }

        t.step(4098); // 1024 x (SUBS + BNE), then two `mov r0, r0`

        println!("AGS TIMER CONNECT");
        println!("  TM0 = {}", t.tm0cnt_l);
        println!("  TM1 = {}", t.tm1cnt_l);
        println!("  TM2 = {}", t.tm2cnt_l);
        println!("  TM3 = {}   (AGS expects 512)", t.tm3cnt_l);
    }

    /// The cascade bit has no effect on timer 0. Before this was pinned, the code
    /// skipped timer 0 entirely whenever the bit was set — the exact opposite of
    /// what its own comment claimed — which stalled the whole cascade chain and
    /// is why AGS `TIMER CONNECT` could not pass.
    #[test]
    fn timer_zero_keeps_counting_when_the_cascade_bit_is_set() {
        let mut t = Timers::default();
        t.set_reload(0, 0xFFFE); // wrap period of two ticks
        t.set_control(0, ENABLE | IRQ | CASCADE);

        // The delay is symbolic here on purpose: this test pins the cascade bit,
        // not the constant. `the_ags_window_counts_4096_of_its_4098_cycles` is
        // what pins the value.
        let delay = u64::from(Timers::START_DELAY_CYCLES);
        t.step(delay);
        assert_eq!(t.tm0cnt_l, 0xFFFE, "counting must not start inside the delay");

        assert!(!t.step(1).timer0_overflow);
        assert_eq!(t.tm0cnt_l, 0xFFFF);

        // The second counted cycle completes the period and overflows.
        assert!(t.step(1).timer0_overflow);
    }

    /// The AGS `TIMER CONNECT` chain, under a **simultaneous-enable** model.
    ///
    /// What this pins: that the chain is alive. Before the cascade-bit fix,
    /// timer 0 stopped dead whenever the bit was set, the chain never started
    /// and TM3 read 0. That is a real defect and this test kills it.
    ///
    /// What this does **not** claim: that AGS `TIMER CONNECT` passes. It still
    /// fails on the cartridge after the fix, because the real routine enables
    /// TM3..TM0 with four separate stores a few cycles apart, so each timer's
    /// start delay expires at a different point and the chain's phase differs
    /// from this model's. Reading 512 here is a property of the simplified
    /// model, not evidence about the cartridge. Treat the AGS screenshot as the
    /// authority on that test and this one as the authority on the chain.
    #[test]
    fn the_ags_timer_connect_chain_reaches_512() {
        const CONTROL: u16 = 0x0084; // enable + cascade

        let mut t = Timers::default();
        t.set_reload(3, 0);
        t.set_control(3, CONTROL);
        for timer in 0..3 {
            t.set_reload(timer, 0xFFFE);
            t.set_control(timer, CONTROL);
        }

        t.step(4098);

        assert_eq!(t.tm3cnt_l, 512);
    }

    /// Research-only probe: sweep the phase of the four enables in `sub_8009294`.
    ///
    /// The real routine enables TM3 first (while its reload is still 0), then
    /// TM2, TM1 and TM0 with three separate stores a few cycles apart. The
    /// earlier probe fired all four enables at the same instant, which cannot
    /// represent that, and it read 512 anyway — so either the phase does not
    /// matter or the model is missing something.
    ///
    /// This answers it directly: sweep the spacing between enables and whether
    /// cascaded timers work off their own start delay, and report every
    /// combination that lands on the cartridge's 512. If none does, the gap is
    /// not a phase problem and this line of attack is finished.
    #[test]
    #[ignore = "prints a measurement to compare by hand"]
    fn sweep_the_phase_of_the_four_enables() {
        const CONTROL: u16 = 0x0084;
        const EXPECTED: u16 = 512;

        println!("AGS TIMER CONNECT phase sweep");
        println!("  (TM3 enabled first with reload 0; TM0 last; reading is TM3)");
        println!("   spacing = cycles between successive enables");
        println!("   extra   = cycles after the 4096-cycle loop, before the read");
        print!("   extra: ");
        for extra in 0..5u64 {
            print!("{extra:>6}");
        }
        println!();

        let mut hits = 0;
        for spacing in 0..16u64 {
            print!("  {spacing:>2}:     ");
            for extra in 0..5u64 {
                let mut t = Timers::default();
                t.set_reload(3, 0);
                t.set_control(3, CONTROL);
                for timer in 0..3 {
                    t.step(spacing);
                    t.set_reload(timer, 0xFFFE);
                    t.set_control(timer, CONTROL);
                }
                // Measured from **timer 0's** enable, which is the last of the
                // four: the loop (4096) plus the two `mov r0, r0`. The spacing
                // sits *before* that point and must not be subtracted from it.
                t.step(4096 + extra);

                let reading = t.tm3cnt_l;
                if reading == EXPECTED {
                    hits += 1;
                }
                print!("{reading:>6}");
            }
            println!();
        }
        println!("  combinations: 80, matches for {EXPECTED}: {hits}");

        // Does the *chunking* of `step` matter? The bus drains timers once per
        // instruction with whatever has accumulated, while the sweep above hands
        // the whole window over in one call. A cascade chain can lose a tick at
        // those boundaries, and the real machine never sees the one-big-call
        // shape — so this is the most likely place the model and the cartridge
        // part company.
        println!();
        println!("  chunked the way the bus actually drains it:");
        for chunk in [1u64, 2, 4] {
            let mut t = Timers::default();
            t.set_reload(3, 0);
            t.set_control(3, CONTROL);
            for timer in 0..3 {
                t.step(2);
                t.set_reload(timer, 0xFFFE);
                t.set_control(timer, CONTROL);
            }
            let mut left = 4098;
            while left > 0 {
                let n = chunk.min(left);
                t.step(n);
                left -= n;
            }
            println!("    chunk {chunk} -> TM3 = {}", t.tm3cnt_l);
        }
        println!("    one call of 4098 -> TM3 = 512 (see above)");
    }

    /// A start delay that is consumed once, not re-armed every step: the
    /// remainder must survive across calls instead of resetting.
    ///
    /// Discriminates a *re-arm* bug, not the constant's value 鈥?with the delay
    /// at 0 this stays green, which is correct: `the_ags_window_counts_4096_of_
    /// its_4098_cycles` is the test that pins the value.
    #[test]
    fn the_start_delay_is_consumed_once_and_not_re_armed() {
        let mut t = Timers::default();
        t.set_reload(0, 0);
        t.set_control(0, ENABLE); // 梅1

        // One cycle at a time: the first START_DELAY_CYCLES do nothing, then
        // counting resumes. If the delay were re-armed per step the counter
        // would never move at all.
        for _ in 0..Timers::START_DELAY_CYCLES {
            t.step(1);
            assert_eq!(t.tm0cnt_l, 0, "counting started inside the delay");
        }
        t.step(1);
        assert_eq!(t.tm0cnt_l, 1);
        t.step(9);
        assert_eq!(t.tm0cnt_l, 10);
    }

    #[test]
    fn disabled_timer_does_not_advance() {
        let mut t = Timers::default();
        t.set_reload(0, 0);
        // Not enabled.
        let r = t.step(1_000_000);
        assert!(!r.timer0_overflow);
        assert_eq!(t.tm0cnt_l, 0);
    }
}
