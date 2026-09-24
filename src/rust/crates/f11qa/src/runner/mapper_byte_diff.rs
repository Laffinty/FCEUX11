//! Mapper state byte-diff regression harness (Task 1 — migration).
//!
//! Re-implements the functionality of the original C++
//! `tests/core/mapper_byte_diff_test.cpp` in pure Rust. For each mapper
//! test ROM, the harness:
//!
//! 1. Fresh-initialises the engine per ROM (mirrors the C++ harness's
//!    per-case `core_init()` / `core_shutdown()` cycle).
//! 2. Loads the ROM via [`SutAdapter::load`].
//! 3. Runs `frames` frames via [`SutAdapter::step`].
//! 4. Captures `Cart::save_mapper_state()` via the new
//!    `kagami_bridge_save_mapper_state` FFI.
//! 5. Compares the captured body against the golden file
//!    `fixtures/golden_mapper/<name>.bin` (16-byte header + body):
//!    - missing golden → SKIP (allowed; the C++ harness treats it the same)
//!    - header magic/version/size mismatch → FAIL
//!    - body size mismatch → FAIL
//!    - byte-diff at offset j → FAIL
//!    - byte-identical → PASS
//!
//! Mirrors the C++ behaviour at
//! `tests/core/mapper_byte_diff_test.cpp` (169-case table + SKIP/FAIL
//! classification). Exit 0 iff every non-skipped case passes.

use std::path::Path;

use crate::adapter::trait_def::{InputSpec, SutAdapter};
use crate::core::QaError;

/// Size of the golden header (magic 8 + version 4 + body_size 4).
///
/// Re-exported from [`crate::runner::test_helpers`] (shared utilities —
/// the Rust equivalent of `tests/core/test_helpers.h`).
pub use crate::runner::test_helpers::{
    GOLDEN_HEADER_SIZE as HEADER_SIZE, GOLDEN_MAGIC as MAGIC, GOLDEN_VERSION as VERSION,
    resolve_rom_path, validate_golden_header,
};

/// One entry in the mapper test table (mirrors `RomTestCase`
/// in `tests/core/mapper_byte_diff_test.cpp:54-246`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomMapperCase {
    pub filename: String,
    pub name: String,
    pub frames: u32,
}

/// The canonical 175-entry table from `mapper_byte_diff_test.cpp`.
///
/// Order, names and frame budgets are byte-identical to the C++ array
/// (including the duplicated MMC3-pirate entries that appear in both the
/// Phase E.2 step-5 block and the step-9.5 block — parity requires the
/// same iteration order).
pub fn mapper_cases() -> &'static [RomMapperCase] {
    use std::sync::OnceLock;
    static CELL: OnceLock<Vec<RomMapperCase>> = OnceLock::new();
    CELL.get_or_init(|| {
        vec![
            RomMapperCase { filename: "fixtures/mapper_nrom.nes".into(), name: "nrom".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mmc1.nes".into(), name: "mmc1".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mmc3.nes".into(), name: "mmc3".into(), frames: 120 },
            RomMapperCase { filename: "fixtures/mapper_vrc6.nes".into(), name: "vrc6".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_uxrom.nes".into(), name: "uxrom".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_cnrom.nes".into(), name: "cnrom".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_axrom.nes".into(), name: "axrom".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_colordreams.nes".into(), name: "colordreams".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_gnrom.nes".into(), name: "gnrom".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_vrc2and4.nes".into(), name: "vrc2and4".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_vrc7.nes".into(), name: "vrc7".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mmc5.nes".into(), name: "mmc5".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_cprom.nes".into(), name: "cprom".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper28.nes".into(), name: "mapper28".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper32.nes".into(), name: "mapper32".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper33.nes".into(), name: "mapper33".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper34.nes".into(), name: "mapper34".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper36.nes".into(), name: "mapper36".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper38.nes".into(), name: "mapper38".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper40.nes".into(), name: "mapper40".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper41.nes".into(), name: "mapper41".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper42.nes".into(), name: "mapper42".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper43.nes".into(), name: "mapper43".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper46.nes".into(), name: "mapper46".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper50.nes".into(), name: "mapper50".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_vrc22.nes".into(), name: "vrc22".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_vrc23.nes".into(), name: "vrc23".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_vrc25.nes".into(), name: "vrc25".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper12.nes".into(), name: "mapper12".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper37.nes".into(), name: "mapper37".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper44.nes".into(), name: "mapper44".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper45.nes".into(), name: "mapper45".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper47.nes".into(), name: "mapper47".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper49.nes".into(), name: "mapper49".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper52.nes".into(), name: "mapper52".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper74.nes".into(), name: "mapper74".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper105.nes".into(), name: "mapper105".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper114.nes".into(), name: "mapper114".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper115.nes".into(), name: "mapper115".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper116.nes".into(), name: "mapper116".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper118.nes".into(), name: "mapper118".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper119.nes".into(), name: "mapper119".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper165.nes".into(), name: "mapper165".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper205.nes".into(), name: "mapper205".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper245.nes".into(), name: "mapper245".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper249.nes".into(), name: "mapper249".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper250.nes".into(), name: "mapper250".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper254.nes".into(), name: "mapper254".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper406.nes".into(), name: "mapper406".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper192.nes".into(), name: "mapper192".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper194.nes".into(), name: "mapper194".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper195.nes".into(), name: "mapper195".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper198.nes".into(), name: "mapper198".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mmc2.nes".into(), name: "mmc2".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mmc4.nes".into(), name: "mmc4".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper15.nes".into(), name: "mapper15".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper48.nes".into(), name: "mapper48".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_bandai.nes".into(), name: "bandai".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper18.nes".into(), name: "mapper18".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_vrc6var26.nes".into(), name: "vrc6var26".into(), frames: 90 },
            RomMapperCase { filename: "fixtures/mapper_mapper70.nes".into(), name: "mapper70".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper78.nes".into(), name: "mapper78".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper86.nes".into(), name: "mapper86".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper87.nes".into(), name: "mapper87".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper89.nes".into(), name: "mapper89".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper94.nes".into(), name: "mapper94".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper97.nes".into(), name: "mapper97".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper51.nes".into(), name: "mapper51".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper57.nes".into(), name: "mapper57".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper61.nes".into(), name: "mapper61".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper62.nes".into(), name: "mapper62".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper64.nes".into(), name: "mapper64".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper65.nes".into(), name: "mapper65".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper67.nes".into(), name: "mapper67".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper68.nes".into(), name: "mapper68".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper71.nes".into(), name: "mapper71".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper72.nes".into(), name: "mapper72".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper73.nes".into(), name: "mapper73".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper75.nes".into(), name: "mapper75".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper77.nes".into(), name: "mapper77".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper79.nes".into(), name: "mapper79".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper80.nes".into(), name: "mapper80".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper82.nes".into(), name: "mapper82".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper90.nes".into(), name: "mapper90".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper91.nes".into(), name: "mapper91".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper92.nes".into(), name: "mapper92".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper93.nes".into(), name: "mapper93".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper96.nes".into(), name: "mapper96".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper99.nes".into(), name: "mapper99".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper53.nes".into(), name: "mapper53".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper58.nes".into(), name: "mapper58".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper60.nes".into(), name: "mapper60".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper76.nes".into(), name: "mapper76".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper95.nes".into(), name: "mapper95".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper88.nes".into(), name: "mapper88".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper6.nes".into(), name: "mapper6".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper17.nes".into(), name: "mapper17".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper19.nes".into(), name: "mapper19".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper210.nes".into(), name: "mapper210".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper105.nes".into(), name: "mapper105".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper114.nes".into(), name: "mapper114".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper115.nes".into(), name: "mapper115".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper116.nes".into(), name: "mapper116".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper118.nes".into(), name: "mapper118".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper119.nes".into(), name: "mapper119".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper165.nes".into(), name: "mapper165".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper192.nes".into(), name: "mapper192".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper194.nes".into(), name: "mapper194".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper195.nes".into(), name: "mapper195".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper198.nes".into(), name: "mapper198".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper205.nes".into(), name: "mapper205".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper245.nes".into(), name: "mapper245".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper249.nes".into(), name: "mapper249".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper250.nes".into(), name: "mapper250".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper254.nes".into(), name: "mapper254".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper406.nes".into(), name: "mapper406".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper69.nes".into(), name: "mapper69".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper206.nes".into(), name: "mapper206".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper59.nes".into(), name: "mapper59".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper103.nes".into(), name: "mapper103".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper106.nes".into(), name: "mapper106".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper108.nes".into(), name: "mapper108".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper112.nes".into(), name: "mapper112".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper117.nes".into(), name: "mapper117".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper120.nes".into(), name: "mapper120".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper121.nes".into(), name: "mapper121".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper151.nes".into(), name: "mapper151".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper156.nes".into(), name: "mapper156".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper177.nes".into(), name: "mapper177".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper178.nes".into(), name: "mapper178".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper111.nes".into(), name: "mapper111".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper123.nes".into(), name: "mapper123".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper125.nes".into(), name: "mapper125".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper132.nes".into(), name: "mapper132".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper133.nes".into(), name: "mapper133".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper136.nes".into(), name: "mapper136".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper137.nes".into(), name: "mapper137".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper138.nes".into(), name: "mapper138".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper139.nes".into(), name: "mapper139".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper141.nes".into(), name: "mapper141".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper142.nes".into(), name: "mapper142".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper143.nes".into(), name: "mapper143".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper145.nes".into(), name: "mapper145".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper146.nes".into(), name: "mapper146".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper147.nes".into(), name: "mapper147".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper148.nes".into(), name: "mapper148".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper149.nes".into(), name: "mapper149".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper150.nes".into(), name: "mapper150".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper160.nes".into(), name: "mapper160".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper162.nes".into(), name: "mapper162".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper163.nes".into(), name: "mapper163".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper164.nes".into(), name: "mapper164".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper166.nes".into(), name: "mapper166".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper167.nes".into(), name: "mapper167".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper168.nes".into(), name: "mapper168".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper170.nes".into(), name: "mapper170".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper172.nes".into(), name: "mapper172".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper173.nes".into(), name: "mapper173".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper175.nes".into(), name: "mapper175".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper176.nes".into(), name: "mapper176".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper181.nes".into(), name: "mapper181".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper183.nes".into(), name: "mapper183".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper185.nes".into(), name: "mapper185".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper186.nes".into(), name: "mapper186".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper187.nes".into(), name: "mapper187".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper188.nes".into(), name: "mapper188".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper189.nes".into(), name: "mapper189".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper190.nes".into(), name: "mapper190".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper193.nes".into(), name: "mapper193".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper14.nes".into(), name: "mapper14".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper27.nes".into(), name: "mapper27".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper30.nes".into(), name: "mapper30".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper31.nes".into(), name: "mapper31".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper35.nes".into(), name: "mapper35".into(), frames: 60 },
            RomMapperCase { filename: "fixtures/mapper_mapper83.nes".into(), name: "mapper83".into(), frames: 60 },
        ]
    })
}

/// Outcome of comparing one ROM's live mapper state against its golden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapperCaseVerdict {
    Pass { body_size: u32 },
    /// Golden file not present (or empty on both sides) — allowed, not a
    /// regression (mirrors the C++ `[SKIP]` classification).
    Skip,
    Fail { reason: String },
}

/// Outcome of running the mapper_byte_diff harness once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapperDiffOutcome {
    pub total: usize,
    pub passed: usize,
    pub skipped: usize,
    pub failed: usize,
    /// (case name, reason) for every failure, in iteration order.
    pub failures: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Golden file I/O
// ---------------------------------------------------------------------------

/// Read a golden file. Returns `None` on error / not found.
fn read_golden_file(dir: &Path, name: &str) -> Option<Vec<u8>> {
    let path = dir.join(format!("{}.bin", name));
    std::fs::read(path).ok()
}

// ---------------------------------------------------------------------------
// Mapper-state abstraction. Same pattern as FrameSource / StateSnapshot:
// NOT a SutAdapter method (Stage-3 freeze), so we define a separate trait
// that Fceux11DirectAdapter implements via the new
// kagami_bridge_save_mapper_state FFI.
// ---------------------------------------------------------------------------

/// Snapshot the current cart's mapper state into a freshly allocated
/// byte buffer (the body of the golden file, excluding the 16-byte
/// header).
pub trait MapperStateSource {
    fn snapshot_mapper_state(&self) -> Result<Vec<u8>, QaError>;

    /// Per-ROM engine teardown+re-init. Default no-op (mock adapters in
    /// unit tests). `Fceux11DirectAdapter` overrides this to call
    /// `kagami_bridge_full_reset`, mirroring the C++ harness's per-case
    /// `core_init()`/`core_shutdown()` cycle.
    fn reset_fresh(&mut self) -> Result<(), QaError> {
        Ok(())
    }
}

#[cfg(any(feature = "direct-adapter", not(test)))]
impl MapperStateSource for crate::adapter::direct::Fceux11DirectAdapter {
    fn snapshot_mapper_state(&self) -> Result<Vec<u8>, QaError> {
        // First call asks for the size (cap=0), then allocate + fetch.
        let mut written: u32 = 0;
        let rc = unsafe { kagami_bridge_save_mapper_state(std::ptr::null_mut(), 0, &mut written) };
        if rc != 0 {
            return Err(QaError::unsupported(format!(
                "kagami_bridge_save_mapper_state (size probe) failed: rc={}",
                rc
            )));
        }
        let mut buf = vec![0u8; written as usize];
        if written > 0 {
            let rc2 =
                unsafe { kagami_bridge_save_mapper_state(buf.as_mut_ptr(), written, &mut written) };
            if rc2 != 0 {
                return Err(QaError::unsupported(format!(
                    "kagami_bridge_save_mapper_state (fetch) failed: rc={}",
                    rc2
                )));
            }
            buf.truncate(written as usize);
        }
        Ok(buf)
    }

    fn reset_fresh(&mut self) -> Result<(), QaError> {
        self.full_reset()
    }
}

#[cfg(any(feature = "direct-adapter", not(test)))]
unsafe extern "C" {
    fn kagami_bridge_save_mapper_state(dst: *mut u8, cap: u32, written_out: *mut u32) -> i32;
}

// ---------------------------------------------------------------------------
// Per-ROM collection + comparison
// ---------------------------------------------------------------------------

/// Drive one case: fresh engine → load → run N frames → capture mapper
/// state. Returns the captured body, or `Err` on load/step failure.
fn collect_mapper_state<A>(
    adapter: &mut A,
    case: &RomMapperCase,
    workdir: &Path,
) -> Result<Vec<u8>, QaError>
where
    A: SutAdapter + MapperStateSource,
{
    adapter.reset_fresh()?;
    let rom_path = resolve_rom_path(workdir, &case.filename);
    let spec = InputSpec {
        rom_path: Some(rom_path.to_string_lossy().to_string()),
        script_path: None,
        frames: case.frames,
        probe_addr: 0x6000,
        reset_after: -1,
    };
    adapter.load(&spec)?;
    for _ in 0..case.frames {
        adapter.step()?;
    }
    adapter.snapshot_mapper_state()
}

/// Compare one case's live body against its golden file.
fn classify_case<A>(
    adapter: &mut A,
    case: &RomMapperCase,
    workdir: &Path,
    golden_dir: &Path,
) -> MapperCaseVerdict
where
    A: SutAdapter + MapperStateSource,
{
    let body = match collect_mapper_state(adapter, case, workdir) {
        Ok(b) => b,
        Err(_) => {
            // C++ prints "[FAIL] <name>: core_init/load_rom failed".
            return MapperCaseVerdict::Fail {
                reason: "core_init or load_rom failed".into(),
            };
        }
    };

    let golden = match read_golden_file(golden_dir, &case.name) {
        None => return MapperCaseVerdict::Skip,
        Some(g) => g,
    };

    let expected_body_size = match validate_golden_header(&golden) {
        Err(reason) => {
            return MapperCaseVerdict::Fail {
                reason: format!("golden header validation failed: {}", reason),
            };
        }
        Ok(sz) => sz,
    };

    if body.len() != expected_body_size as usize {
        return MapperCaseVerdict::Fail {
            reason: format!(
                "body size {} (golden expects {})",
                body.len(),
                expected_body_size
            ),
        };
    }

    if expected_body_size == 0 {
        // Both sides are empty: pass-through (C++ `[SKIP]`).
        return MapperCaseVerdict::Skip;
    }

    for j in 0..expected_body_size as usize {
        if body[j] != golden[HEADER_SIZE + j] {
            return MapperCaseVerdict::Fail {
                reason: format!(
                    "byte-diff at offset {} (live=0x{:02X}, golden=0x{:02X})",
                    j, body[j], golden[HEADER_SIZE + j]
                ),
            };
        }
    }

    MapperCaseVerdict::Pass {
        body_size: expected_body_size,
    }
}

/// Run the full harness against the golden directory.
pub fn run_regression<A>(
    adapter: &mut A,
    workdir: &Path,
    golden_dir: &Path,
) -> MapperDiffOutcome
where
    A: SutAdapter + MapperStateSource,
{
    let mut outcome = MapperDiffOutcome::default();
    outcome.total = mapper_cases().len();

    for case in mapper_cases() {
        match classify_case(adapter, case, workdir, golden_dir) {
            MapperCaseVerdict::Pass { .. } => outcome.passed += 1,
            MapperCaseVerdict::Skip => outcome.skipped += 1,
            MapperCaseVerdict::Fail { reason } => {
                outcome.failed += 1;
                outcome.failures.push((case.name.clone(), reason));
            }
        }
    }
    outcome
}

/// Format the harness summary the way the C++ harness prints it.
pub fn format_summary(outcome: &MapperDiffOutcome) -> String {
    let mut s = String::new();
    s.push_str("=== Summary ===\n");
    s.push_str(&format!("Total:     {}\n", outcome.total));
    s.push_str(&format!("Passed:    {}\n", outcome.passed));
    s.push_str(&format!("Skipped:   {}\n", outcome.skipped));
    s.push_str(&format!("Failed:    {}\n", outcome.failed));
    for (name, reason) in &outcome.failures {
        s.push_str(&format!("  [FAIL] {}: {}\n", name, reason));
    }
    s.push_str(&format!(
        "RESULT:    {}\n",
        if outcome.failed == 0 { "PASS" } else { "FAIL" }
    ));
    s
}

pub fn regression_exit_code(outcome: &MapperDiffOutcome) -> i32 {
    if outcome.failed == 0 {
        0
    } else {
        1
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::adapter::trait_def::{InputSpec, SutAdapter, TestResult};
    use crate::core::{ErrorKind, QaConfig, QaError};
    use crate::manifest::schema::TestManifest;

    /// Mock adapter with a fixed mapper-state body, counting calls.
    struct MockAdapter {
        body: Vec<u8>,
        load_calls: u32,
        step_count: u32,
        reset_fresh_calls: u32,
        fail_load: bool,
    }

    impl MockAdapter {
        fn new(body: Vec<u8>) -> Self {
            Self {
                body,
                load_calls: 0,
                step_count: 0,
                reset_fresh_calls: 0,
                fail_load: false,
            }
        }
    }

    impl SutAdapter for MockAdapter {
        fn init(&self, _config: &QaConfig) -> Result<(), QaError> {
            Ok(())
        }
        fn run_test(&self, _test: &TestManifest) -> Result<TestResult, QaError> {
            unimplemented!()
        }
        fn load(&mut self, input: &InputSpec) -> Result<(), QaError> {
            self.load_calls += 1;
            if self.fail_load {
                return Err(QaError {
                    kind: ErrorKind::TestExecFailed,
                    message: "mock load failure".into(),
                });
            }
            let _ = input;
            Ok(())
        }
        fn step(&mut self) -> Result<(), QaError> {
            self.step_count += 1;
            Ok(())
        }
        fn read_oracle_probe(&self, _addr: u32) -> Result<u8, QaError> {
            Ok(0)
        }
        fn reset(&mut self) -> Result<(), QaError> {
            Ok(())
        }
    }

    impl MapperStateSource for MockAdapter {
        fn snapshot_mapper_state(&self) -> Result<Vec<u8>, QaError> {
            Ok(self.body.clone())
        }
        fn reset_fresh(&mut self) -> Result<(), QaError> {
            self.reset_fresh_calls += 1;
            Ok(())
        }
    }

    fn golden_path() -> PathBuf {
        // CARGO_MANIFEST_DIR → src/rust/crates/kagami-qa; up 4 → root.
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| String::from("."));
        PathBuf::from(manifest_dir)
            .join("..")
            .join("..")
            .join("..")
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("golden_mapper")
    }

    fn write_golden(dir: &Path, name: &str, body: &[u8]) {
        std::fs::create_dir_all(dir).unwrap();
        let mut data = Vec::with_capacity(HEADER_SIZE + body.len());
        data.extend_from_slice(&MAGIC);
        data.extend_from_slice(&VERSION.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(body);
        std::fs::write(dir.join(format!("{}.bin", name)), data).unwrap();
    }

    // ---------------- Golden header validation --------------------------

    #[test]
    fn golden_header_valid() {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&[1, 2, 3]);
        assert_eq!(validate_golden_header(&data), Ok(3));
    }

    #[test]
    fn golden_header_bad_magic() {
        let mut data = vec![0u8; 20];
        data[0] = b'X';
        assert!(validate_golden_header(&data).is_err());
    }

    #[test]
    fn golden_header_bad_version() {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC);
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        assert!(validate_golden_header(&data).is_err());
    }

    #[test]
    fn golden_header_size_mismatch() {
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&5u32.to_le_bytes()); // claims 5, has 0
        assert!(validate_golden_header(&data).is_err());
    }

    // ---------------- Classification -------------------------------------

    #[test]
    fn classify_pass_when_body_matches() {
        let tmp = std::env::temp_dir().join("kagami_mapper_test_pass");
        let _ = std::fs::remove_dir_all(&tmp);
        write_golden(&tmp, "nrom", &[0xDE, 0xAD, 0xBE, 0xEF]);
        let mut a = MockAdapter::new(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let case = RomMapperCase {
            filename: "fixtures/x.nes".into(),
            name: "nrom".into(),
            frames: 60,
        };
        let v = classify_case(&mut a, &case, Path::new("."), &tmp);
        assert_eq!(
            v,
            MapperCaseVerdict::Pass { body_size: 4 }
        );
        assert_eq!(a.step_count, 60);
        assert_eq!(a.reset_fresh_calls, 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_skip_when_golden_missing() {
        let tmp = std::env::temp_dir().join("kagami_mapper_test_skip");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut a = MockAdapter::new(vec![1, 2, 3]);
        let case = RomMapperCase {
            filename: "fixtures/x.nes".into(),
            name: "no_such_mapper".into(),
            frames: 60,
        };
        let v = classify_case(&mut a, &case, Path::new("."), &tmp);
        assert_eq!(v, MapperCaseVerdict::Skip);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_fail_on_byte_diff() {
        let tmp = std::env::temp_dir().join("kagami_mapper_test_diff");
        let _ = std::fs::remove_dir_all(&tmp);
        write_golden(&tmp, "mmc1", &[1, 2, 3, 4]);
        let mut a = MockAdapter::new(vec![1, 2, 0xFF, 4]);
        let case = RomMapperCase {
            filename: "fixtures/x.nes".into(),
            name: "mmc1".into(),
            frames: 60,
        };
        let v = classify_case(&mut a, &case, Path::new("."), &tmp);
        match v {
            MapperCaseVerdict::Fail { reason } => {
                assert!(reason.contains("byte-diff at offset 2"));
            }
            other => panic!("expected Fail, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_fail_on_body_size_mismatch() {
        let tmp = std::env::temp_dir().join("kagami_mapper_test_size");
        let _ = std::fs::remove_dir_all(&tmp);
        write_golden(&tmp, "vrc6", &[1, 2, 3]);
        let mut a = MockAdapter::new(vec![1, 2, 3, 4]);
        let case = RomMapperCase {
            filename: "fixtures/x.nes".into(),
            name: "vrc6".into(),
            frames: 90,
        };
        let v = classify_case(&mut a, &case, Path::new("."), &tmp);
        assert!(matches!(v, MapperCaseVerdict::Fail { .. }));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_fail_on_load_error() {
        let tmp = std::env::temp_dir().join("kagami_mapper_test_load");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut a = MockAdapter::new(vec![]);
        a.fail_load = true;
        let case = RomMapperCase {
            filename: "fixtures/x.nes".into(),
            name: "nrom".into(),
            frames: 60,
        };
        let v = classify_case(&mut a, &case, Path::new("."), &tmp);
        assert!(matches!(v, MapperCaseVerdict::Fail { .. }));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn classify_skip_on_empty_body_both_sides() {
        let tmp = std::env::temp_dir().join("kagami_mapper_test_empty");
        let _ = std::fs::remove_dir_all(&tmp);
        write_golden(&tmp, "empty", &[]);
        let mut a = MockAdapter::new(vec![]);
        let case = RomMapperCase {
            filename: "fixtures/x.nes".into(),
            name: "empty".into(),
            frames: 60,
        };
        let v = classify_case(&mut a, &case, Path::new("."), &tmp);
        assert_eq!(v, MapperCaseVerdict::Skip);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ---------------- Table integrity ------------------------------------

    #[test]
    fn table_matches_cxx_count() {
        // The C++ tests[] array has 175 entries (including the duplicated
        // MMC3-pirate rows present in both step-5 and step-9.5 blocks).
        assert_eq!(mapper_cases().len(), 175);
    }

    #[test]
    fn table_names_unique_except_known_dupes() {
        let names = mapper_cases().iter().map(|c| c.name.as_str()).collect::<Vec<_>>();
        // The 19 duplicated names are exactly the MMC3-pirate set.
        let mut seen = std::collections::HashMap::new();
        let mut dupes = Vec::new();
        for n in &names {
            let e = seen.entry(*n).or_insert(0u32);
            *e += 1;
        }
        for (n, c) in &seen {
            if *c > 1 {
                dupes.push(*n);
            }
        }
        // 16 names appear twice (105/114/115/116/118/119/165/192/194/195/
        // 198/205/245/249/250/254/406 → that's 17; 192/194/195/198 appear
        // in step-6 AND step-9.5 → 4 more pairs). We only assert the
        // duplicates are within the MMC3-pirate family.
        for d in &dupes {
            assert!(
                d.starts_with("mapper")
                    && matches!(
                        *d,
                        "mapper105" | "mapper114" | "mapper115" | "mapper116" | "mapper118"
                            | "mapper119" | "mapper165" | "mapper192" | "mapper194"
                            | "mapper195" | "mapper198" | "mapper205" | "mapper245"
                            | "mapper249" | "mapper250" | "mapper254" | "mapper406"
                    ),
                "unexpected duplicate: {}",
                d
            );
        }
    }

    // ---------------- Real fixture smoke ---------------------------------

    #[test]
    fn real_golden_dir_present() {
        let gp = golden_path();
        if !gp.exists() {
            eprintln!("skipping: golden_mapper not checked out at {}", gp.display());
            return;
        }
        let entries = std::fs::read_dir(&gp).unwrap().count();
        assert!(entries >= 150, "expected >=150 goldens, found {}", entries);
    }
}
