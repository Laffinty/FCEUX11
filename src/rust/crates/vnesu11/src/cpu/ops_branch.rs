//! Branch instructions: BCC, BCS, BEQ, BMI, BNE, BPL, BVC, BVS.
//!
//! Cycle cost: +1 if branch taken, +1 more if target crosses a page.
//! Implemented by decrementing `count` at the end.

use super::flags::{C_FLAG, N_FLAG, V_FLAG, Z_FLAG};
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    /// Common branch: if `cond`, jump to relative target with penalty.
    #[inline(always)]
    fn branch_if<BC: BusContext>(&mut self, cond: bool, bus: &mut BC) {
        let (target, crossed) = self.relative(bus);
        if cond {
            self.pc = target;
            self.count -= 1; // taken
            if crossed {
                self.count -= 1; // page-cross
            }
        }
    }

    #[inline(always)] pub(crate) fn bcc<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & C_FLAG == 0, bus); }
    #[inline(always)] pub(crate) fn bcs<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & C_FLAG != 0, bus); }
    #[inline(always)] pub(crate) fn beq<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & Z_FLAG != 0, bus); }
    #[inline(always)] pub(crate) fn bmi<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & N_FLAG != 0, bus); }
    #[inline(always)] pub(crate) fn bne<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & Z_FLAG == 0, bus); }
    #[inline(always)] pub(crate) fn bpl<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & N_FLAG == 0, bus); }
    #[inline(always)] pub(crate) fn bvc<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & V_FLAG == 0, bus); }
    #[inline(always)] pub(crate) fn bvs<BC: BusContext>(&mut self, bus: &mut BC) { self.branch_if(self.p & V_FLAG != 0, bus); }
}
