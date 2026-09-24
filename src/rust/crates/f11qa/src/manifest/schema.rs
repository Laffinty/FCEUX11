use serde::{Deserialize, Serialize};

/// A single test case entry in the manifest (F11QA v1.8 schema).
///
/// Phase 7.1 migration: `id` 字段重命名为 `kgmqa_id`，`description` 改名
/// `title`；新增 `kind` / `vendor_state` / `spec_source` / `license` /
/// `mirror_ref` / `mirror_path` / `legacy_id`。`legacy_id` 保留 v1.17 命名
/// 以兼容旧 baseline (`tests/fixtures/f11qa_baseline_frozen.json` keyed
/// by v1.17 id)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestManifest {
    /// F11QA v1.8 主键 (kgmqa-001 ~ kgmqa-120)。
    #[serde(alias = "id")]
    pub kgmqa_id: String,
    /// v1.17 id（保留以兼容旧 baseline / 旧 fixtures）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_id: Option<String>,
    /// 用例标题（v1.17 叫 `description`，v1.8 改名 `title` 避免与 description 字段混淆）。
    #[serde(alias = "description")]
    pub title: String,
    /// F11QA v1.8 §三 §3.6: 10 种 kind 标签数组。可选（v1.17 baseline 无此字段）。
    #[serde(default)]
    pub kind: Vec<String>,
    /// F11QA v1.8 §三 §3.6: A/B 二元 orcale 通道（regression-equivalence vs
    /// hardware-consistency）。v1.8 tests.json 用 `kind` 表达用例类型
    /// (10 种 kind 标签数组)；`oracle_type` 仍可显式填，但缺省时由
    /// `effective_oracle_type()` 自动派生：`kind` 含 `rom-suite` → B，否则 A。
    #[serde(default = "default_oracle_type")]
    pub oracle_type: OracleType,
    pub layer: TestLayer,
    #[serde(default)]
    pub input: TestInput,
    pub expected: ExpectedResult,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    pub failure_means: FailureSeverity,
    pub provenance: String,
    /// F11QA v1.8 §三 §3.6 F: ROM 镜像源 vendor 状态。
    /// 仅 kind 包含 `rom-suite` 的用例填，其余为 None。
    /// v1.17 baseline 无此字段 → skip_serializing_if 让序列化干净。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_state: Option<String>,
    /// F11QA v1.8 §三 §3.6 E: 规格来源 (e.g. `nesdev.org/wiki/...`, `internal`)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_source: Option<String>,
    /// F11QA v1.8 §三 §3.6 H: 许可证标识 (PD/CC0/GPL-2.0/...)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// F11QA v1.8 §四: ROM 镜像源 git tag (e.g. `v1.8.0-mirror`)。仅 rom-suite 填。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_ref: Option<String>,
    /// F11QA v1.8 §四: 镜像源相对路径 (e.g. `blargg/cpu/instr_test_v5_all.nes`)。仅 rom-suite 填。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror_path: Option<String>,
}

fn default_timeout() -> u64 {
    60
}

fn default_oracle_type() -> OracleType {
    OracleType::A
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OracleType {
    /// Regression-equivalence (exit-code based, unit tests).
    #[serde(rename = "A")]
    A,
    /// Hardware-consistency (output/ROM based).
    #[serde(rename = "B")]
    B,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestLayer {
    #[serde(rename = "core")]
    Core,
    #[serde(rename = "boards")]
    Boards,
    #[serde(rename = "driver")]
    Driver,
    #[serde(rename = "lua")]
    Lua,
    #[serde(rename = "script")]
    Script,
    #[serde(rename = "benchmark")]
    Benchmark,
}

/// How to invoke the test.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestInput {
    /// Binary name (resolved relative to bin_dir).
    pub binary: String,
    /// Command-line arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional working directory override.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// P2: ROM path for Oracle B / ROM-based tests.
    #[serde(default)]
    pub rom: Option<String>,
    /// P2: Probe address for Oracle B $6000 protocol (default 0x6000).
    #[serde(default)]
    pub probe_addr: Option<u32>,
    /// P3: Lua script path for Lua script channel (run via fceux11_lua_runner).
    #[serde(default)]
    pub script_path: Option<String>,
    /// v1.17 H-1: if > 0, the runner steps this many frames, then issues a
    /// soft reset, then steps the remaining frames. -1 = no mid-run reset
    /// (default). 0 = reset immediately after load. This is a sibling
    /// parameter to `probe_addr` / `frames` — same scope (Oracle B driving),
    /// same shape (i64 default), same justification (blargg `$6000` protocol
    /// has ROMs that need a one-shot reset partway through to converge).
    ///
    /// Default is -1 (NOT 0) so manifest entries that do not carry the
    /// field (e.g. tests.json regression entries) keep the pre-H-1
    /// behaviour of "load → step all frames → probe" with no mid-run
    /// reset. A serde-default of 0 would have triggered the "reset
    /// immediately after load" path for every entry lacking the field,
    /// which breaks the direct adapter (reset() unloads the ROM).
    #[serde(default = "default_reset_after")]
    pub reset_after: i64,
}

/// Default `reset_after`: -1 → no mid-run reset.
fn default_reset_after() -> i64 {
    -1
}

/// Expected outcome for the test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedResult {
    pub exit_code: i32,
    #[serde(default)]
    pub stdout_contains: Option<String>,
}

/// Severity classification for failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureSeverity {
    /// Must pass; blocks release / CI gate.
    #[serde(rename = "blocking")]
    Blocking,
    /// Advisory only; continue-on-error.
    #[serde(rename = "advisory")]
    Advisory,
}
