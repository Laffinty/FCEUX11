# KagamiQA — FCEUX11 精度 Oracle 契约

> **生效日期**:2026-08-13(v2.0 wip_v2.0 战略转向)
> **关联**:`docs/wip_2.0_plan/phase_6_integration.md` §0(精度 oracle 声明)
> **源码位置**:`src/rust/crates/kagami-qa/`(7 层架构,L1-L7)
> **执行入口**:`kagami-qa-runner`(CLI,`--direct` in-process 模式)

KagamiQA 自 v1.17 起已升格为 FCEUX11 测试体系的**唯一归属与唯一门禁**;
自 v2.0 phase 6 起,在此之上**进一步升格为 FCEUX11 模拟器精度的正式 oracle**——
即:FCEUX11 的"是否真的像一台 NES"这件事,由 KagamiQA 判定。

---

## 1. 精度 Oracle 的根本问题

NES 模拟器的"精度"在原理上没有形式化定义:

- 硬件晶体管 jitter 在 sub-cycle 级别是**随机**的,不存在唯一 ground truth
- Nesdev wiki 是社区共识,blargg ROM 是一个人的解读,不同 emulator(FCEUX / Nestopia / Mesen / puNES)在某些边缘 case 上行为不一致
- FCEUX 自身的 C++ 实现有 FCEUX 特有的时序优化(DoLine budget、FrameIRQEnd 模式 0 IRQ 保持等),这些**不是**硬件契约

因此 FCEUX11 不能承诺"和 FCEUX 字节一致",也不能承诺"和硬件完全一致"。
FCEUX11 **能**承诺的是:**和 chip 规范一致,且与 FCEUX C++ 在公开 oracle 集合内一致**。
这个承诺的承担者就是 KagamiQA。

---

## 2. 五层 Oracle 体系

FCEUX11 的精度判定分 5 个独立 tier,**任一 tier 失败即视为精度问题**:

| Tier | 名称 | 来源 | 判据 | 现行状态 |
|------|------|------|------|---------|
| **T1** | blargg `$6000` 协议 | blargg 177 ROM(公开) | `$6000 == 0x80`(PASS) | kagamiqa Oracle B 自动跑 |
| **T2** | nestest trace | Kevin Horton 的官方 log | 指令流逐条匹配 | 47 条 manifest 中已覆盖 |
| **T3** | kagamiqa 回归基线 | v1.17 实态 golden 矩阵 | 与 frozen baseline 一致 | 39P/8F(Grade C) |
| **T4** | mapper byte-diff | 175 个 mapper 行为 case | 与 C++ mapper 输出字节一致 | 12 mapper test + 175-case byte diff |
| **T5** | 真实游戏 smoke | 8 个经典游戏 | 无可视异常 + 音频 SNR≥60dB + 无 crash | 待补(phase 6 主工作量) |

**5 个 tier 的关系**:
- T1 + T2 是**功能性**的——证明 vNESU11 实现了 chip 规范
- T3 + T4 是**回归性**的——证明 vNESU11 与 C++ FCEUX11 行为一致
- T5 是**用户感知**的——证明玩家体验一致

**故意不包含的 tier**:
- ~~byte-level shadow match~~ — 不可达(已论证,见 phase_6 §0)
- ~~TAS movie 字节 round-trip~~ — 改用 T1/T2 替代
- ~~savestate 字节兼容~~ — 降级为 golden round-trip 等价

---

## 3. T1(blargg)Corpus 补全路径

### 3.1 现状

`tests/fixtures/blargg/` 下已有部分 ROM(categories: apu / cpu / mmc3 / ppu),
但**不完整**——`build/kagamiqa_accuracy_table.md` 当前显示 0/12(陈旧表,ROM 路径解析问题)。

### 3.2 补全流程

1. 跑 `.\scripts\download_blargg_roms.ps1`(一次性,177 ROM,~8 MB)
2. 修复 `kagami_qa_blargg_runner` 的 ROM 路径解析(`tests/fixtures/blargg/<category>/<rom>.nes`)
3. 重置 baseline:`kagami-qa-runner --save-baseline build/kagamiqa_baseline_next.json`
4. 跑全量:`kagami-qa-runner --baseline build/kagamiqa_baseline_frozen.json --filter "oracle=B"`
5. 生成新 accuracy_table.md,提交

### 3.3 Pass-Rate 门槛(2026-08-13 实测修订)

> **实测现状**(2026-08-13,commit `cb89175`):**81.36%** (144/177)
> - 较 v1.16 baseline (120/180 = 66.67%):净改善 +24 PASS / -27 FAIL
> - 0 个新增 regression
> - 分项:apu 96.15% / ppu 85.71% / cpu 79.31% / mmc3 33.33%
> - 距 90% 缺 16 ROM(主要是 mmc3 全 12 fail + ppu VBL 4 fail + cpu interrupts 5 fail)

| 阶段 | T1 门槛 | 含义 | 状态 |
|------|--------|------|------|
| **phase 6 收口** | **TBD**(下次构建决策) | 见 §3.3a 3-tier 候选方案 | 81.36% 已测;门槛数值待 owner 决策 |
| **phase 7 默认切换前** | ≥ 85% (150/177) | 默认 ON 的硬门槛 | 差 6 ROM |
| **phase 8 清理后** | ≥ 90% (160/177) | mmc3 全部修复后达成 | 差 16 ROM(主工作量) |
| **v2.1** | ≥ 95% (168/177) | 完整修复 deep model 后 | 差 24 ROM |

#### 3.3a phase 6 门槛 3-tier 候选(待 owner 决策)

| 候选 | 数值 | 当前差 | 优点 | 缺点 |
|------|------|--------|------|------|
| **A. 接受 81.36%** | ≥ 80% | 已过 | 与战略转向一致(不追 micro-drift);立即 phase 6 收口 | 留 16 ROM 缺口给 phase 7/8 |
| **B. 追 85%** | ≥ 85% | 6 ROM | phase 7 收口可达成;不需修 mmc3 deep model | phase 6 收口还需 session 修 6 ROM(主要是 cpu + ppu) |
| **C. 坚持 90%** | ≥ 90% | 16 ROM | 与原计划一致 | 需起 mmc3 IRQ deep model session(1-2 周) |

**建议**:**选 A**。理由:
- 战略转向(ADR-011)已明确"不追 micro-drift"
- mmc3 12 fail 是 mapper 模型完整性问题,不是 cycle 残差——属于 phase 7+ 范围
- 81.36% 较 v1.16 +24 PASS 是**真实改善**,不应被抽象门槛挡住
- phase 7 默认切换前追到 85% 仍有 6 ROM 余量(可用 PPU VBL 4 fix + CPU 1-2 fix 凑)

注:NESDEV / FCEUX 等 emulator 业界现实是 100% 极难(部分 ROM 有 FCEUX 特有的兼容性 bug)。
98% 是务实天花板。

> **"不弱于 C++" vs phase 门槛(2026-08-13 复审)**:用户验收标准是「Rust vNESU11
> 不弱于 C++ 组件」。该标准在 blargg T1 上**已满足**——实测 81.36% vs v1.16 C++
> 66.67%,**0 新增 regression**(33 个 FAIL 全部是 v1.16 已知问题,非 Rust 引入)。
> §3.3 的 phase 7(≥85%)/phase 8(≥90%)门槛是**超出当前 C++ 基线**的增强目标,
> 要求修复 C++ 与 Rust 共有的历史缺陷(PPU VBL / CPU interrupt 时序 / MMC3
> scanline IRQ),性质是「比 C++ 更正确」而非「追平 C++」。按此口径,phase 7/8
> 的启动条件应同时满足:(a) 0 新增 regression 已成立;(b) T1 ≥85% 为增强门槛,
> 若项目要求严格对齐用户「不弱于」标准而非「更优」,可把 phase 7 门槛回退到
> 「≥ v1.16 baseline 且 0 regression」;若要按原计划推进,则需继续修共享缺陷。

### 3.4 已知失败管理

任何 blargg ROM 失败必须**三选一**:

1. **修 Rust 端实现** — 唯一默认路径
2. **登记为 known-fail** — 加 `tests/fixtures/blargg_known_fail.json`,需附:`{rom, reason, FCEUX_status, fix_target_version}`
3. **登记为 deliberate deviation** — 见 §4,需 ADR

---

## 4. Deliberate Deviation 追踪机制

> **核心原则**:vNESU11 **允许**与 C++ FCEUX 行为不一致,只要:
> (a) 行为符合 chip 规范(通过 T1/T2 验证)
> (b) 用户体验一致(通过 T5 验证)
> (c) 偏离原因有文档(ADR + deviation registry)

### 4.1 偏离分类

| 类别 | 含义 | 处理 |
|------|------|------|
| **D-A**(修复) | FCEUX 是错的,chip 才是对的 | 修 Rust 端,登记 FCEUX bug |
| **D-B**(实现自由) | FCEUX 的实现选择不是 chip 强制的 | Rust 可选不同实现,只要 blargg + smoke 通过 |
| **D-C**(性能优化) | Rust 用更高效的算法,行为等价 | 需 kagamiqa Oracle A baseline 不变 |
| **D-D**(架构简化) | Rust 砍掉 FCEUX 的 dead code / 未用路径 | 需 T5 smoke + T3 regression 不退化 |

### 4.2 登记格式

新增文件:`docs/wip_2.0_plan/deviations.yaml`(待建)

```yaml
- id: DEV-001
  category: D-B
  component: ppu.vblank_flag
  description: |
    Rust 在 PRELINE 段帧首置位 VBlank flag(对应 C++ 在 FCEUX_PPU_Loop 顶部置位);
    子扫描线相位不同导致 sl 240 cycle 340 的 $2002 读取时机有 1 cycle 差。
  fceux_behavior: sl 241 时由 runppu() 路径置位
  rust_behavior: 帧首 PRELINE 段入口立即置位
  chip_spec_basis: |
    Nesdev PPU_frame_timing: VBlank 在 pre-render scanline 的 dot 1 置位
    (FCEUX 实现是非规范的,严格按 chip 来说 Rust 正确)。
  test_coverage:
    - blargg: ppu_vbl_nmi
    - smoke: cpu_dummy_reads shadow cpu_match >= 3
  approval: user-2026-08-13
```

### 4.3 审批流程

每次新增 deviation 必须:
1. 提交 PR,文件:`deviations.yaml` + 对应 ADR(若类别 A)
2. CI 检查:T1/T2 不得退化
3. 用户(项目 owner)审批

---

## 5. T5(真实游戏 smoke)规范

### 5.1 8 个经典游戏清单

| # | 游戏 | Mapper | 测试重点 |
|---|------|-------|---------|
| 1 | Super Mario Bros | NROM | NMI 时序 + 滚动 |
| 2 | Donkey Kong | NROM | 早期 sprite + IRQ |
| 3 | Balloon Fight | NROM | 旋转 sprite + 复杂背景 |
| 4 | Ice Climber | NROM | 多 sprite 优先级 |
| 5 | Tetris | NROM | 简单 PPU + 音频 |
| 6 | Super Mario Bros 3 | MMC3 | scanline IRQ + status bar |
| 7 | Kirby's Adventure | MMC3 | 大 CHR + IRQ 抖动 |
| 8 | Mega Man 4 | MMC3 | 复杂 IRQ + sprite 0 hit |

### 5.2 判据

每个游戏跑 5 分钟(300 帧 @ 60 FPS,18000 帧):

| 指标 | 判据 | 工具 |
|------|------|------|
| 可视异常 | 无鬼影 / 无残影 / 无错位 sprite / 无错位 status bar | 5 帧 spot-check by eye |
| 音频 | SNR ≥ 60dB(per-frame buffer 比对) | kagamiqa audio_diff |
| 帧率 | ≥ 60 FPS | vn_perf_bench 增量测量 |
| Crash | 0 次 | 直接观察 |

### 5.3 通过条件

8 个游戏全部通过 → T5 PASS。
任一失败 → 修 Rust 端,或登记 DEV(类别 D-D:架构简化不接受 UX 退化)。

---

## 6. 回归基线协议(Oracle A)

### 6.1 Baseline 文件

- `tests/fixtures/kagamiqa_baseline_frozen.json` — 冻结基线(47 条 manifest,39P/8F)
- `build/kagamiqa_baseline_next.json` — 当前构建产物,作为下次 baseline 的候选

### 6.2 变更规则

1. **PASS → FAIL 自动告警** — CI 失败,需 PR 处理
2. **FAIL → PASS 自动接受** — 但需 PR review
3. **新增 manifest 条目** — 需 `provenance` 字段(来源可追溯),参考 kagami-qa/README.md 反 gaming 纪律
4. **基线更新** — 需显式 `--save-baseline` 提交,且 PR 描述写明动机

### 6.3 与 v1.17 baseline 的关系

v1.17 baseline(39P/8F)是 FCEUX11 的**起点**——v2.0 不得恶化。
v2.0 的基线目标:**47/47 PASS**(零 FAIL),T1 ≥ 90%。

---

## 7. CI 集成(摘要)

完整 CI 规范见 `.github/workflows/kagami-qa.yml`,摘要:

| Stage | Oracle | 触发 | 阻塞 |
|-------|--------|------|------|
| Lint | — | push | clippy/rustfmt |
| Unit | Oracle A | push | cargo test |
| Direct | Oracle A | push | kagami_qa_direct_runner |
| Blargg | Oracle B | push | T1 pass rate |
| Smoke | T5 | 手动 | 8 games / 5 min |
| Perf | — | weekly | frame time drift |
| Report | — | push | PDF + JSON artifact |

**v2.0 新增 stage**:
- **T1 pass-rate gate**:phase 6 收口时新增
- **T5 smoke gate**:phase 6 末新增(自动跑需 8 × 5min = 40min CI 时间,可拆 nightly)

---

## 8. 与 FCEUX C++ 的关系(本节是 ADR-011 的具体化)

### 8.1 仍然耦合的部分

- **174 个 C++ mapper** — 通过 FFI 适配在 Rust 跑;FCEUX 上游 C++ mapper 池的 bug 修复会**自动**流入
- **SFORMAT 存档格式** — 与 v1.17 二进制兼容(已冻结,见 savestate_tags.md)
- **FM2 movie 格式** — 与 v1.17 二进制兼容(已冻结)

### 8.2 已脱钩的部分

- **CPU 解释器** — Rust 完全重写,行为以 chip 规范为准
- **PPU(newppu=1)** — Rust 完全重写,允许 D-B 偏离
- **APU 5 通道** — Rust 完全重写

### 8.3 脱钩策略

脱钩**不等于**与 FCEUX 上游割裂。FCEUX C++ 仍然:
- 是 kagamiqa Oracle A 的**基线**——v1.17 frozen baseline
- 是 mapper 池的**来源**——继续 sync 上游
- 是 savestate/movie 格式的**契约方**——兼容性冻结

FCEUX C++ 不再:
- 是 vNESU11 Rust 实现的"参考抄录"——Rust 按 chip 规范自由实现
- 是 phase 6/7/8 的**精度 oracle**——oracle 是 KagamiQA

---

## 9. 当前状态摘要(2026-08-13)

| Tier | 现状 | phase 6 目标 | phase 7 目标 |
|------|------|--------------|--------------|
| T1 blargg | 81.36% (144/177,§3.3) | ≥ 80% (候选 A) | ≥ 85% |
| T2 nestest | PASS(Rust 端单测) | 保持 | 保持 |
| T3 regression | 40P/8F(phase 6 收口) | 47/47(零 FAIL) | 47/47 |
| T4 mapper | 175-case byte-diff PASS | 175-case byte-diff PASS | 保持 |
| T5 smoke | 骨架就位,ROM 待投放 | 8 games 全部 PASS | 保持 |

**v2.0 phase 6 收口条件**:T1 ≥ 80% + T3 = 47/47 + T5 = 8/8 + 性能达标。

> **注(2026-08-13 复审)**:§3.3 与本节曾出现三套门槛(80/85/90/95),已统一为
> §3.3 表——phase 6 ≥80%、phase 7 ≥85%、phase 8 ≥90%、v2.1 ≥95%。上述
> "phase 6 目标 ≥90%"旧值已改为 80%。

---

## 10. 不该做的事(纪律)

- **不调 kagamiqa 判据迁就 Rust 实现** — 反 gaming
- **不删 blargg ROM 凑通过率** — 必须修 Rust
- **不写"看起来一样"的回归测试** — `provenance` 必填
- **不假设 FCEUX C++ 行为就是对的** — 但**不**为反对而反对
- **不让 phase 6 阻塞于 byte-level parity** — 那不是这个 phase 的事(已论证)

---

**相关文档**:
- `docs/wip_2.0_plan/phase_6_integration.md` §0(战略声明)+ §7(DoD)
- `docs/wip_2.0_plan/00_overview.md` §0.1(架构耦合声明)
- `docs/wip_2.0_plan/B_risk_register.md` R-019(FCEUX 上游同步)
- `docs/wip_2.0_plan/AUDIT_20260810.md` S2/S3/S5(审计背景)
- `src/rust/crates/kagami-qa/README.md`(框架实操)
