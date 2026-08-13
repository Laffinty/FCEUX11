//! Stack instructions: PHA, PHP, PLA, PLP, JSR, RTS, RTI, JMP (+BRK in
//! ops_system.rs).

use super::flags::{self, B_FLAG, U_FLAG};
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // PHA — push A.
    #[inline(always)]
    pub(crate) fn pha<BC: BusContext>(&mut self, bus: &mut BC) {
        self.push(bus, self.a);
    }

    // PHP — push P with B and U set (the pushed value has B=1, U=1).
    #[inline(always)]
    pub(crate) fn php<BC: BusContext>(&mut self, bus: &mut BC) {
        let pushed = self.p | B_FLAG | U_FLAG;
        self.push(bus, pushed);
    }

    // PLA — pull A, sets Z/N.
    #[inline(always)]
    pub(crate) fn pla<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.pop(bus);
        self.a = v;
        self.p = flags::set_zn(self.p, v);
    }

    // PLP — pull P (U always set). Like CLI/SEI, this must NOT touch
    // `moo_pi`: the pre-PLP snapshot (whatever I flag was before the
    // pull) is what the next instruction's IRQ sample uses (C++
    // `ops.inc` case 0x28 does not set `_PI`).
    #[inline(always)]
    pub(crate) fn plp<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.pop(bus);
        self.p = v | U_FLAG;
    }

    // JSR — push return-1, jump to absolute.
    #[inline(always)]
    pub(crate) fn jsr<BC: BusContext>(&mut self, bus: &mut BC) {
        let target = self.abs(bus);
        // Return address is PC-1 (already advanced past the operand).
        let ret = self.pc.wrapping_sub(1);
        self.push(bus, (ret >> 8) as u8);
        self.push(bus, (ret & 0xFF) as u8);
        self.pc = target;
    }

    // RTS — pull return, jump to ret+1.
    #[inline(always)]
    pub(crate) fn rts<BC: BusContext>(&mut self, bus: &mut BC) {
        let lo = self.pop(bus) as u16;
        let hi = self.pop(bus) as u16;
        self.pc = ((hi << 8) | lo).wrapping_add(1);
    }

    // RTI — pull P then return.
    #[inline(always)]
    pub(crate) fn rti<BC: BusContext>(&mut self, bus: &mut BC) {
        let p = self.pop(bus);
        self.p = p | U_FLAG;
        self.moo_pi = self.p;
        let lo = self.pop(bus) as u16;
        let hi = self.pop(bus) as u16;
        self.pc = (hi << 8) | lo;
    }

    // JMP absolute.
    #[inline(always)]
    pub(crate) fn jmp_abs<BC: BusContext>(&mut self, bus: &mut BC) {
        self.pc = self.abs(bus);
    }

    // JMP (indirect) — with the 6502 page-wrap bug.
    #[inline(always)]
    pub(crate) fn jmp_ind<BC: BusContext>(&mut self, bus: &mut BC) {
        self.pc = self.indirect(bus);
    }
}
