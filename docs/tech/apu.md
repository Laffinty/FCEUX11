# APU（2A03 音频部）— 原理、FCEUX11 设计与经验知识

> **STATUS: ACTIVE**（知识库参考文档；最后核对 2026-09-19）
> 素材来源：NESdev Wiki（CC BY-SA，`nesdev.org/wiki/APU`、`APU_Frame_Counter`、
> `APU_DMC`、`APU_Length_Counter`、`APU_Envelope`、`APU_Sweep`）+ FCEUX11 源码。
> 注意：APU 的**实现**在 `src/sound.cpp`（约 1700 行）；`src/apu.cpp/apu.h` 只是
> `fceu11::Apu g_apu` 状态类 + 旧全局引用别名（`apu.cpp:19-81`）。

---

## 一、硬件原理（Wiki 综合）

### 1.1 五通道总览

| 通道 | 波形 | 音量单元 | 频率单元 | 长度单元 |
|---|---|---|---|---|
| Pulse 1/2 | 4 级占空比（12.5/25/50/75%） | envelope | timer 11bit + sweep | 4bit length |
| Triangle | 32 级步进波 | 无（linear counter 兼职音量） | timer 11bit | 4bit length |
| Noise | 15bit LFSR | envelope | 周期查表（NTSC 16 档） | 4bit length |
| DMC | 1bit Δ调制 / 7bit 直载 | 无（7bit 计数器即音量） | 采样率查表（16 档） | 无 |

- Length counter：32 项查表（`APU_Length_Counter` 的 reload 表），half-frame 时钟递减，
  halt 位（$4000 bit5 / $4008 bit7）挂起递减；$4015 写清 length，读返回通道剩余长度。
- Envelope：divider + decay level，quarter-frame 时钟；loop 模式复用 halt 位。
- Sweep：period divider + negate 模式；**Pulse 2 的 negate 补偿是 +1**（两通道补偿方向
  不同，`CheckFreq` 的负周期回绕 mute 判定对应 Wiki 的「silence」条件）。
- Triangle linear counter：quarter-frame 时钟，reload 标志由 $4008 写与 length>0 共同控制。

### 1.2 帧计数器（Wiki `APU_Frame_Counter`）

- 4 步模式（约 1/60s 序列）：quarter/half 事件在 CPU 周期 **7457 / 14913 / 22371**，
  IRQ flag 在 **29828**，序列重置在 29830；half-frame = length + sweep。
- 5 步模式：7457 / 14913 / 22371 / 37281，重置在 37282，**无 IRQ**；
  $4017 写入 5 步模式成熟时**立即 clock 一次 quarter+half**。
- **$4017 写的成熟延迟**：偶 CPU 周期后 3 周期、奇周期后 4 周期成熟（`fc_reset_in = 3/4`）。
- IRQ flag：读 $4015 清除；写 $4017 置 inhibit（bit6）同时清 flag。
- PAL 事件点为 8313 / 16627 / 24939 / 33252-33254 与 41565/41566。

### 1.3 DMC（Wiki `APU_DMC`）

- $4010：IRQ enable(bit7)、loop(bit6)、rate(bit0-3)；$4011：7bit DAC 直载；$4012/13：
  起始地址（`$C000 + V*64`）与长度（`(V*16)+1` 字节）。
- Sample fetch：每 sample 字节触发一次 CPU **halt 最多 4 周期**（加对齐抖动 1-4），
  期间 CPU 被冻结；fetch 遵循 DMA 优先级（挂起时与 sprite DMA 交错）。
- 输出：7bit 计数器 ±2 步进（delta bit），上溢 126 停、下溢 2 停；变化仅发生在
  每 timer 周期之一。

### 1.4 混音公式（Wiki `APU_Mixer` 的近似式）

- 脉冲通道（两路合成一个查找）：`pulse_out = 95.52 / (8128/pulse + 100)`。
- TND 联合：`tnd_out = 163.67 / (24329/t + 100)`，其中
  `t = triangle/8227 + noise/12241 + dmc/22638`（权重 0.00851/0.01260/0.00443 的整数化）。

---

## 二、FCEUX11 具体设计

### 2.1 寄存器写入路径

`Write_PSG`（`sound.cpp:229-314`，$4000-$400F）→ `Write_DMCRegs`（:316-355，$4010-$4013）
→ `StatusRead/StatusWrite`（:357-410，$4015）→ `Write_IRQFM`（:1211-1280，$4017）。
注册在 `SetNESSoundMap`（:1282-1290）。swapDuty 配置交换 duty 位（:238-239）。

### 2.2 帧计数器（Mesen2/RustyNES 精度模型，v1.16 重写）

- `fhcnt` 语义 = **自序列起点起的 CPU 周期位置**（不是剩余倒数），`fcnt` 是步序号；
  事件表硬编码在 `FrameCounterTick`（`sound.cpp:570-634`）。
- `FCEU_SoundCPUHook`（:685-774）每条 CPU 指令逐周期推进；$4017 写用**单调绝对周期
  计数**判定奇偶（`fc_reset_in = (abs_ts&1)?4:3`，:1268-1269），避免帧边界翻转。
- **IRQ flag 与 IRQ 线分离**：`FrameIRQSet`（:519-539）无条件置 `SIRQStat` bit6，
  仅非 inhibit 才拉 IRQ 线；`FrameIRQEnd`（:541-565）按 inhibit 分支。
- R6-2a 修复：$4017 写**仅当 raw bit6=1** 才清 frame IRQ flag（:1246-1274）。
- 复位语义：上电等效 $4017=$00；soft reset 保留最后写入值；起始相位 `fhcnt=4, fcnt=0`
  （:1406-1428）。

### 2.3 DMC

- `DMCDMA`（`sound.cpp:659-683`）调用 `X6502_DMR` **4 次**，前 3 次丢弃、第 4 次进
  buffer；每次调用 `ADDCYC(1)`（`x6502.cpp:92-100`）→ 每字节 4 周期 stall 的近似
  （未做 halt/dummy-read 细分与 DMA 交错仲裁——见 precision.md）。
- `PrepDPCM`（:172-181）：`DMCAddress = (latch<<6)+0x4000`、`DMCSize = (latch<<4)+1`。
- 位流出在 `FCEU_SoundCPUHook` 内：`delta = ((DMCShift&1)<<2) - 2`，RawDALatch 溢出回退；
  `DMCacc` 负溢出曾致 INT32_MIN UB（hotfix3 C-3 修复，:752-763）。

### 2.4 双质量路径与扩展音频

- HQ：`RDoSQ1/2`、`RDoTriangle`、`RDoNoise`、`RDoPCM` 累积进 `WaveHi`（:776-1209）；
  LQ：`RDoSQLQ`、`RDoTriangleNoisePCMLQ` 合并查 `wlookup2`（:896-1156）。函数指针
  `DoSQ1...` 按配置选择（:1549-1565）。
- 扩展音频（VRC6/VRC7/FDS/N163/5B…）：`EXPSOUND` 函数指针 ABI + 新 C++
  `ExpansionAudio` 基类（`src/expansion_audio.h:21-62`，fill/hi_fill/hi_sync/
  region_changed/kill + VRC7 `neo_fill`），在 `FlushEmulateSound` 中调用（:1317-1347）。

### 2.5 Savestate 兼容（**禁忌**）

chunk 名/大小/序 `FHCN`/`PSG`/`LEN0..3`/`5ACC` 等不可改（`sound.cpp:1303-1307, 1633-1688`），
改了会碎 golden savestate 测试（`tests/kagami/golden_savestate_test.cpp`）；改运行期
起始值同样危险（`DMC_7bit` 兼容 hack，:337-353）。详见 precision.md §3。

---

## 三、经验与魔数（开发者备忘）

| 魔数 | 含义 | 位置 |
|---|---|---|
| `wlookup1[x] = 16^4 · 95.52/(8128/x+100)` | 脉冲混音表（8128 = 脉冲满量程单位） | sound.cpp:1537-1542 |
| `wlookup2[x] = 16^4 · 163.67/(24329/x+100)` | TND 混音表（24329 = 8227+12241+... 归一化前的整数和） | sound.cpp:1543-1548 |
| Triangle `tcout = (15−step 或 step) × 3` | 32 级三角波 15 级×3 的整数近似 | sound.cpp:972-974, 1030-1034 |
| Noise LFSR：tap 短模式 8 / 长 13，bit14 输出 | 15bit 移位寄存器（seed=1） | sound.cpp:1053-1055, 1428 |
| `NoiseFreqTableNTSC/PAL`（16 档） | NTSC {4,8,16,...,254}÷2 表；PAL 值带有不确定性注释 | sound.cpp:104-114 |
| `NTSC_CPU = 1789772.7272727272`、`PAL_CPU = 1662607.125`、Dendy 1773447.467 | 采样率/重采样换算基准 | x6502.h:90-92 |
| `nesincsize = (1<<17)·CPU/(SndRate·16)` | LQ 以 1/16 子采样累积 | sound.cpp:1577 |
| `wlcount=2048`、`nreg=1` | 上电初值（波形周期占位 / LFSR seed） | sound.cpp:1445-1455, 1428 |
| DMC 周期表 NTSC 428→54（16 档） | 每 sample 位周期（CPU 周期/8？按 Wiki rate 表） | sound.cpp:117-133 |
| `fhinc = (PAL?16626:14915)*24` | 遗留常数，仅兼容参考，勿用于新逻辑 | sound.cpp:1532 |
| LQ 曾把 Square1/Square2 音量弄反 | hotfix1 P2-7 历史教训：两脉冲参数顺序 | sound.cpp:932-939 |

## 四、相关文档

- 帧计数器与 R6 调查：[precision.md](precision.md)、`docs/history/surveys/e6_apu/`
- 模数总表：[constants.md](constants.md)
- 扩展音频 mapper（VRC6/VRC7/N163）：[cartridge.md](cartridge.md)
- 测试治理：[KagamiQA.md](KagamiQA.md)
