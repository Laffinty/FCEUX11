# FCEUX11 v2.0.3 — GBA 卡带兼容性优化计划

> **STATUS: DRAFT**（业主 2026-11-08 批准立项范围：**三段式** + **卡带不进仓库、判据换成自建最小夹具**）
> **⚠️ r2（2026-11-09）：§一.2.2 的「可触达 panic 点」清单已按源码复核并收窄（6 处更正），§二.2 / §二.4 的夹具清单与出口判据已改写。**
> **动 GBA 兼容性代码前先读 §一.2.2 开头的「更正」小节**——原文清单里 13 条有 11 条不可达，
> **真正可达的只有 OAM 的两条**（shape=3 与 gfx mode=3）。
> **立项目标**：把「商用手写 GBA 卡带跑不起来」这一类问题，从**靠人看**变成**可复现、可证伪、可回归**。
> **基线**：`2960998`（v2.0.2 收口批次 r26 之后）· NES 侧 `ctest -LE perf` **36 项全绿** · F11QA 矩阵 **122 total / 112 passed / 10 failed / grade B / `pass_to_fail` 空** · GBA 锁测试 **249** + vendor **242** + debug **93** + media **52**
> **产出定位**：只立项、排期、定验收；实施按工作项逐个开独立任务，**先 PLAN 后动手**的纪律不变。

---

## 〇、范围与不变式

### 范围内

1. **GBA 卡带兼容性**：让真实商用手写卡带能启动、能出画面、能存读档、不闪退。
2. **可复现门禁**：自建**最小可复现夹具**（合成 ROM），让每一条已定位缺陷都有**一条会红的锁测试**。
3. **探针工程**：把仓库现有的 `cartprobe` / `probe_why_a_game_stalls` / `probe_trace_a_stuck_game`
   升级成能**注入输入 + 长窗口 + 判定卡死点**的取证工具。

### 范围外（排除项）

1. **不碰 NES 侧任何行为**。本轮所有代码改动只落在 GBA 侧
   （`src/rust/src/gba/`、`src/rust/crates/gba-core/`、`src/gba_load.cpp`）。
2. **不把 7 张商业卡带放进仓库**（业主裁决 2026-11-08）。它们只作**本地取证**用，
   仓库内的判据一律换成**自建合成 ROM**。
3. **不做 GBA 精度对齐**（本轮是**能不能跑**，不是**跑得准不准**）。精度另立项。
4. **外部参考实现只读不抄**：mGBA 是 MPL-2.0、FCEUX 上游与本仓库同源 —— 两者都**只读**。

### 不变式（继承 AGENTS.md，红灯即停）

1. **NES 零回归**：任何 GBA 改动后 `ctest -LE perf` 36 项全绿 + F11QA 矩阵 `pass_to_fail = 0` +
   grade ≥ B。金标三件套零漂移。
2. **GBA 长期 Alpha，不阻塞本体发版**（AGENTS.md 不变式 2）。本计划**不设**本体发版门槛。
3. **extern "C" 出口面必须能被安全调用**（AGENTS.md 纪律 6）：导出函数里**不允许**出现
   会 unwind 的 panic —— 这正是本轮「闪退」的一半原因，见 §一.2。
4. **变异验证**（AGENTS.md 纪律 3）：每条新锁测试都要能把「修复前的值」改回去看到它转红；
   **全绿不算证据**。
5. **不凭记忆写位级格式**（AGENTS.md 纪律 4）：位定义一律取 GBATEK 权威快照（§一.5）。
6. **vendor 树补丁集要登记**：改 `src/rust/crates/gba-core/` 必须同步
   `src/rust/crates/gba-core/ATTRIBUTION.md`（逐处说明 + 重新 vendor 流程）。
7. **instrument-first**（`docs/tech/precision.md` §4）：改行为前先有能看见它的读数。

---

## 一、问题陈述与已确认的事实（取证基线）

> 本节是本计划的**事实底座**。它取代被删除的两份旧计划中与 GBA 兼容性相关的部分。
>
> **路径简写约定**（免得看的人找不到文件）：
> `gba-core/src/...` = `src/rust/crates/gba-core/src/...`；
> `hw/` = `src/rust/crates/gba-core/src/cpu/hardware/`；
> `lcd/` = `hw/lcd/`；`lcd.rs` = `hw/lcd.rs`；`lcd/layers/` = `hw/lcd/layers/`；
> `gba/` = `src/rust/src/gba/`；`swi/` = `src/rust/src/gba/swi/`。

### 1.1 实测样本

来源：业主提供的实测报告 `fceux11_v2.0.3卡带兼容性实测报告.odt`（真实卡带 + 真实运行截图，
报告真实性由业主背书）+ 7 个 `.gba` 卡带本体 + 5 个自动生成的 `.srm` 存档。

| # | 卡带 | 容量 | 存档型号（ROM 内 ID 串） | 结果 | 症状（报告原文） |
|---|---|---|---|---|---|
| 1 | Kirby - Nightmare in Dream Land | 8 MB | `SRAM_V112` | ✅ 完全兼容 | — |
| 2 | Need for Speed - Most Wanted | 8 MB | `EEPROM_V124` | ✅ 完全兼容 | — |
| 3 | Super Mario Advance 2 - Super Mario World | 4 MB | `EEPROM_V122` | ✅ 完全兼容 | — |
| 4 | Mario Kart - Super Circuit | 4 MB | `FLASH_V124` | ❌ 不兼容 | 「启动动画和音乐正常，**马上就卡死**，fceux11 无法操作，然后**自动闪退**」 |
| 5 | Dragon Ball - Advanced Adventure | 16 MB | `EEPROM_V124` | ❌ 不兼容 | 「开场动画和音频正常，**按点击按钮后就卡死**，然后**闪退**」 |
| 6 | Legend of Zelda - The Minish Cap | 16 MB | `EEPROM_V124` | ❌ 不兼容 | 「加载/开场/存档/前情提要正常，**进入主线游戏时黑屏**，按键无反应，**不卡死不闪退**，可正常关闭 ROM」 |
| 7 | Pokemon - FireRed | 16 MB | `FLASH1M_V103` | ❌ 不兼容 | 「**无法加载任何画面**，音频刺耳杂音」 |

**存档文件实测**：5 个 `.srm` **全部是 32768 字节**，与卡带真实芯片容量无关
（详见 §一.2.4）；Mario Kart 与 FireRed **没有生成** `.srm`（二者都没跑到存档那一步）。

**ROM 头实测**：7 张卡的 GBA 头**全部合法**（Nintendo logo 在 0x04–0x9F 逐字节正确、
固定值 `0x96` 在 0xB2、标题/游戏码/厂商码在 0xA0/0xAC/0xB0 全部可读）。
⇒ **ROM 头不是问题**。

### 1.2 已定位的根因

#### 1.2.1「卡死」＝ 未实现的 BIOS SWI 落到 stub BIOS 的 SWI 向量，而那个向量是 `b .`

- 模块 `dispatch` 只认领 **16 个** SWI 号码（`src/rust/src/gba/swi/mod.rs:856-900`
  的锁测试即事实源）：`0x01–0x05, 0x09, 0x0A, 0x0D–0x15`。
- vendor core 另服务 **6 个**（`src/rust/crates/gba-core/src/cpu/arm7tdmi.rs:1134`（`handle_swi_hle` 起点））：
  `0x00, 0x06, 0x07, 0x08, 0x0B, 0x0C`。
- **完全没实现的号码**（模块的 `Swi` 枚举里有名字，但 `dispatch` 没有分支 ⇒
  **有名字 ≠ 有实现**）：`0x16/0x17/0x18`（Diff8bit/Diff16bit 反滤波）、`0x19` SoundBias、
  **`0x1A–0x1E`（SoundDriver 五兄弟）**、`0x1F` MidiKey2Freq、
  **`0x20–0x24`（MusicPlayer 五兄弟）**、`0x25` MultiBoot、`0x26` HardReset、
  `0x27` CustomHalt、`0x28/0x29/0x2A`。
- **未处理 ⇒ 永久自旋**：core 拒绝后走真异常路径 PC = 0x08 ⇒
  `src/rust/src/gba/bios.rs:131-134` 的 SWI 向量是 `BRANCH_SELF`
  （`bios.rs:97` `const BRANCH_SELF: u32 = 0xEAFF_FFFE;` = `b .`）
  ⇒ **CPU 在 BIOS 0x08 永久自旋**。
- 仓库**早就知道这条**：`src/rust/src/gba/frame.rs:375-380` 的 `gba_step_frame` 文档写着
  「Bounded so a machine wedged before it ever reaches VBlank -- **a game looping on an
  unclaimed SWI, say** -- returns instead of hanging the emulator」。
- ⚠️ **这条机制目前被两条锁测试锁成了「预期行为」**：
  `src/rust/src/gba/bios.rs:396-401`（`the_stub_does_not_implement_swi`）与
  `bios.rs:218-223`（`unhandled_vectors_park_instead_of_wandering`）
  都**断言 SWI 向量必须是 `BRANCH_SELF`**。
  ⇒ 修这一条**必然要改这两条锁测试**，且断言必须从「park」改成「返回并记录」，
  否则等于把缺陷重新锁回去（AGENTS.md 纪律 3）。

#### 1.2.2「闪退」＝ vendor 核 panic，而 release 是 `panic = "abort"`

> ⚠️ **更正（2026-11-09，r2）——本小节的「可触达」清单已按源码复核并收窄。**
> **行号全部仍准确**，但清单里 **13 条中有 11 条不可达**：
>
> 1. `bus.rs` 的 11 处 `panic!` 是各 `*_raw` 函数的**兜底分支**，而 `read_raw`(`bus.rs:1077`) /
>    `write_raw`(`bus.rs:1152`) 的 `match` 对各自 I/O 区间**已全覆盖** ⇒ 落不进去。
>    「写一个未建模的 IO 寄存器」今天的行为是 `bus.rs:1203-1211` 的
>    **静默忽略 + debug 日志**，零 panic。
> 2. `internal_memory.rs:660` 同理不可达：`read_at` / `write_at` 的**全部**调用方只有
>    `bus.rs:1088` / `bus.rs:1094` / `bus.rs:1157` 三处，而 `read_at` 的 match 缺口
>    （`0x0400_0000–0x07FF_FFFF` 的 I/O、调色板、VRAM、OAM 区）**恰好由 bus 自己接管**。
> 3. `operations.rs:896` 的 PLD `todo!` 是**死代码**：`arm7tdmi.rs:458-463` 先拦 `Pld` 抛
>    UndefinedInstruction，`psr.rs:74` 的 `NV => false` 再挡一道 ⇒ PLD 的真实后果是
>    「静默 no-op」或「跳 `0x04` 向量自旋」，**都不是 panic**。
> 4. OAM 四处里 **gfx-mode=3 的 panic 现场是 `object_attributes.rs:144` 的 `unreachable!()`**，
>    不是 `262`——`262` 的 `.unwrap()` 只在值 = 4 时才响，而该字段是 2 bit（0..3）⇒ 死分支；
>    `389` / `475` 只向上转发。
>
> ⇒ **真正可达的 panic 面只有两条**：`object_attributes.rs:265`（OAM attr0 **shape = 3**）与
> `object_attributes.rs:144`（OAM attr0 **gfx mode = 3**）。
> ⇒ 另：真机上「禁用精灵」是 **obj mode = 2**（vendor 已正确处理，`layer_obj.rs:234-236` 跳过）；
> **shape = 3 / gfx mode = 3 是「禁止值」（prohibited）**，真机当作「精灵不可见」，vendor 是崩溃。
> ⇒ **S2-a 的修复语义应当是「映射为精灵禁用」**（与 obj mode = 2 的既有处理一致），不是「忽略」。
>
> **逐条证据与复核命令**：
> `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md` §一（含 §九.3 的复核命令）。
> ⇒ **本节的结论方向不变**（闪退确实来自 vendor 核 panic + `panic = "abort"`），
> **变的是证据清单** ⇒ S2-a 的表要按上面对照重读。

- `src/rust/Cargo.toml:9-11`：`[profile.release] panic = "abort"`。
  在 `extern "C"` 帧里 panic **无法 unwind** ⇒ 直接带走整个进程 ⇒ **闪退**。
- panic 点清单（**已按 r2 复核收窄**；行号为原表，判定列是 r2 的复核结论）：
  | 位置 | 触发条件 | r2 判定 |
  |---|---|---|
  | `gba-core/src/bus.rs:596` | `panic!("Not implemented write memory address: {address:x}")` | ❌ 不可达（兜底分支） |
  | `gba-core/src/bus.rs:291/302/343/422/520/538/558/576` | 各 I/O 区域读写越界 panic | ❌ 不可达（兜底分支） |
  | `gba-core/src/bus.rs:957/1069` | LCD 区域读写越界 panic | ❌ 不可达（兜底分支） |
  | `gba-core/src/cpu/hardware/internal_memory.rs:660` | `unimplemented!("Unimplemented memory region. {address:x}")` | ❌ 不可达（bus 只转交三个区间） |
  | `gba-core/src/cpu/arm/operations.rs:896` | `todo!("implement single data transfer operation")`（PLD 指令） | ❌ 死代码（两道防线） |
  | `gba-core/src/cpu/hardware/lcd/object_attributes.rs:265` | OAM attr0 **shape = 3** ⇒ `.unwrap()` on `Err("Prohibited ObjShape")` | ✅ **可达** |
  | `gba-core/src/cpu/hardware/lcd/object_attributes.rs:144` | OAM attr0 **gfx mode = 3** ⇒ `unreachable!()` in `GfxMode::try_from` | ✅ **可达** |
  | `gba-core/src/cpu/hardware/lcd/object_attributes.rs:262/389/475` | 向上转发 | ❌ `262` 是死分支；`389`/`475` 只转发 |
- 关键性质：**这两个 panic 发生在**渲染中途**（OBJ 属性每扫描线解析一次，
  `lcd/layers/layer_obj.rs:691-693`）⇒ 帧步进中途 abort ⇒ C++ 侧看到 `gba_step_frame()`
  永不正常返回 ⇒ **进程崩**。

#### 1.2.3 FireRed 的开机忙等（已定位到具体指令）

用仓库自带探针实测（`probe_why_a_game_stalls` + `probe_trace_a_stuck_game`）：

- PC 稳定停在 ROM `0x080008C2–0x080008CA`（**Thumb**），指令序列是
  `LDRH R1,[R2,#28]` / `ADD R0,R3,#0` / `AND R0,R1` / `CMP R0,#0` / `BEQ loop`。
- 实测寄存器：**`r2 = 0x030030F0`、`r3 = 1`** ⇒ 它在等
  **IWRAM 的 `*(u16*)0x030030FC` 的 bit0** 变成 1，而该位**永远是 0**。
- 30M 指令内 **distinct frame hashes = 1（画面完全冻结）**、每帧步数恒为 163724
  ⇒ 与「无法加载任何画面、音频刺耳杂音」完全吻合（冻结 + CPU 空转出噪声）。
- **尚未确定**：谁应该写这个标志位。⚠️ **高度可疑但未证实**的一条线索：
  `internal_memory.rs:222` 的 `gpio_present` 初始为 `false`，而
  `internal_memory.rs:688-689` 只在**游戏写入** 0xC4–0xC9 时才置 `true`；
  若游戏的 RTC 探测**先读后写**，读到的就是普通 ROM 字节而不是 GPIO 值
  ⇒ RTC 探测失败 ⇒ 标志位永不置位。
  （S3-c 要用夹具证实或证伪这条，**不得当结论用**。）

#### 1.2.4 存档芯片模型不完整

GBATEK 权威（摘录见 §附录 A）：

| ID 串 | 真实芯片 | 本仓库映射 | 判定 |
|---|---|---|---|
| `EEPROM_Vnnn` | EEPROM **512 B 或 8 KB** | `BackupType::Eeprom`，缓冲 32 KB（`internal_memory.rs:97`） | ❌ **无真模型** |
| `SRAM_Vnnn` | SRAM **32 KB** | `BackupType::Sram`，`0x8000` | ✅ |
| `FLASH_Vnnn` | FLASH **64 KB** | `BackupType::Flash64`，`0x10000` | ✅ |
| `FLASH512_Vnnn` | FLASH **64 KB** | `BackupType::Flash64`，`0x10000` | ✅ |
| `FLASH1M_Vnnn` | FLASH **128 KB** | `BackupType::Flash128`，`0x20000` | ✅ |

- **容量映射是对的**（⚠️ 本轮我曾按「1M = 1MB」误判，被 GBATEK 否证 —— 见 §一.5）。
- ⚠️ **r2 更正**：**位级 EEPROM 模型是存在的** —— `internal_memory.rs:523-611` 有
  `is_eeprom_access` / `eeprom_write` / `eeprom_read` / `eeprom_interpret_command`，
  能解释请求位、方向位、6 位与 14 位地址宽、以及按位流长度推断宽度。
  **真正缺的是下面三条**（原表把「无真模型」写得太宽）：
  - **E1 · 访问窗口过宽 16 MB**：`internal_memory.rs:523-532` 的 `is_eeprom_access()`
    对 ≤16 MB 的卡返回 `0x0D000000–0x0DFFFFFF`，而真机 EEPROM 只有 `0x0D00_0000–0x0D00_FFFF`
    （64 KB）。⇒ `bus.rs:1638` / `bus.rs:1674` 会把 **`0x0D01_0000–0x0DFF_FFFF`
    的每一次半字读写劫持进 EEPROM 位流** —— 那是卡的 **ROM（wait state 2）区**
    ⇒ **EEPROM 卡读自己的 ROM 会拿到位流垃圾**（比原表的「窗口宽度错」更严重）。
  - **E2 · 芯片容量不建模**：`internal_memory.rs:92-98` 的 `buffer_size()` 对 `Eeprom`
    返回 `0x8000`（32 KB），真机是 **512 B 或 8 KB** ⇒ `.srm` 尺寸恒为 32768
    （实测 5 个 `.srm` 全是 32768 字节，与此对上）。
  - **E3 · `0x0E` 区域对 Eeprom 一律回 `0xFF`**：`internal_memory.rs:636`
    （读）与 `:724`（写丢弃）—— EEPROM 内容实际存放在别处（`0x0D` 窗口的位流接口）。
  - **Dragon Ball / Minish Cap / NFS 三张卡都是 `EEPROM_V124`（8 KB）** ⇒
    它们读到的存档是全 `0xFF`，写进去的偏移也是错的。
- **Flash 状态寄存器根本没有**：读 `0x0E000006/7` 落到 `internal_memory.rs:639-653`
  的「普通读」分支 ⇒ 返回**存档原始字节**，而不是芯片状态（ready / write-enable / erase-complete）。
  ⇒ 任何轮询 Flash 状态的商用卡都会死等。
- **`.srm` 尺寸 = `buffer_size()`**（`src/rust/src/gba/frame.rs:787`
  `gba_battery_size` → `battery_data().len()`）⇒ EEPROM 卡存出来**永远 32 KB**
  （实测 5 个 `.srm` 全是 32768 字节，正好对上）。

#### 1.2.5 ROM 窗口没有镜像（次要，但确实存在）

`internal_memory.rs:377-404` 的 `read_rom()`：对 `address >= rom.len()` 返回
**开放总线垃圾**（`(address >> 1) & 0xFFFF` 的低/高字节），
**而不是镜像卡带**。真机上 ≤32 MB 的卡在 `0x08000000–0x09FFFFFF` 是镜像的。
⇒ 任何访问超过自己容量的地址（8 MB 卡访问 `0x08800000+`）拿到的是垃圾。

#### 1.2.6 显示侧基本是全的（**不是**黑屏主因）

| 能力 | 状态 | 坐标 |
|---|---|---|
| 视频模式 0–5 | ✅ 全部实现 | `lcd/layers/layer_0.rs`、`lcd/layers/layer_1.rs`、`lcd/layers/layer_2.rs:149/173/212`、`lcd/layers/layer_3.rs:105` |
| OBJ / 仿射 / 旋转缩放 | ✅ | `lcd/layers/layer_obj.rs:140-154/166-201/420-462` |
| WIN0 / WIN1 / WINOUT / WINOBJ | ✅ | `lcd/registers.rs:523-573`、`lcd.rs:543-556` |
| 混合（alpha/brighten/darken）、BLDY 衰减、半透明 OBJ | ✅ | `lcd.rs:400-528`、`layer_obj.rs:487-490` |
| **强制空白渲染成白色**（真机也是白） | ✅ | `lcd.rs:303-307` |

⇒ **Minish Cap 的「黑屏」不是强制空白**（强制空白是白屏），也不是显示模式缺失。
剩余显示侧缺陷只有两条，都在 S3-e：mode 4 的 palette index 0 被误当透明
（`lcd/layers/layer_2.rs:193-196`，真机 mode 4 的 index 0 **不是**透明）、
**mode 5 的页面翻转（DISPCNT bit 4）没实现**（`lcd/layers/layer_2.rs:212-237`，
`render_mode4` 有 `frame_offset` 而 `render_mode5` 完全没有）。

### 1.3 被否证的两个假设（**不要按这两个方向立项**）

1. **「DMA 起始时序 3 没实现」——不是原因。**
   - GBATEK 权威：起始时序在 **bits 12–13**（`gba-core/src/cpu/hardware/dma.rs:22-26`
     的 `timing()` **解码是对的**）；地址控制才是 bits 5–6 / 7–8。
   - **DMA1/DMA2 的时序 3（Sound FIFO）已经实现了**，走**定时器溢出驱动**的
     `run_fifo_dma`（`gba-core/src/bus.rs:700-745`），不是 `run_event_dma(3)`。
   - ⚠️ **仓库自带 `cartprobe` 的判词本身是错的**：
     `src/rust/src/gba/cartprobe.rs:338-345` 的 verdict 只看 `seen_timing[3]`
     （**通道 3**）判断「SPECIAL armed」，却在逐通道清单里把 DMA1/DMA2 的时序 3
     也印成「never fired by this core」。**用它的判词下结论会掉进这个坑。**
     真正没实现的只有 **DMA3 的时序 3（Video Capture）**。
2. **「Flash 芯片容量映射错」——不是原因**（GBATEK 已证，见 §1.2.4 表）。

### 1.4 仍未定位 / 尚未复现的部分（诚实标注）

| 现象 | 状态 |
|---|---|
| Minish Cap 进主线黑屏 | **未复现**。探针窗口 240 帧（4 秒）内画面一直在变（50 个不同帧哈希），故障点没跑到 |
| Dragon Ball 按键后卡死 | **未复现**。探针**不送按键**，窗口内 32 个不同帧哈希 |
| Mario Kart「马上就卡死」 | **未复现**。探针窗口内 128 个不同帧哈希，仍在跑 |
| FireRed 那个标志位的写入方 | **未定** |
| RTC 探测「先读后写」这条线索 | **高度可疑，未证实** |
| 4 张失败卡各自实际调用了哪些 SWI | **未知**（无 SWI 调用日志） |

### 1.5 取证纪律（本轮的教训，必须固化）

**位级格式一律取 GBATEK 权威快照，不凭记忆。** 本轮我**两次**凭记忆写位级格式都写错：

1. 把 DMA 起始时序记成 bits 5–6（实际 **12–13**）；
2. 把 `FLASH1M` 记成 1 MB（实际 **128 KB**）。

两次都是被 GBATEK 否掉的 —— AGENTS.md 纪律 4 说「已犯过两次」，**本轮又犯两次**。

⇒ **行动项**：GBATEK 快照固化成本地取证基线（`.zcode/gbatek_plain.txt`，
经代理 `127.0.0.1:7897` 抓取 `https://mgba-emu.github.io/gbatek/`），
**关键条目的摘录写进本计划 §附录 A**，并给仓库加一条**抓取脚本**，
免得每次重新抓、更免得每次靠记忆。

> ⚠️ **r2 追加**：附录 A 当前**只有三节**（DMA 控制寄存器 / 存档 ID 串 / 卡带头布局），
> **缺 EEPROM 命令协议与 GPIO/RTC 两章** ⇒ S1-a 的夹具 5（EEPROM 协议）与
> 夹具 10（未写入前的 GPIO 行为）**取不到权威依据** ⇒ 这是 S1-a 的**前置条件**
> （见 §二.4 最后一条）。**取不到就停下来记录，不许凭记忆写**（纪律 4，本项目已因此翻车两次）。

---

## 二、S1 阶段 · 可复现门禁（**必须先做**）

### 2.1 为什么必须先做

**仓库现有的探针复现不出 4 个失败中的 3 个**：

| 探针 | 局限 | 后果 |
|---|---|---|
| `cartprobe`（`src/rust/src/gba/cartprobe.rs:348-369`） | 不注入按键、只跑 240 帧、判词逻辑有错（§1.3） | Dragon Ball / Minish Cap / Mario Kart 的故障点跑不到 |
| `probe_why_a_game_stalls`（`frame.rs:2680+`） | 每 1M 指令打一次快照，不归因到具体子系统 | 只知道「PC 在哪」，不知道为什么 |
| `probe_trace_a_stuck_game`（`frame.rs:~2800+`） | 只打印寄存器快照，不看**谁该写那个变量** | FireRed 的标志位写入方查不出 |
| `probe_what_one_frame_changes`（`frame.rs:2929+`） | **要求机器 halt 才开窗** | FireRed 是**忙等**，永不安抚 ⇒ 没有窗口 |

⇒ **不先解决这个，后面每个修复都无法证伪** —— 这正是 AGENTS.md 纪律 3
（变异验证）与纪律 10（门禁没跑 ≠ 通过）的同一个坑。

### 2.2 S1-a 最小可复现夹具（自建合成 ROM，**Rust 内建，不入库二进制**）

业主裁决：卡带不进仓库，判据换成**自建最小夹具**。

> ⚠️ **r2 改写（业主 2026-11-09 拍板 D1）——载体形态与清单都改了。**
>
> **载体**：夹具改为 **Rust 内建**（`src/rust/src/gba/fixtures.rs` 里的函数，现场拼出 ROM 镜像），
> **不再**放 `tests/fixtures/gba/`、**不再**由 `generate_test_roms.py` 生成、**不产生任何二进制入库**。
> `.gitignore` 相应**只加一条 `*.gba`（全局忽略，不加例外）** —— 机械保证商业卡带永远进不了仓库。
> 理由三条：① 纪律 5「测试向量按字段拼装」在二进制 blob 上无法满足；② 门禁链路上**没有 cargo 环节**
> （CI 侧），「测试依赖仓库里一个二进制文件」是**无监控的依赖**；③ ARM 编码器只能有一份实现。
>
> **清单**：删 `gba_io_unmapped.gba`（该缺陷**不可达**，夹具今天就是绿的 ⇒ 无牙门禁）；
> EEPROM 拆成三条独立缺陷 ⇒ **8 条变 9 条**（详见 r2 与
> `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md` §五）。

| # | 夹具 | 触发什么 | 对应缺陷 | 期望断言 | 归属 |
|---|---|---|---|---|---|
| 1 | `swi_unimpl_1a` | 执行 SWI `0x1A`（SoundDriverInit） | §1.2.1 | **不永久自旋**；机器能继续跑下一条指令；读数里报出「SWI 0x1A 未实现」 | S2-b |
| 2 | `swi_unimpl_16` | 执行 SWI `0x16`（Diff8bitUnFilterWram） | §1.2.1 | 同上 | S2-b（兜底）+ S3-a（真实现） |
| 3 | `oam_bad_shape` | 用 `STRH` 把 OAM 精灵 0 的 attr0 写成 **shape = 3** | §1.2.2（`:265`） | **不 panic**；该精灵按「禁用」处理；帧正常步进 | S2-a |
| 4 | `oam_gfx_mode3` | 同上，但 attr0 写成 **gfx mode = 3** | §1.2.2（`:144`） | 同上 | S2-a |
| 5 | `eeprom_8k_protocol` | ROM 含 `EEPROM_V124`；按 **GBATEK 协议**读写 8 KB EEPROM | §1.2.4 E2/E3 | 读回写进去的值 | S3-b |
| 6 | `eeprom_window_width` | ROM 含 `EEPROM_V124`；对 `0x0D01_0000`（真机是 **ROM 区**）做一次半字写 | §1.2.4 **E1** | 该地址**不被劫持进 EEPROM 位流**；读到的是 ROM 内容 | S3-b |
| 7 | `eeprom_srm_size` | ROM 含 `EEPROM_V124`；走一遍存档通路 | §1.2.4 **E2** | `.srm` 尺寸 = 8192（8 KB 芯片） | S3-b |
| 8 | `flash_status_reg` | ROM 含 `FLASH_V124`；先往 `0x0E00_0000` 写 `0x00`，再读 `0x0E00_0006` | §1.2.4 **E3** | 读回的是**芯片状态**（ready 位 bit0 = 1），不是存档原始字节 | S3-b |
| 9 | `rom_window_mirror` | ROM 只有 `0x200` 字节；读 `0x0800_0040` 与 `0x0800_0240` | §1.2.5 | 两个地址读到**同一个绝对金标准值**（不是开放总线垃圾） | S3-d |
| 10 | `gpio_read_before_write` | ROM 含 `EEPROM_V124`；**先读后写** GPIO 0xC4–0xC9 | §1.2.3 线索 | 第一次读就该拿到 **GPIO 状态**（不是 ROM 字节） | S3-c |

**夹具的纪律**：
- 每条夹具**必须能单独触发一条缺陷**，一个夹具不混两件事；
- 夹具的期望值**按字段拼装，不手写字面量**（AGENTS.md 纪律 5）——
  **二进制 blob 无法满足这一条**，这是 r2 改载体形态的首要理由；
- 每条夹具配套**变异验证**：把被断言的行为改回去 ⇒ 该锁测试必须转红；
- ⚠️ **OAM 必须用半字/字写**：`bus.rs:1231-1234` 把 `STRB` 写 OAM **忽略**了，
  用字节写造不出非法 shape/gfx ⇒ 夹具会「测了个寂寞」；
- ⚠️ **断言必须能区分「状态」与「数据」**（AGENTS.md 纪律 13）：
  夹具 8 若直接读 `0x0E00_0006`，今天的返回值 `0xFF` 的 bit0 **恰好也是 1** ⇒ 无牙门禁；
  必须先往存档区写 `0x00` 再读状态位才能区分。

### 2.3 S1-b 探针改造（在**现有**探针上加，不另起炉灶）

| 改造 | 落点 | 目的 |
|---|---|---|
| **修 cartprobe 的 verdict 逻辑** | `cartprobe.rs:338-345` | 现在只看通道 3 却印 DMA1/2 的「never fired」⇒ 改成按通道分别判，别再输出误导判词 |
| **加 SWI 陷阱记录** | `swi/mod.rs` dispatch 入口 | 记录每个 SWI 号码与「是否已实现」⇒ 4 张失败卡各自踩了哪些号码，**直接可读** |
| **加 panic 捕获** | `frame.rs:375` `gba_step_frame` 外层 | `catch_unwind` 包住帧步进，把 panic 转成**可读读数**（哪个地址、什么消息），而不是带走进程 |
| **加输入注入** | `frame.rs` 已有 `gba_set_buttons` | 让探针能在指定帧按指定键 ⇒ 复现 Dragon Ball 的「按键后卡死」 |
| **加长窗口 + 卡死点判定** | `cartprobe.rs` | 窗口从 240 帧拉到可配置；判「卡死」用「PC 连续 N 帧不变」而不是「帧预算耗尽」 |
| **让 `probe_what_one_frame_changes` 能对付忙等** | `frame.rs:2929+` | 现在必须 halt 才开窗 ⇒ 改成「忙等也能开窗」（按指令窗口而非 halt 窗口） |

### 2.4 S1 出口判据（**可证伪**）

> ⚠️ **r2 改写（业主 2026-11-09 拍板 D2）——前两条改了口径与数字。**
>
> **原第一条与「§〇 范围内第 2 条：每条缺陷都要有一条会红的锁测试」互相矛盾**：
> 一条断言「SWI 0x1A 之后机器不永久自旋」的测试，在 S2-b 修复之前**必然是红的**，
> 「会红」与「全绿」不可能同时成立。
>
> ⇒ **裁决：复现器标 `#[ignore]`** —— 断言写的是**目标行为**（不是缺陷行为），
> 修复落地时转成常驻锁测试（当场先红后绿 + 变异验证）；
> **另加 3 条常驻守卫**（孤儿 `#[ignore]` / 夹具完整性 / 复现器存在性）防止 ignore 腐烂。
> ⇒ **不采用「断言写成缺陷当前行为以求全绿」**：那等于把缺陷锁成预期行为，
> 与本计划 §1.2.1 自己对 `bios.rs:396-401` / `bios.rs:218-223` 的批评冲突。

- [ ] 9 条夹具全部就位（`src/rust/src/gba/fixtures.rs`，**Rust 内建、无二进制入库**）；
- [ ] `cargo test -p fceux11-rust --no-default-features --features gba --lib`
      **从 249 项增加到 269 项，全绿**（= 249 + **8** 条编码器端到端自检 + **3** 条守卫）；
      另有 **9 条 `#[ignore]` 复现器**，在 `-- --ignored` 下运行，
      **它们必须逐条为红**（红 = 缺陷在场）；
- [ ] 每条复现器都带 `#[ignore = "S2-x / S3-x: 修复后转常驻锁测试"]` 的**指名归属**；
- [ ] 每条新锁测试的**变异验证双向可红**（改回修复前的值 ⇒ 转红；还原 ⇒ 复绿）；
- [ ] **编码器端到端自检逐条变异可红** —— 编码器是全部 9 条夹具的**共因**，
      它错了 ⇒ 夹具都在测别的东西，而且照样「全绿」（AGENTS.md 纪律 12 的同族，更隐蔽）；
- [ ] `cartprobe` 在 4 张失败卡带本地跑出**明确读数**（哪张卡、卡在哪个 PC、
      踩了哪些未实现 SWI、有没有 panic）；**进仓库的是夹具读数，不是卡带读数**；
- [ ] `cartprobe` 的 verdict 判词修正后**不再出现**误导性表述；
- [ ] **NES 零回归**：`ctest -LE perf` 36 项全绿 + F11QA 矩阵 `pass_to_fail = 0` + grade ≥ B；
- [ ] ⚠️ **前置条件**：GBATEK 快照须补齐 **EEPROM 命令协议** 与 **GPIO/RTC** 两章
      （§一.5 已承诺的动作项；附录 A 当前**只有三节**，缺这两章）
      ⇒ 夹具 5 与夹具 10 的协议级断言**取不到权威依据就不许写**（AGENTS.md 纪律 4，
      本项目已因此翻车两次）。

---

## 三、S2 阶段 · 止血（**先不崩**）

> 目标不是「跑对」，是**把「闪退」变成「画面不对但还在跑」** —— 只有这样，后面的
> 定位才能继续做下去。

### 3.1 S2-a vendor 核的 panic → 开放总线 / 忽略写

把**游戏可触达路径**上的 panic 全部改成「读回开放总线 / 写被忽略 + 记一次日志」：

| 位置 | 改成 |
|---|---|
| `bus.rs:596` | 写未建模 DMA 寄存器 → 忽略 + 计数 |
| `bus.rs:291/302/343/422/520/538/558/576/957/1069` | 读越界 → 回开放总线；写越界 → 忽略 |
| `internal_memory.rs:660` | 读未映射区域 → 回开放总线（`0x0D`/`0x0E` 区域按芯片类型处理） |
| `arm/operations.rs:896` | PLD 指令 → 按 NOP 语义实现（不是 `todo!`） |
| `object_attributes.rs:262/265/389/475` | OAM 非法 shape/gfx-mode → **映射为「精灵禁用」**，不 `.unwrap()` |

**纪律**：**内部不变量**（如 `unreachable!()` 覆盖的 2 bit 字段越界）**保留** ——
只改**外部输入可达**的那些。改完要在注释里写明「这是外部可达路径，改为容错」。

### 3.2 S2-b 未实现 SWI 的兜底

- 把 `bios.rs:134` 的 SWI 向量从 `BRANCH_SELF` 改成「**返回 + 记录 SWI 号码**」。
- ⚠️ **必然要改两条锁测试**：`bios.rs:396-401`、`bios.rs:218-223`
  —— 断言从「park」改成「返回并记录」（§1.2.1 已预警）。
- **兜底语义**：SWI 的寄存器返回值按 mGBA 的做法给「合理默认」，而不是随机值。
- ⚠️ 这是**降级**不是**修复**：真正的修复是 S3-a 补齐实现。兜底只是把
  「永久卡死」变成「功能缺失但游戏能跑」。

### 3.3 S2-c C++ 侧不崩

- `src/gba_load.cpp` 的每帧步进包一层错误处理：Rust 侧返回错误码时
  **不弹窗、不崩**，只记日志并继续（对齐 AGENTS.md 纪律 6）。

### 3.4 S2 出口判据

- [ ] 8 条夹具全部**从「会崩/会卡死」变成「能跑完」**（对应条目逐条打勾）；
- [ ] `gba_step_frame` 在所有夹具下**不返回 panic**；
- [ ] 4 张失败卡带本地跑：**不再闪退**（卡死/黑屏允许存在 —— 那是 S3 的事）；
- [ ] NES 零回归（同上）。

---

## 四、S3 阶段 · 修根因

### S3-a SWI `0x16–0x2A` 补齐

| 优先 | 号码 | 名称 | 备注 |
|---|---|---|---|
| P0 | `0x16/0x17/0x18` | Diff8bitUnFilterWram / Vram / Diff16bitUnFilter | 压缩数据加载常用，商用卡命中率最高 |
| P0 | `0x1A–0x1E` | SoundDriverInit / Mode / Main / VSync / ChannelClear | **官方 SDK 声音驱动**，Nintendo 自家卡几乎必用 |
| P1 | `0x1F` | MidiKey2Freq | |
| P1 | `0x20–0x24` | MusicPlayerOpen / Start / Stop / Continue / FadeOut | 官方音乐播放器 |
| P2 | `0x19` | SoundBias | 依赖 `0x270` 处的声学校准表 |
| P2 | `0x25/0x26/0x27` | MultiBoot / HardReset / CustomHalt | 合卡与复位 |
| P2 | `0x28/0x29/0x2A` | SoundDriverVSyncOff / On / GetJumpList | |

**每一号都要有**：
1. **GBATEK 权威依据**（附录 A 补摘录，**不许凭记忆**）；
2. 一条**锁测试**（期望值按字段拼装）；
3. **变异验证**双向可红；
4. 对 `0x19 SoundBias` 这类依赖**只读常量表**的，登记表从哪来（mGBA 的表**只读不抄**，
   数值须另寻权威来源或自行推导并**标注为推导**）。

### S3-b 存档芯片模型

1. **EEPROM 真模型**：支持 512 B 与 8 KB 两种；地址宽度、位序、命令协议按 GBATEK；
   `internal_memory.rs:523-532` 的 `is_eeprom_access()` 窗口**收窄**到真机范围。
2. **Flash 状态寄存器**：`0x0E000006/7` 按 GBATEK 的状态位定义返回；
   擦除/写入的「忙→就绪」状态机要真的走。
3. **`.srm` 尺寸跟芯片走**：`buffer_size()` 已按芯片给，但 EEPROM 目前是占位 32 KB
   ⇒ 改为 512 B / 8 KB 真实尺寸。
4. **存档类型可覆盖**：`gba_set_save_type` 已有 ⇒ UI 侧接线（属产品决定，另议）。

### S3-c GPIO / RTC 与 FireRed 的标志位

1. **先取证再动手**：用 `gba_gpio_rtc.gba` 夹具 + 探针，**证实或证伪**「先读后写导致
   RTC 探测失败」这条线索（§1.2.3）。
2. 若成立 ⇒ 让 GPIO 在**卡带类型判定为 RTC 卡**时**一开始就可读**，
   而不是「被写过一次才生效」。
3. **FireRed 的标志位写入方**：S1-b 的「忙等也能开窗」改造落地后，用它找出写方。

### S3-d ROM 窗口镜像

`internal_memory.rs:377-404`：`address >= rom.len()` 时**镜像卡带**而不是回开放总线
（≤32 MB 的卡在 `0x08000000–0x09FFFFFF` 是镜像的）。

### S3-e 显示侧小修

1. mode 4 的 palette index 0 **不当作透明**（`lcd/layers/layer_2.rs:193-196`）；
2. **mode 5 的页面翻转**（DISPCNT bit 4）实现（`lcd/layers/layer_2.rs:212-237`）；
3. BG2 文本层只在它有意义的模式下渲染（`lcd.rs:343-349`）。

### S3 出口判据

- [ ] S3-a 每个 SWI 号都有锁测试 + 变异验证；
- [ ] `gba_eeprom8k.gba` / `gba_flash_status.gba` / `gba_rom_mirror.gba` 逐条转绿；
- [ ] **4 张失败卡带本地逐张复核**：
      | 卡带 | 目标 |
      |---|---|
      | FireRed | 出画面 + 存档正确 |
      | Mario Kart SC | 进游戏不卡死 |
      | Dragon Ball | 按键后不卡死 |
      | Minish Cap | 进主线出画面 |
- [ ] **NES 零回归** + GBA 锁测试数 ≥ 257 + vendor / debug / media 三条门禁全绿；
- [ ] vendor 树改动**全部登记进 `ATTRIBUTION.md`**（逐处说明 + 重新 vendor 流程）。

---

## 五、阶段编排与依赖

```
S1 可复现门禁（夹具 + 探针改造）        ← 必须先做，否则后面不可证伪
 ├─ S1-a 最小夹具（8 条）
 ├─ S1-b 探针改造（SWI 陷阱 / panic 捕获 / 输入注入 / 长窗口 / 忙等开窗）
 └─ S1-c 修 cartprobe 的 verdict 判词
        │
        ▼
S2 止血（panic 全改容错 + SWI 兜底）    ← 让机器还能跑，才谈得上定位
 ├─ S2-a vendor 核 panic → 开放总线
 ├─ S2-b SWI 兜底（改 bios.rs:134 + 两条锁测试）
 └─ S2-c C++ 侧不崩
        │
        ▼
S3 修根因（可多路并行，文件域互斥）
 ├─ S3-a SWI 0x16-0x2A      → swi/mod.rs（独占）
 ├─ S3-b 存档芯片模型        → internal_memory.rs（独占）
 ├─ S3-c GPIO/RTC + 取证     → internal_memory.rs + frame.rs（**与 S3-b 同文件，须串行**）
 ├─ S3-d ROM 窗口镜像        → internal_memory.rs（**与 S3-b/S3-c 同文件，须串行**）
 └─ S3-e 显示侧小修          → lcd/（独占）
```

**并行切分按文件域**：`swi/` 一路、`lcd/` 一路、`internal_memory.rs` 一路（内部再串行）。
**探针与夹具（S1）独立成路**，可与 S2/S3 的准备并行。

---

## 六、风险与红线

1. **NES 零回归**（不变式 1）：GBA 改动后必跑 `ctest -LE perf` + F11QA 矩阵。
2. **「把 panic 改成容错」会掩盖真实缺陷**：每个容错点必须**留一个计数/日志**，
   否则「不再闪退」可能只是「缺陷换了个地方」。⇒ S2 出口要求读数里能看到容错计数。
3. **兜底 SWI 会掩盖「未实现」**：S2-b 的兜底必须让未实现号码**可见**（日志 + 计数），
   否则 S3-a 就没有「还差哪些」的清单。
4. **改锁测试的红线**：`bios.rs:396-401` / `bios.rs:218-223` 断言的是**当前行为**；
   改它们时必须在 commit message 里写清「为什么原来断言的行为是缺陷」，
   且**必须先有一个会红的夹具**证明新行为是对的。
5. **vendor 树**：本轮要改 `gba-core` 若干处 ⇒ ATTRIBUTION.md 逐处登记；
   记不清的改动**不做**（宁可不修，不可乱改 vendor）。
6. **外部参考只读不抄**（mGBA = MPL-2.0，与本项目许可不相容）。
7. **GBATEK 取证**：位级格式一律查快照，不凭记忆（本轮已两次翻车）。
8. **GBATEK 快照不进仓库**（5.4 MB）⇒ 只把**关键条目摘录**写进 §附录 A，
   并给一条**可重跑的抓取命令**。

---

## 七、整包出口判据

- [ ] `ctest -LE perf` **36 项全绿**；F11QA 矩阵 **`pass_to_fail = 0`**、**grade ≥ B**
- [ ] GBA 锁测试 **≥ 257 项全绿**（249 + 8 条夹具）；vendor **242** / debug **93** / media **52** 全绿
- [ ] **8 条夹具**逐条绿，且每条的**变异验证双向可红**
- [ ] **4 张失败卡带本地逐张复核通过**（判据见 §四 S3 出口表）；
      **卡带不进仓库**，仓库内判据一律是夹具
- [ ] `cartprobe` 的 verdict 判词**已修正**，不再输出误导性表述
- [ ] vendor 树改动**全部登记进 `ATTRIBUTION.md`**
- [ ] 已知限制**如实写进发版说明**：本轮只解决「能跑」，**不声称 GBA 精度已达标**；
      未实现的 SWI、容错点、未复现的症状逐条列出
- [ ] ⚠️ **仍需人工**：4 张卡带的**最终画面观感**（有没有音乐、动画对不对）——
      机器判据只能证明「不崩、不卡、能出画面」，**观感仍要人看**

---

## 八、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 门禁（NES 零回归）—— 必须等构建结束再跑，不要与构建/测试并行
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf

# GBA 锁测试（含新增夹具）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib

# 夹具生成（新增后）
python tests\fixtures\generate_test_roms.py

# 本地取证：4 张失败卡带（**不进仓库**，只在本地跑）
$env:FCEUX11_GBA_PROBE_DIR = "C:\Users\ikrx2\Desktop\v2.0.3实测依据"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --release --no-default-features --features gba --lib -- --ignored --nocapture cartprobe"

# 单卡定点取证
$env:GBA_PROBE_ROM = "C:\Users\ikrx2\Desktop\v2.0.3实测依据\Pokemon - FireRed Version (USA, Europe) (Rev 1).gba"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --release --no-default-features --features gba --lib -- --ignored --nocapture probe_why_a_game_stalls"

# GBATEK 取证快照（重新抓，需代理）
curl.exe -x http://127.0.0.1:7897 -o .zcode\gbatek_probe.html https://mgba-emu.github.io/gbatek/
```

---

## 九、变更记录

| # | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-08 | **初稿**。业主提供 7 张真实商用手写卡带 + 实测报告（真实性背书）。主会话以 4 个并行只读 agent + 自跑探针完成取证：**① 读出 ODT 报告全文（4 个失败症状 + 截图位置）；② 解析 7 个 ROM 头（全部合法）与存档 ID 串；③ 实跑仓库自带 `cartprobe`（7 卡 × 240 帧窗口）；④ 实跑 `probe_why_a_game_stalls` + `probe_trace_a_stuck_game` 定位 FireRed 卡死指令与寄存器；⑤ 抓 GBATEK 权威快照核对位级格式（**本轮据此否证了自己两条记忆**）；⑥ 代码审计：SWI 覆盖、存档芯片模型、显示管线、panic 面。**结论：三条独立根因（未实现 SWI→`b .` 自旋 / vendor 核 panic + `panic="abort"` ⇒ 闪退 / 存档芯片模型不完整）+ 一条次要缺陷（ROM 窗口无镜像）+ 一条显示侧小修。** 业主拍板范围：**三段式**（S1 可复现门禁 → S2 止血 → S3 修根因）+ **卡带不进仓库、判据换成自建最小夹具**。本条为计划立项，未动任何生产代码 |
| r2 | 2026-11-09 | **S1-a 开工前的只读复核 —— 计划的事实底座有 6 处需要更正，夹具清单与出口判据随之改写。未动任何生产代码。** **① §1.2.2 的「可触达 panic 点」清单收窄**：行号全部仍准确，但 **13 条里 11 条不可达** —— `bus.rs` 的 11 处 `panic!` 是各 `*_raw` 的兜底分支，而 `read_raw`(`bus.rs:1077`)/`write_raw`(`bus.rs:1152`) 的 match 对各自 I/O 区间**已全覆盖**（「写未建模 IO」今天 = `bus.rs:1203-1211` 的**静默忽略 + debug 日志**，零 panic）；`operations.rs:896` 的 PLD `todo!` 是**死代码**（`arm7tdmi.rs:458-463` 先拦 `Pld` 抛 UndefinedInstruction，`psr.rs:74` 的 `NV => false` 再挡一道）；`internal_memory.rs:660` 同样不可达（`read_at`/`write_at` 的**全部**调用方只有 `bus.rs:1088/1094/1157` 三处，缺口区间恰由 bus 自己接管）。**② OAM 的真现场是两条不是一条**：gfx-mode=3 的 panic 现场是 **`object_attributes.rs:144` 的 `unreachable!()`**，不是 `262`（`262` 的 `.unwrap()` 只在值=4 时才响，而该字段是 2 bit ⇒ 死分支）；shape=3 才是 `265`。另更正「真机禁用值」的表述：真机禁用精灵是 **obj mode=2**（vendor 已正确处理），**shape=3 / gfx mode=3 是「禁止值」**，真机当作精灵不可见、vendor 是崩溃 ⇒ **S2-a 的修复语义应是「映射为精灵禁用」**。⇒ **可达 panic 面收窄为 OAM 两条**（`:265` shape=3、`:144` gfx=3）⇒ §1.2.2 加「更正」小节。**结论方向不变**（闪退仍来自 vendor 核 panic + `panic="abort"`），**变的是证据清单** ⇒ S2-a 的表须按对照重读。**③ §1.2.4 的「EEPROM 没有真模型」不准确**：位级模型**存在**（`internal_memory.rs:523-611` 的 `is_eeprom_access`/`eeprom_write`/`eeprom_read`/`eeprom_interpret_command`），真正缺的是三条 —— **E1 访问窗口过宽 16 MB**（`is_eeprom_access` 对 ≤16 MB 的卡返回 `0x0D00_0000–0x0DFF_FFFF`，而真机 EEPROM 只有 64 KB ⇒ `bus.rs:1638/1674` 会把 **`0x0D01_0000–0x0DFF_FFFF` 的半字读写劫持进 EEPROM 位流**，那是卡的 ROM 区 ⇒ **EEPROM 卡读自己的 ROM 会拿到位流垃圾**，比原表的「窗口宽度错」更严重）、**E2 芯片容量不建模**（`.srm` 恒 32768）、**E3 Flash 状态寄存器不存在**。**④ 业主拍板 S1-a 的三个决策**：**D1 载体 = Rust 内建**（`src/rust/src/gba/fixtures.rs` 现场拼 ROM 镜像，**不产生二进制入库**；`.gitignore` **只加一条 `*.gba`，不加例外** —— 例外规则是它的漏洞；理由：纪律 5「按字段拼装」在二进制 blob 上做不到 + 门禁链路上**没有 cargo 环节**，「测试依赖仓库里一个二进制文件」是无监控的依赖 + ARM 编码器只能有一份实现）；**D2 门禁口径 = 复现器标 `#[ignore]` + 3 条常驻守卫**（消解「§〇 说会红」与「§2.4 说全绿」的矛盾；**断言写目标行为，不写缺陷行为** —— 写成缺陷行为就是把缺陷锁成预期行为，与 §1.2.1 自己对 `bios.rs:396-401` 的批评冲突）；**D3 夹具条数 = 9 条**（删不可达的 `gba_io_unmapped.gba`，EEPROM 拆协议/窗口/尺寸三条）⇒ **§2.2 与 §2.4 相应改写**（出口计数 **249 → 269 全绿** + **9 条 `#[ignore]` 复现器**，且复现器在 `--ignored` 下必须逐条为红）。**⑤ 新增前置条件**：GBATEK 快照须补齐 **EEPROM 命令协议** 与 **GPIO/RTC** 两章（附录 A 当前只有三节）⇒ 夹具 5 与夹具 10 的协议级断言**取不到权威依据就不许写**。**⑥ 登记不修的观察项**：vendor 的 `OperandKind` 判别与真 ARM 相反（`cpu/arm/instructions.rs:796`）⇒ `bios.rs:78` 的 `ADD_LR_PC` 在真 ARM 上是 `ADD lr, pc, r0`；vendor 的 `SingleDataTransfer` 立即数偏移**不乘 4**。实施计划见 `docs/plans/FCEUX11-v2.0.3_S1a_最小可复现夹具-实施计划.md`（Step 0..7） |

---

## 附录 A · GBATEK 摘录（位级格式取证基线）

> **快照来源**：`https://mgba-emu.github.io/gbatek/`（经代理 `127.0.0.1:7897` 抓取，2026-11-08）
> **本地副本**：`.zcode/gbatek_probe.html` / `.zcode/gbatek_plain.txt`（gitignored，**不进仓库**）
> **用法**：动手前先查这里的摘录；摘录没有的条目**回快照查**，**不许凭记忆**。

### A.1 GBA DMA 控制寄存器（`DMA CNT_H`，`40000BAh` 等）

```
Bit   Expl.
0-4   Not used
5-6   Dest Addr Control  (0=Increment,1=Decrement,2=Fixed,3=Increment/Reload)
7-8   Source Adr Control (0=Increment,1=Decrement,2=Fixed,3=Prohibited)
9     DMA Repeat                   (0=Off, 1=On) (Must be zero if Bit 11 set)
10    DMA Transfer Type            (0=16bit, 1=32bit)
11    Game Pak DRQ  - DMA3 only -  (0=Normal, 1=DRQ <from> Game Pak, DMA3)
12-13 DMA Start Timing  (0=Immediately, 1=VBlank, 2=HBlank, 3=Special)
      The 'Special' setting (Start Timing=3) depends on the DMA channel:
      DMA0=Prohibited, DMA1/DMA2=Sound FIFO, DMA3=Video Capture
14    IRQ upon end of Word Count   (0=Disable, 1=Enable)
15    DMA Enable                   (0=Off, 1=On)
```

### A.2 卡带存档 ID 串与真实芯片

```
EEPROM_Vnnn    EEPROM 512 bytes or 8 Kbytes (4Kbit or 64Kbit)
SRAM_Vnnn      SRAM 32 Kbytes (256Kbit)
FLASH_Vnnn     FLASH 64 Kbytes (512Kbit) (ID used in older files)
FLASH512_Vnnn  FLASH 64 Kbytes (512Kbit) (ID used in newer files)
FLASH1M_Vnnn   FLASH 128 Kbytes (1Mbit)
```

### A.3 GBA 卡带头布局（本轮实测 7/7 卡全部吻合）

```
0000-0003  Nintendo Logo (前 4 字节)
0004-009F  Nintendo Logo (156 字节)
00A0-00AB  Game Title (12 字节)
00AC-00AF  Game Code (4 字节)
00B0-00B1  Maker Code (2 字节)
00B2       Fixed Value (96h)
00B3       Main Unit Code
00B4       Device Type
00B5-00BB  Reserved (7 字节)
00BC       Software Version
00BD-00BF  Complement Check
00C0-00FF  Start of ARM7/ARM9 Program
```

---

*本计划由独立 code review 产生（2026-11-08）。实施按工作项逐个开独立任务，先 PLAN 后动手。*
