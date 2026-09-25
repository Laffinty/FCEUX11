// F11QA v1.8 — f11qa-rom-runner
// 统一 ROM runner dispatcher（kgmqa-031 ~ 112 全部 rom-suite 用例）。
//
// 用法：
//   f11qa-rom-runner --kgmqa-id <kgmqa-NNN-...> [--tests-json <path>] [case args...]
//
// 路由逻辑：
//   - --kgmqa-id K, --tests-json P  →  dispatcher 消费（路由）
//   - 其他参数（--frames, --log, --reset-after 等）→ 转发给底层 runner
//   - --rom 不接受 forwarded；dispatcher 从 mirror_path 派生 tests/fixtures/<basename>
//
// 协议分发（Phase 5 真实协议，取代 protocol-stub）：
//   - vendor_state=advisory / pending-vendor  → skip + exit 0
//   - kgmqa-043 (blargg suite batch)          → f11qa_blargg_runner --manifest
//   - protocol=nestest-trace  (kgmqa-048)     → f11qa_blargg_runner --rom + --log
//   - protocol=aggregate-mapperel (kgmqa-077) → 47-ROM 循环，全部 PASS 才 PASS
//   - protocol=$6000 (rom-suite 默认)         → f11qa_blargg_runner --rom
//         NES 测试 ROM 社区标准状态口协议：跑 N 帧后读 $6000，0x00=PASS。
//         bisqwit / tepples / damianyerrick / lidnariq / rainwarrior / awj /
//         natt / nk / drag / quietust / sour / 3gengames / rahsennor / flubba
//         等 suite 的测试 ROM 均按此协议上报结果（寄存器/屏幕/log 差分是
//         套件内部测试手法，结果仍走 $6000）。与 blargg 系列共用同一执行器。
//
// 返回码：0=PASS 或 skip；1=FAIL；2=参数/文件缺失；3=底层 runner spawn 失败。

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const TESTS_JSON_DEFAULT: &str = "tests/tests.json";
const BLARGG_RUNNER_BIN: &str = "f11qa_blargg_runner";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: f11qa-rom-runner --kgmqa-id <kgmqa-NNN-...> \\\n\
             [--tests-json <path>] [-- <case args...>]\n\n\
             Routing args consumed by dispatcher:\n  \
             --kgmqa-id     required; identifies the test case\n  \
             --tests-json   default tests/tests.json\n\n\
             Forwarded args passed through to underlying runner:\n  \
             --frames N     frame budget for $6000 / aggregate protocols\n  \
             --log <path>   nestest trace log path (nestest-trace only)\n  \
             --reset-after N mid-run soft-reset cadence\n  \
             --rom is NOT forwarded; dispatcher derives it from mirror_path\n\n\
             Protocols: $6000 (default) | nestest-trace | aggregate-mapperel\n\
             vendor_state=advisory/pending-vendor → skip + exit 0"
        );
        return ExitCode::from(2);
    }

    // Routing vs forwarded args
    let mut kgmqa_id: Option<String> = None;
    let mut tests_json_path: PathBuf = PathBuf::from(TESTS_JSON_DEFAULT);
    let mut forwarded: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--kgmqa-id" => {
                kgmqa_id = args.get(i + 1).cloned();
                i += 2;
            }
            "--tests-json" => {
                tests_json_path = PathBuf::from(
                    args.get(i + 1).cloned().unwrap_or_default(),
                );
                i += 2;
            }
            "--rom" | "--rom=" => {
                // drop: dispatcher derives --rom from mirror_path
                if args[i] == "--rom" {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                forwarded.push(args[i].clone());
                i += 1;
            }
        }
    }

    let kgmqa_id = match kgmqa_id {
        Some(s) => s,
        None => {
            eprintln!("missing --kgmqa-id");
            return ExitCode::from(2);
        }
    };

    // Load tests.json v1.8 → read kind, vendor_state, mirror_path
    let raw = match std::fs::read_to_string(&tests_json_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "FAIL: read {}: {}",
                tests_json_path.display(),
                e
            );
            return ExitCode::from(1);
        }
    };
    let case = match find_case(&raw, &kgmqa_id) {
        Some(c) => c,
        None => {
            eprintln!(
                "FAIL: {} not found in {}",
                kgmqa_id,
                tests_json_path.display()
            );
            return ExitCode::from(1);
        }
    };

    let kind: Vec<String> = case["kind"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let vendor_state =
        case.get("vendor_state").and_then(|v| v.as_str()).unwrap_or("");
    let mirror_path =
        case.get("mirror_path").and_then(|v| v.as_str()).unwrap_or("");

    // Advisory / pending-vendor: skip + exit 0
    if vendor_state == "advisory" || vendor_state == "pending-vendor" {
        eprintln!(
            "[f11qa-rom-runner] kgmqa_id={} kind={:?} vendor_state={:?}",
            kgmqa_id, kind, vendor_state
        );
        eprintln!(
            "[skip] vendor_state={}; ROM not fetched; exit 0",
            vendor_state
        );
        return ExitCode::SUCCESS;
    }

    // Dispatch: real protocols (Phase 5 — no more protocol-stub)
    let protocol = resolve_protocol(&case, &kgmqa_id);

    eprintln!(
        "[f11qa-rom-runner] kgmqa_id={} protocol={} kind={:?} vendor_state={:?} mirror_path={:?}",
        kgmqa_id, protocol, kind, vendor_state, mirror_path
    );

    // kgmqa-043 — 整套 blargg manifest batch（无单 ROM mirror_path）
    if kgmqa_id.starts_with("kgmqa-043-blargg-suite") {
        return dispatch_blargg_batch(&kgmqa_id, &forwarded);
    }

    match protocol.as_str() {
        PROTO_NESTEST => dispatch_nestest(&kgmqa_id, mirror_path, &forwarded),
        PROTO_AGGREGATE => dispatch_aggregate(&kgmqa_id, &case, &forwarded),
        PROTO_6000 => dispatch_blargg(&kgmqa_id, mirror_path, &forwarded),
        other => {
            eprintln!(
                "[fail] unknown protocol {:?} for {} (expected '{}', '{}', '{}')",
                other, kgmqa_id, PROTO_6000, PROTO_NESTEST, PROTO_AGGREGATE
            );
            ExitCode::from(2)
        }
    }
}

// ============================================================================
// Protocol resolution
// ============================================================================

/// Standard blargg-style status-port protocol: after `frames` steps, `$6000`
/// reads 0x00 for PASS. Used by the NES test-ROM community (bisqwit, tepples,
/// damianyerrick, lidnariq, rainwarrior, awj, natt, nk, drag, quietust, sour,
/// 3gengames, rahsennor, flubba, …) as well as blargg's own suites.
const PROTO_6000: &str = "$6000";
/// kevtris nestest: CPU trace compared against a Nintendulator golden log.
const PROTO_NESTEST: &str = "nestest-trace";
/// Holy Mapperel: 47 mapper-identification ROMs, all must PASS.
const PROTO_AGGREGATE: &str = "aggregate-mapperel";

/// Explicit `protocol` field wins; otherwise infer from kgmqa_id. All
/// rom-suite cases default to the $6000 status-port protocol.
fn resolve_protocol(case: &serde_json::Value, kgmqa_id: &str) -> String {
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

// ============================================================================
// Dispatch helpers
// ============================================================================

fn spawn_blargg(args: &[String]) -> ExitCode {
    eprintln!(
        "[dispatch] -> {} {:?}",
        BLARGG_RUNNER_BIN,
        args
    );
    let mut cmd = Command::new(BLARGG_RUNNER_BIN);
    for a in args {
        cmd.arg(a);
    }
    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("[fail] {} spawn: {}", BLARGG_RUNNER_BIN, e);
            ExitCode::from(3)
        }
    }
}

fn dispatch_blargg(
    kgmqa_id: &str,
    mirror_path: &str,
    forwarded: &[String],
) -> ExitCode {
    let rom_local = rom_local_path(mirror_path);
    if !rom_local.exists() {
        eprintln!(
            "[fail] ROM not in tests/fixtures: {} (mirror_path={})",
            rom_local.display(),
            mirror_path
        );
        eprintln!("[hint] run scripts/fetch_roms_from_mirror.py first");
        return ExitCode::from(2);
    }
    let mut args = vec![
        "--rom".to_string(),
        rom_local.display().to_string(),
        "--kgmqa-id".to_string(),
        kgmqa_id.to_string(),
    ];
    args.extend_from_slice(forwarded);
    spawn_blargg(&args)
}

fn dispatch_blargg_batch(
    kgmqa_id: &str,
    forwarded: &[String],
) -> ExitCode {
    // kgmqa-043 — 整套 blargg manifest（tests/fixtures/blargg_manifest.json）
    let manifest = Path::new("tests/fixtures/blargg_manifest.json");
    if !manifest.exists() {
        eprintln!(
            "[fail] blargg manifest missing: {} (run scripts/fetch_roms_from_mirror.py first)",
            manifest.display()
        );
        return ExitCode::from(2);
    }
    let mut args = vec![
        "--manifest".to_string(),
        manifest.display().to_string(),
        "--kgmqa-id".to_string(),
        kgmqa_id.to_string(),
    ];
    args.extend_from_slice(forwarded);
    spawn_blargg(&args)
}

fn dispatch_nestest(
    kgmqa_id: &str,
    mirror_path: &str,
    forwarded: &[String],
) -> ExitCode {
    // nestest trace: ROM + 同名 .log 文件
    let rom_local = rom_local_path(mirror_path);
    let log_local = rom_local.with_extension("log");
    if !rom_local.exists() || !log_local.exists() {
        eprintln!(
            "[fail] nestest ROM/log missing: {} / {}",
            rom_local.display(),
            log_local.display()
        );
        return ExitCode::from(2);
    }
    let mut args = vec![
        "--rom".to_string(),
        rom_local.display().to_string(),
        "--log".to_string(),
        log_local.display().to_string(),
        "--kgmqa-id".to_string(),
        kgmqa_id.to_string(),
    ];
    args.extend_from_slice(forwarded);
    spawn_blargg(&args)
}

/// Holy Mapperel aggregate (kgmqa-077): 47 mapper-identification ROMs run
/// through the $6000 protocol one by one; every member must PASS.
///
/// Member discovery uses `mirror_glob` (default `holy_mapperel/M*.nes`);
/// the leaf pattern is matched against `tests/fixtures/` with the same
/// flat-basename rule as `rom_local_path` (see fetch_roms_from_mirror.py).
fn dispatch_aggregate(
    kgmqa_id: &str,
    case: &serde_json::Value,
    forwarded: &[String],
) -> ExitCode {
    let glob = case
        .get("mirror_glob")
        .and_then(|v| v.as_str())
        .unwrap_or("holy_mapperel/M*.nes");
    let leaf_pat = glob.rsplit_once('/').map(|x| x.1).unwrap_or(glob);
    let roms = match list_fixtures(leaf_pat) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[fail] list tests/fixtures ({}): {}", leaf_pat, e);
            return ExitCode::from(2);
        }
    };
    if roms.is_empty() {
        eprintln!(
            "[fail] aggregate {}: no ROMs match tests/fixtures/{} (mirror_glob={})",
            kgmqa_id, leaf_pat, glob
        );
        eprintln!("[hint] run scripts/fetch_roms_from_mirror.py first");
        return ExitCode::from(2);
    }
    eprintln!(
        "[aggregate] kgmqa={} members={} pattern={}",
        kgmqa_id,
        roms.len(),
        leaf_pat
    );
    let mut failed: Vec<String> = Vec::new();
    let mut spawn_failed = 0usize;
    for (i, rom) in roms.iter().enumerate() {
        let member_id = format!(
            "{}:{}",
            kgmqa_id,
            rom.file_name().unwrap_or_default().to_string_lossy()
        );
        let mut args = vec![
            "--rom".to_string(),
            rom.display().to_string(),
            "--kgmqa-id".to_string(),
            member_id,
        ];
        args.extend_from_slice(forwarded);
        eprintln!(
            "[aggregate] ({}/{}) {}",
            i + 1,
            roms.len(),
            rom.display()
        );
        let code = spawn_blargg(&args);
        if code != ExitCode::SUCCESS {
            failed.push(rom.display().to_string());
            if code == ExitCode::from(3) {
                spawn_failed += 1;
            }
        }
    }
    if failed.is_empty() {
        eprintln!("[aggregate] {} all {} members PASS", kgmqa_id, roms.len());
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "[aggregate] {} FAIL: {}/{} members failed: {:?}",
            kgmqa_id,
            failed.len(),
            roms.len(),
            failed
        );
        // Pure environment failure (runner binary missing) is not a test FAIL.
        if spawn_failed == failed.len() {
            ExitCode::from(3)
        } else {
            ExitCode::from(1)
        }
    }
}

fn rom_local_path(mirror_path: &str) -> PathBuf {
    // mirror_path = "<suite>/<rom>.nes" → tests/fixtures/<rom>.nes (just basename)
    // 与 scripts/fetch_roms_from_mirror.py 的 shutil.copy2(dst=output_dir/leaf) 一致
    let leaf = mirror_path.rsplit_once('/').map(|x| x.1).unwrap_or(mirror_path);
    Path::new("tests/fixtures").join(leaf)
}

/// List `tests/fixtures/<pattern>` where `pattern` supports a single `*`
/// wildcard (enough for `M*.nes`). Returns basenames sorted for stable order.
fn list_fixtures(pattern: &str) -> std::io::Result<Vec<PathBuf>> {
    let dir = Path::new("tests/fixtures");
    let (prefix, suffix) = match pattern.split_once('*') {
        Some((p, s)) => (p, s),
        None => {
            // literal filename
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

// ============================================================================
// tests.json scanner
// ============================================================================

fn find_case(json_text: &str, kgmqa_id: &str) -> Option<serde_json::Value> {
    let needle = format!("\"kgmqa_id\": \"{}\"", kgmqa_id);
    let pos = json_text.find(&needle)?;
    let start = json_text[..pos].rfind('{')?;
    let rest = &json_text[start..];
    let mut depth = 0;
    let mut end = 0;
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if end == 0 {
        return None;
    }
    serde_json::from_str(&rest[..end]).ok()
}