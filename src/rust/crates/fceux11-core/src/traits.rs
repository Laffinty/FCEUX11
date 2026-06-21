//! Core abstraction traits for the FCEUX11 emulator.
//!
//! These traits define the clock/bus/interrupt interfaces that v0.3.x
//! will use to gradually migrate CPU, PPU, APU and Mapper logic from
//! C++ into Rust.  They are intentionally minimal — only the surface
//! area required to wire components together inside a `System` struct.
//!
//! # Design principles
//! 1. **No distributed shared state** — `System` owns all components;
//!    cross-component access happens through `&mut self` on `System`.
//! 2. **Cycle-accurate by default** — `tick()` advances exactly one
//!    master clock cycle.
//! 3. **No `unsafe` in trait definitions** — unsafe is relegated to
//!    FFI shims and raw-memory buffer operations.

/// 6502 CPU interface.
///
/// The C++ `X6502` struct is the current authoritative implementation.
/// This trait sketches the boundary at which a Rust-first CPU core
/// could be dropped in during v0.3.x without changing the outside
/// world.
pub trait Cpu {
    /// Reset vector and internal state.
    fn reset(&mut self);

    /// Power-on initialization (distinct from reset on NES).
    fn power(&mut self);

    /// Execute until the local cycle counter reaches `target_cycles`.
    /// Returns the number of cycles actually consumed (may be less if
    /// an IRQ/NMI was triggered early).
    fn run_to(&mut self, target_cycles: u32) -> u32;

    /// Read the 8-bit data bus at address `addr`.
    /// In a full Rust core this would be mediated by `Bus`.
    fn read_u8(&mut self, addr: u16) -> u8;

    /// Write `val` to the data bus at address `addr`.
    fn write_u8(&mut self, addr: u16, val: u8);

    /// Assert an IRQ source.  Multiple sources are ORed together.
    fn irq_begin(&mut self, source: IrqSource);

    /// De-assert an IRQ source.
    fn irq_end(&mut self, source: IrqSource);

    /// Trigger an NMI (edge-sensitive).
    fn trigger_nmi(&mut self);

    /// Fill `out` with the current register snapshot (for debugger / savestate).
    fn registers(&self) -> CpuRegs;

    /// Restore registers from a snapshot.
    fn set_registers(&mut self, regs: &CpuRegs);
}

/// CPU register snapshot.
/// Matches the layout of C++ `X6502` (see `src/x6502struct.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CpuRegs {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub s: u8,
    pub p: u8,
    pub db: u8, // data bus "cache"
    pub irq_low: u32,
    pub tcount: i32,
    pub count: i32,
}

/// IRQ source bitmask values (match C++ `FCEU_IQ*` constants).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum IrqSource {
    External = 0x001,
    External2 = 0x002,
    Reset = 0x020,
    Nmi = 0x080,
    Dpcm = 0x100,
    FrameCounter = 0x200,
    Temp = 0x800,
}

/// Ricoh 2C02 (PPU) interface.
///
/// The PPU is the most timing-sensitive component.  This trait is
/// deliberately high-level — it exposes scanline/dot queries and
/// the frame buffer, but does not prescribe the internal rendering
/// pipeline.
pub trait Ppu {
    /// Reset state (warm boot).
    fn reset(&mut self);

    /// Power-on initialization.
    fn power(&mut self);

    /// Advance by one PPU clock (3 PPU cycles = 1 CPU cycle on NTSC).
    fn tick(&mut self, cpu_cycle: bool) -> PpuTickResult;

    /// Current scanline (-1 .. 261 for NTSC).
    fn scanline(&self) -> i32;

    /// Current dot within the scanline (0 .. 340).
    fn dot(&self) -> i32;

    /// Read from PPU register port at CPU address $2000-$2007.
    fn read_reg(&mut self, reg: u8) -> u8;

    /// Write to PPU register port at CPU address $2000-$2007.
    fn write_reg(&mut self, reg: u8, val: u8);

    /// Fill `out` with the current NTSC frame buffer (256×240 RGB).
    /// `out` must be at least 256×240×4 bytes (RGBA).
    fn frame_buffer_rgba(&self, out: &mut [u8]);

    /// Save-state snapshot (internal registers only; CHR/RAM is saved
    /// by the cart subsystem).
    fn snapshot(&self) -> PpuSnapshot;
    fn restore_snapshot(&mut self, snap: &PpuSnapshot);
}

/// Result of a single PPU tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PpuTickResult {
    /// True if the frame buffer is ready for output (end of frame).
    pub frame_ready: bool,
    /// True if an NMI should be asserted this cycle.
    pub nmi: bool,
    /// True if the PPU is in VBlank.
    pub in_vblank: bool,
}

/// Serializable PPU state (for savestates).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PpuSnapshot {
    pub ctrl: u8,        // $2000
    pub mask: u8,        // $2001
    pub status: u8,      // $2002
    pub oam_addr: u8,    // $2003
    pub scroll: [u8; 2], // $2005
    pub addr: [u8; 2],   // $2006
    pub v: u16,          // current VRAM address
    pub t: u16,          // temporary VRAM address
    pub x: u8,           // fine X scroll
    pub w: bool,         // write toggle
}

/// Ricoh 2A03 APU / DMC interface.
pub trait Apu {
    /// Reset.
    fn reset(&mut self);

    /// Power-on initialization.
    fn power(&mut self);

    /// Advance by one CPU cycle, generating audio samples.
    fn tick(&mut self, irq_out: &mut bool);

    /// Read from APU status register ($4015).
    fn read_status(&self) -> u8;

    /// Write to any APU register ($4000-$4017).
    fn write_reg(&mut self, reg: u8, val: u8);

    /// Fill `out` with the next `count` stereo samples (interleaved i16).
    fn fill_samples(&mut self, out: &mut [i16], count: usize);

    /// Save-state snapshot.
    fn snapshot(&self) -> ApuSnapshot;
    fn restore_snapshot(&mut self, snap: &ApuSnapshot);
}

/// Serializable APU state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ApuSnapshot {
    pub pulse1: PulseChannelState,
    pub pulse2: PulseChannelState,
    pub triangle: TriangleChannelState,
    pub noise: NoiseChannelState,
    pub dmc: DmcChannelState,
    pub frame_counter: u8,
    pub frame_irq: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PulseChannelState {
    pub duty: u8,
    pub volume: u8,
    pub timer: u16,
    pub length_counter: u8,
    pub envelope: u8,
    pub sweep: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TriangleChannelState {
    pub timer: u16,
    pub length_counter: u8,
    pub linear_counter: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NoiseChannelState {
    pub volume: u8,
    pub timer: u16,
    pub length_counter: u8,
    pub envelope: u8,
    pub shift_reg: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DmcChannelState {
    pub sample_addr: u16,
    pub sample_len: u16,
    pub current_byte: u8,
    pub bits_remaining: u8,
    pub irq: bool,
}

/// Cartridge mapper interface.
///
/// Mappers control PRG/CHR banking, name-table mirroring, IRQ timers,
/// and sometimes contain extra audio hardware.  This trait is the
/// minimal surface required to wire a mapper into the bus.
pub trait Mapper {
    /// Mapper number (iNES mapper ID).
    fn mapper_number(&self) -> u16;

    /// Reset (called on soft reset).
    fn reset(&mut self);

    /// Power-on initialization (called once at cart load).
    fn power(&mut self);

    /// CPU bus read.  Address is in CPU address space ($0000-$FFFF).
    fn cpu_read(&mut self, addr: u16) -> u8;

    /// CPU bus write.  Address is in CPU address space.
    fn cpu_write(&mut self, addr: u16, val: u8);

    /// PPU bus read.  Address is in PPU address space ($0000-$3FFF).
    fn ppu_read(&mut self, addr: u16) -> u8;

    /// PPU bus write.
    fn ppu_write(&mut self, addr: u16, val: u8);

    /// Return the current name-table mirroring mode.
    fn mirroring(&self) -> Mirroring;

    /// If the mapper generates audio (e.g. VRC6, MMC5, N163), fill
    /// the next `count` samples into `out` (interleaved i16).
    fn fill_exp_audio(&mut self, _out: &mut [i16], _count: usize) {}

    /// If the mapper has an IRQ timer, this is called every CPU cycle
    /// and may assert the IRQ line through `irq_out`.
    fn tick_irq(&mut self, _irq_out: &mut bool) {}

    /// Save-state snapshot (mapper-specific).
    fn snapshot(&self) -> Vec<u8>;
    fn restore_snapshot(&mut self, data: &[u8]);
}

/// Name-table mirroring modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreenLow,
    SingleScreenHigh,
    FourScreen,
}

/// Master system struct that owns all components.
///
/// This is the target architecture for v0.3.x: a single `NesSystem`
/// struct with `&mut self` dispatch, eliminating the distributed
/// global state that currently exists in C++.
#[derive(Debug)]
pub struct NesSystem<C: Cpu, P: Ppu, A: Apu, M: Mapper> {
    pub cpu: C,
    pub ppu: P,
    pub apu: A,
    pub mapper: M,
    pub wram: [u8; 0x800], // 2 KiB CPU WRAM ($0000-$07FF, mirrored to $1FFF)
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub cycle_count: u64,
}

impl<C: Cpu, P: Ppu, A: Apu, M: Mapper> NesSystem<C, P, A, M> {
    /// Create a new system.  PRG/CHR ROMs are taken ownership of.
    pub fn new(cpu: C, ppu: P, apu: A, mapper: M, prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        Self {
            cpu,
            ppu,
            apu,
            mapper,
            wram: [0; 0x800],
            prg_rom,
            chr_rom,
            cycle_count: 0,
        }
    }

    /// Advance the system by one master clock tick.
    ///
    /// On NTSC: 1 CPU cycle = 3 PPU cycles = 1 APU cycle.
    /// This is a placeholder for the full v0.3.x scheduler.
    pub fn tick(&mut self) -> PpuTickResult {
        // Placeholder: advance CPU, then PPU, then APU.
        // The real implementation will interleave at sub-cycle
        // granularity for cycle-accurate mappers.
        let ppu_result = self.ppu.tick(true);
        let mut apu_irq = false;
        self.apu.tick(&mut apu_irq);
        let mut mapper_irq = false;
        self.mapper.tick_irq(&mut mapper_irq);
        if apu_irq || mapper_irq {
            self.cpu.irq_begin(IrqSource::FrameCounter);
        }
        self.cycle_count += 1;
        ppu_result
    }

    /// CPU-visible bus read.  This is the "source of truth" for the
    /// CPU address space decode.
    pub fn cpu_bus_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => self.ppu.read_reg(((addr - 0x2000) & 7) as u8),
            0x4000..=0x4015 => self.apu.read_status(),
            0x4016..=0x4017 => 0, // TODO: input controllers
            0x4018..=0x5FFF => 0, // TODO: expansion / test
            0x6000..=0x7FFF => self.mapper.cpu_read(addr),
            0x8000..=0xFFFF => self.mapper.cpu_read(addr),
        }
    }

    /// CPU-visible bus write.
    pub fn cpu_bus_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize] = val,
            0x2000..=0x3FFF => self.ppu.write_reg(((addr - 0x2000) & 7) as u8, val),
            0x4000..=0x4017 => self.apu.write_reg((addr - 0x4000) as u8, val),
            0x4018..=0x5FFF => { /* TODO: expansion */ }
            0x6000..=0xFFFF => self.mapper.cpu_write(addr, val),
        }
    }
}
