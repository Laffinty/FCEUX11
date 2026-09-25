# F11QA ROM Runner Protocol — kgmqa-031 ~ kgmqa-112

> v1.8 计划 §四 §4.6 落地文档。第三方 ROM suite 在 f11qa 下的统一调度协议。

## 1. 目标

v1.8 接入 [f11qa-rom-mirror](https://github.com/Laffinty/f11qa-rom-mirror) 镜像源后，新增 65 项第三方 ROM suite 用例（kgmqa-048 ~ 112 + 31 ~ 043 + 053 ~ 067 + 111 共 78 项）。每项都需要：

- ROM 字节按 mirror_ref 锁定从镜像源拉取
- SHA-256 与镜像源 `SHA-256SUMS.txt` 比对
- 跑在 F11QA 测试框架内，与既有 harness-rust / harness-cpp / unit-* 用例并列
- 进入 CI 后给出 PASS/FAIL 信号，提交 mirror drift 报警

为避免 30+ 独立 `f11qa_*_runner` 二进制（v1.8 §3.6 F 表的设计假设），Phase 5 收敛到**一个统一 dispatcher**：

```
f11qa-rom-runner --kgmqa-id <id> [--tests-json tests/tests.json] [forwarded args...]
```

dispatcher 内部按 `kgmqa_id` 前缀路由到对应协议实现。

## 2. CLI 接口

```
f11qa-rom-runner
  --kgmqa-id <kgmqa-NNN-...>      必选；标识 tests/tests.json 里的用例
  [--tests-json <path>]           默认 tests/tests.json
  [forwarded args...]             --frames / --log / --reset-after 等转发给底层 runner
                                  --rom 由 dispatcher 从 mirror_path 派生，不接受 forwarded
```

返回码：
- `0` PASS（vendored ROM 跑通，或 advisory / pending-vendor 跳过）
- `1` FAIL（vendored ROM 跑挂）
- `2` 参数错误 / 用例找不到 / ROM 字节缺失（未先跑 fetch_roms_from_mirror.py）
- `3` 底层 runner spawn 失败（PATH 上找不到 `f11qa_blargg_runner` 等）

## 3. 路由表

| kgmqa_id 前缀             | 协议              | 底层调用                                       |
| ------------------------- | ----------------- | ---------------------------------------------- |
| kgmqa-048-nestest         | nestest trace     | `f11qa_blargg_runner --rom <p> --log <p.log> --kgmqa-id <id>` |
| kgmqa-043-blargg-suite    | blargg batch      | `f11qa_blargg_runner --manifest blargg_manifest.json --kgmqa-id <id>` |
| kgmqa-077-holy-mapperel   | aggregate-mapperel| 47 ROM 循环 × `$6000`，全部 PASS 才 PASS        |
| 其余全部 rom-suite        | `$6000` 状态口    | `f11qa_blargg_runner --rom <p> --kgmqa-id <id>` |

协议字段优先级：cases 的显式 `protocol` 字段 > 按 kgmqa_id 推断 > rom-suite 默认 `$6000`。
`tests/tests.json` 已对 78 条 rom-suite 全部落 `protocol` 字段（`$6000` ×76 / `nestest-trace` ×1 / `aggregate-mapperel` ×1）。

### 3.1 nestest（kgmqa-048，protocol=`nestest-trace`）

`nestest.nes` 是 NES 社区 CPU 指令正确性基准套件，输出走 `$6000` 端口写文本。本地期望值存在 `tests/fixtures/nestest.log`，由 v1.17 baseline 冻结。

dispatcher 派生：
- ROM 路径：`tests/fixtures/<basename>` ← `mirror_path` 的末段
- log 路径：同 basename 换 `.log` 扩展名

### 3.2 `$6000` 状态口协议（protocol=`$6000`，rom-suite 默认）

NES 测试 ROM 社区标准结果上报协议（blargg 最早推广）：跑 `--frames N` 帧后读 CPU `$6000`，`0x00`=PASS，`0x81`=运行中/需复位，其他值=FAIL 码（诊断串在 `$6001+`）。

**适用 suite**（寄存器/屏幕/log 差分是套件内部测试手法，结果仍走 `$6000`）：

| suite | kgmqa | 说明 |
|-------|-------|------|
| blargg | 031~043, 053~067, 111 | 原生 $6000 |
| bisqwit | 049, 050, 051, 052 | $6000 |
| damianyerrick / pinobatch | 068, 099 | $6000 |
| nk / drag / awj / natt | 069, 070, 071, 072, 073 | $6000 |
| tepples | 074, 075, 076, 093, 096, 097, 100, 101, 102, 103, 104, 109 | $6000（tvpassfail/porttest 等亦是） |
| lidnariq | 078, 106 | $6000 |
| quietust | 079, 080 | $6000（寄存器为测试对象） |
| sour FDS | 081 | $6000 |
| rainwarrior | 086~095, 098, 107, 108 | $6000 |
| 3gengames | 105 | $6000 |
| rahsennor | 110 | $6000（DMA 时序为测试对象） |
| flubba NEStress | 112 | $6000 |

`f11qa_blargg_runner` 的单 ROM 模式对任意 `$6000` ROM 通用：`--rom <p> --frames N`，返回 0/1。

### 3.3 aggregate-mapperel（kgmqa-077，protocol=`aggregate-mapperel`）

Holy Mapperel（pinobatch）47 个 mapper 识别 ROM，`mirror_glob: holy_mapperel/M*.nes`。

dispatcher 行为：
1. `list_fixtures("M*.nes")` 枚举 `tests/fixtures/` 下成员（fetch 脚本按同一扁平 basename 规则复制）
2. 逐个跑 `$6000` 协议，`--kgmqa-id kgmqa-077-...:<rom>` 区分成员
3. 全部 PASS → exit 0；任一 FAIL → exit 1；纯 spawn 环境失败 → exit 3；0 成员 → exit 2

### 3.4 vendor_state 三态（先于协议分发）

`advisory` / `pending-vendor` 在路由早期 skip + exit 0，**不会**进入协议分发。
`vendored` 缺 ROM 字节 → exit 2 + fetch hint（真实协议的文件前置检查，不是 stub）。

## 4. ROM 路径派生规则

`fetch_roms_from_mirror.py` 把 vendored ROM 复制到 `tests/fixtures/<basename>`（`Path(mirror_path).name`，仅取文件名末段，不保留 suite 子目录）。dispatcher 必须遵循同一规则，否则找不到 ROM：

```rust
fn rom_local_path(mirror_path: &str) -> PathBuf {
    let leaf = mirror_path.rsplit_once('/').map(|x| x.1).unwrap_or(mirror_path);
    Path::new("tests/fixtures").join(leaf)
}
```

示例：

| mirror_path                              | rom_local                              |
| ---------------------------------------- | -------------------------------------- |
| `nestest/nestest.nes`                    | `tests/fixtures/nestest.nes`           |
| `blargg/cpu/instr_test_v5_all.nes`       | `tests/fixtures/instr_test_v5_all.nes` |
| `sour/fdsirqtests/fdsirqtestsV7_patched.fds` | `tests/fixtures/fdsirqtestsV7_patched.fds` |
| `holy_mapperel/M0_P32K_C8K_V.nes`        | `tests/fixtures/M0_P32K_C8K_V.nes`     |

## 5. vendor_state 三态

`docs/f11qa-vendor-state.md` 定义了 78 项 rom-suite 用例的 vendor_state：

- ✅ **vendored** — ROM 字节已纳入镜像源，runner 跑通 = PASS，挂 = FAIL
- ⏸ **advisory** — 已知 ROM 字节暂时拿不到（如上游删档 / 协议不明），runner 跳过不 fail_to_pass
- ❌ **pending-vendor** — ROM 字节已知但镜像源上游 license / 重分发问题待决，runner 跳过不 fail_to_pass

dispatcher 在加载 case 后**第一个**判断 vendor_state：

```rust
if vendor_state == "advisory" || vendor_state == "pending-vendor" {
    return ExitCode::SUCCESS;  // skip
}
```

## 6. 协议落地状态（原 protocol-stub 路线图 → 已实现）

Phase 5 已把全部 stub 换成真实协议分发（`rom_runner.rs`）：

| suite              | kgmqa                | 协议            | 状态 |
| ------------------ | -------------------- | --------------- | ---- |
| bisqwit            | 049, 050, 051, 052   | `$6000`         | ✅ 路由到 f11qa_blargg_runner |
| holy_mapperel      | 077                  | `aggregate-mapperel`（47 ROM 聚合） | ✅ dispatch_aggregate |
| FDS (sour / takuikaninja) | 081 ~ 085     | `$6000`         | ✅ |
| rainwarrior submapper | 086 ~ 089          | `$6000`         | ✅ |
| rainwarrior n163   | 091, 092             | `$6000`         | ✅ |
| rainwarrior mapper | 090, 094             | `$6000`         | ✅ |
| tepples            | 074, 075, 076, 093, 096, 097, 100, 101, 102, 103, 104, 109 | `$6000` | ✅ |
| nk / drag          | 069, 070             | `$6000`         | ✅ |
| awj                | 071, 072             | `$6000`         | ✅ |
| natt               | 073                  | `$6000`         | ✅ |
| lidnariq           | 078, 106             | `$6000`         | ✅ |
| quietust           | 079, 080             | `$6000`         | ✅ |
| rainwarrior misc   | 098, 107, 108        | `$6000`         | ✅ |
| damianyerrick      | 068, 099             | `$6000`         | ✅ |
| 3gengames          | 105                  | `$6000`         | ✅ |
| rahsennor          | 110                  | `$6000`         | ✅ |
| nesstress          | 112                  | `$6000`         | ✅ |

协议选择依据：这些 ROM 采用 NES 测试 ROM 社区标准状态口上报（跑 N 帧 → `$6000`==0x00 即 PASS），
"寄存器 / 屏幕 / log 差分"描述的是套件**内部测试对象**，不是结果上报通道。
若某 ROM 在 CI 实战中被证实不走 `$6000`（如需屏幕 hash / log diff 黄金值），
在 tests.json 该条目加 `protocol: "<新协议>"` 并在 `rom_runner.rs` 的 `match protocol` 加分支即可，
无需改路由骨架。协议细节可写 `docs/f11qa-runner-protocol-<suite>.md`。

## 7. 与 v1.17 f11qa_blargg_runner 的关系

- v1.17 `f11qa_blargg_runner`（位于 `src/rust/crates/f11qa/src/runner/blargg.rs`，C ABI 入口在 `lib.rs::blargg_entry::kagami_qa_blargg_main`）保留
- Phase 5 不重写 blargg runner 内部；只让 `f11qa-rom-runner` 当 dispatcher
- `f11qa_blargg_runner --kgmqa-id <id>` 是 v1.17 已经接受的参数，Phase 5 复用

## 8. 落地验证

本地 smoke（已完成）：

```pwsh
$ # kgmqa-082 advisory → skip + exit 0
& src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe `
  --kgmqa-id kgmqa-082-fds-mirroring-takuikaninja `
  --tests-json tests\tests.json

$ # kgmqa-048 nestest → ROM/log 派生路径正确（缺文件时 exit 2 + hint）
& src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe `
  --kgmqa-id kgmqa-048-nestest `
  --tests-json tests\tests.json
```

CI 验证（Linux runner，等 Phase 8 冻结基线）：

1. 跑 `python scripts/fetch_roms_from_mirror.py` 拉 ROM
2. 跑 `ctest -R '^kgmqa-0(31|32|33|48|53|54|55|111|...)'` 跑 vendored 子集
3. 看 matrix 里 78 项 rom-suite 的 transition；advisory / pending-vendor 应保持 skip，vendored 应全 PASS

## 9. 与 kgmqa-117 mirror_snapshot_check 的关系

`kgmqa-117`（mirror snapshot 一致性检查）跑的是 `tests/f11qa/mirror_snapshot_check.py`，与 `f11qa-rom-runner` 互补：

- kgmqa-117 静态检查：mirror_ref 一致 / license 接受集 / SHA-256SUMS 完整性
- f11qa-rom-runner 动态跑：用例级 ROM 跑通验证

两者都依赖 `tests/fixtures/f11qa_mirror_pin.json` 锁定 mirror_ref。