// KagamiQA — FCEUX11 in-process (direct) adapter via C ABI FFI.
//
// Implements SutAdapter by calling the kagami_bridge C ABI functions
// exported from src/kagami_bridge.cpp.  This gives the Rust runner
// frame-by-frame control over the emulator — the fundamental requirement
// for P5 (runppu 重批) where per-frame oracle probes are needed.
//
// Unlike SubprocessAdapter (which shells out to pre-built CTest binaries),
// this adapter links directly against fceux11_core and drives the null
// driver headless loop without spawning a subprocess.

use std::ffi::CString;
use std::os::raw::c_char;

use crate::adapter::trait_def::{InputSpec, SutAdapter};
use crate::core::{QaConfig, QaError};

// ---------------------------------------------------------------------------
// C ABI FFI declarations — link against kagami_bridge.cpp
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn kagami_bridge_init() -> i32;
    fn kagami_bridge_load_rom(path: *const c_char) -> i32;
    fn kagami_bridge_emulate_frame() -> i32;
    fn kagami_bridge_read_byte(addr: u16) -> u8;
    fn kagami_bridge_reset() -> i32;
    fn kagami_bridge_full_reset() -> i32;
    fn kagami_bridge_kill();
    fn kagami_bridge_set_newppu(on: i32);
    // NOTE: `kagami_bridge_read_ppu` / `kagami_bridge_extract_frame_buffer`
    // / `kagami_bridge_save_state` are declared in the runner modules that
    // actually call them (rom_regression, savestate_regression, etc.) —
    // the adapter itself only needs the seven core primitives above. The
    // kagami_bridge FFI symbols are resolved once at link time, so the
    // duplicate `unsafe extern "C"` blocks must not both declare the same
    // symbol with a different signature (linker error).
}

// ---------------------------------------------------------------------------
// Fceux11DirectAdapter
// ---------------------------------------------------------------------------
pub struct Fceux11DirectAdapter {
    initialized: bool,
    rom_loaded: bool,
    /// PPU implementation to use. C++ blargg_runner sets newppu=1; C++
    /// rom_regression_test and savestate_regression_test do NOT (they use
    /// the legacy PPU). Default false to match the C-2/C-3 goldens; the
    /// blargg harness opts into newppu explicitly.
    use_newppu: bool,
}

impl Fceux11DirectAdapter {
    pub fn new() -> Self {
        Self {
            initialized: false,
            rom_loaded: false,
            use_newppu: false,
        }
    }

    /// Opt into the new PPU (C++ blargg_runner parity: newppu=1).
    pub fn with_newppu(mut self) -> Self {
        self.use_newppu = true;
        self
    }

    /// Full teardown + re-init (no ROM loaded afterwards). Mirrors the C++
    /// savestate harness's per-ROM Initialize/Kill cycle; used by the C-3
    /// harness between ROMs so each one starts from a pristine engine.
    pub fn full_reset(&mut self) -> Result<(), QaError> {
        let rc = unsafe { kagami_bridge_full_reset() };
        if rc != 0 {
            return Err(QaError::unsupported(format!(
                "kagami_bridge_full_reset failed: rc={}", rc
            )));
        }
        self.initialized = true; // bridge re-initialised
        self.rom_loaded = false;
        Ok(())
    }
}

impl Drop for Fceux11DirectAdapter {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { kagami_bridge_kill(); }
        }
    }
}

impl SutAdapter for Fceux11DirectAdapter {
    // ------------------------------------------------------------------
    // Subprocess-mode: unused for this adapter; not supported.
    // ------------------------------------------------------------------
    fn init(&self, _config: &QaConfig) -> Result<(), QaError> {
        Err(QaError::unsupported(
            "Fceux11DirectAdapter: use load()/step() interface, not init()+run_test()"
        ))
    }

    fn run_test(
        &self,
        _test: &crate::manifest::schema::TestManifest,
    ) -> Result<crate::adapter::trait_def::TestResult, QaError> {
        Err(QaError::unsupported(
            "Fceux11DirectAdapter: use load()/step()/read_oracle_probe() instead of run_test()"
        ))
    }

    // ------------------------------------------------------------------
    // In-process interface
    // ------------------------------------------------------------------
    fn load(&mut self, input: &InputSpec) -> Result<(), QaError> {
        // Lazy initialise on first load.
        if !self.initialized {
            let rc = unsafe { kagami_bridge_init() };
            if rc != 0 {
                return Err(QaError::unsupported(format!(
                    "kagami_bridge_init failed: rc={}", rc
                )));
            }
            // PPU implementation matches the C++ harness that produced the
            // golden baseline (blargg → new PPU; regression/savestate → legacy).
            unsafe { kagami_bridge_set_newppu(if self.use_newppu { 1 } else { 0 }); }
            self.initialized = true;
        }

        if let Some(ref rom) = input.rom_path {
            let c_path = CString::new(rom.as_str())
                .map_err(|e| QaError::unsupported(format!("invalid ROM path: {}", e)))?;
            let rc = unsafe { kagami_bridge_load_rom(c_path.as_ptr()) };
            if rc != 0 {
                return Err(QaError::unsupported(format!(
                    "kagami_bridge_load_rom('{}') failed: rc={}", rom, rc
                )));
            }
            self.rom_loaded = true;
        }

        Ok(())
    }

    fn step(&mut self) -> Result<(), QaError> {
        if !self.rom_loaded {
            return Err(QaError::unsupported("step: no ROM loaded"));
        }
        let rc = unsafe { kagami_bridge_emulate_frame() };
        if rc != 0 {
            return Err(QaError::unsupported(format!(
                "kagami_bridge_emulate_frame failed: rc={}", rc
            )));
        }
        Ok(())
    }

    fn read_oracle_probe(&self, addr: u32) -> Result<u8, QaError> {
        if addr > 0xFFFF {
            return Err(QaError::unsupported(format!(
                "read_oracle_probe: address 0x{:X} out of range", addr
            )));
        }
        Ok(unsafe { kagami_bridge_read_byte(addr as u16) })
    }

    fn snapshot(&self) -> Result<Vec<u8>, QaError> {
        // Read the full $6000–$6003 range as a minimal snapshot.
        let mut buf = Vec::with_capacity(4);
        for offset in 0..4u32 {
            buf.push(unsafe { kagami_bridge_read_byte(0x6000 + offset as u16) });
        }
        Ok(buf)
    }

    fn reset(&mut self) -> Result<(), QaError> {
        let rc = unsafe { kagami_bridge_reset() };
        if rc != 0 {
            return Err(QaError::unsupported(format!(
                "kagami_bridge_reset failed: rc={}", rc
            )));
        }
        // Soft reset: the ROM stays loaded in the bridge (kagami_bridge_reset
        // now calls fceu11::ResetNES, not CloseGame), so keep rom_loaded true
        // and allow frame-stepping to continue. Task 1 parity fix.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_starts_uninitialized() {
        let adapter = Fceux11DirectAdapter::new();
        assert!(!adapter.initialized);
        assert!(!adapter.rom_loaded);
    }

    /// step() calls FFI → needs C++ core linked.  Only run with ffi-stubs
    /// or in the full cmake build (not plain `cargo test`).
    #[test]
    #[cfg(feature = "ffi-stubs")]
    fn step_without_rom_returns_error() {
        let mut adapter = Fceux11DirectAdapter::new();
        let result = adapter.step();
        assert!(result.is_err());
    }
}
