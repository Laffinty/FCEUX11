//! F11QA v1.8 — rom-suite runner protocol selection & fixture discovery.
//!
//! Pure helpers shared by `f11qa-rom-runner` (bin) and unit tests. Kept in
//! the lib so the routing rules are testable without spawning the C++
//! `f11qa_blargg_runner` (which needs a CMake/vcpkg build).

use std::path::{Path, PathBuf};

/// Standard blargg-style status-port protocol: after `frames` steps, `$6000`
/// reads 0x00 for PASS. Used by the NES test-ROM community (bisqwit, tepples,
/// damianyerrick, lidnariq, rainwarrior, awj, natt, nk, drag, quietust, sour,
/// 3gengames, rahsennor, flubba, …) as well as blargg's own suites.
pub const PROTO_6000: &str = "$6000";
/// kevtris nestest: CPU trace compared against a Nintendulator golden log.
pub const PROTO_NESTEST: &str = "nestest-trace";
/// Holy Mapperel: 47 mapper-identification ROMs, all must PASS.
pub const PROTO_AGGREGATE: &str = "aggregate-mapperel";

/// Default `mirror_glob` for the Holy Mapperel aggregate (kgmqa-077).
pub const HOLY_MAPPEREL_GLOB: &str = "holy_mapperel/M*.nes";

/// Every protocol the dispatcher can execute. Unknown values are rejected
/// with exit 2 rather than silently PASSed.
pub fn known_protocol(p: &str) -> bool {
    matches!(p, PROTO_6000 | PROTO_NESTEST | PROTO_AGGREGATE)
}

/// Explicit `protocol` field wins; otherwise infer from kgmqa_id. All
/// rom-suite cases default to the $6000 status-port protocol.
pub fn resolve_protocol(case: &serde_json::Value, kgmqa_id: &str) -> String {
    if let Some(p) = case.get("protocol").and_then(|v| v.as_str()) {
        return p.to_string();
    }
    if kgmqa_id.starts_with("kgmqa-048") {
        return PROTO_NESTEST.to_string();
    }
    if kgmqa_id.starts_with("kgmqa-077") {
        return PROTO_AGGREGATE.to_string();
    }
    PROTO_6000.to_string()
}

/// Extract the leaf pattern from a mirror path/glob (`holy_mapperel/M*.nes`
/// → `M*.nes`). Same basename rule as `rom_local_path`.
pub fn leaf_pattern(mirror_path_or_glob: &str) -> &str {
    mirror_path_or_glob
        .rsplit_once('/')
        .map(|x| x.1)
        .unwrap_or(mirror_path_or_glob)
}

/// List `tests/fixtures/<pattern>` where `pattern` supports a single `*`
/// wildcard (enough for `M*.nes`). Returns basenames sorted for stable order.
pub fn list_fixtures(dir: &Path, pattern: &str) -> std::io::Result<Vec<PathBuf>> {
    let (prefix, suffix) = match pattern.split_once('*') {
        Some((p, s)) => (p, s),
        None => {
            let p = dir.join(pattern);
            return Ok(if p.exists() { vec![p] } else { Vec::new() });
        }
    };
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.len() >= prefix.len() + suffix.len()
            && name.starts_with(prefix)
            && name.ends_with(suffix)
        {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explicit_protocol_wins() {
        let case = json!({"protocol": PROTO_AGGREGATE});
        assert_eq!(resolve_protocol(&case, "kgmqa-049-x"), PROTO_AGGREGATE);
    }

    #[test]
    fn infers_nestest_from_id() {
        assert_eq!(resolve_protocol(&json!({}), "kgmqa-048-nestest"), PROTO_NESTEST);
    }

    #[test]
    fn infers_aggregate_from_id() {
        assert_eq!(
            resolve_protocol(&json!({}), "kgmqa-077-holy-mapperel-tepples"),
            PROTO_AGGREGATE
        );
    }

    #[test]
    fn rom_suite_defaults_to_6000() {
        assert_eq!(resolve_protocol(&json!({}), "kgmqa-049-x"), PROTO_6000);
        assert_eq!(resolve_protocol(&json!({}), "kgmqa-112-x"), PROTO_6000);
        assert_eq!(resolve_protocol(&json!({}), "kgmqa-053-x"), PROTO_6000);
    }

    #[test]
    fn known_protocol_set_is_exactly_three() {
        assert!(known_protocol(PROTO_6000));
        assert!(known_protocol(PROTO_NESTEST));
        assert!(known_protocol(PROTO_AGGREGATE));
        assert!(!known_protocol("screen-hash"));
        assert!(!known_protocol(""));
    }

    #[test]
    fn leaf_pattern_strips_suite_dir() {
        assert_eq!(leaf_pattern("holy_mapperel/M*.nes"), "M*.nes");
        assert_eq!(leaf_pattern("nestest/nestest.nes"), "nestest.nes");
        assert_eq!(leaf_pattern("bare.nes"), "bare.nes");
    }

    #[test]
    fn list_fixtures_single_star_matches_prefix_suffix() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["M0_P32K.nes", "M1_S16K.nes", "M28_x.nes", "N1_other.nes", "M0.txt"] {
            std::fs::write(dir.path().join(name), b"x").unwrap();
        }
        let got = list_fixtures(dir.path(), "M*.nes").unwrap();
        let names: Vec<String> = got
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["M0_P32K.nes", "M1_S16K.nes", "M28_x.nes"]);
    }

    #[test]
    fn list_fixtures_literal_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("nestest.nes"), b"x").unwrap();
        let got = list_fixtures(dir.path(), "nestest.nes").unwrap();
        assert_eq!(got.len(), 1);
        assert!(list_fixtures(dir.path(), "missing.nes").unwrap().is_empty());
    }

    #[test]
    fn list_fixtures_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_fixtures(dir.path(), "M*.nes").unwrap().is_empty());
    }
}
