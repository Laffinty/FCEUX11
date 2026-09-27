# FCEUX11 v2.0 构建计划 — GBAEUX11 模块（GBA 运行能力移植）

> **STATUS: OPEN**（2026-09-27 立项；2026-09-27 四次修订：采纳第三方架构审计，见 §十三）
> **模块名**：**GBAEUX11**（FCEUX11 Rust 侧的第二模拟核心）
> **版本**：v2.0（BETA 阶段）
> **日期**：2026-09-27 立项 / 二次修订（M4A 延期）/ 三次修订（音频契约）/ 四次修订（架构审计响应）
> **分支**：S0 开工时创建
> **前置**：v1.18.1 已发布（`main` @ `3e33f2b`）；F11QA R4 gate green，grade B
> **关联**：`docs/plans/FCEUX11-v2.0_GBAEUX11构建计划_架构审计报告.md`、`COPYRIGHT_AUDIT.md`、`DERIVATIVE_WORK_NOTICE.txt`

---

## 〇、TL;DR

v2.0 只做一件事：**让 FCEUX11 能运行 GBA 游戏**，前端复用现有 Qt 驱动，不重写 UI。

| # | 决策项 | 结论 |
|---|---|---|
| 1 | 模拟核心 | 移植成熟开源 GBA 硬件模拟核心（Rust，MIT） |
| 2 | 前端 | 复用现有 Qt 驱动，NES 侧资产在 GBA 侧显式禁用（见 §7.4） |
| 3 | BIOS | **自建极简 stub BIOS + Rust 原生 SWI 实现**（不再使用第三方 HLE BIOS） |
| 4 | M4A | 延期至 2.0 GA 之后（§六） |
| 5 | 测试 | 暂不接入 F11QA，改手工实测 + jsmolka 进 CI（§7.5） |
| 6 | 标识 | 所有 GBA 画面左上角常驻 `BETA` 水印 |
| 7 | 键位 | BETA 阶段采用 NES 兼容键位模式 |
| 8 | 音频 | 复用现有声卡接口 `WriteSound(int32*, int)`（§4.2） |

**一句话收束**：GBAEUX11 是与 NES 核心**并列**的第二模拟核心（GBA 不是 mapper 概念，不进 `boards/`）；核心用 Rust 写、经 cbindgen C ABI 接入既有 `fceux11_rust` 静态库，前端零重写。

> **本次修订的实质变化**：BIOS 路线从「移植第三方 HLE BIOS（2–3 天）」改为「自建 stub + Rust 实现约 25 个 SWI（2–3 周）」。理由见 §五，这是审计暴露的根本性缺陷，必须照实重估。

---

## 一、范围

### 1.1 做

- `.gba` ROM 识别、加载、运行
- 视频输出（240×160）、音频输出（int32 单声道 / ±32768，经 `WriteSound` 复用现有声卡）
- 存档：`.srm` 电池存档、即时存档、**手动覆盖存档类型**
- RTC（S3511，宿主时钟驱动）—— 宝可梦 RSE 依赖
- 键位：NES 兼容模式（8 键一一映射 + L/R 附加键）
- `BETA` 水印常驻左上角

### 1.2 明确不做（v2.0 BETA 范围外）

| 项 | 理由 |
|---|---|
| 接入 F11QA / KagamiQA 全套 | 仅 jsmolka 进 CI（§7.5）；完整接入待精度收敛（§8.1 P3） |
| **M4A / MP2K 增强混音** | 已决议延期至 GA 后；非可玩性前提（§六） |
| **立体声输出** | 受限于现有 `WriteSound` 单声道设备契约，BETA 放弃；成本评估见 §4.3 |
| 移植上游调试器 UI | 反汇编器 / 内存查看器 / 宝可梦调试器全部丢弃 |
| 补齐 wait cycle | 上游 issue #204 仅 BIOS 区完成 |
| 上游同步自动化 | 手工评估（R1） |
| 触觉 / 陀螺仪 / 太阳传感器 | GBA 外设扩展 |
| MultiBoot（`.mb`）/ overdump | 显式拒绝并报错，不静默失败（§7.6） |
| 非 `.gba` 格式 | BETA 只认 `.gba` |

---

## 二、核心选型实证

| 维度 | 事实 |
|---|---|
| 许可 | MIT，无附加条款 |
| 核心规模 | 45 文件 / **19,547 行** |
| 构建 | `cargo check` **37s 通过** |
| 依赖 | 仅 `rtrb` / `serde` / `serde_with` / `tracing` — **零 GUI 库** |
| 分层 | 硬件核心 / CLI / 调试器 UI 三分，**仅取硬件核心** |
| 活跃度 | 最后提交 2026-08-27，PGP 签名，近 8 次提交集中于安全加固 |
| Rust 版本 | **edition 2024** — 与 FCEUX11 `src/rust` workspace 一致 |
| 存档类型识别 | 核心内 `BackupType::detect()` 扫描 ROM 正文签名串 |
| RTC | 核心内 `hardware/rtc.rs`（10.6 KB，S3511 over GPIO） |
| 即时存档 | 核心内 `#[derive(Serialize, Deserialize)]` + 版本化 + 完整性校验 |

> **审查入口**：以上全部事实主张的可核验出处（仓库 URL、基线 commit SHA、上游 issue 链接）**统一记录于 `crates/f11gba/ATTRIBUTION.md`**，该文件在 S0 首个 commit 落地。审查者必须先读该文件方能核验本节。此安排是「对外发行物使用功能性命名」与「内部事实可独立审查」两条要求的分离实现。

### 2.1 上游已验证的精度

其测试套件（对 [jsmolka/gba-tests](https://github.com/jsmolka/gba-tests)）：

- **通过 9 项**：`arm` `thumb` `memory` `nes` `unsafe` `save/{none,sram,flash64,flash128}`
- **失败 1 项**：`bios/bios.gba`，`#[ignore]`，注释 *"BIOS function emulation is incomplete"*

> `memory.gba` 正是 SWI 0x0B/0x0C/0x11–0x18 的专项测试。**在自建 HLE 路线下，它从「附带通过」升级为 §五 的核心验收门。**

### 2.2 现成接口面

| 接口 | 说明 |
|---|---|
| `Gba::new(bios: [u8; 0x4000], cartridge: &[u8])` | 只需喂 16KB |
| `step() -> bool` | `true` = 进入 VBlank |
| `init_audio(output_rate: u32, capacity: usize)` | **采样率由调用方指定**，核心内部重采样（见 §4.2） |
| 视频 | 240×160，15-bit |
| `battery_data()` / `load_battery()` / `save_dirty` | 存档硬件在核心内；`.srm` 文件 I/O 需自写 |

---

## 三、许可与合规

| 来源 | 许可 | 处置 |
|---|---|---|
| GBA 硬件模拟核心 + 本地 SWI 实现 | **MIT** | **保留上游文件头与版权行 + 随发行物附许可全文**；目录/crate 可改名，**文件头不可删** |
| stub BIOS 汇编与产物 | 本项目自著 | 无外部依赖 |
| GBATEK 等硬件文档 | 公开文档 | 仅作行为规格参考，**不复制代码** |

> **较前版的实质改善**：自建 stub BIOS 后，**不再使用任何 MPL-2.0 代码**，MPL 兼容性论证、Exhibit B 检查、修改标注义务等全部消失。原第三方 HLE BIOS 依赖已整体移除。
>
> **MIT 义务的准确表述**（前版表述偏弱）：随发行物保留 ① 上游版权行 ② MIT 许可全文 ③ 本地修改说明。「代码内不散落上游项目标识」仅适用于**目录名 / crate 名 / 模块名**，**不得演变为剥离上游文件头**。

---

## 四、目录布局

```
src/rust/crates/
├── f11gba/                      # 新增 crate
│   ├── Cargo.toml
│   ├── ATTRIBUTION.md           # 上游来源、许可、基线 SHA（唯一权威出处）
│   ├── bios/
│   │   ├── stub.s               # 自建 stub BIOS 源码（ARM 汇编）
│   │   ├── stub.bin             # 预汇编 16KB 产物（入库，免工具链依赖）
│   │   └── REGENERATE.md        # 重新汇编的步骤与校验哈希
│   ├── src/
│   │   ├── lib.rs
│   │   ├── ffi.rs               # cbindgen 导出
│   │   ├── swi/                 # SWI 原生实现（替代 BIOS 代码）
│   │   │   ├── mod.rs           #   分发表 0x00–0x2A
│   │   │   ├── decompress.rs    #   LZ77 / Huffman / RL / UnFilter
│   │   │   ├── bitunpack.rs
│   │   │   ├── math.rs          #   Div / Sqrt / ArcTan / ArcTan2
│   │   │   ├── affine.rs        #   BgAffineSet / ObjAffineSet
│   │   │   ├── memory.rs        #   CpuSet / CpuFastSet / RegisterRamReset
│   │   │   └── wait.rs          #   Halt / Stop / IntrWait / VBlankIntrWait / SoftReset
│   │   ├── save.rs              # 存档类型覆盖、.srm 布局、RTC 区
│   │   └── overlay.rs           # BETA 水印
│   └── build.rs
└── gba-core/                    # vendor 核心 = 独立 path 依赖 crate
    └── src/                     # 保留上游文件头，仅新增 SWI hook（§5.4）
```

**Cargo 语义（修正前版冲突）**：`vendor/` 置于 crate 根下、位于 `src/` 之外时 **Cargo 不会编译其中的 `.rs`**。故采用**独立 path 依赖 crate**（`crates/gba-core/`，自带 `Cargo.toml`），而非 `#[path]` 挂载或塞进 `src/vendor/`。

**stub BIOS 工具链**：`stub.bin` **入库**，`stub.s` 同时入库。正常构建与 CI **不需要 ARM 汇编器**；`REGENERATE.md` 记录用 `arm-none-eabi-as` 重新生成的方法与产物 SHA-256。避免在 Windows/MSVC 工具链上引入新构建依赖。

### 4.1 C ABI 面

```c
// 错误码（gba_last_error 取最近一次错误文本）
enum { GBA_OK=0, GBA_ERR_NO_ROM, GBA_ERR_BAD_ROM, GBA_ERR_BIOS, GBA_ERR_UNSUPPORTED, GBA_ERR_STATE, GBA_ERR_CAPACITY };

int         gba_last_error(char *dst, uint32_t cap);

// 生命周期
int         gba_init(void);
int         gba_set_bios(const char *path, int use_default_if_missing);  // UTF-8
int         gba_load_rom(const char *path);                                 // UTF-8
int         gba_unload_rom(void);
int         gba_reset(void);
int         gba_rom_loaded(void);

// 运行
int         gba_step_frame(void);
int         gba_frame_buffer(uint8_t *dst, uint32_t len);   // 240*160*4 RGBA，含水印
void        gba_set_buttons(uint16_t mask);
void        gba_set_overlay(int enable);                     // 见 §7.1 双策略

// 音频（调用层次见 §4.3）
int         gba_render_audio(int32_t *dst, uint32_t frames);  // 已下混 + 音量
uint32_t    gba_samples_per_frame(void);                      // 由帧率算出，避免调用方猜

// 存档
int         gba_savestate_size(uint32_t *size);                // 必补：避免调用方猜 cap
int         gba_savestate_save(uint8_t *dst, uint32_t cap, uint32_t *written);
int         gba_savestate_load(const uint8_t *src, uint32_t len);
int         gba_battery_read(uint8_t *dst, uint32_t cap);
int         gba_battery_write(const uint8_t *src, uint32_t len);
int         gba_set_save_type(int type);                       // 手动覆盖，检测失败时使用
int         gba_get_save_type(void);
```

**线程模型（明确声明）**：**全部函数仅限模拟线程调用**。`gba_render_audio` 由模拟线程在 `gba_step_frame` 内部或紧邻调用；**不得**从 SDL 音频回调线程调用。SDL 侧通过 §4.3 的桥接取数，不直接进 FFI。

### 4.2 音频链路（复用现有声卡接口）

> **决议**：GBAEUX11 **复用 FCEUX11 现有声卡接口**，不自持 SDL 设备。采样率 / 音量 / 缓冲 / 静音 / Turbo 抑制与 NES 共用同一套 UI 与配置。

生产侧接口在 `src/drivers/Qt/dface.h`：

```c
int    InitSound();
void   WriteSound(int32 *Buffer, int Count);
uint32 GetMaxSound();
uint32 GetWriteSound();
```

**契约（易错点）**：

| 项 | 事实 | 出处 |
|---|---|---|
| 送入缓冲类型 | **`int32`，单声道**（非 i16） | `dface.h:12` |
| 幅度范围 | **±32768** | `sdl-sound.cpp:77,167` |
| 设备侧格式 | `AUDIO_S16SYS` / `channels=1` / `samples=512` | `sdl-sound.cpp:256-259` |
| 降位时机 | SDL 回调内直接截断，**无缩放** | `sdl-sound.cpp:167` |
| 背压 | 写前用 `GetWriteSound()` 判剩余 | `sdl-sound.cpp:345-348` |
| 音量 | **`WriteSound` 不施加音量** → GBAEUX11 须自行应用 `FSettings.SoundVolume` | `sdl-sound.cpp:354-366` |
| Turbo 抑制 | `WriteSound` 内建 | `sdl-sound.cpp:363-367` |

**采样率转换归属（前版误判已更正）**：

- 上游核心的 `Gba::init_audio(output_rate, capacity)` **接受调用方指定的宿主采样率**，核心内部即以 sample-and-hold 重采样到该率。`rtrb` 流中不存在「硬件原生率」，故**不需要我们再实现 SRC**。
- 唯一仍需处理的是**分数样本数**：`samplesPerFrame = 采样率 / 帧率`。GBA 帧率 59.7275 Hz，`44100 / 59.7275 ≈ 738.35` 非整数。
  该问题**并非 GBA 特有**：NES 同样非整数（`44100 / 60.0988 ≈ 733.8`），FCEUX11 已有 `frmRateAdjRatio` / `g_fpsScale` 处理。
  → GBAEUX11 复用同一套机制，**不自造第二套**。
- 新增 `gba_samples_per_frame()` 由帧率统一算出，避免调用方与驱动各算一遍产生漂移。

### 4.3 音频分层与增益结构（前版分层混乱已更正）

固定分层，前端只与最外层打交道：

```
gba_step_frame()
   ├─ 步进 CPU 直到 VBlank
   ├─ 从 rtrb 抽取 f32 交织立体声（率 = 我们传入的宿主率）
   ├─ 下混：(L + R) × 0.5            ← 必须，先于放大，否则双声道同相必削波
   ├─ × 32768
   ├─ × 音量系数（FSettings.SoundVolume / 最大值）
   └─ 饱和截断到 int32 → 帧缓冲
                    ↓
   驱动层：gba_render_audio(buf, gba_samples_per_frame()) → WriteSound(buf, n)
```

**调用方只调 `WriteSound`**；下混、增益、截断全部在 `f11gba` 内完成，`WriteSound` 仅作环形缓冲搬运。

**立体声取舍（诚实表述）**：GBA 硬件为立体声（2×PSG + 2×Direct Sound，可声像）。BETA 受限于 `WriteSound` 的单声道设备契约而放弃立体声。改为立体声需另开 SDL 设备，与「复用现有声卡」决议冲突，故留待 GA 后评估。

---

## 五、BIOS 与 SWI 策略（本次修订重点）

### 5.1 修订缘由：原路线存在根本性缺陷

原计划主张「移植第三方 HLE BIOS 源码 → 删一条指令 → 自行汇编 16KB → 喂进核心，上游零改动」，并以「两种 BIOS 均能启动 ROM 到游戏画面」为 S1 出口标准。

**该主张不成立。** 第三方 HLE BIOS 的汇编部分中：

- `Lz77UnCompWram/Vram`、`HuffmanUnComp`、`RlUnComp*`、`BitUnPack`、`BgAffineSet`、`ObjAffineSet`、`RegisterRamReset`、`Div`、`Sqrt`、`ArcTan`、`ArcTan2` **要么是空转计时循环，要么直接 `bx lr` 空返回**；
- 真正的语义实现在**该模拟器自己的 C 侧**（`src/gba/bios.c`），经其 **CPU 内核的 SWI 拦截钩子**（`GBASwi16(cpu, immediate)`）分派，**是其内核专有机制**；
- 其中的 `swieq 0xF00000` 正是向该钩子索取 stall 计数的回程协议，删掉它只消除崩溃路径，**不恢复任何被空转掉的功能**。

后果：商业游戏大量使用 SWI 0x11/0x12/0x13/0x14/0x15 解压图块与地图数据，这是图形管线主干。按原路线交付，这些 SWI 会**静默失效**（无报错），表现为花屏 / 黑屏 / 死机。

原计划 §5.3 自己列出了「由模拟器 C 侧实现」与「未实现」两张表，却未在任何阶段安排实现它们——**自相矛盾**。

### 5.2 新路线：stub BIOS + Rust 原生 SWI

不再使用任何第三方 BIOS 代码。分两层：

| 层 | 职责 | 实现位置 |
|---|---|---|
| **stub BIOS（16KB）** | 复位向量 → 跳转到卡带入口；异常向量；IRQ 经 `0x03007FFC` 间接跳转 | `bios/stub.s`，约 30 条指令 |
| **SWI 语义** | 0x00–0x2A 的实际功能 | `f11gba/src/swi/`，Rust |

SWI 拦截点在 vendored 核心内，**仅两处**，均为单行调用：

| 位置 | 现状 |
|---|---|
| `arm7tdmi.rs:504-506`（ARM 模式） | `ArmModeInstruction::SoftwareInterrupt => self.handle_exception(SoftwareInterrupt)` |
| `arm7tdmi.rs:628-630`（Thumb 模式） | `Instruction::Swi => self.handle_exception(SoftwareInterrupt)` |

改动量：两处各插入「若注册了 hook 且 immediate 命中分发表，则原生执行并跳过异常」；另需让解码器携带 SWI immediate。**合计约 50–100 行，跨 2–3 个文件**，非重写。

### 5.3 SWI 实现分档

| 档 | SWI | 说明 |
|---|---|---|
| **T1（BETA 必须）** | `CpuSet` `CpuFastSet` `Div` `DivArm` `Lz77UnCompWram/Vram` `HuffmanUnComp` `RlUnCompWram/Vram` `Halt` `Stop` `IntrWait` `VBlankIntrWait` `RegisterRamReset` `GetBiosChecksum` | 缺任一则大量游戏无法启动或黑屏 |
| **T2（BETA 应做）** | `Sqrt` `ArcTan` `ArcTan2` `BgAffineSet` `ObjAffineSet` `BitUnPack` `SoftReset` | 影响 mode 7、精灵缩放、部分 2D 游戏 |
| **T3（可延后）** | `Diff*UnFilter`×3 `MidiKey2Freq` `MultiBoot` `SoundDriver*` 段 | 低频；声音段依据 §六 结论可 Nop |

`ArcTan` 需与 BIOS 的 16.16 定点多项式**逐位对齐**，是 T2 中唯一高风险项，单独设验证点。

### 5.4 实现来源与许可纪律

所有 SWI 实现**依据 GBATEK 等公开硬件文档的行为规格自行编写**，参照公开算法，不复制任何受许可约束的第三方实现代码（先前路线会引入的 MPL-2.0 代码已整体移除）。

### 5.5 装载策略

```
用户指定真 BIOS（存在、size == 0x4000、校验和匹配已知值） → 优先使用
否则                                                        → 使用自建 stub BIOS
```

真 BIOS 路径须**同时校验长度与校验和**（如 `0xBAAE187F`），避免坏 dump 静默运行；校验失败时明确报错并回落 stub，不静默继续。

---

## 六、M4A 延期至 2.0 GA 之后

**原判断**（立项时）：HLE BIOS 下 M4A 游戏静音 → 必须补齐。
**核查结论**：原判断不成立。游戏自带 MP2k 驱动在卡带 ROM 中运行，调用的是自身符号而非 BIOS SWI；vgmdocs 明确商业游戏「几乎从不」使用 BIOS 内的那套系统函数。

**决议（2026-09-27）**：延期至 GA 之后，BETA 不做。理由：非可玩性前提；是全计划技术不确定性最高的工作；收益是渐进式音质提升。

> **该结论的适用边界（本次审计补充）**：M4A 的结论**不可外推**到解压类 SWI。游戏自带音频驱动 ≠ 游戏自带解压驱动；SWI 0x11–0x18 确实是游戏运行时主动调用的，故 §5.3 T1 必须实现。

**GA 后待办**：拦截 m4a 调用、以宿主语言重写混音，音质优于原驱动的 sample-and-hold；位置 `src/m4a.rs`（GA 前不建空占位文件）。

---

## 七、其他实现约定

### 7.1 `BETA` 水印

| 项 | 约定 |
|---|---|
| 位置 | 游戏画面左上角常驻 |
| 合成点 | Rust 侧（`overlay.rs`，出帧前一步），单一合成点 → 窗口/截图/录像全覆盖 |
| 机制 | **构建期 `GBA_BETA_OVERLAY` 开关**：发布构建**强制 ON 且无运行时关闭入口**；测试构建可 OFF（供未来帧校验基线使用） |
| GA | 关闭开关并移除 `gba_set_overlay` 运行时入口 |

> 修正前版「编译期/运行期开关」「无隐藏开关」「测试以关闭状态建基线」三处口径矛盾：现统一为**发布构建不可关闭、测试构建可关闭**。

### 7.2 NES 兼容键位（BETA）

| NES | → GBA | 备注 |
|---|---|---|
| A / B / Select / Start / D-pad | 同名 | 一一映射 |
| — | **L** | NES 无对应，默认绑 Z |
| — | **R** | NES 无对应，默认绑 X |

### 7.3 存档（识别机制已更正）

**GBA ROM 头部没有存档类型字段。** 识别机制为三级：

| 级 | 机制 | 说明 |
|---|---|---|
| 1 | ROM 正文签名串扫描 | `EEPROM_V###` / `SRAM_V###` / `FLASH_V###` / `FLASH512_V###` / `FLASH1M_V###`；长串优先于短串 |
| 2 | 外部数据库 | No-Intro / GBA DAT；BETA 不内置 |
| 3 | 运行时探测 | Flash 芯片 ID、EEPROM 位宽试探 |

**新增要求**：

- **检测失败或误判时可手动覆盖**：`gba_set_save_type()`，配置持久化。
- **`.srm` 字节布局须与 mGBA / VBA 通行格式对齐**，供用户跨模拟器迁移存档。
- 硬件支持：SRAM 32KB / Flash 64K·128K / EEPROM 512B·8K。
- 写入前必须确认介质类型，**类型不明时拒绝写入并报错**，不得默认按 SRAM 处理（防存档损毁）。

### 7.4 遗产能力的处置（显式声明，避免被当 bug 报）

| 能力 | GBA 侧 |
|---|---|
| NES 录像 / TAS（`.fm2`） / 回退 | **禁用**（格式与 CPU 架构不兼容） |
| netplay | **禁用** |
| 金手指（Game Genie / Action Replay） | **禁用**（NES 编码格式不通用） |
| Lua 脚本 | **暂不接入** |
| 截图 / 录像输出 | **保留**（前端能力，与核心无关） |

### 7.5 测试策略

| 层 | 内容 |
|---|---|
| **CI（进）** | jsmolka 合成测试集，`memory.gba` 为 SWI 实现的核心验收门；ARM 汇编单元测试 |
| **CI（进）** | **授权 homebrew** 做黄金帧哈希与音频冒烟（无版权风险） |
| **本地手工（不进 CI）** | 商业 ROM 实测清单（版权与来源策略 **S0 即定**，不留到 S4） |
| **F11QA** | 完整接入推迟至 §8.1 P3 |

### 7.6 输入格式边界

`.mb`（MultiBoot）与 overdump **显式拒绝并给出明确错误**，不静默失败。

---

## 八、阶段编排

| 阶段 | 内容 | 出口标准 | 估 |
|---|---|---|---|
| **S0** | 建 `f11gba` + `gba-core` 两个 crate，vendor 核心，落地 `ATTRIBUTION.md`；确定 ROM 来源策略 | `cargo check` 过；CMake 产出含新符号的 `fceux11_rust.lib`；NES 构建零回归 | 3–5 天 |
| **S1a** | stub BIOS（汇编 + 产物入库）；SWI hook 接入核心；`CpuSet`/`CpuFastSet`/`Div`/wait 类 | jsmolka `arm`/`thumb` 通过；可启动到游戏画面 | 1 周 |
| **S1b** | **T1 解压类 SWI**：LZ77×2 / Huffman / RL×2 / `RegisterRamReset` / `GetBiosChecksum` | **jsmolka `memory.gba` 全绿**；`bios.gba` 由 `#[ignore]` 转 pass | 1–1.5 周 |
| **S1c** | **T2 高级 SWI**：`Sqrt` / `ArcTan`×2 / `BgAffineSet` / `ObjAffineSet` / `BitUnPack` / `SoftReset` | 逐位对齐验证；mode 7 与精灵缩放样例通过 | 1–1.5 周 |
| **S2** | C ABI + Qt 前端：`.gba` 识别、240×160 渲染、音频接入、RTC 暴露、`BETA` 水印 | 可玩游戏，画面/音频正常；**性能验收线达标**（见下） | 2 周 |
| **S3** | NES 兼容键位、`.srm` 存档（含覆盖机制与跨模拟器互通）、即时存档、RTC | 存档跨会话可读、跨模拟器字节级互通、键位符合 §7.2 | 1–1.5 周 |
| **S4** | 手工实测清单逐游戏过 | 清单内游戏可玩，已知限制逐条编目 | 长尾 |

**合计约 8–10 周**（前版「约 3 周」的估算已作废，原因是 §5 的 BIOS 路线重估）。

**性能验收线（F-15 补入）**：在中档桌面（i5-8xxx / 16GB）下，240×160 稳定 59.7275 FPS，音频 underrun 计数为 0，实测 30 分钟无音画漂移。

### 8.1 GA 后排期

| 阶段 | 内容 | 估 |
|---|---|---|
| **P1** | M4A 补齐（§六） | 2–3 周 |
| **P2** | wait cycle 补齐、已知限制清零 | 另议 |
| **P3** | 接入 F11QA / KagamiQA（jsmolka 通道产品化） | 另议 |
| **P4** | 立体声输出评估（另开 SDL 设备） | 另议 |

### 8.2 GA 门禁（前版缺失，本次补入）

全部满足方可宣布 2.0 GA：

1. S0–S4 全部出口标准达成；
2. jsmolka **10/10 全绿**（含 `bios.gba`）；
3. 手工实测清单内游戏全部可玩，已知限制 **≤ 8 条**且逐条有据；
4. `.srm` 跨会话 + 跨模拟器（mGBA / VBA）字节级互通验证通过；
5. F11QA `pass_to_fail = 0`；
6. 性能验收线达标；
7. RTC 场景（宝可梦 RSE 类）实测通过，或明确列入已知限制。

---

## 九、风险登记册

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| **R1** | **vendor 分叉成本**。上游仍活跃 | 上游修复无法自动流入 | `ATTRIBUTION.md` 记录基线 SHA；补丁集最小化（仅 SWI hook）；定期评估合并 |
| **R2** | **无商业游戏验证**。合成测试 ≠ 商业游戏可跑 | BETA 可能大面积跑不动 | S4 手工实测 + GA 门禁第 3 条 |
| **R3** | **wait cycle 大面积未实现** | DMA / 精灵窗口敏感游戏异常 | 列入已知限制，S4 重点验证 |
| **R4** | **`ArcTan` 定点逐位对齐困难** | T2 延期或 mode 7 游戏异常 | 单独设验证点；T2 可裁剪，优先 T1 上线 |
| **R5** | **巴士因子**。上游维护者极少 | 上游停摆需自维护 19.5K 行 | MIT 无法律障碍；R1 缓解 |
| **R6** | **水印污染帧校验** | 未来 F11QA 基线不可比 | 测试构建可关（§7.1） |
| **R7** | **音频下混有损 + 立体声丢失** | 音质弱于专用 GBA 模拟器 | `(L+R)×0.5` 避免削波；立体声列 P4 |
| **R8** | **BETA 无 M4A 增强** | 混音沿用原驱动 sample-and-hold | 已显式声明；GA 后 P1 |
| **R9** | **stub BIOS 的 IRQ 间接跳转**若实现有误，表现为随机崩溃 | 难以定位 | 与 `irq.gba` 类用例对照；stub 极小，可逐条核 |
| **R10** | **CMake `GLOB_RECURSE` 不触发重配置** | 新 crate 未被纳入构建而不报错 | 改用 `CONFIGURE_DEPENDS` 或显式列出；S0 验证 |

---

## 十、不变式

1. **NES 核心零回归**：不得改变 F11QA 现有矩阵（`106P / 14F`，grade B）。`pass_to_fail` 必须为 0，出现即回滚。
2. **前端复用不打折**：不得为 GBA 另起 UI 框架或分叉 Qt 驱动。
3. **BIOS 不夹带任天堂代码**：绝不提交真 BIOS；真 BIOS 仅存于用户本地路径。
4. **许可可审计**：保留上游文件头与版权行 + 随发行物附许可全文；出处集中于 `ATTRIBUTION.md`。**目录/crate 可改名，文件头不可删。**
5. **BETA 水印在发布构建中不可关闭**；仅测试构建可关（§7.1）。
6. **HLE 与真 BIOS 双路径均须可用**，真 BIOS 须过长度与校验和双检。
7. **延期项须显式声明**：M4A、立体声、wait cycle、F11QA 接入均为**已决议的范围取舍**，不是遗漏；不得在 BETA 文案中暗示具备。
8. **GBA 侧为独立可裁剪项**：置于 cargo feature + CMake option 之后，NES-only 构建可物理剔除，结构性保证不变式 1。

---

## 十一、状态回写

阶段完成后：

1. 本文件 §八 对应行勾选并更新出口标准
2. `ATTRIBUTION.md` 更新基线 SHA 与补丁集
3. `COPYRIGHT_AUDIT.md` 同步新增来源
4. `CHANGELOG.md` 记入 Added 段
5. 已知限制逐条编目进本文件 §9

---

## 十二、待确认

| # | 事项 | 状态 |
|---|---|---|
| 1 | ROM 版权与来源策略（决定 S0 能否落地实测手段） | ⏳ **S0 前须定** |
| 2 | 阶段分支命名 | S0 开工时确定 |
| 3 | 商业 ROM 实测清单具体条目 | S4 前确定 |
| 4 | 2.0 GA 是否移除 `BETA` 水印 | 计划移除，GA 前确认 |
| 5 | 审查入口形式：出处仅存 `ATTRIBUTION.md`，还是同时写入本文件正文 | ⏳ 见 §十三 F-02 |

---

## 十三、第三方架构审计响应

审计报告：[`FCEUX11-v2.0_GBAEUX11构建计划_架构审计报告.md`](./FCEUX11-v2.0_GBAEUX11构建计划_架构审计报告.md)（r1，2026-09-27，结论：有条件不通过）。

### 13.1 逐条处置

| ID | 严重度 | 审计主张 | 处置 | 说明 |
|---|---|---|---|---|
| F-01 | P0 | HLE BIOS「零改动」不成立，SWI 语义在 mGBA C 侧 | **完全采纳** | 已独立核源 `bios.c` 的 `GBASwi16()` 确认。审计成立且描述的问题比前版更严重。**BIOS 路线整体重写**（§五），工期由 2–3 天改为 2–3 周 |
| F-02 | P0 | 上游匿名化致计划不可独立审查 | **部分采纳** | 出处集中于 `ATTRIBUTION.md`（§二「审查入口」），S0 首个 commit 落地。**但「目录命名去上游标识」是用户明确指令**，故不在计划正文具名。已列为 §十二#5 待用户裁定 |
| F-03 | P1 | 缺 SRC 设计 | **部分驳回 + 采纳结论** | 审计前提有误：`init_audio(output_rate, capacity)` 接受宿主率，核心内部即完成重采样，`rtrb` 流不存在「硬件原生率」，**无需我们实现 SRC**。但「分数样本数」问题成立且已纳入 §4.2（复用 FCEUX11 既有机制，不自造第二套） |
| F-04 | P1 | 存档识别机制描述错误 | **完全采纳** | 前版「按 ROM 头部 ID 识别」确属错误。已改写为三级机制（§7.3），并补入手动覆盖、`.srm` 跨模拟器互通、类型不明时拒写 |
| F-05 | P1 | RTC 缺席 | **完全采纳** | 属实。核心内已有 `hardware/rtc.rs`（S3511），仅需暴露。已进 S2 |
| F-06 | P1 | 即时存档能力未证实 | **采纳（降级为待核实）** | 核心内有 `Serialize/Deserialize` 派生与版本化状态。S0 出口标准加入「核实 savestate 能力与格式版本策略」，不在 S3 前假设 |
| F-07 | P1 | C ABI 契约不完整 | **完全采纳** | 已补错误码、`gba_set_bios`、`gba_unload_rom`/`gba_reset`、线程模型、UTF-8 路径约定（§4.1） |
| F-08 | P1 | `vendor/` 与 Cargo 语义冲突；汇编器工具链未落实 | **完全采纳** | 已改为独立 path 依赖 crate；`stub.bin` 入库免工具链依赖，`stub.s` + `REGENERATE.md` 保留可重建性（§四） |
| F-09 | P1 | 「前端零重写」低估；遗产能力未声明 | **完全采纳** | 已补 §7.4 显式处置表；S2 由 1–2 周上调至 2 周 |
| F-10 | P1 | 水印开关三处口径矛盾 | **完全采纳** | 已统一为「发布构建强制 ON 无运行时入口 / 测试构建可 OFF」（§7.1） |
| F-11 | P1 | 阶段编号悬空；工期矛盾；GA 门禁缺失 | **完全采纳** | 已重排 S0/S1a/S1b/S1c/S2/S3/S4；工期改为 8–10 周；新增 §8.2 GA 门禁 |
| F-12 | P2 | MPL 合规表述缺前提 | **已消解** | 自建 stub 后不再使用任何 MPL-2.0 代码，合规问题整体消失。MIT 义务表述已加强（§三） |
| F-13 | P2 | 「音源以单声道为主」不成立；增益未定义 | **完全采纳** | 已改为诚实表述（放弃立体声）+ 明确 `(L+R)×0.5` 下混与增益顺序（§4.3） |
| F-14 | P2 | `gba_read_audio` / `WriteSound` 分层混乱 | **完全采纳** | 已固定分层图（§4.3），调用方只调 `WriteSound` |
| F-15 | P2 | 无性能验收线 | **完全采纳** | 已补具体指标（§八 S2） |
| F-16 | P2 | 测试策略过弱；ROM 版权时点过晚 | **完全采纳** | 已分三层（§7.5），版权策略提前至 S0 前必答（§十二#1） |
| F-17 | P2 | CMake GLOB 脆弱 | **完全采纳** | 已登记 R10，S0 验证 |
| F-18 | P3 | 优化建议 | **部分采纳** | 采纳 1（feature 隔离→不变式 8）、2（BIOS 校验和→§5.5）、4（`.mb` 显式拒绝→§1.2/§7.6）、5（产品叙事）。删除 `m4a.rs` 空占位（YAGNI）已执行。6/7 留作后续 |

### 13.2 对审计结论的保留意见

审计整体质量高，**F-01 是本次修订的关键驱动**，其价值在于暴露了计划的自相矛盾。但有三点需记录在案：

1. **F-03 的事实前提不成立**（SRC 已由核心提供），若照单全收会导致重复实现。
2. **F-01 的方案成本被高估**。审计未掌握 vendored 核心的 SWI 分派点仅有两处单行调用这一事实，据此判断「工作量远超 S1」并不准确。实际改动约 50–100 行、跨 2–3 文件。
3. **F-02 与用户明确指令冲突**。合规匿名化（发行物命名）与计划可审查性可以分离，但审计主张的「计划正文具名」与用户「尽量少提及原项目」的指令直接冲突，已提交用户裁定（§十二#5）。

### 13.3 放行条件自评

| 审计放行条件 | 状态 |
|---|---|
| F-01 选定方案并写入计划，S1 估期重估 | ✅ 选定方案：自建 stub + Rust 原生 SWI（§五），已重排为 S1a/S1b/S1c |
| F-02 §二 具名上游 repo + 基线 SHA | ⚠️ 部分：出处集中于 `ATTRIBUTION.md`（S0 落地），计划正文未具名（依用户指令，待 §十二#5 裁定） |
| F-03 §4.2 补齐 SRC 与分数采样设计 | ✅ 已补（§4.2），并驳回「需自行实现 SRC」的前提 |
| F-04 §7.3 改写存档识别，S3 出口增补覆盖与互通 | ✅ 已补（§7.3、§八 S3、§8.2） |
| F-07 / F-14 / F-15（S2 放行条件） | ✅ 已补（§4.1、§4.3、§八 S2） |
| F-05 / F-16（GA 放行条件） | ✅ 已补（§1.1、§7.5、§8.2） |
| F-11 GA 门禁 | ✅ 已补（§8.2） |

**自评**：除 F-02 的具名形式待用户裁定外，审计所列全部 P0/P1/P2 均已处置，**具备进入 S0 的条件**。但 §十二 尚有 2 项待定（ROM 来源策略、审查入口形式），建议连同开工与否一并裁定。
