//! CPU module — 6502 interpreter (Phase 0 stub).
//!
//! Phase 0: only `regs.rs` (CpuRegsLayout) is implemented. The interpreter
//! itself lands in Phase 1.

pub mod regs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regs_size_and_align() {
        // Sanity check: layout is locked at 64-byte aligned.
        assert_eq!(std::mem::size_of::<regs::CpuRegsLayout>(), 64);
        assert_eq!(std::mem::align_of::<regs::CpuRegsLayout>(), 64);
    }
}
