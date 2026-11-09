//! The three guards that keep the S1-a fixture set from rotting.
//!
//! The nine fixtures of S1-a are the **only** record that the nine defects
//! are still defects. They rot in three ways, and each way has a guard:
//!
//! | guard | what it stops |
//! |---|---|
//! | `g1` | a reproducer that ignores itself **without naming the work item** that will turn it back into a permanent lock test — an ignore nobody owns is an ignore nobody will ever remove |
//! | `g2` | a fixture that is **renamed or deleted** — the set has to stay exactly the one the plan asks for |
//! | `g4` | a reproducer whose `#[ignore]` is **removed** (which would turn a red reproducer into a green permanent test that hides the defect) or whose **ownership string** drifts from the plan |
//!
//! # How the guards read the fixtures
//!
//! The fixtures live in four modules next to this one, one per defect
//! domain. The guards read their **source text** with `include_str!` rather
//! than importing them, because the thing being guarded is the *shape* of the
//! source — an `#[ignore]` attribute and the test's name — and importing a
//! test module does not give you its attributes.
//!
//! Reading the source also means the guard fails **loudly and early** when a
//! fixture is renamed: the name simply is not in the set any more.

use std::collections::BTreeSet;

/// The four domain modules, as source text.
const SWI: &str = include_str!("swi.rs");
const OAM: &str = include_str!("oam.rs");
const BACKUP: &str = include_str!("backup.rs");
const ROM: &str = include_str!("rom.rs");

/// Every fixture the plan asks for, with the work item that owns it.
///
/// The owners are the `归属` column of the v2.0.3 plan's §5 table (and of
/// the S1-a plan's §5). The seven red ones are **reproducers**: they carry
/// `#[ignore]` and must be red today. The last two are **green locks**: they
/// assert the behaviour that is already correct and exist so that the S2/S3
/// fixes cannot break it.
const PLANNED: &[(&str, &[&str])] = &[
    ("swi_unimpl_1a", &["S2-b"]),
    ("swi_unimpl_16", &["S2-b", "S3-a"]),
    ("oam_bad_shape", &["S2-a"]),
    ("oam_gfx_mode3", &["S2-a"]),
    ("eeprom_chip_capacity", &["S3-b"]),
    ("flash_status_reg", &["S3-b"]),
    ("rom_window_mirror", &["S3-d"]),
    ("eeprom_protocol_lock", &["S3-b"]),
    ("gpio_read_before_write", &["S3-c"]),
];

/// The ones that must be red today, and only those. The two OAM reproducers
/// (`oam_bad_shape` / `oam_gfx_mode3`) left this list on S2-a (v2.0.3 r7):
/// their defect is fixed, their `#[ignore]` is gone, and they are resident
/// green locks now — which is exactly what G4 must never see (a reproducer
/// that lost its `#[ignore` without a fix).
const REPRODUCERS: &[&str] = &[
    "swi_unimpl_1a",
    "swi_unimpl_16",
    "eeprom_chip_capacity",
    "flash_status_reg",
    "rom_window_mirror",
];

/// Every `#[test]` function in the four domain modules, with the `#[ignore]`
/// reason it carries, if any.
///
/// A reason of `Some("")` means a **bare** `#[ignore]` — the test is ignored
/// but says nothing about why.
fn fixtures() -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for source in [SWI, OAM, BACKUP, ROM] {
        let mut ignore: Option<String> = None;
        let mut is_test = false;
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#[ignore") {
                ignore = Some(
                    trimmed
                        .split_once('"')
                        .and_then(|(_, rest)| rest.split_once('"'))
                        .map(|(reason, _)| reason.to_string())
                        .unwrap_or_default(),
                );
                continue;
            }
            if trimmed.starts_with("#[test]") {
                is_test = true;
                continue;
            }
            let head = trimmed
                .strip_prefix("fn ")
                .or_else(|| trimmed.strip_prefix("pub fn "));
            if let Some(head) = head {
                let name = head.split('(').next().unwrap_or("").trim();
                if is_test {
                    out.push((name.to_string(), ignore.take()));
                }
                is_test = false;
                ignore = None;
            }
        }
    }
    out
}

/// The ignored ones, with their reason.
fn ignored() -> Vec<(String, String)> {
    fixtures()
        .into_iter()
        .filter_map(|(name, reason)| reason.map(|reason| (name, reason)))
        .collect()
}

/// Every ignored fixture must name the work item that will fix it.
///
/// An `#[ignore]` with an empty reason — or one that does not name a work
/// item — is an **orphan**: nobody knows which task is supposed to turn it
/// back into a permanent lock test, so it stays ignored forever and the
/// defect it watches stays unfixed and unwatched.
#[test]
fn g1_every_ignored_fixture_names_the_work_item_that_will_fix_it() {
    let orphans: Vec<String> = ignored()
        .iter()
        .filter(|(_, reason)| !reason.contains("S2-") && !reason.contains("S3-"))
        .map(|(name, reason)| format!("{name} ({reason:?})"))
        .collect();
    assert!(
        orphans.is_empty(),
        "these fixtures ignore themselves without naming the work item that will \
         turn them back into permanent lock tests: {orphans:?}"
    );
}

/// The fixture set must be exactly the planned one — nothing missing, nothing
/// extra.
///
/// A missing fixture means a defect lost its reproducer; an extra one means
/// somebody added a test that the plan never asked for and nobody reviews.
#[test]
fn g2_the_fixture_set_is_exactly_the_one_the_plan_asks_for() {
    let found: BTreeSet<String> = fixtures().into_iter().map(|(name, _)| name).collect();
    let planned: BTreeSet<String> = PLANNED
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect();
    assert_eq!(
        found, planned,
        "the fixture set drifted from the plan's §5 table; \
         missing: {:?}, unexpected: {:?}",
        planned.difference(&found).collect::<Vec<_>>(),
        found.difference(&planned).collect::<Vec<_>>(),
    );
}

/// Every reproducer must still be there, still ignored, and still owned by
/// the work item the plan names.
///
/// Losing the `#[ignore]` is the dangerous direction: the reproducer would
/// become a permanent test that asserts the **target** behaviour, so it would
/// be red for ever — or, worse, somebody "fixes" it by asserting the
/// defect's current behaviour, and the defect is now green and unwatched.
#[test]
fn g4_every_reproducer_is_still_there_and_still_owned() {
    let all = fixtures();
    let ignored_names: BTreeSet<String> = ignored().into_iter().map(|(name, _)| name).collect();

    let missing: Vec<&str> = REPRODUCERS
        .iter()
        .copied()
        .filter(|name| !all.iter().any(|(found, _)| found == name))
        .collect();
    assert!(
        missing.is_empty(),
        "these reproducers are gone: {missing:?}"
    );

    let not_ignored: Vec<&str> = REPRODUCERS
        .iter()
        .copied()
        .filter(|name| !ignored_names.contains(*name))
        .collect();
    assert!(
        not_ignored.is_empty(),
        "these reproducers no longer carry #[ignore] -- a red reproducer that is \
         not ignored either blocks the gate or gets \"fixed\" by asserting the \
         defect's current behaviour: {not_ignored:?}"
    );

    let extra: Vec<&str> = ignored_names
        .iter()
        .map(String::as_str)
        .filter(|name| !REPRODUCERS.contains(name))
        .collect();
    assert!(
        extra.is_empty(),
        "these fixtures are ignored but are not reproducers of the plan's §5 table: {extra:?}"
    );

    for (name, owners) in PLANNED {
        if !REPRODUCERS.contains(name) {
            continue;
        }
        let reason = all
            .iter()
            .find(|(found, _)| found == name)
            .and_then(|(_, reason)| reason.clone())
            .unwrap_or_default();
        for owner in *owners {
            assert!(
                reason.contains(owner),
                "the reproducer `{name}` must name `{owner}` in its #[ignore] \
                 reason (the plan's §5 table assigns it there); it says {reason:?}"
            );
        }
    }
}
