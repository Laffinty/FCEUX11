//! Manifest filtering — select a subset of tests by id / tag / layer / oracle.
//!
//! Task 4 (FCEUX11-1.17_计划.md §5.3 step 6): `--filter` support so precision
//! work can run "just the blargg_* subset" without external grep/ctest.
//!
//! Grammar (pure Rust, no external deps):
//! ```text
//! expr := term ("&" term)*
//! term := key "=" value      key ∈ {id, tag, layer, oracle}
//! ```
//! Semantics: terms are AND-ed. `id` is a substring match on `test_id`;
//! `tag` is an exact match on any entry tag; `layer` and `oracle` are exact
//! matches (case-insensitive on the value).

use std::collections::BTreeMap;

use super::schema::{OracleType, TestLayer, TestManifest};

/// A parsed `--filter` expression (AND of terms).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    terms: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Term {
    IdContains(String),
    TagEquals(String),
    LayerEquals(TestLayer),
    OracleEquals(OracleType),
}

impl Filter {
    /// Parse a filter expression. `&` separates AND-ed terms; whitespace
    /// around terms and keys/values is ignored.
    pub fn parse(expr: &str) -> Result<Self, String> {
        let raw_terms: Vec<&str> = expr
            .split('&')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if raw_terms.is_empty() {
            return Err("empty filter expression".into());
        }
        let mut terms = Vec::with_capacity(raw_terms.len());
        for raw in raw_terms {
            terms.push(parse_term(raw)?);
        }
        Ok(Self { terms })
    }

    /// Does this filter match a manifest entry?
    pub fn matches(&self, test: &TestManifest) -> bool {
        self.terms.iter().all(|t| t.matches(test))
    }

    /// Apply the filter to a manifest map, keeping only matching entries.
    pub fn apply(&self, manifest: &BTreeMap<String, TestManifest>) -> BTreeMap<String, TestManifest> {
        manifest
            .iter()
            .filter(|(_, t)| self.matches(t))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl Term {
    fn matches(&self, test: &TestManifest) -> bool {
        match self {
            Term::IdContains(needle) => test.id.contains(needle.as_str()),
            Term::TagEquals(tag) => test.tags.iter().any(|t| t == tag),
            Term::LayerEquals(layer) => &test.layer == layer,
            Term::OracleEquals(oracle) => &test.oracle_type == oracle,
        }
    }
}

fn parse_term(raw: &str) -> Result<Term, String> {
    let (key, value) = raw.split_once('=').ok_or_else(|| {
        format!("invalid filter term '{raw}': expected key=value (key ∈ id|tag|layer|oracle)")
    })?;
    let key = key.trim();
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("invalid filter term '{raw}': empty value"));
    }
    match key {
        "id" => Ok(Term::IdContains(value.to_string())),
        "tag" => Ok(Term::TagEquals(value.to_string())),
        "layer" => Ok(Term::LayerEquals(parse_layer(value)?)),
        "oracle" => Ok(Term::OracleEquals(parse_oracle(value)?)),
        other => Err(format!("unknown filter key '{other}': expected id|tag|layer|oracle")),
    }
}

fn parse_layer(value: &str) -> Result<TestLayer, String> {
    match value.to_ascii_lowercase().as_str() {
        "core" => Ok(TestLayer::Core),
        "boards" => Ok(TestLayer::Boards),
        "driver" => Ok(TestLayer::Driver),
        "lua" => Ok(TestLayer::Lua),
        "script" => Ok(TestLayer::Script),
        "benchmark" => Ok(TestLayer::Benchmark),
        other => Err(format!("unknown layer '{other}'")),
    }
}

fn parse_oracle(value: &str) -> Result<OracleType, String> {
    match value.to_ascii_uppercase().as_str() {
        "A" => Ok(OracleType::A),
        "B" => Ok(OracleType::B),
        other => Err(format!("unknown oracle '{other}': expected A or B")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::schema::{ExpectedResult, FailureSeverity, TestInput};

    fn make_test(id: &str, oracle: OracleType, layer: TestLayer, tags: &[&str]) -> TestManifest {
        TestManifest {
            id: id.into(),
            description: String::new(),
            oracle_type: oracle,
            layer,
            input: TestInput::default(),
            expected: ExpectedResult {
                exit_code: 0,
                stdout_contains: None,
            },
            timeout_seconds: 60,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            failure_means: FailureSeverity::Blocking,
            provenance: "test".into(),
        }
    }

    fn sample_manifest() -> BTreeMap<String, TestManifest> {
        let mut m = BTreeMap::new();
        m.insert(
            "blargg_cpu_instrs".into(),
            make_test("blargg_cpu_instrs", OracleType::B, TestLayer::Core, &["blargg", "cpu", "oracle-b"]),
        );
        m.insert(
            "cpu_test".into(),
            make_test("cpu_test", OracleType::A, TestLayer::Core, &["unit", "cpu"]),
        );
        m.insert(
            "lua_bit_test".into(),
            make_test("lua_bit_test", OracleType::A, TestLayer::Lua, &["lua", "unit"]),
        );
        m
    }

    #[test]
    fn tag_filter_converges() {
        let f = Filter::parse("tag=blargg").unwrap();
        let out = f.apply(&sample_manifest());
        assert_eq!(out.len(), 1);
        assert!(out.contains_key("blargg_cpu_instrs"));
    }

    #[test]
    fn combined_layer_oracle_filter_converges() {
        let f = Filter::parse("layer=core & oracle=B").unwrap();
        let out = f.apply(&sample_manifest());
        assert_eq!(out.len(), 1);
        assert!(out.contains_key("blargg_cpu_instrs"));
    }

    #[test]
    fn id_substring_filter_converges() {
        let f = Filter::parse("id=blargg_cpu_instrs").unwrap();
        let out = f.apply(&sample_manifest());
        assert_eq!(out.len(), 1);
        assert_eq!(out.keys().next().map(String::as_str), Some("blargg_cpu_instrs"));
    }

    #[test]
    fn id_substring_matches_partial() {
        let f = Filter::parse("id=blargg_cpu").unwrap();
        let out = f.apply(&sample_manifest());
        assert_eq!(out.len(), 1);
        assert!(out.contains_key("blargg_cpu_instrs"));
    }

    #[test]
    fn no_terms_matches_nothing() {
        let f = Filter::parse("id=blargg_cpu").unwrap();
        let t = make_test("cpu_test", OracleType::A, TestLayer::Core, &["unit"]);
        assert!(!f.matches(&t));
    }

    #[test]
    fn unknown_key_errors() {
        assert!(Filter::parse("nope=x").is_err());
    }

    #[test]
    fn missing_equals_errors() {
        assert!(Filter::parse("tagblargg").is_err());
    }

    #[test]
    fn empty_expression_errors() {
        assert!(Filter::parse("").is_err());
        assert!(Filter::parse("   ").is_err());
    }

    #[test]
    fn bad_layer_errors() {
        assert!(Filter::parse("layer=plasma").is_err());
    }

    #[test]
    fn bad_oracle_errors() {
        assert!(Filter::parse("oracle=C").is_err());
    }

    #[test]
    fn oracle_value_is_case_insensitive() {
        let f = Filter::parse("oracle=b").unwrap();
        let t = make_test("t", OracleType::B, TestLayer::Core, &[]);
        assert!(f.matches(&t));
    }

    #[test]
    fn whitespace_tolerated() {
        let f = Filter::parse("  layer = core   &  oracle = B ").unwrap();
        let out = f.apply(&sample_manifest());
        assert_eq!(out.len(), 1);
    }
}
