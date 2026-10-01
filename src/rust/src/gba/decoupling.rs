//! The decoupling guards (v2.0 S2-b4, plan invariant 9).
//!
//! # Why these are tests and not a paragraph
//!
//! The project rule, in the user's words on 2026-10-01: the GBA module plugs
//! into **no** quality-detection system, stays Alpha for the foreseeable
//! future, does not block FCEUX11 proper, and stays **as decoupled from the NES
//! side as it can be**. A paragraph in the plan is the right place to state
//! that and the wrong place to enforce it — paragraphs get read once, and the
//! cost of breaking them is a reviewer's memory.
//!
//! So the rule is written twice: once as invariant 9, which says what we mean,
//! and once as this file, which goes red. Two things are checked, and they are
//! deliberately different checks rather than one stronger one:
//!
//! 1. **The C ABI has exactly one C++ caller.** The GBA core is reached through
//!    `gba_*` functions. Every C++ file that names one is a file that now
//!    depends on the GBA ABI, and the set is allowed to be exactly one.
//! 2. **Every C++ file that mentions GBA at all is on a written list.** Looser
//!    than the first, so it catches the ways GBA spreads that do not go through
//!    the ABI — a new `if (gba_active())` in a core file, a GBA case in a
//!    palette function, an include of `gba_load.h` from somewhere new.
//!
//! # The allowlist, and why it is not shorter
//!
//! Every entry is a place where GBA has to be visible, with the reason it could
//! not be avoided:
//!
//! | file | why |
//! |---|---|
//! | `gba_load.{h,cpp}` | the module itself: the only C ABI caller, and the owner of the session state |
//! | `fceuWrapper.cpp` | the per-frame branch — one process, one emulation thread, one frame loop |
//! | `fceu.cpp` | the loader chain — one "open this file" entry point |
//! | `ConsoleFile.cpp` | the open dialog's filter list, so a `.gba` can be picked |
//!
//! Nothing else. When stage 2' adds its own presentation path it adds its
//! entries **here**, explicitly and in the same commit that adds the code —
//! which is the whole point: a new touch point has to be a decision rather than
//! a consequence.
//!
//! # Why the source is stripped before matching
//!
//! The first version of this file matched raw text, and it was wrong in a way
//! that would have made it useless. A log line or a comment that mentions
//! `gba_` would trip it, and the cheapest way to make a test pass is then to
//! reword the comment — at which point the guard is measuring the author's
//! willingness to reword comments. Comments and string literals are removed
//! first, so what is matched is code.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    /// Where the C++ tree lives, relative to this crate.
    ///
    /// `src/rust` (this crate) and the C++ sources (`src/*.cpp`,
    /// `src/drivers/**`, ...) are siblings, so the crate's parent is `src`.
    fn cxx_tree() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the crate lives inside src/, so it has a parent")
            .to_path_buf()
    }

    /// Files allowed to mention GBA at all. See the module docs for each entry.
    const ALLOWED: &[&str] = &[
        "gba_load.cpp",
        "gba_load.h",
        "fceuWrapper.cpp",
        "fceu.cpp",
        "ConsoleFile.cpp",
    ];

    /// The one file allowed to call the GBA C ABI.
    const SOLE_ABI_CALLER: &str = "gba_load.cpp";

    /// Remove `//` line comments, `/* */` block comments and `"` / `'` string
    /// literals, replacing each with a space so byte offsets do not shift.
    ///
    /// Deliberately simple rather than correct: it does not know about raw
    /// strings, line continuations inside literals, or a `/` that is not a
    /// comment. Those cases would have to be handled for a C++ parser, and the
    /// failure mode of getting one wrong is a missed mention — the guard then
    /// sees slightly more code than it should, which is the safe direction.
    ///
    /// Works on bytes, not `char`s, and the first version of this function did
    /// not: `byte as char` turns every UTF-8 continuation byte into a
    /// *different* multi-byte character, so the string's length changes and
    /// every position after the first non-ASCII comment shifts. The visible
    /// symptom was eleven NES files reported as mentioning GBA, none of which
    /// do — the guard was matching against mangled text. A guard that fires on
    /// noise gets disabled, which is worse than not having it.
    fn strip_comments_and_literals(source: &str) -> String {
        let bytes = source.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(source.len());
        let mut index = 0usize;
        while index < bytes.len() {
            match bytes[index] {
                b'/' if bytes.get(index + 1) == Some(&b'/') => {
                    while index < bytes.len() && bytes[index] != b'\n' {
                        out.push(b' ');
                        index += 1;
                    }
                }
                b'/' if bytes.get(index + 1) == Some(&b'*') => {
                    let mut previous = b' ';
                    while index < bytes.len() {
                        let current = bytes[index];
                        if previous == b'*' && current == b'/' {
                            out.push(b' ');
                            index += 1;
                            break;
                        }
                        out.push(if current == b'\n' { b'\n' } else { b' ' });
                        previous = current;
                        index += 1;
                    }
                }
                b'"' | b'\'' => {
                    let quote = bytes[index];
                    out.push(b' ');
                    index += 1;
                    while index < bytes.len() {
                        let current = bytes[index];
                        out.push(if current == b'\n' { b'\n' } else { b' ' });
                        index += 1;
                        if current == b'\\' && index < bytes.len() {
                            out.push(b' ');
                            index += 1;
                            continue;
                        }
                        if current == quote {
                            break;
                        }
                    }
                }
                other => {
                    out.push(other);
                    index += 1;
                }
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    /// Every C/C++ source under `src/`, excluding the Rust tree, as
    /// (file name, path) pairs.
    fn cxx_sources() -> Vec<(String, PathBuf)> {
        fn walk(root: &Path, out: &mut Vec<(String, PathBuf)>) {
            let Ok(entries) = std::fs::read_dir(root) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // The Rust crate is a sibling of the C++ sources, not part of
                    // them, and it mentions GBA everywhere by design.
                    if path.file_name().is_some_and(|name| name == "rust") {
                        continue;
                    }
                    walk(&path, out);
                } else if path
                    .extension()
                    .is_some_and(|ext| ext == "cpp" || ext == "h" || ext == "hpp" || ext == "c")
                {
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    out.push((name, path));
                }
            }
        }
        let mut out = Vec::new();
        walk(&cxx_tree(), &mut out);
        out.sort();
        out
    }

    /// The sources, with comments and literals removed.
    fn stripped_sources() -> Vec<(String, String)> {
        cxx_sources()
            .into_iter()
            .map(|(name, path)| {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                (name, strip_comments_and_literals(&text))
            })
            .collect()
    }

    /// Call sites of the GBA C ABI, by file.
    ///
    /// The anchor is `gba_` at a word boundary, which is what keeps the C++
    /// side's own `fceu11_gba_*` symbols out of the result: in
    /// `fceu11_gba_step_frame` the character before `gba` is `_`, a word
    /// character, so there is no boundary there. A substring match would report
    /// the module's own wrapper functions as ABI calls.
    fn abi_callers() -> Vec<String> {
        stripped_sources()
            .into_iter()
            .filter(|(_, code)| {
                code.split_inclusive(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|token| token.starts_with("gba_") && token.contains('('))
            })
            .map(|(name, _)| name)
            .collect()
    }

    /// Files that mention GBA in any form — the ABI, the C++ wrapper symbols,
    /// the loader, or the module's own header.
    ///
    /// Case-sensitive and prefix-anchored, which is the whole difficulty. The
    /// first version matched a bare lowercased `"gba"` substring and reported
    /// eleven NES files that have nothing to do with this: `Format_RGBA8888`,
    /// `PRGBanks`, `logBankNumCbox`, `bookmarkPreviewPopup` and friends all
    /// contain the three letters. A guard that fires on noise gets turned off,
    /// and a turned-off guard protects nothing — so "mentions GBA" has to mean
    /// "names one of *our* symbols", which is what these four prefixes are.
    fn gba_mentioning_files() -> Vec<String> {
        const MENTIONS: [&str; 4] = ["gba_", "fceu11_gba", "GbaLoad", "gba_load"];
        stripped_sources()
            .into_iter()
            .filter(|(_, code)| MENTIONS.iter().any(|needle| code.contains(needle)))
            .map(|(name, _)| name)
            .collect()
    }

    /// The C ABI is called from exactly one C++ file.
    ///
    /// Every additional caller is a file that now cannot be built or reasoned
    /// about without the GBA module, which is what invariant 9 forbids. The
    /// module's own file is the one place allowed to know the ABI exists.
    #[test]
    fn the_c_abi_is_called_from_exactly_one_cpp_file() {
        let callers = abi_callers();
        assert_eq!(
            callers,
            vec![SOLE_ABI_CALLER.to_owned()],
            "the GBA C ABI is called from {:?}; invariant 9 allows it in exactly one \
             file, and every extra caller is a new dependency on a module that must \
             not block the NES build",
            callers
        );
    }

    /// Every C++ file that mentions GBA is on the written list.
    ///
    /// The looser of the two checks on purpose: the one above cannot see GBA
    /// spreading through the C++ side's own symbols, through a new include, or
    /// through a bare mention in a core file. This one can.
    #[test]
    fn gba_references_stay_inside_the_allowlist() {
        let found = gba_mentioning_files();
        let unexpected: Vec<&String> = found
            .iter()
            .filter(|name| !ALLOWED.contains(&name.as_str()))
            .collect();
        assert!(
            unexpected.is_empty(),
            "{unexpected:?} mention GBA but are not on the allowlist {ALLOWED:?}. \
             Invariant 9 wants each touch point to be a decision: add the file there \
             with a reason, in the same commit that adds the reference."
        );
    }

    /// The allowlist is not stale: every name in it is still a real file.
    ///
    /// A guard whose own list has drifted is worse than no guard, because it
    /// reads as coverage. A renamed or deleted file would otherwise just stop
    /// being checked, silently.
    #[test]
    fn the_allowlist_has_no_dead_entries() {
        let present: Vec<String> = stripped_sources().into_iter().map(|(name, _)| name).collect();
        for name in ALLOWED {
            assert!(
                present.iter().any(|found| found == name),
                "{name} is on the GBA allowlist but no such file exists -- a stale \
                 entry makes this guard look like coverage while checking nothing"
            );
        }
    }

    /// The two guards are looking at a real tree.
    ///
    /// `cxx_sources()` returns nothing rather than failing on a missing
    /// directory, which would make every assertion above pass for the wrong
    /// reason — the shape of failure r36 had to fix in `drift_guard`.
    #[test]
    fn the_scan_finds_the_cxx_tree() {
        let sources = cxx_sources();
        assert!(
            sources.len() > 50,
            "only {} C/C++ sources found under {}; the scan is not looking at the \
             real tree, so every guard above would pass for the wrong reason",
            sources.len(),
            cxx_tree().display()
        );
    }
}
