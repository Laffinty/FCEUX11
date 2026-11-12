# FCEUX11 v2.0.3 — S3-a「SWI 0x16–0x2A 补齐」前置 RE 证据表

> **STATUS：RE 基线（2026-11-11，只读取证，随 r12 记入主计划 §九）**
> **用途**：S3-a 开工时的纪律 4 依据——每条论断带 GBATEK 快照行号；「GBATEK 未指定」的项
> **必须停下记录**，不许凭记忆实现。
> **资料源**：GBATEK 快照 `D:\Project\FCEUX11\.zcode\gbatek_plain.txt`（下称 `:行号`，
> 2026-11-08 抓取；重抓用 `tools/fetch_gbatek.ps1`，行号不变）。
> ⚠️ **mGBA 0.10.5 源码不在本机**（`Research_only/` 目录已不存在）——下表 mGBA 列全部
> n/a，**mGBA 恢复可用前，任何「按 mGBA 做法」的口径都不可执行**。
> 本仓现状：`dispatch_inner`（`src/rust/src/gba/swi/mod.rs:69-135`）认领
> 0x01–0x05/0x09/0x0A/0x0D–0x15；**0x16–0x2A 全部落 decline**（r12 起返回而非自旋）。
> 名称以 GBATEK 汇总表（`:192200-192271`）为准。

---

## 〇、两个影响口径的硬事实

1. **SOUNDBIAS = `0x04000088`**（`:1605`、`:4570`「4000088h - SOUNDBIAS - Sound PWM Control
   (R/W)」，默认 0200h `:4572-4573`）。**`0x04000208` 是 IME**（`:1745`）——早前任务简报里
   的「0x04000208 SoundBias」是错的。vendor 已把 `0x0400_0088` 建模为
   `sound.sound_pwm_control`（`gba-core/src/bus.rs:772/836`）。
2. **SWI 异常返回的 LR 约定**（独立复核 r5 ⑥(b)，r12 已照此落地）：
   - 「For Returning from SWI use "MOVS PC,R14" … PC=R14_svc, and CPSR=SPSR_svc」`:185672-185674`；
   - ARM 态 `R14_svc=PC+4` `:185686`；Thumb 态 `R14_svc=PC+2` `:189222`；
   - 号码提取位置：ARM `[R14_svc-4]` `:185676-185678`、Thumb `[R14_svc-2]` `:189203-189205`
     —— 证明 LR 停在 SWI 后一条指令；
   - 「MOVS … will also restore THUMB mode」`:189203-189210`；异常 LR 与来源态无关 `:184894-184897`。
   - 本仓对照：vendor HLE 侧 LR=`next_ins`（`arm7tdmi.rs:712-714`）与公式一致；stub BIOS
     0x08 向量在 r12 已改为 `MOVS PC, LR`（`0xE1B0F00E`）。

## 一、逐号证据表（0x16–0x2A，共 15 个）

> 列「真实现要触到」指在 fceux11 里做**真实现**（区别于 r12 的兜底）需要的落点；
> `MachinePort` / `BusLike` / `serve_decompress` 均为 `src/rust/src/gba/swi/decompress.rs`
> 与 `swi/mod.rs` 的现成基建。**mGBA 列全部 n/a（源码不在）。**

### SWI 0x16 — Diff8bitUnFilterWrite8bit（"Wram"）
- GBATEK：标题 `:192804`；「These aren't actually real decompression functions, destination
  data will have exactly the same size as source data.」`:192816-192818`；参数
  「r0 Source address (must be aligned by 4)…Bit 0-3 Data size (must be 1 for Diff8bit,
  2 for Diff16bit) / Bit 4-7 Type (must be 8 for DiffFiltered) / Bit 8-31 24bit size after
  decompression…r1 Destination address」`:192848-192862`；「Return: No return value, Data
  written to destination address.」`:192867`。
- 输入/输出：r0=源（4 对齐），r1=目的；无寄存器返回；目的写入字节数 = 头 24bit size，
  单元 8bit 差分累加。
- 真实现：`parse_lz77_header`（decompress.rs:72-78 同款 32bit 头）+ `MachinePort`
  halfword/byte 写模式（:265-330）+ `BusLike`（:346）；新增 type==8 / size==1 校验与差分
  累加循环；「拒绝不写」语义沿 `serve_decompress` 先例（mod.rs:222-238）。风险低。

### SWI 0x17 — Diff8bitUnFilterWrite16bit（"Vram"）
- GBATEK：标题 `:192808`；参数/返回同 0x16（`:192848-192867` 共享一节）；「Data units may
  be either 8bit or 16bit used with Diff8bit or Diff16bit functions respectively.」`:192840-192841`。
- 输入/输出：同 0x16；差异只在**目的写宽度**（Vram 必须半字写——VRAM 字节写的复制/丢弃
  副作用，vendor `bus.rs:1141-1186`，仓库 decompress.rs:20-26 已记录）。
- ⚠️ GBATEK **未写明** Vram 变体的写宽度；锁测试断言依据是仓库已建立的事实
  （decompress.rs:20-26）。风险低。

### SWI 0x18 — Diff16bitUnFilter
- GBATEK：标题 `:192812`；头「Bit 0-3 Data size (must be …2 for Diff16bit)」`:192850`；
  汇总表单名 `:192253`；返回 `:192867`。
- 输入/输出：同 0x16，单元 16bit（半字差分累加）。
- 小设计点：`MachinePort` 的 halfword 模式是「字节配对」（:305-324），Diff16 可直接半字写
  ——二选一，锁测试按所选宽度断言。风险低。

### SWI 0x19 — SoundBias
- GBATEK：标题 `:194287`；「Increments or decrements the current level of the SOUNDBIAS
  register (with short delays) until reaching the desired new level. The upper bits of the
  register are kept unchanged.」`:194291-194293`；「r0 BIAS level (0=Level 000h, any other
  value=Level 200h)」`:194300`；「r1 Delay Count (NDS/DSi only) (GBA uses a fixed delay
  count of 8)」`:194301`；「Return: No return value.」`:194305`。
- 输入/输出：r0=目标档（仅两档语义），r1 GBA 忽略；写 `0x04000088` 低 10 位（高位保持）。
- 真实现：总线写 `sound_pwm_control` 即可。**⚠️ 计划所称「`0x270` 声学校准表」在快照中
  零依据**（SoundBias 节 + 全文 grep `0270`/calibration 声音上下文零命中）——渐变步长/延时
  表无来源；最小可辩护实现 = 直接写目标档 + 注释声明「渐变未建模」。

### SWI 0x1A — SoundDriverInit
- GBATEK：标题 `:194330`；「Initializes the sound driver. Call this only once when the game
  starts up.」`:194334`；「It is essential that the work area already be secured…」`:194338-193339`；
  「You cannot execute this driver multiple times…」`:194343-194344`；r0=SoundArea 指针 +
  结构定义 `:194351-194430`（`vchn[MAX]`「currently 8 channels」`:194362-194364`、
  `pcmbuf[PCM_BF*2]` `:194367`、`fr`「Write the value obtained with the MidiKey2Freq
  function here.」`:194399-194400`、WaveData.freq 公式「sampling rate x
  2^((180-original MIDI key)/12)」`:194413-194414`）。
- 输入/输出：r0=工作区指针；输出 = BIOS 向 SoundArea 写初始化数据（**具体字节效果未指定**，
  §三-④）；无寄存器返回。
- 真实现：写游戏 RAM（经 `BusLike`）；**无硬件寄存器耦合**（官方驱动的音频 = 驱动混音进
  pcmbuf → 驱动自设 DMA1/2 喂 FIFO；vendor 的 FIFO DMA 已在硬件侧服务，
  `bus.rs:689-709`）。**出口口径建议钉为「认领 + 可辩护 SoundArea 初态 + 不自旋」**；
  真实音频产出是独立缺口。风险中。

### SWI 0x1B — SoundDriverMode
- GBATEK：标题 `:194454`；r0 位定义 `:194465-194475`（0-6 reverb、7 使能、8-11 同时发声数
  1-12 默认 8、12-15 主音量 1-15 默认 15、16-19 播放频率 12 档
  「5734,7884,10512,13379,15768,18157,21024,26758,31536,36314,40137,42048, def 4=13379 Hz」、
  20-23 DAC 位数）；「Return: No return value.」`:194478`。
- 输出落点未指定（§三-④⑤）。风险低（stub 级即可不卡机）。

### SWI 0x1C — SoundDriverMain
- GBATEK：标题 `:194432`；「Main of the sound driver.」`:194436`；「Call every 1/60 of a
  second. The flow of the process is to call SoundDriverVSync…immediately after the V-Blank
  interrupt. After that, this routine is called after BG and OBJ processing is
  executed.」`:194440-194445`；「No parameters, no return value.」`:194447`。
- 真实现：无（最简立即返回）；真混音 = 整个软合成器，超出 SWI 补齐范围。
- ⚠️ 无参调用如何定位 SoundArea 未指定（§三-⑤）。风险低。

### SWI 0x1D — SoundDriverVSync
- GBATEK：标题 `:194486`；「An extremely short system call that resets the sound DMA. The
  timing is extremely critical, so call this function immediately after the V-Blank
  interrupt every 1/60 second.」`:194490-194493`；「No parameters, no return value.」`:194495`。
- 「reset sound DMA」的寄存器级效果未指定（§三-⑥）。真实现：无（vendor FIFO 重填是硬件
  自动的，`bus.rs:689-709`）。风险低。

### SWI 0x1E — SoundChannelClear
- GBATEK：标题 `:194313`；「Clears all direct sound channels and stops the sound.」`:194317`；
  「This function may not operate properly when the library which expands the sound driver
  feature is combined afterwards. In this case, do not use it.」`:194319-194321`；
  「No parameters, no return value.」`:194323`。
- 真实现（做全）需记住 0x1A 存下的 SoundArea 指针（HLE 自持状态；savestate 序列化面参照
  `PENDING_INTR_WAIT` 先例 mod.rs:466-469）。最简 stub 返回则零风险。风险中。

### SWI 0x1F — MidiKey2Freq
- GBATEK：标题 `:194239`；「Calculates the value of the assignment to
  ((SoundArea)sa).vchn[x].fr when playing the wave data, wa, with the interval (MIDI KEY)
  mk and the fine adjustment value (halftones=256) fp.」`:194243-194245`；
  「r0 WaveData* wa / r1 u8 mk / r2 u8 fp」`:194252-194254`；「Return: r0 u32」`:194263`。
  另载 BIOS 读保护破法（`:194276-194280`「a = (MidiKey2Freq(i-(((i AND 3)+1)OR 3), 168, 0)
  * 2) SHR 24」）——本号在无驱动游戏里也常被调用的原因。
- 真实现：纯计算 + 读 `wa->freq`（WaveData+8 偏移，`:194410-194414` 可拼）。
- ⚠️ **GBATEK 未给换算公式**（§三-①）——只有 WaveData.freq 公式 `:194413-194414` 可作推导
  锚；实现值必须**标注为推导**（预期形如 `wa.freq * 2^((mk + fp/256 − 180)/12)` 的定点版）。

### SWI 0x20–0x24 — SoundWhatever0..4（计划名 MusicPlayerOpen/Start/Stop/Continue/FadeOut）
- GBATEK：标题「SWI 20h..24h (GBA) - SoundWhatever0..4 (Undocumented)」`:194537`；全文仅
  「Whatever undocumented sound-related BIOS functions.」`:194541`。汇总表 `:192261-192265`。
- **零记载**（无参数、无返回、无语义）——没有任何权威依据支持超出「认领 + 立即返回 +
  记录」的实现；计划用的 MusicPlayer* 名来自官方 SDK 头文件（不在 GBATEK）。
- **证据缺口最大的一组**（§二 ❌ 行）。锁测试只能断言「返回且机器继续跑」。

### SWI 0x25 — MultiBoot
- GBATEK：标题 `:193873`；「This function uploads & starts program code to slave GBAs…」
  `:193877-193881`；「…covers only 45% of the required Transmission Protocol, the other 55%
  must be coded in the master cartridge」`:193884-193886`；「r0 Pointer to MultiBootParam
  structure / r1 Transfer Mode (0=256KHz 32bit Normal, 1=115KHz 16bit MultiPlay, 2=2MHz
  32bit Normal)」`:193893-193899`；「Return: r0 0=okay, 1=failed」`:193914`；参数结构须先置
  字段 `:193921-193935`。
- 单机环境没有从机 ⇒ 唯一诚实出口是 **r0=1（failed）**（`:193914` 行号背书）；vendor SIO
  只存不传（`bus.rs:306-360`）。风险低。

### SWI 0x26 — HardReset
- GBATEK：标题 `:193545`；「This function reboots the GBA (including for getting through
  the time-consuming nintendo intro, which is making the function particularly useless and
  annoying).」`:193549-193551`；「Parameters: None. Return: Never/Reboot.」`:193553-193554`；
  「Execution Time: About 2 seconds (!)」`:193559`。
- **结构性障碍**：`dispatch` 只拿 `&mut Arm7tdmi`（mod.rs:59），整机所有权在 frame.rs 的
  机器锁里 ⇒ SWI 处理器内无法重建整机。可行形状：vendor 已有 halt 同类机制上加「reset
  请求」由帧循环消费，或 `swi_return` 到 `0x00000000` 做软级重启——**需设计裁决**，且
  reboot 步骤未指定（§三-⑧）。风险中。

### SWI 0x27 — CustomHalt
- GBATEK：标题 `:193414`；「Writes the 8bit parameter value to HALTCNT, below values are
  equivalent to Halt and Stop/Sleep functions, other values reserved, purpose
  unknown.」`:193418-193420`；「r2 8bit parameter (GBA: 00h=Halt, 80h=Stop)」`:193426`；
  「No return value.」`:193428`。HALTCNT=`0x04000301` W（`:1748`、`:7726`）。
- 真实现：复用 `enter_low_power`（mod.rs:442-459）——**参数从 r2 取**（不是 r0）；
  HALTCNT 寄存器 vendor 只存不执行（`bus.rs:250-251`），halt 须由 HLE 主动置 `cpu.halted`
  （与 0x02/0x03 同路）。reserved 值按 `IntrWait`「不可满足即返回 + trace」先例。风险低。

### SWI 0x28 — SoundDriverVSyncOff
- GBATEK：标题 `:194500`；「…if the V-Blank interrupts are stopped, and SoundDriverVSync
  cannot be called every 1/60 a second, this function must be used to stop sound DMA.」
  `:194502-194506`；「Otherwise, even if you exceed the limit of the buffer the DMA will
  not stop and noise will result.」`:194508-194509`；「No parameters, no return value.」`:194511`。
- 真实现：无（最简返回）。**不碰 vendor 的 FIFO DMA**——GBATEK 说的 stop 是**驱动**停混音，
  不是 DMA 控制器寄存器写。效果落点未指定（§三-⑥）。风险低。

### SWI 0x29 — SoundDriverVSyncOn
- GBATEK：标题 `:194519`；「This function restarts the sound DMA stopped with the previously
  described SoundDriverVSyncOff.」`:194523-194524`；「After calling this function, have a
  V-Blank occur within 2/60 of a second and call SoundDriverVSync.」`:194526-194527`；
  「No parameters, no return value.」`:194529`。镜像 0x28。风险低。

### SWI 0x2A — SoundGetJumpList
- GBATEK：标题 `:194545`；「Receives pointers to 36 additional sound-related BIOS
  functions.」`:194549`；「r0 Destination address (must be aligned by 4) (120h bytes
  buffer)」`:194556`。
- 输出 = 向缓冲写 36 个函数指针；**内容表零记载**（120h=288 > 36×4=144，差额未解释）。
- **风险高（内容不可知）**：写伪 ROM 指针 → 游戏一调即跳垃圾；写 0 同样危险；stub BIOS
  也没有这些函数。锁测试只能断言「写了 144 字节的某物 + 返回」，且必须注释声明内容为占位。

## 二、计划 §四 S3-a P0/P1/P2 分级 × 证据交叉核对

| 分级 | 号码 | 证据结论 |
|---|---|---|
| P0 | 0x16/0x17/0x18 | ✅ 完全支持：头格式/参数/返回齐备（`:192848-192867`），仓库基建现成，夹具已红 |
| P0 | 0x1A–0x1E | ⚠️ 支持但不完整：五号都有语义描述，「不自旋」目的成立；但实现深度受限（§三-④⑤⑥）。**建议出口钉为「认领 + 可辩护 RAM 初态 + 不自旋」，音频产出单列** |
| P1 | 0x1F | ⚠️ 支持优先级，公式缺口（§三-①），实现须标注推导 |
| P1 | 0x20–0x24 | ❌ 证据不支持「实现」：GBATEK 仅一句（`:194541`）；只能做 stub，MusicPlayer* 名称无权威来源 |
| P2 | 0x19 | ✅ 支持（寄存器已建模、两档语义清晰）；「0x270 校准表」零 GBATEK 依据，mGBA 缺席后来源问题加剧 |
| P2 | 0x25/0x26/0x27 | ✅ 支持；0x25 有「r0 0=okay,1=failed」背书（`:193914`）；0x27 可复用 halt 机制；**0x26 有结构性障碍** |
| P2 | 0x28/0x29/0x2A | ⚠️ 支持：0x28/0x29 stub 级安全；**0x2A 内容不可知**，锁测试口径要显式声明占位 |

没有「证据反证该分级」的条目；两个「证据不支持该分级所暗示的实现深度」（0x1A–0x1E、
0x20–0x24）与一个「实现有架构外障碍」（0x26）——已随 r12 记入主计划变更记录。

## 三、「GBATEK 未指定」清单（S3-a 开工时必须停下记录的项，纪律 4）

1. **0x1F MidiKey2Freq 换算公式**（检索 `:194239-194290` 全节无）——只有 WaveData.freq
   公式 `:194413-194414` 可推导，须标注推导。
2. **0x20–0x24 全部语义**（`:194537-194541`；汇总 `:192261-192265`）——仅一句
   "Whatever undocumented"。
3. **0x19 的渐变步进表（计划所称 0x270 校准表）**（SoundBias 节 `:194287-194305` + 全文
   grep `0270`/calibration 声音上下文零命中）。
4. **0x1A 对 SoundArea 的实际初始化写入**（ident 值、vchn 初值、DmaCount/func/intp/NoUse
   处置；`:194330-194430` 只给结构定义，不给 BIOS 写什么）。
5. **无参声音 SWI（0x1C/0x1D/0x1E/0x28/0x29）如何定位 SoundArea**（`:194313-194535`）。
6. **0x1D/0x28/0x29「resets/stops/restarts the sound DMA」的寄存器级效果**
   （`:194490-194493` / `:194502-194511` / `:194523-194529` 各一句话）。
7. **0x2A 的 36 个函数指针内容**（`:194549`/`:194556` 只给数量与缓冲尺寸）。
8. **0x26 HardReset 的 reboot 步骤**（`:193549-193559` 只给语义与耗时）。

## 四、顺带发现（同文件同性质，S3-a 顺手可修/须记账）

- **SWI 0x01 RegisterRamReset 有 GBATEK 记载的强制空白副作用**：「The function always
  switches the screen into forced blank by setting DISPCNT=0080h (regardless of incoming
  R0, screen becomes white)」`:193530-193531`（归属 0x01——在 `:193501` RegisterRamReset
  标题之后、`:193545` HardReset 标题之前）；「Bug: LSBs of SIODATA32 are always
  destroyed…」`:193522`。本仓 `serve_register_ram_reset`（mod.rs:404-431）未做 DISPCNT
  强制空白。
- 命名三处差异（r5 ⑥(c) 已记）：GBATEK `Diff8bitUnFilterWrite8bit`（`:192804`）/ 仓库
  `Diff8BitUnFilterWram`（mod.rs:717）/ 计划 `Diff8bitUnFilterWram`——号码相同。
- SWI 通用 ABI：出参寄存器「Outgoing registers R0,R1,R3 are typically containing either
  garbage, or return value(s). All other registers (R2,R4-R14) are kept unchanged.」
  `:192058-192062`——锁测试断言「只写合同内寄存器」时可引用（`serve_bios_checksum`
  注释 mod.rs:386-391 已采用同口径）。
- ARM 态调用编码「specify SWI NN*10000h」`:192068-192070`；汇总表「2Bh+ … SWI xxh..FFh do
  jump to garbage addresses」`:192270`——给「越界号 = crash」的 stub 行为提供真机语义锚。

---

*本表由只读 RE 产生（2026-11-11，r12 同轮）；mGBA 源码恢复可用后，§一的 mGBA 列应补齐。*

---

## 五、r16/r17 更正与处置（2026-11-12，第二批落地后回填）

1. **`0x1F` 的 freq 位置更正：在 WaveData+4，不是 +8**。本表原记 +8 有误；
   mGBA `bios.c` 的 `load32(wa+4)` 与 GBATEK 公式段佐证。第二批实现与锁均按 +4 落地。
2. **mGBA 参照版本补齐**：v0.10.5 tag（URL 形如
   `https://raw.githubusercontent.com/mgba-emu/mgba/0.10.5/src/gba/bios.c`，tag 无 `v` 前缀）。
3. **`0x19` 的「依赖 0x270 声学校准表」维持判伪**（GBATEK `:194291-194301` 语义完备、
   vendor 已建模 SOUNDBIAS `0x04000088`）⇒ 已按 GBATEK 语义实现（低 10 位替换、高位保持）。
4. **12 个声音号（`0x1A`–`0x1E`/`0x20`–`0x24`/`0x28`/`0x29`）处置**：mGBA HLE（无真 BIOS）
   对它们也全是 no-op stub（`bios.c:607-608` default 分支），任何模拟器都没有官方软合成器
   的 HLE ⇒ 本仓**维持 S2-b 兜底 + 已知限制**，不做「认领 + 文档化 no-op」（会掩盖红线 3
   的「未实现必须可见」）。
5. **`0x2A` 处置**：36 项函数表内容 GBATEK 零记载（`:194549`/`:194556` 只给数量与缓冲
   尺寸）、mGBA 显式放弃（转抛真 BIOS）⇒ **BLOCKED**，维持 S2-b 兜底。
6. **`0x26` HardReset**：结构障碍确认维持（dispatch 只拿 `&mut Arm7tdmi`，
   `arm7tdmi.rs:1134-1142`）+ 零观测需求 ⇒ 挂起。
7. **`0x01` RegisterRamReset 的 DISPCNT 强制空白已补**（r17；本表 §四记的行号
   `:193530-193531` 实为空行，实际原文在 `:193540-193541`，实现按更正后行号落地）。
8. **7 卡 240 帧基线对 `0x19`–`0x2A` 全部零踩踏**——本批的先验优先级无实测反证；
   `note_unimplemented_swi` 的计数是「真实需求」的唯一证据源，后续按它驱动第三批
   （若某号被真实游戏踩到，再按纪律 4 逐号取证实现）。
