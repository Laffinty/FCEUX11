# 决策点理论分析 · S3 时序模型 & S5 newppu 策略

> **日期**：2026-08-10
> **背景**：`AUDIT_20260810.md` 提出两个架构决策点（S3 时序模型 A/B、S5 newppu 选项 ①/②/③），
> 架构组未定。本文通过联网检索（2026-08-10，来源见 §4）从理论层面分析最优解。
> **结论**：S3 → **决策 A（v2.0）+ 架构预留决策 B 接口（v2.1+）**；S5 → **选项③**。

---

## 1. S3 · 时序模型：budget 复刻（A）vs dot 重写（B）

### 1.1 理论框架：翻译 vs 重写

| 维度 | 决策 A：budget 复刻 | 决策 B：dot 紧交错重写 |
|------|-------------------|----------------------|
| 本质 | **翻译**（translation）：行为等价移植 | **重写**（rewrite）：行为改变 + 模型升级 |
| 正确性目标 | 与现状逐帧等价（81.4% blargg） | 向 Mesen 级精度靠拢（接近 100%） |
| 风险 | 低（shadow run 可验证等价） | 高（PPU pipeline + NMI sampling 重写） |
| 是否改变产品定位 | 否 | 是（工具链仿真器 → 精度仿真器） |
| 可验证性 | 帧级三级 diff 即可 | 需 blargg 全量 + 硬件对比重新建立基线 |

### 1.2 检索证据（关键发现）

**证据 1：FCEUX 的 33 个 blargg FAIL 是"模型层面"限制，不是实现 bug**

> "FCEUX's PPU/CPU timing model is approximate... deliberately not a full
> cycle-accurate recreation of every PPU pipeline quirk, NMI sampling window,
> or 341-dot scanline edge case. This is a long-standing, acknowledged tradeoff."
>
> "full Blargg compliance is a **non-trivial rewrite of the PPU pipeline and
> NMI sampling**."

含义：决策 B 不是"顺带修几个 bug"，是**换掉 PPU pipeline 的时序模型**——
这是 Mesen 级的工程（Mesen 花了多年迭代）。

**证据 2：社区把 FCEUX 的 blargg 失败视为"expected"，不是回归**

> "Treat Blargg `vbl_nmi_timing` failures as **expected rather than regressions**
> unless a recent FCEUX change made them worse."

含义：FCEUX 的生态位从来不是"精度之王"（Mesen 才是），而是**工具链**
（Lua、TAS rerecording、调试器）。把精度当作 v2.0 目标 = 改变产品定位。

**证据 3：scanline/catch-up 模型 vs cycle-accurate 的经典权衡**

> "Scanline-based: faster, simpler, good enough for the large majority of
> commercial games. Cycle-accurate: highest compatibility with obscure games,
> demos, and test suites; **higher CPU cost; more complex implementation**."

含义：决策 B 的性能/复杂度代价真实存在，收益集中在"小众游戏 + 测试 ROM"，
对主流商业游戏用户无感。

### 1.3 理论最优解：两阶段渐进

```
v2.0（本次迁移）      → 决策 A：budget 复刻
                          · 风险隔离：迁移 ≠ 模型重写，两类风险不互相污染
                          · 行为等价：shadow run 可验证，工具链零破坏
                          · 保住 FCEUX 生态位（TAS 工具链）

架构预留               → Scheduler 段模型内部就是 dot 推进
                          · "段粒度 → dot 粒度"是增量细化，不是推倒重来
                          · ADR-008 记录决策 B 的接口预留

v2.1+（独立项目）     → 决策 B：dot 重写（可选）
                          · 若用户/社区对精度有硬需求
                          · 单独立项，独立验证基线（blargg 全量 + 硬件对比）
                          · 不污染 v2.0 的迁移成果
```

**为什么不直接选 B**：
1. 决策 B 不是"修 bug"是"换模型"，混入 v2.0 = 同时承担迁移 + 重写两类风险
2. FCEUX 社区把精度限制视为 expected；用户要精度会去用 Mesen（搜索原话：
   "If you need the tests to pass cleanly, switch to Mesen"）
3. 决策 B 改变产品定位（工具链 → 精度），这是产品决策，不该塞进技术迁移

**为什么不直接选 A 且不留接口**：
- 若架构上把段模型写死（无 dot 粒度接口），未来想升级到 B 需要推倒 Scheduler——
  把"可选升级"变成"必须重写"，丧失了渐进路径

---

## 2. S5 · newppu 策略：①拒绝旧 movie / ②双 PPU 全迁 / ③旧 PPU 保留回退

### 2.1 理论框架：兼容性 vs 技术债

| 维度 | ①拒绝旧 movie | ②双 PPU 全迁 Rust | ③旧 PPU 保留 C++ 回退 |
|------|--------------|------------------|----------------------|
| TAS 旧 movie 兼容 | ❌ 全毁 | ✅ | ✅ |
| 代码统一度 | 最高 | 最高（Rust 全统一） | 中（C++ 旧 PPU 并存） |
| 工作量 | 最低 | **最高**（old PPU ~1000 行迁移价值极低） | 低（只保留，不迁移） |
| 上游先例 | 无 | 无 | **FCEUX 上游现行做法** |
| 技术债 | 无（但制造用户债） | 低 | 中（旧 PPU 长期并存） |

### 2.2 检索证据（关键发现）

**证据 1：FCEUX 上游保留双 PPU 就是为了"不一夜之间毁掉所有旧 movie"**

> "The dual-PPU design was retained so users (especially TASers) could choose
> compatibility vs. improved behavior rather than **breaking every old movie
> overnight**."

含义：FCEUX 官方**已经做过一次这个决策**——结论就是双实现并存。选项①
（拒绝旧 movie）正是上游刻意避免的结果。

**证据 2：TAS 社区标准实践 = "新 TAS 用 new PPU，旧 movie 用 old PPU"**

> "Match the PPU (and other settings) used when the movie was created.
> Prefer New PPU for new work... unless an old movie requires Old PPU."

含义：选项③（保留旧 PPU 为回退）不是"妥协"，是**社区既定的工作流**。
`movie_io.cpp:138` 记录 `PPUflag` 正是为此。

**证据 3：old PPU 的价值定位 = legacy 兼容层**

> "Old PPU... historically the default for a long time. Many older TAS movies
> and savestates were made with it; playback often requires matching the Old
> PPU setting for compatibility."

含义：old PPU 不是 hot path、不是精度追求，**纯粹为旧存档/旧 movie 存在**。
把它迁到 Rust 的工程价值极低（选项②），保留 C++ 是合理的技术债。

### 2.3 理论最优解：选项③

```
理由链：
1. TAS 兼容是 FCEUX 的立身之本（工具链生态位）→ 选项①违反产品定位
2. FCEUX 上游已实践选项③（双 PPU 并存）→ 有先例，不是理论空想
3. old PPU 迁移 Rust 价值极低（纯 legacy 兼容层）→ 选项②是无效投入
4. v2.1 可选收尾：若 old PPU 的使用率降到可忽略，再决定是否迁移/移除

结论：选项③ = 与 FCEUX 上游实践、TAS 社区标准、工程价值判断三者的
      **唯一一致解**。
```

---

## 3. 两决策点的联动效应

S3 与 S5 不是孤立的，选 A + ③ 有**协同**：

| 组合 | 效果 |
|------|------|
| 决策 A + 选项③ | newppu=1 走 Rust（budget 复刻，行为等价）；newppu=0 走 C++ 旧 PPU（原样）。**两套路径都是现状行为的忠实复刻，shadow run 全部可验证** |
| 决策 B + 选项③ | 精度提升只在 newppu=1 生效；newppu=0 旧 movie 行为不变（**复刻的"新旧 PPU 行为差异"保持**）——这其实是正确行为：旧 movie 就该按旧 PPU 行为放 |
| 决策 B + 选项① | 精度提升 + 拒绝旧 movie——双重破坏兼容性，最差组合 |

**推荐组合：决策 A + 选项③**（v2.0），预留 B 的架构接口（v2.1+ 可选）。

---

## 4. 检索来源（2026-08-10）

| # | 检索词 | 关键来源 | 支撑结论 |
|---|--------|---------|---------|
| 1 | "NES emulator CPU PPU timing model scanline based vs cycle accurate Mesen Nestopia" | NESDev Wiki、Mesen/Nestopia 文档 | S3 证据 3：scanline vs cycle-accurate 权衡 |
| 2 | "FCEUX new PPU old PPU two implementations TAS timing accuracy" | FCEUX 文档、TASVideos 指南、社区讨论 | S5 全部证据 + S3 背景 |
| 3 | "FCEUX blargg test failures vbl nmi timing known limitations" | FCEUX 社区、nesdev 论坛 | S3 证据 1/2：33 FAIL 是模型层面限制、视为 expected |

---

## 5. 落地动作（若采纳）

- [ ] `README.md` ADR-008 标注"决策 A + 架构预留 B 接口（v2.1+ 可选）"
- [ ] `README.md` ADR-009 标注"选项③，依据：FCEUX 上游实践 + TASVideos 指南"
- [ ] `02_architecture.md §4` 补一句"Scheduler 段粒度可细化为 dot 粒度（预留决策 B 接口）"
- [ ] `phase_3_ppu.md` 补"newppu=0 与 newppu=1 的行为差异需保留（旧 movie 按旧 PPU 行为放）"
- [ ] `B_risk_register.md` R-015 状态更新："决策 A 已定（2026-08-10）"
