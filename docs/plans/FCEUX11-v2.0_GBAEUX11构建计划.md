# FCEUX11 v2.0 构建计划 — GBAEUX11 模块（GBA 运行能力移植）

> **STATUS: FINAL（r6 · 采纳 r3 终审）**（2026-09-27 六次修订；执行期发现按 §十一 回写，变更须记入 §十四）
> **模块名**：**GBAEUX11**（FCEUX11 Rust 侧的第二模拟核心）
> **版本**：v2.0（BETA 阶段）
> **日期**：2026-09-27 立项 / 二次（M4A 延期）/ 三次（音频契约）/ 四次（r1 审计）/ 五次（定稿）/ **六次（r3 终审，具名上游 + SWI 扩展点修正）**
> **分支**：S0 开工时创建
> **前置**：v1.18.1 已发布（`main` @ `3e33f2b`）；F11QA R4 gate green，grade B
> **关联**：`docs/plans/FCEUX11-v2.0_GBAEUX11构建计划_架构审计报告.md`（r3 终审）、`COPYRIGHT_AUDIT.md`、`DERIVATIVE_WORK_NOTICE.txt`

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
| **任何 ROM 的分发 / 下载 / 内置** | **与 NES 同策略：ROM 完全由玩家自行准备**。见 §7.7 |

---

## 二、核心选型实证

### 2.0 上游具名（构建可执行性要求）

| 项 | 值 |
|---|---|
| **项目** | **clementine** |
| **仓库** | `https://github.com/RIP-Comm/clementine` |
| **许可** | MIT |
| **基线 commit SHA** | `ee77922dd293b70e945458e104f3b2de794f0151`（`main`，2026-08-27，PGP 签名） |
| **wait cycle 跟踪 issue** | `https://github.com/RIP-Comm/clementine/issues/204` |
| **测试套件** | `emu/tests/jsmolka.rs`（对 `jsmolka/gba-tests`） |
| **唯一取用部分** | `emu/` crate（硬件核心）。`ui/`（egui 调试器）与 `src/main.rs`（CLI）**整包丢弃** |

> **命名纪律（分层，勿混淆）**：
> - **构建计划文档**（含本节）必须具名上游，供构建 agent 与审查者直接定位代码库。
> - **源代码 / 架构 / crate / 目录命名**继续使用功能性命名（`f11gba` / `gba-core` / `vendor/`），不散落上游标识。
> - **`ATTRIBUTION.md`** 保留完整出处与 MIT 许可全文（发行合规层）。

### 2.1 基本事实

| 维度 | 事实 |
|---|---|
| 许可 | MIT，无附加条款 |
| 核心规模 | `emu/` 45 文件 / **19,547 行** |
| 构建 | `cargo check` **37s 通过** |
| 依赖 | 仅 `rtrb` / `serde` / `serde_with` / `tracing` — **零 GUI 库** |
| 分层 | 硬件核心 / CLI / 调试器 UI 三分，**仅取硬件核心** |
| 活跃度 | 最后提交 2026-08-27；73 star |
| Rust 版本 | **edition 2024** — 与 FCEUX11 `src/rust` workspace 一致 |
| 存档类型识别 | 核心内 `BackupType::detect()` 扫描 ROM 正文签名串 |
| RTC | 核心内 `cpu/hardware/rtc.rs`（S3511 over GPIO） |
| 即时存档 | `Arm7tdmi` / `Bus` / `InternalMemory` 均 `#[derive(Serialize, Deserialize)]`，版本化 |
| **SWI HLE 骨架** | **`handle_swi_hle()`（`arm7tdmi.rs:1044`）已存在**，覆盖 0x00–0x0C；`_ => false`（`:1322`）回落到真 BIOS |

> **S0 剩余核验项**（r3 审计未覆盖）：基线 SHA 落库确认、`BackupType::detect` 实际行为、savestate 格式版本策略。

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
| `init_audio(output_rate: u32, capacity: usize)` | **采样率由调用方指定**，核心内部以 sample-and-hold 重采样（见 §4.2） |
| 视频 | 240×160，15-bit |
| `battery_data()` / `load_battery()` / `save_dirty` | 存档硬件在核心内；`.srm` 文件 I/O 需自写 |
| **`handle_swi_hle(swi_num, old_cpsr, return_addr) -> bool`** | **SWI 扩展点已存在**（`arm7tdmi.rs:1044`），由 `arm7tdmi.rs:670` 调用；返回 `false` 则回落到真 BIOS |

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

// 音频（调用层次见 §4.3；分数采样见 §4.2）
int32_t     gba_render_audio(int32_t *dst, uint32_t cap, uint32_t *out_frames);
                    // 返回 GBA_OK/GBA_ERR_*；实际产出帧数走 out_frames。
                    // 因每帧目标样本数为分数（44100/59.7275 ≈ 738.35），
                    // 以定点累加 + 残余结转决定本帧产出量（仿 sound.cpp:1359-1366 的 soundtsoffs/left），
                    // 且必然出现 out_frames 不等于 dst 所按 738 计算的值的情形。
void        gba_samples_per_frame_fixed(uint32_t *num, uint32_t *den);  // 返回帧率比分数
void        gba_set_overlay(int enable);   // 发布构建中 enable=0 返回 GBA_ERR_STATE（语义不可关闭）

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

**采样率转换归属（经 r3 终审核验确认）**：

- 上游 `Gba::init_audio(output_rate, capacity)` **接受调用方指定的宿主采样率**，核心内部即以 sample-and-hold 重采样到该率（`sound.rs:81-82`；`cycle_accumulator += cycles * output_rate`）。`rtrb` 流中**不存在「硬件原生率」**，故**不需要我们再实现 SRC**。
- **但分数样本问题仍在，且与之无关**：
  - `44100 / 59.7275 ≈ 738.35` 非整数；
  - 前版提议「复用 FCEUX11 既有 `frmRateAdjRatio` 机制」**经核验为错误**：`frmRateAdjRatio` 是**帧节流**参数（`sdl-throttle.cpp`），且在 `sound.cpp:759` 仅作为 `DoPCM` 定时 fudge 使用，**不参与防漂移**。
  - NES 真正的防漂移在 `FlushEmulateSound`（`sound.cpp:1293`）以 `soundtsoffs` / `left` 做**残余结转**（`sound.cpp:1359-1366`）。
  - → **GBAEUX11 必须自持同样的分数累加 + 残余结转**，不能复用 `frmRateAdjRatio`，也不应假设 `int` 帧长可表达 738.35。
- ABI 相应修正（见 §4.1）：`gba_render_audio` **不得以 `int` 返回值兼作错误码与样本数**，须分离。

**音质取舍**：核心使用 sample-and-hold 上采样，存在镜像/混叠劣化。登记为 R11（已知音质取舍），非缺陷。

### 4.3 音频分层与增益结构（前版分层混乱已更正）

固定分层，前端只与最外层打交道：

```
gba_step_frame()
   ├─ 步进 CPU 直到 VBlank
   ├─ 从 rtrb 抽取 f32 交织立体声（率 = 我们传入的宿主率）
   ├─ 下混：(L + R) × 0.5            ← 必须，先于放大，否则双声道同相必削波
   ├─ × 32768
   ├─ × 音量系数（FSettings.SoundVolume / 最大值）
   ├─ 饱和截断到 int32 → 帧缓冲
   └─ 分数累加：phase += samples_per_frame 分子×rate
                本帧产出 floor(phase/分母)，残余结转到下帧   ← 防漂移，见 §4.2
                    ↓
   驱动层：gba_render_audio(buf, cap, &n) → WriteSound(buf, n)
```

**调用方只调 `WriteSound`**；下混、增益、截断、分数采样全部在 `f11gba` 内完成，`WriteSound` 仅作环形缓冲搬运。

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

SWI 拦截点**已在上游核心中存在**，无需新建：

| 位置 | 内容 |
|---|---|
| `arm7tdmi.rs:670` | 异常处理中先尝试 HLE：`if self.handle_swi_hle(swi_num, old_cpsr, next_ins) { return; }` |
| `arm7tdmi.rs:1044` | `fn handle_swi_hle(&mut self, swi_num: u32, old_cpsr: Psr, return_addr: u32) -> bool` |
| `arm7tdmi.rs:1322` | `_ => false` —— 未覆盖的 SWI 回落真 BIOS |
| `arm7tdmi.rs:1327` | `swi_return()` 辅助函数 |

**故 S1 的实际工作量是「向 `handle_swi_hle` 的 `match` 补分支 + 实现算法」，而非搭建拦截机制。**

> **前版表述更正**：r5 曾称「两处各插入 hook、约 50–100 行、跨 2–3 文件」。此说法不准确——扩展点本已存在，改动集中于 `handle_swi_hle` 一处 `match`。但**「hook 小」不等于「SWI 语义小」**：真正的工作量在算法实现（LZ77 / Huffman / RL / 仿射 / wait 类），见 §5.3。

### 5.3 SWI 实现分档

**前置事实（已核源）**：`handle_swi_hle` 覆盖 0x00–0x0C，但其中**四个 wait 类分支是空壳**：

```rust
// arm7tdmi.rs:1146-1157
0x02 => { self.swi_return(old_cpsr, return_addr);   // 空壳
          /* just return because the main loop will handle waiting */ true }
0x03..=0x05 => { self.swi_return(old_cpsr, return_addr); true }   // 同样空壳
```

且 `RegisterRamReset`(0x01) 的 **IWRAM 清空被主动跳过**（`arm7tdmi.rs:1106-1115`，源码 TODO：*"would break IRQ handlers"*）。

**故「上游已实现」与「已实现」是两回事，分档如下**：

| 档 | SWI | 上游现状 | 缺口 |
|---|---|---|---|
| **T1-a** | `CpuSet` `CpuFastSet` `Div` `DivArm` `SoftReset` | ✅ 已实现 | 无（回归验证即可） |
| **T1-b** | `Halt` `Stop` `IntrWait` `VBlankIntrWait` | ⚠️ **空壳** | **必须真做**：halt 到中断、ack IF/IE 语义 |
| **T1-c** | `RegisterRamReset` | ⚠️ **部分实现** | IWRAM 清空缺失，须按 GBATEK 补全或显式记入已知限制 |
| **T1-d** | `Lz77UnCompWram/Vram` `HuffmanUnComp` `RlUnCompWram/Vram` `GetBiosChecksum` | ❌ **未 HLE**，回落真 BIOS | **必须补全**，缺则图形管线主干失效 |
| **T2** | `Sqrt` `ArcTan` `ArcTan2` `BgAffineSet` `ObjAffineSet` `BitUnPack` | ❌ 未 HLE | 影响 mode 7 / 精灵缩放 / 部分 2D |
| **T3** | `Diff*UnFilter`×3 `MidiKey2Freq` `MultiBoot` `SoundDriver*` 段 | ❌ 未 HLE | 低频；声音段依 §六 可 Nop |

`ArcTan` 需与 BIOS 的 16.16 定点多项式**逐位对齐**，是 T2 中唯一高风险项，单独设验证点。

**stub BIOS 下的硬性要求**：`handle_swi_hle` 覆盖 0x00–0x0C 之外的所有 SWI 会因 `_ => false` 落进 `handle_exception` 跳向 `0x00000008`，而 stub BIOS 不会实现它们 → 行为未定义。**T1-d 与 T2 必须全部补齐**，否则 stub 路线下这些 SWI 等于直接落进 stub 的空实现。

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
| 机制 | **构建期 `GBA_BETA_OVERLAY` 开关**：发布构建**强制 ON**；测试构建可 OFF（供未来帧校验基线使用） |
| 运行时入口 | `gba_set_overlay()` **始终导出**（避免 ABI 随构建变体漂移），但**发布构建中 `gba_set_overlay(0)` 返回 `GBA_ERR_STATE` 并不生效**；仅测试构建接受关闭 |
| GA | 关闭编译期开关；发布构建下该调用恒返回 `GBA_ERR_STATE` |

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

### 7.7 ROM 来源策略（定稿）

> **决议：ROM 完全由玩家自行准备，与现有 NES 游戏同策略。**

| 项 | 处置 |
|---|---|
| 分发物内置 ROM | **禁止** |
| 提供 ROM 下载 / 索引 / 网盘 | **禁止** |
| 商业 ROM 进入 CI | **禁止** |
| 商业 ROM 进入仓库 | **禁止**（含 .gitignore 约束） |
| 玩家本地 ROM | 正常使用，功能不因此受限 |
| CI / 回归用 ROM | **仅授权 homebrew** 与 jsmolka 合成测试集（§7.5） |

**推论**：本策略使 S0 之前原本悬置的「版权与来源」问题自动消解——**没有需要事先确定的 ROM 来源，只有「不得获取」这一条约束**。§十二 原 #1 随之关闭。

---

## 八、阶段编排

| 阶段 | 内容 | 出口标准 | 估 |
|---|---|---|---|
| **S0** | 建 `f11gba` + `gba-core` 两个 crate，vendor 核心，落地 `ATTRIBUTION.md`；确定 ROM 来源策略 | `cargo check` 过；CMake 产出含新符号的 `fceux11_rust.lib`；NES 构建零回归 | 3–5 天 |
| **S1a** | 回归验证 T1-a（已实现项）；**T1-b 真做**：`Halt`/`Stop`/`IntrWait`/`VBlankIntrWait` 的真等待语义 | jsmolka `arm`/`thumb` 通过；可启动到游戏画面；wait 类有专项验证（非空壳） | 1 周 |
| **S1b** | **T1-c** `RegisterRamReset` 补 IWRAM；**T1-d** 解压类：`Lz77×2` / `Huffman` / `Rl×2` / `GetBiosChecksum` | **jsmolka `memory.gba` 在 stub BIOS 下全绿**（判据重定义，见下） | 1–1.5 周 |
| **S1c** | **T2**：`Sqrt` / `ArcTan`×2 / `BgAffineSet` / `ObjAffineSet` / `BitUnPack` | 逐位对齐验证；mode 7 与精灵缩放样例通过 | 1–1.5 周 |
| **S2** | C ABI + Qt 前端：`.gba` 识别、240×160 渲染、音频接入（含 §4.2 分数采样）、RTC 暴露、`BETA` 水印 | 可玩游戏，画面/音频正常；**性能验收线达标**（见下）；30 分钟无音画漂移 | 2 周 |
| **S3** | NES 兼容键位、`.srm` 存档（含覆盖机制与跨模拟器互通）、即时存档（含**加载后重挂 `init_audio`**）、RTC | 存档跨会话可读、跨模拟器字节级互通、键位符合 §7.2、**savestate 往返后音频不哑** | 1–1.5 周 |
| **S4** | 手工实测清单逐游戏过 | 清单内游戏可玩，已知限制逐条编目 | 长尾 |

**合计约 8–10 周**（前版「约 3 周」的估算已作废，原因是 §5 的 BIOS 路线重估）。

**`bios.gba` 判据重定义（重要）**：上游测试**当前依赖真 BIOS**（README 硬编码 `gba_bios.bin`），其 `bios.gba` 失败基线是「以真 BIOS 运行」的结果。stub BIOS 是**新交付物**，不是「换个 blob」。
故 S1b 的验收判据是 **「`memory.gba` 在 stub 下全绿」**（该 ROM 专项测 SWI 0x0B/0x0C/0x11–0x18，正是 T1-d 的靶子）。
`bios.gba` **不作为 BETA 判据**——它测的是 BIOS 函数行为，在 stub 下部分语义本就不等价；是否转 pass 列为 GA 后事项（§8.1 P5）。

**性能验收线（F-15 补入）**：在中档桌面（i5-8xxx / 16GB）下，240×160 稳定 59.7275 FPS，音频 underrun 计数为 0，**连续 30 分钟无音画漂移**（此项直接检验 §4.2 的分数采样实现）。

### 8.1 GA 后排期

| 阶段 | 内容 | 估 |
|---|---|---|
| **P1** | M4A 补齐（§六） | 2–3 周 |
| **P2** | wait cycle 补齐、已知限制清零 | 另议 |
| **P3** | 接入 F11QA / KagamiQA（jsmolka 通道产品化） | 另议 |
| **P4** | 立体声输出评估（另开 SDL 设备） | 另议 |
| **P5** | `bios.gba` 在 stub 下的可达成范围评估 | 另议 |

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
| **R11** | **sample-and-hold 音质**。核心以 S&H 上采样，存在镜像/混叠 | 音质弱于高阶重采样 | 已知取舍，非缺陷；如需改善须改核心或自行重采样 |
| **R12** | **分数采样实现错误**。`int` 帧长无法表达 738.35，若实现偷懒取整必长期漂移 | 音画不同步 | ABI 分离错误码与样本数（§4.1）；30 分钟无漂移为 S2 硬判据 |
| **R13** | **wait 类空壳被误当已实现**（上游 `0x02` / `0x03..=0x05` 均为 `swi_return`） | 游戏以 100% CPU 空转、耗电、行为异常 | S1a 硬性要求真实现，并设专项验证 |

---

## 十、不变式

1. **NES 核心零回归**：不得改变 F11QA 现有矩阵（`106P / 14F`，grade B）。`pass_to_fail` 必须为 0，出现即回滚。
2. **前端复用不打折**：不得为 GBA 另起 UI 框架或分叉 Qt 驱动。
3. **BIOS 不夹带任天堂代码**：绝不提交真 BIOS；真 BIOS 仅存于用户本地路径。
4. **许可可审计**：保留上游文件头与版权行 + 随发行物附许可全文；出处集中于 `ATTRIBUTION.md`。**目录/crate 可改名，文件头不可删。**
5. **BETA 水印在发布构建中不可关闭**：`gba_set_overlay(0)` 在发布构建返回 `GBA_ERR_STATE`；仅测试构建可关（§7.1）。
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

## 十二、未决项（定稿时全部关闭）

| # | 事项 | 决议 |
|---|---|---|
| 1 | ~~ROM 版权与来源策略~~ | ✅ **已定：ROM 完全由玩家自行准备，与 NES 同策略**（§7.7） |
| 2 | ~~审查入口形式~~ | ✅ **已定：计划正文具名上游（clementine + URL + 基线 SHA + issue #204），见 §二.0；源代码/架构/目录命名仍功能性** |
| 3 | 阶段分支命名 | S0 开工时确定（唯一剩余的执行期决定） |
| 4 | 商业 ROM 实测清单具体条目 | 属 §7.5 本地手工环节，由执行者按 §7.7 约束自行选取 |
| 5 | 2.0 GA 是否移除 `BETA` 水印 | 计划移除，GA 前确认（§7.1 已定机制，GA 只需确认执行） |

---

## 十三、第三方架构审计响应

审计报告：[`FCEUX11-v2.0_GBAEUX11构建计划_架构审计报告.md`](./FCEUX11-v2.0_GBAEUX11构建计划_架构审计报告.md)

| 轮次 | 日期 | 结论 | 本计划的响应 |
|---|---|---|---|
| **r1** 初审 | 2026-09-27 | 有条件不通过（4×P0） | 触发 **r4 修订**：BIOS 路线整体重写，工期重估 |
| **r2** 复审 | 2026-09-27 | 有条件通过（仅 S0） | 本计划 r5 未采纳其「正文不具名」折中 |
| **r3** 终审 | 2026-09-27 | **批准进入 S0 与 S1** | 触发 **r6 修订**：具名上游 + SWI 扩展点修正 + wait 类空壳发现 |

### 13.1 r1 逐条处置（已闭环）

| ID | 严重度 | 审计主张 | 处置 | 说明 |
|---|---|---|---|---|
| F-01 | P0 | HLE BIOS「零改动」不成立，SWI 语义在 mGBA C 侧 | **完全采纳** | 已独立核源 `bios.c` 的 `GBASwi16()` 确认。**BIOS 路线整体重写**（§五） |
| F-02 | P0 | 上游匿名化致计划不可独立审查 | **r3 已闭合** | r5 曾按「正文不具名」折中；r3 经用户澄清后**改为正文具名**（§二.0） |
| F-03 | P1 | 缺 SRC 设计 | **r3 终审完全接受我方主张** | `init_audio` 确含重采样；但分数样本问题独立成立，见 §4.2 |
| F-04 | P1 | 存档识别机制描述错误 | **完全采纳** | 三级识别 + 手动覆盖 + 跨模拟器互通（§7.3） |
| F-05 | P1 | RTC 缺席 | **完全采纳** | 核心内已有 `rtc.rs`，仅需暴露（§1.1、S2） |
| F-06 | P1 | 即时存档能力未证实 | **采纳（转为 S0 核验项）** | §二 已记「savestate 格式版本」为 S0 剩余核验项 |
| F-07 | P1 | C ABI 契约不完整 | **完全采纳** | 错误码 / 生命周期 / 线程模型 / UTF-8 路径（§4.1） |
| F-08 | P1 | `vendor/` 与 Cargo 语义冲突；汇编器未落实 | **完全采纳** | 独立 path 依赖 crate；`stub.bin` 入库免工具链依赖（§四） |
| F-09 | P1 | 「前端零重写」低估；遗产能力未声明 | **完全采纳** | §7.4 显式禁用表；S2 上调至 2 周 |
| F-10 | P1 | 水印开关口径矛盾 | **完全采纳** | 统一为发布构建语义不可关闭（§7.1、不变式 5） |
| F-11 | P1 | 阶段编号悬空；工期矛盾；GA 门禁缺失 | **完全采纳** | 重排 S0–S4；§8.2 GA 门禁 |
| F-12 | P2 | MPL 合规表述缺前提 | **已消解** | 自建 stub 后不使用任何 MPL-2.0 代码 |
| F-13 | P2 | 「音源以单声道为主」不成立；增益未定义 | **完全采纳** | §4.3 明确放弃立体声 + `(L+R)×0.5` |
| F-14 | P2 | 音频分层混乱 | **完全采纳** | §4.3 分层图 |
| F-15 | P2 | 无性能验收线 | **完全采纳** | §八 S2 + §8.2 |
| F-16 | P2 | 测试策略过弱；ROM 版权时点过晚 | **完全采纳** | §7.5 三层；版权策略落为 §7.7「不得获取」 |
| F-17 | P2 | CMake GLOB 脆弱 | **完全采纳** | 登记 R10 |
| F-18 | P3 | 优化建议 | **部分采纳** | 1→不变式 8；2→§5.5；4→§7.6；`m4a.rs` 空占位已删 |

### 13.2 r3 终审逐条处置（本轮）

| ID | 内容 | 处置 | 落地位置 |
|---|---|---|---|
| **裁定 ③** | **计划正文必须具名 clementine + URL + 基线 SHA + issue #204** | **完全采纳** | 新增 §二.0 上游具名表；命名纪律分层写明 |
| **裁定 ①** | `init_audio` 已含重采样，无需自研 SRC | **完全接受** | §4.2 已据此表述（r5 即为此结论，r3 完成核验） |
| **裁定 ① 尾巴 1** | 分数样本问题独立成立；**「复用 `frmRateAdjRatio`」被驳回** | **完全采纳** | §4.2 已改为「自持定点累加 + 残余结转（仿 `sound.cpp:1359-1366`）」；ABI 分离错误码与样本数（§4.1）；登记 R12 |
| **裁定 ① 尾巴 2** | sample-and-hold 音质取舍 | **采纳** | §4.2 末段 + R11 |
| **裁定 ②** | hook 行号命中但 **`handle_swi_hle` 已存在**；wait 类是空壳 | **完全采纳** | §5.2 改为「扩展点已存在，补 match 分支」；§5.3 按 T1-a/b/c/d 重分档；登记 R13 |
| **N-04** | `Halt`/`Stop`/`IntrWait`/`VBlankIntrWait` 为空壳 | **采纳（已自行核源 `arm7tdmi.rs:1146-1157` 确认）** | §5.3 T1-b「必须真做」+ S1a 出口设专项验证 |
| **N-09** | `RegisterRamReset` 的 IWRAM 清空被上游跳过 | **采纳（已核源 `arm7tdmi.rs:1106-1115` 确认）** | §5.3 T1-c「部分实现」+ S1b 补全或记入已知限制 |
| **N-10** | savestate 不序列化音频 producer | **采纳** | S3 出口增补「savestate 往返后音频不哑」；§7.3 相关 |
| **N-11** | 上游当前强制真 BIOS，`bios.gba` 基线依赖真 BIOS | **采纳** | `bios.gba` **移出 BETA 判据**；S1b 判据改为「`memory.gba` 在 stub 下全绿」；新增 §8.1 P5 |
| **N-07** | `gba_set_overlay` 与「无运行时入口」冲突 | **采纳** | §7.1 改为「ABI 恒导出，发布构建中 `gba_set_overlay(0)` 返回 `GBA_ERR_STATE`」；不变式 5 同步 |
| **N-08** | R1「仅 SWI hook」措辞；S1a 出口过度承诺 | **采纳** | §5.2 加「前版表述更正」注；S1a 出口改为「wait 类有专项验证」 |

### 13.3 自评与放行

**已独立复核的 r3 关键 claim**（构建方自行核源，非转述）：

| claim | 核验 | 结果 |
|---|---|---|
| `handle_swi_hle` 存在于 `arm7tdmi.rs:1044`，由 `:670` 调用，`:1322` 为 `_ => false` | 直读源码 | ✅ 属实 |
| `0x02` 与 `0x03..=0x05` 为空壳 `swi_return` | 直读 `arm7tdmi.rs:1146-1157` | ✅ 属实 |
| `RegisterRamReset` IWRAM 清空被跳过（TODO） | 直读 `arm7tdmi.rs:1106-1115` | ✅ 属实 |
| NES 防漂移在 `FlushEmulateSound` 的 `soundtsoffs`/`left` 残余结转，而非 `frmRateAdjRatio` | 直读 `sound.cpp:1293,1359-1366` | ✅ 属实，我方 r5 判断有误 |

**自评**：r3 全部主张已处置。**我方 r5 存在两处实质判断失误**（① 误以为需新建 SWI hook，实际扩展点已存在；② 误提议复用 `frmRateAdjRatio` 防漂移），均已更正。

**放行结论**：r3 终审已批准 S0 与 S1；本计划已同步至 r6。

---

## 十四、变更记录

| 版本 | 日期 | 变更 | 触发源 |
|---|---|---|---|
| r1 | 2026-09-27 | 立项。移植 GBA 硬件核心，Qt 前端复用，默认 HLE BIOS，M4A 补齐 | 用户决策 |
| r2 | 2026-09-27 | M4A 延期至 GA 后；工期收敛；新增 R8 | 联网核查推翻「HLE 下 M4A 静音」判断 |
| r3 | 2026-09-27 | 音频契约更正为 `WriteSound(int32*, int)`；标注 `WriteSound` 不施加音量 | 核实 `dface.h` / `sdl-sound.cpp` |
| r4 | 2026-09-27 | **BIOS 路线整体重写**：自建 stub + Rust 原生 SWI；工期重估 8–10 周；新增 §十三 审计响应、§8.2 GA 门禁；F-04/05/07/08/09/10/11/13/14/15/16/17 处置 | 第三方架构审计（CONDITIONAL FAIL） |
| **r5** | 2026-09-27 | **定稿**：STATUS → FINAL；新增 §7.7 ROM 来源策略（玩家自备，同 NES）；关闭 §十二 全部未决项 | 用户裁定 |
| **r6** | 2026-09-27 | **采纳 r3 终审**：① §二.0 上游具名（clementine + URL + 基线 SHA + issue #204）；② §5.2 更正 SWI 扩展点为**已存在的 `handle_swi_hle`**；③ §5.3 按 T1-a/b/c/d 重分档，wait 类与 `RegisterRamReset` 降级为「空壳/部分实现」；④ §4.2 撤回「复用 `frmRateAdjRatio`」，改为自持定点累加 + 残余结转，ABI 分离错误码与样本数；⑤ `bios.gba` 移出 BETA 判据；⑥ savestate 后重挂音频；⑦ 新增 R11/R12/R13 | r3 终审（批准 S0/S1） |

> **定稿后的变更纪律**：本计划状态为 FINAL。执行期若发现计划与实态不符，**先记入本表再改**，不得静默偏离 §八 的阶段出口标准与 §十 的不变式。
