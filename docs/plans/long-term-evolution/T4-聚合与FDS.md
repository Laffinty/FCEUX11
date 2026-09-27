# 分项计划 T4 · 聚合收敛 与 FDS 子系统

> **STATUS: OPEN** · **难度** ★★★★★（结构）/ 依赖上游 cascade
> **基线**：run 98252605787 · `git_rev=b0658c9` · 106P/14F
> **当前**：`b6fe92a` / run 98267393461 / **107P / 13F** / grade B / **① 056 已 DONE**，余 13 项
> **包出口**：矩阵 FAIL **→ 0** 或有据 known_limit 全覆盖
> **主报告**：§三 ⑬⑭

---

## 包目标

两条「收口型」缺口：一个聚合器（等上游修完自然收敛）+ 一个双故障子系统（先加载后 IRQ）。

| 序 | kgmqa | 主题 | 预估 | 状态 |
|---|---|---|---|---|
| 13 | **077** holy_mapperel | 13 mapper / 47 ROM 聚合 | cascade | TODO |
| 14 | **081** fds_irq_tests | FDS 加载 0xFE + IRQ | 2–4 周 | TODO |

**依赖**：

- 077 依赖 ② 078 / ④ 097 / ⑤ 096 及 L2 mmc3 组进展 —— **禁止整包硬攻**；
  排期口径见主报告 §2.1：**β 收口即重估**，不押到 ε
- 081 与 T2/T3 无硬依赖，可单独开线

---

## ⑬ kgmqa-077-holy-mapperel-tepples（cascade）

| 字段 | 值 |
|---|---|
| 协议 | `aggregate-mapperel`（13 mapper / 47 ROM 内部循环） |
| known_limit | Holy Mapperel aggregate 13 mappers / 47 ROMs; partial mapper coverage |

### 现象

47 子 ROM 聚合，**全过才 PASS**。本地 accuracy 表常见 `M0_*/M1_*/M4_*/M11_*` 等 FAIL。

### 根因

多个 mapper 边界缺口的**合成结果**，不是单一 bug。

### 步骤（严禁整包硬攻）

- [ ] **归组**：47 子 ROM 按 mapper 分（M0/M1/M3/M4/M9/M10/M11/M18/M34/M66/M69/M78/M118…）
- [ ] **单组复现**：runner 循环内日志定位到子 ROM
- [ ] **优先清**：
  1. MMC1 组（吃 T0-078）
  2. FME-7 / 简单 UNROM 组（吃 T1-097/096）
  3. MMC3 IRQ 组（与 L2 `mmc3_*` 呼应）
  4. 其余长尾
- [ ] **每组一个 PR**；观察 077 是否仍 FAIL

### 验收

kgmqa-077 PASS（47/47）。

### 风险

单项中等 / 整包高。禁止一次改 13 个 mapper。  

> **排期口径（覆盖上方「预估 cascade」的读法）**：077 的成本是**派生的** ——
> = ② 078 + ④ 097 + ⑤ 096 的剩余部分 + L2 MMC3 IRQ 组。它不单独立项、不额外占窗口，
> 随 β/δ 顺带推进。**β 收口时立即重估剩余子 ROM 数**：≤ 10 且不含 MMC3 IRQ 组 → 并入 δ 尾部；
> 仍集中在 MMC3 IRQ 组 → 归入 T2 模型级工作的副产品，单独立项。
> **度量只看子组完成数**，47/47 是合取判定，滞后于任何单点进展。
**进度度量**：用子组完成数，不要只看 077 总码。

---

## ⑭ kgmqa-081-fds-irq-tests-sour

| 字段 | 值 |
|---|---|
| ROM | `sour/fdsirqtests/fdsirqtestsV7_patched.fds` |
| 错误码 | **`0xFE` = ROM load failure**（duration 55ms） |
| known_limit | FDS IRQ timing subsystem; not v1.8 focus |

### 现象（双故障）

1. **加载失败**：`.fds` 未能 `LoadGame`（`0xFE`）
2. **IRQ 时序**：即便加载成功，FDS IRQ 仍是缺口

### 根因

- **路径 A**：FDS 镜像加载对 patched `.fds` 头/形态处理不完整（`kgmqa-011 fds_load` 是绿的，说明基础加载可用）
- **路径 B**：FDS IRQ（磁盘传输 / 定时器）未建模完整

### 步骤（先 A 后 B）

- [ ] **A1** 对比 kgmqa-011 覆盖的 FDS 样例 vs 本 ROM 形态
- [ ] **A2** 修加载；`0xFE` 消失 = 阶段性成功
- [ ] **A3** 单独记 FAIL 类型（若从 load-fail 变 accuracy-fail，更新 provenance — **需评审**）
- [ ] **B1** FDS IRQ 状态机调查页
- [ ] **B2** 探针记录 IRQ 边沿
- [ ] **B3** 实现并过测

### 不要求本包

TakuikaNinja FDS 族 082–085（advisory LICENSE 未决）——另案。

### 验收

kgmqa-081 从 `0xFE` → 可运行 → 最终 PASS；或 A 完成后拆项并经评审。  
### 风险（高）：FDS 与存档/磁盘状态耦合；savestate 金标可能受影响（人工授权）。

---

## 包出口检查（演进收口）

- [ ] 077 PASS（47/47）
- [ ] 081 PASS 或有据拆项
- [ ] L1 FAIL = **0**
- [ ] advisory-FAIL 比例 &lt; 5%
- [ ] L2 `eventually_pass=true` 清零计划启动
- [ ] 具备 grade **A** 评审条件

## 复验命令

同 T0。077 建议先单子 ROM 手跑，再跑聚合；081 建议加 `--frames` 与加载日志开关调试。
