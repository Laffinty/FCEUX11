# FCEUX11 v2.0 构建计划 — GBAEUX11 模块（GBA 运行能力移植）

> **STATUS: OPEN**（2026-09-27 立项，尚未开工）
> **模块名**：**GBAEUX11**（FCEUX11 Rust 侧的第二模拟核心）
> **版本**：v2.0（BETA 阶段）
> **日期**：2026-09-27 立项 / 2026-09-27 二次修订（M4A 延期至 GA 后）/ 三次修订（音频接口契约更正为 `WriteSound(int32*)`）
> **分支**：S0 开工时创建
> **前置**：v1.18.1 已发布（`main` @ `3e33f2b`）；F11QA R4 gate green，grade B
> **关联**：`src/rust/crates/f11qa/README.md`、`docs/tech/precision.md`、`COPYRIGHT_AUDIT.md`、`DERIVATIVE_WORK_NOTICE.txt`
> **路线图位置**：v1.15 C++ 现代化 → v1.16 KagamiQA 闭环 → v1.17 统合与分级 → v1.18 残留精度长期演进 → **v2.0 跨平台扩张（首次引入第二主机）**

---

## 〇、TL;DR

v2.0 只做一件事：**让 FCEUX11 能运行 GBA 游戏**，前端完全复用现有 Qt 驱动，不重写 UI。

| # | 决策项 | 结论 |
|---|---|---|
| 1 | 模拟核心 | 移植成熟开源 GBA 硬件模拟核心（Rust，MIT） |
| 2 | 前端 | 复用 FCEUX11 现有 Qt 驱动，仅加 GBA 渲染/音频接线 |
| 3 | BIOS | **默认自建 HLE BIOS**（源自 mGBA，MPL-2.0）；用户若提供真 BIOS 则优先 |
| 4 | M4A | **延期至 2.0 GA 之后**（见 §六；BETA 不含此项，不阻塞发布） |
| 5 | 测试 | **暂不接入 F11QA**，改手工实测清单 |
| 6 | 标识 | 所有 GBA 画面左上角常驻 `BETA` 水印 |
| 7 | 键位 | BETA 阶段采用 **NES 兼容键位**模式 |
| 8 | 音频 | **复用现有声卡接口**（`WriteSound(int32*, int)`），与 NES 共用设置与 UI（见 §4.2） |

**一句话收束**：GBAEUX11 是与 NES 核心**并列**的第二模拟核心（GBA 不是 mapper 概念，不进 `boards/`）；核心用 Rust 写、经 cbindgen C ABI 接入既有 `fceux11_rust` 静态库，前端零重写。

---

## 一、范围

### 1.1 做

- `.gba` ROM 识别、加载、运行
- 视频输出（240×160）、音频输出（**int32 单声道 / ±32768**，经 `WriteSound` 复用现有声卡接口）
- 存档：`.srm` 电池存档（SRAM / Flash 64K·128K / EEPROM 512B·8K）、即时存档
- 键位：NES 兼容模式（8 键一一映射 + L/R 附加键）
- `BETA` 水印常驻左上角

### 1.2 明确不做（v2.0 BETA 范围外）

| 项 | 理由 |
|---|---|
| 接入 F11QA / KagamiQA | 当前为测试阶段，手工实测先行；待精度收敛后再议（§8.1 P3） |
| **M4A / MP2K 增强混音** | **已决议延期至 2.0 GA 之后**；非可玩性前提，理由见 §六.2 |
| 移植上游调试器 UI | 反汇编器 / 内存查看器 / Pokemon 调试器全部丢弃 |
| 补齐 wait cycle | 上游 issue #204 仅 BIOS 区完成，其余内存区未实现 |
| 上游同步自动化 | 手工评估（见 §九 R1） |
| 触觉 / 陀螺仪 / 太阳传感器 | GBA 外设扩展，BETA 不做 |
| 非 `.gba` 格式（`agb`/`bin` 等） | BETA 只认 `.gba` |

---

## 二、核心选型实证

对上游 GBA 模拟核心（Rust，MIT）实测：

| 维度 | 事实 |
|---|---|
| 许可 | MIT，无附加条款 |
| 核心规模 | 45 文件 / **19,547 行** |
| 构建 | `cargo check` **37s 通过** |
| 依赖 | 仅 `rtrb` / `serde` / `serde_with` / `tracing` — **零 GUI 库** |
| 分层 | 硬件核心 / CLI / 调试器 UI 三分，**仅取硬件核心** |
| 活跃度 | 最后提交 2026-08-27，PGP 签名，近 8 次提交集中于安全加固 |
| Rust 版本 | **edition 2024** — 与 FCEUX11 `src/rust` workspace 一致，无翻译成本 |

> 精确来源见 `crates/f11gba/ATTRIBUTION.md`（唯一权威出处）与 §三 许可矩阵。

### 2.1 上游已验证的精度

其测试套件（对 [jsmolka/gba-tests](https://github.com/jsmolka/gba-tests)）：

- **通过 9 项**：`arm` `thumb` `memory` `nes` `unsafe` `save/{none,sram,flash64,flash128}`
- **失败 1 项**：`bios/bios.gba`，被 `#[ignore]`，注释为 *"BIOS function emulation is incomplete"*

### 2.2 现成接口面

| 接口 | 签名 | 说明 |
|---|---|---|
| 构造 | `Gba::new(bios: [u8; 0x4000], cartridge: &[u8])` | **只需喂 16KB，无需改动核心** |
| 步进 | `step() -> bool` | 返回 `true` 表示进入 VBlank（新帧就绪） |
| 视频 | 240×160，15-bit | 由渲染层持有 framebuffer |
| 音频 | `rtrb::Producer<f32>` | **交织立体声 f32**，已含 DC 阻断滤波 |
| 电池存档 | `battery_data()` / `load_battery()` / `save_dirty` | **存档硬件在核心内**；`.srm` 文件 I/O 不在核心内，需重写（数十行） |

---

## 三、许可与合规

| 来源 | 许可 | 与 GPL-2.0 的关系 | 处置 |
|---|---|---|---|
| GBA 硬件模拟核心 | **MIT** | 兼容 | 保留 MIT 声明；出处集中于 `ATTRIBUTION.md` + `COPYRIGHT_AUDIT.md` |
| mGBA `hle-bios.s` | **MPL-2.0** | **MPL-2.0 §1.12 明确将 GPL-2.0 列为兼容 Secondary License** | 保留 MPL-2.0 头，登记 `DERIVATIVE_WORK_NOTICE.txt` |
| M4A / MP2k | 学术参考 | 仅读不改写 | 不复制代码，独立实现 |

> **MIT 义务说明**：MIT 要求在「所有副本或实质性部分」中保留版权与许可声明。因此无法做到完全匿名，**最低合规做法是在 `ATTRIBUTION.md` 与 `COPYRIGHT_AUDIT.md` 各保留一条出处记录**；目录名、模块名、crate 名一律使用功能性命名，不含上游项目标识。

---

## 四、目录布局

```
src/rust/crates/f11gba/               # 新增 crate
├── Cargo.toml
├── ATTRIBUTION.md                     # 上游来源、许可、基线 SHA（唯一权威出处）
├── vendor/gba_core/                   # 上游硬件模拟核心，原样 vendor
│   ├── bitwise.rs · bus.rs · cartridge_header.rs · gba.rs · ring_buffer.rs
│   ├── cpu/                           # ARM7TDMI / bus / hardware 子模块
│   └── render/
├── src/
│   ├── lib.rs
│   ├── ffi.rs                         # cbindgen 导出（约 10 个 extern "C"）
│   ├── bios.rs                        # HLE BIOS 装载 / 真 BIOS 回退
│   ├── m4a.rs                         # M4A — 2.0 GA 后补齐，GA 前留空占位
│   └── overlay.rs                     # BETA 水印合成
└── build.rs                           # HLE BIOS 汇编

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
int  gba_read_audio(int32_t *dst, uint32_t frames);  // f32 立体声 → int32 单声道（±32768）
int  gba_set_buttons(uint16_t mask);
int  gba_battery_read(uint8_t *dst, uint32_t cap);
int  gba_battery_write(const uint8_t *src, uint32_t len);
int  gba_savestate(uint8_t *dst, uint32_t cap, uint32_t *written);
int  gba_kill(void);
```

### 4.2 音频链路（已核实，复用现有声卡接口）

> **决议（2026-09-27）**：GBAEUX11 **复用 FCEUX11 现有声卡接口**，不自持 SDL 设备。
> 收益：采样率 / 音量 / 缓冲 / 静音 / Turbo 抑制等设置与 NES 共用同一套 UI 与同一份配置，无分叉。

生产侧接口在 `src/drivers/Qt/dface.h`：

```c
int    InitSound();
void   WriteSound(int32 *Buffer, int Count);   // ← GBAEUX11 每帧调用
int    KillSound();
uint32 GetMaxSound();                          // 环形缓冲总容量
uint32 GetWriteSound();                        // 当前剩余可写空间
```

**关键契约（易错点）**：

| 项 | 事实 | 出处 |
|---|---|---|
| 送入缓冲类型 | **`int32`，单声道**（非 i16） | `dface.h:12` |
| 幅度范围 | **±32768**（即已按 int16 幅度产出） | `sdl-sound.cpp:77,167` |
| 设备侧格式 | `AUDIO_S16SYS` / `channels=1` / `samples=512`（≥1024 时改 1024） | `sdl-sound.cpp:256-259` |
| 降位时机 | SDL 回调 `fillaudio` 内 `sample = s_Buffer[s_BufferRead]` **直接截断**，无缩放 | `sdl-sound.cpp:167` |
| 每帧样本数 | `Count = samplesPerFrame = 采样率 / 帧率`；GBA 帧率 59.7275 Hz | `sdl-sound.cpp:263` |
| 背压 | 写前用 `GetWriteSound()` 判剩余空间 | `sdl-sound.cpp:345-348` |
| 音量 | **`WriteSound` 不施加音量**，NES 侧由 `sound.cpp` 混音阶段完成 → **GBAEUX11 须自行应用 `FSettings.SoundVolume`** | `sdl-sound.cpp:354-366` |
| Turbo 抑制 | `WriteSound` 内建 `(NoWaiting & 0x01) \|\| turbo` 时直接返回 | `sdl-sound.cpp:363-367` |

**设备开关与设置**（`src/io_api.h:199-228`）：

```c
namespace fceu11 {
    void Sound(int Rate);                 // 开设备
    void SetSoundQuality(int quality);
    void SetSoundVolume(uint32 volume);
    void SetTriangleVolume / SetSquareVolume / SetNoiseVolume / SetPCMVolume;
}
void FCEUD_SoundToggle(void);
void FCEUD_SoundVolumeAdjust(int);
```

> 旧的 `FCEUI_Sound` / `FCEUI_SetSoundVolume` 已标记 `FCEUX11_DEPRECATED`，新代码一律用 `fceu11::` 命名空间版本。

**故 GBAEUX11 的唯一格式转换点为**：`f32 交织立体声 → int32 单声道`（下混 + ×32768 + 饱和截断），而非转 i16。

---

## 五、BIOS 策略

### 5.1 关键认识：HLE BIOS 是**可执行二进制**，不是 SWI 拦截器

mGBA 的 `src/gba/hle-bios.c` 是 `const uint8_t hleBios[GBA_SIZE_BIOS] = {...}` —— 一段**真实 ARM 机器码**。
故 `Gba::new()` 只需喂不同的 16KB 即可，**上游核心一行都不用改**。

### 5.2 但不能直接拷贝字节数组

`hle-bios.s` 的 `swiBase` 含：

```asm
swieq 0xF00000   @ Special mGBA-internal call to load the stall count into r11
```

mGBA 靠自家 CPU 钩子回填 stall 计数。上游核心无此钩子，会以 comment byte `0xF0` 越界索引 `swiTable` → 野指针。

**正确做法**：移植 `hle-bios.s` **源码**（MPL-2.0），删除该行，**自行 assemble** 出 16KB。功能上该指令仅为计时代码，删除不影响正确性。

### 5.3 覆盖度

`hle-bios.s` 的 SWI 表覆盖 0x00–0x2A 全部 43 项槽位：

| 状态 | SWI |
|---|---|
| ARM 原生实现 | `Halt` `Stop` `IntrWait` `VBlankIntrWait` `CpuSet` `CpuFastSet` `SoftReset` `SoundDriverGetJumpList` |
| 由模拟器 C 侧实现（汇编仅占位计时） | `Div` `DivArm` `Sqrt` `ArcTan` `ArcTan2` `Lz77UnComp*` |
| **未实现（Nop）** | `RegisterRamReset` `GetBiosChecksum` `BgAffineSet` `ObjAffineSet` `BitUnPack` `HuffmanUnComp` `RlUnComp*` `Diff*UnFilter*` 及声音段 0x19–0x24 / 0x28–0x2A |

### 5.4 HLE 的实际可用性（联网核查结论）

| 来源 | 结论 |
|---|---|
| RetroHandheldHQ | mGBA 内置开源 BIOS 替代品，**「99% 的游戏库无需真 BIOS 即可运行」** |
| mGBA 官方 README | Features 列出 *"A built-in BIOS implementation, and ability to load external BIOS files"* |
| PSGamer | HLE *"covers ordinary retail software"*；不复现的是启动画面与少数 demo 读取的底层行为 |
| vgmdocs（权威逆向文档） | MP2k 驱动虽也实现在 BIOS 中，但 **「商业游戏几乎从不调用 BIOS 里的这些系统函数」**，因为游戏自带修正过的兼容驱动 |

**综合结论**：HLE 路线**风险显著低于初判**。零售商业游戏普遍自带音效驱动并在模拟的 ARM7TDMI 上执行，不依赖 BIOS 声音段，故 **§5.3 的「声音段 Nop」对绝大多数游戏无实际影响**。

### 5.5 装载策略

```
用户指定真 BIOS（存在且 16KB） → 优先使用，行为最接近真机
否则                          → 使用自建 HLE BIOS，开箱即用
```

---

## 六、M4A 延期至 2.0 GA 之后

### 6.1 原判断与核查结果

| | 内容 |
|---|---|
| **原判断**（2026-09-27 立项时） | HLE BIOS 下 M4A 游戏静音 → 必须补齐，否则 Gen 3 宝可梦无声 |
| **核查结论** | **原判断不成立。** 游戏自带 MP2k 驱动在卡带 ROM 中运行，调用的是 `m4aSoundInit` / `m4aSoundVSync` / `m4aSoundMain` / `m4aSongNumStart` 等**自身符号**，而非 BIOS SWI。vgmdocs 明确：商业游戏「几乎从不」使用 BIOS 内的那套系统函数 |

### 6.2 决议

> **2026-09-27 决定：M4A 补齐延期至 2.0 GA 之后，BETA 阶段不做。**

理由：
1. **非可玩性前提**。Gen 3 宝可梦等 M4A 游戏在 HLE BIOS 下**照样有声**（驱动在卡带 ROM 中由模拟的 ARM7TDMI 执行），延期不影响 BETA 的核心承诺。
2. **解耦长尾**。M4A 是全计划唯一技术不确定性最高的工作（原估 2–3 周、R4 高风险），移出 BETA 使 S0–S4 工期收敛。
3. **收益是渐进式**。音质提升与 homebrew/demo 兼容属锦上添花，不构成 BETA 阶段的用户可感知缺陷。

### 6.3 GA 后待办（保留技术结论）

| 项 | 内容 |
|---|---|
| 目标 | 将 m4a 调用拦截、以宿主语言重写混音，音质优于原驱动的 sample-and-hold |
| 价值 | 高音质混音；兼容 homebrew / demo / 极少数调用 BIOS 声音 SWI 的作品 |
| 参考 | mGBA、NanoBoyAdvance 的既有做法（**仅参考，不复制代码**） |
| 位置 | `src/rust/crates/f11gba/src/m4a.rs`（GA 前留空占位） |
| 排期 | 2–3 周，待 2.0 GA 后单独立项 |


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
- 硬件支持来自核心（SRAM 32KB / Flash 64K·128K / EEPROM 512B·8K，按 ROM 头部 ID 自动识别）

---

## 八、阶段编排

| 阶段 | 内容 | 出口标准 | 估 |
|---|---|---|---|
| **S0** | 建 `f11gba` crate，vendor 核心，建 `ATTRIBUTION.md` | `cargo check` 过；CMake 产出含新符号的 `fceux11_rust.lib` | 2–3 天 |
| **S1** | 移植 `hle-bios.s`，删 `swieq 0xF00000`，自建 16KB；真 BIOS 回退配置项 | 两种 BIOS 均能启动 ROM 到游戏画面 | 2–3 天 |
| **S2** | C ABI + Qt 前端：`.gba` 识别、240×160 渲染、音频接入 `WriteSound`、`BETA` 水印 | 可玩游戏，画面/音频正常 | 1–2 周 |
| **S3** | NES 兼容键位、`.srm` 存档、即时存档 | 存档跨会话可读；键位符合 §7.2 | 1 周 |
| **S4** | 手工实测清单，逐游戏过 | 清单内游戏全部可玩并记录已知限制 | 长尾 |

**S0–S3 约 3 周产出可玩 BETA**，S4 为持续收敛。

### 8.1 GA 后排期

| 阶段 | 内容 | 估 |
|---|---|---|
| **P1** | M4A 补齐（§六.3） | 2–3 周 |
| **P2** | 精度收敛：wait cycle 补齐、已知限制清零 | 另议 |
| **P3** | 接入 F11QA / KagamiQA（jsmolka 通道） | 另议 |


---

## 九、风险登记册

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| **R1** | **vendor 分叉成本**。上游仍活跃（2026-08-27 有提交） | 上游修复无法自动流入，长期可能需接盘 | `ATTRIBUTION.md` 记录基线 SHA；定期评估合并；补丁集最小化 |
| **R2** | **无商业游戏验证**。jsmolka 是合成测试，9/10 通过 ≠ 主流游戏可跑 | BETA 可能大面积跑不动 | S5 手工实测清单；已知限制逐条编目 |
| **R3** | **wait cycle 大面积未实现**（issue #204 仅 BIOS 区完成） | DMA / 精灵窗口敏感游戏可能异常 | 列入已知限制；S5 重点验证 |
| **R4** | ~~M4A 实现难度未知~~ **已关闭** | 核查确认非可玩性前提；已延期至 GA 后（§六.2） | 不阻塞 BETA |
| **R5** | **巴士因子**。上游 73 star、实质 1–2 人维护 | 上游停摆则需自维护全部 19.5K 行 | MIT 无法律障碍；R1 缓解 |
| **R6** | **水印污染帧校验** | 未来接入 F11QA 时基线不可比 | 开关可关；接入测试时以关闭状态建基线 |
| **R7** | **音频下混失真**。f32 立体声 → 单声道为有损转换 | 音质低于直连 | GBA 原始音源以单声道为主，实际影响有限；S2 实测确认。另须自行应用 `FSettings.SoundVolume`（`WriteSound` 不施加音量，§4.2） |
| **R8** | **BETA 无 M4A 增强**。混音沿用原驱动的 sample-and-hold | 音质弱于 mGBA / NanoBoyAdvance | 已在 §6.2 显式声明；GA 后 P1 补齐 |

---

## 十、不变式

1. **NES 核心零回归**：v2.0 任何改动不得改变 F11QA 现有矩阵（`106P / 14F`，grade B）。`pass_to_fail` 必须为 0，出现即回滚。
2. **前端复用不打折**：不得为 GBA 另起 UI 框架或分叉 Qt 驱动。
3. **BIOS 不夹带任天堂代码**：仓库内只允许自建 HLE 或用户自备路径，绝不提交真 BIOS。
4. **许可可审计**：每条外部来源登记 `COPYRIGHT_AUDIT.md`，保留原始声明；出处集中于 `ATTRIBUTION.md`，代码内不散落上游项目标识。
5. **BETA 水印不可绕过**：BETA 阶段所有 GBA 画面必带水印，无隐藏开关。
6. **HLE 与真 BIOS 双路径均须可用**，不得只保一条。
7. **延期项须显式声明**：M4A 混音未做（§六.2 / R8）是**已决议的范围取舍**，不是遗漏。不得在 BETA 文案中暗示具备该能力。

---

## 十一、状态回写

阶段完成后：

1. 本文件 §八 对应行勾选并更新出口标准
2. `ATTRIBUTION.md` 更新基线 SHA 与补丁集
3. `COPYRIGHT_AUDIT.md` 同步新增来源
4. `CHANGELOG.md` 记入 Added 段
5. 已知限制逐条编目进本文件 §9，不做无据 `known_limit`

---

## 十二、待确认

| # | 事项 | 状态 |
|---|---|---|
| 1 | ~~M4A 是否保留在 BETA 范围~~ | ✅ **已决议：延期至 2.0 GA 之后**（§六.2） |
| 2 | ~~GBAEUX11 音频走哪条路~~ | ✅ **已决议：复用现有声卡接口**，契约见 §4.2 |
| 3 | 阶段分支命名 | S0 开工时确定 |
| 4 | 实测游戏清单（版权与来源） | S4 前确定 |
| 5 | 2.0 正式版是否移除 `BETA` 水印 | 计划移除，GA 前确认 |
| 6 | F11QA 接入时点 | 精度收敛后另议（§8.1 P3） |
