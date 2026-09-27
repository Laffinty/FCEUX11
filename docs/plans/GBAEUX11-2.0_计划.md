# FCEUX11 v2.0 构建计划 — GBAEUX11 模块（GBA 运行能力移植）

> **STATUS: OPEN**（2026-09-27 立项，尚未开工）
> **模块名**：**GBAEUX11**
> **版本**：v2.0（BETA 阶段）
> **日期**：2026-09-27
> **分支**：S0 开工时创建
> **前置**：v1.18.1 已发布（`main` @ `3e33f2b`）；F11QA R4 gate green，grade B
> **关联**：`src/rust/crates/f11qa/README.md`、`docs/tech/precision.md`、`COPYRIGHT_AUDIT.md`、`DERIVATIVE_WORK_NOTICE.txt`
> **路线图位置**：v1.15 C++ 现代化 → v1.16 KagamiQA 闭环 → v1.17 统合与分级 → v1.18 残留精度长期演进 → **v2.0 跨平台扩张（NES 之外首次引入第二主机）**

---

## 〇、TL;DR

v2.0 只做一件事：**让 FCEUX11 能运行 GBA 游戏**，前端完全复用现有 Qt 驱动，不重写 UI。

策略是**移植 Rust 模拟核心 + 自建开源 HLE BIOS + 前端薄接线**，不重写模拟器。

| # | 决策项 | 结论 |
|---|---|---|
| 1 | 模拟核心 | 移植 [clementine](https://github.com/RIP-Comm/clementine) 的 `emu/` crate（MIT） |
| 2 | 前端 | 复用 FCEUX11 现有 Qt 驱动，仅加 GBA 渲染/音频接线 |
| 3 | BIOS | **默认自建 HLE BIOS**（源自 mGBA，MPL-2.0）；用户若提供真 BIOS 则优先使用 |
| 4 | M4A | **补齐**（MusicPlayer2000），使 HLE 模式下 Gen 3 宝可梦有声 |
| 5 | 测试 | **暂不接入 F11QA**，改手工实测清单（当前为测试阶段） |
| 6 | 标识 | 所有 GBA 画面左上角常驻 `BETA` 水印 |
| 7 | 键位 | BETA 阶段采用 **NES 兼容键位**模式 |

**一句话收束**：GBAEUX11 是 FCEUX11 的第二个模拟核心，与 NES 核心并列而非嵌入 `boards/`（GBA 不是 mapper 概念）；核心用 Rust 写、经 cbindgen C ABI 接入既有 `fceux11_rust` 静态库，前端零重写。

---

## 一、范围

### 1.1 做

- `.gba` ROM 识别、加载、运行
- 视频输出（240×160）、音频输出（交织立体声）
- 存档：`.srm` 电池存档（SRAM / Flash 64K·128K / EEPROM 512B·8K）、即时存档
- 键位：NES 兼容模式（8 键一一映射 + L/R 附加键）
- `BETA` 水印常驻左上角

### 1.2 明确不做（v2.0 BETA 范围外）

| 项 | 理由 |
|---|---|
| 接入 F11QA / KagamiQA | 当前为测试阶段，手工实测先行；待精度收敛后再议 |
| 移植 clementine 的 `ui/` | egui 调试器 / 反汇编器 / 内存查看器 / Pokemon 调试器全部丢弃 |
| 补齐 wait cycle | clementine issue #204 仅 BIOS 区完成，其余内存区未实现 |
| 上游同步自动化 | 手工评估（见 §七 风险 R1） |
| 触觉 / 陀螺仪 / 太阳传感器 | GBA 外设扩展，BETA 不做 |
| 非 `.gba` 格式（`agb`/`bin` 等） | BETA 只认 `.gba` |

---

## 二、上游选型实证

对 clementine `main`（`ee77922`，2026-08-27）实测：

| 维度 | 事实 |
|---|---|
| 许可 | MIT，无附加条款 |
| 核心规模 | `emu/` 45 文件 / **19,547 行** |
| 构建 | `cargo check -p emu --all-targets` **37s 通过** |
| 依赖 | 仅 `rtrb` / `serde` / `serde_with` / `tracing` — **零 egui** |
| 分层 | `emu/`（核心）· `src/main.rs`（CLI）· `ui/`（egui），**仅取 `emu/`** |
| 活跃度 | 最后提交 2026-08-27，PGP 签名，近 8 次提交集中于安全加固 |
| Rust 版本 | **edition 2024** — 与 FCEUX11 `src/rust` workspace 一致，无翻译成本 |

### 2.1 clementine 已验证的精度

`emu/tests/jsmolka.rs`（对 [jsmolka/gba-tests](https://github.com/jsmolka/gba-tests)）：

- **通过 9 项**：`arm` `thumb` `memory` `nes` `unsafe` `save/{none,sram,flash64,flash128}`
- **失败 1 项**：`bios/bios.gba`，被 `#[ignore]`，注释为 *"BIOS function emulation is incomplete"*

### 2.2 现成接口面

| 接口 | 签名 | 说明 |
|---|---|---|
| 构造 | `Gba::new(bios: [u8; 0x4000], cartridge: &[u8])` | **只需喂 16KB，无需改动 emu** |
| 步进 | `step() -> bool` | 返回 `true` 表示进入 VBlank（新帧就绪） |
| 视频 | 240×160，15-bit | 经 renderer 持有 framebuffer |
| 音频 | `rtrb::Producer<f32>` | 交织立体声，已含 DC 阻断滤波 |
| 电池存档 | `InternalMemory::battery_data()` / `load_battery()` / `save_dirty` | **存档硬件在 `emu/` 内**；`.srm` 文件 I/O 在 `ui/`，需重写（数十行） |

---

## 三、许可与合规

| 来源 | 许可 | 与 GPL-2.0 的关系 | 处置 |
|---|---|---|---|
| clementine `emu/` | MIT | 兼容 | 保留 MIT 声明，登记 `COPYRIGHT_AUDIT.md` |
| mGBA `hle-bios.s` | **MPL-2.0** | **MPL-2.0 §1.12 明确将 GPL-2.0 列为兼容 Secondary License** | 保留 MPL-2.0 头，登记 `DERIVATIVE_WORK_NOTICE.txt` |
| M4A 参考实现 | 学术参考 | 仅读不改写 | 不复制代码，独立实现 |

> HLE BIOS 属 MPL-2.0 Covered Software 文件，随 FCEUX11 以 GPL-2.0 分发时须保留其 MPL-2.0 声明（MPL-2.0 §3.3 允许 Larger Work 采用其他条款分发）。

---

## 四、目录布局

```
src/rust/crates/f11gba/               # 新增 crate
├── Cargo.toml
├── VENDOR.md                          # 上游基线 SHA + 本地补丁集清单
├── vendor/clementine_emu/             # clementine emu/ 原样 vendor
├── src/
│   ├── lib.rs
│   ├── ffi.rs                         # cbindgen 导出（约 10 个 extern "C"）
│   ├── bios.rs                        # HLE BIOS 装载 / 真 BIOS 回退
│   ├── m4a.rs                         # M4A 驱动（独立实现）
│   └── overlay.rs                     # BETA 水印合成
└── build.rs                           # HLE BIOS 汇编（见 §五）

src/gba_bridge.h / .cpp                # C ABI 桥，仿 kagami_bridge.h
src/drivers/Qt/                        # GBA 渲染 / 音频接线
```

**不新建构建体系**：`src/rust/CMakeLists.txt` 现有 `add_custom_command` 已用 `GLOB_RECURSE crates/*.rs`，新增 crate 自动纳入 `fceux11_rust.lib`。

### 4.1 C ABI 面

```c
int  gba_init(void);
int  gba_load_rom(const char *path);
int  gba_step_frame(void);
int  gba_frame_buffer(uint8_t *dst, uint32_t len);   // 240*160*4 RGBA，含 BETA 水印
int  gba_read_audio(int16_t *dst, uint32_t frames);  // f32 → i16 转换在此层
int  gba_set_buttons(uint16_t mask);
int  gba_battery_read(uint8_t *dst, uint32_t cap);
int  gba_battery_write(const uint8_t *src, uint32_t len);
int  gba_savestate(uint8_t *dst, uint32_t cap, uint32_t *written);
int  gba_kill(void);
```

唯一格式转换点：`f32 → i16`（clementine 出 f32，FCEUX11 走 SDL i16）。

---

## 五、BIOS 策略

### 5.1 关键认识：HLE BIOS 是**可执行二进制**，不是 SWI 拦截器

mGBA 的 `src/gba/hle-bios.c` 是 `const uint8_t hleBios[GBA_SIZE_BIOS] = {...}` —— 一段**真实 ARM 机器码**。
故 `Gba::new()` 只需喂不同的 16KB 即可，**clementine 的 emu 一行都不用改**。

### 5.2 但不能直接拷贝字节数组

`hle-bios.s` 的 `swiBase` 含：

```asm
swieq 0xF00000   @ Special mGBA-internal call to load the stall count into r11
```

mGBA 靠自家 CPU 钩子回填 stall 计数。clementine 无此钩子，会以 comment byte `0xF0` 越界索引 `swiTable` → 野指针。

**正确做法**：移植 `hle-bios.s` **源码**（MPL-2.0），删除该行，**自行 assemble** 出 16KB。功能上该指令仅为计时代码，删除不影响正确性。

### 5.3 覆盖度与已知缺口

`hle-bios.s` 的 SWI 表覆盖 0x00–0x2A 全部 43 项槽位，但：

| 状态 | SWI |
|---|---|
| ARM 原生实现 | `Halt` `Stop` `IntrWait` `VBlankIntrWait` `CpuSet` `CpuFastSet` `SoftReset` `SoundDriverGetJumpList` |
| mGBA 侧 C 实现（汇编仅占位计时） | `Div` `DivArm` `Sqrt` `ArcTan` `ArcTan2` `Lz77UnComp*` |
| **未实现（Nop）** | `RegisterRamReset` `GetBiosChecksum` `BgAffineSet` `ObjAffineSet` `BitUnPack` `HuffmanUnComp` `RlUnComp*` `Diff*UnFilter*` **及 0x19–0x24 / 0x28–0x2A 整个声音段** |

**后果**：HLE BIOS 下使用 Nintendo MusicPlayer2000（M4A）的游戏静音 —— Gen 3 宝可梦全系在内。
notorious_beeg_bios（MIT）README 同样写明 *"TODO implement remaining BIOS calls (mainly sound functions)"*，**这是开源生态的共识性缺口**。

→ 由 §六 的 M4A 补齐解决。

### 5.4 装载策略

```
用户指定真 BIOS（存在且 16KB） → 优先使用，M4A 由真 BIOS 驱动
否则                          → 使用自建 HLE BIOS，M4A 由 f11gba 侧实现
```

---

## 六、M4A 补齐

M4A 是运行在 GBA ARM CPU 上的一整段音频驱动，读写 `SOUNDCNT` / FIFO / 定时器。
补齐方式：将其驱动逻辑**重写为宿主语言（Rust）**，挂进 clementine 的混音路径。

- 参考实现：mGBA、NanoBoyAdvance（均为独立实现，非本项目复制对象）
- 位置：`src/rust/crates/f11gba/src/m4a.rs`
- 验收：Gen 3 宝可梦（FireRed / LeafGreen / Emerald / Ruby / Sapphire）在 **HLE 模式下**有 M4A 音源

> S3 是全计划唯一技术不确定性最高的一项。S2 出包后先用 3–4 个 M4A 游戏实测，再决定投入深度。

---

## 七、其他实现约定

### 7.1 `BETA` 水印

| 项 | 约定 |
|---|---|
| 位置 | 游戏画面**左上角**常驻 |
| 合成点 | **Rust 侧**（`overlay.rs`，出帧前一步） |
| 覆盖范围 | 窗口、截图、录像自动全部带上（单一合成点） |
| 开关 | 编译期/运行期开关，2.0 正式版关闭 |

### 7.2 NES 兼容键位（BETA）

| NES | → GBA | 备注 |
|---|---|---|
| A | A | 一一映射 |
| B | B | 一一映射 |
| Select | Select | 一一映射 |
| Start | Start | 一一映射 |
| D-pad | D-pad | 一一映射 |
| — | **L** | NES 无对应，默认绑 Z |
| — | **R** | NES 无对应，默认绑 X |

BETA 只承诺「8 键零学习成本」；L/R 属附加键，可重绑定。

### 7.3 存档

- 电池存档：`<rom>.srm`，与 ROM 同目录
- 即时存档：独立通道，`gba_savestate()`
- 硬件支持来自 `InternalMemory`（SRAM 32KB / Flash 64K·128K / EEPROM 512B·8K，按 ROM 头部 ID 自动识别）

---

## 八、阶段编排

| 阶段 | 内容 | 出口标准 | 估 |
|---|---|---|---|
| **S0** | 建 `f11gba` crate，vendor `emu/`，接入既有 staticlib | `cargo check` 过；CMake 产出含新符号的 `fceux11_rust.lib` | 2–3 天 |
| **S1** | 移植 `hle-bios.s`，删 `swieq 0xF00000`，自建 16KB；真 BIOS 回退配置项 | 两种 BIOS 均能启动 ROM 到游戏画面 | 2–3 天 |
| **S2** | C ABI + Qt 前端：`.gba` 识别、240×160 渲染、音频 `f32→i16`、`BETA` 水印 | 可玩非 M4A 游戏，画面/音频正常 | 1–2 周 |
| **S3** | M4A 译为 Rust，接混音（可与 S2 并行） | Gen 3 宝可梦在 HLE 模式下有声 | 2–3 周 |
| **S4** | NES 兼容键位、`.srm` 存档、即时存档 | 存档跨会话可读；键位符合 §7.2 | 1 周 |
| **S5** | 手工实测清单，逐游戏过 | 清单内游戏全部可玩并记录已知限制 | 长尾 |

**S0–S2 约 3–4 周产出可玩 BETA**；S3 决定 M4A 游戏体验。

---

## 九、风险登记册

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| **R1** | **vendor 分叉成本**。clementine 仍在活跃开发（2026-08-27 有提交） | 上游修复无法自动流入，长期可能需接盘 | `VENDOR.md` 记录基线 SHA；定期评估合并；补丁集最小化 |
| **R2** | **无商业游戏验证**。jsmolka 是合成测试，9/10 通过 ≠ 主流游戏可跑 | BETA 可能大面积跑不动 | S5 手工实测清单；已知限制逐条编目 |
| **R3** | **wait cycle 大面积未实现**（issue #204 仅 BIOS 区完成） | DMA / 精灵窗口敏感游戏可能异常 | 列入已知限制；S5 重点验证 |
| **R4** | **M4A 实现难度未知** | S3 可能超期 | 先做 3–4 游戏小样本验证再定深度 |
| **R5** | **巴士因子**。clementine 73 star、实质 1–2 人维护 | 上游停摆则需自维护全部 19.5K 行 | MIT 无法律障碍；R1 缓解 |
| **R6** | **水印污染帧校验** | 未来接入 F11QA 时基线不可比 | 开关可关；接入测试时以关闭状态建基线 |

---

## 十、不变式

1. **NES 核心零回归**：v2.0 任何改动不得改变 F11QA 现有矩阵（`106P / 14F`，grade B）。`pass_to_fail` 必须为 0，出现即回滚。
2. **前端复用不打折**：不得为 GBA 另起 UI 框架或分叉 Qt 驱动。
3. **BIOS 不夹带任天堂代码**：仓库内只允许自建 HLE 或用户自备路径，绝不提交真 BIOS。
4. **许可可审计**：每条外部来源登记 `COPYRIGHT_AUDIT.md`，保留原始声明。
5. **BETA 水印不可绕过**：BETA 阶段所有 GBA 画面必带水印，无隐藏开关。
6. **HLE 与真 BIOS 双路径均须可用**，不得只保一条。

---

## 十一、状态回写

阶段完成后：

1. 本文件 §八 对应行勾选并更新出口标准
2. `VENDOR.md` 更新基线 SHA 与补丁集
3. `COPYRIGHT_AUDIT.md` 同步新增来源
4. `CHANGELOG.md` 记入 Added 段
5. 已知限制逐条编目进本文件 §9，不做无据 `known_limit`

---

## 十二、待确认

| # | 事项 | 状态 |
|---|---|---|
| 1 | 阶段分支命名 | S0 开工时确定 |
| 2 | 实测游戏清单（版权与来源） | S5 前确定 |
| 3 | 2.0 正式版是否移除 `BETA` 水印 | 计划移除，2.0 GA 前确认 |
| 4 | F11QA 接入时点 | 精度收敛后另议 |
