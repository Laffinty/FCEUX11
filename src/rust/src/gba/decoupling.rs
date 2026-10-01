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
//! | `ConsoleViewerSDL.{h,cpp}` | the GBA presentation path — its own texture and its own draw call, reached from one branch in `render()` |
//! | `state.cpp` | the savestate path — one branch in `FCEUSS_Save` / `FCEUSS_Load`, which every entry point already funnels through |
//!
//! Nothing else. When a stage adds its own touch point it adds its entries
//! **here**, explicitly and in the same commit that adds the code —
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
        // v2.0 S2-b4 stage 2' (r44 ⑤): the viewer. It needs the session flag
        // and the frame accessors, and it owns a GBA texture. Both files are
        // listed because the header carries the member as well as the cpp
        // carrying the include -- a half-registration would leave the other
        // half invisible to the guard.
        "ConsoleViewerSDL.cpp",
        "ConsoleViewerSDL.h",
        // v2.0 S3-3: the savestate path. `FCEUSS_Save` and `FCEUSS_Load` are the
        // one place every entry point already funnels through -- the ten slots,
        // Save/Load State As, F8/F9 and the hotkeys all call them -- so the GBA
        // branch goes there rather than into each caller. It is still a
        // decision that has to be written down, which is why this entry exists
        // instead of the branch being invisible.
        "state.cpp",
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

    /// Whether stripped source mentions anything of ours.
    fn mentions_gba(code: &str) -> bool {
        const MENTIONS: [&str; 5] = ["gba_", "fceu11_gba", "GbaLoad", "gba_load", "Gba"];
        MENTIONS.iter().any(|needle| code.contains(needle))
    }

    /// Files that mention GBA in any form — the ABI, the C++ wrapper symbols,
    /// the loader, the module's own header, or a GBA-specific member name.
    ///
    /// Case-sensitive and prefix-anchored, which is the whole difficulty. The
    /// first version matched a bare lowercased `"gba"` substring and reported
    /// eleven NES files that have nothing to do with this: `Format_RGBA8888`,
    /// `PRGBanks`, `logBankNumCbox`, `bookmarkPreviewPopup` and friends all
    /// contain the three letters. A guard that fires on noise gets turned off,
    /// and a turned-off guard protects nothing — so "mentions GBA" has to mean
    /// "names one of *our* symbols", which is what these five patterns are.
    ///
    /// The bare `Gba` is the one that had to be added later (r44 ⑤): stage 2'
    /// puts a `sdlGbaTexture` member in the viewer, and that name matches none
    /// of the other four. **A GBA-only member could otherwise walk straight
    /// past the guard** — the rule was not too strict, it was too narrow. Still
    /// case-sensitive, so `RGBA` / `PRGBanks` / `logBank` stay out.
    fn gba_mentioning_files() -> Vec<String> {
        stripped_sources()
            .into_iter()
            .filter(|(_, code)| mentions_gba(code))
            .map(|(name, _)| name)
            .collect()
    }

    /// The throttle and the sound device know nothing about a second machine.
    ///
    /// v2.0 S2-b4 stage 3'. Those four files carry the GBA's needs without
    /// naming it: the throttle takes a base-rate override, the sound device
    /// exposes the rate it opened at, and the GBA half of the work sits in
    /// `gba_load.cpp` and `fceuWrapper.cpp` instead. That is the decoupling
    /// claim, and **without this test it is only true because somebody read
    /// four files** — a property that erodes the first time someone reaches
    /// for the convenient thing.
    ///
    /// Checked on code with comments and literals stripped, so a file may
    /// explain itself in prose without tripping this.
    #[test]
    fn the_timing_and_sound_files_stay_gba_free() {
        const MUST_STAY_CLEAN: [&str; 4] =
            ["sdl-throttle.cpp", "throttle.h", "sdl-sound.cpp", "dface.h"];
        let sources: std::collections::HashMap<String, String> = stripped_sources().into_iter().collect();
        for name in MUST_STAY_CLEAN {
            let code = sources
                .get(name)
                .unwrap_or_else(|| panic!("{name} is not in the tree; the guard would check nothing"));
            assert!(
                !mentions_gba(code),
                "{name} refers to the GBA. That is allowed -- it just has to be registered \
                 in ALLOWED with a reason, because a second machine reaching into the timing \
                 or sound layer is the coupling invariant 9 exists to prevent."
            );
        }
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

    /// The scan finds a GBA-specific name even when nothing calls the ABI.
    ///
    /// The `Gba` rule exists because of a member called `sdlGbaTexture`: it
    /// carries no `gba_` prefix, no `fceu11_gba`, and is not `GbaLoad` or
    /// `gba_load`, so it matched none of the four earlier patterns and the
    /// viewer could hold a GBA texture without the allowlist noticing. The
    /// assertion is that the viewer is *in* the found set — which is only true
    /// because the rule catches it.
    #[test]
    fn a_gba_specific_name_is_enough_to_need_the_allowlist() {
        let found = gba_mentioning_files();
        for viewer in ["ConsoleViewerSDL.cpp", "ConsoleViewerSDL.h"] {
            assert!(
                found.iter().any(|name| name == viewer),
                "{viewer} holds GBA state but the scan did not find it -- the rule is \
                 too narrow again, and a GBA member is walking past the guard"
            );
        }
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

    /// The GBA branch in the savestate path stays where it was put.
    ///
    /// `state.cpp` is the NES savestate implementation, and the whole argument
    /// for S3-3's branch living there is that `FCEUSS_Save` and `FCEUSS_Load`
    /// are the one pair every entry point already funnels through — the ten
    /// slots, Save/Load State As and F8/F9 all call them and nothing else. That
    /// argument holds only while the GBA mentions stay inside those two
    /// functions. A `gba_active()` check added to the movie backup, the undo
    /// bookkeeping or the slot viewer would still be a plausible-looking edit,
    /// would still compile, and would still pass the allowlist guard — while
    /// quietly putting the second machine inside NES code that has no reason to
    /// know about it.
    ///
    /// **The assertion is two-sided on purpose.** "No mention outside those two
    /// functions" is satisfied perfectly well by a file with no mentions at
    /// all, so this also asserts the branch is still there. A guard that goes
    /// green when the thing it guards has been deleted is not a guard.
    ///
    /// **What it does not catch**, stated rather than implied: it is a
    /// function-level check, so a GBA test added *inside* `FCEUSS_Load` next
    /// to the branch still passes. The two functions are already allowed to
    /// know about the GBA; the claim is about where the file does, not about
    /// every line within it.
    #[test]
    fn the_savestate_branch_is_the_only_gba_in_the_state_file() {
        const BRANCH_HOSTS: [&str; 2] = ["FCEUSS_Save", "FCEUSS_Load"];

        let sources: std::collections::HashMap<String, String> =
            stripped_sources().into_iter().collect();
        let code = sources.get("state.cpp").unwrap_or_else(|| {
            panic!("state.cpp is not in the tree; the guard would check nothing")
        });

        assert!(
            mentions_gba(code),
            "state.cpp no longer mentions GBA at all -- either the S3-3 branch was removed, \
             or it was renamed past the point where the scan recognises it. The savestate path \
             would then hand a NES state to a GBA machine, and the assertion below would pass \
             for the wrong reason."
        );

        // The function a line belongs to, taken as the last column-zero definition
        // above it. A line that opens no parentheses, starts with `#`, or ends in
        // `;` is a statement rather than a header, and is not treated as one.
        // A mention that no header accounts for is attributed to the empty string
        // and fails, which is the direction a mistake in this heuristic has to go.
        let mut current = String::new();
        let mut outside: Vec<(usize, String)> = Vec::new();
        for (index, line) in code.lines().enumerate() {
            if !line.starts_with(char::is_whitespace)
                && !line.starts_with('#')
                && line.contains('(')
                && !line.trim_end().ends_with(';')
            {
                if let Some(open) = line.find('(') {
                    let name = line[..open]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or_default();
                    if !name.is_empty() {
                        current = name.to_owned();
                    }
                }
            }
            if mentions_gba(line) && !BRANCH_HOSTS.contains(&current.as_str()) {
                outside.push((index + 1, current.clone()));
            }
        }

        assert!(
            outside.is_empty(),
            "state.cpp mentions GBA outside FCEUSS_Save / FCEUSS_Load, at {outside:?} \
             (line, enclosing function). The branch belongs in those two because they are \
             the single point every savestate entry point already passes through; anywhere \
             else it puts the second machine into NES code with no reason to know about it."
        );
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
