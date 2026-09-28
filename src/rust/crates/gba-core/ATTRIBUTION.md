# ATTRIBUTION — GBAEUX11

> This file is the single authoritative record of the GBA hardware core's
> provenance and of every local modification applied to it.
> Referenced by the v2.0 build plan (section 11 "状态回写" item 2, and
> section 3 "许可与合规"). Directory / crate / module names are functional
> (`f11gba`, `gba-core`) and deliberately carry no upstream identifiers;
> this file, plus the copied `LICENSE`, is where the attribution lives.

## 1. Upstream

| Field | Value |
|---|---|
| Project | **clementine** |
| Repository | `https://github.com/RIP-Comm/clementine` |
| Crate taken | `emu/` — the hardware core only |
| Baseline commit | `ee77922dd293b70e945458e104f3b2de794f0151` (`main`) |
| Upstream copyright | **Copyright (c) 2024 RIPsters** |
| Licence | **MIT** |
| Vendored as | `src/rust/crates/gba-core/` (crate name `gba-core`) |
| Licence text | `src/rust/crates/gba-core/LICENSE` (verbatim copy of the upstream root `LICENSE`) |

### What was taken, and what was discarded

| Upstream path | Disposition |
|---|---|
| `emu/src/**` (45 files) | **taken** → `crates/gba-core/src/**` |
| `emu/Cargo.toml` | taken, then rewritten (see §3) |
| `emu/tests/jsmolka.rs` (112 lines) | **not vendored** — it is the upstream test harness, not core code. FCEUX11 runs its own jsmolka integration (plan section 7.5). |
| `ui/**` (egui debugger) | **discarded** — plan section 2.0 |
| `src/main.rs` (CLI) | **discarded** — plan section 2.0 |
| `LICENSE` | **taken** → `crates/gba-core/LICENSE` |

## 2. Per-file headers

**Do not add or strip upstream file headers.** The measured situation is:

- of the 45 vendored sources, **44 carry no copyright or licence line at all**;
- the single exception is `src/cpu/thumb/alu_instructions.rs`.

The MIT attribution duty is therefore discharged by **`crates/gba-core/LICENSE` +
this file + shipping the licence text with the release artefact**, not by
per-file headers. Inventing headers on the other 44 files would misattribute
material that upstream never marked.

> This corrects a wording in the v2.0 plan section 3, which said the duty is
> discharged by "保留上游文件头与版权行". The *intent* — do not remove what
> upstream had — holds; the *mechanism* is this file plus the LICENSE copy.

## 3. Local modifications to the vendored tree

### 3.1 `Cargo.toml` — rewritten

The upstream `emu/Cargo.toml` took these from **its own** workspace root via
inheritance:

| Field | Upstream | Here | Why |
|---|---|---|---|
| `edition` | `edition.workspace` | `edition = "2024"` | Same value; FCEUX11's `[workspace.package] edition` is also 2024, so behaviour is identical, but spelled out to keep the vendored crate self-describing. |
| `repository` | `repository.workspace` | `repository = "https://github.com/RIP-Comm/clementine"` | FCEUX11's workspace does not define this field at all — inheritance would fail to resolve. |
| `license` | `license.workspace` | `license = "MIT"` | Same reason. |
| `serde` | `serde.workspace` | `serde = { version = "1", features = ["derive"] }` | FCEUX11 has no `[workspace.dependencies]` table; the `derive` feature was what upstream's workspace supplied, so it is spelled out. |
| `tracing` | `tracing.workspace` | `tracing = "0.1"` | Same reason. |
| `serde_with` | `serde_with = "3"` | unchanged | already explicit upstream |
| `rtrb` | `rtrb = "0.3"` | unchanged | already explicit upstream |
| `rand` | `rand = { workspace = true, optional = true }` | `rand = { version = "0.8", optional = true }` | Same reason; kept optional. |

`[lints.clippy]` was carried over unchanged.

### 3.2 Source files — 5 local changes, all S0' extension-point wiring

`crates/gba-core/src/**` was byte-identical to upstream `emu/src/**` at the end
of **S0** (verified file-by-file with SHA-256). Local edits begin at **S0'**, when
the core was given the hook it needs before any SWI can be supplied from
outside — `handle_swi_hle` is a private method, so there is no other way.

All five are in `src/cpu/arm7tdmi.rs`:

| # | Change | Why |
|---|--------|-----|
| 1 | Added `pub type SwiHook = fn(&mut Arm7tdmi, u32, Psr, u32) -> bool;` | Lets the embedder service SWI 0x00-0x2A. A plain `fn` pointer, not a trait object, keeps the dependency one-directional: the embedder calls this crate, never the reverse. |
| 2 | Added `#[serde(skip)] pub swi_hook: Option<SwiHook>` on `Arm7tdmi` | The field itself. `serde(skip)` because a function pointer has no meaning to restore, and loading a savestate must not depend on it. |
| 3 | `impl Default for Arm7tdmi` initialises it to `None` | Required by the manual `Default` impl. |
| 4 | `handle_swi_hle` consults the hook before its own `match` | The seam. The hook gets first refusal, so it can both fill gaps and override the arms that exist but are shells. |
| 5 | `swi_return` made `pub` | Without it a hook could service a call but not return from it. |

No upstream logic was altered: the existing `match` arms, the `_ => false`
fallback, and every other function body are untouched.

> Upstream line references (for a future re-vendor check): struct at
> `cpu/arm7tdmi.rs:144`, `Default` at `:217`, hook dispatch site
> `handle_swi_hle` at `:1044`, `swi_return` at `:1327`.

### 3.3 `src/gba/` — new code, no upstream content

`src/rust/src/gba/` is entirely first-party: the SWI implementations and the
C ABI. It is a plain **module of the root crate**, not a `crates/f11gba`
crate — see the v2.0 plan's R14 and the r8 entry for why a separate crate does
not build on rustc 1.96 fat LTO. Its C ABI surface is specified in the v2.0
plan section 4.1; S0' exposes the probe surface only, with the real surface
landing in S2.

## 4. Patch-set size

| Item | Status |
|---|---|
| `gba-core` source changes | **5**, all S0' hook wiring in one file (`src/cpu/arm7tdmi.rs`); no upstream logic altered |
| `gba-core/Cargo.toml` | rewritten (metadata only, no version changes) |
| `src/gba/` | new, 100% first-party, 5 files |

The plan's risk **R1** says "keep the patch set minimal (SWI hook only)". Five
additive edits in a single file, none of which change upstream behaviour, is
the tightest position that is still reachable — the alternative (implementing
SWI inside this crate) would be a far larger divergence.

Every change is `#[serde(skip)]`-safe and therefore invisible to savestates.

## 5. Update procedure

When re-vendoring from a newer upstream commit, update in this order:

1. `Baseline commit` in §1 and in plan section 2.0.
2. `crates/gba-core/src/**` and `crates/gba-core/LICENSE`.
3. Re-run §3.1: if upstream's workspace still supplies different dependency
   versions, reconcile against `Cargo.lock` and record the delta here.
4. Re-check §2 (per-file header count) — if a future upstream revision adds
   headers to more files, note it so the rule in §2 stays accurate.
5. Re-run the plan's S0 exit criteria: `cargo check`, the staticlib symbol
   check, and the NES zero-regression gate.
