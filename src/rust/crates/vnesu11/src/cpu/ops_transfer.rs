//! Transfer instructions: TAX, TXA, TAY, TYA, TSX, TXS.

use super::flags::set_zn;
use super::CpuCore;

impl CpuCore {
    // TAX — A -> X, sets Z/N.
    #[inline(always)]
    pub(crate) fn tax(&mut self) {
        self.x = self.a;
        self.p = set_zn(self.p, self.x);
    }

    // TXA — X -> A, sets Z/N.
    #[inline(always)]
    pub(crate) fn txa(&mut self) {
        self.a = self.x;
        self.p = set_zn(self.p, self.a);
    }

    // TAY — A -> Y, sets Z/N.
    #[inline(always)]
    pub(crate) fn tay(&mut self) {
        self.y = self.a;
        self.p = set_zn(self.p, self.y);
    }

    // TYA — Y -> A, sets Z/N.
    #[inline(always)]
    pub(crate) fn tya(&mut self) {
        self.a = self.y;
        self.p = set_zn(self.p, self.a);
    }

    // TSX — S -> X, sets Z/N.
    #[inline(always)]
    pub(crate) fn tsx(&mut self) {
        self.x = self.s;
        self.p = set_zn(self.p, self.x);
    }

    // TXS — X -> S, no flags.
    #[inline(always)]
    pub(crate) fn txs(&mut self) {
        self.s = self.x;
    }
}
