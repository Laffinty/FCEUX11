# FCEUX11 v2.0 构建计划 — GBAEUX11 模块（GBA 运行能力移植）

> **STATUS: FINAL（r8 · S1 开工前的矛盾点排查与路线重定）**（2026-09-28 多次修订；执行期发现按 §十一 回写，变更须记入 §十四）
> **模块名**：**GBAEUX11**（FCEUX11 Rust 侧的第二模拟核心）
> **版本**：v2.0（BETA 阶段）
> **日期**：2026-09-27 立项 / 二次（M4A 延期）/ 三次（音频契约）/ 四次（r1 审计）/ 五次（定稿）/ 六次（r3 终审，具名上游 + SWI 扩展点修正）/ **七次（r7，S0 执行期回写）** / **八次（r8，GBA 逻辑层 crate → 模块）**
> **分支**：`wip2.0`（S0 开工时创建，已建）
> **前置**：v1.18.1 已发布（`main` @ `3e33f2b`）；F11QA R4 gate green，grade B
> **关联**：`docs/history/reports/FCEUX11-v2.0_GBAEUX11-架构审计报告.md`（r3 终审，已归档）、`COPYRIGHT_AUDIT.md`、`DERIVATIVE_WORK_NOTICE.txt`

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
> - **源代码 / 架构 / 目录命名**继续使用功能性命名（`src/gba/` / `crates/gba-core/`），不散落上游标识。
> - **`ATTRIBUTION.md`** 保留完整出处与 MIT 许可全文（发行合规层）。
>
> **crate 名订正（r8）**：原写「`f11gba` / `gba-core` / `vendor/`」。其中 `f11gba`
> **不再是 crate 名** —— 工具链约束使其无法作为独立 crate 存在，见 §十四 r8。
> 现在只有一个新 crate（`gba-core`），GBA 逻辑层是根 crate 内的**模块** `src/gba/`。

### 2.1 基本事实

| 维度 | 事实 |
|---|---|
| 许可 | MIT，无附加条款 |
| 核心规模 | `emu/src` 45 文件 / **22,311 行**（见下方口径订正） |
| 构建 | `cargo check` **37s 通过** |
| 依赖 | 仅 `rtrb` / `serde` / `serde_with` / `tracing` — **零 GUI 库** |
| 分层 | 硬件核心 / CLI / 调试器 UI 三分，**仅取硬件核心** |
| 活跃度 | 最后提交 2026-08-27；73 star |
| Rust 版本 | **edition 2024** — 与 FCEUX11 `src/rust` workspace 一致 |
| 存档类型识别 | 核心内 `BackupType::detect()` 扫描 ROM 正文签名串 |
| RTC | 核心内 `cpu/hardware/rtc.rs`（S3511 over GPIO） |
| 即时存档 | `Arm7tdmi` / `Bus` / `InternalMemory` 均 `#[derive(Serialize, Deserialize)]`，版本化 |
| **SWI HLE 骨架** | **`handle_swi_hle()`（`arm7tdmi.rs:1044`）已存在**，覆盖 0x00–0x0C；`_ => false`（`:1322`）回落到真 BIOS |

> **S0 剩余核验项 —— 已于 2026-09-28 全部核验完毕（结论见下）**。

#### S0 核验结论（2026-09-28）

| 核验项 | 结论 |
|---|---|
| **基线 SHA 落库确认** | ✅ `ee77922dd293…` **正是上游 `main` 的当前 HEAD**（实测 `git fetch` 该 SHA 成功，且 HEAD 解析到同一 commit）。**无漂移**，R1 的即时风险低于原判。 |
| **`BackupType::detect` 实际行为** | ⚠️ 行为与 §7.3 大体一致，但有两处计划未载：① 它是**私有 `fn detect`**（`internal_memory.rs:75`），不是 `pub` —— 计划的 `gba_get_save_type()` / `gba_set_save_type()` 需要另找调用路径或提升其可见性；② 实际匹配的签名串是 `EEPROM_V` / `FLASH1M_V` / `FLASH512_V` / `FLASH_V` / `SRAM_V` / **`SRAM_F_V`**，计划漏了 `SRAM_F_V`。 |
| **savestate 格式版本策略** | ⚠️ 上游**只有 `#[derive(Serialize, Deserialize)]`，没有任何序列化/反序列化 API**（无 `bincode` / `postcard` / `ron` 依赖，全仓库搜不到 `serialize(` / `deserialize(` 调用点）。即**格式选择与版本策略完全由本项目决定**，不是「沿用上游」。§4.1 的 `gba_savestate_{size,save,load}` 需自选格式 + 加版本头。 |

> **规模口径订正**：原文「`emu/` 45 文件 / 19,547 行」的行数是用 PowerShell `Measure-Object -Line` 统计的，该 cmdlet 在本机对大文件数组少计。**实测物理行数为 22,311 行**（`count(\n)` 与 `len(splitlines())` 一致；单文件对照 `arm7tdmi.rs` 实际 2,743 行 vs PowerShell 报 2,344）。文件数 45 正确（指 `emu/src`；`emu/tests/jsmolka.rs` 112 行未纳入）。
> **影响**：R5「上游停摆需自维护 19.5K 行」应改为 **22.3K 行（+14%）**。

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
>
> **「保留上游文件头」在实际上无处可履行（2026-09-28 实测）**：
> vendor 的 45 个源文件中，**44 个完全没有任何版权或许可声明**，
> 唯一例外是 `src/cpu/thumb/alu_instructions.rs`。因此本条的义务**不通过逐文件
> 文件头履行**，而由 **crate 根的 `LICENSE`（MIT 全文，Copyright (c) 2024 RIPsters）
> + `crates/gba-core/ATTRIBUTION.md` + 随发行物附许可全文** 三者承担。
> 本条的准确含义是「**不得删除上游原有的一切署名**」，而非「须给每个文件补头」——
> 给 44 个无标注的文件补上署名会误归属上游从未标注的材料。
> 详见 `src/rust/crates/gba-core/ATTRIBUTION.md` §2。

---

## 四、目录布局

> **r8 修订（2026-09-28）**：原计划的 `f11gba` **独立 crate 已被证明不可构建** ——
> rustc 1.96 fat LTO 无法加载依赖链中第二个 archive 的 bitcode，六组对照实验
> 证明唯一变量是「是不是独立 crate」，与代码量、依赖关系、泛型均无关。
> 因此 GBA 逻辑层降级为**根 crate 的模块** `src/gba/`。详见 §十四 r8 与 R14。
>
> **原布局（已作废）**：
> ```
> src/rust/crates/
> ├── f11gba/                      # 独立 crate —— 不可构建，见 r8
> │   ├── ATTRIBUTION.md
> │   ├── bios/{stub.s, stub.bin, REGENERATE.md}
> │   ├── src/{lib.rs, ffi.rs, swi/*, save.rs, overlay.rs}
> │   └── build.rs
> └── gba-core/
> ```
>
> **现行布局**：
```
src/rust/
├── src/
│   ├── lib.rs                    # GBA C ABI 持有点 + swi_hook 注册
│   └── gba/                      # GBA 逻辑层（根 crate 模块，非独立 crate）
│       ├── mod.rs
│       ├── ffi.rs                # C ABI（§4.1 的全部函数，S2 落地）
│       ├── bios/
│       │   ├── stub.s            # 自建 stub BIOS 源码（ARM 汇编）
│       │   ├── stub.bin          # 预汇编 16KB 产物（入库，免工具链依赖）
│       │   └── REGENERATE.md     # 重新汇编的步骤与校验哈希
│       ├── swi/                  # SWI 原生实现（替代 BIOS 代码）
│       │   ├── mod.rs            #   分发表 0x00–0x2A
│       │   ├── decompress.rs     #   LZ77 / Huffman / RL / UnFilter
│       │   ├── bitunpack.rs
│       │   ├── math.rs           #   Div / Sqrt / ArcTan / ArcTan2
│       │   ├── affine.rs         #   BgAffineSet / ObjAffineSet
│       │   ├── memory.rs         #   CpuSet / CpuFastSet / RegisterRamReset
│       │   └── wait.rs           #   Halt / Stop / IntrWait / VBlankIntrWait
│       ├── save.rs               # 存档类型覆盖、.srm 布局、RTC 区
│       └── overlay.rs            # BETA 水印
└── crates/
    └── gba-core/                 # vendor 核心 = 独立 path 依赖 crate
        ├── Cargo.toml            # 依赖全部内联（本 workspace 无 workspace.dependencies）
        ├── LICENSE               # 上游 MIT 全文逐字副本
        ├── ATTRIBUTION.md        # 上游来源 / 许可 / 基线 SHA / 本地修改（唯一权威出处）
        └── src/                  # 45 文件与上游逐字节相同（S0 状态）；S1b 起有本地修改
```

**为什么不降级成「全塞进 lib.rs」而保留模块目录**：计划 §四 的意图是「GBA 逻辑
独立成层 + 清晰的 C ABI 边界」，这两点由**模块目录**同样满足；被工具链否掉的
只有**编译单元边界**这一层。将来 rustc 修复该缺陷后，模块可在不改调用方的前提下
升级为独立 crate。

**Cargo 语义（修正前版冲突）**：`vendor/` 置于 crate 根下、位于 `src/` 之外时
**Cargo 不会编译其中的 `.rs`**。故 vendor 核心仍采用**独立 path 依赖 crate**
（`crates/gba-core/`，自带 `Cargo.toml`），而非 `#[path]` 挂载或塞进 `src/vendor/`。
此条只针对 **vendor 的 .rs**；我们自己写的代码没有该限制，可直接放 `src/gba/`。

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

// 音频（调用层次见 §4.3；分数采样见 §4.2）
int32_t     gba_render_audio(int32_t *dst, uint32_t cap, uint32_t *out_frames);
                    // 返回 GBA_OK/GBA_ERR_*；实际产出帧数走 out_frames。
                    // 因每帧目标样本数为分数（44100/59.7275 ≈ 738.35），
                    // 以定点累加 + 残余结转决定本帧产出量（仿 sound.cpp:1359-1366 的 soundtsoffs/left），
                    // 且必然出现 out_frames 不等于 dst 所按 738 计算的值的情形。
void        gba_samples_per_frame_fixed(uint32_t *num, uint32_t *den);  // 返回帧率比分数
void        gba_set_overlay(int enable);   // 见 §7.1 双策略；发布构建中 enable=0 返回 GBA_ERR_STATE（语义不可关闭）

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

**调用方只调 `WriteSound`**；下混、增益、截断、分数采样全部在 `src/gba/` 内完成，`WriteSound` 仅作环形缓冲搬运。

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
| **stub BIOS（16KB）** | 复位向量 → 跳转到卡带入口；异常向量；IRQ 经 `0x03007FFC` 间接跳转 | `src/gba/bios/stub.s`，约 30 条指令 |
| **SWI 语义** | 0x00–0x2A 的实际功能 | `src/gba/swi/`，Rust |

SWI 拦截点**已在上游核心中存在**，无需新建：

| 位置 | 内容 |
|---|---|
| `arm7tdmi.rs:670` | 异常处理中先尝试 HLE：`if self.handle_swi_hle(swi_num, old_cpsr, next_ins) { return; }` |
| `arm7tdmi.rs:1044` | `fn handle_swi_hle(&mut self, swi_num: u32, old_cpsr: Psr, return_addr: u32) -> bool` |
| `arm7tdmi.rs:1322` | `_ => false` —— 未覆盖的 SWI 回落真 BIOS |
| `arm7tdmi.rs:1327` | `swi_return()` 辅助函数 |

> **r8 关键补充：`handle_swi_hle` 是私有 `fn`，不是 `pub`**（2026-09-28 核源）。
> 这意味着「SWI 实现放在我们这边、核心调用」**不能靠改自己的 crate 完成** ——
> 必须先让核心愿意转交控制权。原文「S1 的实际工作量是向 `handle_swi_hle` 的
> `match` 补分支」隐含了「我们能改那个 `match`」，成立的前提是代码在同一个
> crate 内；在 r8 的模块化布局下**不成立**。见下方「扩展点接线」。

#### 扩展点接线（gba-core 的本地修改，共 5 处 —— 仅 S0' 接缝，补丁集总数见 R15）

gba-core 暴露一个**可选回调字段**，由根 crate 在 `gba_init()` 时注入。
根 crate 用**函数指针**传入，不产生第二个 rlib 依赖（这是 r8 唯一可行的接法）：

| # | gba-core 改动 | 目的 |
|---|---|---|
| 1 | `pub type SwiHook = fn(&mut Arm7tdmi, u32, Psr, u32) -> bool;` | 回调签名：拿得到 `&mut self`，故 hook 内可读写 BIOS flags、寄存器。用 `fn` 指针而非 trait object，保证依赖单向：根 crate 依赖 gba-core，反之不成立 |
| 2 | `Arm7tdmi` 新增 `#[serde(skip)] pub swi_hook: Option<SwiHook>` | 存放外部实现；`serde(skip)` 保证不进 savestate（函数指针无可恢复语义，载入存档也不应依赖它） |
| 3 | `impl Default for Arm7tdmi` 初始化为 `None` | 手工 `Default` impl 必须补上，否则编译不过 |
| 4 | `handle_swi_hle` 开头：`if let Some(h) = self.swi_hook { if h(self, …) { return true; } }` | **先问外部**，未处理才走核心自己的 match |
| 5 | `swi_return`（及 hook 需要的 `bus` 访问）提升为 `pub` | 没有它，hook 能服务调用却无法从调用返回 |

> **r8 原文写「4 处」，实为 5 处** —— 漏了 `Default` impl 的字段初始化。
> 已在 S0' 实施时按实际改动数订正，并同步到 `crates/gba-core/ATTRIBUTION.md` §3.2 / §4。

**优先级语义**：hook 先于核心 match 被询问，因此 `src/gba/swi/` 可以**覆盖**
核心已有的实现（如 T1-b 的 wait 类），也可以只补核心缺失的部分（T1-d / T2）。
两者共存不冲突。

**这 5 处改动落在 R1 允许的「仅 SWI hook」补丁集内**，不改动任何上游逻辑：
现有 match 分支、`_ => false` 回落、以及其余全部函数体原样保留。
须记入 `crates/gba-core/ATTRIBUTION.md` 的本地修改节（§十一 第 2 项）。
`gba-core` 在 **S0 结束时是 0 处修改**（已逐文件 sha256 验证），
**本地修改自 S0' 起**（r8 原文误写「自 S1b 起」—— S0' 就要接线，已订正）。

> **前版表述更正**：r5 曾称「两处各插入 hook、约 50–100 行、跨 2–3 文件」。此说法不准确——扩展点本已存在，改动集中于 `handle_swi_hle` 一处 `match`。但**「hook 小」不等于「SWI 语义小」**：真正的工作量在算法实现（LZ77 / Huffman / RL / 仿射 / wait 类），见 §5.3。
>
> **r8 再次更正**：上一条「改动集中于 `handle_swi_hle` 一处 `match`」在 r8 布局下**不成立**——
> 该函数是 gba-core 的私有方法，我们的模块无法触及。改为「gba-core 暴露 hook 字段 +
> 根 crate 注入实现」，共 4 处本地修改（见上方「扩展点接线」）。工作量级仍是「小」，
> 但落点从「改一处 match」变成「改 4 处 + 一条注册链」。

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
| **T1-b** | `Halt` `Stop` `IntrWait` `VBlankIntrWait` | ⚠️ **不仅空壳，机制本身不存在**（见下） | **必须新建 CPU 状态**：halt/stop 字段 + ARM/Thumb 两条 step 路径 + 恢复条件 |
| **T1-c** | `RegisterRamReset` | ⚠️ **部分实现** | IWRAM 清空缺失，须按 GBATEK 补全或显式记入已知限制 |
| **T1-d** | `Lz77UnCompWram/Vram` `HuffmanUnComp` `RlUnCompWram/Vram` `GetBiosChecksum` | ❌ **未 HLE**，回落真 BIOS | **必须补全**，缺则图形管线主干失效 |
| **T2** | `Sqrt` `ArcTan` `ArcTan2` `BgAffineSet` `ObjAffineSet` `BitUnPack` | ❌ 未 HLE —— **r23 订正：`0x08`/`0x09`/`0x0A` 核心已实现但 `0x09` 是错的**（见 L9） | 影响 mode 7 / 精灵缩放 / 部分 2D |
| **T3** | `Diff*UnFilter`×3 `MidiKey2Freq` `MultiBoot` `SoundDriver*` 段 | ❌ 未 HLE | 低频；声音段依 §六 可 Nop |

> **T1-b 缺口比原表严重（2026-09-28 核源）**：原文称四个 wait 类分支是「空壳」，
> 暗示只要把 `swi_return` 换成真逻辑即可。实测**上游根本没有 halt 机制**：
> - `Arm7tdmi`（`arm7tdmi.rs:144-169`）**没有 halted / stopped 任何字段**；
> - `step()`（`:705` 起，ARM 与 Thumb 各一条路径）**没有 halt 分支**，
>   每步都直接检查 IRQ 并执行指令。
>
> 所以 T1-b 至少要：给 `Arm7tdmi` 加状态字段（须 `#[serde(...)]` 兼顾 savestate）、
> 在两条 `step` 路径都插入 halt 判据、实现 `Halt`（任意使能中断恢复）与
> `Stop`（等 LCD 控制器）恢复条件的差异。**这是 CPU 模型级改动，不是 1 周的补丁。**
> 已在 `src/gba/swi/wait.rs` 的模块注释中记录同一事实。

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
| **S0** | ~~建 `f11gba` + `gba-core` 两个 crate~~ → **实际落地**：vendor `gba-core` + 根 crate 持 GBA C ABI + `ATTRIBUTION.md`；确定 ROM 来源策略 | `cargo check` 过；CMake 产出含新符号的 `fceux11_rust.lib`；NES 构建零回归 | 3–5 天 | **✅ 已完成**（2026-09-28） |
| **S0'** | 建立 SWI 扩展点接线：gba-core 的 **5 处**本地修改 + 根 crate 注册链（§5.2「扩展点接线」） | ~~`gba_swi_probe()` 经真实 SWI 调用返回正确分派结果~~ → **据实改写**：真实 ROM 里的 `SWI` 指令必须带着它编码的号码到达 `dispatch`（S0' 按设计**不认领任何号码**，「返回正确分派结果」要到 S1 才有对象，改写依据见 §十四 r9）；`cargo check` 两态 + staticlib 符号 + NES 零回归 | 1–2 天 | ✅ 已完成（2026-09-29） |
| **S1a-0** | **stub BIOS**（§5.2 第一层）：复位向量 → 写 `WAITCNT` → 跳卡带入口；异常向量；IRQ 经 `0x03007FFC` 间接跳转 | 合成卡带能被交到控制权（入口代码真的跑起来）；`WAITCNT` 被写成 `0x4317` | 1 天 | ✅ 已完成（2026-09-29） |
| **S1a-1** | 回归验证 T1-a（已实现项）；**T1-b 真做**：给 `Arm7tdmi` 加 halt/stop 状态、在 `step()` 插入判据、实现 `Halt`/`Stop`/`IntrWait`/`VBlankIntrWait` | **出口判据已按 §十四 r11 裁决 (a) 重定义**：wait 类专项验证（非空壳且 CPU 确实停住）✅；T1-a 回归验证 ✅；stub BIOS 的启动与 IRQ 全链路端到端 ✅。~~jsmolka `arm`/`thumb` 通过；可启动到游戏画面~~ → **移至 S2**（判据自身依赖帧缓冲，见 r11） | ~~1 周~~ → **2–3 周**（见 §5.3 T1-b 注） | ✅ 已完成（2026-09-29） |
| **S1b** | **T1-c** `RegisterRamReset` 补全（**缺口是两个不是一个**，见 §十四 r15）；**T1-d** 解压类：`Lz77×2` / `Huffman` / `Rl×2` / `GetBiosChecksum` | **按 r11 同样重定义**：T1-c/T1-d 逐项**算法级锁测试**（含往返压缩/解压与参考向量）+ 认领集扩大到 **11** 个号码后核心其余 SWI 仍全部可达。**认领为全有全无**（r15 ②）。**⚠️ 号码已由 r25 更正：`0x10`–`0x14` → `0x11`–`0x15`（GBATEK 的 `0x10` 是 BitUnPack）**。~~jsmolka `memory.gba` 在 stub BIOS 下全绿~~ → **移至 S2** | 1–1.5 周 | **S1b ✅ 已完成**（2026-09-30）—— 认领集 11 个，达标，**号码经 r25 更正**。T1-c ✅ · T1d-a ✅ · T1d-b ✅。锁测试 27 → **89 项** |
| **S1c** | **T2**：`Sqrt` / `ArcTan`×2 / `BgAffineSet` / `ObjAffineSet` / `BitUnPack` | 逐位对齐验证；mode 7 与精灵缩放样例通过 | 1–1.5 周 | **✅ 已完成**（2026-09-30）—— `0x09`（r24）· `0x08` 验证不认领（r25 ④）· `0x10`（r26）· `0x0E`+`0x0F`（r27）· `0x0A`（r28）。**认领集 16 个号码。锁测试 89 → 132。**⚠️ 出口判据「mode 7 与精灵缩放样例通过」需帧缓冲，**只能在 S2 取得**；本阶段达成的是算法级逐位对齐。⚠️ **NES 零回归仍未取得**（L11 / L12） |
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
| **R5** | **巴士因子**。上游维护者极少 | 上游停摆需自维护 19.5K 行 → **实为 22.3K 行** | MIT 无法律障碍；R1 缓解 |
| **R6** | **水印污染帧校验** | 未来 F11QA 基线不可比 | 测试构建可关（§7.1） |
| **R7** | **音频下混有损 + 立体声丢失** | 音质弱于专用 GBA 模拟器 | `(L+R)×0.5` 避免削波；立体声列 P4 |
| **R8** | **BETA 无 M4A 增强** | 混音沿用原驱动 sample-and-hold | 已显式声明；GA 后 P1 |
| **R9** | **stub BIOS 的 IRQ 间接跳转**若实现有误，表现为随机崩溃 | 难以定位 | 与 `irq.gba` 类用例对照；stub 极小，可逐条核 |
| **R10** | **CMake `GLOB_RECURSE` 不触发重配置** | 新 crate 未被纳入构建而不报错 | 改用 `CONFIGURE_DEPENDS` 或显式列出；S0 验证 |
| **R11** | **sample-and-hold 音质**。核心以 S&H 上采样，存在镜像/混叠 | 音质弱于高阶重采样 | 已知取舍，非缺陷；如需改善须改核心或自行重采样 |
| **R12** | **分数采样实现错误**。`int` 帧长无法表达 738.35，若实现偷懒取整必长期漂移 | 音画不同步 | ABI 分离错误码与样本数（§4.1）；30 分钟无漂移为 S2 硬判据 |
| **R13** | **wait 类空壳被误当已实现**（上游 `0x02` / `0x03..=0x05` 均为 `swi_return`） | 游戏以 100% CPU 空转、耗电、行为异常 | S1a 硬性要求真实现，并设专项验证 |
| **R14** | **rustc 1.96 fat LTO 拒绝加载依赖链中第二个 archive 的 bitcode**（2026-09-28 六组对照实验，见 §十四 r8） | GBA 逻辑若做成独立 crate（`f11gba`），构建直接失败，报空诊断的 `failed to load bitcode of module …-cgu.0.rcgu.o` | **已规避并已定位**：唯一变量是「是不是独立 crate」，与代码量（2 函数 vs 381 行）、依赖关系（有无 gba-core）、泛型均无关；`lto = "thin"` 可绕过但会波及 NES 侧性能。**现行方案**：GBA 逻辑降级为根 crate 模块 `src/gba/`，经 gba-core 的 hook 字段注入（§5.2），不走 rlib 边界。**复查触发条件**：rustc 升级后重跑 §十四 r8 的六组对照 |
| **R15** | **T1-b 把补丁集推出 R1 允许的范围**。R1 的缓解写的是「补丁集最小化（仅 SWI hook）」，而 halt 机制必须落在 `step()` 里：`Bus::step` 是 `pub(crate)`（`bus.rs:1214`），根 crate 拿不到推进外设的入口，`handle_swi_hle` 也是私有的 | 补丁集 **5 → 9 处**（`halted` 字段 + 唤醒判据 hook + 两处 `Default` 初始化 + `step()` 守卫 + **`Arm7tdmi::new` 清 CPSR 的 I 位**），其中 `step()` 与 `new()` **改动 CPU 执行路径与复位状态**——不再是「不动上游逻辑」 | 已按最小实现收敛：三条执行路径（ARM/Thumb/IRQ）**只需在 `step()` 顶部一处守卫**，不必分插两处；唤醒条件经 hook 回调，语义全在我们这侧。`ATTRIBUTION.md` §3.2 同步为 9 处并写明第 6–9 处确实改变行为 |

### 9.1 已知限制编目

§十一 第 5 条要求「已知限制逐条编目」；这张表就是那张表。**GA 门禁第 3 条（§8.2）以「≤ 8 条且逐条有据」为判据**，所以新增条目必须同时写清「影响什么」与「什么时候消解」，不能只记一句现状。

| # | 限制 | 影响 | 消解时点 |
|---|---|---|---|
| **L1** | `Stop`（SWI 0x03）的唤醒条件按「任意使能中断」实现，**不查 GBATEK 的 `0x4000301` 控制寄存器** | 少数显式用 Stop 并选择唤醒条件的程序会提前退出低功耗模式；游戏极少使用 Stop | 若实测有游戏受影响，再按 `0x4000301` 精确实现（§十四 r10 记录） |
| **L2** | `GetBiosChecksum`（0x0D）返回**固定的 retail 值 `0xBAAE187F`**，而非当前实际映射的 BIOS 镜像的校验和 | **有意为之的取舍，不粉饰：我们跑的是自建 stub BIOS，却对外声称自己是 retail BIOS。** 据此分岔逻辑的程序会得到「我是 retail」的答案而实际不是。收益是游戏走正常路径（本项目的目标是让游戏跑对），且 §5.5 的真 BIOS 路径本就以该常量校验，两条路径口径一致。**未选「按 stub 实算」** —— 一个从未在野外出现过的校验和不是任何游戏能识别的信号 | 若实测有游戏依赖「非 retail」分岔，再改为实算 stub 校验和（§十四 r19 ②③） |
| **L3** | ~~T1-d 的 Huffman 解压器（0x12）尚未实现~~ → **S1d-b 已消**（2026-09-30）：认领 `0x12` 并实现。**⚠️ r25 更正：`HuffUnComp` 的真实号码是 `0x13`；`0x12` 是 `Lz77UnCompVram`。** 实现代码未改，认领号码整体后移一位（`0x11`–`0x15`）。`0x0D`（r19）、LZ77 ×2 与 RLE ×2 亦均已消。**S1b 的 11 个号码全部认领，S2 的 jsmolka 前置已清** | — | **已消**（2026-09-30，号码经 r25 更正） |
| **L4** | 未经 jsmolka 外部基准验证。S0–S1a 的全部结论来自本项目自建的锁测试 | 自测无法发现「实现与规格不符」——LZ77 头解析已实证过一次（§十四 r12 / r13） | **S2**（判据已按 §十四 r11 移至那里，届时接入帧缓冲） |
| **L5** | ~~`RegisterRamReset`（0x01）的 IWRAM 清零由**核心**实现且**主动跳过**；bits 5–7 更是一行代码都没有~~ → **S1b-c 已消**：认领 `0x01` 并补全 bits 0–4 + `0x80`，IWRAM 末 `0x200`（栈 / IRQ 向量 / BIOS flags）保留 | — | **已消**（2026-09-29） |
| **L6** | `RegisterRamReset`（0x01）的 **bits 5/6（复位 SIO 与声音寄存器）不实现** | 显式请求复位 SIO/声音寄存器的程序不复位它们。游戏极少调用；声音寄存器复位与 §六 的 **M4A 延期**绑定，GA 后 P1 一并补 | **GA 后 P1**（与 M4A 同批） |
| **L7** | **LZ77 标志字节的位序（MSB 优先）在 jsmolka 外部基准跑过之前只有自建证据。** §十四 r17 以 5 : 1 的来源对照推翻了 r13 与 S1d-a 原先的 LSB 读法；**r21 ③ 又取得第二个独立模拟器（gbajs2，MIT）的源码互证 —— 其 `lz77()` 先判 `0x80`**，即同为 MSB 优先。现状是「mGBA 一家持 LSB」对上「两个模拟器 + BIOS 反汇编 + 规格书 + 两个 Nintenlord 系工具持 MSB」。**仍无法排除 mGBA 对两种顺序都做了容错** | 若位序判断有误，真实游戏的压缩图形会整体错位 —— 不崩、不越界，只是画错，故 CI 与自建锁测试**全部照绿**，只有跑真实资产才暴露 | **S2**（jsmolka `memory.gba` 首次执行即覆盖 SWI 0x11） |
| **L8** | ~~`0x12` Huffman 的 `treesize` 偏置两说并存~~ → **已消（r22 ①）**：位流起点必须 4 字节对齐，而 `[头 4][树长 1][树 N][位流]` 的布局要求 **N 为奇数**；`(byte4+1)*2` 恒为偶数、永远不可能满足对齐，故取 `(byte4 << 1) + 1`。r17 ④ 的「订正」本身是错的 | — | **已消**（2026-09-30） |
| **L9** | **核心 `handle_swi_hle` 的 `0x08`–`0x0A` 三个 arm 现状各异，全部处置完毕**（r23 ①②④ → r24 → r25 ④ → r28 ①）。<br>· **`0x09` ArcTan：已认领**（r24），并于 r28 ① 修正取整顺序，**此前并未逐位对齐**。<br>· **`0x08` Sqrt：已验证为正确**（r25 ④），补 4 项锁测试（契约锚点 13 例、`0..=20000` 穷举 floor 语义、全部 65535 个完全平方数及两侧、GBATEK 取整例的 floor 语义）全部通过，**故确认不认领**。<br>· **`0x0A` ArcTan2：已认领**（r28）。原判「数学上大概率正确故可不认领」被推翻 —— mGBA 的实现是分象限调多项式的**整数算法、逐位对齐 BIOS**，而核心是 `f64::atan2` + π 重映射，轴向情形（`x=0` 或 `y=0`）差距明显。 | — | **已消**（2026-09-30，r28） |
| **L10** | **`BitUnPack`（0x10）的偏移溢出不做截断，按硬件行为照实实现。** 当 `源元素 + 偏移` 超出目标元素宽度时，多出的比特**溢出到同一字内的后续目标元素**（`gba` crate 描述此行为，mGBA 的实现证实它真的发生）。调用方应保证 `源宽 + 偏移 ≤ 目标宽` | 描述不自洽的输入会产生跨元素污染的输出 —— 与真 BIOS 一致，但真 BIOS 同样不保证什么。已加锁测试钉住该行为，**若将来改为截断即为行为变更**，须重新评估 | **GA 后**（若实测有工具产出此类数据） |
| **L11** | **当前环境无法执行不变式 1 的 `ctest` 判据。** 2026-09-30 实测：`build/CMakeCache.txt` 的 `CMAKE_MAKE_PROGRAM` 为 `NOTFOUND`（环境漂移，非本项目改动），重配置后全量构建在 `tests/` 下 4 个目标失败 —— MSVC **14.51** 新版标准头在既有测试代码上触发 warning-as-error（`C2220`，`__msvc_ostream.hpp` / `chrono`）。**已用 `git stash` 在干净树上复现完全相同的失败，证明与 v2.0 改动无关** | `0x10` 认领后的 NES 零回归**尚未取得证据**；不变式 1 处于未验证状态，直到该环境问题消解 | 修 `build/` 的 Ninja 定位 + 处理 MSVC 14.51 的既有告警；或换用工具链未升级的构建环境 |
| **L12** | **不变式 1（NES 核心零回归）自 2026-09-30 起处于「未验证」状态，且已连续多个阶段未取得证据。** 根因是 L11 的构建问题：4 个测试目标（`fceux11_i18n_regression_test` + 3 个 `fceux11_ppu_simd_probe_*`）走裸 `add_executable`，拿不到主工程的编译标志，而 `chrono` 等头需要 `/EHsc`；**全仓零处显式 `/EHsc`**。MSVC 升级前这不触发告警，14.51 起升为 `C4530` 并撞上 `/WX` → `C2220`。<br>**这不是 v2.0 引入的**（干净树复现，判据同上），但**风险是累积的**：S1c 已落地 `0x10`，S2/S3 尚未开工，若继续放着不管，会出现「连续三个阶段没有任何 NES 回归证据、而没人记得」的局面。**本条存在的意义是让这种静默失守可见**，防止它被当成「反正和 GBA 无关」而长期搁置。 | v2.0 的 GBA 侧改动**结构上**不可能影响 NES（不变式 8：关闭 `gba` feature 时整个 vendor 树退出编译图，两态 `cargo check` 已复核），故本项**不构成 GBA 代码的缺陷风险**，但确实**丧失了一道纵深防御** | 给那 4 个目标补 `/EHsc`（或改走 `fceux11_add_test_executable` 辅助函数）。**属 NES 侧构建配置，不在本阶段授权范围内，故未擅自修改** —— 需单独立项 |

---

## 十、不变式

1. **NES 核心零回归**：不得改变 F11QA 现有矩阵（`108P / 12F`，grade B）。`pass_to_fail` 必须为 0，出现即回滚。
   > **口径订正（2026-09-28）**：原文写 `106P / 14F`，那是 v1.16 的基线。
   > v1.18.1 修通 `kgmqa-056`、v1.18.2 修通 `kgmqa-078`，矩阵已由 106P/14F 推进到 **108P/12F**。
   > 本条是 S0 出口「NES 构建零回归」的判据，基线数字错了会把「本来就没有的 2 项 PASS」误判成回归。
2. **前端复用不打折**：不得为 GBA 另起 UI 框架或分叉 Qt 驱动。
3. **BIOS 不夹带任天堂代码**：绝不提交真 BIOS；真 BIOS 仅存于用户本地路径。
4. **许可可审计**：保留上游文件头与版权行 + 随发行物附许可全文；出处集中于 `ATTRIBUTION.md`。**目录/crate 可改名，文件头不可删。**
5. **BETA 水印在发布构建中不可关闭**：`gba_set_overlay(0)` 在发布构建返回 `GBA_ERR_STATE`；仅测试构建可关（§7.1）。
6. **HLE 与真 BIOS 双路径均须可用**，真 BIOS 须过长度与校验和双检。
7. **延期项须显式声明**：M4A、立体声、wait cycle、F11QA 接入均为**已决议的范围取舍**，不是遗漏；不得在 BETA 文案中暗示具备。
8. **GBA 侧为独立可裁剪项**：置于 cargo feature `gba` + CMake option 之后，NES-only 构建（`--no-default-features`）下 `gba-core` 整个 crate 退出依赖图、`src/gba/` 被 `#[cfg]` 剔除，**结构性保证不变式 1**。
   > **r8 说明**：本条最初设想由「独立 crate + feature」实现。降级为根 crate 模块后
   > **可裁剪性不变**——`gba-core` 仍是 `optional = true` 的独立依赖，
   > 关闭 feature 时连整个 22K 行的 vendor 树都不参与编译；
   > `src/gba/` 由 `#[cfg(feature = "gba")] pub mod gba;` 控制。
   > 与原设想唯一的差别是 GBA 逻辑代码本身不进 NES-only 构建的编译单元。

---

## 十一、状态回写

阶段完成后：

1. 本文件 §八 对应行勾选并更新出口标准
2. `ATTRIBUTION.md` 更新基线 SHA 与补丁集
3. `COPYRIGHT_AUDIT.md` 同步新增来源
4. `CHANGELOG.md` 记入 Added 段
5. 已知限制逐条编目进本文件 **§9.1**（`已知限制编目` 表；GA 门禁第 3 条以它为准）

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

审计报告：[`docs/history/reports/FCEUX11-v2.0_GBAEUX11-架构审计报告.md`](../history/reports/FCEUX11-v2.0_GBAEUX11-架构审计报告.md)（已归档）

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
| **N-08 补**（r8 自查） | 上述「扩展点已存在」隐含它**可被我们触及**。实测 `handle_swi_hle` 是**私有 `fn`**，外部无法调用或替换 | **补入 §5.2** | §5.2「扩展点接线」小节：gba-core 4 处本地修改 + 根 crate 注册链 |

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
| **r7** | 2026-09-28 | **S0 执行期回写**（§十一 第 1、2 项已执行）。实测发现与订正：① 核心规模 45 文件 **22,311 行**（原文 19,547 是 PowerShell 口径少计），R5 同步；② 基线 SHA **正是上游 `main` 当前 HEAD**，无漂移；③ `BackupType::detect` 是**私有 fn**，且实际匹配含计划漏列的 `SRAM_F_V`；④ savestate 上游**只有 derive、无任何序列化 API**，格式与版本策略须自定；⑤ 45 个源文件中 **44 个无版权头**，「保留上游文件头」改由 `LICENSE` + `ATTRIBUTION.md` 承担；⑥ **`f11gba` 独立 crate 未能落地**（新增 R14），GBA C ABI 改由根 crate 持有；⑦ 关闭 **R10**：`GLOB_RECURSE` → `CONFIGURE_DEPENDS`；⑧ 不变式 1 基线由 v1.16 的 `106P/14F` 订正为 `108P/12F`；⑨ §4.1 `gba_set_overlay` 重复声明去重 | S0 执行 + 实测 |
| **r8** | 2026-09-28 | **全面矛盾点排查与路线重定**（S1 开工前）。r7 的第 ⑥ 条只记录了「`f11gba` 未落地」这一**现象**，未定位**规律**，其缓解措施「等 S1 有了实质代码后重建」随后被实验证伪。本轮以六组对照实验定位并给出可行路线：<br>**① R14 定位**：唯一变量是「是不是独立 crate」。与代码量（2 个探针函数 vs **381 行真实 SWI 代码**）、依赖关系（依赖 gba-core vs **零依赖**）、是否含泛型**全部无关**；删空 target 后复现（排除陈旧产物）；`lto = "thin"` 可绕过但波及 NES 侧性能，不采用。已撤回 r7 期间的错误推断「模块 hash 未变 = 未重新编译」——该 hash 是固定的 LTO cache key，仅随 `Cargo.toml` 变化，重编后不变恰恰说明失败可稳定复现。<br>**② 路线**：GBA 逻辑降级为根 crate **模块** `src/gba/`，经 gba-core 新增的 `swi_hook` 字段注入；实验证实 4 个 C ABI 符号全部进入 `fceux11_rust.lib`。<br>**③ 架构矛盾**：`handle_swi_hle` 是**私有 `fn`**，r7 及之前的「向其 match 补分支」在模块化布局下不成立 → 补 §5.2「扩展点接线」（gba-core 4 处本地修改 + 注册链），并补记 §13.2 N-08。<br>**④ 工作量重估**：T1-b 的缺口不是「空壳待填」而是**机制不存在**——`Arm7tdmi` 无 halted/stopped 字段、两条 `step` 路径无 halt 分支（已核源 `:144-169` / `:705`）。S1a 由 **1 周上调为 2–3 周**，并在 §八 新增 **S0'** 阶段承载扩展点接线。<br>**⑤ 不变式 8 复核**：可裁剪性**不变**（`gba-core` 仍为 `optional` 独立依赖，关闭 feature 时整个 vendor 树退出编译图）。 | 六组对照实验 + 核源 |

| **r9** | 2026-09-29 | **S0' 出口验收**（§八 S0' 行据实回写）。四项判据全部实测通过：<br>**① `cargo check`**：`--workspace` 与 `--workspace --no-default-features` 两态均通过；后者同时复核不变式 8（关闭 feature 时整个 vendor 树退出编译图）。<br>**② staticlib 符号**：`fceux11_rust.lib` 实测含 `gba_abi_revision` / `gba_core_probe` / `gba_swi_probe` / `gba_swi_count` / `gba_probe_lz77_header`，`fceux11_rust.h:4050-4054` 同步声明。C++ 侧尚无调用点 —— S0/S0' 只到符号面，与原设计一致。<br>**③ 真实 SWI 通路**：新增 6 项锁测试。核心判据 `core_calls_our_dispatch_with_the_encoded_swi_number` 把 `SWI n` 写进真实卡带并驱动核心，断言核心带着**它编码的号码**调进 `dispatch`（实测第 2 步命中）；只验 `swi_hook.is_some()` 是不够的 —— 那在核心删掉调用点时依然为真。两次变异验证：改写记录号码 → 2 项转红；卡带回退旧值 → 转红。<br>**④ NES 零回归**：`ctest` **34/34 通过**。<br>**出口标准的一处据实改写**：原文「返回正确分派结果」在 S0' **无对象** —— `dispatch` 按设计对任何号码都返回 `false`（认领是 S1 的活）。改写为「真实 `SWI` 指令带着编码号码到达 `dispatch`」，并用 `s0_dispatch_claims_nothing` 把「S0' 不认领任何号码」这一前提锁住：认领落地那天它必然转红。<br>**顺带修掉一个 abort 级缺陷**：`gba_core_probe()` / `gba_swi_probe()` 传 `0xC0` 卡带，而 `CartridgeHeader` 要读到 `0x0E4` → `extern "C"` 帧内 panic **无法 unwind**，实测进程以 `0xc0000409` 终止。S0 的判据只查符号是否入 lib，**从未调用过这些探针**，所以一直没暴露。改用 `0x200` 零卡带并加回归测试，还原旧值即复现 abort。 | S0' 出口验收实测 |

| **r10** | 2026-09-29 | **S1a 拆分为 S1a-0 / S1a-1，并完成 S1a-0（stub BIOS）**。执行前的只读调查推翻了计划的两处判断：<br>**① S1a 有一个计划里没有的前置件：stub BIOS 从未被任何阶段拥有**。§5.2 把它列为两层之一，但 §八 阶段表里没有一行负责它，S0/S0' 也没碰，而 S1a 出口标准却写着「可启动到游戏画面」。据此拆出 **S1a-0**（stub BIOS）与 **S1a-1**（T1-b）。<br>**② 「ARM+Thumb 两条 step 路径都插判据」是高估**。两条路径在同一个 `step()` 函数内（`arm7tdmi.rs:719` / `:769`），在 `current_cycle += 1` 之后、`match initial_mode` 之前放**一处**守卫即可同时覆盖；且 `step()` 末尾无条件 `self.bus.step()`（`:827`），halt 时 `return self.bus.step()` 就能让外设继续走而流水线原样保留。<br>**③ R1 的补丁集约束到 T1-b 失效**，新增 **R15**：`Bus::step` 是 `pub(crate)`，根 crate 无法在核心外实现 halt。补丁集 5 → 8 处。<br>**④ 核源订正两处**：`0x03007FF8` 的 IntrWait flag 由**游戏自己的中断处理程序**写，不是 BIOS 写（GBATEK / mgba 一致），故 stub 不碰它；三个栈指针在 `Arm7tdmi::new`（`arm7tdmi.rs:845`）已被核心初始化成 BIOS 的值，stub 也不重复做。<br>**S1a-0 交付**：`src/gba/bios.rs` —— 16 KB 镜像由 `const fn` 逐字写入（不引入交叉汇编器），含复位向量、`WAITCNT=0x4317`、卡带头入口跳转、IRQ 间接跳转（并对空指针做保护，跳过调用而不是跳回复位）、其余向量一律 `B .` 自旋。6 项锁测试：镜像只含向量与两段代码、复位向量落点、未处理向量的行为、**合成卡带真的拿到控制权**、等待状态寄存器被写入、stub 不实现 SWI。实测全绿。<br>**已知限制（记入本表，S1a-1 实施时按此口径）**：`Stop`（SWI 0x03）的唤醒条件本轮按「任意使能中断」实现，不查 GBATEK 的 `0x4000301` 控制寄存器 —— 游戏极少使用 Stop，精确实现的收益/成本比差。 | S1a 只读调查 + S1a-0 实施 |

| **r11** | 2026-09-29 | **S1a-1 部分完成，并记下一条计划自身的结构缺陷。**<br>**① 出口标准里两条判据在 S1a-1 无法成立，不是工作没做完。** 「jsmolka `arm`/`thumb` 通过」与「可启动到游戏画面」都要求**看到画面**：`jsmolka/gba-tests` 的判定信号是**屏幕上 mode 4 文字**（全过或显示第一个失败的编号），而按 §八 S2，本项目**到 S2 才有 240×160 渲染与帧缓冲出口**。S1a-1 没有任何读结果的通路 —— 与 §5.3 判据重定义时处理 `bios.gba` 是同一类问题，只是这次忘了同样处理。另核实：该仓库是公开 MIT，但**给的是汇编源码，ROM 需 FASMARM 自行汇编**，本项目无此工具链。<br>**② 已达成**：wait 类专项验证（6 项锁测试 + 变异验证）、T1-a 回归（Div 有符号/CpuFastSet/其余号码仍可达）、**stub BIOS 的 IRQ 全链路**（启动 → 异常 → 向量压栈 → 经 `0x03007FFC` 间接跳转 → 处理程序 `bx lr` → 弹栈 → `subs pc,lr,#4` 回到被打断处）。第三条此前完全没有测试覆盖，而 S1b 的 jsmolka 门禁会立刻踩到它。<br>**③ 实施中发现的第二个上游缺陷（已修）**：ARM 复位时 CPSR 的 I 位为 1，`gba-core` 内再无别处清它 —— **置 1 的 CPU 永远取不到中断**，任何游戏都会卡死在 `Halt` 里，且 wait 家族的测试无一能过。已在 `Arm7tdmi::new` 清该位（与该处已在做的 BIOS 栈预置同属一类：核心预置 BIOS 的复位状态）。补丁集因此由 r10 预估的 8 处变为 **9 处**，R15 措辞同步订正。<br>**④ 交付**：`cargo check` 两态、全量构建、**NES 零回归 ctest 34/34**、锁测试 6 → **24** 项全绿。<br>**⑤ 裁决（2026-09-29，用户定 (a)）**：jsmolka 门禁**整体移至 S2**。S1a-1 与 S1b 的出口判据改为**算法级锁测试 + 启动/IRQ 链路端到端**；S2 接入帧缓冲后，jsmolka `arm`/`thumb`/`memory.gba` 作为**外部门禁**首次执行——那时它才第一次真正测到 S1 与 S1a 的成果。选 (a) 而非 (b)（先做 VRAM mode 4 文字读取器把门禁前移，约 1 天）的理由：那 1 天要自己解 4bpp tile 与字体表，而**同一份 VRAM 读取能力在 S2 本来就要写**，提前做等于把同一份工作做两遍。 | S1a-1 出口复核 + 用户裁决 |

| **r12** | 2026-09-29 | **S1b 开工第一步：修掉 S0' 留下的 LZ77 头解析缺陷；T1-d 的块格式暂不实施。**<br>**① 缺陷**：`parse_lz77_header` 把输出长度读成 `raw & 0x0FFF_FFFF` —— 把**压缩长度**的低 24 位并进了输出长度。真实头部的压缩长度永远非零（压缩数据总比 4 字节头长），所以返回值对任何真实文件都是垃圾。旧测试用的 `0x8000_0100` 低 24 位为零，**恰是唯一不可能出现的取值**，错误因此被测试盖章通过 —— 与 S0' 那次「只查符号在不在库里」同类：断言写在实现自己的假设上。已改为 `(raw >> 24) & 0x7F`，并加 4 项锁测试（两字段不重叠、bit 30 属于长度、损坏头被拒、字节单位往返）。判定不依赖外部规格：**一个 24 位字段不可能同时是两个长度**。<br>**② 顺带记一次自己的失误**：新测试第一版用手写常量，`0x20 << 24 | 1 << 31` 并非 `0x8020_0000`，且压缩长度一旦带 bit 24 就会污染长度字段 —— 两次都测出来了。测试改为**按字段拼装头部**，并对越界字段 assert，这类手算失误从此不可能进测试。<br>**③ T1-d 的块格式暂不实施**：flag 字节的位序、token 布局、Huffman 树结构都需要位级权威规格。GBATEK 的 BIOS Functions 整节在当前环境取不到（全文过长被工具截断，problemkaputt 直连 403），检索到的两份材料还存在关键分歧（GBA BIOS 的 LZ77 是 **LSB 优先**扫 flag 字节，而 NDS/Wii 变体是 MSB 优先）。按「不凭记忆写位级格式」的纪律，**宁可不写**。取得权威章节或一份真实压缩资产后再实施，并与 S2 的 jsmolka 门禁合流验证。<br>**④ T1-c 可独立推进**：`RegisterRamReset` 的 IWRAM 清零是纯内存语义，可直接测。 | S1b 开工 |

| **r13** | 2026-09-29 | **规格取证完成，并撤回 r12 之后的那个"修复"——它本身是错的。**<br>**① 取证源**：mGBA 的 HLE BIOS（`src/gba/bios.c`，`_unLz77` / `_unHuffman` / `_unRl` / `_RegisterRamReset`），该实现经真实硬件验证。**只作为格式参考阅读，不复制代码**（MPL-2.0 与本项目许可不相容），实现仍为本项目一手 Rust。<br>**② GBA 与 NDS/Wii 的压缩头不兼容，而两者常被并列描述，极易混淆**：GBA 是「字节 0 = 签名 `0x10`/`0x20`/`0x30`，字节 1-3 = 24 位解压长度」；NDS/Wii 是「bit 0-23 = 压缩长度，bit 24-30 = 长度，bit 31 = 是否按 8 字节单位」。**本文件先后错成两次**：S0' 读成 `raw & 0x0FFFFFFF`（把签名并进去了），S1b 的"修复"读成 `(raw >> 24) & 0x7F`（那是 NDS 布局）。正确读法是 `raw >> 8`。两次的错误都被**自建测试**盖章通过，因为两次的测试向量都用了低 24 位为零的头——真实文件不可能出现。现已改正，并加一项测试专门演示 NDS 头按 GBA 读会**静默**得到大 0xA0000 倍却仍通过合理性上限。<br>**③ 已取到的完整格式（供 S1b 实现）**：<br>· **LZ77**：标志字节后 8 个块，**低位在前**；块 = 2 字节大端，`disp = dest - (block & 0xFFF) - 1`、`len = (block >> 12) + 3`。
> ⚠️ **本条已被 §十四 r17 推翻**：GBA 的 LZ77 是**高位在前**（MSB first），GBA 与 NDS 在此**并无差异**；r13 采信的 mGBA 是唯一持相反读法的来源。实施时以 r17 为准。<br>· **RLE**：块首字节 bit7 置位 = 重复 `((b & 0x7F) + 3)` 次的 1 字节；清零 = 紧随其后的 `((b & 0x7F) + 1)` 个字面字节；末尾补齐到 4 字节。<br>· **Huffman**：头低半字节 = 每元素位数，高 3 字节 = 长度；`treesize = (byte4 << 1) + 1`，树基址 = src+5，位流从 src+5+treesize 起按 **32 位字、高位在前**；节点字节 bit0-5 = 偏移、bit6 = 右子为叶、bit7 = 左子为叶，子节点位于 `(nodeAddr & ~1) + offset*2 + 2`（右子 +1）；按位数聚成 32 位后整字写出。
> ⚠️ **`treesize` 已被 §十四 r17 ④ 订正为 `(byte4 << 1) + 2`**（GBATEK 的定义是「树表长度/2 减 1」）。r13 的式子在 `byte4 = 0` 时给出 1 字节的树，而最小合法树为 2 字节，自相矛盾。**S1d-b 开工前须按 r13 + r17 重新取证。**<br>· **`GetBiosChecksum` (0x0D)**：r0 = BIOS 校验和常量、r1 = 1、r3 = `0x4000`。stub BIOS 无真校验和可算，取常量并记为已知限制。<br>· **`RegisterRamReset` (0x01)** 位定义（`0x02` 位 = 清 IWRAM **但保留末 0x200 字节** —— 栈、IRQ 指针、IntrWait flag 都在那里）；`0x80` 位含 IE=0 / IF=0xFFFF / WAITCNT=0 / IME=0。<br>**④ 这条记录本身是 r11 那条结论的验证**：S1b 卡住不是因为它工作量更大，而是因为**当时手上没有可核的材料**。取证只花了半小时，而它立刻推翻了我自己的一个错误结论。 | S1b 规格取证 |

| **r14** | 2026-09-29 | **补正 S1a-0 / S1a-1 遗漏的 §十一 状态回写。** §十一 列了 5 项回写目标，S0 / S0' 全部执行，**S1a 两个阶段只完成了第 1 项（§八 阶段表）与第 5 项（§9.1 已知限制）**，第 2 / 3 / 4 项漏执行：<br>① **`ATTRIBUTION.md` §3.2 标题与 §4 尾段停在 S0' 口径** —— 正文 §3.2 与 §4 表格已是 9 处，标题仍写「5 local changes」，尾段仍写「Five additive edits… none of which change upstream behaviour」，与同文件第 6–9 条「`step()` gains a branch」**自相矛盾**；一份归属文件在同一页上讲两种补丁集口径，比没有更糟。<br>② **`COPYRIGHT_AUDIT.md` §5 表格行**仍写「local patch set (5 SWI-hook edits)」。<br>③ **`CHANGELOG.md` [Unreleased]** 只有 S0 / S0' 一条，S1a-0（stub BIOS）与 S1a-1（halt 机制）**完全缺失**。<br>**成因与 r12 同型**：r11 复核的是出口判据（锁测试全绿 / ctest 34/34 / 两态 check），**没有逐项对照 §十一 的回写清单** —— 核了交付面，漏了登记面。登记面不是次要项：补丁集处数是 R1 / R15 的审计对象，CHANGELOG 是发布文案的来源，两者都会被人直接引用。<br>**纪律补强（本次新口径）**：阶段收尾**按 §十一 逐项点名核对**，不得以「出口判据全绿」推定「回写已完成」—— 出口判据与回写清单是两组互不覆盖的检查。 | 阶段完成度盘点（复跑锁测试 27 项、`ctest` 34/34） |

| **r15** | 2026-09-29 | **S1b 开工：只读调查推翻 L5 的缺口清单，认领语义比假设的更硬。**<br>**① L5 少记了一个缺口。** L5 说 `RegisterRamReset` 的缺口是「IWRAM 清零被主动跳过」（核源属实，`arm7tdmi.rs:1187-1192` 带 TODO）。但**bits 5–7 根本没有代码**（`:1215-1218` 仅三行 `not fully implemented` 注释），r13 只给了 bit `0x80` 的内容（IE=0 / IF=0xFFFF / WAITCNT=0 / IME=0），bits 5/6 无位级规格。**用户裁决（2026-09-29）**：实现 bits 0–4 + `0x80`，bits 5/6 记为 **L6**，理由是声音寄存器复位与 §六 的 M4A 延期绑定，GA 前无游戏依赖 —— 宁可留缺口，不凭记忆写硬件行为（纪律 4）。<br>**② 认领是全有全无的（承重事实，此前未写进计划）。** 核源 `arm7tdmi.rs:1112-1120`：hook 返回 `true` 即 `return true`，核心自己的 `match swi_num` **一行都不执行**。故认领 `0x01` 必须连 bits 0/2/3/4 一并重实现；**只补 IWRAM 会让已经能用的四个位倒退**。T1-d 的 `0x10`/`0x11` 同理。<br>**③ 补丁集不增加，仍 9 处。** `Arm7tdmi::bus` 是 `pub`，`Bus` 的 `read_byte`/`write_byte`/`read_word`/`write_word`/`read_half_word`/`write_half_word` 全为 `pub`（`bus.rs:1128/1136/1436/1470/1497/1534`）—— T1-c / T1-d 所需内存访问现有接口全够，**不碰 vendor 树**。<br>**④ 出口判据的「认领集扩大到 8 个号码」订正为 11。** 逐个编号：T1-c 1 个（`0x01`）+ T1-d 6 个（`0x0D`、`0x10`–`0x14`）= 7 个新号，加已认领的 4 个 wait = **11**。原文的 8 与 §5.3 的分档对不上。<br>**⑤ `decompress.rs:65-67` 有一条会在 T1-d 失效的注释。** 该处称 `0x10`/`0x11` 的孔径差异「enforced on the core side」；认领 `0x11` 后核心 arm 不执行，**没有任何东西**强制 VRAM 半字对齐，该责任落到我们这侧。T1-d 开工时必须同步改掉 —— 这是本文件继 r12 之后的第二处「假设写在注释里而无人执行」。<br>**⑥ 两条锁测试是预期转红，不是缺陷**：`dispatch_claims_exactly_the_wait_family`（`mod.rs:385`）与 `t1a_dispatch_leaves_the_other_core_arms_reachable`（`:778`，断言 `RegisterRamReset` 必须穿透）—— 两条当初就写成「落地那天必然转红」，S1b 认领时改写。 | S1b 只读调查 + 用户裁决 |

| **r16** | 2026-09-30 | **T1d-a（LZ77 ×2 / RL ×2）开工：r13 留白的两处由核源定案。**<br>**① 「VRAM 变体的孔径由核心强制」是错的，而且错的方向危险。** r15 ⑤ 记的是「认领后没人强制半字对齐」；核源 `bus.rs:1141-1186` 后发现更强的约束：**`Bus::write_byte` 对 VRAM 不是普通字节写** —— BG VRAM 把该字节复制到整个半字，OBJ VRAM **直接丢弃**。真实 BIOS 的 `Lz77UnCompVram` / `RlUnCompVram` 走半字写，所以按字节解压到 VRAM 会得到**两处都错**的结果（BG 半字内字节重复、OBJ 区整段丢失）。故 `0x11` / `0x14` **必须**经半字汇流后写，`0x10` / `0x13` 走普通字节写。解压器因此必须写成「源/汇皆为 trait」的泛型形态：同算法两个变体只在汇的实现上分叉。<br>**② 损坏数据的处置策略（r13 未定，本轮定案）**：签名不符 / 零长度 / 超上限 → 返回 `Err` 且**一个字节都不写**；回溯距离越过已写起点 → `Err`；解压长度超过目标区可写容量 → `Err`。宁可确定性地不动内存，也不让坏流写坏 VRAM。`Err` 由分派层转成「打印 trace 后直接返回」，不陷入 BIOS 的 0x08 空向量。<br>**③ 未取到权威材料的一项**：Ghidra 侧无第二来源交叉验证，本轮格式以 r13 记录的 mGBA HLE 为准。锁测试用**测试内现写的压缩器**做往返（纪律 5），并另设一条**专门钉住 LSB 优先**的用例 —— LSB/MSB 弄反的产物是「能解压、能往返、只是块序不同」，普通往返测试抓不到。 | T1d-a 只读调查 + 核源 |

| **r17** | 2026-09-30 | **GBATEK 交叉核查推翻 r13 与 S1d-a：GBA 的 LZ77 是 MSB 优先，不是 LSB。本轮已实现并已推送的位序是错的。**<br>**① 这是本项目第三次在同一处栽跟头，且与前两次同型：外部参考读错。r13 把 mGBA 的 HLE 当成「经真实硬件验证」的定论，据此写下「GBA 的 LZ77 是 **LSB 优先**扫 flag 字节，而 NDS/Wii 是 MSB 优先」——**这个 GBA/NDS 的对比本身不存在**，两者相同。S1d-a 据此实现，**并单设了一条测试把它「钉死」在 LSB 上**，等于把错误焊得更牢。<br>**② 裁决证据（5 : 1）**：`problemkaputt.de/gbatek-bios-decompression-functions.htm` 原文「Type Flags for next 8 Blocks, **MSB first**」；**GBA BIOS 反汇编**（`0x000010FC`：`ldrb lr,[r0],#1` → `lsl lr,lr,#24` → 循环内 `lsls lr,lr,#1` / `bcs`）——`lsl #24` 把字节的 bit0 放到 bit24，**第一次进 carry 的是 bit31 即原 bit7**，故 MSB 优先，此为最硬证据；`gba` crate（docs.rs 0.15）转述 GBATEK 亦为 MSB；gba_explorer 与 JAAE 两个 **Nintenlord 系**实现（GBA 圈事实标准的压缩器出处）均先测 `0x80` / 用 `(byte >> 7-i) & 1`。**仅 mGBA 一家持 LSB**，故推翻 mGBA。<br>**③ 为什么自建测试抓不到**：纯字面量流在两种读法下输出**完全相同**，所以往返测试天然对位序不敏感。上一轮为此单设的两条测试只是把错误方向锁得更死 —— **给错误结论配一条「专项锁测试」，比没有测试更危险**，它会阻止后续的外部证据纠正它。这是本条最该记住的一条。<br>**④ 一并纠正的 Huffman 取证错误**：r13 记 `treesize = (byte4 << 1) + 1`，GBATEK 的定义是 `tree_table/2 - 1`（即该字节存的是「树表长度/2 减 1」），故 **`treesize = (byte4 << 1) + 2`**。r13 的式子在 `byte4 = 0` 时给出 1 字节的树，而最小合法树是 2 字节 —— 一个自相矛盾的下界，正是错位的征兆。**S1d-b 实施前必须按 r13 + 本条重新取证，不得直接照 r13 写。**<br>**⑤ GBATEK 的一条 CAUTION（本实现是它的安全超集）**：「写 16 位到 `[dest-1]` 而非 8 位到 `[dest]`，意味着读 `[dest-1]` 不成立，即 **Vram 版本只在 `disp=001h..FFFh` 下正确，`disp=000h` 不正确**」。我们的回溯走 4 KB 窗口而非回读目标内存，故 `disp=000` 照样正确 —— 比真 BIOS 宽松，**不是缺陷**；但这解释了为何面向 VRAM 的压缩器不会产出 `disp=000` 的串。<br>**⑥ 处置**：S1d-a 的位序已按 MSB 修正，`lz77_body_with_reference` 的两条方向测试已重写（仍从两端钉，但钉的是 MSB）。<br>**⑦ 残留风险（据实记录）**：本次推翻的唯一对手方 mGBA 是被大量真实游戏验证过的模拟器，它在这点上若真错了会很显眼。三条可能的解释——(a) mGBA 的 HLE 此处确有缺陷；(b) r13 读 mGBA 代码时读反了（r13 本身就因读反 GBA/NDS 头布局被推翻过两次）；(c) mGBA 对两种顺序都做了容错。**本轮无法排除 (c)**，故 §9.1 新增 **L7**：该位序在首次外部基准（jsmolka `memory.gba`，S2）之前只有自建证据。 | GBATEK 交叉核查 + 五源对照 |

| **r18** | 2026-09-30 | **T1d-b（Huffman）取证：五项已多源确认，两项只有 GBATEK 单一来源 —— 据纪律 4 停下，不实施。**<br>**① 已确认（多源一致：GBATEK 原文 + `gba` crate 逐条转述 + CowBite 模拟器规范 + gbajs2 + deepwiki）：<br>· 头 4 字节：bit0-3 = 每元素位数（常 4 或 8）、**bit4-7 必须为 2**（Huffman 类型）、bit8-31 = 24 位解压长度；<br>· 第 5 字节 = **树表长度/2 − 1**（⇒ `treesize = (byte4+1)*2`，**证实 r17 ④ 的订正**，r13 的 `+1` 确为错）；树表自 src+5 起，位流自 `src+5+treesize` 起；<br>· 节点字节 bit0-5 = 子节点偏移（**2 字节为单位**）、bit7 = node0/左 为叶、bit6 = node1/右 为叶；子节点地址 = `(nodeAddr & ~1) + offset*2 + 2`，右子 +1；置了叶标志的那一格存的是**数据本身**（元素 < 8 位时高位须为 0）；<br>· 位流按 **32 位字**存放，**bit31 = 第一个比特**，0 = node0、1 = node1；到叶后写数据并复位回根。<br>**② 未确认（只有 GBATEK 一家，且无任何来源提出异议）—— 这两项不写代码**：<br>· **(a) 头部长度的单位是「字节」还是「元素个数」。** GBATEK 原文「24bit size of decompressed data **in bytes**」，`gba` crate 与 CowBite 均转述为 bytes。**但这三家是同一条传承**，等于只有一个独立信源。歧义只在元素 < 8 位时暴露（4 位元素下两者差一倍），而 4 位正是 GBA 图形数据的主要用法。<br>· **(b) 元素装进 32 位字时的顺序。** GBATEK 只说「Data is written in units of 32bits」，**没有说**先解出的元素落在字的高位还是低位。这**正是一个位序问题**，而 r17 刚刚证明 LZ77 的位序可以与共识相反。<br>**③ 为什么不「无异议即照做」**：r17 的教训是「多数一致」也会错 —— 那里 mGBA 是唯一异议者却是对的一边；这里虽无异议，但**独立信源数实际为 1**。在刚刚因位序栽过跟头之后，用一个 MSB/LSB 存疑的假设去写三个算法里最复杂的一个，是把 r17 的错误模式原样再执行一次。<br>**④ 消解路径（三选一，都比猜便宜）**：(1) 找一份**真实的 GBA Huffman 资产**（例如某个公开的压缩 tile 数据），跑真机或对照工具验证；(2) 读一份**生产方**的实现（如 NLZ-Advance 的 Huffman 模式、或任一生成 GBA Huffman 的工具）看它怎么写长度与打包顺序；(3) 等 S2 的 jsmolka `memory.gba`。**在 (1) 或 (2) 落地前，`0x12` 保持未认领**，相应 SWI 落进 stub 的 `0x08` 向量自旋（已记入 **L3**）。 | Huffman 多源取证 |

| **r19** | 2026-09-30 | **T1d-b 的 `0x0D` `GetBiosChecksum`：取值口径裁决 + r13 契约订正。**<br>**① GBATEK 交叉核查再次推翻 r13 的契约。** GBATEK 原文（`gbatek-bios-misc-functions.htm`）：「**Parameters: None. Return: r0=Checksum.**」，校验和为 **`BAAE187Fh`**（GBA 与 GBA SP），NDS/3DS 以 GBA 模式运行时为 `BAAE1880h`（仅 `[3F0Ch]` 由 `00h` 变 `01h`）。r13 记的「r0 = 常量、**r1 = 1、r3 = 0x4000**」来自 mGBA，GBATEK 明确说这两个寄存器不在契约内 —— 按 GBATEK 的总则「Outgoing registers R0,R1,R3 are typically containing either garbage, or return value(s)」，本实现**只写 r0，不动 r1/r3**。<br>**② 返回值口径（用户裁决 2026-09-30）：返回 `0xBAAE187F`，即真 BIOS 的校验和。** 理由与 L2 原口径相反，原口径的理由是「返回非 retail 值对 flashcart 语义反而是想要的」—— 那是把 flashcart 的需求当成了本项目的目标。本项目的目标是**让游戏跑对**，绝大多数游戏走的是 retail 分支；且 §5.5 的真 BIOS 路径本来就以该常量做校验和校验，返回它才自洽。<br>**③ 由此产生的诚实性代价（记入 L2，不粉饰）**：**我们跑的是自建 stub BIOS，却对外声称自己是 retail BIOS。** 一个据校验和分岔逻辑的程序会得到「我是 retail」的答案而实际不是。这是一次**有意的兼容性与诚实性的取舍**，不是缺陷也不是疏忽：收益是游戏能走正常路径，代价是这类程序拿不到「我在模拟器上」的信号。**未选「按 stub 实算校验和」** —— 那需要重新遍历 16 KB stub 镜像，且返回的是一个从未在野外出现过的值，游戏无从识别。 | `0x0D` 取值裁决 + 契约交叉核对 |

| **r20** | 2026-09-30 | **r18 ④ 的三条消解路径全部尝试过，本环境内全部受阻 —— `0x12` Huffman 继续暂停，且它是 S2 的阻塞项。**<br>**① 路径 (2)「读 BIOS 反汇编」（本应最硬，LZ77 就是这么定的）：找到了 `camthesaxman/gba_bios`，其 SWI 分派表确证 **`HuffUnComp` 在 BIOS 地址 `0x00001014`**（`asm/bios.s`，`ldr r12,[r11,r12,lsl #2]` 取表后 `bx r12`）。**但取不到正文** —— `raw.githubusercontent.com` 连接失败，`deepwiki.com` 挂在 Vercel 浏览器校验页。<br>**② 路径 (1)「真实资产」**：本仓库无任何 `.gba` / Huffman 资产（`tests/fixtures/` 下只有 NES 的 mapper 金标与 blargg/f11qa 夹具），本机也无 jsmolka 套件。<br>**③ 路径 (2) 的另一个候选 —— 生产方实现**：查下来 **NL's Compressor 4.2（Nintenlord，GBA 圈事实标准压缩器）只支持两种压缩**，即 LZ77 与 RLE，**并不产出 Huffman**。这是本轮最有价值的一条负面结论：它把 Huffman 的实际使用面收窄了 —— 常见工具链根本不生成它。仍会用它的只有 John Sensebe 的 **GBA Crusher**（其资料称支持全部 GBA BIOS 压缩格式），但那是老 C++ 工具，本轮未能取到其 Huffman 写法。<br>**④ 由此产生的一个必须记下的后果：`0x12` 阻塞 S2，不只是阻塞 S1b。** jsmolka `memory.gba` 专项测 SWI `0x0B`/`0x0C`/`0x11`–`0x18`，**含 `0x12`**；而未认领的号码在 stub BIOS 下会落到 `0x08` 向量自旋（**确定性挂住**，见 L3）。所以 r18 ④ 的路径 (3)「等 S2 的 jsmolka」**不能单独成立** —— 必须先解掉歧义、或让 `0x12` 在 S2 之前落地，否则 S2 的外部门禁必然红。**这是 r18 当时没有预见到的依赖倒置。**<br>**⑤ 当前可行的两条出路（待用户裁决）**：(A) 由用户侧提供材料 —— problemkaputt.de 对本工具的 UA 返回 403，用户浏览器可正常打开；或提供 GBA BIOS 反汇编正文、或任一含 Huffman 块的 GBA 资产。这是**唯一能在本轮消解歧义的路径**。(B) 接受风险，按 GBATEK 的两项（长度=字节、整字写出）实施，并把 **L7 扩展到 `0x12`**；依据是两项均**无任何来源提出异议**，只是缺独立佐证（r18 ②）。<br>**⑥ 本轮新增的、已可确认的事实**（不依赖上述阻塞项）：GBATEK 完整 Huffman 章节已取到并逐条核对；`treesize = (byte4+1)*2` 获**独立第二来源**（CowBite 独立模拟器规范）佐证，不再是 r18 里的单源项。 | r18 ④ 三路径逐一尝试 |

| **r21** | 2026-09-30 | **`0x12` 的三项歧义解开两项 —— 取到 gbajs2（`andychase/gbajs2`，**MIT**，能跑真实游戏的工作实现）源码 `js/irq.js`，逐行读其 `huffman()`。与 mGBA 同样只作格式参考阅读，不复制代码。**<br>**① r18 ②(a)「长度单位」= **字节，已定案。** 实现把头部 24 位读作 `remaining`，且**每写出一个 32 位字就 `remaining -= 4`** —— 计数器是字节，每字减 4，语义上不可能是「元素个数」。另见它 `remaining &= 0xfffffffc` 后单独处理零头，尾部不足一整字的部分**仍以整字写出**（高位补零），这与我们 LZ77 `Vram` 变体「丢掉奇数尾字节」的处理**不同**，是格式差异不是矛盾。<br>**② r18 ②(b)「元素装入 32 位字的顺序」= **从低位往上填**，已定案。** 实现是 `block |= (符号值 & ((1<<bits)-1)) << bitsSeen` 且 `bitsSeen` 从 0 递增，即**第一个解出的符号落在字的最低位**，随后向上累满 32 位整字写出。GBATEK 只写「written in units of 32bits」确实没说清这一点。<br>**③ 顺带独立证实 r17：LZ77 是 MSB 优先。** gbajs2 的 `lz77()` 先判 `blockheader & 0x80` 再 `blockheader <<= 1`，即**先测 bit 7**。这是**第二个独立模拟器**（作者与 mGBA 不同）与 GBATEK、BIOS 反汇编站在一边。**L7 的残留风险显著下降**：现在是「mGBA 一家持 LSB」对上「两个模拟器 + BIOS 反汇编 + 规格书 + 两个 Nintenlord 系工具持 MSB」。<br>**④ 仍然存疑，且变成有据的分歧（不得再当作「GBATEK 单源」处理）**：`treesize` 的偏置。**GBATEK 的定义（`tree_table/2 - 1`）蕴含 `treesize = (byte4+1)*2`，而 gbajs2 用 `(byte4 << 1) + 1`** —— 与 r13（采信 mGBA）相同、与 r17 ④ 的订正**相反**。两条推导都有来源，且**树表按成对节点寻址（子节点地址是偶数偏移）指向偶数长度**，而 gbajs2 的式子恒为奇数。两说必有一错，**本轮不裁决**，按「(byte4<<1)+1 / +2 两说并存」记入 L3，**实施时以真 BIOS 反汇编（`0x00001014`）为准**。<br>**⑤ 另两条实现约束（GBATEK 未提）**：`bits` 必须整除 32（否则 gbajs2 直接抛「unaligned Huffman」，即合法宽度实为 1/2/4/8/16/32）；`dest` 被对齐到 4 字节边界。<br>**⑥ 子节点寻址获第二源互证**：GBATEK 的 `child0 = (CurrentAddr AND NOT 1) + offset*2 + 2` 与 gbajs2 的 `((parentIndex-1)|1) + stored*2 + 2` 在索引空间下**完全等价**（树表起于 `source+5`，而 `source` 4 字节对齐，故树首地址 ≡1 mod 4，`& ~1` 正好对应索引 −1）—— r18 ① 的该项**确认无误**。 | `0x12` 取证：gbajs2 源码 |

| **r22** | 2026-09-30 | **`0x12` 的最后一项歧义由内部一致性定案，并且结论是：r17 ④ 那次「订正」本身是错的。**<br>**① 位对齐论证。** GBATEK 要求源地址 4 字节对齐，且位流「stored in units of 32bits」、由 BIOS 以 32 位字读出。布局是 `[头 4][树长 1][树 N][位流]`，故位流起点 = `source + 5 + N`，**必须 ≡ 0 (mod 4)**，即 **N 必须是奇数**。<br>· gbajs2 / mGBA 的 `N = (byte4 << 1) + 1` —— 恒为奇数，**可能**满足（当 byte4 为奇数时）。<br>· r17 ④ 依 GBATEK 措辞推出的 `N = (byte4+1)*2` —— **恒为偶数，永远不可能满足对齐**。<br>一个让对齐**恒不可能**的公式不可能是对的。故 **`treesize = (byte4 << 1) + 1`**，与 r13 原始记录一致。<br>**② 这意味着 r17 ④ 改错了。** 当时我读到 GBATEK 的「Size of Tree Table/2-1」就断定 `+1` 是错的，没有验证这个式子能否产生一个 4 字节对齐的位流。**r13 当初的取值是对的，错的是我上一轮的「修正」。** 这是本项目第四次栽在「凭规格书的措辞推论、没有验证内部一致性」上，与 r12 / r13 / r17 同型。<br>**③ 顺带一条**：树表长度恒为奇数，与「树 = 1 字节根 + 偶数个成对节点」的结构一致 —— 子节点地址是 `(addr & ~1) + off*2 + 2`，必为偶数偏移，故根之后的节点成对出现；我此前用「树表应为偶数长度」去质疑 gbajs2，那个论证本身就不成立。<br>**④ L8 消号。** `0x12` 的三项歧义（长度单位 / 元素打包顺序 / `treesize` 偏置）至此**全部定案**，且每项都有至少两个独立来源。`0x12` 解除阻塞，可实施。 | `treesize` 偏置由位对齐定案 |

| **r23** | 2026-09-30 | **S1c 开工前调查：核源后推翻了 §5.3 对 T2「上游现状」的记载，并发现核心里有一处是全项目最严重的错误。**<br>**① 核心的 `handle_swi_hle` 实际覆盖 `0x00`–`0x0C`，故 T2 六个号码里有**三个（`0x08`/`0x09`/`0x0A`）核心已经实现**，而 §5.3 的 T2 行记的是「❌ 未 HLE，回落真 BIOS」——**该记载对这三个是错的**。`0x0E`/`0x0F`/`0x10` 确实不在核心范围内，是真正的新写。<br>**② `0x09` ArcTan 是全项目当前最严重的错误。** 核源 `arm7tdmi.rs`（S1a-1 加了 9 处后位移，行号以 grep 为准）：`let result = (tan / 2) as u32;` —— **把输入减半当作反正切**。它只在输入恰好为 `0x4000`（1.0）时对上正确值 `0x2000`，其余全错；且 `tan` 为负时经 `as u32` 变成巨大的无符号数。**该 arm 零测试覆盖**（核心 `mod tests` 中无任何 sqrt/arctan/affine/swihle 命名的用例）。**S1a-1 的 T1-a 回归只覆盖 `CpuSet`/`CpuFastSet`/`Div`/`DivArm`/`SoftReset`，不含这三个**，所以一直没被发现。<br>**③ 但 `0x08` Sqrt 核心其实是对的 —— 这一项推翻了我自己动手时的初判。** 我先前据「BIOS 契约是 16.16 定点」判它实现成了整数开方、属于错误函数；GBATEK 原文却是「Return: r0 **unsigned 16bit number**，The result is an integer value, so Sqrt(2) would return 1, to avoid this inaccuracy, **shift left incoming number by 2\*N**」——**结果本来就是整数**，输入左移才拿到小数精度。核心的整数牛顿迭代与该契约一致，且任何 u32 输入的 floor(sqrt) ≤ 65535 必然落在 16 位内，无需额外截断。**结论：`0x08` 不必认领**，只需补测试确认，认领它反而是无谓的补丁风险。<br>**④ `0x0A` ArcTan2 数学上大概率正确**（f64 `atan2` + π 重映射，输出 `0000h`-`FFFFh` 符合 GBATEK；`atan2` 对缩放不变，故把 1.14 当裸整数读不影响角度），但**不逐位对齐** BIOS 的整数算法，且 `x=y=0` 等边界未定义。<br>**⑤ 新写的三个（`0x0E`/`0x0F`/`0x10`）规格已取全**，其中一条**位级细节极易做错**：GBATEK 明确「旋转角指定为 `0-FFFFh`，但 **GBA BIOS 只取高 8 位**，低 8 位可为小数部分、**被 BIOS 忽略**」。用 `Math.cos` 之类浮点全精度实现必然不逐位对齐（可参照 gbajs2 的 0x0E/0x0F 写法，其 `theta = ((loadU16 >> 8) / 128) * PI` 倒是体现了「只取高 8 位」，但仍是浮点）。`0x10` BitUnPack 的 15 位 Data Offset + bit31 Zero Data Flag、源/目的位宽约束亦已取全。<br>**⑥ 由此得到 S1c 的执行顺序判据**：认领 `0x09` 是**最高优先**（现状是静默错误，比未认领的挂死更糟），`0x0E`/`0x0F`/`0x10` 次之，**`0x08` 不认领**，`0x0A` 视是否要求逐位对齐而定。**已新增 L9** 记录核心这三个 arm 的现状。 | S1c 开工前：核心实况 + T2 规格取证 |

| **r24** | 2026-09-30 | **S1c-a：`0x09` ArcTan 认领并实现。R4「定点逐位对齐困难」这条风险以**契约作为裁决依据**化解，而非靠来源投票。**<br>**① 算法与系数：两个独立来源给出**完全相同的 8 个系数**（`0x00A9` `0x0390` `0x091C` `0x0FB6` `0x16AA` `0x2081` `0x3651` `0xA2F9`）——endrift（Elk 作者，称其逆向自 BIOS）与 Jonas Wagner（coranac.com / libgba 作者，有专文分析并给出系数表与有理近似对照）。**系数本身无争议。**<br>**② 定点移位两来源冲突（14 vs 15），用契约裁决。** GBATEK 给出输出范围 `C000h`-`4000h` 对应 `-PI/2 < THETA < PI/2`，这定死了标度：`PI/2 <-> 0x4000`，故 `atan(1.0) = PI/4 <-> 0x2000`。**穷举全部 `(horner_shift, final_shift)` 组合，只有 `(14, 16)` 精确复现契约锚点**（`atan(0x4000)=0x2000` 精确、`atan(0x2000)=0x12E4` 与理论值分毫不差）。另一来源的 `fixShift=15` 属**另一套 Q 归一化**（它把 `QDIV(y,x,15)` 折进了切线，其 `t` 是 Q15，本实现用的是原始 1.14 输入）。<br>**③ 两个独立佐证算法找对了**：全域 `|v| <= 0x4000` 相对理想值的**最坏误差仅 2 个单位**（1 单位 ≈ 0.0055°）；**奇对称全域 0 违例**；且输出存在 **14 处 1 单位的下跳**（正负各 7，位置不对称），**全部落在 GBATEK 明说「THETA 在 ±PI/4 之外精度有问题」的区间**。抖动位置与文档所述区间吻合，是该多项式自身的特征。<br>**④ 途中发现一处会导致进程abort 的隐患**：`arctan` 在**契约域外**（`|v| > 0x4000`）Horner 乘法溢出 i32。这条路径可达 `extern "C"` 帧，**panic 无法 unwind、会直接带走整个模拟器**（纪律 6）。已改为 `wrapping_*` 算术，与 BIOS 在 32 位 ARM 上的回绕行为一致，并加锁测试确保域外不 panic。<br>**⑤ 认领集 11 → 12 个。** | S1c-a：ArcTan 取证与实现 |

| **r25** | 2026-09-30 | **`Swi` 枚举从 `0x10` 起整体错位一格 —— S1d 认领的五个解压号码全是错的，本轮更正。这是本项目迄今最严重的一处自身缺陷。**<br>**① 症状。** 枚举把 `0x0E` 标成 `BitUnPack`、完全没有 `0x0F`，并把 `0x10`–`0x1F` 依次错配成 LZ77/Huffman/RL/Diff/Sound。GBATEK 的 GBA 列表是 `0x0E BgAffineSet`、`0x0F ObjAffineSet`、**`0x10 BitUnPack`**、`0x11 LZ77Wram`、`0x12 LZ77Vram`、`0x13 HuffUnComp`、`0x14 RLWram`、`0x15 RLVram`、`0x16`–`0x18` Diff、`0x19 SoundBias`、…、`0x25 MultiBoot`、`0x26 HardReset`。**五个独立来源一致**（libtonc/deepwiki、`gba` crate、gbadev.net、mGBA 自带的 GBATEK 镜像、neser）。<br>**② 后果。** S1d 认领的 `0x10`/`0x11`/`0x12`/`0x13`/`0x14` 实际是 `BitUnPack`/`LZ77Wram`/`LZ77Vram`/`HuffUnComp`/`RLWram`。真实游戏调 `SWI 0x11`（LZ77 写回 WRAM）会拿到我们的 LZ77 写 VRAM 版本，调 `0x13`（Huffman）会拿到 RLE。**每个都静默地做错事，不崩、不越界。** 计划 r11 写的「jsmolka `memory.gba` 测 `0x11`–`0x18`」与 GBATEK 一致，**是枚举单方面错的**。<br>**③ 为什么 84 项测试全绿。** 唯一的结构性测试 `every_mapped_number_round_trips_through_its_name` 只验 `from_raw` 与枚举**互相自洽**——两边同时错位一格，它照样绿。这正是「对称的 bug 永远绿」：**两处同源错误互相抵消，任何只比自洽的断言都看不见。** 本轮新增 `every_number_carries_the_name_the_specification_gives_it`：**按号码独立写死名称表**，并断言 `0x2B`–`0xFF` 无名。<br>**④ 顺带把 `0x08` 从「读代码推的」变成「测出来的」。** r23 判定核心的 Sqrt 与契约一致、本项目不认领；本轮补 4 项锁测试（契约锚点 13 例、`0..=20000` 穷举 floor、全部 65535 个完全平方数及两侧、GBATEK 取整例的 floor 语义），**全部通过** —— 「不认领 `0x08`」从判断变成结论。<br>**⑤ 认领集号码更正**：`0x10`–`0x14` → **`0x11`–`0x15`**，个数仍为 12。分派臂按枚举名书写，故实现代码一行未改，**只有号码变了**。 | Swi 枚举整体错位一格（严重） |

| **r26** | 2026-09-30 | **S1c 剩余三项的取证完成；仿射矩阵符号的冲突已裁决；`0x0A` 由「不认领」改判为「应认领」。**<br>**① 取证源**：mGBA 的 HLE BIOS（`src/gba/bios.c`，MPL-2.0，**只读不抄**，实现仍为本项目一手 Rust）。此前 r20 记的「`raw.githubusercontent.com` 连接失败」本轮**未复现**，四个解码器与两个仿射函数一次性取到全文。<br>**② 仿射符号冲突已裁决（3 : 1）**：GBATEK 的 A–D 表格把 `B` 与 `C` **都**写成正的 `Sin(alpha)/xMag`、`Sin(alpha)/yMag`；而 tonc（coranac/libgba 作者，矩阵形式）与 mGBA（`(a,d)=cos`、`(b,c)=sin`，随后 `b *= -sx`、`c *= sy`）**一致给出 `PB` 为负、`PC` 为正**。mGBA 经大量真实游戏验证、tonc 是 GBA 圈事实标准的教学实现，两位不同作者独立同侧 → **GBATEK 该栏是错的**，按 `pb = -sin/sx`、`pc = +sin/sy` 实施。**这正是 tonc 自己警告过的失效模式**（他称 GBATEK 的元素描述在翻译成矩阵形式时丢失了信息），只是这次错在 GBATEK 原始表格里而非翻译环节。符号错误的代价是**画面整体上下颠倒**——不崩、不越界，只是画错，故必须靠取证而非靠锁测试发现。<br>**③ 两条结构规格此前无人写明，且都与结构体描述不符**（仅 mGBA 写明，故均为单源，需专门锁测试钉住）：<br>· **`BgAffineSet` 的源条目步长是 20 字节**（`2×s32 + 5×s16` = **18 字节有效** + 2 字节对齐 padding），**目标 16 字节**。GBATEK 与 CowBite 的结构体定义加起来只有 18 字节且不带尾部对齐，按字面布局实现会**在第二个条目上错位 2 字节**（偶数条目正常、奇数条目整体偏移），是最难自查的一种错。只读 mGBA 原文的 `offset += 20` 才能得到真实步长。⚠️ 本条初稿曾误记为「12 字节有效 + 8 字节 padding」，是**我在写计划时算错了字段数**，实施时由锁测试当场抓出；字段数 2+5 是对的，错的只是它与 20 的差。<br>· **`BgAffineSet` 不使用 `r3`**（mGBA 只读 `gprs[0..2]`）；`r3` 仅是 `ObjAffineSet` 的目标步长（2=连续、8=OAM）。gbadev 旧文档称 BgAffineSet 的 `r3` 是「offset between calculations」，**该说法无来源支持**。<br>**④ `0x10` BitUnPack 四源一致，零歧义**：GBATEK + mGBA `_unBitPack` 全文 + tonc `BUP` 结构 + `gba` crate 0.9.2 深读，四者对源长度/源宽/目标宽/偏移+零标志/32 位字对齐/字节内**低位优先**/目标**从低位往上**累积全部一致。**溢出语义亦定案**：`gba` crate 描述「值超出目标元素宽会污染后续元素」，mGBA 的实现证实它是**真的发生**（`out |= scaled << bitsEaten` 不做 mask）→ 照实实现并加锁测试钉住，记 **L10**。<br>**⑤ `0x0A` ArcTan2 改判（推翻 r23 ④ 与本轮开工前的初判）**：r23 判「数学上大概率正确，故可不认领」，本轮取得 mGBA 的 `_ArcTan2` 后发现**它是以 `_ArcTan` 为基础、分象限调多项式的整数实现，逐位对齐 BIOS**；核心的 `f64::atan2` + π 重映射与之在轴向情形差距明显（BIOS 对 `x=0` 或 `y=0` 返回 `0x0000/0x4000/0x8000/0xC000` 四个精确值，浮点版不保证）。`0x09` 已实证核心的三角实现不可信（r23 ②），`0x0A` 同属一个函数族 → **改为应认领**。<br>**⑥ 补丁集仍不增加（9 处）**：`0x0E`/`0x0F`/`0x0A` 所需的内存与寄存器访问，`Arm7tdmi::bus`（`pub`）与 `Bus` 的六个 `pub` 读写方法已全部够用，**不碰 vendor 树**。<br>**⑦ 认领集 12 → 最终 15**（`0x10` + `0x0E` + `0x0F` + `0x0A`），S1c 出口达标。 | mGBA 源码通读 + 四源交叉 |

| **r27** | 2026-09-30 | **S1c-d：认领 `0x0E` `BgAffineSet` 与 `0x0F` `ObjAffineSet`，认领集 13 → 15。**<br>**① 实施中当场抓出本文件 r26 的一处算错（已订正）**：r26 记「源条目 20 字节 = 12 字节有效 + 8 字节 padding」。**正确是 18 字节有效 + 2 字节对齐 padding**（`2×s32 + 5×s16`）。字段数 2+5 我在 r26 就写对了，错的是它与 20 的差。锁测试 `the_bg_source_entry_is_twenty_bytes_with_two_bytes_of_alignment` 当场转红（`left: 18, right: 12`）把它顶了出来 —— 若没有这条按算术写的断言，我会照着错误计划一路写下去。**这是纪律 5（测试向量按字段拼装）在计划文档一侧也成立的一次例证**：断言写成 `4+4+2*5 == 12` 时，错的是我心里的目标值而不是算式，只有把它和常量对比才能发现。<br>**② 这 2 字节为什么危险**：按 18 字节的字段布局走，**第一个条目完全正确**，从第二个条目起整体偏 2 字节。单条目测试 100% 通过，多条目静默出错，且不崩、不越界。故锁测试特意用**两个不同 scale 的条目** + 在 padding 填 `0xDEAD` 毒值，让错位必然产生可观测的垃圾值。变异验证：步长改成 18 → **恰好 2 项转红，且都只红在「第二个条目」**。<br>**③ 符号裁决已被测试钉死**：按 GBATEK 表格那个错法（`pb` 取正）实现后，**6 项转红**，报出的矩阵是 `(0, 256, 256, 0)` 而非 `(0, -256, 256, 0)` —— 即 r26 ② 预判的「画面整体上下颠倒」。这条错误**只靠锁测试是发现不了的**（自洽的测试会全绿），是取证先行的又一例。<br>**④ `r3` 语义两条都验了**：`BgAffineSet` **不读 `r3`**（对 `0`/`2`/`8`/`0xFFFFFFFF` 四种取值结果必须完全相同 —— 游戏随手留下的垃圾不能改变矩阵）；`ObjAffineSet` 的 `r3` 是**四个元素之间的步长**，`r3=8` 时 OAM 组内的填充字节必须原样保留。两条变异（把 `r3` 忽略 / 把步长写死 2）各自转红 2 项。<br>**⑤ 实施期手算错误又三处**（纪律 5，已全部改正）：`angle_theta(0xC000)` 误按 `-π/2` 断言（实为 `1.5π`）；把 8.8 定点的参考点位移当成像素除以 256；`0xFFFF` 误当作整圈（实为 255/256 圈）。<br>**⑥ 补丁集仍 9 处**，锁测试 102 → **122 项**，`cargo check` 两态通过。**NES 零回归仍未取得（L11 / L12）。** | 实施 + 三轮变异验证 |

> **定稿后的变更纪律**：本计划状态为 FINAL。执行期若发现计划与实态不符，**先记入本表再改**，不得静默偏离 §八 的阶段出口标准与 §十 的不变式。

| **r28** | 2026-09-30 | **S1c-e：认领 `0x0A` `ArcTan2`，S1c 收尾。实施中挖出 `0x09` 的一处位级缺陷并修复 —— 这是本项目最隐蔽的一次自查。**<br>**① `0x09` 此前并未逐位对齐 BIOS。** mGBA 的 `_ArcTan` 首个 Horner 项是 `-((i * i) >> 14)`：**先移位,后取负**。r24 的实现写成 `-(i * i) >> 14`，**先取负,后移位**。算术右移向负无穷取整，故两种写法在 `i * i` 不是 2¹⁴ 倍数时**恰好差 1**（几乎总是）。**这个差别不会崩、不会越界、精度仍在 2 单位内，只会让每个结果差 1 个单位** —— r24 的全部测试（契约锚点、2 单位精度、范围、单调性）**全部照绿**。直到 r26 取到 mGBA 全文、逐行比对 Horner 链才暴露。<br>**② 更深一层：取「绝对值再最后取负」也不对。** 那种写法是**严格奇函数**，而 BIOS 不是 —— 它在有符号输入上运算、由算术移位决定取整方向。这条不对称会被 `arctan2` 的八个象限继承，故**折叠掉它就会污染 `arctan2`**。已改写为有符号运算，并把「奇对称仅在取整方向上成立」写成性质测试。<br>**③ 副作用是好的**：wobble 点从 14 处降到 6 处（正负各 3）。旧写法在负半轴上的额外抖动**不是硬件的**，是自己造的。<br>**④ `0x0A` 实施中抓到第二类错误 —— 两个函数的标度不同**。`arctan` 的 45° 是 `0x2000`，`arctan2` 的 90° 是 `0x4000`；我在象限偏移里误用了 `QUARTER_PI`，结果答案**落在错误的象限而不是越界**。变异验证：换回去 → 4 项转红。教训与 r26 ② 同型 —— **符号/标度这类量，锁测试必须按「从契约推」而不是按「从另一个函数搬」来写**。<br>**⑤ 一个等价变异，如实记录**：删掉 `arctan2` 的 `x == 0` 短路后，**全部测试仍然全绿**。查证后确认这是**等价变异而非逃逸**：`x == 0` 时比值为 0、`arctan(0) = 0`，第二/第六象限分支本就求值到同样的 `AXIS_POS_Y` / `AXIS_NEG_Y`。短路保留的理由是「硬件这么做」而非「不这么做会错」，已写成测试把这个冗余固定下来，免得下一个���重新推导一遍。<br>**⑥ S1c 状态**：认领集 16 个（`0x08` 是唯一**验证过**不认领的），锁测试 89 → **132 项**，`cargo check` 两态通过，补丁集仍 9 处。**出口判据「mode 7 与精灵缩放样例通过」需帧缓冲，只能在 S2 取得** —— 与 jsmolka 门禁同属「判据自身依赖尚未交付的能力」，处置方式同 r11。**NES 零回归仍未取得（L11 / L12）。** | 实施 + 三轮变异验证 |
