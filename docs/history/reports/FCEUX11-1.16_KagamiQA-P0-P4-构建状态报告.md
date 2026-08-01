# FCEUX11 v1.16 KagamiQA P0–P4 构建状态报告

> **日期**：2026-07-28
> **分支**：`wip_1.16`（领先 `origin/wip_1.16` 5 commits）
> **原则**：新 PPU (`FCEUX_PPU_Loop`) 是 FCEUX11 精度优化的核心——**绝不回退到旧 PPU**。

---

## 一、提交历史

```
1c02772 P4-bridge: new PPU headless init + runppu cycle fix
b32d071 P4-report: full §八 report format (transition_matrix + baseline)
cda40fe Revert "P4-2: APU length counter unconditional reload" (被 blargg 证伪)
562f0e8 P4-2: APU length counter (WRONG — reverted)
592201e P4-1: PPU VBL NMI timing fix + blargg $6004+ diagnostic enhancement
7618472 P2+P3: Oracle B 接入 + Qt 解耦 + Lua 通道
1180b41 P1: 收编 + headless 全款
```

有效交付（除去 revert 对）：**6 个 commit**。

---

## 二、各 Phase 构建状态

### P0：报告转译

| 项目 | 状态 | 说明 |
|------|------|------|
| 计划文档 | ✅ | `docs/history/plans/FCEUX11-1.16_KagamiQA-PLAN.md` |
| 代码级交叉验证 | ✅ | 审计了 30 CTest / 7 Rust crate / Lua 引擎 / 双 oracle 缺失实证 |
| 技术选型 | ✅ | Rust + Lua/JSON 双通道 + SutAdapter trait |

### P1：收编 + headless 全款

| 项目 | 状态 | 说明 |
|------|------|------|
| kagami-qa crate | ✅ | `src/rust/crates/kagami-qa/` — 6 模块 (core/manifest/adapter/runner/oracle/report) |
| tests.json 清单 | ✅ | 39 条目，覆盖全部 CTest + 脚本门禁 + blargg + Lua |
| null driver | ✅ | `src/drivers/null/null_driver.h/.cpp` — 全 nullptr DriverCallbacks |
| headless 无 Qt 运行 | ✅ | `fceux11_add_headless_test_executable` CMake function，link `fceux11_drivers_null` |
| nes_shm 下沉 | ✅ | `drivers/Qt/nes_shm.*` → `drivers/common/nes_shm.*` |
| golden_hashes 审计 | ✅ | `tests/fixtures/golden_hashes_audit.json` — 结论：非错误固化 |
| SutAdapter trait | ✅ | `adapter/trait_def.rs` — 被测物无关抽象 |
| SubprocessAdapter | ✅ | `adapter/subprocess.rs` — 封装现有 CTest 二进制 |
| Rust 测试 | ✅ | 22/22 PASS |
| **P1 退出条件** | ✅ | 全部达成 |

### P2：Oracle B 接入

| 项目 | 状态 | 说明 |
|------|------|------|
| blargg runner | ✅ | `tests/blargg_runner.cpp`（398 行）— `--rom` 单 ROM + `--manifest` 批模式 |
| $6000 协议解析 | ✅ | `oracle/hardware.rs` — BLARGG_RESULT 行解析 + accuracy table + known-fail 交叉校验 |
| 精度对照表 | ✅ | `docs/history/obsolete/FCEUX11-1.16_KagamiQA-P2-accuracy-table.md` — TASVideos 格式 |
| 已知失败清单 | ✅ | `tests/fixtures/blargg_known_fail.json` — baseline_version v1.16.0-P2 |
| ROM 下载脚本 | ✅ | `scripts/download_blargg_roms.ps1` — 22 ROM (cpu/ppu/apu) |
| ROM fixtures | ✅ | `tests/fixtures/blargg/` — 22 个 .nes 全部就位 |
| **P2 退出条件** | ✅ | 全部达成 |

**Oracle B 基线（旧 PPU, newppu=0）**：

| 类别 | PASS | FAIL | ERROR |
|------|------|------|-------|
| CPU | 1 (cpu_timing_test) | 2 (all_instrs, official_only) | 1 (cpu_interrupts — ROM format) |
| PPU Sprite Overflow | 5 | 0 | 0 |
| PPU Sprite 0 Hit | 11 | 0 | 0 |
| PPU VBL NMI | 0 | 1 (ppu_vbl_nmi) | 0 |
| APU | 0 | 1 (apu_test) | 0 |
| **合计** | **17** | **4** | **1** |

### P3：软件侧输入通道

| 项目 | 状态 | 说明 |
|------|------|------|
| lua-engine.cpp Qt 解耦 | ✅ | 删除 10 行 Qt-specific includes |
| headless Lua runner | ✅ | `tests/lua_runner.cpp`（256 行）— null driver + fceux11-lua FFI |
| fceux11-lua 注册修正 | ✅ | `lua.globals().set()` 替代直接 `register()` |
| tests.json Lua 条目 | ✅ | 4 个 Lua 测例（bit/emu/memory/joypad） |
| fceux11_drivers_null headless stubs | ✅ | RefreshThrottleFPS / AviIsRecording / TAS editor / input stubs |
| **P3 退出条件** | ✅ | 全部达成 |

### P4：精度攻关 + 报告格式

#### P4-report：完整 §八 JSON 格式

| 字段 | 状态 | 实现位置 |
|------|------|----------|
| `report_version` | ✅ | `matrix.rs:194` |
| `run_id` | ✅ | `matrix.rs:195` — `YYYYMMDD-HHMMSS-xxxxxx` |
| `engine` | ✅ | `matrix.rs:196` — version / toolchain / git_rev |
| `summary` | ✅ | `matrix.rs:197` — total / passed / failed / skipped |
| `transition_matrix` | ✅ | `matrix.rs:198` — fail_to_pass / pass_to_pass / pass_to_fail / fail_to_fail |
| `oracle_breakdown` | ✅ | `matrix.rs:199` — A_regression / B_hardware |
| `baseline_drift` | ✅ | `matrix.rs:200` — 字段就位，检测逻辑 stub |
| `details` | ✅ | `matrix.rs:201` — 每条含 oracle_type / layer / duration_ms |
| baseline I/O | ✅ | `baseline.rs` — `--baseline` / `--save-baseline` CLI flags |
| Rust 测试 | ✅ | 22/22 PASS |

#### P4-1：PPU VBL NMI 时序修正

| 项目 | 状态 | 说明 |
|------|------|------|
| 根因 | ✅ | VBL flag 在 scanline 241 cycle 0 置位——真机为 cycle 1 |
| 修复代码 | ✅ | `ppu_rendering.cpp:1545` — `runppu(1)` 先推进 1 cycle |
| 修复位置 | ✅ | `FCEUX_PPU_Loop`（新 PPU）——旧 PPU 无 PPU dot 概念，无法修复 |
| 实测验证 | ⚠️ | 修复代码已就位，需 newppu=1 headless 路径通后验证 |

#### P4-2：APU length counter（已回退）

| 项目 | 状态 | 说明 |
|------|------|------|
| 修复方向 | ❌ | 无条件 reload length counter——被 blargg 诊断证伪 |
| blargg 诊断 | — | `"When disabled via $4015, length shouldn't allow reloading"` |
| 结论 | — | 真机在 channel disable 时不 reload。子测试 #7 是更精细的边界条件 |

#### P4-bridge：新 PPU headless 初始化（进行中）

| 项目 | 状态 | 说明 |
|------|------|------|
| 修复 #1：ppudead VBL+NMI | ✅ | 对齐旧 PPU 行为，消除功能缺口 |
| 修复 #2：runppu() 周期计数器 | ✅ | `if`→`while` 消除多 wrap 时计数器损坏 |
| 修复 #3：blargg_runner newppu=1 | ✅ | LoadGame 后、Emulate 前正确设置 |
| 帧 1-2（ppudead）实测 | ✅ | 完成 |
| 帧 3+（正常渲染）实测 | ❌ | 挂起——根因待查 |

---

## 三、详细问题记录

### 3.1 P4-bridge：新 PPU 正常渲染路径挂起

**症状**：`newppu=1` 时，ppudead 帧（1-2）正常完成，首次进入正常渲染路径（帧 3）后进程挂起，无输出、无崩溃信息。

**已排除的根因**：

| 假说 | 排除依据 |
|------|----------|
| ppudead 路径缺 VBL+NMI | 已修复——帧 1 完成 |
| runppu() 周期计数器单次 wrap | 已修复——`if`→`while` |
| normalscanlines 值错误 | 测试过 241 / 240 / 5——均挂起 |
| 渲染循环 scanline 数量过大 | normalscanlines=5 时仍挂起 |
| XBuf 未分配 | `FCEU_InitVirtualVideo()` 在 Initialize 阶段调用 |
| 视频输出回调崩溃 | null driver 全 nullptr，安全 no-op |
| A2004 sprite 循环膨胀 | 周期计数器修复后 s_r.last==cycle，循环 0 次 |
| TriggerNMI 导致死循环 | ppudead 帧已有 NMI——帧 1-2 完成 |

**剩余候选方向**：

1. per-scanline sprite evaluation (`spr_read.start_scanline()` → 后续 OAM 读取)
2. per-tile BG fetch (`bgdata.main[xt+2].Read()`) — 读 PPU 内存时触发 mapper 回调
3. `CALL_PPUREAD` → `ARead[addr](addr)` — 特定地址（$2007 VRAM read）在 headless 下的行为
4. `runppu1_inline()` 中的 `X6502_Run(1)` — CPU 在正常渲染期间的 NMI handler 可能执行了触发再入的 PPU 访问
5. 正常渲染路径中某处对 Qt driver 回调的隐式依赖（尽管已 nullptr-safe）

**推荐调试方式**：Visual Studio Debug 配置，在 `FCEUX_PPU_Loop` 的 `for (int sl = 0; sl < normalscanlines; sl++)` 行设断点，逐 scanline 跟踪。若未到达此行，断点上移至 VBlank 区 `PPU_status = 0` 行。

**强调**：此问题的解决方案是**继续追查并修复新 PPU 的 headless 渲染路径**，绝不可回退到旧 PPU。旧 PPU 没有 PPU dot 概念，无法通过 blargg 的 cycle 级精度测试——回退等于放弃 Oracle B 的权威性。

### 3.2 P4-2：APU length counter 修复被证伪

blargg 诊断字符串明确揭示真机行为：**$4015 disable 后 length counter 不应 reload**。原代码的 `EnabledChannels` 门禁是正确的。失败在子测试 #7（共 8 项），是更精细的边界条件（如 reload 时序或 $4015 读回行为），不是 reload 门禁逻辑。

### 3.3 P4-1：PPU VBL NMI 代码无法在旧 PPU 下验证

`ppu_vbl_nmi.nes` 诊断：`"VBL period is too short with BG off\n01-vbl_basics\nFailed #7"`。此测试要求 PPU-cycle 级精度——旧 PPU (`FCEUPPU_Loop`) 使用 `X6502_Run(N)` 批量推进 CPU 周期，没有 PPU dot 概念，无法通过。修复（cycle 0→cycle 1）已写入新 PPU (`FCEUX_PPU_Loop`)，需 headless 路径通后验证。

### 3.4 CPU 指令测试：需更深诊断

`official_only.nes` 诊断：`"Running test 4 of 16"`——在第 4/16 个指令组失败。`all_instrs.nes` 无诊断字符串输出（$6004 区为空）。需增强 blargg ROM 的诊断输出（可能需修改 ROM 参数或增加帧数）以获取具体失败指令。

---

## 四、当前架构全景

```
KagamiQA 框架 (src/rust/crates/kagami-qa/)
├── core/           QaConfig, QaError
├── manifest/       tests.json schema + parser (39 条目)
├── adapter/        SutAdapter trait + SubprocessAdapter
├── runner/         TestScheduler + manifest_snapshot()
├── oracle/
│   ├── regression.rs   Oracle A: exit-code 判定
│   └── hardware.rs     Oracle B: blargg $6000 协议 + accuracy table
├── report/
│   ├── matrix.rs       完整 §八 报告 + transition_matrix
│   └── baseline.rs     baseline I/O + drift 检测 (stub)
└── main.rs             CLI runner (--baseline / --save-baseline)

测试基础设施
├── tests/tests.json                   39 条目清单
├── tests/blargg_runner.cpp            Oracle B 执行器
├── tests/lua_runner.cpp              Lua 通道执行器
├── tests/headless_smoke_test.cpp     P1 headless 冒烟
├── tests/fixtures/blargg/            22 ROM
├── tests/fixtures/golden_hashes_audit.json
├── tests/fixtures/blargg_known_fail.json
└── tests/fixtures/blargg_manifest.json

模拟器改动
├── src/ppu_rendering.cpp
│   ├── P4-1: VBL+NMI cycle 1 alignment (FCEUX_PPU_Loop)
│   └── P4-bridge: ppudead VBL+NMI + runppu() while-loop
├── src/drivers/null/     null driver + headless stubs
├── src/drivers/common/   nes_shm 下沉
└── src/lua-engine.cpp    Qt includes 移除
```

---

## 五、下一步行动建议

### 优先级 1：新 PPU 正常渲染路径挂起（阻塞 P4-1 验证）

**必须在 Visual Studio 中调试**——纯代码审查已穷尽。步骤：
1. `cmake --build build --config Debug --target fceux11_blargg_runner`
2. VS 打开 `fceux11_blargg_runner.exe`，参数 `--rom tests/fixtures/blargg/ppu/ppu_vbl_nmi.nes --frames 3`
3. 断点设在 `ppu_rendering.cpp:1629`（渲染 for 循环）
4. 若未到达，逐层上移断点

### 优先级 2：渲染路径通后立即验证 P4-1

```bash
fceux11_blargg_runner --rom fixtures/blargg/ppu/ppu_vbl_nmi.nes --frames 300
# 预期：status=PASS（首个 Oracle B FAIL→PASS）
```

### 优先级 3：增强 CPU 诊断

`official_only.nes` 在测试 4/16 失败——需获取具体指令名。可能需要增加帧数或修改 ROM 参数。

### 优先级 4：APU 子测试 #7 根因

`1-len_ctr` 的 8 个子测试中 #7 失败——需要 blargg APU ROM 的更细粒度诊断。

---

## 六、关键约束重申

1. **新 PPU 是唯一方向**：旧 PPU (`FCEUPPU_Loop`) 没有 PPU dot 概念，无法通过 blargg 的 cycle 级精度测试。FCEUX11 的性能优化以新 PPU 为核心。
2. **headless 是 Oracle B 的基础**：自动化测试必须在无 GUI 环境下运行。Qt GUI driver 不可作为测试依赖。
3. **Oracle A 全绿是每次修改的前置条件**：任何精度修复前必须先跑 `ctest` 确认无回归。
4. **AI 不得修改已入库的 expected 值**：基线更新走与代码同级评审。
