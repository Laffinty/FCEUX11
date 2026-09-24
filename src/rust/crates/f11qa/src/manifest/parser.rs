use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::core::{ErrorKind, QaError};
use super::schema::TestManifest;

/// F11QA v1.8 §三 §3.6: tests.json v1.8 形状
///
/// ```json
/// {
///   "schema_version": "1.8",
///   "cases": [ { TestManifest }, ... ]
/// }
/// ```
///
/// v1.17 旧形状是 `{id: TestManifest}` 平铺，Phase 7.1 起弃用。
#[derive(Debug, Deserialize)]
struct ManifestV1_8 {
    pub schema_version: String,
    pub cases: Vec<TestManifest>,
}

/// Load and parse a tests.json manifest file (F11QA v1.8 schema).
/// Returns an ordered map of `kgmqa_id` → `TestManifest`.
pub fn load_manifest(path: &Path) -> Result<BTreeMap<String, TestManifest>, QaError> {
    let content = fs::read_to_string(path).map_err(|e| QaError {
        kind: ErrorKind::ManifestError,
        message: format!("Cannot read manifest '{}': {}", path.display(), e),
    })?;

    let manifest: ManifestV1_8 = serde_json::from_str(&content).map_err(|e| QaError {
        kind: ErrorKind::ManifestError,
        message: format!(
            "Cannot parse manifest '{}': {}\n  hint: tests.json must be F11QA v1.8 schema \
             ({{\"schema_version\": \"1.8\", \"cases\": [...]}}); see \
             docs/plans/FCEUX11-v1.8_F11QA-构建计划.md §六",
            path.display(),
            e
        ),
    })?;

    if manifest.schema_version != "1.8" {
        return Err(QaError {
            kind: ErrorKind::ManifestError,
            message: format!(
                "tests.json schema_version is '{}', expected '1.8'",
                manifest.schema_version
            ),
        });
    }

    let mut map: BTreeMap<String, TestManifest> = BTreeMap::new();
    for mut c in manifest.cases {
        // 把 v1.17 `id` 别名填到 `legacy_id`（serde alias 已把 v1.17 `id` 读到 kgmqa_id）。
        // 如果 v1.8 形态（带 kgmqa_id 显式字段），legacy_id 保持 None。
        if c.legacy_id.is_none() && c.kgmqa_id.starts_with("kgmqa-") {
            // v1.8 形态已有 kgmqa_id，legacy_id 留空
            c.legacy_id = None;
        } else if c.legacy_id.is_none() {
            // 既无 legacy_id 也无 kgmqa- 前缀：可能是旧 v1.17 `id` 通过 alias 进来
            c.legacy_id = Some(c.kgmqa_id.clone());
        }
        if let Some(prev) = map.get(&c.kgmqa_id) {
            return Err(QaError {
                kind: ErrorKind::ManifestError,
                message: format!(
                    "duplicate kgmqa_id '{}' in manifest (previous: legacy_id={:?})",
                    c.kgmqa_id,
                    prev.legacy_id
                ),
            });
        }
        map.insert(c.kgmqa_id.clone(), c);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_v1_8_minimal_entry() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let json = r#"{
            "schema_version": "1.8",
            "cases": [
                {
                    "kgmqa_id": "kgmqa-001-smoke",
                    "title": "smoke test",
                    "kind": ["smoke"],
                    "oracle_type": "A",
                    "layer": "core",
                    "input": { "binary": "fceux11_smoke_test" },
                    "expected": { "exit_code": 0 },
                    "failure_means": "blocking",
                    "provenance": "v1.8"
                }
            ]
        }"#;
        tmp.as_file().write_all(json.as_bytes()).unwrap();

        let m = load_manifest(tmp.path()).unwrap();
        assert_eq!(m.len(), 1);
        let t = m.get("kgmqa-001-smoke").unwrap();
        assert_eq!(t.kgmqa_id, "kgmqa-001-smoke");
        assert_eq!(t.title, "smoke test");
        assert_eq!(t.kind, vec!["smoke"]);
        assert_eq!(t.input.binary, "fceux11_smoke_test");
        assert_eq!(t.expected.exit_code, 0);
    }

    #[test]
    fn parse_v1_8_with_rom_suite_fields() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let json = r#"{
            "schema_version": "1.8",
            "cases": [
                {
                    "kgmqa_id": "kgmqa-031-blargg-smoke",
                    "legacy_id": "blargg_smoke",
                    "title": "blargg smoke",
                    "kind": ["rom-suite", "smoke"],
                    "oracle_type": "B",
                    "layer": "core",
                    "input": { "binary": "f11qa-rom-runner" },
                    "expected": { "exit_code": 0 },
                    "timeout_seconds": 60,
                    "tags": ["blargg"],
                    "failure_means": "blocking",
                    "provenance": "v1.8 mirror",
                    "vendor_state": "vendored",
                    "spec_source": "internal",
                    "license": "GPL-2.0",
                    "mirror_ref": "v1.8.0-mirror",
                    "mirror_path": "nestest/nestest.nes"
                }
            ]
        }"#;
        tmp.as_file().write_all(json.as_bytes()).unwrap();

        let m = load_manifest(tmp.path()).unwrap();
        let t = m.get("kgmqa-031-blargg-smoke").unwrap();
        assert_eq!(t.legacy_id.as_deref(), Some("blargg_smoke"));
        assert_eq!(t.vendor_state.as_deref(), Some("vendored"));
        assert_eq!(t.license.as_deref(), Some("GPL-2.0"));
        assert_eq!(t.mirror_ref.as_deref(), Some("v1.8.0-mirror"));
        assert_eq!(t.mirror_path.as_deref(), Some("nestest/nestest.nes"));
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let json = r#"{"schema_version": "1.7", "cases": []}"#;
        tmp.as_file().write_all(json.as_bytes()).unwrap();
        let err = load_manifest(tmp.path()).unwrap_err();
        assert!(
            err.message.contains("schema_version"),
            "expected schema_version error, got: {}",
            err.message
        );
    }

    #[test]
    fn rejects_missing_schema_version() {
        // v1.17 旧形状 → 应该报错（明确 schema_version 必须）
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let json = r#"{"smoke_test": {"id": "smoke_test"}}"#;
        tmp.as_file().write_all(json.as_bytes()).unwrap();
        let err = load_manifest(tmp.path()).unwrap_err();
        assert!(
            err.message.contains("Cannot parse manifest"),
            "expected parse error, got: {}",
            err.message
        );
    }

    #[test]
    fn rejects_duplicate_kgmqa_id() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let json = r#"{
            "schema_version": "1.8",
            "cases": [
                { "kgmqa_id": "kgmqa-001-dup", "title": "x", "oracle_type": "A", "layer": "core",
                  "input": { "binary": "x" }, "expected": { "exit_code": 0 }, "failure_means": "blocking", "provenance": "x" },
                { "kgmqa_id": "kgmqa-001-dup", "title": "y", "oracle_type": "A", "layer": "core",
                  "input": { "binary": "y" }, "expected": { "exit_code": 0 }, "failure_means": "blocking", "provenance": "y" }
            ]
        }"#;
        tmp.as_file().write_all(json.as_bytes()).unwrap();
        let err = load_manifest(tmp.path()).unwrap_err();
        assert!(
            err.message.contains("duplicate kgmqa_id"),
            "expected duplicate error, got: {}",
            err.message
        );
    }
}