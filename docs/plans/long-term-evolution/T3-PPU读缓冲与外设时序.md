# 分项计划 T3 · 总线模型 / PPU读缓冲 / 外设时序

> **STATUS: OPEN** · **难度** ★★★★☆–★★★★★ · **预估** 每项 1–4 周，可长期并行立项
> **基线**：run 98252605787 · `git_rev=b0658c9` · 106P/14F
> **当前**：v1.18.2 / 矩阵 **108P / 12F** / grade B
> **包出口**：矩阵 FAIL **−5**（038 defer + 049 / 099 / 107 / 108）
> **主报告**：§三 ③⑨⑩⑪⑫ · §2.2

---

## 包目标

五项：一条**总线模型前哨**（defer）+ 四条**业界著名难点**。
不强制同窗完成；**每项单独立项**，按里程碑推进。

| 序 | kgmqa | 主题 | 预估 | 状态 | 可并行 |
|---|---|---|---|---|---|
| 3′ | **038** instr_misc | CPU 数据总线锁存（**049 前哨**） | 1–2 周 | **defer**（2026-09-28 裁定；★2→★4，风险 低→中） | 是 |
| 9 | **049** ppu_read_buffer | $2007 读缓冲 | 2–4 周 | TODO | 是 |
| 10 | **099** 240pee | NTSC/PAL/Dendy TV 时序 | 2–4 周 | TODO | 是 |
| 11 | **107** mset | SNES 鼠标协议 | 1–2 周 | TODO | 是 |
| 12 | **108** mict | FC 麦克风协议 | 1–2 周 | TODO | 是 |

**切分建议**：
- 总线/PPU 线：**038-S0+S1 → 049**（038 是 049 的开工前置，详见下）
- 区制式线：099（产品目标制式先决策）
- 输入线：107 → 108（同属扫描协议，可连续）

---

## 3′ kgmqa-038-instr-misc-blargg　**defer**（2026-09-28 由 T0 移入）

> 完整论证见主报告 **§2.2**；调查证据与 S0/S1/S2/S3 分解见主报告 **§三③**。
> 原执行卡 [`T0-指令时序与板级约束.md`](T0-指令时序与板级约束.md) ③ 段已改为指针。

| 字段 | 值 |
|---|---|
| ROM | `blargg/cpu/instr_misc.nes`（L2：`instr_misc_03_dummy`） |
| 错误码 | `0x01` / `0x03` |
| 难度 | ★★☆☆☆ / 3–5 天 / 风险低 → **★★★★☆ / 1–2 周 / 风险 高** |
| known_limit | 计划原文「combined group; singles pass」**已过时** |

**根因（v1.18.3 实测改判）**：不是「`LDA abs,x` dummy read 不完整」。
真正阻塞点是 **FCEUX11 没有 CPU 数据总线模型** ——
`Cpu::set_db()` 全仓库零调用点，`DB` 永不赋值，`::ANull` 因此返回常量，**open bus 是死的**。

**步骤（defer 期间不投入）**

- [x] **S0a**（2026-09-28 已完成）只记读路径 + `FCEUX11_BUSDB` 开关，默认关。
      改动 `src/bus.h` +37/-1、`src/bus.cpp` +17。零回归：
      ctest 34/34、blargg 147P/30F、bench 开关关 48.205ms vs 开 48.612ms（差 0.8%，噪声内）
- [x] **S1**（2026-09-28 已完成）`instr_misc_03_dummy` 开关关/开各跑 600 + 3000 帧
      → **结果完全相同**（`value=0x03` / `Failed #3` / `diag=[0xDE,0xB0,0x61]`）
      → **主假设不成立**
- [ ] **S2 需重新定义** —— 原定「补齐 8 种指令形式的 dummy read 时序/取值」的前提
      已被两个独立负结果削弱（见下）
- [ ] 候选：**`GetIX` 补 dummy read + 跨页周期**（`LDA (z,x)` / `STA (z,x)` 目前
      **完全没有 dummy read，也没有跨页周期**，而它们正在 ROM 声明的 8 种被测形式里）。
      2026-09-27 的探针只统计了 `GetABIRD` 路径的 33 次 abs 索引读，**没看 `GetIX`**。
      需新探针 → 属独立调查，**先出 PLAN 再动手**

**两个独立负结果（2026-09-27 / 2026-09-28）**

| 变量 | 结果 |
|---|---|
| 改 `GetABIRD` 加硬件 dummy read | 无变化（改动已回退） |
| 让 `DB` 活起来（S0a） | 无变化 |

→ 038 的阻塞点**既不是 dummy read 的有无，也不是 open bus 的值**。

**S0a 故意留在树里**（默认关），是资产不是残留。使用与代价见主报告 §2.2 ⑶-4。
开之前必须知道：20+ 处 open-bus 读取者全变、`input.cpp:148` 影响所有输入测试、
`state.cpp:129` 会让 golden savestate hash 漂移（须人工授权重生成）。

**原本为什么先做 S0+S1（2026-09-28 实测后已作废）**：
原判断是「它的价值在于给 ⑨ 049 铺路（两者共用『活的总线值』这一层），
所以性价比最高的顺序是 038-S0+S1 → 再开 049」。
**实测结论为负**：S0a 没有降低 049 的不确定性，038 也未转绿。
**故 ⑨ 049 不必再等 038，可直接开。** 038 维持 defer。

**风险（高）**：S0a 会改变**每一次 CPU 读**之后所有 open-bus 读取者看到的东西。
实测活跃读取点 **20+ 处**（不是计划原先说的 6 处），其中两处要害计划没提到：
- `input.cpp:148` 手柄读 `ret |= DB & 0xC0` → 影响**所有输入测试**
- `state.cpp:129` `DB` 进 savestate → **golden savestate hash 漂移**，
  须人工授权重生成（2026-09-28 人工裁定走授权路线）

S0a 本身**默认关**，故出厂零影响；S2 若要常开才需付这 20+ 处的对账代价。

**验收**：kgmqa-038 PASS；L2 `instr_misc*.nes` 全 PASS。→ **defer 期间不追求**。

---

## ⑨ kgmqa-049-ppu-read-buffer-bisqwit

| 字段 | 值 |
|---|---|
| ROM | `bisqwit/test_ppu_read_buffer.nes` |
| known_limit | read-buffer monster; open-bus/decay incomplete |

### 根因

`$2007` **读缓冲** + 总线 **open-bus / 衰减** 路径不完整；与 VRAM 增量、镜像、渲染期读交织。
**业界公认最难的 NES 精度测试之一。**

### 里程碑（允许长期多 PR）

- [ ] **M0** `$2007` 读缓冲状态机文档（何时填 / 何时透传 / 何时衰减）
- [ ] **M1** 探针：非渲染期读、渲染期读
- [ ] **M2** 缓冲刷新语义
- [ ] **M3** open-bus 位衰减
- [ ] **M4** bisqwit 分段全过 → kgmqa-049 PASS

### 参考

`docs/history/surveys/e1_vbl/`（VBL/NMI 已调查）、`ppu_bucketC/`

### 验收

最终 kgmqa-049 PASS；过程里程碑可单独记进度。  
### 风险（很高）：易牵动 golden frame / savestate。**PPU 金标重生成必须人工授权。**

---

## ⑩ kgmqa-099-240pee-damianyerrick

| 字段 | 值 |
|---|---|
| ROM | `240pee/240pee.nes` |
| known_limit | NTSC/PAL/Dendy TV timing display test |

### 根因

核心以 NTSC 为主；PAL/Dendy 的 CPU/PPU 时钟比、扫描线数、帧长未完整建模。

### 步骤（先决策后动手）

- [ ] **决策**：产品是否正式支持 PAL/Dendy？
  - **否** → 可申请有据 `known_limit` + 范围声明（**需评审**），本项降级为文档债
  - **是** → 继续
- [ ] 制式表：CPU Hz / PPU 点 / 扫描线 / 帧
- [ ] 接到 core 配置
- [ ] 先过单制式子项，再过全制式

### 验收

kgmqa-099 PASS，或经评审的制式范围声明 + 分制式子测试 PASS。  
### 风险（高）：多制式影响所有时序默认假设；按特性级变更走。

---

## ⑪ kgmqa-107-mset-rainwarrior

| 字段 | 值 |
|---|---|
| ROM | `rainwarrior/mset/mset.nes` |
| known_limit | SNES mouse input scan protocol |

### 根因

$4016/$4017 串行协议中的 **SNES 鼠标** 扩展未实现/不完整（分辨率位、溢出、移位时序）。

### 步骤

- [ ] 对照 mset 文档列协议状态机
- [ ] 输入扫描路径加鼠标设备类型
- [ ] 与标准手柄 / Zapper（已 PASS）共存
- [ ] 回归哨兵：`porttest`（101）、`allpads` 路径、`test_buttons`（111）

### 验收

kgmqa-107 PASS。  
### 风险：中–高。回归面在输入子系统；**不碰 PPU/CPU 金标**。

---

## ⑫ kgmqa-108-mict-rainwarrior

| 字段 | 值 |
|---|---|
| ROM | `rainwarrior/mict/mict.nes` |
| known_limit | Famicom microphone input scan protocol |

### 根因

$4016 麦克风采样位（通常 bit 2/3）未实现；与手柄移位交织。

### 步骤

- [ ] 实现麦克风采样位
- [ ] 测试夹具（恒定电平 / 噪声）
- [ ] 与 ⑪ 同属输入协议，建议连续立项

### 验收

kgmqa-108 PASS。  
### 风险：中–高。同 ⑪。

---

## 包出口检查（按完成子集勾选）

- [ ] 049 PASS（或里程碑 M0–M3 归档 + 残缺原因有据）
- [ ] 099 PASS 或评审后的制式范围声明
- [ ] 107 PASS
- [ ] 108 PASS
- [ ] 038：defer 期间无要求；若做，仅要求 S0+S1 结论入库（见 3′ 段）
- [ ] 全程 `pass_to_fail = 0`；PPU 金标变更均有人工授权记录

## 复验命令

同 T0。049 额外建议单 ROM 长帧预算调试；107/108 可用 porttest 做输入回归。
