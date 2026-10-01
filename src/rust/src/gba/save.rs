//! The savestate codec (v2.0 S2-b2).
//!
//! # What the vendored core does and does not give us
//!
//! Every state struct in `gba-core` derives `Serialize` / `Deserialize`, but
//! the crate ships **no serialization API and no format dependency** — there is
//! not a `bincode`, `postcard` or `ron` in it, and no call site that calls
//! `serialize()`. So the format and the version policy are entirely ours
//! (plan r7, section 2.1), and so is the list of things the derive *drops*.
//!
//! Three classes of field are not in the payload, and each needs a different
//! answer:
//!
//! | field | why it is skipped | what we do about it |
//! |---|---|---|
//! | `InternalMemory::rom`, `bios_system_rom` | read-only, loaded at startup | **not in the payload at all** — the caller loads the ROM and `gba_savestate_load` rebuilds from it. A fingerprint in the header is what stops a state from being restored onto the wrong cartridge. |
//! | `swi_hook`, `wake_hook` | function pointers | re-installed after a load. A loaded state has neither until `frame.rs` puts them back. |
//! | `PENDING_INTR_WAIT` (a `thread_local` in `swi/mod.rs`) | not part of `Arm7tdmi` at all | carried explicitly in [`WireState::pending_wait`]. **Skipping this one is silent**: the `halted` flag survives, so a state taken inside `VBlankIntrWait` comes back as a state that wakes on any interrupt (plan r37 ⑤). |
//!
//! `Gba::cartridge_header` is a fourth thing that is not in the payload: it is
//! a plain struct with no `Serialize` derive, so it is rebuilt from the ROM
//! rather than restored.
//!
//! # postcard was tried first and does not work
//!
//! It is the natural choice on paper: binary, maintained, no schema, stable.
//! It is also a **no-alloc** format, and this core's state carries six
//! `Vec<u8>` (`bios_system_rom`, `rom`, `working_ram`, `working_iram`, `sram`
//! in `InternalMemory`, and `Rtc::buffer`). `postcard::to_allocvec(&gba.cpu)`
//! does not compile against `Arm7tdmi` at all. That is a property of the
//! format, not of a version or a feature flag.
//!
//! # What is used instead, and what it costs
//!
//! `serde_json` — already in this workspace's graph via `f11qa`, allocates, and
//! a measured round trip of a real `Arm7tdmi` returns with `r0` and `pc`
//! identical. The cost is real and is not glossed over: a `Vec<u8>` becomes a
//! JSON array of decimal numbers, so the payload is several times the size of
//! the state and takes milliseconds to parse. It serves the first cut; the
//! final on-disk format is still open (plan r30). The only lever available
//! without patching the vendor tree is swapping the format — the core's fields
//! are private and derive-only, so a field-by-field encoding is not reachable
//! from here.
//!
//! # The stack, measured rather than assumed
//!
//! Plan r30 ④ recorded that the codec overflows a Rust test thread's 2 MB
//! stack, and used that to justify running the lock tests on a 16 MB stack. The
//! conclusion was right and the reasoning was not: `Arm7tdmi` is 82,032 bytes,
//! which is nowhere near 2 MB, and an earlier revision of this file claimed
//! that `Box::deserialize` avoids materialising the machine on the stack while
//! `from_slice` does not. Both were wrong. serde 1.0.229 implements
//! `Deserialize for Box<T>` as
//! `forwarded_impl! { (T), Box<T>, Box::new }`
//! (`serde_core/src/de/impls.rs`) — it deserialises a `T` **by value on the
//! calling stack** and only then boxes it.
//!
//! What actually eats the stack is `Lcd::buffer`: `[[Color; 240]; 160]`, a
//! 76,800-byte array **inline in the struct**, which serde materialises through
//! `serde_with`'s array adapter. Measured by bisecting `stack_size` from
//! outside (a stack overflow is a hard process abort, so it cannot be caught
//! in-process to report the number it failed at):
//!
//! | direction | 48 KB | 1 MB | 1.5 MB | 2 MB | 6 MB | 8 MB |
//! |---|---|---|---|---|---|---|
//! | encode, release | ok | ok | ok | ok | ok | ok |
//! | decode, release | — | **overflow** | **overflow** | ok | ok | ok |
//! | decode, debug | — | overflow | overflow | overflow | overflow | ok |
//!
//! The two directions differ by more than 40x, and the reason is worth keeping
//! in mind: serialisation walks the machine **by reference**, so it needs
//! almost nothing, while deserialisation builds it **by value**.
//!
//! So only the decode needs a thread of its own. [`load_on_worker`] gives it
//! [`CODEC_STACK`], eight times the measured peak, and the caller's frame keeps
//! nothing but a pointer — which is the property plan r31 ② asks for: no caller
//! has to know about stacks. Encoding stays on the caller's thread, where the
//! measurement says it fits with room to spare.

use gba_core::cpu::arm7tdmi::Arm7tdmi;
use gba_core::gba::Gba;
use serde::{Deserialize, Serialize};

use crate::gba::swi::wait::{IntrWaitRequest, WaitMode};

/// The four bytes every savestate starts with.
///
/// A magic rather than a version alone, so a file that is not ours is rejected
/// by inspection instead of by a deserializer that happens to accept the first
/// plausible byte pattern.
pub const SAVESTATE_MAGIC: [u8; 4] = *b"FCEG";

/// Format version, bumped whenever the payload stops being readable.
///
/// Not a per-state field: it travels in the header, so a mismatch is caught
/// before a single byte of payload is interpreted. Version 2 is the
/// [`WireState`] shape; version 1 was a bare `Arm7tdmi` with none of the
/// fields a machine needs to actually come back.
pub const SAVESTATE_VERSION: u32 = 2;

/// The stack [`load_on_worker`] decodes on.
///
/// Eight times the measured release peak (2 MB) and four times the debug one.
/// A constant rather than a habit: the numbers behind it are in the module
/// docs, and `the_codec_fits_in_the_callers_stack` re-measures them.
pub const CODEC_STACK: usize = 16 * 1024 * 1024;

/// Which cartridge a state belongs to.
///
/// The ROM is not in the payload — it is loaded once and shared by every state
/// taken from it, and the core marks it `#[serde(skip)]` for exactly that
/// reason. The consequence is that nothing stops a state from being restored
/// onto a *different* cartridge, and a state that loads is not a state that
/// works. These four fields are the check.
///
/// `game_code` and `marker_code` are the header's own identifiers ("BPEE",
/// "01"), so together they name a game, a region and a revision. The
/// one-byte `calculated_checksum` the header carries is deliberately **not**
/// used: it covers 156 bytes and would not notice a re-dump. `content_hash` is
/// FNV-1a over the whole cartridge, which is a few microseconds once per load.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RomFingerprint {
    game_code: String,
    marker_code: String,
    rom_len: u32,
    content_hash: u32,
}

impl RomFingerprint {
    /// Fingerprint the cartridge a machine is running.
    #[must_use]
    pub fn of(gba: &Gba) -> Self {
        let rom = &gba.cpu.bus.internal_memory.rom;
        Self {
            game_code: gba.cartridge_header.game_code.clone(),
            marker_code: gba.cartridge_header.marker_code.clone(),
            // A cartridge longer than 4 GiB cannot exist, and the ABI carries
            // this in a `u32`; truncating is louder than wrapping silently.
            rom_len: u32::try_from(rom.len()).unwrap_or(u32::MAX),
            content_hash: fnv1a(rom),
        }
    }
}

/// FNV-1a, 32-bit.
///
/// Not a cryptographic hash and does not need to be: it catches "the user
/// loaded a different game", which is a mistake a person makes, not an
/// adversary. Choosing it over a dependency is the point — the alternative is
/// a new crate in the graph for one 32-bit value.
fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// The sample clock's position, so a restored machine keeps counting where the
/// state left off.
///
/// `phase` is the residual in units of `rate * 4389`; dropping it would make
/// the next frame's sample count wrong by up to one sample, and the error would
/// repeat every frame — a drift of exactly the kind §4.2 exists to prevent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioState {
    phase: u64,
    rate: u32,
}

impl AudioState {
    /// Read the clock's position off a live [`crate::gba::audio::AudioOut`].
    #[must_use]
    pub fn capture(audio: &crate::gba::audio::AudioOut) -> Self {
        let (phase, rate) = audio.clock_state();
        Self { phase, rate }
    }

    /// The residual, to hand back on restore.
    #[must_use]
    pub const fn phase(&self) -> u64 {
        self.phase
    }

    /// The rate the residual is counted in.
    #[must_use]
    pub const fn rate(&self) -> u32 {
        self.rate
    }
}

/// The `IntrWait` a machine was asleep on, flattened for the wire.
///
/// `wait.rs` types are ours and could derive `Serialize` directly, but keeping
/// the wire form a separate POD means a change to the wait logic cannot
/// silently change the file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingWait {
    wanted: u8,
    always_wait: bool,
}

impl PendingWait {
    /// Flatten a live request.
    #[must_use]
    pub fn of(request: IntrWaitRequest) -> Self {
        Self {
            wanted: request.wanted,
            always_wait: request.mode == WaitMode::AlwaysWait,
        }
    }

    /// Rebuild the request the SWI layer understands.
    #[must_use]
    pub const fn to_request(self) -> IntrWaitRequest {
        IntrWaitRequest {
            wanted: self.wanted,
            mode: if self.always_wait {
                WaitMode::AlwaysWait
            } else {
                WaitMode::ReturnIfSet
            },
        }
    }
}

/// A decoded state: the core's machine, plus everything the core does not
/// carry.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireState {
    /// The machine. `Box` because the decode builds it by value and a by-value
    /// return would put it on the caller's stack — which is the whole problem
    /// this module is arranged around.
    pub cpu: Box<Arm7tdmi>,
    /// The cartridge this state came from.
    pub fingerprint: RomFingerprint,
    /// Where the sample clock had got to.
    pub audio: AudioState,
    /// The `IntrWait` the CPU was parked on, if any.
    pub pending_wait: Option<PendingWait>,
}

/// The encoding side of [`WireState`].
///
/// A separate type from [`WireState`] because the core's `Arm7tdmi` derives
/// `Serialize` and `Deserialize` but **not** `Clone`, so an encode that owned
/// its machine would have to copy ~460 KB of buffers to do it. Serialisation
/// walks by reference, so borrowing costs nothing — which is also why encode
/// needs 48 KB of stack where decode needs 2 MB.
///
/// The two types must keep the same field names. Nothing enforces that except
/// the round-trip tests: a name changed on one side only shows up as a decode
/// failure, because both sides carry `deny_unknown_fields`.
#[derive(Serialize)]
struct WireRef<'a> {
    cpu: &'a Arm7tdmi,
    fingerprint: &'a RomFingerprint,
    audio: AudioState,
    pending_wait: Option<PendingWait>,
}

/// Why a state could not be written or read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveError {
    /// The first four bytes are not [`SAVESTATE_MAGIC`].
    NotOurs,
    /// The header's version is newer than [`SAVESTATE_VERSION`].
    FutureVersion(u32),
    /// The payload did not decode into a machine state.
    Payload,
}

/// A serialized state plus its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveState {
    /// `SAVESTATE_MAGIC` followed by `SAVESTATE_VERSION`, little-endian.
    pub header: [u8; 8],
    /// The encoded [`WireRef`].
    pub payload: Vec<u8>,
}

/// Serialize a machine state.
///
/// The `Bus` is reached through the CPU because that is where the hook and the
/// whole address space already are; `Arm7tdmi` derives `Serialize`, so this is
/// a direct call rather than a hand-rolled walk of the fields.
///
/// Runs on the caller's thread. Measured: encoding needs 48 KB, because
/// `serde` walks the machine by reference.
pub fn save(
    cpu: &Arm7tdmi,
    fingerprint: &RomFingerprint,
    audio: AudioState,
    pending_wait: Option<PendingWait>,
) -> Result<SaveState, SaveError> {
    let wire = WireRef {
        cpu,
        fingerprint,
        audio,
        pending_wait,
    };
    let payload = serde_json::to_vec(&wire).map_err(|_| SaveError::Payload)?;
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(&SAVESTATE_MAGIC);
    header[4..].copy_from_slice(&SAVESTATE_VERSION.to_le_bytes());
    Ok(SaveState { header, payload })
}

/// Read a state back.
///
/// **Needs about 2 MB of stack** — see the module docs. Use [`load_on_worker`]
/// unless the caller is a test that is measuring the requirement.
///
/// The hooks come back unset — they are `serde(skip)`ped and have `Default`
/// values, and `frame.rs` re-installs them. The ROM and BIOS come back empty
/// for the same reason, and `frame.rs` rebuilds the machine around them.
pub fn load(bytes: &[u8]) -> Result<WireState, SaveError> {
    if bytes.len() < 8 || bytes[..4] != SAVESTATE_MAGIC {
        return Err(SaveError::NotOurs);
    }
    let mut version_bytes = [0u8; 4];
    version_bytes.copy_from_slice(&bytes[4..8]);
    let version = u32::from_le_bytes(version_bytes);
    if version > SAVESTATE_VERSION {
        return Err(SaveError::FutureVersion(version));
    }
    serde_json::from_slice(&bytes[8..]).map_err(|_| SaveError::Payload)
}

/// [`load`], on a thread with a stack big enough for it.
///
/// The decode is the only part of the codec that needs one: it builds an
/// `Arm7tdmi` by value, and the caller's frame — a `QThread` with the Windows
/// default of 1 MB — cannot hold one. Doing it here rather than asking the
/// caller for a bigger stack is the whole of plan r31 ②.
///
/// The two failure kinds are kept apart because the caller reports them
/// differently. `None` means the thread could not be started, which is this
/// machine's problem and not the file's; `Some(Err(..))` is the file's, and the
/// error says which way.
///
/// The release profile is `panic = "abort"`, so a panic inside the decode takes
/// the process with it rather than arriving here as an `Err`. That is why
/// `load` reports every failure it can as a `Result`: the one thing it cannot
/// report is a stack overflow, and the stack is what this function exists to
/// make impossible.
#[must_use]
pub fn load_on_worker(bytes: Vec<u8>) -> Option<Result<WireState, SaveError>> {
    std::thread::Builder::new()
        .stack_size(CODEC_STACK)
        .spawn(move || load(&bytes))
        .ok()?
        .join()
        .ok()
}

#[cfg(test)]
mod tests {
    use gba_core::gba::Gba;

    use super::{
        AudioState, CODEC_STACK, PendingWait, RomFingerprint, SAVESTATE_MAGIC, SAVESTATE_VERSION,
        SaveError, load, load_on_worker, save,
    };
    use crate::gba::swi::wait::{IntrWaitRequest, VBLANK_FLAG, WaitMode};

    /// The stack the C++ side's emulation thread actually has.
    ///
    /// `gba_savestate_load` runs inside a `QThread` (`ConsoleEmulatorThread.cpp`
    /// overrides `run()`), and Qt creates a Windows thread with no explicit
    /// stack size, so it gets the platform default of 1 MB. This is the number
    /// the ABI has to survive, not the test harness's 2 MB and not
    /// [`CODEC_STACK`].
    const CALLER_STACK: usize = 1024 * 1024;

    /// A machine that has actually run, so the state is not all zeroes.
    ///
    /// A round trip over a fresh machine proves almost nothing: every field is
    /// its default, so a serializer that dropped half the state would still
    /// come back equal.
    fn stepped_machine(steps: usize) -> Box<Gba> {
        let mut gba = Box::new(Gba::new([0u8; 0x4000], &[0u8; 0x200]));
        for _ in 0..steps {
            gba.step();
        }
        gba
    }

    /// A cartridge-shaped ROM whose bytes differ, so two machines can be told
    /// apart the way two real games would be.
    fn cart(seed: u8) -> Vec<u8> {
        let mut rom = vec![seed; 0x200];
        // The header's own identifiers, so the fingerprint has real content.
        rom[0xAC..0xB0].copy_from_slice(if seed % 2 == 0 { b"BPEE" } else { b"AXVE" });
        rom[0xB0..0xB2].copy_from_slice(b"01");
        rom
    }

    /// The bytes of a saved state, header included.
    fn bytes_of(machine: &Gba) -> Vec<u8> {
        let state = save(
            &machine.cpu,
            &RomFingerprint::of(machine),
            AudioState { phase: 7, rate: 44_100 },
            None,
        )
        .expect("serialize");
        state
            .header
            .iter()
            .chain(state.payload.iter())
            .copied()
            .collect()
    }

    /// The header is ours and carries the current version.
    #[test]
    fn the_header_identifies_the_format() {
        let machine = stepped_machine(4);
        let state = save(
            &machine.cpu,
            &RomFingerprint::of(&machine),
            AudioState { phase: 0, rate: 44_100 },
            None,
        )
        .expect("a fresh machine serializes");
        assert_eq!(&state.header[..4], &SAVESTATE_MAGIC);
        assert_eq!(
            u32::from_le_bytes(state.header[4..].try_into().expect("4 bytes")),
            SAVESTATE_VERSION
        );
    }

    /// A machine that has run comes back with its registers intact.
    ///
    /// This is the test that decides the format works. It runs the machine
    /// first so the state is not all defaults — the mistake a lossy serializer
    /// makes is a silently zeroed machine, which only a non-trivial state can
    /// catch.
    #[test]
    fn a_ran_machine_comes_back_with_its_registers() {
        let machine = stepped_machine(64);
        let restored = load_on_worker(bytes_of(&machine)).expect("the worker started").expect("decode");
        assert_eq!(
            restored.cpu.registers.program_counter(),
            machine.cpu.registers.program_counter(),
            "the program counter was lost"
        );
        for bank in 0..4usize {
            assert_eq!(
                restored.cpu.registers.register_at(bank),
                machine.cpu.registers.register_at(bank),
                "r{bank} was lost"
            );
        }
    }

    /// Memory comes back byte for byte, not just the registers.
    ///
    /// The register check above is satisfied by a state that keeps the CPU and
    /// drops everything the CPU points at, which is the shape of the bug r37 ①
    /// describes. Comparing a region of working RAM is what makes the
    /// difference observable.
    #[test]
    fn memory_comes_back_byte_for_byte() {
        let mut machine = stepped_machine(64);
        // A recognisable pattern, written through the same path a game uses.
        let original: Vec<u8> = (0..64u16)
            .map(|offset| {
                let value = (0xA5u8) ^ offset as u8;
                machine
                    .cpu
                    .bus
                    .write_byte((0x0200_0000u32 + u32::from(offset)) as usize, value);
                value
            })
            .collect();

        let mut restored = load_on_worker(bytes_of(&machine))
            .expect("the worker started")
            .expect("decode");
        for (offset, expected) in original.iter().enumerate() {
            let got = restored
                .cpu
                .bus
                .read_byte((0x0200_0000u32 + offset as u32) as usize);
            assert_eq!(got, *expected, "working RAM differs at +{offset}");
        }
    }

    /// The decoded machine carries no ROM and no BIOS, and that is correct.
    ///
    /// The core marks both `#[serde(skip)]`, so a decoded `Arm7tdmi` has empty
    /// vectors there. Without this fact written down, the next person reads it
    /// as a broken codec; with it, it is the reason `frame.rs` rebuilds the
    /// machine around the decoded CPU instead of installing it directly.
    #[test]
    fn a_decoded_state_carries_no_rom_or_bios() {
        let mut machine = Box::new(Gba::new([0u8; 0x4000], &cart(1)));
        machine.step();
        let restored = load_on_worker(bytes_of(&machine)).expect("the worker started").expect("decode");
        assert!(
            restored.cpu.bus.internal_memory.rom.is_empty(),
            "a cartridge leaked into the payload; frame.rs would then load the wrong one"
        );
        assert!(
            restored.cpu.bus.internal_memory.bios_system_rom.is_empty(),
            "the BIOS leaked into the payload"
        );
    }

    /// A loaded state has no hook until one is installed again.
    ///
    /// `swi_hook` and `wake_hook` are `serde(skip)`ped, so they round trip as
    /// `None`. Asserting it makes that contract explicit rather than a surprise
    /// at the first SWI after a load.
    #[test]
    fn a_loaded_state_has_no_hook_until_one_is_installed() {
        let machine = stepped_machine(4);
        let restored = load_on_worker(bytes_of(&machine)).expect("the worker started").expect("decode");
        assert!(
            restored.cpu.swi_hook.is_none(),
            "a function pointer must not survive"
        );
        assert!(restored.cpu.wake_hook.is_none());
    }

    /// The sample clock's residual survives, in both directions.
    ///
    /// Without this the next frame's sample count is off by up to one, every
    /// frame — the drift §4.2 exists to rule out, arriving through the back
    /// door.
    #[test]
    fn the_audio_residual_is_carried() {
        let machine = stepped_machine(4);
        let state = save(
            &machine.cpu,
            &RomFingerprint::of(&machine),
            AudioState { phase: 123_457, rate: 48_000 },
            None,
        )
        .expect("serialize");
        let restored = load_on_worker(
            state
                .header
                .iter()
                .chain(state.payload.iter())
                .copied()
                .collect(),
        )
        .expect("the worker started").expect("decode");
        assert_eq!(restored.audio.phase(), 123_457);
        assert_eq!(restored.audio.rate(), 48_000);
    }

    /// A pending `IntrWait` survives, in both directions.
    ///
    /// The CPU's `halted` flag is serialised, so a state taken inside
    /// `VBlankIntrWait` really does come back asleep. If the *condition* were
    /// dropped, it would come back asleep with the wrong condition — waking on
    /// any interrupt — and nothing would report it.
    #[test]
    fn a_pending_wait_is_carried() {
        let machine = stepped_machine(4);
        let request = IntrWaitRequest {
            wanted: VBLANK_FLAG,
            mode: WaitMode::AlwaysWait,
        };
        let state = save(
            &machine.cpu,
            &RomFingerprint::of(&machine),
            AudioState { phase: 0, rate: 44_100 },
            Some(PendingWait::of(request)),
        )
        .expect("serialize");
        let restored = load_on_worker(
            state
                .header
                .iter()
                .chain(state.payload.iter())
                .copied()
                .collect(),
        )
        .expect("the worker started").expect("decode");
        let pending = restored.pending_wait.expect("the wait was dropped");
        assert_eq!(pending.to_request(), request);
    }

    /// The wait's mode is not flattened to a boolean by accident.
    ///
    /// `ReturnIfSet` and `AlwaysWait` differ only in `r0`, and a codec that
    /// stored "is there a wait" rather than "which wait" would pass the test
    /// above for `AlwaysWait` and silently turn `ReturnIfSet` into it.
    #[test]
    fn both_wait_modes_survive_the_round_trip() {
        let machine = stepped_machine(4);
        for mode in [WaitMode::ReturnIfSet, WaitMode::AlwaysWait] {
            let request = IntrWaitRequest { wanted: VBLANK_FLAG, mode };
            let state = save(
                &machine.cpu,
                &RomFingerprint::of(&machine),
                AudioState { phase: 0, rate: 44_100 },
                Some(PendingWait::of(request)),
            )
            .expect("serialize");
            let restored = load_on_worker(
                state
                    .header
                    .iter()
                    .chain(state.payload.iter())
                    .copied()
                    .collect(),
            )
            .expect("the worker started").expect("decode");
            assert_eq!(
                restored.pending_wait.expect("the wait was dropped").to_request(),
                request,
                "{mode:?} did not survive"
            );
        }
    }

    /// Two cartridges are two fingerprints, and the same cartridge is one.
    ///
    /// Both directions, because a fingerprint that changes every time would be
    /// just as useless as one that never changes — and the first half alone
    /// would pass on a codec that hashed a fresh `RandomState`.
    #[test]
    fn the_fingerprint_separates_cartridges_and_is_stable() {
        let first = Gba::new([0u8; 0x4000], &cart(1));
        let same = Gba::new([0u8; 0x4000], &cart(1));
        let other = Gba::new([0u8; 0x4000], &cart(2));

        assert_eq!(RomFingerprint::of(&first), RomFingerprint::of(&same));
        assert_ne!(RomFingerprint::of(&first), RomFingerprint::of(&other));
    }

    /// The fingerprint notices a cartridge of the same header but different
    /// content.
    ///
    /// The header identifiers are only four bytes; a re-dump with a patched
    /// header would defeat them. This is the case the content hash is there
    /// for, and it is the one a header-only check misses.
    #[test]
    fn the_fingerprint_notices_a_changed_body() {
        let original = cart(1);
        let mut patched = original.clone();
        patched[0x100] = patched[0x100].wrapping_add(1);
        assert_ne!(
            RomFingerprint::of(&Gba::new([0u8; 0x4000], &original)),
            RomFingerprint::of(&Gba::new([0u8; 0x4000], &patched)),
            "a one-byte change in the body was not noticed"
        );
    }

    /// A file that is not ours is rejected by its magic, and a newer one by its
    /// version — both before a payload byte is interpreted.
    #[test]
    fn a_foreign_file_is_rejected_before_it_is_decoded() {
        let junk = vec![0u8; 64];
        // `Arm7tdmi` has no `Debug`, so these compare the error rather than
        // the `Result` as a whole.
        assert!(matches!(load(&junk), Err(SaveError::NotOurs)));
        let mut stale = SAVESTATE_MAGIC.to_vec();
        stale.extend_from_slice(&(SAVESTATE_VERSION + 1).to_le_bytes());
        stale.extend_from_slice(&[0u8; 16]);
        assert!(
            matches!(load(&stale), Err(SaveError::FutureVersion(v)) if v == SAVESTATE_VERSION + 1),
            "a newer version is refused, not guessed at"
        );
    }

    /// A payload with a field the format does not have is refused.
    ///
    /// `deny_unknown_fields` is what turns "the core renamed a field" into a
    /// clean refusal instead of a state that loads and is quietly wrong. This
    /// is the guard for that, written as a test because the attribute itself
    /// cannot fail loudly on its own.
    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        let machine = stepped_machine(4);
        let mut bytes = bytes_of(&machine);
        // Splice an extra key into the payload's top-level object.
        let payload = String::from_utf8(bytes[8..].to_vec()).expect("the payload is text");
        let spliced = payload.replacen('{', r#"{"surprise":1,"#, 1);
        bytes.truncate(8);
        bytes.extend_from_slice(spliced.as_bytes());
        assert!(
            matches!(load(&bytes), Err(SaveError::Payload)),
            "an unknown field was accepted"
        );
    }

    /// Both directions of the codec fit in the stack the C++ side's caller has.
    ///
    /// The measurement behind [`CODEC_STACK`] and behind the whole arrangement
    /// of this module. `GBA_PROBE_STACK_BYTES` drives the size and
    /// `GBA_PROBE_DIR` picks a direction (`encode`, `decode`, or both), so the
    /// two peaks — 48 KB and 2 MB, a factor of 40 — can be told apart and
    /// re-measured after a toolchain change. Unset, this asserts the size the
    /// ABI actually has to survive.
    ///
    /// It calls [`load`] rather than [`load_on_worker`] on purpose: the worker
    /// would hide the very thing being measured.
    /// The decode survives the stack the C++ side's caller actually has.
    ///
    /// This is the ABI contract, and it is about the *production path*:
    /// `gba_savestate_load` calls [`load_on_worker`], so the caller's frame
    /// holds a pointer and nothing else. The raw [`load`] does not fit in 1 MB
    /// — 2 MB in release, 6–7 MB in debug — which is exactly why the worker
    /// exists, and why this test is written against the wrapper rather than
    /// against `load`.
    ///
    /// Spawned on a 1 MB thread on purpose. Run on the harness's 2 MB it would
    /// pass a decoder that overflows the real thing.
    #[test]
    fn the_decode_survives_the_callers_stack() {
        let machine = stepped_machine(64);
        let bytes = bytes_of(&machine);

        let decoded = std::thread::Builder::new()
            .stack_size(CALLER_STACK)
            .spawn(move || load_on_worker(bytes).is_some())
            .expect("spawn a 1 MB thread")
            .join()
            .expect("the probe thread must not panic");
        assert!(
            decoded,
            "the decode did not survive a {CALLER_STACK}-byte stack"
        );
    }

    /// Measure the raw decoder's peak: one stack size per process run.
    ///
    /// **Ignored by default, and that is the point.** A stack overflow is a
    /// hard process abort — `0xC00000FD`, no unwinding, no message — so a test
    /// that measured it in-process would take the whole suite down with it
    /// instead of reporting a number. Bisecting therefore has to be driven from
    /// outside, one process per size:
    ///
    /// ```text
    /// set GBA_PROBE_DIR=decode && set GBA_PROBE_STACK_BYTES=1048576
    /// cargo test -p fceux11-rust --no-default-features --features gba --lib -- --ignored
    /// ```
    ///
    /// Exit code 0 means that size was enough; a crash means it was not. The
    /// results this produced on 2026-10-01 are in the module docs — re-run it
    /// after a toolchain change rather than trusting them, and remember the
    /// release figure is the one that matters.
    #[test]
    #[ignore = "a measurement tool: it takes the process down at small sizes, on purpose"]
    fn the_raw_decoder_needs_a_stack_of_its_own() {
        let direction = std::env::var("GBA_PROBE_DIR").unwrap_or_else(|_| "decode".to_owned());
        let probe = std::env::var("GBA_PROBE_STACK_BYTES")
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(CODEC_STACK);

        let machine = stepped_machine(64);
        let fingerprint = RomFingerprint::of(&machine);
        let state = save(
            &machine.cpu,
            &fingerprint,
            AudioState { phase: 0, rate: 44_100 },
            None,
        )
        .expect("serialize");
        let payload_len = state.payload.len();
        let bytes: Vec<u8> = state
            .header
            .iter()
            .chain(state.payload.iter())
            .copied()
            .collect();

        if direction == "encode" {
            let encoded = std::thread::Builder::new()
                .stack_size(probe)
                .spawn({
                    let fingerprint = fingerprint.clone();
                    move || {
                        save(
                            &machine.cpu,
                            &fingerprint,
                            AudioState { phase: 0, rate: 44_100 },
                            None,
                        )
                        .map(|s| s.payload.len())
                    }
                })
                .expect("spawn the probe thread")
                .join()
                .expect("the probe thread must not panic")
                .expect("a stepped machine serializes");
            assert_eq!(encoded, payload_len, "the round trip is not the same length");
            return;
        }

        let decoded = std::thread::Builder::new()
            .stack_size(probe)
            .spawn(move || load(&bytes).is_ok())
            .expect("spawn the probe thread")
            .join()
            .expect("the probe thread must not panic");
        assert!(decoded, "decoding did not fit in a {probe}-byte stack");
    }

    /// The worker's stack is sized from the measurement, not from a habit.
    ///
    /// A cheap arithmetic guard in place of a comment: if someone lowers
    /// [`CODEC_STACK`] to "just enough" the decode starts overflowing, and the
    /// first sign of it is a crashed emulator with no diagnostic.
    #[test]
    fn the_worker_stack_clears_the_measured_peak() {
        assert!(
            CODEC_STACK >= 4 * 2 * 1024 * 1024,
            "CODEC_STACK is {CODEC_STACK}, below the 2 MB release peak"
        );
    }

    /// How many numbers a JSON subtree holds, at any depth.
    ///
    /// Bytes *per number* is the figure that decides the on-disk format: a
    /// `Vec<u8>` costs one decimal number and a comma per byte, so the ratio is
    /// what a binary or base64 encoding would divide out.
    fn count_numbers(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Number(_) => 1,
            serde_json::Value::Array(items) => items.iter().map(count_numbers).sum(),
            serde_json::Value::Object(entries) => entries.values().map(count_numbers).sum(),
            _ => 0,
        }
    }

    /// The byte arrays in a payload, biggest first, named by their key.
    ///
    /// The whole machine arrives under one `cpu` key, so a top-level split
    /// cannot say *which* memory is expensive -- and that is the question a
    /// format decision turns on. A byte array is recognised by being an array
    /// of plain integers, which is what every memory region in the core is and
    /// what no other field in the state is.
    fn byte_arrays(value: &serde_json::Value, path: &str, out: &mut Vec<(String, usize)>) {
        match value {
            serde_json::Value::Array(items) => {
                let all_numbers = items
                    .iter()
                    .all(|item| matches!(item, serde_json::Value::Number(_)));
                if all_numbers && !items.is_empty() {
                    out.push((path.to_owned(), items.len()));
                }
                for (index, item) in items.iter().enumerate() {
                    byte_arrays(item, &format!("{path}[{index}]"), out);
                }
            }
            serde_json::Value::Object(entries) => {
                for (key, child) in entries {
                    byte_arrays(child, &format!("{path}.{key}"), out);
                }
            }
            _ => {}
        }
    }

    /// What one state costs on disk and in time.
    ///
    /// **Ignored by default, because it reports rather than gates.** These
    /// numbers are what plan limitation **L13** turns on: the payload is JSON,
    /// so working RAM and the frame buffer are arrays of decimal numbers, and
    /// L13's own resolution date reads "when the instant-save wiring lands" --
    /// which is the phase this measurement belongs to. Asserting a ceiling here
    /// would go stale the moment the core grows a memory region, so what this
    /// test actually asserts is only that a state round-trips; the rest is
    /// printed for whoever has to choose a format.
    ///
    /// ```text
    /// cargo test -p fceux11-rust --no-default-features --features gba --lib \
    ///     measure_a_state_on_disk -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a measurement tool: it reports the numbers L13 turns on, it does not gate them"]
    fn measure_a_state_on_disk() {
        const REPEATS: u32 = 20;

        let machine = stepped_machine(64);
        let fingerprint = RomFingerprint::of(&machine);
        let audio = AudioState {
            phase: 123_457,
            rate: 48_000,
        };

        let started = std::time::Instant::now();
        let mut encoded = None;
        for _ in 0..REPEATS {
            encoded = Some(save(&machine.cpu, &fingerprint, audio, None).expect("serialize"));
        }
        let encode = started.elapsed() / REPEATS;
        let state = encoded.expect("encoded at least once");
        let total = state.header.len() + state.payload.len();

        // The path the C ABI takes, thread spawn included: that spawn is part of
        // what a player waits for, so measuring `load` alone would flatter it.
        let bytes: Vec<u8> = state
            .header
            .iter()
            .chain(state.payload.iter())
            .copied()
            .collect();
        let started = std::time::Instant::now();
        for _ in 0..REPEATS {
            assert!(load_on_worker(bytes.clone()).expect("a worker").is_ok());
        }
        let decode = started.elapsed() / REPEATS;

        let tree: serde_json::Value =
            serde_json::from_slice(&state.payload).expect("the payload is the JSON we wrote");
        let mut parts: Vec<(String, usize, usize)> = tree
            .as_object()
            .expect("an object at the top")
            .iter()
            .map(|(key, value)| {
                let len = serde_json::to_vec(value).expect("re-encode a subtree").len();
                (key.clone(), len, count_numbers(value))
            })
            .collect();
        parts.sort_by_key(|(_, len, _)| std::cmp::Reverse(*len));

        let mut arrays = Vec::new();
        byte_arrays(&tree, "cpu", &mut arrays);
        arrays.sort_by_key(|(_, len)| std::cmp::Reverse(*len));
        // The total has to come from the whole list, not the printed head: the
        // scanline buffers are 228 separate small arrays, so truncating first
        // would leave out a region that is a real share of the file.
        let raw: usize = arrays.iter().map(|(_, len)| *len).sum();
        let shown = arrays.len().min(8);
        arrays.truncate(shown);

        println!("state: {total} bytes (header {} + payload {})", state.header.len(), state.payload.len());
        println!("  {total} B = {:.1} KiB, {REPEATS} repeats, debug timings unless --release", total as f64 / 1024.0);
        println!("encode: {encode:?}   decode (worker, spawn included): {decode:?}");
        println!("largest top-level fields:");
        for (key, len, numbers) in &parts {
            let per = if *numbers == 0 {
                f64::NAN
            } else {
                *len as f64 / *numbers as f64
            };
            println!("  {key:<20} {len:>9} B  {numbers:>9} numbers  {per:>5.2} B/number");
        }
        println!("largest byte arrays (path, element count):");
        for (path, len) in &arrays {
            println!("  {path:<48} {len:>8} B raw");
        }
        let count = arrays.len();
        println!(
            "  every byte array in the state holds {raw} B raw; the {count} largest are above"
        );
        println!(
            "  JSON costs {:.2} B per raw byte, so a byte-for-byte format lands near {raw} B",
            total as f64 / raw as f64
        );
    }
}
