//! SWI 0x0A-adjacent groundwork: the savestate serialization surface (S2-b).
//!
//! # Why the format is a question and not a detail
//!
//! The vendored core derives `Serialize` / `Deserialize` on every state
//! struct, but ships **no serialization API and no format dependency** — there
//! is not a `bincode`, `postcard` or `ron` anywhere in it, and no call site
//! that calls `serialize()` at all. So the format and the version policy are
//! entirely ours (plan r7, section 2.1).
//!
//! # postcard was tried first and does not work
//!
//! It is the natural choice on paper: binary, maintained, no schema, and
//! stable. It is also a **no-alloc** format, and this core's state carries six
//! `Vec<u8>`:
//!
//! | field | where |
//! |---|---|
//! | `bios_system_rom` | `InternalMemory` |
//! | `rom` | `InternalMemory` |
//! | `working_ram` | `InternalMemory` |
//! | `working_iram` | `InternalMemory` |
//! | `sram` | `InternalMemory` |
//! | `buffer` | `Rtc` |
//!
//! `postcard::to_allocvec(&gba.cpu)` does not compile against `Arm7tdmi` at
//! all. This is a property of the format, not of a version or a feature flag,
//! and it would not be fixed by adding a feature.
//!
//! # What is used instead, and what it costs
//!
//! `serde_json` — already in this workspace's graph via `f11qa`, allocates,
//! and a measured round trip of a real `Arm7tdmi` returns with `r0` and `pc`
//! identical. The cost is real and is not being glossed over: JSON is large
//! and slow for a machine state, so it is the wrong shape for a hot on-disk
//! file. It serves the first cut; the final on-disk format is still open
//! (plan r30).
//!
//! # `serde(skip)` fields
//!
//! `Arm7tdmi::swi_hook` and `wake_hook` are function pointers marked
//! `#[serde(skip)]`, and correctly so: a function pointer has no savestate
//! meaning, and the embedder re-installs both on load. Everything the
//! `Deserialize` impl substitutes there is the `Default` value, so **a loaded
//! state has no hook until one is installed again** — see the round-trip test
//! below, which asserts the data rather than the hooks.

use gba_core::cpu::arm7tdmi::Arm7tdmi;

/// The four bytes every savestate starts with.
///
/// A magic rather than a version alone, so a file that is not ours is rejected
/// by inspection instead of by a deserializer that happens to accept the first
/// plausible byte pattern.
pub const SAVESTATE_MAGIC: [u8; 4] = *b"FCEG";

/// Format version, bumped whenever the payload stops being readable.
///
/// Not a per-state field: it travels in the header, so a mismatch is caught
/// before a single byte of payload is interpreted.
pub const SAVESTATE_VERSION: u32 = 1;

/// A serialized state plus its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveState {
    /// `SAVESTATE_MAGIC` followed by `SAVESTATE_VERSION`, little-endian.
    pub header: [u8; 8],
    /// The encoded `Arm7tdmi`.
    pub payload: Vec<u8>,
}

/// Why a state could not be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveError {
    /// The first four bytes are not [`SAVESTATE_MAGIC`].
    NotOurs,
    /// The header's version is newer than [`SAVESTATE_VERSION`].
    FutureVersion(u32),
    /// The payload did not decode into a machine state.
    Payload,
}

/// Serialize a machine state.
///
/// The `Bus` is reached through the CPU because that is where the hook and the
/// whole address space already are; `Arm7tdmi` derives both traits, so this is
/// a direct call rather than a hand-rolled walk of the fields.
pub fn save(cpu: &Arm7tdmi) -> Result<SaveState, SaveError> {
    let payload = serde_json::to_vec(cpu).map_err(|_| SaveError::Payload)?;
    let mut header = [0u8; 8];
    header[..4].copy_from_slice(&SAVESTATE_MAGIC);
    header[4..].copy_from_slice(&SAVESTATE_VERSION.to_le_bytes());
    Ok(SaveState { header, payload })
}

/// Read a state back.
///
/// The hooks come back unset — they are `serde(skip)`ped and have `Default`
/// values, and the caller re-installs them. That is why this returns a `Box`:
/// `Arm7tdmi` embeds the whole `Bus`, so a by-value return means a machine
/// state on the stack of whichever frame called us, and the C ABI calls this
/// from a chain that has no room for one.
///
/// The hooks are the caller's job to reinstall; see the round-trip test.
pub fn load(bytes: &[u8]) -> Result<Box<Arm7tdmi>, SaveError> {
    if bytes.len() < 8 || bytes[..4] != SAVESTATE_MAGIC {
        return Err(SaveError::NotOurs);
    }
    let mut version_bytes = [0u8; 4];
    version_bytes.copy_from_slice(&bytes[4..8]);
    let version = u32::from_le_bytes(version_bytes);
    if version > SAVESTATE_VERSION {
        return Err(SaveError::FutureVersion(version));
    }
    // `Box::deserialize` rather than `from_slice::<Arm7tdmi>`: the latter
    // materialises the machine on this frame, and `Arm7tdmi` is large enough
    // (it embeds the whole `Bus`) that two of them overflow a test thread.
    let cpu: Box<Arm7tdmi> =
        serde_json::from_slice(&bytes[8..]).map_err(|_| SaveError::Payload)?;
    Ok(cpu)
}

#[cfg(test)]
mod tests {
    use gba_core::gba::Gba;

    use super::{SAVESTATE_MAGIC, SAVESTATE_VERSION, SaveError, load, save};

    /// A fresh machine, used where the state itself is irrelevant.
    fn fresh_machine() -> Box<Gba> {
        Box::new(Gba::new([0u8; 0x4000], &[0u8; 0x200]))
    }

    /// Run a test body on a thread with a stack big enough for a machine
    /// state.
    ///
    /// `Arm7tdmi` embeds the entire `Bus` — the 240x160 frame buffer, 256K of
    /// WRAM, 32K of IWRAM, 96K of VRAM and the cartridge — so a value of this
    /// type is close to a megabyte, and serde's generated `Deserialize` builds
    /// one on the stack. Rust's test harness gives each test a 2 MB thread
    /// stack, which is not enough once a state, a decoded state and the
    /// intermediate frames are all live at once.
    ///
    /// This is not a test-only concern: the C ABI has the same requirement, and
    /// S2's `gba_savestate_load` will need the same treatment. Recorded here so
    /// the next person does not read the overflow as a serialization bug.
    fn with_big_stack(body: impl FnOnce() + Send + 'static) {
        const STACK: usize = 16 * 1024 * 1024;
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(body)
            .expect("spawn a test thread")
            .join()
            .expect("the test body must not panic");
    }

    /// A machine that has actually run, so the state is not all zeroes.
    ///
    /// A round trip over a fresh machine proves almost nothing: every field is
    /// its default, so a serializer that dropped half the state would still
    /// come back equal.
    ///
    /// An `Arm7tdmi` is close to a megabyte -- it embeds the `Bus`, which
    /// embeds the 240x160 frame buffer plus 256K of WRAM, 32K of IWRAM, 96K of
    /// VRAM and the cartridge ROM. Two of them as locals overflow a test
    /// thread's stack outright (`STATUS_STACK_OVERFLOW`), which is why the
    /// tests keep the machine boxed and only ever move *pointers*. The core
    /// itself carries `#[allow(clippy::large_stack_frames)] // because of Bus`
    /// on this type for the same reason.
    fn stepped_machine(steps: usize) -> Box<Gba> {
        let mut gba = Box::new(Gba::new([0u8; 0x4000], &[0u8; 0x200]));
        for _ in 0..steps {
            gba.step();
        }
        gba
    }

    /// The header is ours and carries the current version.
    #[test]
    fn the_header_identifies_the_format() {
        let machine = fresh_machine();
        let state = save(&machine.cpu).expect("a fresh machine serializes");
        assert_eq!(&state.header[..4], &SAVESTATE_MAGIC);
        assert_eq!(
            u32::from_le_bytes(state.header[4..].try_into().expect("4 bytes")),
            SAVESTATE_VERSION
        );
    }

    /// A machine that has run comes back with its registers intact.
    ///
    /// This is the test that decides the format works. It runs the machine
    /// first so the state is not all defaults — the mistake postcard would have
    /// produced is a *compile* error, and the mistake a lossy serializer makes
    /// is a silently zeroed machine, which only a non-trivial state can catch.
    #[test]
    fn a_ran_machine_comes_back_with_its_registers() {
        with_big_stack(|| {
            let machine = stepped_machine(64);
            let state = save(&machine.cpu).expect("serialize");
            let bytes: Vec<u8> = state
                .header
                .iter()
                .chain(state.payload.iter())
                .copied()
                .collect();
            let restored = load(&bytes).expect("deserialize");
            assert_eq!(
                restored.registers.program_counter(),
                machine.cpu.registers.program_counter(),
                "the program counter was lost"
            );
            for bank in 0..4usize {
                assert_eq!(
                    restored.registers.register_at(bank),
                    machine.cpu.registers.register_at(bank),
                    "r{bank} was lost"
                );
            }
        });
    }

    /// A loaded state has no hook until one is installed again.
    ///
    /// `swi_hook` is `serde(skip)`ped, so it round trips as `None`. The
    /// caller re-installs; asserting it here makes that contract explicit
    /// rather than a surprise at the first SWI after a load.
    #[test]
    fn a_loaded_state_has_no_hook_until_one_is_installed() {
        with_big_stack(|| {
            let machine = fresh_machine();
            let state = save(&machine.cpu).expect("serialize");
            let bytes: Vec<u8> = state
                .header
                .iter()
                .chain(state.payload.iter())
                .copied()
                .collect();
            let restored = load(&bytes).expect("deserialize");
            assert!(
                restored.swi_hook.is_none(),
                "a function pointer must not survive"
            );
            assert!(restored.wake_hook.is_none());
        });
    }

    /// A file that is not ours is rejected by its magic, not by a decode
    /// failure that might be mistaken for corruption.
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
}
