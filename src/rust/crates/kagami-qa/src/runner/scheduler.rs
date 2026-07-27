use std::collections::BTreeMap;

use crate::adapter::trait_def::{SutAdapter, TestResult};
use crate::core::QaConfig;
use crate::manifest::schema::TestManifest;

/// Holds manifest + config, drives test execution through an adapter.
pub struct TestScheduler {
    #[allow(dead_code)]
    config: QaConfig,
    manifest: BTreeMap<String, TestManifest>,
}

impl TestScheduler {
    pub fn new(config: QaConfig, manifest: BTreeMap<String, TestManifest>) -> Self {
        Self { config, manifest }
    }

    /// Run all tests in the manifest, returning results in insertion order.
    pub fn run_all(&self, adapter: &dyn SutAdapter) -> Vec<TestResult> {
        let mut results = Vec::with_capacity(self.manifest.len());

        for (_id, test) in &self.manifest {
            match adapter.run_test(test) {
                Ok(result) => results.push(result),
                Err(e) => results.push(TestResult {
                    test_id: test.id.clone(),
                    passed: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: format!("Adapter error: {}", e.message),
                    duration_ms: 0,
                    migration_note: Some(format!("setup_error: {}", e.message)),
                }),
            }
        }

        results
    }

    /// Return the number of tests in the manifest.
    pub fn len(&self) -> usize {
        self.manifest.len()
    }

    /// Return true if the manifest is empty.
    pub fn is_empty(&self) -> bool {
        self.manifest.is_empty()
    }

    /// Return a snapshot of the manifest for drift detection.
    pub fn manifest_snapshot(&self) -> BTreeMap<String, crate::manifest::schema::TestManifest> {
        self.manifest.clone()
    }
}
