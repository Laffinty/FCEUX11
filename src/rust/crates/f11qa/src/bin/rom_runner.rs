// F11QA v1.8 — f11qa-rom-runner
// 统一 ROM runner dispatcher（kgmqa-031 ~ 067 + 048 + 111 第三方 ROM + blargg 系列）。
//
// 用法：
//   f11qa-rom-runner --kgmqa-id <kgmqa-NNN-...> [--tests-json <path>] [case args...]
//
// 路由逻辑：
//   - --kgmqa-id K, --tests-json P  →  dispatcher 消费（路由）
//   - 其他参数（--frames, --log, --reset-after 等）→ 转发给底层 runner
//   - --rom 不接受 forwarded；dispatcher 从 mirror_path 派生 tests/fixtures/<basename>
//
// 调度分支：
//   - kgmqa-031 ~ 043, 053 ~ 067, 111 (blargg 系列)        → f11qa_blargg_runner --rom <derived>
//   - kgmqa-048 (nestest)                                  → f11qa_blargg_runner --rom + --log
//   - kgmqa-049 ~ 112 其他第三方 ROM（协议未实现）            → protocol-stub exit 0
//   - vendor_state=advisory / pending-vendor                 → skip + exit 0
//
// Phase 5 框架：v1.8 §四 §4.6 runner protocol 抽象层。
// Phase 9 (CI 实战) 时填充每个 suite-specific 协议细节（$6000 / 屏幕 hash / log diff 等）。

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
             --frames N     blargg-series frame count\n  \
             --log <path>   nestest trace log path\n  \
             --reset-after N blargg reset cadence\n  \
             --rom is NOT forwarded; dispatcher derives it from mirror_path"
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

    eprintln!(
        "[f11qa-rom-runner] kgmqa_id={} kind={:?} vendor_state={:?} mirror_path={:?}",
        kgmqa_id, kind, vendor_state, mirror_path
    );

    // Advisory / pending-vendor: skip + exit 0
    if vendor_state == "advisory" || vendor_state == "pending-vendor" {
        eprintln!(
            "[skip] vendor_state={}; ROM not fetched; exit 0",
            vendor_state
        );
        return ExitCode::SUCCESS;
    }

    // Dispatch by kgmqa_id prefix (Phase 5 framework)
    if is_blargg_series(&kgmqa_id) {
        return dispatch_blargg(&kgmqa_id, mirror_path, &forwarded);
    }
    if kgmqa_id.starts_with("kgmqa-043-blargg-suite") {
        // kgmqa-043 — 整套 blargg manifest batch 跑
        return dispatch_blargg_batch(&kgmqa_id, &forwarded);
    }
    if kgmqa_id.starts_with("kgmqa-048-nestest") {
        return dispatch_nestest(&kgmqa_id, mirror_path, &forwarded);
    }

    // Phase 5 framework: 第三方 ROM (kgmqa-049 ~ 112 大部分) — protocol-stub
    // Phase 9 实战时,每个 suite 适配 ($6000 / 屏幕 hash / log 文本 等)
    eprintln!(
        "[protocol-stub] kgmqa_id={} mirror_path={}",
        kgmqa_id, mirror_path
    );
    eprintln!("[protocol-stub] Phase 5 framework; Phase 9 will implement suite-specific protocol.");
    eprintln!("[protocol-stub] exit 0 (Phase 5 stub; Phase 9 will replace with real PASS/FAIL)");
    ExitCode::SUCCESS
}

// ============================================================================
// Dispatch helpers
// ============================================================================

fn is_blargg_series(kgmqa_id: &str) -> bool {
    // blargg 系列 (kgmqa-031~043, 053~067, 111)
    let prefixes = [
        "kgmqa-031-blargg-smoke",
        "kgmqa-032-cpu-instrs-blargg",
        "kgmqa-033-cpu-timing-blargg",
        "kgmqa-034-ppu-vbl-nmi-blargg",
        "kgmqa-035-mmc3-4-scanline-blargg",
        "kgmqa-036-mmc3-v2-4-scanline-blargg",
        "kgmqa-037-cpu-int-2-nmi-brk-blargg",
        "kgmqa-038-instr-misc-blargg",
        "kgmqa-039-oam-stress-blargg",
        "kgmqa-040-vbl-05-nmi-timing-blargg",
        "kgmqa-041-ppu-read-buffer-blargg",
        "kgmqa-042-sprdma-dmc-dma-blargg",
        "kgmqa-043-blargg-suite",
        "kgmqa-053-branch-timing-tests-blargg",
        "kgmqa-054-cpu-interrupts-v2-blargg",
        "kgmqa-055-cpu-reset-blargg",
        "kgmqa-056-instr-timing-blargg",
        "kgmqa-057-instr-test-v3-blargg",
        "kgmqa-058-ppu-sprite-hit-blargg",
        "kgmqa-059-sprite-overflow-blargg",
        "kgmqa-060-ppu-open-bus-blargg",
        "kgmqa-061-nmi-sync-blargg",
        "kgmqa-062-oam-read-blargg",
        "kgmqa-063-apu-test-blargg",
        "kgmqa-064-apu-mixer-blargg",
        "kgmqa-065-dmc-tests-blargg",
        "kgmqa-066-dmc-dma-during-read-blargg",
        "kgmqa-067-square-timer-div2-blargg",
        "kgmqa-111-read-joy3-blargg",
    ];
    prefixes.iter().any(|p| kgmqa_id.starts_with(p))
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
    eprintln!(
        "[dispatch] kgmqa={} -> {} --rom {} (forwarded={:?})",
        kgmqa_id,
        BLARGG_RUNNER_BIN,
        rom_local.display(),
        forwarded
    );
    let mut cmd = Command::new(BLARGG_RUNNER_BIN);
    cmd.arg("--rom").arg(&rom_local);
    cmd.arg("--kgmqa-id").arg(kgmqa_id);
    for a in forwarded {
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
    eprintln!(
        "[dispatch] kgmqa={} -> {} --manifest {} (forwarded={:?})",
        kgmqa_id,
        BLARGG_RUNNER_BIN,
        manifest.display(),
        forwarded
    );
    let mut cmd = Command::new(BLARGG_RUNNER_BIN);
    cmd.arg("--manifest").arg(manifest);
    cmd.arg("--kgmqa-id").arg(kgmqa_id);
    for a in forwarded {
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
    eprintln!(
        "[dispatch] kgmqa={} -> {} --rom {} --log {} (forwarded={:?})",
        kgmqa_id,
        BLARGG_RUNNER_BIN,
        rom_local.display(),
        log_local.display(),
        forwarded
    );
    let mut cmd = Command::new(BLARGG_RUNNER_BIN);
    cmd.arg("--rom").arg(&rom_local);
    cmd.arg("--log").arg(&log_local);
    cmd.arg("--kgmqa-id").arg(kgmqa_id);
    for a in forwarded {
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

fn rom_local_path(mirror_path: &str) -> PathBuf {
    // mirror_path = "<suite>/<rom>.nes" → tests/fixtures/<rom>.nes (just basename)
    // 与 scripts/fetch_roms_from_mirror.py 的 shutil.copy2(dst=output_dir/leaf) 一致
    let leaf = mirror_path.rsplit_once('/').map(|x| x.1).unwrap_or(mirror_path);
    Path::new("tests/fixtures").join(leaf)
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