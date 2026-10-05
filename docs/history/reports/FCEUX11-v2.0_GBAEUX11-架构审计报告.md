# 架构审计报告（r3 · 终审）— FCEUX11 v2.0 GBAEUX11 构建计划

> **STATUS: COMPLETED**
> **归档日期**：2026-09-27（2026-09-27 自 `docs/plans/` 移入；审计轮次已收口，r3 终审批准 S0 与 S1）
> **关联计划**：`docs/history/plans/FCEUX11-v2.0_GBAEUX11构建计划.md`（r6，已处置本报告全部发现）
> **上游**：`https://github.com/RIP-Comm/clementine` @ `ee77922dd293b70e945458e104f3b2de794f0151`

| 项 | 内容 |
|---|---|
| **审计对象** | `docs/history/plans/FCEUX11-v2.0_GBAEUX11构建计划.md`（r5 稿） |
| **审计类型** | 终审（上游源码核验 + 用户指令澄清后的裁定修订） |
| **审计日期** | 2026-09-27 |
| **上游已核验** | **clementine** — `https://github.com/RIP-Comm/clementine`（MIT，73 star，edition 2024） |
| **核验手段** | 上游源码直读（`gba.rs` / `arm7tdmi.rs` / `sound.rs` / `bus.rs` / `Cargo.toml` / issue #204）+ FCEUX11 本地源码 |
| **本次结论** | **批准进入 S0 与 S1**（撤销 r2 的 S1 阻塞） |

---

## 0. 终审裁定

### 0.1 用户指令澄清（推翻 r2 的 F-02 折中）

用户明确：

> 上游是 **clementine**。「少提及原项目」约束的是**源代码与架构命名**，不是构建计划文档。构建计划里完全不提，构建 agent 根本找不到代码库。

**裁定**：

| 层 | 要求 |
|---|---|
| **构建计划文档** | **必须具名**上游：项目名 **clementine**、仓库 URL、基线 commit SHA、issue #204 链接 |
| **源代码 / 架构 / crate / 目录命名** | 继续功能性命名（`f11gba` / `gba-core`），不散落上游标识 |
| **`ATTRIBUTION.md`** | 保留完整出处与 MIT 许可全文（发行合规层） |

r2 的「正文只放 SHA、名称收进 ATTRIBUTION」**撤回**——过度限制，且损害可执行性。

### 0.2 三条反驳的终局裁定

| # | 构建方主张 | r2 裁定 | **r3 终审** | 依据 |
|---|---|---|---|---|
| ① | `init_audio(output_rate)` 核心内已重采样，无需自研 SRC | 部分接受（未核验） | **完全接受** | `gba.rs` + `sound.rs` 源码证实 |
| ② | SWI 分派点仅两处，hook 50–100 行 | 驳回（偷换论题） | **部分接受 + 维持成本结论** | 行号精确命中；但 hook **已存在**，且 Halt/IntrWait 是空壳 |
| ③ | 计划正文不应具名上游 | 折中（SHA 入正文） | **驳回**（按用户澄清） | 计划必须具名 clementine |

---

## 1. 上游源码核验结果（clementine）

### 1.1 已证实的计划主张

| 计划主张 | 核验 | 证据 |
|---|---|---|
| MIT，无附加条款 | ✅ | `LICENSE` / `Cargo.toml` `license = "MIT"` |
| 73 star / 最近活跃 / PGP | ✅ | GitHub 73 stars，740 commits |
| edition 2024 | ✅ | `Cargo.toml` `edition = "2024"` |
| 依赖 `rtrb`/`serde`/`serde_with`/`tracing`，零 GUI | ✅ | `emu/Cargo.toml` 四件套；GUI 在 `ui/`（egui/eframe），分层干净 |
| `Gba::new(bios: [u8; 0x4000], cartridge: &[u8])` | ✅ | `gba.rs` |
| `step() -> bool` = 进入 VBlank | ✅ | `gba.rs` 文档注释 |
| **`init_audio(output_rate: u32, capacity: usize) -> rtrb::Consumer<f32>`** | ✅ | `gba.rs` |
| 音频为交织立体声 f32 + DC 阻断 | ✅ | `sound.rs:81-82,140-145,622-640` |
| **核心内部 sample-and-hold 重采样到宿主率** | ✅ | `sound.rs:81-82` *"resampled to the host output rate with a sample-and-hold"*；`cycle_accumulator += cycles * output_rate` |
| SWI 分派点 `arm7tdmi.rs:504-505` / `628-629` | ✅ **行号精确命中** | ARM `SoftwareInterrupt` / Thumb `Instruction::Swi` |
| issue #204 wait cycle 仅 BIOS 区完成 | ✅ | issue #204 "Tracking issue for cycles"，仅 BIOS 勾选 |
| 核心规模 ~45 文件 | ✅ | `emu/` 下 45 个文件 |
| jsmolka 测试套件 | ✅ | `emu/tests/jsmolka.rs` |
| RTC（S3511） | ✅ | `cpu/hardware/rtc.rs` |
| savestate Serialize/Deserialize | ✅ | `Arm7tdmi`/`Bus` `#[derive(Serialize, Deserialize)]` |
| 存档硬件（SRAM/Flash/EEPROM）在核心内 | ✅ | `bus.rs` EEPROM/Flash/SRAM 访问路径 |

### 1.2 源码揭示的、计划未写透的事实

| 发现 | 影响 |
|---|---|
| **clementine 已内建 `handle_swi_hle()`**，覆盖 **0x00–0x0C**（SoftReset / RegisterRamReset / Halt / Stop / IntrWait / VBlankIntrWait / Div / DivArm / Sqrt / ArcTan / ArcTan2 / CpuSet / CpuFastSet） | SWI 扩展点**已存在**，S1 是「补 match 分支」而非「从零搭 hook」——比构建方说的还便宜 |
| **`Halt`(0x02)、`Stop`/`IntrWait`/`VBlankIntrWait`(0x03–0x05) 是空壳**：`swi_return()` 直接返回，注释写 *"just return because the main loop will handle waiting"* | **N-04 获得源码级证实**。这些不是已实现，是占位 |
| `RegisterRamReset` 的 IWRAM 清空被跳过（源码 TODO：*"would break IRQ handlers"*） | T1 列表里它「已实现」的说法要打折 |
| **0x0D 及以后（GetBiosChecksum / Affine / BitUnPack / Lz77 / Huffman / RL / Diff*）未 HLE**，落到 `handle_exception` → 真 BIOS | stub BIOS 路线下这些 SWI **目前无人实现**——F-01 的核心担忧仍然成立，但实现载体（`handle_swi_hle`）已就位 |
| **上游 README 明确要求真 BIOS**（`gba_bios.bin`，16KB，路径硬编码） | 自建 stub 是**新能力**不是「换个 blob」；`jsmolka` 的 `bios.gba` 当前依赖真 BIOS |
| savestate **不序列化音频 producer**（`take_audio_out`/`restore_audio_out`） | 设计正确，但 C ABI 侧须在 load 后重挂 `init_audio` |

---

## 2. 三条反驳详裁（终审版）

### ① F-03（SRC）— **完全接受**

`sound.rs` 原文：

```text
Every channel is resampled to the host output rate with a sample-and-hold
and pushed to `audio_out`.
```

```rust
pub fn set_audio_out(&mut self, producer: rtrb::Producer<f32>, output_rate: u32) {
    self.output_rate = output_rate;
    ...
}
// cycle_accumulator += cycles * u64::from(self.output_rate);
```

**结论**：`init_audio(output_rate)` 确实接受宿主率并在核心内完成 sample-and-hold 重采样。r1 的「需自研 SRC」**正式撤回**，构建方 ① 成立。

**保留两条尾巴**（不推翻裁定）：

1. **N-01 分数样本问题仍在**，且与 SRC 无关。`gba_render_audio` 的 `int` 返回值被错误码占用，调用方拿不到「本帧实际样本数」；`gba_samples_per_frame()` 的 `uint32_t` 表达不了 738.35。本地源码证实 `frmRateAdjRatio` 是**帧节流**参数（`sdl-throttle.cpp:55,193-208`），NES 的真防漂移在 `FlushEmulateSound` 的 `soundtsoffs`/`left` 残余结转（`sound.cpp:1293-1368`）。**「复用 frmRateAdjRatio 即可」这个具体解法仍被驳回。**
2. **N-06 sample-and-hold 音质**：S&H 上采样会有镜像/混叠，应登记为已知音质取舍（同类 R8）。

### ② F-01（成本）— **部分接受；成本结论改为「低于 r1 高估、高于构建方暗示」**

| 层 | 事实 | 谁对 |
|---|---|---|
| SWI 分派 hook | 行号 `504-505` / `628-629` 精确命中；且 **`handle_swi_hle` 已存在** | 构建方对，甚至比他说的更省 |
| Halt/IntrWait 等 wait 类 | **空壳 `swi_return`**，不是实现 | r1/N-04 对 |
| Lz77/Huffman/RL/BitUnPack/Affine | **完全不存在**，落真 BIOS | r1 F-01 的核心担忧对 |
| 总工作量 | 扩展 `handle_swi_hle` 的 match + 解压/仿射算法实现 | S1a/S1b/S1c 合计 2–3 周**合理** |

**终审表述**：r1 说「工作量远超 S1 现估 2–3 天」是对的（r5 已改判 2–3 周）；构建方说「不是重写、hook 很小」也是对的——但两者说的不是同一层。**hook 小 ≠ SWI 语义小。** r2 说「驳回」过重，改为**部分接受**。

### ③ F-02（具名）— **驳回构建方；按用户澄清执行**

- 计划正文**具名 clementine + URL + 基线 SHA + issue #204 链接**；
- 源代码/架构/crate 命名继续功能性命名；
- `ATTRIBUTION.md` 保留 MIT 全文与版权声明（合规层）。

**给构建 agent 的可执行指令**：§二 改写为「上游：clementine（RIP-Comm/clementine），基线 `<SHA>`」；§十三 交叉引用修正（消除 #2/#5 错位）；`ATTRIBUTION.md` 仍按原计划在 S0 落地。

---

## 3. 新发现与遗留项

### 源码级证实 / 修正

| ID | 级别 | 内容 | 处置 |
|---|---|---|---|
| **N-04** | **P1→已证实** | `Halt`/`Stop`/`IntrWait`/`VBlankIntrWait` 是空壳 | S1a **必须**实现真等待语义；不可把现有占位当「已完成」 |
| **N-09** | **P1（新）** | `RegisterRamReset` IWRAM 清空被上游跳过（TODO） | T1 列为「部分实现」，S1b 须按 GBATEK 补全或列入已知限制 |
| **N-10** | **P2（新）** | 上游 savestate 不序列化音频 producer | `gba_savestate_load` 后必须重挂 `init_audio`；写入 S3 出口 |
| **N-11** | **P2（新）** | 上游当前**强制真 BIOS**（README 硬编码 `gba_bios.bin`） | stub BIOS 是新交付物；`bios.gba` 测试当前基线依赖真 BIOS，S1b 的「#[ignore] 转 pass」以 stub 为准重新定义 |

### 仍然有效（与源码无关）

| ID | 内容 | 状态 |
|---|---|---|
| **N-01** | 分数样本 ABI（`gba_render_audio` 返回实际样本数 + 残余结转） | **仍要求修**，但不阻塞 S1 开工，**阻塞 S2 验收** |
| N-02 | S0 事实核验清单 | 大半已被本审计完成；剩余「基线 SHA / BackupType::detect 行为 / savestate 格式版本」S0 落地 |
| N-03 | 计划 F-02 交叉引用自相矛盾 | 随 ③ 一并修 |
| N-05 | BIOS 按地址直调路径 | 仍列已知限制 / S4 验证 |
| N-06 | S&H 音质风险登记 | 仍要求 |
| N-07 | `gba_set_overlay` 与「无运行时入口」冲突 | 仍要求 |
| N-08 | R1「仅 SWI hook」措辞；S1a 出口过度承诺 | 仍要求 |

---

## 4. 最终放行条件

### 4.1 进入 S0 — **批准**（无附加阻塞）

S0 = 建 crate、vendor clementine、落地 `ATTRIBUTION.md`、`cargo check`。
附带完成：基线 SHA 写入计划正文；§二/§十三 交叉引用修正（F-02 具名）。

### 4.2 进入 S1 — **批准**（撤销 r2 阻塞）

**理由变更**：SWI 扩展点 `handle_swi_hle` 已存在，行号已核验，工作量模型清晰（补 match + 算法实现）。r2 以「事实未核验」阻塞 S1 已无必要——本审计已完成核验。

**S1 期强制约束**（非阻塞，但是出口标准）：

1. **不可把上游占位当实现**：`Halt`/`Stop`/`IntrWait`/`VBlankIntrWait` 必须真做（N-04）；
2. `RegisterRamReset` 补全 IWRAM 行为或显式记入已知限制（N-09）；
3. stub BIOS 路线下，T1/T2 全部 SWI 必须由 `handle_swi_hle` 兜住（0x0D+ 当前会漏到真 BIOS）。

### 4.3 进入 S2 — 须先关闭

| 条件 | 对应 |
|---|---|
| **N-01 分数样本 ABI 定稿** | 否则 30 分钟无漂移验收必炸 |
| N-07 `gba_set_overlay` 发布构建语义 | 水印不变式 |
| N-06 S&H 音质入风险登记 | 文案诚实 |

### 4.4 GA

维持 r5 §8.2 七条门禁 + N-10（savestate 重挂音频）写入 S3 出口。

---

## 5. 变更记录

| 版本 | 日期 | 说明 |
|---|---|---|
| r1 | 2026-09-27 | 初审 CONDITIONAL FAIL（4×P0） |
| r2 | 2026-09-27 | 复审 CONDITIONAL PASS — S0 ONLY；裁定三条反驳 |
| **r3** | 2026-09-27 | **终审**。上游具名 **clementine** 并完成源码核验；①完全接受 ②部分接受 ③按用户澄清驳回；**批准进入 S0 与 S1**；新增 N-09/N-10/N-11 |

---

## 6. 一句话收束

**clementine 是个分层干净、已有 SWI HLE 骨架的 MIT 核心，选型成立。**
构建方 ① 说对了（SRC 不用重做）、② 说对了一半（hook 确实小，但 Halt/IntrWait 是空壳、解压 SWI 缺席）、③ 说错了（计划必须具名，否则 agent 迷路）。

**结论：S0、S1 放行。** S1 记住三件事——wait 类要真做、解压 SWI 要补全、`RegisterRamReset` 别当已完成。N-01（分数样本）在 S2 前修完即可。

**审计人**：架构评审（终审）
**关联**：`docs/history/plans/FCEUX11-v2.0_GBAEUX11构建计划.md`（r5）· 上游 `RIP-Comm/clementine`
