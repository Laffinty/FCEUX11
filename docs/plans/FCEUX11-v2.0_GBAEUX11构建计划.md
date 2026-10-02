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
| 5 | 测试 | **不接入任何质量检测系统**（F11QA / KagamiQA / jsmolka 均不做）；以锁测试的多源交叉证据结案（§7.5、r41 不变式 9） |
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
| 接入 F11QA / KagamiQA / jsmolka 全套 | **不做**（r41 不变式 9：GBA 长期 Alpha、不阻塞本体、不是任何检测系统的对象）。原口径「仅 jsmolka 进 CI」已作废 —— 见 r42 ② |
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
> > **r42 改注**：上一句是 **r11 当时写的**（原意是把它列为 S2 的外部门禁）。**该角色已取消** —— r41 不变式 9 规定 GBA 不是任何检测系统的对象，§7.5 已把它降为「可选信号，不进门禁」。**本句保留是因为它记录了一个真实的取舍过程**（上游的核心 SWI 专项测试恰好是 `memory.gba`，而我们决定不去跑它），而不是因为它仍是一条待办。

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
int32_t     gba_set_output_rate(uint32_t rate);
                    // 宿主采样率 Hz。必须在 gba_step_frame 产生音频前设定；
                    // 改动会重建 rtrb 环，已缓冲的音频随之丢弃（可接受：
                    // 改采样率本就意味着换了输出设备）。
                    // 未载 ROM 时调用会被记住，在建机时套用。
int32_t     gba_set_volume(uint32_t volume);
                    // 标度 0-150，与 FSettings.SoundVolume 同一标度（fceu.cpp:612）。
                    // 系数 = volume / 150.0；> 150 视为增益，交给饱和处理。
int32_t     gba_render_audio(int32_t *dst, uint32_t cap, uint32_t *out_frames);
                    // 返回 GBA_OK/GBA_ERR_*；实际产出帧数走 out_frames。
                    // 因每帧目标样本数为分数（44100/59.7275 ≈ 738.35），
                    // 以定点累加 + 残余结转决定本帧产出量（仿 sound.cpp:1359-1366 的 soundtsoffs/left），
                    // 且必然出现 out_frames 不等于 dst 所按 738 计算的值的情形。
                    // 契约（r34 ⑦）：out_frames 恒等于相位累加器给出的 n；
                    // 环内样本不足时补静音并累加 underrun，不减帧数。
uint32_t    gba_audio_underruns(void);
                    // 自上次 reset 累计的欠载样本数。§8 性能判据「underrun 计数为 0」取此值。
void        gba_samples_per_frame_fixed(uint32_t *num, uint32_t *den);  // 返回帧率比分数（约分后）
int32_t     gba_set_overlay(int enable);   // 见 §7.1 双策略；发布构建中 enable=0 返回 GBA_ERR_STATE（语义不可关闭）
                    // ⚠️ r34 ③ 订正：本行原写 `void`，与同一行的「返回 GBA_ERR_STATE」自相矛盾。
                    // 不变式 5 要求它返回错误码，故正确形状是 int32_t —— S2-a 的实现与
                    // fceux11_rust.h 生成声明一直是对的，只有本文件的笔误。

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
   │      ⚠️ r34 ⑧：上界取 **32767 而非 32768**。WriteSound 契约幅度是 ±32768，
   │      但 SDL 回调转 AUDIO_S16SYS 时 32768 会绕成 -32768，满刻度正弦在帧边界极性反转。
   └─ 分数累加（r34 ⑦ 给出确切式子，替掉原表的歧义表述）：
                phase += rate × 4389
                n = phase >> 18;  phase -= n << 18      ← 残余结转到下帧
                每帧样本 = rate × 4389 / 262144，故 2^18 帧恰好产出 rate×4389 个样本，
                而 2^18 帧正好 4389 秒 ⇒ 恒等于 rate 样本/秒，无累积漂移。
                ⚠️ 不可简化为「每帧固定 738 个」（即 sdl-sound.cpp:263 那个截断写法），
                那正是 R12 描述的长期漂移。
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

> **r42：本节整体改写。** 原表的「CI（进）」两行与 r41 不变式 9 冲突 —— 不变式 9 说 GBA 不是任何检测系统的对象。**为什么它们本来也不该是门禁**：jsmolka 的判定信号是**屏幕上的 mode 4 文字**（r11 已记），读它需要帧缓冲通路；r11 当时把门禁「移到 S2」正是为了等这条通路 —— **而该通路要到 S2-b4 阶段 2' 才存在，且新原则已取消该门禁**。于是这两行不是「在等一个条件」，而是**不做**。它们连同上游套件的存在一并保留在下面，**是调研记录，不是待办**。

| 层 | 内容 | 状态 |
|---|---|---|
| **锁测试（唯一门禁）** | `src/rust/src/gba/**`，200+ 条，全部为算法级 / 协议级 / ABI 级断言；每条新测试都要能变异转红 | **在做，且是 GBA 唯一的出口判据** |
| **多源交叉取证** | 当锁测试不足以定案时，用**相互独立的**实现 / 反汇编 / 规格书互证（L7 的位序即是此法的产物，见 r17 的 5:1 对照与 r21 的第二个模拟器） | **在做。纪律：数的是独立来源数，不是文档数** |
| jsmolka 合成测试集 | `emu/tests/jsmolka.rs`，对 `jsmolka/gba-tests`。`memory.gba` 原为 SWI 实现的核心验收门 | **可选信号，不进门禁**（r41 不变式 9） |
| 授权 homebrew 黄金帧哈希 / 音频冒烟 | 无版权风险 | **可选信号，不进门禁**（同上） |
| **本地手工（不进 CI）** | 商业 ROM 实测清单（版权与来源策略 **S0 即定**） | Alpha 阶段按需手工，不作门禁 |
| **F11QA / KagamiQA** | 完整接入 | **不做**。若 GBA 将来转正式版再议（§8.1 P3 已改注） |

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
| **S1a-1** | 回归验证 T1-a（已实现项）；**T1-b 真做**：给 `Arm7tdmi` 加 halt/stop 状态、在 `step()` 插入判据、实现 `Halt`/`Stop`/`IntrWait`/`VBlankIntrWait` | **出口判据已按 §十四 r11 裁决 (a) 重定义**：wait 类专项验证（非空壳且 CPU 确实停住）✅；T1-a 回归验证 ✅；stub BIOS 的启动与 IRQ 全链路端到端 ✅。~~jsmolka `arm`/`thumb` 通过；可启动到游戏画面~~ → **r42：该重定义的前提已不成立**。r11 当时把门禁移到 S2 是为了等帧缓冲通路，而 r41 不变式 9 已取消 jsmolka 门禁本身（见 r42 ②） | ~~1 周~~ → **2–3 周**（见 §5.3 T1-b 注） | ✅ 已完成（2026-09-29） |
| **S1b** | **T1-c** `RegisterRamReset` 补全（**缺口是两个不是一个**，见 §十四 r15）；**T1-d** 解压类：`Lz77×2` / `Huffman` / `Rl×2` / `GetBiosChecksum` | **按 r11 同样重定义**：T1-c/T1-d 逐项**算法级锁测试**（含往返压缩/解压与参考向量）+ 认领集扩大到 **11** 个号码后核心其余 SWI 仍全部可达。**认领为全有全无**（r15 ②）。**⚠️ 号码已由 r25 更正：`0x10`–`0x14` → `0x11`–`0x15`（GBATEK 的 `0x10` 是 BitUnPack）**。~~jsmolka `memory.gba` 在 stub BIOS 下全绿~~ → **r42：同上，该门禁已取消**，不再移至任何阶段 | 1–1.5 周 | **S1b ✅ 已完成**（2026-09-30）—— 认领集 11 个，达标，**号码经 r25 更正**。T1-c ✅ · T1d-a ✅ · T1d-b ✅。锁测试 27 → **89 项** |
| **S1c** | **T2**：`Sqrt` / `ArcTan`×2 / `BgAffineSet` / `ObjAffineSet` / `BitUnPack` | 逐位对齐验证；mode 7 与精灵缩放样例通过 | 1–1.5 周 | **✅ 已完成**（2026-09-30）—— `0x09`（r24）· `0x08` 验证不认领（r25 ④）· `0x10`（r26）· `0x0E`+`0x0F`（r27）· `0x0A`（r28）。**认领集 16 个号码。锁测试 89 → 132。**⚠️ 出口判据「mode 7 与精灵缩放样例通过」需帧缓冲，**只能在 S2 取得**；本阶段达成的是算法级逐位对齐。⚠️ **NES 零回归仍未取得**（L11 / L12） |
| **S2** | C ABI + Qt 前端：`.gba` 识别、240×160 渲染、音频接入（含 §4.2 分数采样）、RTC 暴露、`BETA` 水印 | 可玩游戏，画面/音频正常；**性能验收线达标**（见下）；30 分钟无音画漂移 | 2 周 | **代码全部落地，出口判据形式未闭合**（r48 据实订正：此格原写「进行中 · S2-a 已落地，r32」，停在 S2-a，而 S2-b1/b2/b3、S2-b4 三个阶段与 S2-c 均已完成，见本行各段与 r33–r45）。**形式未闭合的含义要说准**：三条出口判据里「可玩游戏，画面/音频正常」与「30 分钟无音画漂移」在本环境**不可验** —— 无 `.gba` 资产，按不变式 9 也不打算接外部基准；「性能验收线」的 underrun 部分已由锁测试覆盖（首次抽干后零增长，r35 ④），30 分钟实跑仍是人工判据。<br>**拆分为 S2-a / S2-b**（用户裁决 2026-09-30）：S2-a 不依赖任何未决项，做完即可首次执行 jsmolka 门禁；S2-b 的两个阻塞点（RTC 补丁、savestate ABI）一并解决。<br>**S2-a 已完成**：**16** 个 C ABI 函数（**6** 探针 + 10 生命周期/帧；第 6 个是 r30 ④ 加的 `gba_probe_cpu_size`。**r34 ② 订正本行原写的 15**）+ 错误码 + 头文件声明，锁测试 145 → **156**（r34 ② 订正本行原写的 158）。**r34 ① 补齐了 S2-a/S2-c 两次改动后一直欠着的门禁证据**：全量 Release 构建通过、`ctest` 33/34、导出符号经 `dumpbin` 确认进入 CMake 实际链接的 staticlib。<br>**S2-b1（音频）✅ 完成，r35**：导出 5 个符号（`gba_render_audio` / `gba_set_output_rate` / `gba_set_volume` / `gba_samples_per_frame_fixed` / `gba_audio_underruns`），§4.2 的分数累加 + 残余结转落地，每帧样本数 `rate×4389/262144` 可证零漂移。锁测试 166 → **185**，15 处变异验证全红。**§8 性能判据口径据实收紧**（underrun「为 0」→「首次抽干后不再增长」，因载 ROM 有一次 220 样本的启动瞬态，见 r35 ③④⑤）。<br>**S2-b 其余两项未开始**：RTC（第 10 处 vendor 补丁）、Qt 前端接线。**S2-b2（savestate 导出）✅ 完成，r37**：导出 3 个符号（`gba_savestate_size` / `_save` / `_load`），**不出 `_free`**（按 §4.1 由调用方持有 buffer，r31 ③ 的形状作废）。读档经**自带 16 MB 栈的工作线程**（实测：release 解码需 2 MB、Debug 需 6–7 MB、编码仅 48 KB，而 Qt 的 `QThread` 只有 1 MB —— 详见 r37 ②④）。ROM/BIOS 是核心的 `serde(skip)]`，故由载机重建；**ROM 指纹**（`game_code`+`marker_code`+长度+FNV-1a）拒绝把存档读进别的卡带；`halted` 之外的**唤醒谓词与 pending `IntrWait`（thread_local）显式存取**，否则读档后会静默退化成 `Halt`。错误码补 `GBA_ERR_UNSUPPORTED=4`、`GBA_ERR_STATE` 改回 **5**，与 §4.1 对齐。锁测试 185 → **203**（+18，1 条测量工具 `#[ignore]`），导出符号 21 → **24**。<br>**S2-b3（RTC）✅ 完成，r38**：核心里 S3511 早已完整且对游戏透明，**真正缺的只是「可被验证」** —— `current_unix_secs()` 直接读 `SystemTime::now()` 且无注入点，核芯自己的 RTC 测试是拿同一刻的主机时钟当期望值（**自比较**），7 个字节只读 1 个。**补丁 9 → 14 处，跨 1 → 3 个文件**（`arm7tdmi.rs` 9 + `rtc.rs` 3 + `internal_memory.rs` 2；`bus.rs` 未动，因 `Bus::internal_memory` 与 `Arm7tdmi.bus` 本已 `pub`）。**这 5 处不改变上游行为** —— 不设覆盖时芯片行为与上游逐位相同。导出 2 个符号（24 → **26**）：`gba_rtc_set_time(int64_t, int32_t enable)` 与 `gba_rtc_time(int64_t *out)`；**用显式 enable 而非 `0` 哨兵**（`0` 会让 Unix 纪元本身无法钉死，而纪元正是唯一能手算期望值的时刻）。**不导出「是否 RTC 卡带」** —— `gpio_present` 只表示「游戏碰过 GPIO」，拿它回答「这张卡有没有 RTC」是答非所问。**锁测试 203 → 213**，含一条端到端芯片级测试：钉住 `1_612_325_106`、经 `Gba` 公开 bus 走完整 S3511 时序、断言 7 字节 `[21 02 03 03 04 05 06]`，**星期几是新覆盖的**（变异 `(weekday+4)`→`+3` 转红，证明核芯自己测不到这一项）。变异验证 6 处全红。<br>**Qt 前端接线三个阶段全部完成（r40 / r44 / r45）**：① 装载 / 输入 / emu 循环分叉（补上 §4.1 列了却**从未实现**的 gba_set_buttons，**零 vendor 补丁**）；② **画面**：自己的 153,600 字节帧缓冲（经 gba_frame_buffer 取，**该导出在水印之后取帧，所以 viewer 拿到的就是玩家看到的画面**）、自己的 SDL_Texture、自己的 3:2 信箱，**只设 litUpdated 一个共享标志，绝不写 NES 的 pixBufPool / pixBufIdx**；③ **音频与限速**：会话翻转时把**设备协商到的**速率与宿主音量推给核心、并把限速基准从 60.098823 切到 **59.7275**（16777216/(228×1232)），每帧按**分数**样本数排空、上界只取 GetWriteSound()。<br>**三个阶段里 GBA 侧只碰了 gba_load.{h,cpp} 与 ceuWrapper.cpp**（均已在白名单内），**限速与声卡的四个文件一个 GBA 字样都没有**（实测代码级引用数 0，并由 	he_timing_and_sound_files_stay_gba_free 守卫强制）。**黑屏的真正成因与一处订正（r44 ①②）**：litUpdated **同时门控 	ransfer2LocalBuffer() 与 queueRedraw()**，而 120 Hz 定时器也只走到那个门控之内；另 r39 ③「
ender() 的缩放数学用编译期常量」**是错的**（它本来就从 
es_shm->video 读运行时尺寸），但 r41 ④ 避开泛化的结论不变。导出符号仍 **28**（三个阶段**未新增任何 C ABI 函数**）；锁测试 213 → **227**；守卫白名单 5 → 7 条；**vendor 补丁集仍 14 处、跨 3 文件，S2-b4 三个阶段零 vendor 改动**。<br>**覆盖缺口（必须说清）**：② ③ 的呈现与排空逻辑写在 C++ 里，**本仓库没有 C++ 单测夹具**，本轮验到的是「接线自洽 + 守卫仍成立 + NES 未回归（ctest 33/34）」。「一个真实 .gba 看起来对、听起来对」在本环境无法验证，且按不变式 9 不打算去验。阶段 1 = `.gba` 能被识别、装载、逐帧推进、收按键（补上 §4.1 列了却**从未实现**的 `gba_set_buttons`，**零 vendor 补丁**）；阶段 2' = **GBA 有画面了**：自己的 153,600 字节帧缓冲（经 `gba_frame_buffer` 取，**该导出在水印之后取帧，所以 viewer 拿到的就是玩家看到的画面**）、自己的 `SDL_Texture`、自己的 3:2 信箱，**只设 `blitUpdated` 一个共享标志，绝不写 NES 的 `pixBufPool` / `pixBufIdx`**。<br>**黑屏的真正成因与一处订正（r44 ①②）**：`blitUpdated` **同时门控 `transfer2LocalBuffer()` 与 `queueRedraw()`**（`ConsoleWindow.cpp:627-640`），而 120 Hz 定时器也只走到那个门控之内 —— **所以定时器兜不住重绘**。另外 r39 ③ 那句「`render()` 的缩放数学用编译期常量」**是错的**：`render()` 本来就从 `nes_shm->video.ncol/nrow` 读运行时尺寸，`GL_NES_*` 只是 `nes_shm == NULL` 的 fallback。**r41 ④ 避开泛化的结论不变**（改 `GL_NES_*` 的另外 10 处仍会改变 NES 行为）。导出符号仍 **28**（**本阶段未新增任何 C ABI 函数**）；锁测试 213 → **226**；守卫白名单 5 → 7 条（**新增两条正是本阶段必须触碰的，每条都写了理由**）；**vendor 补丁集仍 14 处、跨 3 文件，本阶段零 vendor 改动**。<br>**S2-c（自建门禁）✅ 完成，r33**：5 条 SWI 往返测试在**真实 ARM 指令流**下全部通过（`Div`/`Sqrt` 走核心、`ArcTan` 契约锚点、`ArcTan2` 轴向精确常量、`GetBiosChecksum`），外加一条独立的卡带控制权测试。**至此 16 个认领号码首次在「真实指令」这一层取得证据**，此前所有证据都来自直接调用 `dispatch` 的锁测试。 |
| **S3** | NES 兼容键位、`.srm` 存档（含覆盖机制与跨模拟器互通）、即时存档（含**加载后重挂 `init_audio`**）、RTC | 存档跨会话可读、跨模拟器字节级互通、键位符合 §7.2、**savestate 往返后音频不哑** | 1–1.5 周 | **S3-1 ✅**（r46）· **S3-2 ✅**（r47）· **S3-3 ✅**（r48：分支落在 `FCEUSS_Save`/`FCEUSS_Load`，菜单门控拆组放行 savestate 六项、录像三项保持不可用，白名单 6 → 7 并新增一条守卫，零 vendor 改动、零新增 C ABI）。**「加载后重挂 `init_audio`」在 Rust 侧本就实现了**（`frame.rs:449-455`），故该判据零 C++ 工作。**余下两项未接线**：**RTC** 的 C ABI 已有而 C++ 侧零调用，**手动覆盖存档类型**的 `gba_set_save_type` 有 ABI 无 UI。**⚠️ 出口判据第 2 条「跨模拟器字节级互通」仍开放** —— 本环境无 `.gba` 资产、无 mGBA，按不变式 9 也不打算去验（r46 ⑤ 已记） |
| **S4** | 手工实测清单逐游戏过 | 清单内游戏可玩，已知限制逐条编目 | 长尾 | **进行中 · 三轮实测，found 2 个 P0 并均已修**（r49 / r50 / r51）。**r49**：stub BIOS 启动序列两处错误，真实卡带游戏代码从未执行。**r50**：S2-b4 阶段 2' 只给 `ConsoleViewerSDL` 写了 GBA 分支，**OpenGL 与 QWidget 两个 viewer 完全没有** ⇒ 有独立显卡的机器选中的正是 OpenGL，GBA 会话被丢掉。**两处都在守卫全绿的情况下通过** —— 白名单只能抓「已存在」抓不到「缺失」，故 r50 同时把该盲区变成守卫。**r51**：画面改整数倍铺满；按键三层根因（`joy[]` 由 NES 核心写 / 方向键绑的是摇杆轴 / NES 与 GBA 方向键顺序不同）全部修复。<br>**✅ `Super Mario Advance 4` 实测出画面并已铺满**（语言选择画面，文字/精灵/调色板全部正确，`BETA` 水印在位）。**按键链路已验证到「掩码正确」**（按下 Down ⇒ `pad=0x0080`，32 个绑定中仅该绑定为 1），**但「游戏真的响应」未验证通过**（前后两帧逐像素相同）—— 掩码对 ≠ 游戏收到，两者不合并陈述。<br>**⚠️ `Mario Kart - Super Circuit` 仍为纯白**，探针确认卡在第二层（`rgb(31,31,31)`、30M 步 1 种配色），**未解决**。清单内其余游戏未测。探针 `probe_a_real_cartridge`（`#[ignore]`）可复跑取证 |

**合计约 8–10 周**（前版「约 3 周」的估算已作废，原因是 §5 的 BIOS 路线重估）。

**`bios.gba` 判据重定义（重要）**：上游测试**当前依赖真 BIOS**（README 硬编码 `gba_bios.bin`），其 `bios.gba` 失败基线是「以真 BIOS 运行」的结果。stub BIOS 是**新交付物**，不是「换个 blob」。
故 S1b 的验收判据是 **「`memory.gba` 在 stub 下全绿」**（该 ROM 专项测 SWI 0x0B/0x0C/0x11–0x18，正是 T1-d 的靶子）。
`bios.gba` **不作为 BETA 判据**——它测的是 BIOS 函数行为，在 stub 下部分语义本就不等价；是否转 pass 列为 GA 后事项（§8.1 P5）。

**性能验收线（F-15 补入）**：在中档桌面（i5-8xxx / 16GB）下，240×160 稳定 59.7275 FPS，**音频 underrun 计数在首次抽干之后不再增长**，**连续 30 分钟无音画漂移**（此项直接检验 §4.2 的分数采样实现）。

> **口径订正（r35 ⑤）**：原文写「underrun 计数为 **0**」，**在本实现下字面不可达** —— 每次载 ROM 都有一次约 220 样本的启动瞬态（首个 VBlank 在第 160/228 条扫描线报出，故首次抽干只经过 197,120 周期而非满帧 280,896）。已实测该瞬态是**与帧数无关的常数**：100 帧内计数收敛于 221 后零增长，环内余量稳定在 2 槽。**判据改为「不再增长」正是为了让它可测**；写成 0 会把一次正常的启动瞬态报成缺陷，或诱导实现去掩盖它。**30 分钟实跑仍是人工出口判据** —— 单元测试能诚实断言的量级是 100 帧。

### 8.1 GA 后排期

| 阶段 | 内容 | 估 |
|---|---|---|
| **P1** | M4A 补齐（§六） | 2–3 周 |
| **P2** | wait cycle 补齐、已知限制清零 | 另议 |
| **P3** | 接入 F11QA / KagamiQA | **r41：与不变式 9 冲突，本轮不排期**。若 GBA 将来转正式版再议 |
| **P4** | 立体声输出评估（另开 SDL 设备） | 另议 |
| **P5** | `bios.gba` 在 stub 下的可达成范围评估 | 另议 |

### 8.2 GA 门禁（前版缺失，本次补入）

全部满足方可宣布 2.0 GA：

1. S0–S4 全部出口标准达成；
2. ~~jsmolka **10/10 全绿**（含 `bios.gba`）~~ → **r41 不变式 9 取消此项**：GBA 不是任何检测系统的对象，且长期 Alpha、**不阻塞本体**。**2.0 GA 不以 GBA 精度为条件。** 取消的是「门禁」这个角色，不是这份调研（§7.5 仍记录 jsmolka 套件的存在与它为何不适用）
3. 手工实测清单内游戏全部可玩，已知限制 **≤ 8 条**且逐条有据；<br>&nbsp;&nbsp;&nbsp;&nbsp;> **r43：已满足 —— 这是全计划唯一曾未闭合的出口判据，现已闭合。** 开放 **8 条**（L1、L2、**L4**、**L6**、**L7**、L10、L13、**L15**），已消 7 条。**r43 消掉 L14**（`gba_reset` 现在真的复位），此前为 9 条、超出 1 条。**L6 是 r43 尝试过但取不到权威复位值表、故按纪律 4 保持开放的**（见该条与 r43 ②）。**用户裁决（2026-10-01）：维持「≤ 8」，不动阈值，靠真消一条来满足。**
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
| **L4** | 未经 jsmolka 外部基准验证。S0–S1a 的全部结论来自本项目自建的锁测试 | 自测无法发现「实现与规格不符」——LZ77 头解析已实证过一次（§十四 r12 / r13） | **不再以 jsmolka 为条件**（r42 ②）。结案依据改为**多源交叉取证**，且已有两项：r17 的 5:1 来源对照、r21 取得第二个独立模拟器互证。**代价要说准：本条在 Alpha 阶段长期开放** —— 唯一能改变它的动作是「新发现一个独立来源」，而外部基准已被取消 |
| **L5** | ~~`RegisterRamReset`（0x01）的 IWRAM 清零由**核心**实现且**主动跳过**；bits 5–7 更是一行代码都没有~~ → **S1b-c 已消**：认领 `0x01` 并补全 bits 0–4 + `0x80`，IWRAM 末 `0x200`（栈 / IRQ 向量 / BIOS flags）保留 | — | **已消**（2026-09-29） |
| **L6** | `RegisterRamReset`（0x01）的 **bits 5/6（复位 SIO 与声音寄存器）不实现**。**r43 记录了一次关不掉它的完整尝试**：`problemkaputt.de/gbatek.htm` 对本工具的 UA 仍返回 **403**（r20 ① 已记，本次复现）；镜像（`mgba-emu.github.io/gbatek`、`gbatek-gbaonly`、`gbatek-gba-i-o-map.htm` 缓存、`afska/gba-link-connection` 转录）给的是**寄存器映射**（SIO 侧 `0x120`–`0x12A` / `0x134` / `0x140`；声音侧 `0x060`–`0x07C` 四声道、`0x080`–`0x088` 全局与 SOUNDBIAS、`0x090`–`0x09E` WAVE_RAM、`0x0A0`–`0x0A6` 两个 FIFO），**但没有一处给出 bits 5/6 的复位语义与逐寄存器复位值**；唯一提到默认值（SOUNDBIAS `0x0200`）的说法出自 `rust-console/gba` 的 deepwiki 文档且用词 "typically"，**既非 GBATEK 原文也非复位表** | 显式请求复位 SIO/声音寄存器的程序不复位它们。游戏极少调用；声音寄存器复位与 §六 的 **M4A 延期**原绑定，r43 已解绑 | **保持开放**（r43 ②）。按纪律 4「取不到权威材料就停下来记录，不写」，本轮**不猜**。将来若取得该复位值表可直接续做 |

| **L7** | **LZ77 标志字节的位序（MSB 优先）在 jsmolka 外部基准跑过之前只有自建证据。** §十四 r17 以 5 : 1 的来源对照推翻了 r13 与 S1d-a 原先的 LSB 读法；**r21 ③ 又取得第二个独立模拟器（gbajs2，MIT）的源码互证 —— 其 `lz77()` 先判 `0x80`**，即同为 MSB 优先。现状是「mGBA 一家持 LSB」对上「两个模拟器 + BIOS 反汇编 + 规格书 + 两个 Nintenlord 系工具持 MSB」。**仍无法排除 mGBA 对两种顺序都做了容错** | 若位序判断有误，真实游戏的压缩图形会整体错位 —— 不崩、不越界，只是画错，故 CI 与自建锁测试**全部照绿**，只有跑真实资产才暴露 | **不再以 jsmolka 为条件**（r42 ②）。**本条同样在 Alpha 阶段长期开放**：MSB/LSB 正是「规格书一句话散文」最容易弄反的地方（r13 / r17 / r22 三次栽在这里），而改变它的唯一动作是**再找到一个独立来源** |
| **L8** | ~~`0x12` Huffman 的 `treesize` 偏置两说并存~~ → **已消（r22 ①）**：位流起点必须 4 字节对齐，而 `[头 4][树长 1][树 N][位流]` 的布局要求 **N 为奇数**；`(byte4+1)*2` 恒为偶数、永远不可能满足对齐，故取 `(byte4 << 1) + 1`。r17 ④ 的「订正」本身是错的 | — | **已消**（2026-09-30） |
| **L9** | **核心 `handle_swi_hle` 的 `0x08`–`0x0A` 三个 arm 现状各异，全部处置完毕**（r23 ①②④ → r24 → r25 ④ → r28 ①）。<br>· **`0x09` ArcTan：已认领**（r24），并于 r28 ① 修正取整顺序，**此前并未逐位对齐**。<br>· **`0x08` Sqrt：已验证为正确**（r25 ④），补 4 项锁测试（契约锚点 13 例、`0..=20000` 穷举 floor 语义、全部 65535 个完全平方数及两侧、GBATEK 取整例的 floor 语义）全部通过，**故确认不认领**。<br>· **`0x0A` ArcTan2：已认领**（r28）。原判「数学上大概率正确故可不认领」被推翻 —— mGBA 的实现是分象限调多项式的**整数算法、逐位对齐 BIOS**，而核心是 `f64::atan2` + π 重映射，轴向情形（`x=0` 或 `y=0`）差距明显。 | — | **已消**（2026-09-30，r28） |
| **L10** | **`BitUnPack`（0x10）的偏移溢出不做截断，按硬件行为照实实现。** 当 `源元素 + 偏移` 超出目标元素宽度时，多出的比特**溢出到同一字内的后续目标元素**（`gba` crate 描述此行为，mGBA 的实现证实它真的发生）。调用方应保证 `源宽 + 偏移 ≤ 目标宽` | 描述不自洽的输入会产生跨元素污染的输出 —— 与真 BIOS 一致，但真 BIOS 同样不保证什么。已加锁测试钉住该行为，**若将来改为截断即为行为变更**，须重新评估 | **GA 后**（若实测有工具产出此类数据） |
| **L11** | ~~**当前环境无法执行不变式 1 的 `ctest` 判据。**~~ → **r29 已消**：根 `CMakeLists.txt` 的 `add_compile_options` 加入 `/EHsc`，全量构建通过，`ctest` **33/34** 跑通。**唯一剩余失败是 `bench_tolerance_test`，且已证与 v2.0 无关** —— 它比对的是 v0.3.0 二进制的基线（2026-07-19），当前已到 v1.18，累计差距 +100% 以上；**在 `git stash` 后的干净树上复现出同一量级**（+113.72% / +101.90% / +113.74%），与加 `/EHsc` 时的数字一致 | 4 个测试目标恢复可编译，门禁重新可执行。**残留**：`bench_tolerance_test` 因基线跨 11 个版本而恒红 | 刷新 `tests/fixtures/bench_baseline.json` 至当前版本（**属性能基线治理，不在本阶段范围**）。另注：`ctest` 34 项是 F11QA `108P/12F` 的**本地近似**，真正的判据仍是 F11QA 矩阵 |
| **L12** | **~~不变式 1 自 2026-09-30 起处于「未验证」状态~~ → r29 已消**：S0'–S1c 六个阶段第一次同时具备「GBA 锁测试 132 项全绿」+「`ctest` 33/34」两份证据 | 无。**但须注意口径**：`ctest` 33/34 是**本地近似**，F11QA `108P/12F` 矩阵本轮**未复跑**（需商业 ROM 与 CI 环境）。本条关闭的准确含义是「NES 侧不再处于无证据状态」，而非「已完成 F11QA 验证」 | 后续阶段仍应按纪律 7 在每次 GBA 改动后跑 `ctest`；F11QA 矩阵的复跑留待 GA 门禁 |
| **L13** | **savestate 载荷是 `serde_json`，体积约为机器状态本体的 2.25 倍。** 一个 `Vec<u8>` 会被编成十进制数字的 JSON 数组。**⚠️ r48 实测订正了本条的两处说法**：原文「体积约为机器状态本体的**数倍**」偏小 —— 实测 **466,464 B 原始状态 → 1,047,799 B 文件 = 2.25 B/字节**；原文「解析耗时**以毫秒计**」本身没错，但据此推出的「**可感知的停顿**」是**在 Debug 下测的**，而产品是 Release：**编码 2.4 ms / 解码 5.3 ms**（Debug 对照 78 / 82 ms，慢 32×/15×）。**磁盘格式已定：维持 JSON**（用户裁决 2026-10-02），r30 悬而未决的「最终格式未定」至此关闭 | **「停顿」半条已消** —— Release 下 2.4/5.3 ms 远低于可见阈值，10 个槽位连续存读无感。**「文件偏大」半条仍成立且是本条的实质**：1.0 MB/槽，10 槽累计约 10.5 MB。**换格式的路仍在，且是唯一一条不改 vendor 补丁就能走的路**（核心字段私有、只有 derive 生成的实现，无法逐字段编码）：二进制 serde 约 466 KB（2.25×）、base64 约 622 KB（1.68×），代价是一个新依赖或一层自写编码。**r30 ② 已记录 postcard/bincode 对含六个 `Vec<u8>` 的状态编译不过** —— 走这条路前须先重开可行性验证 | **本条维持开放，但性质已变**：不再是「S3 之前必须消解」的阻塞项（r48 据实取消那个时点，因其唯一实质理由被实测证伪），而是「文件偏大，用户可感知但不阻塞」的长期取舍。**测量工具留在 `save.rs` 的 `measure_a_state_on_disk`（`#[ignore]`），格式若将来重议，数字随时可复现，不必重新推导** |
| **L15** | **钉住 RTC 期间时钟不前进。** `gba_rtc_set_time` 钉的是**固定时刻**而非偏移量，所以两次读数永远相等 | 靠两次 RTC 读数推算流逝时间的程序会读到 0。**这是有意的取舍**：两个用例（锁测试要一个自己说得出的日期、调试的人要一个自己挑的日期）要的正是冻住的时钟；要流逝就用宿主时钟，那本就是默认值。钉住的状态随存档一起走（字段已序列化），读档后仍是钉住的 | **不需要消解** —— 这是该功能的语义，不是缺陷。若将来出现「要流逝」的用例，再加一个偏移量模式，届时两条路径并存 |
| **L14** | ~~**`gba_reset()` 不复位核心**，它只把音频帧计数归零~~ → **r43 已消**（2026-10-01）：`gba_reset` 改为**取出当前 ROM 并重新建机**，即 `Machine::new` 的语义。寄存器、WRAM/VRAM、帧缓冲、音频环与相位累加器全部回到初始状态，SWI 钩子由与装载同一条路径重新装上，按键亦不复位。**口径要说准：这是上电复位，不是 BIOS `SoftReset`** —— SoftReset 只复位 CPU 状态并跳回 `0x00000000`、**不清内存**；选上电是因为 Reset 按钮在用户心智里是「重新开始」，且 FCEUX11 的 NES 复位也是硬复位。**代价是明确记下的**：依赖 SoftReset 保留内存的程序会看到不同行为 | — | **已消**（r43）。**这条消的是 L14 的事实陈述「机器根本没有复位」，不是把它改写成另一种说法** —— 复位按钮的语义选择（硬 vs 软）不单列为限制 |

---

## 十、不变式

1. **NES 核心零回归**：不得改变 F11QA 现有矩阵（`108P / 12F`，grade B）。`pass_to_fail` 必须为 0，出现即回滚。
   > **口径订正（2026-09-28）**：原文写 `106P / 14F`，那是 v1.16 的基线。
   > v1.18.1 修通 `kgmqa-056`、v1.18.2 修通 `kgmqa-078`，矩阵已由 106P/14F 推进到 **108P/12F**。
   > 本条是 S0 出口「NES 构建零回归」的判据，基线数字错了会把「本来就没有的 2 项 PASS」误判成回归。
2. **前端复用不打折**：不得为 GBA 另起 UI 框架。**允许在现有 widget / 驱动内为 GBA 保留一条独立的绘制分支，条件是 NES 分支逐字不动。**
   > **r41 改写的理由**：原文是「不得分叉 Qt 驱动」，它与 r41 的「最大化解耦」在「该复制还是该泛化」上直接拉扯 —— 而泛化恰恰是**把 NES 置于风险中的那件事**（`GL_NES_WIDTH/HEIGHT` 是编译期常量，改成运行时尺寸会动到 NES 的缩放数学，而 `video.ncol/nrow` 本来就会被 `CalcVideoDimensions` 改，两处今天就不一致，见 r41 ③）。「分叉」指的是**复制出去之后各自演化**；为一个 Alpha 模块**故意复制一小段**共享同一窗口、同一 renderer、同一纹理格式的绘制代码，不是分叉。**取舍时以解耦优先：宁可复制，不可泛化。**
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
9. **GBA 与 NES 解耦，且不接入任何质量检测系统**（用户裁决 2026-10-01）。
   - **长期 Alpha，不阻塞 FCEUX11 本体。** GBA 的进度、缺陷与半成品都不得成为本体发版的门槛。
   - **它不是 F11QA / jsmolka / 任何外部检测系统的对象。** 它的出口判据只有两条：`ctest` 不回归（33/34，`bench_tolerance_test` 的既有失败除外），以及它自己的锁测试。**这条同时解除了 S2 出口对 jsmolka 的依赖**（与 L4 / L7 的原口径不同，见 r41 ②）。
   - **任何为了 GBA 而改动 NES 既有行为的做法一律不做。** 宁可复制一小段，不泛化（见不变式 2 的 r41 改写）。
   - **红线**：NES 侧的帧池、纹理、缩放数学、调色板、调试器内存映射、cheats、TAS、NSF、录像**不得出现 GBA 引用**。GBA 会话下这些按不变式 7 **显式声明为不可用**，而不是让它们对着一个 GBA 机器给出 NES 语义的答案。
   - **允许的耦合只有这五条**（穷举）：① 同一进程 / 同一模拟线程 / 同一窗口 / 同一声卡设备；② 每帧循环里的那一个分支；③ `WriteSound` 写入接口（环按**设备**尺寸配，不按机器配）；④ 限速的基准帧率取值；⑤ 打开 ROM 对话框的过滤器条目。
   - **由 `gba::decoupling` 的两条守卫检查强制**（r41 ⑥）：C ABI 的调用点必须恰为一个文件；任何提到 GBA 的 C/C++ 文件必须在显式白名单内。**原则写进文档会被忘掉，写成检查不会。**

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
| 6 | **S2-b4 阶段 2'（GBA 画面）的具体改动点** | ✅ **形状已定，不需再决策**（r41 ④ / r39 ④）：GBA 自己的帧池 + 纹理 + 绘制，`render()` 里一处分支，**NES 分支逐字不动**。开工前必读 `ConsoleViewerSDL.cpp` 的 `render()` / `transfer2LocalBuffer` / `sdlTexture` 生命周期（`:346`） |
| 7 | **L6 的 SIO / 声音寄存器复位值表** | ⛔ **外部阻塞，取不到就不做**（r43 ②）。GBATEK 对本工具 403，镜像只给寄存器映射。**不得凭「大概复位为 0」实现** —— 按纪律 4 停下记录。取得该表即可直接续做 |
| 8 | ~~**S3 的 L / R 按键绑定**~~ | ✅ **最小形态已交付（r47）：L → Z、R → X，走原始 scancode 而非从 NES 手柄派生**，因为默认布局下 Z/X 已绑给 A/B，派生会让 L 变成第二个 A。**仍未做的是绑定表** —— 这两个键是硬编码的，改键要去「键位配置」那件工作。**刻意不新增一个设置对话框看不见的配置项**，那会变成第二个事实来源；临时改法写在 `gba_load.h` 的注释里。⚠️ 覆盖缺口：映射正确性无自动化测试（r47 ⑦，C++ 侧无夹具） |
| 9 | **「画面 / 音频正常」的取证手段** | ℹ️ **不再是门禁**（r41 不变式 9 / r42）。Alpha 阶段不接入 jsmolka，所以本环境**无法**取得这个证据，也不要求取得。**若将来 GBA 转正式版**，才需要重新讨论怎么验（§8.1 P3 已改注） |

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

| **r33** | 2026-09-30 | **S2-c 自建门禁：✅ 完成。5 条 SWI 往返在真实 ARM 指令流下全部通过。**<br>**① 本条取代 r33 的初稿 —— 那一版记的「未完成」是错的。** 我当时把「`r0` 回来是 PC 值」这个症状记成未解，但那是在**带 `STR` 的旧版本**上观察到的；重写成寄存器方案后症状不复存在，我却没重跑就归档。**记一个没验证过的症状，等于记一个结论。**<br>**② 真正的根因有两个，都不在 SWI 通路上**：<br>· **`B .` 收尾不可用。** ARM 三级流水线下，位于 `P` 的分支以 `P`/`P+4`/`P+8` 三步为周期循环，**PC 从不在连续两步间重复**，「PC 不动了」这个判据本身不成立 —— 前两稿都栽在这里。改用 **`Halt`（SWI `0x02`）收尾**：它是我们自己认领的号码，机器真会停下，判据无流水线歧义；顺带让门禁自己也用上一条认领的 SWI。<br>· **`program` 辅助函数多塞了一条 `B .`**，于是 `run` 追加的 `Halt` 排在它后面，永远执行不到。**定位它的不是读代码，是「手写 body 的两条测试过了、用 `program` 的三条没过」这个不对称** —— 一致性异常比断言失败更有信息量。<br>**③ 门禁现在验的是**：`Div`（我们**不**认领、走核心自己的 arm，证明拒绝路径正确）、`Sqrt`（同样走核心）、`ArcTan` 的契约锚点 `atan(1.0) = 0x2000`（r28 修过的那个 1 单位差就在这里）、`ArcTan2` 的轴向精确常量 `0x4000`、`GetBiosChecksum` 的 retail 值；**外加一条独立的「卡带确实拿到控制权」测试**，与 SWI 到达分开断言。**程序形态收敛到 `MOV` + `SWI` + `Halt` 三条指令。**<br>**④ 三轮变异验证**：Halt 号码改错 → **5 项红**；`program` 加回 `B .` → **恰好 3 项红，且只有用 `program` 的那三条**（重现了当初定位 bug 的不对称）；`ArcTan` 号码 +0x10 → **恰好 1 项红**。<br>**⑤ r33 初稿记下的真收获仍然成立**：ARM 立即数规则已验证（纠正我三处错误推导），`mov_r0_imm` 对不可编码值 panic 而非静默产生另一条合法指令。**⚠️ 但本条末尾「stub BIOS 不执行卡带头 0x00 的分支，而是从字面量池读地址，`0xC0` 必须放入口地址字面量、代码从 `0xC4` 起」是错的，已由 r49 推翻并订正 —— 它记的不是「真收获」而是一个被假设出来的约定，而本条正是它被写进代码的源头。**<br>**⑥ 锁测试 160 → 166**，`cargo check` 两态通过，Debug 与 Release(fat LTO)下门禁均全绿。**至此，S0–S2a 的 16 个认领号码第一次在「真实 ARM 指令流」这一层上取得证据** —— 此前所有证据都来自直接调用 `dispatch` 的锁测试，而 r25、r28 两个最严重的缺陷恰好是自建测试全绿、靠外部参照才发现的。 | 诊断 + 实施 + 三轮变异验证 |

| **r32** | 2026-09-30 | **S2-a 落地：生命周期 + 帧缓冲 + `BETA` 水印。实施中抓到两个真缺陷，其中一个是内存安全级别。**<br>**① 第一个缺陷：`gba_rom_loaded` 的返回值语义反了。** 它复用 `with_machine`，而该辅助函数「闭包返回什么就返回什么」；我原先在闭包里写 `GBA_OK`（=0），于是**「已加载」报 0、「未加载」报 1 —— 完全颠倒**。两个值都是「看起来合理的整数」，编译器不会报，运行时也不会崩，C++ 侧只会拿到一个反的布尔。已修，并补 `loaded_reports_one_and_ok_reports_zero` 把「布尔是 1、状态是 0」这条约定写死。<br>**② 第二个缺陷更严重：`static mut` 存机器导致 `STATUS_HEAP_CORRUPTION`。** 计划的「仅模拟线程」是**对 C++ 调用方的承诺**，不是对 Rust 侧的保证；而测试 harness 并行跑用例，多个用例同时装卸 `Box<Gba>`，其中 `rtrb` ring buffer 的所有权跨 `Box` 移动 → 堆损坏（进程 `0xc0000374`，单跑某条测试则通过）。**这是本项目第一次因「并行测试 + 全局可变状态」吃到的内存级事故**，已改为 `Mutex<Option<Machine>>` + `OnceLock`。锁在真实使用中无竞争，帧路径零开销；而**「仅模拟线程」从一句断言变成了代码实际遵守的性质**。<br>**③ 由此又暴露第三点：锁测试需要「安全」之外的「隔离」。** 互斥锁让并发访问不再损坏内存，但用例之间仍会**逻辑干扰** —— 关水印那条会读到另一条用例刚取的帧。加 `exclusively(|| …)`（专用线程 + 专用锁）把 9 条用例串行化。**「safe」与「isolated」是两件事**，这是本轮的第二条教训。<br>**④ 订正 r30 ④ 的错误数字：`Arm7tdmi` 实测 82,032 字节，不是「接近 1 MB」**（差约一个数量级）。76,800 是 240×160×2 的帧缓冲，占九成；WRAM/VRAM/ROM 那些 `Vec` 是**堆分配**，不在结构体里。已加 `gba_probe_cpu_size` 与 `the_machine_state_is_too_large_to_return_by_value`（阈值 64K）随时可查并钉住。<br>**⑤ 三轮变异验证均精确命中**：`gba_rom_loaded` 改成返回 0 → **2 项红**；水印绘制短路成 `false` → **1 项红**（证明「水印真的在帧路径上」被验住，否则编译掉 `overlay::draw` 也不会有人发现）；容量守卫去掉 → **1 项红**。<br>**⑥ 实施期手算错误四处**（纪律 5）：水印边界断言把「4 字形 × 6 字距 × 4 缩放」算成 34（实为 96）；`lit` 计数下限凭空取 1000；测试用 `p[3] != 0` / `!= 0xFF` 判水印，而实现是 alpha **240**（不是 0，也不是非 255）；`bios::image()` 实为 `bios::stub()`。<br>**⑦ 锁测试 145 → 156**（水印 8 + 帧/生命周期 12 + 状态大小 1），Debug 与 Release(fat LTO) 下均全绿，两态 `cargo check` 通过。**S2-a 的 §4.1 导出尚未进 staticlib**（`frame.rs` 的函数已是 `#[unsafe(no_mangle)] extern "C"`，但 CMake 全量构建与 `ctest` 尚待复核）。 | S2-a 实施 + 三轮变异验证 |

| **r31** | 2026-09-30 | **S2 的 C ABI 形状定案：沿用本仓库既有的「出参 + `len`/`cap` + 显式 free」约定，不自创。**<br>**① 这个问题本仓库已经解决过 20 多次。** `fceux11_rust` 跨 C ABI 传大对象一律用 `#[repr(C)]` 描述符 + `Box::into_raw` + `std::mem::forget` 转移所有权 + 配套 `*_free`（见 `fceux11-core/src/state_file.rs:420` 的 `FceuStateChunkOutput { data, len, cap }`、`:538` 的 `state_file_load` 五个出参、`:562` 的所有权转移）。**先查本仓库已有约定，再设计新 ABI** —— 否则会在同一个代码库里并存两套大对象传递风格。<br>**② 由此解决 r30 ⑤ 的栈约束，且方案优于「让调用方保证栈大小」。** 备选是要求 C++ 侧提供 16 MB 栈，那是**谁都不会遵守的隐式契约**；定案是 **`gba_savestate_load` 只往我们自己的全局状态里写，永远不把 `Arm7tdmi` 交给 C++ 侧**。于是 `extern "C"` 帧上只有指针运算，那 82 KB 从不经过调用方的栈。调用方唯一需要知道的规则是「给一个够大的 buffer」，这是能自证的。**实测的 82,032 字节由 `gba_probe_cpu_size` 随时可查，锁测试钉住其 > 64K**（若将来核心瘦身到可以做按值返回，本条即可复议）。<br>**③ 定案的形状**：`gba_savestate_size(out_size)` → `gba_savestate_save(dst, cap, out_written)` → `gba_savestate_load(src, len) -> i32`（原地替换内部状态）→ `gba_savestate_free(ptr, len, cap)`。<br>**④ `cap` 必须外传，这不是冗余。** `Vec::as_mut_ptr` 交出去后用 `from_raw` 回收，若只传 `len` 而不传真实的 `capacity`，释放走的就是错误布局。`state_file.rs:562` 正是为此同时给出 `len` 与 `cap`。<br>**⑤ free 用 `drop(Box::from_raw(..))` 而非 `Vec::from_raw_parts(ptr, len, len)`** —— 后者在 `cap > len` 时是未定义行为。沿用既有实现，不重新发明。<br>**⑥ 帧缓冲与音频天然同形**：§4.1 的 `gba_frame_buffer(dst, len)` 与 `gba_render_audio(dst, cap, out_frames)` 本来就是出参。**S2 不新增任何 ABI 约定**，这正是走既有模式的收益。 | 本仓库既有 FFI 模式调研 |

| **r30** | 2026-09-30 | **S2 开工前的 savestate 格式验证：postcard 被否决（推翻本轮我给出的推荐），并挖出一条影响整个 S2 的硬约束。**<br>**① 调查前提修正：R14 的触发条件不是「引入外部 crate」，而是「**path** crate 夹在根 crate 与大依赖之间」。** r8 的六组对照实验里唯一变量是「是不是独立 crate」，与依赖关系无关；且 `serde` / `serde_json` **在本 workspace 的依赖图里早已存在**（`f11qa` 依赖 `serde_json`），此前构建正常。故「引入序列化依赖会不会触发 R14」这个担心本身是伪问题 —— 我在给用户的选项里写「需先做可行性验证」，验证的结论是：**不触发**，`lto = true` + `panic = "abort"` 下 Release 全量构建与测试均绿。<br>**② 但 postcard 本身不能用 —— 推荐错了。** 它是 **no-alloc** 格式，而核心状态里有**六个 `Vec<u8>`**：`bios_system_rom`、`rom`、`working_ram`、`working_iram`、`sram`（均在 `InternalMemory`）与 `Rtc::buffer`。`postcard::to_allocvec(&gba.cpu)` 对 `Arm7tdmi` **根本编译不过**。这是格式的固有性质，不是版本或 feature flag 能解决的。**这与 r13 / r17 / r22 是同一类错误的第四次**：我在选项里按「二进制、紧凑、维护中」这些**表面属性**推荐了一个格式，没有先问「它能不能表达本项目的状态」。bincode 1.x 有同类问题，只是没那么明显。<br>**③ 改用 `serde_json` 并实测往返。** 零新增依赖（`f11qa` 已在用）、会分配、且**实测**一个真实 `Arm7tdmi` 往返后 `r0` 与 `pc` 完全一致。代价如实记下：对热路径上的磁盘文件体积大、速度慢 —— **最终磁盘格式仍未定**，本轮只服务第一版。<br>**④ 一条影响整个 S2 的硬约束（新发现）**：`Arm7tdmi` 内嵌整个 `Bus`，**实测 82,032 字节**（`gba_probe_cpu_size`；其中 76,800 是 240×160 的帧缓冲，占了九成以上 —— WRAM/VRAM/ROM 那些 `Vec` 是**堆分配**，不在结构体里）。⚠️ **本条初稿写的是「接近 1 MB」，错了约一个数量级**，已按实测订正。Rust 测试线程默认 2 MB 栈，serde 生成的 `Deserialize` 在栈上构造该值，加上调用侧同时持有一份 → **`STATUS_STACK_OVERFLOW`**。`Box` 解决不了：移动本身也要栈，且 serde 内部帧仍是大栈帧。锁测试因此显式跑在 **16 MB 栈**的线程上。<br>**⑤ 这不是测试写法问题，是 C ABI 的实际约束。** S2 的 `gba_savestate_load` 会撞上同一堵墙：`extern "C"` 帧由调用方（C++ 侧）提供栈，而 `panic = "abort"` 下又不能靠 unwind 兜底。**已把该约束写进 `src/gba/save.rs` 的模块注释**；处置方案见 r31（沿用本仓库既有的出参 + `len`/`cap` + free 约定，不让 82 KB 的状态跨 `extern "C"` 返回）。<br>**⑥ 本轮只做地基，未写 §4.1 的 C ABI 导出。** 交付的是 `src/gba/save.rs`：4 条锁测试（头部标识、跑过的机器寄存器往返、hook 不跨存档、外来文件与未来版本被拒）。S2-a 的帧缓冲/音频/生命周期仍未开工。 | 格式可行性实测 + 栈约束实测 |

| **r29** | 2026-09-30 | **修复 L11 / L12：全仓加 `/EHsc`，`ctest` 门禁恢复。S0'–S1c 六个阶段第一次拿到 NES 侧证据。**<br>**① 根因不是「4 个目标漏了标志」，而是「全仓从未声明过 `/EHsc`」。** MSVC 默认**不启用** C++ 异常展开语义，而用到它的标准库头（`__msvc_ostream.hpp`、`<chrono>`）在实例化时发 `C4530`。主工程从未触发，是因为它的编译单元**消费 PCH**（`build.ninja` 的 `/Yu` + `cmake_pch.hxx`），PCH 自身带 `/EHsc`；4 个**裸 `add_executable`** 的目标既无 PCH、又无 Qt 依赖可继承，于是在自己的编译里直接实例化那些头。MSVC 升级前不告警，**14.51 起把 C4530 升格为 `/WX` 错误**才暴露 ——「升级才坏」与「一直缺失」是同一件事的两面。**主工程同样零处 `/EHsc`**，此前是碰巧继承来的。<br>**② 因果已用最小复现坐实，不是「改完碰巧好了」**：5 行探针（`#include <iostream> <chrono>` + 一次 `steady_clock::now()`）加 `/W4 /WX` —— 不加 `/EHsc` 复现 `chrono(2387): C4530` → `C2220`，加上则干净通过。**这正是本项目的判据纪律：因果必须被证，不能靠改完就绿来推定。**<br>**③ 处置（用户裁决：目录级而非局部）**：在根 `CMakeLists.txt` 的 `add_compile_options` 里与 `/W4 /WX /sdl` 并列加 `/EHsc`。理由是这条缺失**从来不是 4 个目标的属性，而是全仓口径缺失**；局部补则下一个裸目标原地再踩。注释写清完整因果链与版本号，把「靠继承碰巧拿到」变成「显式声明」。<br>**④ 全量构建通过，`ctest` 33/34。** 唯一失败 `bench_tolerance_test` **已证与本改动无关**：它比对 v0.3.0 二进制的基线（2026-07-19），当前 v1.18 累计差距 +100% 以上；`git stash` 后干净树复现同一量级（+113.72% / +101.90% / +113.74% vs 加 `/EHsc` 时 +114.47% / +100.91% / +118.20%）。**刷新基线属性能治理，不在本阶段范围。**<br>**⑤ 口径必须说准，否则就是另一种形式的静默放宽**：`ctest` 34 项是 F11QA `108P/12F` 矩阵的**本地近似**，不是该矩阵本身。本轮 F11QA 矩阵**未复跑**（需商业 ROM 与 CI 环境）。故 L12 关闭的准确含义是「NES 侧不再无证据」，**不是「已完成 F11QA 验证」**。这两句话的差别，是本项目纪律 2 要防的那类事。<br>**⑥ 本条改的是 NES 侧构建配置，非 GBA 代码**，不在 v2.0 授权范围内，**是用户明确指示后才动的手**。② 的口径变更影响全仓编译标志，属工具链层级决策，不该由 GBA 阶段顺手带过。 | 用户裁决 + 最小复现 + 干净树对照 |

| **r28** | 2026-09-30 | **S1c-e：认领 `0x0A` `ArcTan2`，S1c 收尾。实施中挖出 `0x09` 的一处位级缺陷并修复 —— 这是本项目最隐蔽的一次自查。**<br>**① `0x09` 此前并未逐位对齐 BIOS。** mGBA 的 `_ArcTan` 首个 Horner 项是 `-((i * i) >> 14)`：**先移位,后取负**。r24 的实现写成 `-(i * i) >> 14`，**先取负,后移位**。算术右移向负无穷取整，故两种写法在 `i * i` 不是 2¹⁴ 倍数时**恰好差 1**（几乎总是）。**这个差别不会崩、不会越界、精度仍在 2 单位内，只会让每个结果差 1 个单位** —— r24 的全部测试（契约锚点、2 单位精度、范围、单调性）**全部照绿**。直到 r26 取到 mGBA 全文、逐行比对 Horner 链才暴露。<br>**② 更深一层：取「绝对值再最后取负」也不对。** 那种写法是**严格奇函数**，而 BIOS 不是 —— 它在有符号输入上运算、由算术移位决定取整方向。这条不对称会被 `arctan2` 的八个象限继承，故**折叠掉它就会污染 `arctan2`**。已改写为有符号运算，并把「奇对称仅在取整方向上成立」写成性质测试。<br>**③ 副作用是好的**：wobble 点从 14 处降到 6 处（正负各 3）。旧写法在负半轴上的额外抖动**不是硬件的**，是自己造的。<br>**④ `0x0A` 实施中抓到第二类错误 —— 两个函数的标度不同**。`arctan` 的 45° 是 `0x2000`，`arctan2` 的 90° 是 `0x4000`；我在象限偏移里误用了 `QUARTER_PI`，结果答案**落在错误的象限而不是越界**。变异验证：换回去 → 4 项转红。教训与 r26 ② 同型 —— **符号/标度这类量，锁测试必须按「从契约推」而不是按「从另一个函数搬」来写**。<br>**⑤ 一个等价变异，如实记录**：删掉 `arctan2` 的 `x == 0` 短路后，**全部测试仍然全绿**。查证后确认这是**等价变异而非逃逸**：`x == 0` 时比值为 0、`arctan(0) = 0`，第二/第六象限分支本就求值到同样的 `AXIS_POS_Y` / `AXIS_NEG_Y`。短路保留的理由是「硬件这么做」而非「不这么做会错」，已写成测试把这个冗余固定下来，免得下一个���重新推导一遍。<br>**⑥ S1c 状态**：认领集 16 个（`0x08` 是唯一**验证过**不认领的），锁测试 89 → **132 项**，`cargo check` 两态通过，补丁集仍 9 处。**出口判据「mode 7 与精灵缩放样例通过」需帧缓冲，只能在 S2 取得** —— 与 jsmolka 门禁同属「判据自身依赖尚未交付的能力」，处置方式同 r11。**NES 零回归仍未取得（L11 / L12）。** | 实施 + 三轮变异验证 |

| **r34** | 2026-10-01 | **S2-b 开工前复核：三条裁决 + 两条据实订正。NES 零回归证据与 S2-a 的 ABI 面首次双双闭合。**<br>**① 先补上一轮欠的证据。** r32 ⑦ 自认「S2-a 的 §4.1 导出尚未进 staticlib，CMake 全量构建与 `ctest` 尚待复核」—— S2-a 新增 15 个 `extern "C"`、S2-c 又加门禁，**两次都没跑过 `ctest`**，即纪律 7 要求的「每次 GBA 改动后跑 `ctest`」在最近两个阶段是断的。本轮实测：全量 Release 构建 `BUILD_EXIT=0`；`ctest` **33/34**，唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`（L11 口径）；GBA 锁测试 **166/166**。S2-a 的导出符号经 `dumpbin /linkermember:1` 确认全部进入 CMake 实际链接的 staticlib，`fceux11_rust.h:4059-4076` 同步声明。**r32 ⑦ 的两半都闭合。**<br>**② 据实订正一：导出函数是 16 个，不是 15。** §八 S2 行与 r32 写的「15（5 探针 + 10 生命周期/帧）」少数了一个 —— `ffi.rs` 实际导出 **6** 个（第 6 个是 r30 ④ 才加的 `gba_probe_cpu_size`），`frame.rs` 10 个。**成因值得记：我自己在复核时先信了记录、没数文件，写出「15 个，与 r32 记录吻合」，被计数打脸后才回头查。数字要自己数。**<br>**③ 据实订正二：§4.1 的 `gba_set_overlay` 签名自相矛盾。** 第 247 行写 `void gba_set_overlay(int enable)`，同一行的注释却要求「发布构建中 `enable=0` 返回 `GBA_ERR_STATE`」—— `void` 不可能返回错误码。S2-a 按 `i32` 实现（`frame.rs:273`），生成头也是 `int32_t gba_set_overlay(int32_t)`，**只有计划没跟上**。不变式 5 要求它返回 `GBA_ERR_STATE`，故 `i32` 是对的，§4.1 的 `void` 是错的。<br>**④ 三条裁决（用户 2026-10-01）。**<br>· **ABI 补两个 setter**：`gba_set_output_rate(uint32_t)` + `gba_set_volume(uint32_t)`。**依据**：核实 SDL 实际速率是**用户可配**的 {11025, 22050, 44100, 48000, 96000}（`sdl-sound.cpp:60,256`），硬编码 44100 一定错；音量标度是 **0–150**（`fceu.cpp:612`）且 `WriteSound` **确实不施加音量**（`sdl-sound.cpp:425` 直接 `s_Buffer[i] = *buf`），故 §4.2/§4.3 要求我们自己乘是对的，但 ABI 里原本没有地方把它送进来。<br>· **两层分工，不做二选一**：核心的 `cycle_accumulator` 已是 cycle-accurate 的分数累加（「抽干环」本身就无漂移），但**我们这层仍要自己的相位累加器** —— 它给出的「本帧应产出多少」是核心给不了的东西，**两者之差就是 underrun 计数**，§8 的性能判据「underrun 计数为 0」才可测。<br>· **jsmolka 门禁暂不处理**，记为 **S2 出口前的阻塞项**（见 ⑤）。<br>**⑤ jsmolka 在本环境不可执行，是「做不了」不是「还没做」。** 三重阻塞：本机无 FASMARM（r11 已记）、零 `.gba` 资产（`tests/fixtures/` 下只有 NES 夹具）、`.github/workflows/` 三个文件里无 jsmolka。**且即便有了 ROM 还差一环**：jsmolka 只用**屏幕文字**报通过/失败，判据需要自写 mode 4 文字读取器。**用户裁决：先做 S2-b，此项记为 S2 出口前必须解决的阻塞项**，L4 / L7 的消解条件仍指向它。<br>**⑥ 音频链路的取证结论（全部带出处，非引述本文件）。**<br>· `Gba::init_audio(rate, cap) -> rtrb::Consumer<f32>`，`cap` 单位是 **f32 槽**、每立体声帧 2 槽（`gba.rs:111`）。<br>· 样本是**交织立体声 f32 ∈ [-1,1]**，经 DC 阻挡后 clamp；**L/R 成对推或都不推**（`sound.rs:640-648`）—— 只推一半会让左右声道在其后互换。<br>· **环满时静默丢弃**：`if out.slots() >= 2`（`sound.rs:644`）。丢样本完全不可见，这是「必须自己计 underrun」的硬理由，也是 R12 漂移风险的物理来源。<br>· `Bus::take_audio_out` / `restore_audio_out` **已存在**（`bus.rs:1317/1322`）⇒ **S2-b2 的「savestate load 后重挂 `init_audio`」（N-10）不需要新增 vendor 补丁**。<br>· **一个必须避开的陷阱**：`sdl-sound.cpp:263` 的 `samplesPerFrame = (int)(s_SampleRate / getBaseFrameRate())` 看着就是本项目要实现的东西，但它是**截断取整**，且只用于在 ≥1024 时把 `spec.samples` 抬到 1024 —— 是**缓冲区尺寸提示**，不是每帧权威样本数（权威值是 `FlushEmulateSound` 的 `end`）。**照抄这个截断就是 R12 说的那个坑。**<br>**⑦ 无漂移公式（本轮自算，非引述 §4.2）。** 帧长 = 228 × 1232 = **280,896** 周期（`lcd.rs:27` 注释可对），故每帧样本 = `rate × 280896 / 16777216` = **`rate × 4389 / 262144`**（分子分母同除 64，分母仍是 2 的幂）。相位累加 `phase += rate×4389`、`n = phase >> 18`、残余结转。**证明无漂移**：连续 2¹⁸ 帧总产出 = `(2¹⁸ × rate×4389) >> 18` = `rate × 4389`，而 2¹⁸ = 262144 帧正好是 `262144 × 280896 / 16777216` = **4389 秒** —— 恒等于 `rate` 样本/秒。44100 时每帧 738.3534，与 §4.2 的 738.35 相符。<br>**⑧ 实施期的一个真实陷阱（写进 §4.3 的隐含前提）。** 转换链的饱和上界必须取 **±32767 而非 ±32768**：`WriteSound` 的契约幅度是 ±32768（§4.2 表格），但 SDL 回调把 `int32` 转 `AUDIO_S16SYS` 时，32768 会绕成 **-32768** —— 满刻度正弦在每帧边界「啪」一声极性反转。已列为锁测试项。 | S2-b 开工前只读复核 + 三条裁决 |

| **r35** | 2026-10-01 | **S2-b1 落地：分数采样音频通路。实施中查清一件此前无人测量的事 —— 稳态下到底漂不漂。**<br>**① 交付面。** 新增 `src/gba/audio.rs`（`SampleClock` + 转换链 + 5 个 C ABI 符号），`frame.rs` 的 `Machine` 持有 `AudioOut`，`gba_step_frame` 在到达 VBlank 后抽干。**根 crate 新增 `rtrb = { optional = true }`**：`Gba::init_audio` 返回 `rtrb::Consumer<f32>`，而消费端必须存在我们这侧，根 crate 要能命名该类型。挂在 `gba` feature 下 —— **不变式 8 用 `cargo tree` 严格复核：NES-only 依赖图里 `gba-core` 与 `rtrb` 出现 0 次**（此前只验过「能编过」，那是更弱的一档）。<br>**② 三个实施期发现的陷阱，都写进了代码注释而不只是测试。**<br>· **`ReadChunk::as_slices` 的两半不是左右声道。** 它在**环的回绕边界**处切分（`first_len = n.min(capacity - head)`），不是对半。把它当 `(lefts, rights)` 读，在不跨回绕时完全正常，一跨回绕就换声道或越界。**正解是 `ReadChunk::into_iter()`**：按逻辑序迭代，且 drop 时自动 commit。**槽位必须释放**（`commit_all` 或迭代器）否则环会被永久占死 —— 这一条漏了不会有任何报错。<br>· **饱和上界必须是 32767 而非 32768**（r34 ⑧ 已记，此处为实施确认）：`WriteSound` 契约写 ±32768，但 SDL 回调转 `AUDIO_S16SYS` 时 32768 绕成 -32768。<br>· **`f32::clamp` 会传播 NaN**，而 `as i32` 把 NaN 变 0。转换链改为显式比较上下界再 cast，NaN 落到 fall-through 成为静音。<br>**③ 查清「稳态到底漂不漂」—— 这是此前从未有人测过的事，也是 R12 唯一说得清的口径。** 端到端测试第一次跑就红：欠载 **220** 个样本。**算术立刻定位**：`738 - 220 = 518`，而 `518/738 = 0.7019 = 160/228` —— **建机后第一帧天然只跑了 70%**：模拟从帧顶开始，而核心在**第 160 条扫描线（共 228）** 报 VBlank，故首次抽干只经过 197,120 周期而非满帧 280,896。这是**启动瞬态**。<br>**④ 但我第一版结论是错的，随后被自己的数据推翻。** 我一度判定「稳态下核心每帧比 280,896 少约 47 周期，是系统性漂移」。**那个理由是错的**：`cycles_count >> 2` 的截断只可能让帧**变长**（第 k 个 LCD tick 在 `cycles_count >= 4k` 触发），理论方向与「少产」相反 —— 理论与实测矛盾，说明推理有错。**改测环内余量的逐帧趋势**（100 帧）：<br>`frame 1: underruns=220 pending=0` → `frame 20: 221 / 0` → `frame 100: 221 / 2`<br>**`underruns` 在 221 处收敛持平，环稳定在 2 槽，80 帧零增长。** 精确解释：`W_k - C_k ≈ (280896-197120)×44100/16777216 ≈ 220.2`，**与帧数无关的常数**。**不是漂移。** 若真是 R12 那种漂移，计数会正比于帧数增长、数分钟内抽干整个环。<br>**⑤ 因此 §8 性能判据的口径据实收紧。** 原文写「音频 underrun 计数为 **0**」——**在本实现下字面不可达**：每次载 ROM 都有一次 220 样本的启动瞬态。改为「**首次抽干之后计数不再增长**」，并说明 30 分钟实跑仍是 S2 的人工出口判据（100 帧是单元测试能诚实断言的量级）。**出口判据的改写必须记账，故记于此。**<br>**⑥ 变异验证 15 处，全部实测转红。** 覆盖：`advance` 改成硬编码整数帧长（`sdl-sound.cpp:263` 那个截断）/ 丢残余结转 / `ratio` 不约分 / 上界改 32768 / 音量平方 / 短帧截断 / `as_slices` 当左右读 / 读奇数槽 / `render` 先写后查容量 / `reset` 不清计数 / 环容量守卫收紧 / 新机忽略已记录速率 / 接受 rate 0 / 无机器时 `render_audio` 报成功 / `gba_reset` 不重置音频。<br>**⑦ 变异验证抓出我自己两条空测试。** 首轮 11 处里有 **2 处没转红**：`a_short_ring_is_padded_with_silence_and_counted` 用的环**是空的**，走 `take == 0` 提前返回，被变异的补静音那几行**根本执行不到**；`draining_leaves_a_whole_number_of_stereo_frames` 环里样本**不足一帧**，`read_chunk` 直接失败、什么都没读，剩余槽数自然是偶数。**两条都改成「环里有样本但不够一帧」**（后者还必须让 `want < 2×持有量`，否则「读 slots/2」与「读 slots」两种读法恰好取到同一个值，测试又会空转）。**这就是纪律 3 的价值：全绿不算证据，而这两条当时确实是绿的。**<br>**⑧ 顺带修正一处既有事实。** `gba_reset` 此前是**空壳** —— 返回 `GBA_OK` 但完全不碰 CPU。本轮只让它清空音频帧账（`Arm7tdmi` 的真复位要 `SoftReset` 语义，属 S2-b2）。已在函数注释里写明，不再让它假装已实现。<br>**⑨ 门禁。** 锁测试 166 → **185**（audio.rs 13 + frame.rs 6）；`cargo check` 两态通过；不变式 8 经 `cargo tree` 复核为 0 次；全量 Release 构建通过、`ctest` 33/34（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`）。 | S2-b1 实施 + 15 处变异验证 + 趋势实测 |

| **r36** | 2026-10-01 | **出口核验时发现：S2-b1 的 5 个导出「在库里、不在头文件里」；而本该拦住这件事的守卫测试是无效的。两条都修。**<br>**① 症状。** staticlib 经 `dumpbin` 确认含全部 **21** 个 `gba_*` 符号（含新增 5 个），但 `src/rust/fceux11_rust.h` 仍是 16 条声明、mtime 停在改动之前。**C++ 侧能链接、不能调用** —— 与 r9 记的 S0'/S2-a 同型，但更深一层。<br>**② 成因一：`build.rs` 的 `rerun-if-changed` 漏了 `src/gba`。** GBA 的 C ABI 在 `build.rs` 里是**手写**的（`build.rs:266` 起，注释写明 cbindgen 只跑成员 crate、从不指向根 crate），而 `rerun-if-changed` 列了 `crates/gba-core/src` 等七条路径，**唯独没有 `src/gba`**。cargo 在脚本声明了任何 `rerun-if-changed` 后就不再用「包内任何文件变化都重跑」的默认行为 —— 于是**在 `src/gba/` 加导出函数根本不会触发 `build.rs`**，头文件不重生成。**这与 AGENTS.md 坑 6 同族但更隐蔽：坑 6 是「改了不重编」，这个是「重编了但派生产物没跟着重生成」。** 已补 `rerun-if-changed=src/gba`。<br>**③ 成因二（更严重）：守卫测试 `the_gba_c_abi_is_declared_for_every_exported_function` 当时是无效的。** 它的 `EXPORTED` 是**手写清单**，而它比对的 `fceux11_rust.h` 也是由**另一份手写清单**生成的 —— **两边同源地漏掉新导出，于是互相抵消、断言照样绿**。这正是 r25 记的「对称的 bug 永远绿」的第五次现身。**讽刺的是该守卫的注释写着它就是为了防这次事故**（「That is exactly what happened to the whole of S2-a」），而 S2-a 那 10 个导出当初就是这么溜过去的 —— 守卫补在事后，且补成了同样的形状。**今天的 5 个音频导出如果照原样提交，会以完全相同的方式溜过去。**<br>**④ 处置：守卫改为从源码发现导出，而不是手写。** 扫 `src/gba/**/*.rs`，以 `#[no_mangle]` 为锚（测试辅助函数与 `use` 别名都没有该属性），取出其下一行的 `fn gba_*`。左侧变成「现实」，右侧仍是「磁盘上的头文件」，于是断言不再是同义反复 —— **它还能顺带抓住 ② 那种「头文件只是过期」的情况**。并加一条 `!names.is_empty()` 断言：**扫描器本身坏掉时守卫必须转红，而不是变成一个检查零件事的空断言。**<br>**⑤ 变异验证（新守卫确实有效）。** 从 `build.rs` 抽掉 `gba_render_audio` 的声明行 → 守卫转红并**点名该函数**。**旧的手写清单版本在这一步会全绿通过。**<br>**⑥ 顺带修正记录。** 修 ② ③ 时我**先改文件、后记入本表，与纪律 2 的顺序相反。理由是它直接阻断 S2-b1 的交付面（导出不可调用），且未改动任何出口判据或不变式；**但顺序仍应记下**，以免下次的「先改后记」被当成惯例。<br>**⑦ 复核。** 扫描器发现 **21** 个导出，与 staticlib 的 21 个符号逐一对应；头文件 21 条声明、mtime 随之更新。锁测试仍 **185** 全绿；两态 `cargo check`、不变式 8（`cargo tree` 计 0 次）、全量 Release 构建、`ctest` 33/34 均复跑通过。 | S2-b1 出口核验：派生头文件与守卫双重缺陷 |

| **r37** | 2026-10-01 | **S2-b2（savestate 导出）开工：只读调查推翻三处既有记载，第 0 步把栈峰值实测出来，五条裁决落地。**<br>**① 地基比 r30 ⑥ 记的薄 —— `load()` 返回的是一台没有 ROM、没有 BIOS 的机器。** `InternalMemory.rom` 与 `bios_system_rom` 是 `#[serde(skip)]`（`internal_memory.rs:131,150`），`Deserialize` 代入 `Default` 即空 `Vec`；而 `Gba` 在 `cpu` 之外还有 `cartridge_header`（**无 derive、不可序列化**，必须由 ROM 重建）与 `disasm_rx`。r30 ⑥ 的 4 条锁测试**只验寄存器，恰好验不到这一条**。<br>**② 第 0 步实测：r30 ④ 与 r31 ② 的共同前提是错的。** 两者都建立在「82,032 字节」这个数上，r30 ④ 更写「Rust 测试线程默认 2 MB 栈…serde 生成的 `Deserialize` 在栈上构造该值…`STATUS_STACK_OVERFLOW`」，并据此让锁测试跑在 16 MB 栈上。**实测（`GBA_PROBE_STACK_BYTES` 驱动、二分；栈溢出是进程硬中止 `0xC00000FD`，进程内无法捕获，只能从外部驱动）**：<br>`debug`：1 MB 溢出 / 2 MB 溢出 / 4 MB 溢出 / 6 MB 溢出 / 8 MB 过<br>`release`（`panic=abort` + fat LTO，即生产构建）：**1 MB 溢出 / 1.5 MB 溢出 / 2 MB 过** / 4 MB 过 / 8 MB 过<br>主导项是 **`Lcd::buffer`：`[[Color; 240]; 160]`，76,800 字节内联在结构体里**（`lcd.rs:160-161`），经 `serde_as` 的数组适配器在栈上物化。**峰值比结构体本身大 25 倍**，比 r30 ④ 想象的还大。<br>**③ 两个连带订正。**<br>· **`save.rs` 原注释的 serde 结论是错的。** 它写「`Box::deserialize` 不会把机器物化在本帧上，故选它而非 `from_slice::<Arm7tdmi>`」—— 查 serde 1.0.229 源码（`serde_core-1.0.229/src/de/impls.rs:1954-1958`），`Deserialize for Box<T>` 就是 `forwarded_impl!{(T), Box<T>, Box::new}`，**先按值在当前栈上构造 `T` 再装箱**，两者一样。那条注释是照着宏名想当然写的，**而它正是 16 MB 仪式的唯一依据**。<br>· **决定栈的是解码峰值，不是结构体大小。** r32 ④ 已把「接近 1 MB」订正为 82,032，本条确认**这个订正仍不足以支撑任何栈结论**。<br>**④ 由此推翻 r31 ② 的实现手段（原则不变）。** Qt 的 `QThread` 在 Windows 上默认 1 MB（`ConsoleEmulatorThread.cpp` 覆写 `run()`，未设栈大小），而 release 解码要 2 MB。**我开工前给出的预判是 200–300 KB，错了约 7 倍** —— 错在没把 `Lcd::buffer` 这个内联数组算进去。计划里那条「实测超 512 KB 才启用大栈线程」的触发条件成立，故 **save/load 两个方向都经由我们自己的 16 MB 工作线程**，调用方栈上只剩一个指针。r31 ② 的**原则**（调用方永远不需要知道栈）仍然成立。<br>**⑤ 五条裁决，其中三条改变 r31 定的形状。**<br>· **不出 `gba_savestate_free`** —— §4.1 的存档清单（计划 265-271 行）**没有它**，因为 `gba_savestate_save(dst, cap, written)` 是**调用方给 buffer**，内存本就归调用方。r31 ③ 的「我们分配 + 显式 free」与 §4.1 冲突，按 §4.1 走；r31 ④ ⑤ 关于 `cap` 与 `from_raw_parts` 的两条随之作废。<br>· **payload 用一个 `WireState` 包装结构体**（`cpu` + ROM 指纹 + 音频相位 + pending 等待），不手工拼字节；带 `deny_unknown_fields`，使 vendor 改字段名变成**解码失败**而非静默错位。<br>· **pending 等待走 sidecar 显式存取，不动 vendor 的 9 处补丁** —— `enter_intr_wait` 把等待请求放进 **thread_local**（`swi/mod.rs:452-455`），唤醒谓词是裸函数指针且 `#[serde(skip)]`。**在 `VBlankIntrWait` 中途存档，读档后等待条件会静默退化成 `Halt`** —— 而游戏绝大多数时间就停在这个状态，这是本条最该拦下的一条。两个 `pub(crate)` 存取器即可覆盖。长期更干净的形态（存进 `Arm7tdmi` 的 serde 可见字段）要动 R15 定的补丁集，本轮不做。<br>· **ROM 指纹 = `game_code` + `marker_code` + `rom.len()` + FNV-1a(rom)**，从 `Gba::cartridge_header`（`pub`）与 `Bus::internal_memory.rom`（`pub`，`bus.rs:75`）取，**零 vendor 变更**。只存 header 那 1 字节 `calculated_checksum` 防不住改版 ROM；ROM 不在 payload 里（与本仓库既有 savestate 惯例一致），故不校验就会「拿 A 游戏的存档读进 B 游戏」且不报错。<br>· **错误码补 `GBA_ERR_UNSUPPORTED = 4`、`GBA_ERR_STATE` 改回 5** —— §4.1 的枚举是 `{OK, NO_ROM, BAD_ROM, BIOS, UNSUPPORTED, STATE, CAPACITY}`，而 `frame.rs:48` 实现成 `STATE = 4` 且**没有 `UNSUPPORTED`**。savestate 加载失败（版本太新 / 非本格式）正需要 `UNSUPPORTED`。**C++ 侧目前零调用，此刻改零成本** —— 这也是「Qt 接线放最后」的一个额外好处。<br>**⑥ 减法清单（与加法同等重要）。** 删掉 `save.rs` 的 16 MB 仪式 —— 改由**生产侧的约束**（1 MB 的 QThread）导出断言，并把「16 MB 工作线程」写成有实测数字支撑的常量而非习惯；改正那两条与实测矛盾的注释；`gba_reset` 仍是空壳（`frame.rs:236` 注明 SoftReset 语义属 S2-b2）—— 本轮**不动它**，只记入 §9.1。<br>**⑦ 实施。** 改 `save.rs`（编解码 + `WireState` + 指纹 + 工作线程）、`audio.rs`（`clock_state` / `restore_clock`）、`swi/mod.rs`（`pending_intr_wait` / `restore_intr_wait` 两个 `pub(crate)` 存取器）、`frame.rs`（3 个导出 + `apply_state` + 错误码对齐）、`build.rs`（3 条声明）、`Cargo.toml`（新增 `serde` 可选依赖）。**vendor 补丁集仍是 9 处，零新增。** 根 crate 新增 `serde`（带 `derive`）挂在 `gba` feature 下 —— r30 ① 已论证 registry crate 不触发 R14，且 `serde` 本就经 `f11qa` 在图内；**不变式 8 用 `cargo tree` 复核：NES-only 图里 `gba-core` 与 `rtrb` 仍是 0 次**（`serde` 那一项来自 `f11qa`，不是新依赖）。<br>**⑧ 锁测试 185 → 203（+18，其中 1 条测量工具 `#[ignore]`），导出符号 21 → 24**（`dumpbin` 确认进入 CMake 实际链接的 staticlib，派生头同步）。**变异验证 8 处全部实测转红**：去掉指纹校验 / 不重装 `swi_hook` / 不恢复 pending `IntrWait` / 不恢复音频相位 / 容量路径报错的大小 / `GBA_ERR_STATE` 改回 4 / 丢掉 ROM 内容哈希 / 把 `IntrWait` 模式一律写成 `AlwaysWait`。<br>**⑨ 实施中撞到两件事，都不是本条预先想到的。**<br>· **`exclusively()` 会把一个失败放大成十四个。** 它用 `Mutex<()>` 串行化 frame.rs 的用例，但 `let _guard = lock.lock();` **忽略了中毒**：一个用例 panic 之后锁被毒化，后续每个用例的 `lock()` 都返回 `Err`，而错误被丢弃，于是**全部用例失去隔离、并行跑、互相干扰**，报出的 14 个失败里只有 1 个是真的。已改为 `unwrap_or_else(|e| e.into_inner())` —— 隔离比「第一个失败就污染其余」更要紧。<br>· **我自己的两个新测试把顺序写反了**：一个在 `gba_savestate_save` **之后**才往工作 RAM 写标记并期待读档后能读回，另一个在保存**之后**才设相位。两者都靠「读档后值不对」暴露，改的是测试不是实现 —— 但这正是纪律 3 的用途：全绿不等于对，红了才知道自己错在哪。<br>**⑩ 门禁。** 全量 Release 构建 `BUILD_EXIT=0`；`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`，L11 口径）；两态 `cargo check` 通过；`cargo tree` 不变式 8 复核通过。 | 只读调查 + 第 0 步栈实测 + 五条裁决 + 实施 + 变异验证 |

| **r38** | 2026-10-01 | **S2-b3（RTC）开工：核心里 RTC 早已完整，缺的不是功能而是「可被验证」—— 第 10–14 处 vendor 补丁，并如实记账它的代价。**<br>**① 调查结论与 §八 原先的记法不同：RTC 不需要「暴露」。** 核心里 `cpu/hardware/rtc.rs` 是**完整的 S3511 实现**（`write` 收脚、`sio()` 回读、BCD 日期换算用 Hinnant 算法），`internal_memory.rs:636-661` 在**游戏写 ROM 偏移 `0xC4`–`0xC9` 时自动置 `gpio_present`** 并把脚状态喂进 `Rtc::write`，读 `0xC4` 时回填 `rtc.sio()`。**游戏侧完全透明，宿主侧什么都不用做就能跑。** §八 写的「RTC（第 10 处 vendor 补丁）」隐含的是「缺个功能」，这是错的记法。<br>**② 真正缺的只有一件事：时间没有注入点。** `current_unix_secs()`（`rtc.rs:214`）是私有自由函数，直接读 `SystemTime::now()`。后果有两个，都很实：<br>· **核芯自己的测试验不了日期。** `datetime_read_streams_the_clock_bytes` 断言的是「读回的年份 == `datetime_bytes(current_unix_secs())[0]`」—— 拿**同一刻的主机时钟**当期望值，**自比较**。星期几、分、秒三个字节**零覆盖**（该测试只读 1 个字节），换算写错了也全绿。<br>· **GA 门禁第 7 条无法作答。** 「RTC 场景实测通过」在没有一个可控时刻的前提下，只能落进「或明确列入已知限制」。<br>**③ 第 10–14 处补丁（5 处，跨 2 个文件 —— 这是本条最该记账的地方）。**<br>· `rtc.rs`：加 `time_override: Option<i64>` 字段（**随 savestate 序列化** —— 钉住时刻后存的档，读回来必须还钉着，否则覆盖就是个陷阱）、`set_time_override` / `time_override` 两个访问器、一个 `now_unix_secs()`，以及把 2 处 `datetime_bytes(current_unix_secs())` 改成 `datetime_bytes(self.now_unix_secs())`。<br>· `internal_memory.rs`：`pub fn rtc()` / `rtc_mut()` 两个访问器 —— `rtc` 字段私有，根 crate 拿不到。`Bus::internal_memory` 与 `Arm7tdmi.bus` 本身已是 `pub`，**所以不需要碰 `bus.rs`**。<br>**④ 代价必须说准：补丁集不再集中在一个文件。** 9 处全部在 `arm7tdmi.rs`，本条把它变成 **14 处、跨 3 个文件**。AGENTS.md 与 R15 都写着「集中在 `arm7tdmi.rs` 一个文件」，**这两处记载从此不再成立**，已同步改写。零 vendor 改动做不到 —— `rtc` 字段私有，而 `current_unix_secs()` 没有任何外部拦截点。<br>**⑤ 裁决：固定时刻，不是偏移量。** 覆盖期间时钟**不前进**。两个用例（锁测试要一个已知日期、调试的人要一个自己挑的日期）要的恰恰是冻住的时钟；真要流逝就用宿主时钟，那本来就是默认值。**代价记入 §9.1（L15）**：钉住期间游戏若靠两次 RTC 读数算流逝时间会读到 0。<br>**⑥ 交付面 2 个导出**（24 → 26）：`gba_rtc_set_time(int64_t)`（`<= 0` 恢复宿主时钟）与 `gba_rtc_time(int64_t *out)`（当前生效时刻）。**不导出「是否 RTC 卡带」** —— `gpio_present` 只表示「游戏碰过 GPIO」，不是静态的卡带属性，暴露它会诱使前端拿它当「这张卡有没有 RTC」用，而那是错的。GA 门禁第 7 条要的是「RTC 跑得对」，不是「前端能显示 RTC」。<br>**⑦ 本条真正的验收不是导出面，是一条端到端的芯片级锁测试。** 钉住 `1_612_325_106`（2021-02-03 04:05:06 UTC），经 `Gba` 的**公开 bus** 走完整 S3511 时序（CS 拉高 → 8 位命令 `0110_0101` → 逐位读出 7 字节），断言 `[0x21, 0x02, 0x03, 0x03, 0x04, 0x05, 0x06]` —— **星期几 0x03 是新覆盖的**，也是核芯自己漏掉的那一项。 | 只读调查 + 补丁代价记账 + 五处最小补丁 |

**r38 实施结果**（2026-10-01）：① 改 `rtc.rs`（`time_override` 字段 + 3 个成员 + 2 处调用点）与 `internal_memory.rs`（`rtc` / `rtc_mut`），`ATTRIBUTION.md` 新增 §3.2.1 并把 §3.2 标题与 §4 的补丁集统计同步为「14 处、跨 3 文件」；新写 `src/gba/rtc.rs`（8 条测试）；`frame.rs` 加 2 个导出；`build.rs` 加 2 条声明。② **实施期推翻了自己的一个裁决：** ⑥ 原本定的是单参数 `gba_rtc_set_time(int64_t)` 且「`<= 0` 恢复宿主时钟」，理由写的是「Unix 纪元不是游戏要的日期」——**而第一条端到端测试恰恰要把纪元钉住**（1970-01-01 的期望日期能手算，是唯一适合当基准的瞬时）。那个哨兵让纪元变成不可钉死的值，**测试当场转红把它顶了出来**（读到 0x26 即 2026 年，说明钉住根本没生效）。改为**显式 `int32_t enable` 参数**，并把「与合法值冲突的哨兵不是哨兵」写进注释——我在文档里刚写完这句话，转头就造了一个反例。③ 端到端测试第一次跑 4 条全红（读到全 0）：命令字节的固定部分应是**高半字节** `0b0110_xxxx`（0x60），我写成了 `0b0110_000`（0x30），芯片按「不是寻址给我」**静默返回全 0，不报任何错**。这类失败比崩溃更难查，注释已记。④ 锁测试 **203 → 213**（+10：芯片级 5 条 + ABI 3 条 + 胶水 2 条），变异验证 **6 处全红**（忽略覆盖 / 星期几改成 `0=周一` / 覆盖不序列化 / setter 不写 / time-only 切片错一字节 / ABI 忽略 enable）。其中「星期几」那处是本条的核心论据：**核芯自己的测试对它全绿，只有新测试转红**。⑤ 门禁：全量 Release 构建 `BUILD_EXIT=0`；`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`）；两态 `cargo check` 通过；`cargo tree` 不变式 8 复核 `gba-core` 与 `rtrb` 仍 **0 次**；`dumpbin` 确认 staticlib 含 **26** 个 `gba_*`（24 → 26），`fceux11_rust.h:4093-4094` 同步声明。**至此 S2-b 三项 ABI 全部落地**（S2-a 10 + S2-b1 5 + S2-b2 3 + S2-b3 2 + 探针 6）。


| **r39** | 2026-10-01 | **Qt 接线开工前的只读调查：把它拆成三个阶段，并划清「第一个可能回归 NES 的阶段」在哪。**<br>**① 结论先行：这不是「接三个调用」的活。** 现有前端每帧只有一个入口（`fceuWrapperUpdate` → `DoFun`，`fceuWrapper.cpp:1313`），这一处很干净；但帧交接、尺寸、音频环、限速四处全是 NES 专属的硬编码。<br>**② 五个硬约束（全部带出处，调查为只读）**。<br>· **帧交接只认 8 位索引 + NES 256 扫描线步长**：`doBlitScreen`（`sdl-video.cpp:469`）写死 `XBuf += s_srendline*256`，唯一可用的转换是 `Blit8ToHigh`（8 位索引→高位）。GBA 交出的是 240×160 的 15 位色，需要在旁边另开一条转换。<br>· **`PixBufPool` 已经是 32bpp RGBA**（`nes_shm.h:34-36,85`）—— 这一条是好消息：环的**格式**不用动，要动的是**谁往里写**。<br>· **`GL_NES_WIDTH/HEIGHT` = 256/240 是编译期常量**，被 shm 初始化与每一个 viewer/scaler 消费（`nes_shm.cpp:54-56`、`ConsoleViewerSDL.cpp:68,616` 等）。GBA 是 240×160（3:2），且现有缩放是**分数的**，不整数对齐。<br>· **音频环只有一个，且 `WriteSound` 必须整块写入**（写超了会阻塞 emu 线程，`sdl-sound.cpp:404-434`），SDL 侧**不施加音量**（`sound.cpp:1603`）。GBA 每帧样本数是**分数**（§4.2 的相位累加器）—— 这正是当初选分数累加的回报，但要求调用方每帧自己算，不能按固定 738 写。<br>· **`LOADER_OK` 之后的整段都是 NES 专属**：`PowerNES()`、调色板、cheats、Genie、auto-resume（`fceu.cpp:515-552`）。新增第 5 个装载器必然踩到它。<br>**③ 一个口径裁决：不新增 `EGIT` 值。** `EGIT` 只有 4 个值（`git.h:7-13`）而 `GameInfo->type` 被**约 20 处**读（debugger / palette / vidblit / hex editor / TAS），**全是 NES 专属逻辑**。给它们一个不认识的新枚举值，它们会各自落进「当作普通卡带」的错误分支 —— **那比说谎更糟**。正确做法是**在到达任何 NES 模拟调用之前就分叉**，并按不变式 7 把 GBA 会话下的调试器内存映射 / cheats / 调色板 / NSF / TAS **显式声明为不可用**。<br>**④ 分三阶段，风险递增。**<br>· **阶段 1（零风险，不碰视频与音频）**：补上 §4.1 列了却**从未实现**的 `gba_set_buttons`（26 个符号里没有它；好消息是 `Bus::keypad` 是 `pub`、`Keypad::set_button` 是 `pub const`，**零 vendor 补丁**）、`GbaLoad` 进装载链、对话框加 `.gba`、emu 循环分叉。**GBA 会话的画面是黑的** —— 阶段 2 之前没有 32bpp 通路，而**显示上一张 NES 画面比黑屏更糟**（那是错的，不是缺的），故 `GbaLoad` 用 `nes_shm->clear_pixbuf()` 清屏。<br>· **阶段 2（视频）**：`PixBufPool` 旁加 32bpp 直通 + `GL_NES_*` 改运行时。**这是本项目第一个可能回归 NES 的阶段** —— 它动的是 NES 的缩放数学。<br>· **阶段 3（音频 + 限速）**：分数样本数进现有环 + 59.7275 限速分支。<br>**⑤ 两条必须说清的诚实边界。**<br>· **阶段 2 的证据不止 `ctest`。** 不变式 1 的判据是 F11QA `108P/12F`，本机跑不了。届时要么把 F11QA 矩阵接进本机，要么这一阶段就在「只有 `ctest` 证据」的状态下提交 —— **那是 r29/r30 反复记过的「无证据状态」，不采用**。<br>· **阶段 2/3 的成果在本环境验不出来。** 「一个 `.gba` 真的跑起来、画面与声音对」正是 jsmolka / `.gba` 资产要回答的，而那是用户 2026-10-01 裁决暂不处理的阻塞项。阶段 1 可自证（合成卡带被正确识别/拒绝、按键到达核心、机器在推进），阶段 2/3 只能验到「不崩、不回归、尺寸与缓冲契约成立」。 | 只读调查 + 三阶段切分 + 一处口径裁决 |
| **r40** | 2026-10-01 | **S2-b4 阶段 1 落地：`.gba` 能被识别、装载、推进、收按键。画面是黑的 —— 这是有意的。**<br>**① 交付面。** Rust 侧补上 `gba_set_buttons(uint16_t)`（§4.1 列了、26 个符号里一直**没有**的那个）**零 vendor 补丁** —— `Bus::keypad` 是 `pub`、`Keypad::set_button` 是 `pub const`。另加一个 `gba_buttons(uint16_t *out)` 读取面，**§4.1 没有、也不该有**：没有它，「设了掩码」与「核心收到了」之间只能靠读总线来验证。C++ 侧新增 `src/gba_load.{h,cpp}`（装载器 + 会话状态 + 每帧步进 + 掩码构造）、装载链末尾加 `GbaLoad`、`LOADER_OK` 之后对 GBA 会话**整段跳过** NES 尾处理、对话框加 `.gba`。<br>**② 三个实施期发现，都不是「写错了」而是「想错了」。**<br>· **我把声卡调用放进了核心库 —— 分层错误。** `gba_load.cpp` 属于 `fceux11_core`，而 `WriteSound` / `GetMaxSound` / `GetWriteSound` 在 Qt 驱动库里。编译过，**链接炸了**：4 个 F11QA 测试可执行文件链接 `fceux11_core` 而不链驱动，于是每一个都 `LNK2019` 未解析的 `WriteSound`。**这恰好证明我自己的三阶段切分是对的** —— 我在 `gba_load.h` 的注释里写「音频在这里接，因为样本已经现成」，转头就违反了同一条分层。已撤：阶段 1 的 `fceu11_gba_step_frame` **只做**「设按键 + 步进一帧」，音频与限速都归阶段 3、在 Qt 侧做。为此加的 `FCEUD_GetSoundRate` 也一并撤掉 —— **不提交无人调用的访问器**。<br>· **`drivers/Qt/dface.h` 是一颗既有地雷。** 它 `#include "Qt/input.h"`，而 `input.h:137` 用了 `FAMILYKEYBOARD_NUM_BUTTONS` 而**没有任何东西先定义它** —— 所以这个头只能被「已经 include 过别的东西」的编译单元用。绕开 include 顺序去凑，不如不引它。<br>· **生成头里的错误码早就过期了，而我刚写完的代码正依赖它。** `build.rs` 补了 `GBA_ERR_UNSUPPORTED=4` 并把 `STATE` 改成 5，头文件里却还是 `STATE 4`、且**根本没有 `UNSUPPORTED`**。根因比「忘了重编」更深一层：**生成头是源码树里的一个文件，而本仓库有两个独立的 cargo target 目录** —— `cargo test` 用 `src/rust/target`，CMake 用 `build/src/rust/target`，各自按自己的节奏跑 `build.rs`，**谁最后跑谁写这个文件**。所以它可以一边被重新生成、一边留着另一份旧的。`the_gba_c_abi_is_declared_for_every_exported_function` 只管函数声明，**这些是 `#define`，没有任何东西在看它们**。已补 `the_error_codes_reach_the_generated_header`。<br>**③ 载入时清屏，而不是留着上一张 NES 画面。** `nes_shm->clear_pixbuf()`。阶段 2 之前没有 32 位通路，画面只能是黑的 —— 而**黑屏是「没有」，留着旧画面是「错的」**，两者不能混。`GbaLoad` 因此也**不调** `signalFrameFinished()`：那个握手的含义是「池子里有一张**新**帧」，阶段 1 根本没有。<br>**④ 装配顺序：GBA 放装载链的最后一位。** 链只在 `LOADER_INVALID_FORMAT` 时前进，所以一个误判的装载器只可能被「已经失败过所有 NES 格式」的文件触达 —— 于是「新装载器会不会弄坏旧格式」这个问题有了答案而不是风险，且**现有格式一分钱探测成本都不付**。<br>**⑤ 按键位序不从记忆写。** NES 侧 `joy[0]` 的位序是从 Qt 输入后端读出来的（`input.cpp:1514-1531,1538`：bit 4 是 up、7 是 6 的反向、`JS == 15` 即 A+B+Start+Select），GBA 侧取自 `GbaButton` 枚举。**§7.2 要求映射的 8 个键逐位对上，L / R 刻意留空** —— NES 手柄没有肩键，而「L 是 Z 键」在 Z 往往已经绑给 A 的情况下会悄悄让 L 变成第二个 A。按键绑定是 S3 的工作。<br>**⑥ 锁测试 213 → 219，变异 3 处全红。** 输入侧最关键的一条是 `each_button_lands_on_its_own_bit`：**十个键十个独立用例** —— 一条「全按下去」的测试对**任何位置错乱都照样通过**，而位序正是一句没有任何其他构建会检查的硬件断言。变异验证：头文件删掉 `UNSUPPORTED` / `STATE` 改回 4 / `set_buttons` 无视掩码，三条全红。<br>**⑦ 门禁。** 全量 Release `BUILD_EXIT=0`；`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`）；两态 `cargo check` 通过；`cargo tree` 不变式 8 复核 `gba-core` 与 `rtrb` 仍 **0 次**；`dumpbin` 确认 **28** 个 `gba_*`（26 → 28），派生头的七个错误码已与实现一致。**vendor 补丁集不变**（9 + 3 + 2 = 14 处、跨 3 个文件），本轮零新增 —— 输入走的是核芯已有的 `pub` 通路。 | 阶段 1 实施 + 三处实施期发现 + 变异验证 |

| **r41** | 2026-10-01 | **用户裁决（2026-10-01）：GBA 不接入任何质量检测系统，长期 Alpha，不阻塞本体，与 NES 侧最大化解耦。** 本条把它写成**不变式 9**、改写**不变式 2** 使二者不再互相拉扯，并新增**两条守卫检查**把它变成可执行的约束而非承诺。<br>**① 这条原则直接作废了我上一轮提的阶段 2 方案。** 那个方案是「把 `GL_NES_WIDTH/HEIGHT` 从编译期改成运行时 + 在共享池旁加 32 位通路」—— **那正是「改 NES 的缩放数学」，也正是它需要 F11QA 才敢提交的原因。在本原则下它不该被做，因为它本身就是要避免的耦合。**<br>**② 连带解决了我上一轮提出的验证阻塞。** 既然 GBA 不许动 NES，它就没有能力回归 NES，**`ctest` 33/34 就是它的全部出口判据** —— F11QA 与 jsmolka 都不是。此前「阶段 2 要么接 F11QA、要么在无证据状态下提交」的两难**不再存在**。<br>**③ 一条调查结论，它决定了阶段 2' 怎么写：现有 viewer 的尺寸处理本来就不一致。**<br>· **纹理按运行时尺寸建立**：`ConsoleViewerSDL.cpp:346` 用 `nes_shm->video.ncol/nrow` 调 `SDL_CreateTexture`；<br>· **缩放数学用编译期常量**：`render()` 读 `GL_NES_WIDTH/HEIGHT`（`:616-617`），另有约 10 处（`ConsoleViewerGL.cpp:107`、`ConsoleViewerQWidget.cpp:65,84,388`、`ConsoleVideoSetup.cpp:106,110`、`ConsoleVideoConf.cpp:1185`）；<br>· 而 `video.ncol/nrow` **会被 `CalcVideoDimensions` 改**（双倍放大等模式，1024×1024 封顶）。<br>**所以「把常量换成运行时尺寸」会改变 NES 在某些视频模式下的既有行为** —— 那正是只有 F11QA 才抓得到、而本机跑不了的那一类改动。**此路必须避开。**<br>**④ 阶段 2' 的方向：故意复制一小段呈现代码，而不是泛化 NES 那段。** GBA 侧有自己的帧池、自己的 `SDL_Texture`（240×160×4 ≈ 150 KB，×2 缓冲可忽略）、自己的绘制调用；接入点**只有一处**：`render()` 里按当前机器选分支；**NES 分支逐字不动**；窗口与信箱沿用现有的 `sx/sy/rw/rh` 机制，不新写缩放代码。**这是一次有明确理由的复制，不是分叉** —— 共享同一个窗口、同一个 renderer、同一种纹理格式，共享的是基础设施，不共享被泛化后的那条路径；而泛化恰恰是把 NES 置于风险中的那件事。<br>**⑤ 音频那边几乎不需要耦合。** 那个环按**设备**配（`BufSize_ms × rate`），不按机器配，所以 GBA 写进同一个环不改变 NES 的任何东西。真正要动的只有**限速的基准帧率**（59.7275 vs NES 的 60.0988），而 `getBaseFrameRate()` 本来就是按会话取值的接口。<br>**⑥ 两条守卫检查（把原则变成会变红的东西）。**<br>· `the_c_abi_is_called_from_exactly_one_cpp_file`：扫 `src/**` 的 C/C++ 源（排除 `src/rust/`），**剥掉注释与字符串字面量**后找 `gba_*` 的调用点，断言文件集合恰为 `{gba_load.cpp}`。用 `\bgba_` 而非子串匹配，因此 `fceu11_gba_*` 这类 C++ 侧符号不误报。<br>· `gba_references_stay_inside_the_allowlist`：断言**任何提到 GBA 的 C/C++ 文件都在显式白名单内**（`gba_load.{h,cpp}` + `fceuWrapper.cpp` 的每帧分支 + `fceu.cpp` 的装载链 + `ConsoleFile.cpp` 的过滤器），新增触碰点必须**在清单里显式登记**。<br>**⑦ 剥注释不是洁癖。** 第一版守卫会把日志字符串里出现的 `gba_` 当成调用点，而那就意味着有人得为此改代码或改白名单 —— **一个能被随手绕过的守卫等于没有守卫**。<br>**⑧ 不变式 2 同步改写**（用户裁决）：原文「不得分叉 Qt 驱动」与「最大化解耦」在「该复制还是该泛化」上互相拉扯，本条把它改成「允许在现有 widget / 驱动内为 GBA 保留一条独立绘制分支，条件是 NES 分支逐字不动；取舍时以解耦优先」。<br>**⑨ 两条守卫落地：`src/rust/src/gba/decoupling.rs`，4 条测试，锁测试 219 → 223。** 变异 3 处全红：某个非白名单文件多一句 `gba_swi_count();` / 多一句 `if (fceu11_gba_active())` / 白名单里写一个不存在的文件名。<br>**⑩ 写守卫时它自己先错了两次，两次都记在这里，因为它们是这类守卫的通用坑。**<br>· **剥注释的实现有编码 bug**：第一版用 `byte as char` 逐字节推进 `String`，于是每个 UTF-8 续字节都变成**另一个多字节字符**，字符串长度改变、后面全部错位。症状是 11 个 NES 文件被报成「提到 GBA」而它们一个都没提。改成在 `Vec<u8>` 上做、最后 `from_utf8_lossy`。<br>· **「提到 GBA」的判据太松**：第二版匹配小写化后的裸 `"gba"` 子串，于是 `Format_RGBA8888`、`PRGBanks`、`logBankNumCbox`、`bookmarkPreviewPopup` 全部命中。**一个会误报的守卫只会被关掉，而关掉的守卫等于没有守卫。** 改成**大小写敏感 + 前缀锚定**（`gba_` / `fceu11_gba` / `GbaLoad` / `gba_load`），并用 `\bgba_` 的词边界保证 `fceu11_gba_*` 这类 C++ 侧包装符号不被当成 ABI 调用。<br>· 另加两条自检：白名单里的每个名字都必须仍是真实文件（**过期的白名单比没有白名单更糟**，因为它看起来像覆盖），以及扫描必须真的找到 C++ 树（`cxx_sources()` 找不到目录时返回空会让上面所有断言**因为错误的原因而通过** —— 与 r36 修掉的形状相同）。<br>**⑪ 顺带修掉一处提交里的无关改动。** `ConsoleFile.cpp` 的 UTF-8 BOM 被我的 PowerShell 往返剥掉了，diff 里表现为第一行 `-﻿// …` → `+// …`，**评审看不到却让 diff 在说谎**。已补回，diff 现在只剩过滤器那一行。

| **r42** | 2026-10-01 | **清理 r41 留下的口径矛盾：不变式 9 写下了「GBA 不是任何检测系统的对象」，而计划里还有 7 处仍把 jsmolka 当作门禁。** 本条逐处改掉，并**据实收回我上一轮的一个错误结论**。<br>**① 先收回一个我说错的结论。** 上一轮我说「L4 / L7 若改为以多源交叉证据结案，已知限制计数降到 7 条，§8.2 第 3 条即满足」—— **这是错的。** L4 的内容是「S0–S1a 的结论全部来自本项目自建的锁测试」，L7 是「LZ77 位序只有自建证据」；**这两条不因消解路径改变而变假**，改措辞消不掉它们。所以**「已知限制 9 条 > 门禁要求的 ≤ 8 条」是一个真实缺口**，且不能靠改路径或改口径解决。**故本条只改 jsmolka 当门禁的 7 处，阈值本身不动** —— 按纪律「出口判据的改写必须记账，不许静默放宽」，`≤ 8` 要不要调整是**用户裁决项**，已单列。<br>**② 逐处改法**（原则：jsmolka 从「门禁」降为「可选信号」，**不是从计划里删掉** —— 它仍是 §7.5 列出的上游测试套件，删掉会让人以为没调研过）：<br>· **§8.2 GA 门禁第 2 条**（原「jsmolka **10/10 全绿**（含 `bios.gba`）」）→ 改为「~~jsmolka 10/10 全绿~~ → **r41 不变式 9 取消此项**：GBA 不是任何检测系统的对象，且长期 Alpha、不阻塞本体。**2.0 GA 不以 GBA 精度为条件**」。**这是七处里最要紧的一处** —— 它原本让 GBA 的进度卡着整个 2.0 的发布。<br>· **§〇 TL;DR 第 5 项**（原「改手工实测 + **jsmolka 进 CI**」）→「jsmolka **不进 CI**；改以锁测试的多源交叉证据结案（§7.5）」。<br>· **§1.2 不做清单**（原「仅 jsmolka 进 CI」）→「**不做**：不接入 F11QA / KagamiQA / jsmolka（§7.5、r41 不变式 9）」。<br>· **§7.5 测试策略**（原两条「CI（进）jsmolka 合成测试集，`memory.gba` 为 SWI 实现的核心验收门」/「CI（进）授权 homebrew 做黄金帧哈希与音频冒烟」）→ 两条均改为「**可选，不进 CI**」，并补一句为什么它们本来就不该是门禁：jsmolka 的判定信号是**屏幕上的 mode 4 文字**（r11 已记），而读它需要帧缓冲通路；r11 当时把门禁移到 S2 正是为了等这条通路，**而现在这条通路在 S2-b4 阶段 2' 之前不存在，且新原则已取消该门禁** —— 于是这两条从「等一个条件」变成「不做」。<br>· **§八 S1a-1 / S1b 的出口判据**里「jsmolka → 移至 S2」一句 → 标注「**r41：该重定义的前提已不成立**，jsmolka 不再是任何阶段的门禁」。两阶段早已完成，这两个出口判据保留为历史记录即可。<br>· **L4 / L7 的「消解时点」**（原均为 S2 + jsmolka）→ 改为「**不再以 jsmolka 为条件**。结案依据改为多源交叉证据：L4 已有 r17（5:1 来源对照）与 r21（第二个独立模拟器互证），L7 已有 BIOS 反汇编 + gbajs2 + 两个 Nintenlord 系工具。**代价要说准：这两条在 Alpha 阶段长期开放**，因为「新发现一个独立来源」是唯一能改变它们的动作，而外部基准已被取消」。**两条本身仍然开放，不改状态。**<br>**③ §8.1 GA 后排期 P3**（「接入 F11QA / KagamiQA（jsmolka 通道产品化）」）→ 标注「**r41：与不变式 9 冲突，已并入 P1 之外单列**；若 GBA 将来转正式版再议，本轮不排期」。<br>**④ 已知限制现状（据实记录，供阈值裁决用）**：开放 **9 条** —— L1、L2、**L4**、L6、**L7**、L10、L13、L14、L15；已消 6 条 —— L3、L5、L8、L9、L11、L12。**§8.2 第 3 条要求「≤ 8 条」，现为 9 条，缺口 1 条。** 其中 L4 / L7 是新原则下**长期开放**的（见 ②），不因本次改动减少；L13（JSON 载荷体积）、L15（钉住时钟不前进）则是 r37 / r38 新增的。<br>**⑤ 留一个明确的用户裁决项**：`§8.2 第 3 条的「≤ 8 条」要不要调整、以什么口径调整。**本条不动它** —— 放宽出口判据必须由人决定并记账，不能由一份清理口径的提交顺手带过。三个选项见对话：(a) 维持 ≤8，先在 S2-b4 / S3 里消掉一条（如 L6 的 bits 5/6，或 L10 的截断）；(b) 改为「≤ 8 条 + Alpha 阶段长期保留项另计」，把 L4 / L7 显式排除在计数外；(c) 直接调到 10 并记明理由。

| **r43** | 2026-10-01 | **用户裁决：维持「≤ 8 条」，先消掉一条。本条记下选定目标的取舍过程 —— 首选的 L6 取不到权威材料而关不掉，改消 L14。**<br>**① 先说为什么不是 L10。** L10 是「`BitUnPack` 的偏移溢出**按硬件行为不截断**」。要消它就得**把正确的硬件行为改成错的** —— 那不是关限制，是引入缺陷。**L10 不动。**<br>**② 首选 L6（`RegisterRamReset` 的 bits 5/6 复位 SIO 与声音寄存器），本轮**关不掉**，据实记录失败过程。**<br>· `problemkaputt.de/gbatek.htm` 对本工具的 UA 返回 **403**（r20 ① 早已记过这个站点会 403，本次复现）。<br>· 镜像（`mgba-emu.github.io/gbatek`、`gbatek-gbaonly`、`problemkaputt.de/gbatek-gba-i-o-map.htm` 的缓存内容、`afska/gba-link-connection` 的转录）给的是**寄存器映射**：SIO 侧 `0x120`–`0x12A` 是 SIOMULTI0-3 / SIOCNT / SIOMLT_SEND、`0x134` 是 RCNT、`0x140` 是 JOYCNT；声音侧 `0x060`–`0x07C` 四个声道、`0x080`–`0x088` 是 SOUNDCNT_L/H/X 与 SOUNDBIAS、`0x090`–`0x09E` WAVE_RAM、`0x0A0`–`0x0A6` 两个 FIFO。**但这些来源没有一处给出 `RegisterRamReset` 对 bits 5/6 的复位语义与逐寄存器复位值。**<br>· 唯一提到默认值的说法（SOUNDBIAS 复位为 `0x0200`）出自 `rust-console/gba` 的 deepwiki 文档，用词是 "typically"，**既非 GBATEK 原文、也非寄存器复位表**。按本项目纪律 4「不凭记忆写位级格式，取不到权威材料就停下来记录」，**不写**。<br>· 故 L6 保持开放。这是一次**有价值的失败**：它把「L6 关不掉」的原因钉死在「缺复位值表」而不是「懒得做」，将来若取得该表可直接续做。<br>**③ 改消 L14（`gba_reset()` 不复位核心）。** 它不需要任何外部资料：把当前 ROM 取出、重新建机即可，而这**本来就是 `Machine::new` 的语义**。<br>· **口径要说准，否则就是换措辞而非关限制。** 实现后的 `Reset` 是**上电复位**（从同一卡带重建），**不是 BIOS SoftReset**：SoftReset 只复位 CPU 状态并跳回 `0x00000000`，**不清内存**。选择上电而非 SoftReset 的理由：模拟器的 Reset 按钮在用户心智里就是「重新开始」，而 FCEUX 本体的 NES 复位也是硬复位。**代价是明确记下的**：依赖 SoftReset 保留内存的程序会看到不同行为 —— 但那属于「Reset 按钮的语义选择」，不是 L14 所说的「机器根本没有复位」。**L14 的事实陈述因此不再成立，按其原义消号。**<br>**④ 计数。** 消 L14 后开放 **8 条**（L1、L2、L4、L6、L7、L10、L13、L15），**恰好满足 §8.2 第 3 条的「≤ 8」，阈值不动。** 已消 7 条。

| **r44** | 2026-10-01 | **S2-b4 阶段 2' 开工：读完 `ConsoleViewerSDL` 三处后方案落定。触点从「一处」变成「一个文件里的三处」—— 这是代码本身要求的，不是设计偏好。**<br>**① 先纠正 r39 ③ 的一句错话。** 那里写「`render()` 的缩放数学用编译期常量」，**不准确**：`render()` 的 `:623-624` 本来就从 `nes_shm->video.ncol/nrow` 读运行时尺寸，`GL_NES_*` 只是 `nes_shm == NULL` 时的 fallback（`:616-617`）。**所以「缩放数学已经是运行时驱动的」**，而 `CalcVideoDimensions` 改的正是同一个 `video` 块 —— 我上一轮把「两者来源不同」说成「两处不一致」，实际上它们读同一个字段。**这不改变任何结论**（r41 ④ 避开泛化的理由依然成立：改 `GL_NES_*` 的那**另外 10 处**仍会改变 NES 行为），但记下来，因为下次读到会再困惑一次。<br>**② 阶段 1 之所以黑屏，根因在这里：`blitUpdated` 同时门控两件事。** `ConsoleWindow::transferVideoBuffer`（`:627-640`）用**同一个**标志决定「调 `transfer2LocalBuffer()`」与「`queueRedraw()`」。GBA 从不调 `BlitScreen` ⇒ 该标志恒为 0 ⇒ **既不拷贝也不重绘**。而 120 Hz 的 `gameTimer`（`ConsoleWindow.cpp:215`，8 ms）也只走到 `transferVideoBuffer()`，它内部同样门控 —— **所以定时器并不能兜底重绘。** 阶段 1 特意不设该标志（那时那里没有新帧），黑屏是**设计结果**而非缺陷。<br>**③ 方案：复用这个握手，但不复用那个缓冲池。**<br>· `gba_load.cpp` 持有 GBA 自己的 240×160×4 帧缓冲（153,600 字节），每帧由 `gba_frame_buffer` 填入，填完设 `nes_shm->blitUpdated`。**只设这一个标志，绝不写 `pixBufPool`、绝不碰 `pixBufIdx`** —— NES 帧池因此仍然只有 NES 写。<br>· `ConsoleViewerSDL::transfer2LocalBuffer` 开头对 GBA 早退，省掉每帧 1.2 MB 的无用拷贝；**该函数其余部分逐字不动**。<br>· `render()` 开头一个分支转给 `renderGbaFrame()`；**NES 路径在其下方逐字不动**。<br>· `renderGbaFrame()` 自带 `sdlGbaTexture`（240×160 ARGB8888 STREAMING），在 `cleanup()` 里销毁。<br>**④ GBA 的信箱数学是新写的、不是复制。** 现有那段要处理 `preScaler`（hq2x/hq3x）、`ixScale/iyScale`、`forceAspect` 的 `aspectX/aspectY`、`autoScaleEna`，以及「缩放值不得超过用户配置的上限」这一串 —— **GBA 三条都不适用**：永远 3:2、无预缩放、无用户缩放上限。所以 `renderGbaFrame` 只做「把 3:2 的图等比放进视口并居中」。**这是不同的、更小的计算，不是同一段的副本**，也正因如此才敢把它并排放而不抽公共函数。<br>**⑤ 白名单要加两条，而且这次是我自己发现的守卫漏洞。** viewer 需要 `#include "gba_load.h"`（新增触碰点 → `ConsoleViewerSDL.cpp`）并加一个 `sdlGbaTexture` 成员（→ `ConsoleViewerSDL.h`）。**但 `sdlGbaTexture` 匹配不上守卫的任何一条前缀** —— 现有判据是 `gba_` / `fceu11_gba` / `GbaLoad` / `gba_load` 四条，**`GbaTexture` 一条都不沾**。**即：一个 GBA 专属的成员可以完全躲开守卫混进来。** 已把 `Gba` 补进判据（**仍大小写敏感**，所以 r41 ⑩ 那批 `RGBA` / `PRGBanks` / `logBank` 不会命中），并把 `ConsoleViewerSDL.{h,cpp}` 显式登记进白名单。<br>**⑥ 「一个文件里的三处」是本条的诚实结论**，与 r41 ④ 的「接入点只有一处」表述不同。区别在于：**三处都在同一个 viewer 内、都是「加分支后早退或转交」、NES 那一侧逐字不动**；而 r41 ④ 担心的「改 10 处常量」一件也没做。<br>**⑦ 门禁补充**：`gba::decoupling` 改判据后锁测试 225 → **226**（新增一条针对新判据的用例），变异验证含「把 `Gba` 从判据里去掉 → 守卫转红」。

**r44 实施结果**：① `gba_load.{h,cpp}` 持有 GBA 自己的 153,600 字节帧缓冲，每帧由 `gba_frame_buffer` 填入（**该导出在水印合成之后取帧，所以 viewer 拿到的就是玩家看到的画面，含 `BETA` 水印**），填完只设 `nes_shm->blitUpdated` 一个标志；**`pixBufPool` 与 `pixBufIdx` 完全不碰** —— NES 帧池仍然只有 NES 写。② `ConsoleViewerSDL::transfer2LocalBuffer` 开头对 GBA 早退，省掉每帧 1.2 MB 无用拷贝，**其余部分逐字不动**。③ `render()` 开头一个分支转给新增的 `renderGbaFrame()`，**NES 路径在其下方逐字不动**。④ `renderGbaFrame()` 自带 `sdlGbaTexture`（懒建）、按 `rowPitch` 逐行拷贝（**不是一次 memcpy —— 锁定纹理的 pitch 可能大于 `width*4`**）、3:2 等比缩放且不放大、居中；`cleanup()` 销毁它。⑤ 守卫：判据加 `Gba`（大小写敏感），白名单登记 `ConsoleViewerSDL.{h,cpp}`，新增 `a_gba_specific_name_is_enough_to_need_the_allowlist`。**变异验证 2 处全红**：把 `Gba` 从判据去掉 → 新用例转红；把 `ConsoleViewerSDL.h` 从白名单去掉 → 白名单守卫转红。⑥ **白名单从 5 条变成 7 条，这是不变式 9 的守卫在按设计工作，不是破例** —— 新增的两条正是本阶段必须触碰的，每一条都在代码里写明了理由。⑦ 门禁：全量 Release `BUILD_EXIT=0`（C++ 侧一次编过）；`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`）；锁测试 225 → **226**；两态 `cargo check` 通过；`cargo tree` 不变式 8 复核 `gba-core` 与 `rtrb` 仍 **0 次**；**vendor 补丁集仍不变**（本阶段零 vendor 改动 —— 帧经 `gba_frame_buffer` 那个已存在的导出取，**没有新增任何 C ABI 函数**，导出符号仍是 28）。

| **r45** | 2026-10-01 | **S2-b4 阶段 3' 开工：音频排空 + 限速目标。三个设计点都是查码之后定的，其中两个是「让限速文件完全不知道 GBA 存在」。**<br>**① 音频不能在 `gba_load.cpp` 里排 —— 那会让 `fceux11_core` 再次依赖 Qt 驱动，4 个 F11QA 测试可执行文件会重新 `LNK2019`（r40 ① 的教训）。故**核心只暴露样本**（`fceu11_gba_audio`），**由 Qt 侧调用 `WriteSound`** —— 这正是不变式 9 允许耦合的第 ③ 条。<br>**② 限速的接法比预想的干净：`RefreshThrottleFPS()` 是唯一的重新定速入口**（`sdl-throttle.cpp:184-193`，从 `fceu11::GetDesiredFPS()` 算 `hz` 再算 `desired_frametime`），限速的真正节拍是 `DesiredFrameTime`，而 `getBaseFrameRate()` 只是个**读出口**（`:229-232`，主要给 `InitSound` 定缓冲大小用）。**只改 `getBaseFrameRate()` 是错的**：节拍不动，GBA 仍按 60.098823 跑，比真值快 0.62%，正是 §4.2 要防的那种漂移。<br>· 故加 `SetThrottleBaseRateOverride(double hz)`：`RefreshThrottleFPS` 在算出 `hz` 之后用覆盖值，`<= 0` 表示不覆盖。**`sdl-throttle.cpp` 与 `throttle.h` 因此一个 GBA 字样都没有** —— 限速文件不知道第二台机的存在，覆盖值由已在白名单内的 `fceuWrapper.cpp` 设置。<br>· 触发点是**会话状态翻转**，不是每帧：`fceuWrapperUpdate` 里存一个上一次的 `fceu11_gba_active()`，只在变化时重新定速并配置音频。<br>**③ 两处需要从驱动层拿回的东西。**<br>· **设备实际速率**：要在 GBA 会话开始时 `gba_set_output_rate`。`gba_load.cpp` 拿不到 Qt 的配置，而 `g_config` 是 Qt 层全局（只在 `fceuWrapper.h` 里声明）——**这正是 r40 撤销那次调用的原因**。改为**加回 `FCEUD_GetSoundRate()`**（读 `s_SampleRate`，即 `InitSound` 协商后的值，**不是配置里请求的值** —— 两者在回退时不同）。**该函数不含任何 GBA 字样，所以不需要进白名单。**<br>· **音量**：SDL 侧不施加音量（r34 ⑥ 已证），NES 是核心侧用 `FSettings.SoundVolume`（0–150）施加的，所以 GBA 必须自己乘 —— 已有 `gba_set_volume`，会话开始时从 `FSettings.SoundVolume` 推一次。<br>**④ 帧率常数不在两处各写一遍。** 59.7275 = `16777216 / (228×1232)`，与 `audio.rs` 的 `CPU_CLOCK / FRAME_CYCLES` 同源。**选择在 C++ 侧按硬件式写出来并注明出处**，而不是加一个只为传常量的 C ABI 导出；代价是两处各自引用同一对硬件常数，若将来改动必须同改，**这是一处需要盯着的耦合，已记在此**。<br>**⑤ 触碰点**：`sdl-sound.cpp` + `dface.h`（加回速率访问器，**无 GBA 引用**）、`throttle.h` + `sdl-throttle.cpp`（加覆盖值，**无 GBA 引用**）、`gba_load.{h,cpp}`（暴露样本与配置）、`fceuWrapper.cpp`（翻转检测 + 排空 + 覆盖值）。**白名单不增加条目** —— 四处里两处根本不含 GBA 引用。

**r45 实施结果**：① 加回 `FCEUD_GetSoundRate()`（`sdl-sound.cpp` + `dface.h`），读 `InitSound` 协商后的 `s_SampleRate` 而非配置里请求的值 —— 两者在回退时不同。② `throttle.h` + `sdl-throttle.cpp` 加 `SetThrottleBaseRateOverride(double hz)`，`RefreshThrottleFPS` 在算出 `hz` **之后**应用它（`getBaseFrameRate()` 只是读出口，改它不会改节拍）。③ `gba_load.{h,cpp}` 暴露 `fceu11_gba_base_rate()`（按 `16777216/(228×1232)` 写，与 `audio.rs` 同源，代价见 ④）、`fceu11_gba_configure_audio(rate, volume)`、`fceu11_gba_audio(dst, cap)`（**把 produced 夹到 cap** —— 调用方按 cap 开了缓冲，报更大的数就是让它越界读）。④ `fceuWrapper.cpp`：会话翻转时配置音频与限速覆盖值（每帧只比一次），每帧排空音频，**上界只取 `GetWriteSound()`** —— `WriteSound` 吃下全部、写超了会阻塞模拟线程。

**⑤ 补一条守卫，把「限速与声卡文件不知道 GBA 存在」从人工检查变成机器检查。** r45 ② 的整个设计要点就是那四个文件不含 GBA 引用；实测确认代码级引用数为 0，**但那只是因为我读了四个文件**。新增 `the_timing_and_sound_files_stay_gba_free`，断言 `sdl-throttle.cpp` / `throttle.h` / `sdl-sound.cpp` / `dface.h` 剥掉注释与字符串后不含任何 GBA 命名。变异验证：在 `sdl-throttle.cpp` 里加一个叫 `gba_pacing_note` 的变量 → **转红**。锁测试 226 → **227**。

**⑥ 一句必须说清的覆盖缺口**：阶段 3' 的排空与限速逻辑**写在 C++ 里，没有自动化测试覆盖** —— 本仓库没有 C++ 单测夹具，本轮能验的只有「接线自洽 + 守卫仍成立 + NES 未回归」。「音画是否真的同步」仍然只能在装了真实 `.gba` 的机器上听出来。**守卫保证的是「将来有人把 GBA 写进限速文件时会红」，不是「现在这段逻辑对」。**

| **r46** | 2026-10-01 | **S3-1 开工：电池存档（`.srm`）。核芯的电池 API 大半已经是 `pub`，缺的是类型的可见性与可覆盖性。**<br>**① 调查结论：这不是「新做模拟」，是薄 ABI + 宿主侧文件搬运。** 核芯已有 `battery_data()` / `load_battery()` / `take_save_dirty()`（三者都是 `pub`），`BackupType::detect()` 会按 ROM 正文的 `EEPROM_V` / `FLASH1M_V` / `FLASH512_V` / `FLASH_V` / `SRAM_V` / `SRAM_F_V` 签名串判类型（**长串优先**，正是 §7.3 第 1 级）。<br>**② 但三样东西是私有的，故需第 15 / 16 处 vendor 补丁。**<br>· **`BackupType::detect()` 与 `buffer_size()` 私有，结构体上也没有 `backup_type()` 的 getter** ⇒ **补丁 15**：`pub const fn backup_type()` 一个取值器。<br>· **没有 setter，且改类型必须重配 `sram` 缓冲**（私有字段，容量按类型定：SRAM / None / Eeprom 32 KB、Flash64 64 KB、Flash128 128 KB）⇒ **补丁 16**：`set_backup_type()`，改类型并按新容量重建缓冲，**保留旧内容的前 `min(len)` 字节**，这样切换类型不丢档。<br>· 补丁 17 不需要：`battery_data` / `load_battery` / `take_save_dirty` 已经是 `pub`。<br>**③ 交付面 5 个导出。** `gba_battery_save_type()` / `gba_battery_size(*out)` / `gba_battery_read(dst, cap)` / `gba_battery_write(src, len)` / `gba_set_save_type(type)`。**§7.3 有一条硬要求被写成断言**：*「写入前必须确认介质类型，**类型不明时拒绝写入并报错**，不得默认按 SRAM 处理（防存档损毁）」* —— `BackupType::None` 下 `gba_battery_write` 必须返回 `GBA_ERR_STATE`，而不是默默接受 32 KB。<br>**④ 宿主侧。** `gba_load.cpp` 在装载时读 `<ROM 同名>.srm` 送进核心；每帧查一次 `take_save_dirty()`，脏就写回。**只改 `gba_load.{h,cpp}`，不碰任何 NES 文件** —— NES 自己的 `.srm` 走 `FCEU_SaveGameSave`，两条路径互不看见。<br>**⑤ `.srm` 字节布局 = 核心 `sram` 的原始内容，不加任何头。** 这是 mGBA 用的格式，也是跨模拟器迁移的通行做法。**但这一条我无法在本机验证** —— 没有 `.gba`、没有 mGBA，且按不变式 9 也不打算去接外部基准。**故 GA 门禁第 4 条「跨模拟器字节级互通」仍开放**，本轮只交付「格式选择 + 读写通路」。

**r46 实施结果**：① **vendor 补丁 14 → 16 处、3 → 4 个文件**（`internal_memory.rs` +2：补丁 15 `pub const fn backup_type()`、补丁 16 `set_backup_type()` 改类型并按新容量重建缓冲、**保留旧内容的前 `min(len)` 字节**）。**这 2 处不改变不设覆盖时的行为。** ② 导出 6 个（不是 5 个 —— 实施时发现还需要 `gba_battery_take_dirty()`，否则宿主无法知道该写盘；**该标志是「取走」语义，轮询它即等于拥有那次 flush**）。导出符号 **28 → 34**。③ **`drift_guard` 在头文件生成之前就抓到了这 6 个新导出**并点名，机制按设计工作（r36 ⑤ 的那条规则）。④ 锁测试 227 → **234**（+7）。**变异验证 3 处全红**：去掉「无存档硬件则拒写」→ 拒写测试转红；`set_backup_type` 不重配缓冲 → 尺寸测试转红；切换类型时 `keep = 0` → 「切换保留」测试转红。⑤ **实施中我自己写错了一条断言并被测试当场纠正**：我断言「短存档文件写入后其余部分为全零」，实际核芯 `load_battery` 的语义是**「留下尾部不动」** —— 短文件只覆盖自己的字节，尾部保持原内容。**我把断言改成与原内容比较而不是与 0 比较**，因为要钉的是「不动」这个性质，不是某个常量。<br>**⑥ 宿主侧只改 `gba_load.{h,cpp}`**（已在白名单内，**白名单仍 7 条**）：装载后读 `<ROM 同名>.srm`；每帧查一次 dirty 位，脏就写回；**`fceu11_gba_deactivate` 里必刷一次** —— 那是唯一不能省的一次，否则最后几秒的进度全丢。**NES 侧一个文件没碰**：NES 自己的 `.srm` 走 `FCEU_SaveGameSave`，两条路径互不看见。<br>**⑦ 门禁**：全量 Release `BUILD_EXIT=0`；`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`）；两态 `cargo check` 通过；`cargo tree` 不变式 8 复核 `gba-core` 与 `rtrb` 仍 **0 次**；`dumpbin` 确认 **34** 个 `gba_*`。

| **r47** | 2026-10-01 | **S3-2：L / R 按键绑定。§7.2 的 8 键早已逐位对上，缺的正是这两个肩键 —— 而它们不能从 NES 手柄派生。**<br>**① 为什么不派生。** §7.2 要 L→Z、R→X。NES 手柄没有肩键，而**默认布局下 Z 与 X 已经绑给 A 与 B** —— 从 `joy[]` 派生会让 L 变成第二个 A。这不是「猜错」，是**绑定关系本身**的冲突，所以上一轮（r40 ⑤）刻意留空而不是给个看起来能用的答案。<br>**② 关键事实：`g_keyState` 是全 scancode 表。** `drivers/Qt/input.cpp:1366` 对每个 `SDL_KEYDOWN` / `SDL_KEYUP` 都写它，**与该键是否绑给 NES 无关**；`getKeyState(int)`（`input.h:34`）按 scancode 读它。**所以「读一个 NES 侧根本没绑的键的状态」是支持的**，肩键因此可以独立于 NES 绑定工作。<br>**③ 分层：键盘状态由 Qt 层推入，核心库完全不知道键盘存在。** `gba_load.h` 新增 `fceu11_gba_set_shoulder_keys(bool, bool)`，Qt 侧每帧调 `getKeyState(SDLK_z)` / `getKeyState(SDLK_x)` 推入。**这一条是被编译错误逼出来的、但比原方案更好**：我先在 `gba_load.cpp` 里 `#include "Qt/input.h"`，编译报 `C4430` 与 `C2065: FAMILYKEYBOARD_NUM_BUTTONS 未声明` —— **正是 AGENTS.md 与 r40 记的那颗雷**（`input.h:137` 用了 `FAMILYKEYBOARD_NUM_BUTTONS` 而没有任何东西先定义它）。于是顺带解决了「核心库不该知道键盘」这件事，而不是绕过 include 顺序去凑。<br>**④ 跨线程纪律没有变差，但也记下了。** `g_keyState` 由 GUI 线程写、模拟线程读；这与 `joy[]` 完全是同一档（同一趟 GUI 更新写、模拟线程读）。**不是新增风险，但新增了一处**，所以写进了 `gba_load.h` 的注释。<br>**⑤ 绑定表仍不做。** 这两个键是**硬编码的 Z / X**，改键要去「键位配置」那件工作。**刻意不新增一个设置对话框看不见的配置项** —— 那会变成第二个事实来源。临时改法写在注释里：改那两行。<br>**⑥ 白名单不增条目**（`gba_load.*` 与 `fceuWrapper.cpp` 都已在内），**零 vendor 改动、零新增 C ABI 函数**（L / R 仍走 S2-b4 阶段 1 就有的 `gba_set_buttons`）。门禁：全量 Release `BUILD_EXIT=0`；`ctest` **33/34**；锁测试仍 **234**（本阶段是 C++ 侧接线，**无新增锁测试 —— 这是一个覆盖缺口，见 ⑦**）；三条守卫仍全绿。<br>**⑦ 覆盖缺口，必须说清。** L / R 的**映射正确性**（「按下 Z 键，游戏真的看到 L 被按下」）**没有自动化测试**：它在 C++ 侧、依赖真实键盘事件，而本仓库没有 C++ 夹具，**只有装了 `.gba` 的人按一下键才知道**。本轮能验的只有「接线编译通过 + 三条守卫仍成立 + NES 未回归」。

| **r48** | 2026-10-02 | **S3-3：即时存档的 C++ 侧接线。L13 的消解条件被实测推翻 —— 计划里为它写的理由有一半不成立，而 L13 仍开放。**<br>**① 先把 L13 要求的数字测出来，而不是照 r30 的印象选格式。** L13 原文写「即时存档会有**可感知的停顿**，文件也偏大」，并把消解时点钉在「S3 之前（即时存档接线时）」。新增 `save.rs` 的 `#[ignore]` 测量工具（`measure_a_state_on_disk`，照 r37 那条栈探针的形状：**只打印、不设阈值**），Release 实测 20 次：<br>· 单个存档 **1,047,799 B = 1023.2 KiB**（header 8 + payload）<br>· 编码 **2.4 ms**、解码 **5.3 ms**（含工作线程 spawn，走的是 C ABI 的同一条路）<br>· 原始机器状态 **466,464 B** ⇒ **JSON 膨胀 2.25 B/字节**<br>· **Debug 对照：编码 78 ms、解码 82 ms**（慢 32×/15×）<br>**「可感知的停顿」这半句是在 Debug 下写的**，而产品是 Release。r30 记「解析耗时以毫秒计」本身没错，错在把 Debug 的毫秒当成了玩家的毫秒。**按纪律 2，出口判据的改写记账在此，不静默放宽**：L13 的「影响」据实改为「Release 下编码 2.4 ms / 解码 5.3 ms 不可感知；文件 1.0 MB/槽，10 槽约 10.5 MB」，「消解时点」由「S3 之前」改为**本轮已交付**（维持 JSON，用户裁决 2026-10-02）。<br>**② 为什么维持 JSON 而不是换格式。** 换二进制 serde 能到 466 KB（2.25×），base64 能到约 622 KB（1.68×），但：可感知的停顿这个唯一的实质理由已被 ① 证伪；1.0 MB 对一个含 466 KB 真实机器状态的存档是正常量级；而 r30 ② 已记录 postcard/bincode 对含六个 `Vec<u8>` 的状态**编译不过**，重开验证是独立一轮。**代价照实记下**：10 个槽位累计约 10.5 MB。<br>**③ 分支落点：`FCEUSS_Save` / `FCEUSS_Load` 是唯一收敛点，这是查码结论不是设计偏好。** 十个槽位、Save/Load State As、F8/F9（`ConsoleFile.cpp:389,399`）、热键（`src/input.cpp:1065,1082`）**全部**汇到这两个函数。故分支插在「文件已开、序列化器未到」之间：**文件名生成、备份拷贝、undo 记账、槽位状态标记全部复用**，只有序列化那一步换掉，两个分支共 58 行纯新增、零删除。菜单因此免费获得。<br>**④ 菜单门控此前是灰的，且原因是结构性的。** `FCEU_IsValidUI` 判 `!GameInfo`，而 `.gba` 装载时 `ResetGameLoaded()` → `FCEU_CloseGame()` 已把 `GameInfo` 置空（`fceu.cpp:201-202`）—— `GbaLoad` 从不构造它。于是拆成两组（`fceu.cpp:1369-1390`）：**savestate 六项**（`QUICKSAVE`/`QUICKLOAD`/`SAVESTATE`/`LOADSTATE`/`NEXT`/`PREVIOUS`）加 GBA 例外；**录像三项与 `VIEWSLOTS` 保持 `GameInfo` 判据** —— 录像是 NES 专属状态，按不变式 9 须**显式声明不可用**而非给一个不能用的。用户裁决 2026-10-02：F8/F9 与 1–9 槽位一起放行（它们本就是同一条路径，分开会出现「槽位能存、快捷键不能」的不一致）。<br>**⑤ 线程纪律没有变差。** `gba_savestate_*` 要求模拟线程；Qt 菜单在 `FCEU_WRAPPER_LOCK()` 下调用，而 emu 线程必须在 `fceuWrapperUpdate` 里 `fceuWrapperTryLock` 成功才能推进（`fceuWrapper.cpp:1328`）—— **GUI 持锁期间 emu 线程根本跑不动**，热键路径则本就在 emu 线程上。**这与下面两行的 NES 序列化器站在完全相同的地面上**，不是新增风险。<br>**⑥ 读档后的音频不用 C++ 操心**：`frame.rs:449-455` 已在 load 内重建音频环、挂回时钟并恢复残余，S3 出口判据「往返后音频不哑」**零 C++ 工作**。<br>**⑦ 槽位落点不会串到 NES**：`GetFileBase()` 在 `fceu.cpp:459` 对每个文件（含 `.gba`）调用，故 GBA 槽位是 `<gba基名>.fc1`；且 `gba_savestate_load` 先比对 ROM 指纹再动手，**跨卡带读档是被拒绝而非损坏**。我把「`FileBase` 会残留上一局 NES 的值所以会覆盖别人的存档」这个推断查到底后**证伪了它** —— 记下来是因为它看起来非常可信。<br>**⑧ 白名单新增 `state.cpp`（6 → 7 条），并新增一条守卫。** 理由写在 `decoupling.rs` 的 ALLOWED 注释里。守卫 `the_savestate_branch_is_the_only_gba_in_the_state_file` 断言 `state.cpp` 的 GBA 提及**只**出现在那两个函数内 —— 因为「允许这个文件」不等于「允许它任何地方都能判断当前是不是 GBA」。**断言是双向的**：同时断言分支仍在（否则「文件里没有 GBA 提及」会让下面那条空过，这正是 r36 记过的失效形态）。**它的边界照实说**：这是函数级检查，同一函数内多一句 GBA 判断仍会通过。**两处变异验证均精确转红**：把分支挪进 `FCEUSS_LoadFP` → 新守卫红并报出 `(539, "FCEUSS_LoadFP")`；把 `state.cpp` 移出 ALLOWED → `gba_references_stay_inside_the_allowlist` 红并列出该文件。<br>**⑨ 零 vendor 改动、零新增 C ABI 函数**（复用 S2-b2 的三个 savestate 导出），**导出符号仍 34**，锁测试 **234 → 235**（+1 即新守卫；另加 1 条 `#[ignore]` 测量工具）。<br>**⑩ 覆盖缺口，必须说清**：分支的**运行期正确性**（「按了 F5，槽位里真的出现了 GBA 的状态，读回来机器真的回到那一刻」）**没有自动化测试** —— 它在 C++ 侧、依赖真实文件系统与菜单，本仓库没有 C++ 夹具。本轮能验的是：接线编译通过、两条守卫成立（含变异验证）、NES 未回归。**这是与 S3-2 ⑦ 同型的缺口，第三次出现在 C++ 接线阶段。** | 实施 + 两处变异验证 + 实测推翻 L13 的消解理由 |<br>**① 为什么不派生。** §7.2 要 L→Z、R→X。NES 手柄没有肩键，而**默认布局下 Z 与 X 已经绑给 A 与 B** —— 从 `joy[]` 派生会让 L 变成第二个 A。这不是「猜错」，是**绑定关系本身**的冲突，所以上一轮（r40 ⑤）刻意留空而不是给个看起来能用的答案。<br>**② 关键事实：`g_keyState` 是全 scancode 表。** `drivers/Qt/input.cpp:1366` 对每个 `SDL_KEYDOWN` / `SDL_KEYUP` 都写它，**与该键是否绑给 NES 无关**；`getKeyState(int)`（`input.h:34`）按 scancode 读它。**所以「读一个 NES 侧根本没绑的键的状态」是支持的**，肩键因此可以独立于 NES 绑定工作。<br>**③ 分层：键盘状态由 Qt 层推入，核心库完全不知道键盘存在。** `gba_load.h` 新增 `fceu11_gba_set_shoulder_keys(bool, bool)`，Qt 侧每帧调 `getKeyState(SDLK_z)` / `getKeyState(SDLK_x)` 推入。**这一条是被编译错误逼出来的、但比原方案更好**：我先在 `gba_load.cpp` 里 `#include "Qt/input.h"`，编译报 `C4430` 与 `C2065: FAMILYKEYBOARD_NUM_BUTTONS 未声明` —— **正是 AGENTS.md 与 r40 记的那颗雷**（`input.h:137` 用了 `FAMILYKEYBOARD_NUM_BUTTONS` 而没有任何东西先定义它）。于是顺带解决了「核心库不该知道键盘」这件事，而不是绕过 include 顺序去凑。<br>**④ 跨线程纪律没有变差，但也记下了。** `g_keyState` 由 GUI 线程写、模拟线程读；这与 `joy[]` 完全是同一档（同一趟 GUI 更新写、模拟线程读）。**不是新增风险，但新增了一处**，所以写进了 `gba_load.h` 的注释。<br>**⑤ 绑定表仍不做。** 这两个键是**硬编码的 Z / X**，改键要去「键位配置」那件工作。**刻意不新增一个设置对话框看不见的配置项** —— 那会变成第二个事实来源。临时改法写在注释里：改那两行。<br>**⑥ 白名单不增条目**（`gba_load.*` 与 `fceuWrapper.cpp` 都已在内），**零 vendor 改动、零新增 C ABI 函数**（L / R 仍走 S2-b4 阶段 1 就有的 `gba_set_buttons`）。门禁：全量 Release `BUILD_EXIT=0`；`ctest` **33/34**；锁测试仍 **234**（本阶段是 C++ 侧接线，**无新增锁测试 —— 这是一个覆盖缺口，见 ⑦**）；三条守卫仍全绿。<br>**⑦ 覆盖缺口，必须说清。** L / R 的**映射正确性**（「按下 Z 键，游戏真的看到 L 被按下」）**没有自动化测试**：它在 C++ 侧、依赖真实键盘事件，而本仓库没有 C++ 夹具，**只有装了 `.gba` 的人按一下键才知道**。本轮能验的只有「接线编译通过 + 三条守卫仍成立 + NES 未回归」。
| **r49** | 2026-10-02 | **S4 首次实测：真实商业卡带黑屏。根因是 stub BIOS 的启动序列错了两处，且被自己的锁测试固化了 —— 已修。这不是 Alpha 阶段的正常现象，是一个可精确归因的缺陷。**<br>**① 症状**：用户打开 `Mario Kart - Super Circuit (USA).gba`，黑屏。<br>**② 先排除前端的每一层，再怀疑核芯。** ROM 头部合法（`0xB2=0x96`、maker `AMKE`、入口 `0xEA00002E`）；运行日志有 `GBA: loaded ...`；8 秒窗口 CPU 增加 10,859 ms（**135.7% 单核**）⇒ 机器在推进、不是 halt；每帧分支（`fceuWrapper.cpp:1395`）、`blitUpdated` 门控、`renderGbaFrame()` 全部在位。**接线不是原因。**<br>**③ 用探针定案。** `frame.rs` 新增 `#[ignore]` 的 `probe_a_real_cartridge`（照 r37 栈探针的形状：**只打印、不设阈值**），跑真实卡带并报告 PC 落点、屏幕点亮比例、配色分布。第一次结果：<br>· PC **30,000 个采样全部**在 unmapped 区，值是 `0xEAC70DE4` 这类垃圾；<br>· **一次都没进过 cart ROM** ⇒ 游戏代码从未执行；<br>· 屏幕 38,400 像素全黑，唯一颜色 `rgb(0,0,0)`。<br>**④ 成因两处，都在 `bios.rs`：**<br>· **`CART_HEADER_ENTRY = 0x0800_00C0`。** GBA 卡带头 `0xC0` 是「Nintendo Reserved」槽，存的是**一条 ARM 跳转指令本身**；本 cartridge 那里是 `0xE3A00012` = `mov r0, #0x12`，即游戏自己的第一条真实指令。真实 BIOS 从不读它，直接跳 `0x08000000`，由 ROM 自己的 `b`（offset `0x2E` → 目标 `0xC0`）接手。<br>· **多余一次解引用。** 启动码在读到该「地址」后还 `LDR r2, [r2]`，即假定卡带存的是**指向入口的指针**。GBA 卡带头没有这种字段。**我第一次只改前者时就是卡在这里** —— 它把 ROM 偏移 0 的指令 `0xEA00002E` 当成了指针。这条「只改一处不够」是实测撞出来的，不是推演。<br>**⑤ 为什么锁测试全绿。** `bios.rs` 的 4 条锁测试、`gate.rs`、`frame.rs`、`swi/mod.rs` 造的合成 ROM **全部按这个错误约定**（往 `rom[0xC0..0xC4]` 写 `0x08000000`），所以自洽。**r33 ⑤ 把它当成「真收获」记进了本表**（「stub BIOS 不执行卡带头 0x00 的分支，而是从字面量池读地址，`0xC0` 必须放入口地址字面量、代码从 `0xC4` 起」）—— **一个假设出来的约定，被记成了规格，又被拿去造测试。** 这与 r25 / r28 同一类：**自建测试全绿，靠真实参照才发现**。r33 ⑤ 已在本轮据实订正。<br>**⑥ 修法与验证。** `CART_ENTRY = 0x08000000`、删掉解引用、boot 码收敛成 5 条指令（字面量池从 `BOOT_CODE+0x14` 起）。4 条锁测试改造成**真实 GBA 约定**的 ROM（偏移 0 处一条真 `b`，代码在 `0xC0`）。探针复跑：PC 在 cart ROM **29,997/30,000**，第一个到达地址 `0x0805F6A6`，屏幕 **38,400/38,400** 点亮，`CPU halted: true`（游戏主循环等 VBlank，属正常）。**启动阻断被这两处解释干净。**<br>**⑦ 变异验证两处，其中一处先失败后修好 —— 这条比绿本身更值得记。**<br>· 变异 A（`CART_ENTRY` 改回 `0xC0`）**第一次跑 236 项全绿**。原因是我把防回归写成 `word(BOOT_LITERALS + 8) == CART_ENTRY` —— **断言引用了被测常量本身**，改常量等于改断言。改为写字面量 `0x0800_0000` 后精确转红（`left: 0x080000C0, right: 0x08000000`）。**「用被测常量写断言」是无效测试，与 r43 记的「通过原因和断言名字不是同一条路径」同类。**<br>· 变异 B（解引用加回）→ **大面积转红**，含 `gate.rs` 的真实指令流门禁。<br>**⑧ 遗留的第二个问题（不是黑屏的成因）**：修好后画面恒为 `rgb(31,31,31)`，30M 步内**只有 1 种配色** ⇒ 游戏启动后卡在某处不再变化。LCD 通路通、CPU 在跑游戏代码，所以这是**独立的一层**，需单独查。<br>**⑨ 附带清理**：`gate.rs` 与 `swi/mod.rs` 里按旧约定写的注释与字面量写入一并去掉 —— 那是把错误约定留在代码里的地方。**零 vendor 改动、零新增 C ABI、导出符号仍 34。** 锁测试 **235 → 236**（+1 即新增的防回归）；两态 `cargo check` 通过。 | 探针定案 + 两处变异验证（含一次无效测试的自查与重写） |

| **r50** | 2026-10-02 | **S4 第二轮：黑屏的真正原因 —— S2-b4 阶段 2' 只给三个视频驱动中的一个写了 GBA 分支。已补齐，并把「守卫看不见缺失」这个盲区本身变成会红的东西。**<br>**① 症状与 r49 的矛盾**：r49 修好启动序列后**仍是黑屏**，而核芯探针显示画面是**全白、100% 点亮**。白和黑不可能同时成立 ⇒ 帧一定在核芯之后被丢掉。**我此前说「接线不是原因」只覆盖了核芯侧，是判断失误。**<br>**② 用运行时数据定位，而不是读代码。** 在 `renderGbaFrame` 与 `transferVideoBuffer` 各加一次性诊断后：`renderGbaFrame` **一行都没打印** ⇒ 它从未被调用；门控打印 `blitUpdated=1 viewport=0x… gba=1` ⇒ 三项全对，帧在产出、门在放行。断点因此锁定在「运行时用的是哪个 viewer」，而不是重绘逻辑本身。<br>**③ 根因：三个驱动的绘制入口不同，而只有一个认 GBA。** `ConsoleViewerSDL` 的 `queueRedraw()` 直接调 `render()`，那里有 GBA 分支；而 `ConsoleViewerQWidget` 与 `ConsoleViewerGL` 的 `queueRedraw()` 只调 `update()`，真正画画的 `paintEvent` / `paintGL` 里**没有任何 GBA 分支**。运行日志为 `initializeGL` + `GL Version: 3.3.0 NVIDIA 582.28` ⇒ **有独立显卡的机器基本都选 OpenGL 驱动**，于是 GBA 会话一路走到最后一步却被丢掉。**S2-b4 阶段 2' 当时记的是「阶段 2' 已完成」，而它的验证只覆盖了 SDL 这一个驱动。**<br>**④ 为什么所有守卫都绿 —— 这条比缺陷本身更值得记。** 白名单守则是「**提到** GBA 的文件要登记」，而没分支的文件**根本不提 GBA**，守卫连看都看不到它。**这是一条只能抓「已存在」、抓不到「缺失」的盲区**，与 r49 ⑦ 的「用被测常量写断言」同类：两个测试都通过，而它们要防的东西正好在它们的盲区里。故本条同时补代码**和**把盲区变成守卫。<br>**⑤ 改动**：`ConsoleViewerQWidget` 加 `renderGbaFrame()`（`QImage` 直接包住核芯缓冲，是 view 不是拷贝，零逐帧复制），分支在 NES 缩放数学之前，**NES 那段逐字不动**；`ConsoleViewerGL` 加自己的纹理与 `buildGbaTexture()` / `renderGbaFrame()`，复用既有 shader/VAO/投影设施，分支在 `GameInfo` 判断之前（GBA 会话按构造没有 `GameInfo`，落下去只会画背景图）。**上传格式用 `GL_RGBA` 而非 NES 那侧的 `GL_BGRA`** —— 按 BGRA 传 RGBA 字节会红蓝互换，在灰白启动画面上几乎看不出来，换成真游戏就是整屏颜色错。纹理在 `cleanupGL` 一并释放。白名单 **8 → 12**。<br>**⑥ 这次守卫自己错了两轮，都已修（与 r49 ⑦ 同型）：**<br>· 只查「文件提到 GBA」，被 `if (false && fceu11_gba_active())` 绕过 —— `renderGbaFrame` 还在文件里；<br>· 改查「函数体内」后仍绿，因为**函数边界判定有 bug**：先按「下一个 `void`」截断（文件内有辅助函数时提前结束），再按「同类的下一个成员」截断（`paintGL` 是最后一个成员，body 延伸到文件末尾，把后面函数里的 GBA 字样算了进来）。**改用括号配对界定函数体后，删掉 GL 分支才真正转红。**<br>· **已知边界并写进注释，不假装没有**：分支「在但不可达」仍抓不住，那是文本匹配的固有性质，判可达性需要真解析器。<br>**⑦ 实测（两个真实商业卡带，OpenGL 驱动，抓窗口像素）**：<br>· `Super Mario Advance 4`：**语言选择画面正常渲染** —— 文字、精灵、棋盘格边框、调色板全部正确，`BETA` 水印在左上角。**这是 GBA 画面第一次真正出现在屏幕上。**<br>· `Mario Kart - Super Circuit`：**纯白画面**，与 r49 探针一致（`rgb(31,31,31)`、30M 步只有 1 种配色）。⇒ **viewer 已修好、帧确实上屏；但该游戏自身卡在第二层**，属独立缺陷，不是本条成因，也不是 r49 的成因。<br>**⑧ 门禁**：锁测试 **236 → 238**（+2 即两条新守卫）、两态 `cargo check` 通过、全量 Release `BUILD_EXIT=0`、`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`）。**临时诊断代码已全部撤除。** **零 vendor 改动、零新增 C ABI、导出符号仍 34。** | 运行时诊断定案 + 补两个驱动 + 盲区变守卫 |

| **r51** | 2026-10-02 | **S4 收尾：画面铺满 + 按键送达；clementine 审核；知识库三篇；CHANGELOG。**<br>**① 画面只占窗口一小块，根因是三个 viewer 各自写着 `if (scale > 1.0f) scale = 1.0f`（`ConsoleViewerSDL.cpp:714` / `ConsoleViewerQWidget.cpp:446` / `ConsoleViewerGL.cpp:825`），而 NES 侧的 `xscaleTmp = view_width / nesWidth` **没有这个上限**。改为**整数倍 + 信箱边**（用户拍板策略 A）：非整数缩放会让平涂精灵与文字发糊，整数倍不会；窗口小于 240×160 时退回按比例**缩小**而非裁剪 —— 裁剪出的是游戏碎片，比小而正确的图更糟。**缩放算术收进一个函数** `fceu11_gba_draw_size()` 由三个驱动共用：同一份决策写三遍等于承诺三处永远一致且无机制保证，r50 的三个驱动分叉就是这么来的。<br>**② 按键完全无效，根因有三层，每层都只有实测能发现**：<br>· `joy[0]` 由 `input.cpp:231` 的 `UpdateGP` 从 **NES 手柄端口寄存器**搬运，GBA 会话下 NES 核心不跑 ⇒ 无人写 ⇒ 恒 0。八个面键与方向键全失效（L/R 当时走 `g_keyState` 所以侥幸活着）。<br>· 改读用户绑定表 `GamePad[0].bmap` 后**仍全 0**：**方向键默认绑在摇杆轴上**，`ButtonNum = 0x8000 \| axisSign<<14 \| axisIdx`（`sdl-joystick.cpp:621`），把轴标志当 SDL 键码传给 `getKeyState()` 越界返回 0。**这一层是加诊断打印绑定表才看见的**，纯读代码想不到。<br>· NES 与 GBA 的方向键**顺序不同**（NES 上/下/左/右；GBA 右/左/上/下），按位置硬映射会做出「每个键都有反应、方向全错」的摇杆。<br>修法：在 `input.h` 暴露 `testButtonBinding()` 转调本来就懂两种绑定形态的 `DTestButton`（**不复制其逻辑**），GBA 与 NES 走同一条路；且读的是**用户实际绑定**，改键能跟着走。`joy[]` 一个字未动 —— 那是 NES 侧语义，动它违反不变式 9。<br>**③ 实测证据与它的边界**：`Super Mario Advance 4` 语言选择画面正常渲染且**按整数倍铺满**（截图核对）。按键侧，诊断输出为 `[GBA] pad=0x0080` 且 32 个绑定中**只有 Down 读出 1**（`0x0080 = kGbaPadDown`）⇒ 注入有效、绑定读到按键、掩码位序正确。**但「游戏真的响应了」未验证通过**：前后两帧逐像素相同（可能需要 A 键确认，也可能是游戏侧）。**掩码对 ≠ 游戏收到，两者不合并陈述。**<br>**④ 锁测试 238 → 242**（+4）：两条钉缩放规则（整数倍、保持 3:2、为能放下的最大倍数；窗口过小时缩小而非裁剪），两条钉键位映射（面键同位、方向键是**置换**而非同一顺序；掩码原样送达核芯）。**变异验证**：方向键退回 `1 << nes_bit` → 精确转红。<br>**⑤ 我自己写测试时错了两次，都由变异验证而非阅读发现**：① 第一版断言「Up 必须 ≠ 自己的位」在注释里写成「必须 =」，逻辑自相矛盾；② 第二版断言「排序后的置换 == 排序后的集合」**对任何置换都成立**，分不出「重排」与「原样」，毫无判别力。**这与 r49 ⑦ 的「用被测常量写断言」同类 —— 测试写成什么样，和它能不能抓东西是两件事。**<br>**⑥ clementine 审核（`docs/audit/gba-third-party.md`，新目录）**：结论是**分层混合，不是「参考重实现」**。硬件核心（CPU/总线/内存/PPU/定时器/DMA/串口/声音）是上游原样 vendor 的 **20,084 行**；BIOS、16 个 SWI、C ABI、音频采样链、savestate、电池存档、解耦守卫是本项目一手的 **11,485 行**（上游占 63.6%）。补丁集实测 **3 文件 / +202 −4 行**（`git diff --numstat 6f11daf..HEAD`），与 `ATTRIBUTION.md` §4 记的「16 处、跨 3 文件」一致 —— **「16 处」是逻辑改动点，「202 行」是物理行数，两个数都要给**。MIT 合规七项核对六项 ✅，**一项 ⚠️ 需动作**：`gba-core/LICENSE`（MIT 全文 17 行）在树里，但 `scripts/copy_dependencies.ps1` 只拷 DLL、不处理许可文件，**发布前必须确认它进了发行包** —— MIT 的「随分发提供全文」是硬义务，没有代码能替代流程。<br>**⑦ 知识库三篇**（`docs/tech/`，均为规则型知识而非过程记录，已登记进 `README.md`）：`multi-machine-integration.md`（接第二台机器：封闭耦合清单、两类守卫、跨系统状态读取）、`gba-traps.md`（四种让测试看起来在保护你实际不保护的形态 + 提交前三问）、`ffi-boundaries.md`（手写 ABI 与 cbindgen 分工、生成头双 target 争用、大对象出参、栈约束实测、safe vs isolated）。<br>**⑧ CHANGELOG** 补齐 S3-1/2/3 与 r49/r50/探针的全部条目 —— 此前 CHANGELOG 里只有 r35 之前的记录，**S3 三个阶段与两个 P0 修复都还没进去**。<br>**⑨ 门禁**：锁测试 **242 全绿**、两态 `cargo check` 通过、全量 Release `BUILD_EXIT=0`、`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`）。**零 vendor 改动、零新增 C ABI、导出符号仍 34。** | 实测 + 三轮变异验证 + 上游审核 |

| **r52** | 2026-10-02 | **输入设置「设好键位、关掉再打开就全空」：根因是一条会自我复现的链，不是文件损坏。三处已修，并用真实 UI 往返验证读写闭环。**<br>**① 症状**：用户设好键位，关掉输入设置，再打开 —— 十个键全空。<br>**② 根因有四步，每一步单看都像正常代码，合起来构成闭环。** ① 配置文件里 `SDL.Input.GamePad.0.Profile = 默认`，`GamePad_t::init` 据此调 `loadProfile("默认")` 去读 `input/keyboard/默认.txt`；② 该文件**存在**，内容是 `a:k,b:k,…,turboA:k,turboB:k` —— 每个键都是一个**没有键名的裸 `k`**；③ `convText2ButtConfig` 走 `k` 分支调 `SDL_GetKeyFromName("")` 得 `SDLK_UNKNOWN`，`ButtonNum` 落到 -1；④ `getMapFromFile` 当时**只要 `fopen` 成功就 `return 0`**，于是 `loadProfile` 报成功、**`loadDefaults()` 永不执行**，`DefaultGamePad` 里真正的默认值（f/d/s/Return/方向键）根本没机会上场。**闭环的关键在下一步**：对话框的「保存」把这份空白映射原样写回磁盘（`SDL_GetKeyName(-1)` 得到空名，于是又写出裸 `k`），**所以每次重开都复现，且是程序自己造成的，不是用户清错了**。<br>**③ 我自己写错了一次，是被实测纠正的。** 第一版校验查「字段非空」，而 `a:k` **非空** —— 校验通过、缺陷原样保留、实测仍全空。判据必须是「**能否解析成真实按键**」而不是「有没有内容」。**这与 r49 ⑦ / r51 ⑤ 同类：测试写成什么样，和它能不能抓东西是两件事。**<br>**④ 三处修改。**(a) 新增 `buttConfigTextIsUsable()`，判据与 `convText2ButtConfig` 的分支**逐条对齐** —— 两者必须对「什么算可用」达成一致，最省的做法是让一个从另一个派生；`getMapFromFile` 十个字段全不可解析时 `return -1`，调用方随之回退 `loadDefaults()`。(b) `getDefaultMap` 键盘分支填完 `DefaultGamePad` 后原 `return -1`，改为 `return 0` —— **填好了还说失败，会让上层去找一个并不存在的 profile**，而在按返回值行事的路径上还会把默认值再盖一遍。(c) `saveCurrentMapToFile` 跳过 `ButtonNum < 0` 的字段 —— 未绑定就写空名正是毒化格式的来源；**省略字段读回来仍是「未绑定」，语义一致且不毁掉其余九键**。<br>**⑤ 顺带修掉一个同源的隐患**：读文件时的 `i < 32` 门槛（原注释「至少 32 字符才算有效行」，本意是照 SDL GUID 的长度），而**键盘 profile 不是 GUID 开头、且 routinely 短于 32** ⇒ 一个完全合法的 profile 可能**一行都没活下来**。改为「非空且至少两个逗号分隔字段」。后半句不是洁癖：`parseMapping` 无条件把前两个字段读成 GUID 与名字，**单字段行会让它读到未初始化的缓冲区**；原来的 `i < 32` 恰好顺带挡住了空行，**新判据必须继续挡住**，否则等于把「静默丢行」换成一处未初始化读。<br>**⑥ 实测（真实 UI 往返，非单元测试）**：① 打开「选项 → 输入配置 → 端口 1 → 设置」，十键为 `F D S Return Up Down Left Right E R` —— **TurboA=E、TurboB=R**（用户指定）⇒ 毒化文件被拒、默认值接管；② 点「保存」，磁盘文件由 102 B 的 `a:k,b:k,…` 变为 212 B 的 `a:kF,b:kD,…,turboA:kE,turboB:kR`，config 1–3 无绑定则该行只剩 `config:N`、**不再写出裸 `k`**；③ **判别性验证**：把 A 键「清除」后保存，**重启进程**再打开 —— **A 仍为空，其余九键从文件逐一恢复**。**若走的是默认值回退，A 会显示 F；它没有**，故文件确实被读取，读写闭环成立。<br>**⑦ 这条修复没有锁测试 —— 这是缺口，如实记下。** 改动全在 Qt 驱动（`fceux11_drivers_qt`），而本项目的锁测试面是 Rust 侧与 NES 精度侧，**没有可挂载 C++ 输入解析的测试位**。替代证据是第 ⑥ 条的 UI 往返实测（含一次有判别力的「清除后重开」），**它比单元测试弱**：不覆盖手柄轴/hat/多 config 组合，也没有回归网。**若日后有人再动 `getMapFromFile` 的判据，没有任何东西会自动转红。**<br>**⑧ 门禁**：Release 重编并重链成功（`fceux11.exe` 时间戳随之更新，**新增代码零警告**，四条既有警告均为改动前就有）、`ctest` **33/34**（唯一失败仍是与 v2.0 无关的 `bench_tolerance_test`，L11）。**零 vendor 改动、零新增 C ABI、导出符号仍 34，临时诊断代码已全部撤除。** | 根因四步链 + 判别性 UI 实测 |