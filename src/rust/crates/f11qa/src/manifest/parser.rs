use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::core::{ErrorKind, QaError};
use super::schema::TestManifest;

/// Load and parse a tests.json manifest file.
/// Returns an ordered map of test-id → TestManifest.
pub fn load_manifest(path: &Path) -> Result<BTreeMap<String, TestManifest>, QaError> {
    let content = fs::read_to_string(path).map_err(|e| QaError {
        kind: ErrorKind::ManifestError,
        message: format!("Cannot read manifest '{}': {}", path.display(), e),
    })?;

    let manifest: BTreeMap<String, TestManifest> =
        serde_json::from_str(&content).map_err(|e| QaError {
            kind: ErrorKind::ManifestError,
            message: format!("Cannot parse manifest '{}': {}", path.display(), e),
        })?;

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_minimal_entry() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let json = r#"{
            "smoke_test": {
                "id": "smoke_test",
                "description": "smoke",
                "oracle_type": "A",
                "layer": "core",
                "input": { "binary": "fceux11_smoke_test" },
                "expected": { "exit_code": 0 },
                "failure_means": "blocking",
                "provenance": "v1.0"
            }
        }"#;
        tmp.as_file().write_all(json.as_bytes()).unwrap();

        let m = load_manifest(tmp.path()).unwrap();
        assert_eq!(m.len(), 1);
        let t = m.get("smoke_test").unwrap();
        assert_eq!(t.id, "smoke_test");
        assert_eq!(t.input.binary, "fceux11_smoke_test");
        assert_eq!(t.expected.exit_code, 0);
    }
}
