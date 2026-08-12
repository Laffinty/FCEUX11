//! Interrupt handling — NMI / IRQ / RESET with penultimate-cycle sampling.
//!
//! Mirrors the `_IRQlow` block in `src/x6502.cpp` `X6502_RunDebug`
//! (lines ~511-578). Priority: RESET > NMI2 (convert to NMI) > NMI > IRQ.
//! Each sequence pushes PC/P and vectors, then decrements `count` by 7.
//! If the budget is exhausted the CPU returns before fetching the next
//! instruction (C++ `if(_count<=0) return`).

use super::BusContext;
use super::CpuCore;
use super::flags::{self, B_FLAG, U_FLAG};

impl CpuCore {
    /// Sample pending interrupts at the start of an instruction.
    /// Returns true if an interrupt was serviced (and the CPU may have
    /// stopped early due to budget exhaustion).
    #[inline(always)]
    pub fn poll_interrupts<BC: BusContext>(&mut self, bus: &mut BC) -> bool {
        if self.irq_pending == 0 {
            return false;
        }

        let pending = self.irq_pending;

        // RESET has highest priority.
        if pending & Self::IRQ_RESET != 0 {
            self.pc = self.read16(bus, 0xFFFC);
            self.jammed = false;
            self.moo_pi = flags::INITIAL_P;
            self.p = flags::INITIAL_P;
            self.irq_pending &= !Self::IRQ_RESET;
            // C++ does NOT consume cycles for RESET here (it is handled
            // by the caller); matching X6502_RunDebug semantics: RESET is
            // serviced inline and execution continues with the fetch.
            return true;
        }

        // NMI2 (delayed NMI) converts to NMI.
        if pending & Self::IRQ_NMI2 != 0 {
            self.irq_pending &= !Self::IRQ_NMI2;
            self.irq_pending |= Self::IRQ_NMI;
            // Fall through: the NMI bit is now set; service it below.
        }

        // NMI (edge-triggered).
        if self.irq_pending & Self::IRQ_NMI != 0 {
            if !self.jammed {
                self.count -= 7;
                self.push(bus, (self.pc >> 8) as u8);
                self.push(bus, (self.pc & 0xFF) as u8);
                self.push(bus, (self.p & !B_FLAG) | U_FLAG);
                self.p |= flags::I_FLAG;
                self.pc = self.read16(bus, 0xFFFA);
                self.irq_pending &= !Self::IRQ_NMI;
                self.moo_pi = self.p;
            }
            self.irq_pending &= !Self::IRQ_TEMP;
            if self.count <= 0 {
                return true;
            }
        }

        // Generic IRQ (EXT / DPCM / FCOUNT / TEMP / EXT2).
        // Masked by I flag (uses `moo_pi` — the P from before the current
        // instruction, giving SEI/CLI a one-cycle delay).
        let has_irq = self.irq_pending & !(Self::IRQ_NMI | Self::IRQ_NMI2 | Self::IRQ_RESET) != 0;
        if has_irq && self.moo_pi & flags::I_FLAG == 0 && !self.jammed {
            {
                use std::sync::atomic::{AtomicU32, Ordering};
                static COUNT: AtomicU32 = AtomicU32::new(0);
                let n = COUNT.fetch_add(1, Ordering::Relaxed);
                if n < 10 {
                    let mut stderr = std::io::stderr();
                    use std::io::Write as _;
                    let _ = writeln!(
                        stderr,
                        "[irq_service] n={} pc={:04X} irq_pending={:X} I={} count={}",
                        n, self.pc, self.irq_pending,
                        (self.moo_pi & flags::I_FLAG) >> 2, self.count
                    );
                    let _ = stderr.flush();
                }
            }
            self.count -= 7;
            self.push(bus, (self.pc >> 8) as u8);
            self.push(bus, (self.pc & 0xFF) as u8);
            self.push(bus, (self.p & !B_FLAG) | U_FLAG);
            self.p |= flags::I_FLAG;
            self.pc = self.read16(bus, 0xFFFE);
        }

        self.irq_pending &= !Self::IRQ_TEMP;
        if self.count <= 0 {
            return true;
        }
        false
    }
}
