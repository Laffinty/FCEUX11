# Phase 7 · 默认切换 & 灰度发布

> **目标**：把 `VNESU11_CORE` 默认改为 **ON**，逐步移除对 C++ CPU/PPU（newppu=1 路径）的依赖（但保留兼容垫片与 newppu=0 回退）。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` 修订——
> ① 切换范围限定为 newppu=1 路径（旧 PPU 保留，ADR-009）；
> ② shadow run 用帧级三级对比（S10）；
> ③ 性能 DoD 放宽为 ≤ ×1.05（S8）。

## 工期：2 周

---

## 1. 范围

### 1.1 ✅ 在范围内
- `VNESU11_CORE` 默认改 **ON**
- `VNESU11_CORE=OFF` 仍可编译（兼容回退路径）
- 全量 blargg 177 ROM 通过 kagami-qa-runner Oracle B（**newppu=1 模式**）
- 全量 47 清单条目 Oracle A
- 真实游戏回归 ≥ 20 个
- 性能 baseline 确认（与 v1.17 比较，≤ ×1.05）
- Release 构建完整通过（PGO/LTCG）
- 文档同步（BuildGuide、ChangeLog）

### 1.2 ❌ 不在范围内
- 删除 C++ CPU/PPU 源（Phase 8，且仅删 newppu=1 路径）
- 移除兼容垫片（Phase 8）
- 移除 `VNESU11_CORE=OFF` 编译路径（v2.1 才做）
- **移除 newppu=0 旧 PPU**（保留，ADR-009）

---

## 2. 任务清单

### 2.1 CMake 默认切换

```cmake
# CMakeLists.txt（根）
option(VNESU11_CORE "Use vNESU11 Rust core (default ON in v2.0+)" ON)  # 改默认

if(NOT VNESU11_CORE)
    message(WARNING "VNESU11_CORE=OFF: using legacy C++ CPU/PPU. This path will be removed in v2.1.")
endif()
```

### 2.2 Shadow Run 切换为可选

```cmake
# VNESU11_CORE=ON 时，shadow run 默认开；可关闭
option(VNESU11_SHADOW_RUN "Run C++ and vNESU11 side-by-side for equivalence check" ON)

if(NOT VNESU11_SHADOW_RUN)
    add_definitions(-DVNESU11_DISABLE_SHADOW=1)
endif()
```

Release 构建（PGO）建议关闭 shadow run 以获得真实性能数据：
```powershell
cmake -B build-release -DVNESU11_CORE=ON -DVNESU11_SHADOW_RUN=OFF -DFCEUX11_PGO=ON
```

### 2.3 blargg 全量验证

```bash
# kagami-qa-runner Oracle B 全量
cd build
./kagami-qa-runner --matrix ../tests/tests.json --oracle B --tag blargg --output report.html

# 期望：177 ROM 中 ≥ 144 PASS（与 v1.17 一致），零回归
```

### 2.4 真实游戏回归（newppu=1）

| 游戏 | 测试时长 | 验证 |
|------|---------|------|
| SMB1 | 5 min | shadow run 帧级 CRC 0 diff |
| Zelda | 5 min | shadow run 帧级 CRC 0 diff |
| Metroid | 5 min | shadow run 帧级 CRC 0 diff |
| Contra | 5 min | shadow run 帧级 CRC 0 diff |
| Castlevania | 5 min | shadow run 帧级 CRC 0 diff |
| Mega Man 2 | 5 min | shadow run 帧级 CRC 0 diff |
| Kirby | 5 min | shadow run 帧级 CRC 0 diff |
| Final Fantasy | 5 min | shadow run 帧级 CRC 0 diff |
| Tetris | 5 min | shadow run 帧级 CRC 0 diff |
| 9 其它（覆盖各种 mapper） | 3 min each | shadow run 帧级 CRC 0 diff |

> **[修订] newppu=0 回归**：另选 5 个游戏用 newppu=0 跑（C++ 旧 PPU 回退路径），
> 确认无回归（movie PPUflag=0 兼容）。

### 2.5 性能对比

```bash
# Release 构建（PGO）
build-release/src/fceux11.exe -rom smb1.nes -bench 60

# 对比 v1.17 Release
git checkout v1.17
build-release/src/fceux11.exe -rom smb1.nes -bench 60
git checkout wip_v2.0

# 报告：帧时间、CPU cycles、cache miss（VTune 或 perf）
```

### 2.6 文档同步

- `docs/BuildGuide.md`：更新构建步骤（默认 `VNESU11_CORE=ON`）
- `docs/ChangeLog.md`：增补 `[2.0.0]` 条目
- `README.md`：性能数据 / 迁移说明
- `docs/wip_2.0_plan/`：每个 phase 文件标记完成

### 2.7 CI 工作流

```yaml
# .github/workflows/ci.yml
- name: Build with vNESU11 (default ON)
  run: cmake -B build -DVNESU11_CORE=ON && cmake --build build

- name: Build with vNESU11 OFF (legacy)
  run: cmake -B build-legacy -DVNESU11_CORE=OFF && cmake --build build-legacy
  continue-on-error: false  # 必须也通过

- name: Run kagami-qa Oracle A+B
  run: ./build/kagami-qa-runner --matrix tests/tests.json

- name: Run shadow run regression
  run: ./build/src/fceux11.exe --shadow-run-tests tests/shadow_run/roms/
```

---

## 3. 验证策略

### 3.1 全量 blargg（Oracle B）

177 个 ROM 通过 kagami-qa-runner。期望：

| 类别 | 数量 | 期望 |
|------|------|------|
| 完全 PASS | 144+ | 与 v1.17 持平 |
| 部分 FAIL（v1.17 已 FAIL） | 33 | 不增加 |
| 新 FAIL（回归） | **0** | **必须** |

任何新 FAIL = 阻塞 v2.0 发布。

### 3.2 kagami-qa Oracle A

47 清单条目全 PASS（与 v1.17 39P + 8F advisory 一致）。

### 3.3 mapper_byte_diff 175 case

100% parity（与 v1.17 一致）。

### 3.4 savestate byte diff

v1.17 golden savestate 文件，加载后跑 60 帧，再保存 → 字节一致。

### 3.5 TAS movie round-trip

- 录 5 分钟 SMB1 TAS movie
- 加载 movie 录放 5 分钟
- 帧缓冲 / 音频 buffer byte diff

---

## 4. 性能验证

### 4.1 Release 性能基线（[修订] S8——持平定义）

| 项目 | v1.17 (C++) | v2.0 (vNESU11) | 差异 |
|------|-------------|----------------|------|
| SMB1 帧时间 | X ms | Y ms | Δ = (Y-X)/X |
| Zelda 帧时间 | X ms | Y ms | Δ |
| blargg cpu_instrs 总时间 | X ms | Y ms | Δ |

**目标**：Δ ≤ +5%（允许 ±5% 浮动，持平定义）。**性能不是验收项**——
正确性等价（savestate / blargg / shadow run）才是。若 Δ > +5%，触发
`A_performance_model.md §5` 的备选方案决策。

### 4.2 Profiling

- `cargo flamegraph -p vnesu11` 找出 Rust 热点
- `VTune` 或 `perf` 分析 C++ 端残留
- 报告：top-10 热点函数位置

### 4.3 Memory profile

- `vNESU11` RSS / 堆使用 vs v1.17 C++
- sprite_lut 512 KiB 确认 `LazyLock` 后只在首次使用分配

---

## 5. 关键技术决策

### 5.1 灰度策略

Phase 7 不是一刀切——分两周：

**第 1 周**：
- `VNESU11_CORE=ON` 默认
- shadow run ON
- CI 跑两套（ON + OFF）必须都通过
- 真实游戏回归

**第 2 周**：
- 移除内部 `VNESU11_CORE=OFF` 编译路径的 `#ifdef`（保留开关）
- Release 构建用 PGO + shadow OFF
- 完整 CI 流水线（Oracle A + B + 真实游戏 + 性能）
- 文档发布

### 5.2 回退预案

如果 Phase 7 发现严重问题：

1. **立即回退**：把 `option(VNESU11_CORE ... ON)` 改回 `OFF`，发 v1.18 hotfix
2. **修复后再切**：保留 wip_v2.0 分支继续修，下次小版本再切
3. **不回退**：用 `VNESU11_CORE=OFF` 作为长期回退（v2.1 前保留）

### 5.3 性能不达预期的处理

如果 Release 帧时间比 v1.17 高 ≥ 10%：

- Step 1：profile，找出回归点
- Step 2：针对性优化（一般是 PPU 渲染 hot path）
- Step 3：备选 dynarec（仅在确实必要，且 ROI 明确时启动，**默认不做**）

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| blargg 177 ROM 中出现新 FAIL | 🔴 高 | 阻塞发布；hotfix 修复 |
| 真实游戏在某 mapper 上崩 | 🟠 中 | shadow run 早期捕获；fallback to C++ mapper |
| Release 性能回归 ≥ 10% | 🟠 中 | profile + 优化；备选 dynarec |
| TAS movie round-trip 不一致 | 🔴 高 | 阻塞发布；调试 IRQ / Joypad 时序 |
| MSVC PGO 与 Rust LTO 冲突 | 🟡 低 | 分阶段测；备选关闭 Rust LTO |
| 文档遗漏 | 🟡 低 | Phase 7 末强制 review |

---

## 7. DoD

- [ ] `VNESU11_CORE=ON` 默认；可执行文件启动 + 加载 ROM 正常（newppu=1）
- [ ] `VNESU11_CORE=OFF` 仍能编译（回退路径）
- [ ] kagami-qa Oracle B 全 177 ROM（newppu=1）：**新 FAIL = 0**
- [ ] kagami-qa Oracle A 全 47 清单：**新 FAIL = 0**
- [ ] mapper_byte_diff 175 case：**新 FAIL = 0**
- [ ] savestate round-trip：v1.17 golden 一致
- [ ] TAS movie round-trip：5 分钟字节一致（newppu=1 与 newppu=0 各一组）
- [ ] 真实游戏 20 个跑 5 分钟：无 crash，shadow run 帧级 CRC 0 diff
- [ ] FDS / NSF / VS 各 1 个测试文件通过
- [ ] Release 帧时间：≤ v1.17 + 5%
- [ ] PGO 构建：通过
- [ ] 文档全部更新（BuildGuide、ChangeLog、README）
- [ ] CI 全绿
- [ ] 发布 tag：`v2.0.0`

---

## 8. 关键文件交付

```
修改：
  CMakeLists.txt                       # VNESU11_CORE=ON 默认
  .github/workflows/ci.yml             # 新增 vNESU11 工作流
  docs/BuildGuide.md                   # 默认 ON 说明
  docs/ChangeLog.md                    # [2.0.0] 条目
  README.md                            # vNESU11 章节
  src/rust/crates/vnesu11/Cargo.toml   # 版本 0.9 → 1.0
```

下一步：[phase_8_cleanup.md](./phase_8_cleanup.md)
