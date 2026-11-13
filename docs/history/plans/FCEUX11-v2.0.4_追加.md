# FCEUX11 v2.0.4 — 追加项 · 最终清单（技术方向）

> **STATUS: 已归档（2026-11-13，r7）—— 本文已逐条核验并并入
> `docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md`（新增 §0.4 / §1.2 / §二 V9–V25
> / §三 批 4–6 / §五 出口判据 / §六 复验命令 / §七 r7）。**
> **本文内容冻结不再续写**，且**以下坐标与事实已被核验修正**（共 15 处偏差，登记在主计划 §二各节末）：
> 路径写错（`src/AviRecord.cpp`→`src/drivers/Qt/AviRecord.cpp`、`src/guid.cpp`→`src/utils/guid.cpp`）、
> 行号漂移、V12 的 CRC 口径不匹配（**`0xF3C85DC3` 查不中**）、
> V13 的「AVI 可能无人用」不成立、V23 的「F11 站在正确一侧」被上游维护者推翻、
> V16 的可达点不在编译路径上、V20 的上游 POC 已不可复现、V10 的「不走 `FCEU_MemoryRand`」措辞会致 movie 漂移。
> **核验结论**：12 项中 7 项属实 / 5 项部分属实；核验中另新发现 2 项（V24 / V25）⇒ 实收 14 项。
> **以主计划的对应小节为准**（那里是修正后的坐标、方案与验收锚点）。

> ~~**STATUS: 技术方向稿**（2026-11-13 静态分析产出，**未实测**。回开发环境验证后，
> 本文按 §一体例**追加进** `docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md`
> （建议落位：§一 任务总表续行、§二 逐项解决方案续节、§七 变更记录补一条）。）~~
> **来源**：上游 `fceux-fceux-2.7.0` + `fceux-master` 对标静态分析（多线程 / 核心安全 / Mapper 三方向）
> + 上游 issue 集同源排查。
> **本文件的性质**：只定「目标 / 现状坐标 / 做法要点 / 风险 / 验收锚点」，不写逐步构建步骤。
> **筛选结论（2026-11-13 终审，按「用户可感 / 解锁 / 防呆 / 成本」逐项判必要性）**：
> 18 项候选收敛为 **12 项**——必做 7 + 条件 1 + 合并批 4；砍掉 6 项（不再留档）。
> 2.1 版本线随之一并取消（唯一留存项已并入本清单 V23）。
> **编号**：V9–V23 续接主计划 V1–V8，V17/V18/V21 为空号（已裁）。

### 硬约束（沿用主计划）

一切优化**不改变 FCEUX11 现有架构**：registry 注册表（`registry.h:38-73`）+ MapperStrategyA/Mmc3BaseCart
组合式 Cart（`mapper_strategy_a.h:26-52`）+ `fceu11::Bus` 类化（`bus.h:115-167`）+ Rust 解析层 +
`fceuCriticalSection` 锁体系（`fceuWrapper.h:58-83`）。NES 侧零回归不变。

---

## 〇、判定口径与前提

### 0.1 三条影响全文的事实

1. **上游 2.7.0 快照 = 漏洞基线，不是修复版**。
   `fceux-fceux-2.7.0/src/version.h:63-65` 自陈 `2.7.0`；六项安全修复全部发生在 2026-10-18～11-10
   的 master 上，**不在快照内**。⇒ 引用上游修复一律给**修复 commit**。
   **其中 V9/V10/V16/V19/V20 已用 `fceux-master`（2026-11-13 快照，含 2.7.0 发版后全部修复）
   实码交叉验证吻合**（见 §七 r4）。
2. **FCEUX11 的 mapper 矫正活表在 Rust 侧**：`src/rust/crates/fceux11-formats/src/ines/ines_data.rs`
   `ROM_CORRECTIONS`（约 326 条）；`src/ines-correct.h`（278 条）**已无任何 include，是死表**。
   ⇒ 加 CRC 矫正条目必须落 Rust，改 `ines-correct.h` 不生效。
3. **决策依据是「用户可感 / 解锁 / 防呆 / 成本」**，不是「红不红」——本批项在 `tests/tests.json`
   判红体系里收益为 0。⇒ `fail_to_fail_max = 0` 仍成立：**任何一项做坏都是红灯**。

### 0.2 证据纪律（沿用）

- 每条结论带 file:line；坐标以 `FCEUX11-wip204` 与上游两棵树为准。
- 本文件为**静态分析**结论，未实跑、未编译。标 ⚠️ 的项动手前必须先复现 + 建回归基线。
- 上游修复 commit 号转正前在开发环境 `git show <hash>` 复核。

### 0.3 与 V1–V8 的关系

零重叠：V1–V8 全是精度/调试/工具面，本清单全是内存安全/线程安全/mapper 正确性面（V23 为调试器护栏）。
唯一的间接耦合：本批做完后 V2 的 21 行勾选表需复验（尤其 LUA / savestate / input 三行）。

---

## 一、最终清单（V9–V23，12 项 / 3 批）

| # | 项 | 面 | 一句话现状 | 危害/收益 | 级 | 批 |
|---|---|---|---|---|---|---|
| **V9** | mapper 218 VPageR 越界 | Mapper·安全 | 与上游修复前逐字相同，缺 `*0x400 - i*0x400` | 段错误 | **P0** | 1 |
| **V10** | CHR-RAM <8K 堆溢出 + 掩码钳制回归 | 安全·Mapper | 仅 1K 垫层；`Bus::setup_chr_mapping` 丢了掩码 -1 钳制（F11 独有回归） | 堆溢出 | **P0** | 1 |
| **V11** | VRC7 读档 OPLL 悬空指针 | 安全·存档 | `StateRestore` 未调 `OPLL_forceRefresh` | 读档崩溃 | **P1** | 1 |
| **V12** | 3GO.nes 矫正条目（Mapper 0→252） | Mapper | CRC `F3C85DC3` 不在 Rust 矫正表 | 游戏不可玩 | **P1** | 1 |
| **V15** | JSreturn 原子转移 | 多线程 | 裸 `uint32`，GUI 写 / emu 读 | 输入撕裂 | **P1** | 1 |
| **V22** | Lua 鼠标键位掩码判断 | Lua | 等值判断消费位掩码，双键同按丢失；中键永不输出 | 脚本读值错 | **P2** | 1 |
| **V13** | AVI 环形缓冲四索引原子化 | 多线程 | 裸 `int` 跨 emu/磁盘线程 | 录像撕裂/竞态 | **P1** | 1⚠️ |
| **V14** | volatile 跨线程共享量原子化（TraceLogger + CDL 六计数器） | 多线程 | 仍 volatile，含非原子 RMW | 开发工具丢样 | **P2** | 2 |
| **V16** | StringToBytes hex 越读 | 安全 | `str.size()/2` 把 `0X` 前缀算进字节数 | 越读（config 可达） | **P2** | 2 |
| **V19** | GUID 失败置零 + 删死代码 | 安全 | Rust 解析安全但失败留栈垃圾 | 未初始化读 | **P2** | 2 |
| **V20** | movie length 纵深 | 安全 | 主路径已守卫，入库/truncateAt 无负值护栏 | abort 纵深 | **P2** | 2 |
| **V23** | CDL 行为差异文档化 | 调试器 | F11 与上游两处语义不同，未文档化 | 防后人误修 | **P2** | 3 |

**批次定义**：
- **批 1 · 必做（止血 + 正确性）**：V9/V10/V11/V12/V15/V22，外加 V13（⚠️ 动手前先确认 AVI 录制在用，
  确认无人用则降为注记不做）。
- **批 2 · 防御性修正（一次做完，打包提交）**：V14/V16/V19/V20——四项全是一行级防御性修正，
  合计约半天工作量，不值得各自立项。
- **批 3 · 护栏**：V23（纯文档 + 用例固化，同主计划 V3 体例）。

---

## 二、逐项技术方向

### 批 1 · 必做

#### V9 · mapper 218 VPageR 越界（P0）

- **目标**：消除 mapper 218 加载即段错误。
- **现状坐标**：`src/boards/datalatch.cpp:518`
  `VPageR[i] = &NTARAM[mapping[info->mirrorAs2Bits*8+i]];` —— 与上游修复前**逐字相同**。
- **根因**：`mapping[]` 的值是 0/1（2KB NTARAM 的页号），旧写法当**字节偏移**用；PPU 经 `VPageR`
  取图样时 1K 页步距错误，越过 0x800 的 NTARAM（上游 issue #851 栈：`GetCHRAddress`）。
- **做法要点**（上游修复 commit `6738471`，2026-10-18，单行；已与 `fceux-master` 实码吻合）：
  `VPageR[i] = &NTARAM[mapping[info->mirrorAs2Bits*8+i] * 0x400] - i * 0x400;`
- **风险**：极低（只影响 218 初始化一处）。
- **验收锚点**：#851 复现 ROM 不崩；`ctest -LE perf` 全绿。

#### V10 · CHR-RAM <8K 堆溢出 + 掩码钳制回归（P0）

- **目标**：消除 1K/2K CHR-RAM 卡 `$2007` 写越堆；并修复 F11 独有的掩码钳制回归。
- **现状坐标**（三处，缺一不可）：
  1. `src/ines_init.cpp:51-55`：`mCHRRAMSize=(CHRRAMSize<1024)?1024:CHRRAMSize` —— **仅 1K 垫层**，
     PR #859 明言「The 1k padding … isn't enough」（`unif_load.cpp:86` 同样无下限）；
  2. `src/cart.cpp:212-224` `setchr8r`：8 槽共指线性 8K 窗口，`CHRmask8` 对 <8K 钳 0 ⇒ 恒 V=0，
     1K RAM 的 `$0400-$1FFF` 写越堆；
  3. `src/bus.cpp:559-585` `Bus::setup_chr_mapping`：**丢失上游 `cart.cpp:135-138` 的
     `if (CHRmask* >= (unsigned)(-1)) = 0` 钳制** ⇒ size<1K 时掩码下溢 `0xFFFFFFFF`（F11 独有回归）。
- **做法要点**（上游 PR #859 = commit `8510fb8`，2026-11-10；已与 `fceux-master` 实码吻合）：
  - 分配 `max(CHRRAMSize, 8192)`，**垫层 `memset 0`，不走 `FCEU_MemoryRand`**
    （⚠️ **关键细节**：走 MemoryRand 会改变随机 RAM 初始化的 RNG 消耗序列，**破坏既有 movie 同步**）；
  - 映射与 `AddExState` 仍用真实 `CHRRAMSize`（存档格式不变）；
  - `Bus::setup_chr_mapping` 恢复四行掩码钳制（纯函数体内部，不涉架构）。
  - ⚠️ **确认点 1**：NES2.0 的 CHR-RAM 尺寸由 Rust `cart.rs:137`
    （`battery_vram_size + vram_size`，测试 `cart.rs:459-466` 期望 12288，**无 8K 最小钳**）计算。
    分配侧垫到 8K 后理论可覆盖，但**须确认该计算结果确实流经 `ines_init.cpp:51` 分配点**；
    若 UNIF / NES2.0 存在独立分配路径（`unif_load.cpp:86` 为一例），同样垫。
  - **确认点 2（零尺寸 CHR）**：`CHRRAMSize == 0` 的情形（mapper 256 OneBus，无 CHR-ROM 也无 CHR-RAM，
    master `ines.cpp` else 分支置 `VROM = NULL`）。**FCEUX11 已具备同款分支**
    （`ines_init.cpp:59-62` `else { VROM = NULL; }`）——本项实施时**保留不动**即可。
- **风险**：⚠️ **中**——触及 RAM 初始化路径（RNG 序列）。≥8K 卡零影响；<8K 卡从「越堆」变「行为正确」。
- **验收锚点**：`nes-test-roms other/oam3.nes`（mapper 7、1K CHR-RAM）ASan 下 3000 帧无越界；
  **既有 movie 回放零漂移**；`ctest -LE perf` 全绿。

#### V11 · VRC7 读档 OPLL 悬空指针（P1）

- **目标**：消除读档后 OPLL 内部裸指针悬空导致的崩溃/杂音（上游 #821；亦是 #798「随机崩溃」
  最实在的可疑向量）。
- **现状坐标**：`src/boards/vrc7.cpp:51` 将整个 OPLL 结构以 `FCEUSTATE_INDIRECT` 落盘
  （`state.cpp:178` 按字节写出）；结构含裸指针 `OPLL_SLOT.sintbl`（`emu2413.h:47`，`slot[6*2]` 见 :104），
  读回后为跨进程悬空指针，`emu2413.c:778` 解引用。
  `StateRestore`（`vrc7.cpp:198-199`）仅调 `Sync()`（:102-113，纯 bank 映射），**未调**
  `OPLL_forceRefresh`（`emu2413.c:613-627` 会 UPDATE_WF 重建 sintbl）——上游建议的最小修复也缺失。
- **做法要点**：`StateRestore` 加一次 `OPLL_forceRefresh(VRC7Sound)` —— 仅重建派生量，
  **不动 ABI、不动存档格式**。⚠️ 反向验证（2026-11-13）：`fceux-master` 的 `vrc7.cpp` 与 2.7.0 快照
  **逐字节相同**，上游至今未修此项，本仓修完即领先。
- **风险**：低（单点重建调用）。
- **验收锚点**：存档读回不崩、音色连续；跨机器读档过 f11qa savestate 回归；
  VRC7 游戏（Lagrange Point 等）读档回放一致。

#### V12 · 3GO.nes 矫正条目（P1）

- **目标**：修复《三国志中原之霸者》(3GO.nes) 图形错乱（上游 #845/#844）。
- **根因**：iNES 头误标 Mapper 0，实际 Mapper 252；CRC32 `F3C85DC3` 不在矫正表。
  板级名匹配 `"SAN GUO ZHI PIRATE"`（`ines_bmap.h:282`）对 iNES 无从命中（该 ROM 内无此串）。
- **现状坐标**：活表 `src/rust/crates/fceux11-formats/src/ines/ines_data.rs` `ROM_CORRECTIONS`
  （约 326 条），全仓无 `F3C85DC3`；查询/应用链 `ines.rs:309` → `src/ines_load.cpp:279-286`
  （v2.0.3 S5-A2 已使 mapper 矫正生效，**补条目即可工作**）；`src/boards/252.cpp` 实现完好。
  ⚠️ `src/ines-correct.h` 是死表，**别改**。
- **做法要点**：`ROM_CORRECTIONS` 加 `{crc32: 0xF3C85DC3, mapper: 252, mirror: -1}`。
- **风险**：极低（纯查表）。
- **验收锚点**：3GO.nes 加载日志出现 mapper 矫正、画面正常。

#### V15 · JSreturn 原子转移（P1）

- **目标**：GUI 写手柄状态、emu 读手柄状态的跨线程访问原子化（输入是 TAS 立身之本）。
- **现状坐标**：`src/drivers/Qt/input.cpp:151`（裸 `uint32 JSreturn`），写 1584，读 216、1805。
- **做法要点**：`std::atomic<uint32_t>` + emu 侧每帧快照（快照点放 `fceuWrapperUpdate` 帧首，
  **不新增线程、不改输入管线架构**）；`GetGamepadPressedImmediate` 改 relaxed load，语义不变。
- **风险**：低。
- **验收锚点**：狂按按键压测 + 录 movie 回放一致。

#### V22 · Lua 鼠标键位掩码判断（P2）

- **目标**：修复 Lua `input.get` 鼠标键读值错误。
- **现状坐标**：`src/rust/crates/fceux11-lua/src/bindings/input.rs:133-140` 用**等值判断**
  （`click == 1` / `== 2`）消费 `GetMouseData` 的**位掩码**（`src/drivers/Qt/input.cpp:1233-1257`：
  0x01 左 / 0x02 右）⇒ 双键同按（值 3）两个分支都不命中、静默丢失。
  另 `middleclick` 在 `input.rs:12` 声明但**永不输出**（驱动侧无中键 0x04 位）。
- **做法要点**：改位测试（`& 1` / `& 2` / `& 4`）；`GetMouseData` 补中键 0x04 输出。
- **风险**：低（Lua 绑定层，不动输入管线）。⚠️ 唯一行为变化：双键同按的脚本从「读不到」变「读到 3」——
  向正确方向变化，需在提交说明里写明。
- **验收锚点**：单/双/中键脚本读值正确；现有 Lua 测试不回归。

#### V13 · AVI 环形缓冲四索引原子化（P1）⚠️ 先确认使用度

- **目标**：消除 emu 线程与 AVI 磁盘线程之间的索引撕裂。
- **⚠️ 前置动作（决定做不做）**：先确认 FCEUX11 的 AVI 录制功能**实际在用**（有调用路径 + 有人用）。
  若确认无人用 → 降为「已知潜伏项」注记，**不做**；若在用 → 按下述实施。
- **现状坐标**：`src/AviRecord.cpp:63-67`（四个裸 `int`）、`src/AviVideoCodec.h:49-52`（extern 声明）、
  `src/AviRecordDiskThread.cpp:113,167-226`（消费者）。
- **做法要点**：四个 `int`→`std::atomic<int>`，写端 store release / 读端 load acquire
  （**照抄 `nes_shm.h:160-188` 已验证的注释策略**，不新造写法）；extern 声明同步改类型。
- **风险**：低（纯类型替换，ABI 内部）。
- **验收锚点**：录 AVI ≥10 分钟压测无花屏/丢帧；静态审查全树无对这四个符号的裸访问。

### 批 2 · 防御性修正（一次做完，打包提交）

> 四项共性：**一行级防御性修正**、不改行为语义、成本约半天。单独看每一项都不「必要」，
> 打包看是「输入验证与跨线程整洁性」的一次性收口。

#### V14 · volatile 跨线程共享量统一原子化（TraceLogger 环形缓冲 + CDL 六计数器）（P2）

- **目标**：让跨线程共享的索引/计数器有内存序保证。两类同性质缺口合并处理。
- **现状坐标**：
  1. **TraceLogger 环形缓冲**：`src/drivers/Qt/TraceLogger.cpp:117-118`（volatile），用点 1003-1022、2206-2217；
  2. **CDL 六计数器**：`debug.h:85` + `debug.cpp:469`
     （`codecount, datacount, undefinedcount`）、`ppu.h:149` + `ppu.cpp:204`
     （`rendercount, vromreadcount, undefinedvromcount`）、`sound.h:43`/`sound.cpp:159` 同名 extern。
     emu 侧自增（`CodeDataLogger.cpp:889` 等），GUI 侧读（`CodeDataLogger.cpp:368-395,581,842`）。
     ⚠️ `debug.cpp:481-513` 还是**非原子读-改-写**（`int32_t cc = codecount; … codecount = cc;`）。
- **做法要点**：
  - TraceLogger：改 `std::atomic<unsigned int>`；顺带修正 114 行「volatile」过时注释。
  - CDL 六计数器：改 `std::atomic<int>`；`core_state.h:174-183` 六个 `volatile int&` 访问器同步改返回类型
    （改后更新该头注释）；`debug.cpp:481-513` 的 RMW 改 `fetch_add`/`fetch_sub` 或 load/store 成对。
    写端 release / 读端 acquire（照 `nes_shm.h:160-188` 既有策略）。
- **风险**：低-中（类型替换；⚠️ CDL 计数器触及 `core_state.h` 访问器签名，需全量编译回归）。
- **验收锚点**：开 trace logger 长跑 + 后台写盘无断流；CDL 开启下 GUI 计数显示与实际一致；
  `trace_common.cpp:455-459` 的新指令判定不漏报。

#### V16 · StringToBytes hex 越读（P2）

- **目标**：消除 hex 解码对 `0X` 前缀的字节数误算（同上游 #857）。
- **现状坐标**：`src/utils/xstring.cpp:367` `int bytesAvailable = str.size()/2;`
  —— 偶数长度值末轮读过 NUL。可达点：`src/drivers/common/config.cpp:306`（配置值加载）。
  （`movie_fm2.cpp:270/296/306` 同款在 `installValue` 内，但该函数**已无调用方**，死代码。）
- **做法要点**：改 `(str.size()-2)/2`（上游 commit `683c3f0` 同款，已与 master 实码吻合），
  可加奇数长度拒收；顺手删除或加护栏给死代码 `installValue` 路径（防复活）。
- **风险**：低（越读非越写，实害小）。
- **验收锚点**：上游 #857 POC（`romChecksum 0XABAB…` 偶长）ASan 干净；config 加载回归。

#### V19 · GUID 失败置零 + 删死代码（P2）

- **现状坐标**：`src/rust/crates/fceux11-utils/src/guid.rs:108-163`（越界已消：`clean.len()!=32` 即拒）。
  **残留**：失败时不置零，`valuearray.h:24` 无构造零化，`guid.cpp:22-27` `fromString` 失败后
  `data` 为栈垃圾；C++ 死代码 `hexToByte`（`guid.cpp:29-40`）仍在（scan 已不调用）。
- **做法要点**：解析失败时 `guid.data = [0;16]`（对齐上游 master `guid.cpp` `scan()` 失败
  `memset(data,0,…)` 的做法）；删除死代码。
- **风险**：低（错误路径未初始化读，不崩溃）。
- **验收锚点**：`guid 0` 头得全零 GUID；往返测试仍绿。

#### V20 · movie length 纵深（P2）

- **现状坐标**：Rust 主路径守卫齐备（`movie.rs:389-391`、`:492/:513`、`movie_playback.cpp:144` 均 `>=0`）。
  残留：`movie.rs:315` `length` 解析照收负值入库；`movie_fm2.cpp:243-246` `truncateAt` 的
  `resize(frame)` 无负值护栏。
- **做法要点**：`length` 负值拒收（`if v>=0`）；`truncateAt` 加 `if (frame<0) frame=0;`。
  （master `movie.cpp` 三处守卫同式，已吻合。）
- **风险**：低（主路径已安全，纯纵深）。
- **验收锚点**：上游 #858 POC（`length -2` + `binary 1`）不 abort。

### 批 3 · 护栏

#### V23 · CDL 行为差异文档化（P2）【上游 #771｜原 2.1 线 E3，按终审并入本清单】

- **目标**：澄清 FCEUX11 与上游 2.7.0 的两处 CDL 语义差异，防止后人误当 bug 修（护栏类，同主计划 V3 体例）。
- **现状坐标**：`src/rust/crates/fceux11-debug/src/debug.rs:255-273` 对读操作标记 data 时
  **不排除 4C/6C**（即保留了上游 #771 作者想要的「4C 目标 data 标记」，而上游 2.7.0
  `fceux-fceux-2.7.0/src/debug.cpp:557` 已排除）；写路径**无条件清零**（debug.rs:275-292），
  上游仅 FDS 清零（上游 debug.cpp:567-583）——F11 更激进，可能误清 ROM CDL。
- **做法要点**：① 保留 4C 标记（#771 所欲，F11 站在正确一侧，**不改**）；② 写清零行为二选一：
  限定 FDS（对齐上游）或保持现行为并**在文档显式声明差异**；③ CDL 文件对比用例固化。
- **风险**：低（文档 + 可选行为对齐）。
- **验收锚点**：差异声明进文档；CDL 对比用例绿。

---

## 三、风险汇总与回归纪律

> `fail_to_fail_max = 0` ⇒ 本批任何一项做坏都是红灯。⚠️ 项动手前必须先建复现/回归基线。

| # | 风险级 | 触及面 | 必做的回归 |
|---|---|---|---|
| V10 | ⚠️ 中 | RAM 初始化路径（RNG 序列） | **既有 movie 零漂移**（垫层清零正是为此）+ ASan |
| V14 | 低-中 | `core_state.h` 访问器签名 | 全量编译 + CDL 计数一致性 |
| V9/V11/V12/V19/V20/V22/V23 | 低 | 单点防御/查表/文档 | 对应 POC + ctest |
| V13/V15 | 低 | 跨线程索引 | 录 AVI / 输入压测 |
| V16 | 低 | hex 解码 | #857 POC + config 回归 |

**通用出口判据（沿用主计划 §五）**：
1. `ctest -LE perf` 全绿（零回归，含 fail_to_fail=0）；
2. 四门禁计数不降；
3. 本清单每项验收锚点逐条落章；
4. V2 手工验收 21 行中 LUA / savestate / input 三行复验。

**复验命令**：⚠️ 待回开发环境后按实际构建目录补（沿用主计划 §六 体例）。

---

## 四、执行顺序建议

| 序 | 内容 | 说明 |
|---|---|---|
| **第 1 步** | V13 使用度确认（5 分钟） | 决定 V13 做或不做 |
| **第 2 步** | V9 + V10 | 两个 P0，先消灭崩溃与溢出（V10 单独提交，带 movie 零漂移回归） |
| **第 3 步** | V11 + V12 + V15 + V22 | P1/P2 正确性与输入面 |
| **第 4 步** | V14 + V16 + V19 + V20 | 防御性修正批，打包提交 |
| **第 5 步** | V23 | 护栏文档，可随时插队 |

每步一个提交簇，单项独立可回退。

---

## 五、证据索引（关键 file:line）

| 事实 | 坐标 |
|---|---|
| 上游快照版本 = 2.7.0（漏洞基线） | `fceux-fceux-2.7.0/src/version.h:63-65` |
| mapper 218 脆弱写法 | `src/boards/datalatch.cpp:518`（上游修复 commit `6738471`，master 实码吻合） |
| CHR-RAM 仅 1K 垫层 | `src/ines_init.cpp:51-55`（上游修复 `8510fb8` / PR #859，master 实码吻合；`unif_load.cpp:86` 同） |
| 零尺寸 CHR 分支（已具备，保留不动） | `src/ines_init.cpp:59-62` |
| F11 独有掩码钳制回归 | `src/bus.cpp:559-585`（上游参照 `cart.cpp:135-138`） |
| VRC7 OPLL 指针落盘 + 未 forceRefresh | `src/boards/vrc7.cpp:51,198-199`（`emu2413.c:613-627,778`；master 该文件未修） |
| 3GO CRC 缺失 | `src/rust/crates/fceux11-formats/src/ines/ines_data.rs` ROM_CORRECTIONS |
| hex 越读 | `src/utils/xstring.cpp:367`（可达 `config.cpp:306`；master 修复吻合） |
| GUID 失败未置零 | `src/rust/crates/fceux11-utils/src/guid.rs:108-163`（master `guid.cpp` 失败置零做法参照） |
| movie 纵深残留 | `src/rust/crates/fceux11-formats/src/movie.rs:315`、`movie_fm2.cpp:243-246` |
| AVI 四裸索引 | `src/AviRecord.cpp:63-67`（extern `AviVideoCodec.h:49-52`） |
| Trace volatile | `src/drivers/Qt/TraceLogger.cpp:117-118` |
| CDL 六计数器 volatile + 非原子 RMW | `src/debug.h:85`、`debug.cpp:469,481-513`、`ppu.h:149`、`ppu.cpp:204`、`sound.h:43`、`core_state.h:174-183`（读点 `CodeDataLogger.cpp:368-395`） |
| JSreturn 裸 uint32 | `src/drivers/Qt/input.cpp:151` |
| Lua click 等值判断消费位掩码 | `src/rust/crates/fceux11-lua/src/bindings/input.rs:133-140`（位掩码源 `input.cpp:1233-1257`） |
| CDL 两处语义差异 | `src/rust/crates/fceux11-debug/src/debug.rs:255-292` vs 上游 `debug.cpp:557,567-583` |
| F11 原子化已有面（模板写法） | `src/drivers/common/nes_shm.h:160-188`、`src/cpu.h:160-168` |

---

## 六、保持不动（已安全 / 已领先 / 无需动作）

1. **sprintf**：已干净，无需改动，**门禁也无需另建**——编译期天然拦截（`CMakeLists.txt:71` `/sdl`
   将 C4996 升为错误 + `src/CMakeLists.txt:81` 已移除 `_CRT_SECURE_NO_WARNINGS`；全树 622 处
   `snprintf`，活跃树无裸 `sprintf`）。
2. **movie 负长度主路径**：FCEUX11 已走 Rust（`movie_fm2.cpp:407-497` → `fceux11_rust_movie_load_fm2`），
   比上游 C++ 路径更安全，无需照搬上游 C++ 修复（V20 只补纵深残留）。
3. **MMC3 IRQ/A12 核心**：两边逐行一致（上游 `mmc3.cpp:207-220` / F11 `mmc3.cpp:249-274`）。无动作。
4. **MMC1 submapper 5 Fixed-PRG**：F11 领先（`mmc1.cpp:422-457`），无可吸收项。

---

## 附录 · 变更记录

| 版 | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-13 | 初稿：自上游 2.7.0 对标分析分流出「漏洞修复与 Mapper 速赢」批技术方向。未实跑。 |
| r2 | 2026-11-13 | 复核修订：补入遗漏项（V14 扩到 CDL 六计数器及其非原子 RMW；V10 补 Rust `cart.rs:137` 确认点）；自 2.1 批上移 V22（Lua 鼠标键位掩码）。 |
| r3 | 2026-11-13 | 按业主指令（不再跟进的项一律不保留在技术方向文档中）删除全部裁撤项条目。 |
| r4 | 2026-11-13 | 以 `fceux-master`（2026-11-13 快照）交叉验证：V9/V10/V16/V19/V20 修复做法与上游实码逐字吻合；反向验证 `vrc7.cpp`/`mmc1.cpp` 未修（V11 领先）。V10 补零尺寸 CHR 确认点。 |
| r5 | 2026-11-13 | **终审收敛（按必要性逐项判定）**：必要性不足的 6 项裁撤、不保留（V17/V18/V21 留空号以维持与主计划 V1–V8 的编号连续）；V16/V19/V20/V14 归为「防御性修正批」一次做完；原 2.1 线唯一留存项并入本清单为 V23，**2.1 版本线取消**。现最终清单 12 项（必做 7 + 条件 1 + 合并 4）。 |

> **下一步（回开发环境后）**：① V13 使用度确认；② 复现 V9/V10 两个 P0 并落最小修复；
> ③ 为 V10 建回归基线（movie 零漂移 + ASan）；
> ④ 按主计划 §二 体例把选定项写成构建方案，连同本表一并**追加进**
> `docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md`。
