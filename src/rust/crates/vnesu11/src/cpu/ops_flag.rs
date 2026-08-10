//! Flag instructions: CLC, SEC, CLD, SED, CLI, SEI, CLV.

use super::flags::{C_FLAG, D_FLAG, I_FLAG, V_FLAG};
use super::CpuCore;

impl CpuCore {
    #[inline(always)] pub(crate) fn clc(&mut self) { self.p &= !C_FLAG; }
    #[inline(always)] pub(crate) fn sec(&mut self) { self.p |= C_FLAG; }
    #[inline(always)] pub(crate) fn cld(&mut self) { self.p &= !D_FLAG; }
    #[inline(always)] pub(crate) fn sed(&mut self) { self.p |= D_FLAG; }
    #[inline(always)] pub(crate) fn cli(&mut self) {
        self.p &= !I_FLAG;
        self.moo_pi = self.p; // CLI takes effect next instruction
    }
    #[inline(always)] pub(crate) fn sei(&mut self) {
        self.p |= I_FLAG;
        self.moo_pi = self.p; // SEI takes effect next instruction
    }
    #[inline(always)] pub(crate) fn clv(&mut self) { self.p &= !V_FLAG; }
}
