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

#### 扩展点接线（gba-core 的本地修改，共 5 处）

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
| **T2** | `Sqrt` `ArcTan` `ArcTan2` `BgAffineSet` `ObjAffineSet` `BitUnPack` | ❌ 未 HLE | 影响 mode 7 / 精灵缩放 / 部分 2D |
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
| **S1b** | **T1-c** `RegisterRamReset` 补 IWRAM；**T1-d** 解压类：`Lz77×2` / `Huffman` / `Rl×2` / `GetBiosChecksum` | **按 r11 同样重定义**：T1-c/T1-d 逐项**算法级锁测试**（含往返压缩/解压与参考向量）+ 认领集扩大到 8 个号码后核心其余 SWI 仍全部可达。~~jsmolka `memory.gba` 在 stub BIOS 下全绿~~ → **移至 S2** | 1–1.5 周 | 未开始 |
| **S1c** | **T2**：`Sqrt` / `ArcTan`×2 / `BgAffineSet` / `ObjAffineSet` / `BitUnPack` | 逐位对齐验证；mode 7 与精灵缩放样例通过 | 1–1.5 周 | 未开始 |
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
| **R15** | **T1-b 把补丁集推出 R1 允许的范围**。R1 的缓解写的是「补丁集最小化（仅 SWI hook）」，而 halt 机制必须落在 `step()` 里：`Bus::step` 是 `pub(crate)`（`bus.rs:1214`），根 crate 拿不到推进外设的入口，`handle_swi_hle` 也是私有的 | 补丁集 **5 → 9 处**（`halted` 字段 + 唤醒判据 hook + 两处 `Default` 初始化 + `step()` 守卫 + **`Arm7tdmi::new` 清 CPSR 的 I 位**），其中 `step()` 与 `new()` **改动 CPU 执行路径与复位状态**——不再是「不动上游逻辑」 | 已按最小实现收敛：三条执行路径（ARM/Thumb/IRQ）**只需在 `step()` 顶部一处守卫**，不必分插两处；唤醒条件经 hook 回调，语义全在我们这侧。`ATTRIBUTION.md` §3.2 同步为 9 处并写明第 6–9 处确实改变行为 |
| **R11** | **sample-and-hold 音质**。核心以 S&H 上采样，存在镜像/混叠 | 音质弱于高阶重采样 | 已知取舍，非缺陷；如需改善须改核心或自行重采样 |
| **R12** | **分数采样实现错误**。`int` 帧长无法表达 738.35，若实现偷懒取整必长期漂移 | 音画不同步 | ABI 分离错误码与样本数（§4.1）；30 分钟无漂移为 S2 硬判据 |
| **R13** | **wait 类空壳被误当已实现**（上游 `0x02` / `0x03..=0x05` 均为 `swi_return`） | 游戏以 100% CPU 空转、耗电、行为异常 | S1a 硬性要求真实现，并设专项验证 |
| **R14** | **rustc 1.96 fat LTO 拒绝加载依赖链中第二个 archive 的 bitcode**（2026-09-28 六组对照实验，见 §十四 r8） | GBA 逻辑若做成独立 crate（`f11gba`），构建直接失败，报空诊断的 `failed to load bitcode of module …-cgu.0.rcgu.o` | **已规避并已定位**：唯一变量是「是不是独立 crate」，与代码量（2 函数 vs 381 行）、依赖关系（有无 gba-core）、泛型均无关；`lto = "thin"` 可绕过但会波及 NES 侧性能。**现行方案**：GBA 逻辑降级为根 crate 模块 `src/gba/`，经 gba-core 的 hook 字段注入（§5.2），不走 rlib 边界。**复查触发条件**：rustc 升级后重跑 §十四 r8 的六组对照 |

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

| **r13** | 2026-09-29 | **规格取证完成，并撤回 r12 之后的那个"修复"——它本身是错的。**<br>**① 取证源**：mGBA 的 HLE BIOS（`src/gba/bios.c`，`_unLz77` / `_unHuffman` / `_unRl` / `_RegisterRamReset`），该实现经真实硬件验证。**只作为格式参考阅读，不复制代码**（MPL-2.0 与本项目许可不相容），实现仍为本项目一手 Rust。<br>**② GBA 与 NDS/Wii 的压缩头不兼容，而两者常被并列描述，极易混淆**：GBA 是「字节 0 = 签名 `0x10`/`0x20`/`0x30`，字节 1-3 = 24 位解压长度」；NDS/Wii 是「bit 0-23 = 压缩长度，bit 24-30 = 长度，bit 31 = 是否按 8 字节单位」。**本文件先后错成两次**：S0' 读成 `raw & 0x0FFFFFFF`（把签名并进去了），S1b 的"修复"读成 `(raw >> 24) & 0x7F`（那是 NDS 布局）。正确读法是 `raw >> 8`。两次的错误都被**自建测试**盖章通过，因为两次的测试向量都用了低 24 位为零的头——真实文件不可能出现。现已改正，并加一项测试专门演示 NDS 头按 GBA 读会**静默**得到大 0xA0000 倍却仍通过合理性上限。<br>**③ 已取到的完整格式（供 S1b 实现）**：<br>· **LZ77**：标志字节后 8 个块，**低位在前**；块 = 2 字节大端，`disp = dest - (block & 0xFFF) - 1`、`len = (block >> 12) + 3`。<br>· **RLE**：块首字节 bit7 置位 = 重复 `((b & 0x7F) + 3)` 次的 1 字节；清零 = 紧随其后的 `((b & 0x7F) + 1)` 个字面字节；末尾补齐到 4 字节。<br>· **Huffman**：头低半字节 = 每元素位数，高 3 字节 = 长度；`treesize = (byte4 << 1) + 1`，树基址 = src+5，位流从 src+5+treesize 起按 **32 位字、高位在前**；节点字节 bit0-5 = 偏移、bit6 = 右子为叶、bit7 = 左子为叶，子节点位于 `(nodeAddr & ~1) + offset*2 + 2`（右子 +1）；按位数聚成 32 位后整字写出。<br>· **`GetBiosChecksum` (0x0D)**：r0 = BIOS 校验和常量、r1 = 1、r3 = `0x4000`。stub BIOS 无真校验和可算，取常量并记为已知限制。<br>· **`RegisterRamReset` (0x01)** 位定义（`0x02` 位 = 清 IWRAM **但保留末 0x200 字节** —— 栈、IRQ 指针、IntrWait flag 都在那里）；`0x80` 位含 IE=0 / IF=0xFFFF / WAITCNT=0 / IME=0。<br>**④ 这条记录本身是 r11 那条结论的验证**：S1b 卡住不是因为它工作量更大，而是因为**当时手上没有可核的材料**。取证只花了半小时，而它立刻推翻了我自己的一个错误结论。 | S1b 规格取证 |

> **定稿后的变更纪律**：本计划状态为 FINAL。执行期若发现计划与实态不符，**先记入本表再改**，不得静默偏离 §八 的阶段出口标准与 §十 的不变式。
