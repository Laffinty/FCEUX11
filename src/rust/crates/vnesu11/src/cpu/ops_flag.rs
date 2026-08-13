//! Flag instructions: CLC, SEC, CLD, SED, CLI, SEI, CLV.

use super::flags::{C_FLAG, D_FLAG, I_FLAG, V_FLAG};
use super::CpuCore;

impl CpuCore {
    #[inline(always)] pub(crate) fn clc(&mut self) { self.p &= !C_FLAG; }
    #[inline(always)] pub(crate) fn sec(&mut self) { self.p |= C_FLAG; }
    #[inline(always)] pub(crate) fn cld(&mut self) { self.p &= !D_FLAG; }
    #[inline(always)] pub(crate) fn sed(&mut self) { self.p |= D_FLAG; }
    #[inline(always)] pub(crate) fn cli(&mut self) {
        // Do NOT touch `moo_pi` here. `step_one`/`step_inner` snapshot
        // `moo_pi = self.p` BEFORE this instruction, and that pre-CLI
        // value (I=1) is what the NEXT instruction's IRQ sample must
        // see — that is the 6502 one-instruction CLI latency. Setting
        // moo_pi here overwrote it with the post-CLI I=0 and serviced
        // the IRQ one instruction too early (blargg cpu_int_1_cli_latency
        // test 4). C++ `ops.inc` case 0x58 does not touch `_PI`.
        self.p &= !I_FLAG;
    }
    #[inline(always)] pub(crate) fn sei(&mut self) {
        // Same as CLI: the pre-SEI `moo_pi` snapshot must survive so
        // the instruction after SEI can still be sampled (C++ case 0x78
        // also leaves `_PI` alone; the next loop sets `_PI = _P`).
        self.p |= I_FLAG;
    }
    #[inline(always)] pub(crate) fn clv(&mut self) { self.p &= !V_FLAG; }
}
