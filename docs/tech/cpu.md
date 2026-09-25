# CPU（RP2A03 / 6502 核）— 原理、FCEUX11 设计与经验知识

> **STATUS: ACTIVE**（知识库参考文档；最后核对 2026-09-19）
> 素材来源：NESdev Wiki（CC BY-SA，`nesdev.org/wiki/CPU`、`CPU_interrupts`、
> `CPU_unofficial_opcodes`、`CPU_power_up_state`、`Cycle_reference_chart`）+ FCEUX11 源码。
> CPU 仿真 100% 在 C++（`src/x6502.cpp`、`src/cpu.h`、`src/ops.inc`）；Rust 侧只有
> 反汇编器（`fceux11-debug/src/asm.rs`）与 QA 驱动，没有执行核。

---

## 一、硬件原理（Wiki 综合）

### 1.1 总体参数

- 核心为 6502 派生（Ricoh RP2A03），**无十进制模式**（BCD 被芯片裁掉）。
- NTSC 1.789773 MHz（主时钟 21.47727 MHz ÷ 12）、PAL 1.662607 MHz（÷16）、
  Dendy 1.773448 MHz。NTSC 每帧约 29780.5 CPU 周期（平均 89342/3）。
- PPU:CPU = 3:1（NTSC/PAL/Dendy 一致）；APU 按帧序列低频驱动，与 CPU 同频计数。
- 寄存器：A/X/Y 8bit、PC 16bit、S 8bit（上电 0xFD）、P（NV-BDIZC）。
- 上电/复位状态见 Wiki `CPU_power_up_state`：A=X=Y=0、S=FD、P=$34（I=1）等；
  reset 后 7 周期 + 稳定延迟。

### 1.2 周期模型（Wiki `Cycle_reference_chart`、`CPU_cycles`）

- 6502 每条指令由多个总线周期构成：取 opcode → 取操作数 → 读/写 → （RMW 双写）。
- **Dummy read**：abs,X/(zp),Y 跨页时先读一次错误地址（CPU 先用基址算出未修正地址
  发起读）；写指令也总是先对未修正地址做一次假读。
- **Dummy write**：RMW 指令读后先写回**旧值**再写新值（对 mapper 寄存器可观测）。
- **中断轮询**发生在指令的固定周期点（通常倒数第二个周期），因此有 branch/中断
  hijack、BRK 双重压栈等子指令级行为（Wiki `CPU_interrupts`）：
  - NMI 边沿检测在 φ2 采样，延迟到下一指令边界可见；
  - 恰在取中断向量前一条 BRK 会被「劫持」；
  - taken branch 会把中断的响应推迟到分支目标之后（branch-delay）。
- 未官方指令：全 256 opcode 中 45 个非官方 opcode 在实机有确定行为（AAC/ARR/ASR/
  AXS/LAX/DCP/ISB/SLO/SRE/RLA/RRA/SAX/SHA…，含不稳定的 XAA/SHA），KIL/JAM 使
  CPU 完全停摆直到复位。

---

## 二、FCEUX11 具体设计

### 2.1 指令实现：宏 + 生成表（勿手改）

- 语义源文件 `src/ops.inc`（switch-case 体）→ `scripts/generate_x6502_dispatch.py`
  机械生成 `src/ops_table.inc`（每 opcode 一个 `x6502_op_XX` + 256 项函数指针表，
  文件头声明 Do not edit manually），include 进 `x6502.cpp:358`。
- 寻址模式全是宏：`GetAB/GetZP/GetIX/GetABIRD(读,带跨页 dummy read)/GetABIWR(写/RMW,
  总是 dummy read)/GetIYRD/GetIYWR` + 组合包装 `LD_*/ST_*/RMW_*`（x6502.cpp:226-356）。
- N/Z 用 256 字节查表 `ZNTable`。

### 2.2 时钟驱动：PPU 驱动 CPU

- 唯一入口 `X6502_Run(n)`（`x6502.h:58-59` → `x6502.cpp:498-607`），**n 的单位是
  PPU dot**：入口 `_count += n * (PAL?15:16)`，每 CPU 周期扣 48 单位
  （`cpu.h:104-109` `add_cycles`）→ NTSC 3 dot = 1 cycle，PAL 3.2 dot。
- 驱动方是 PPU 循环：旧 PPU 每 scanline 分段 `X6502_Run(256)/(85)/(16)`；
  新 PPU 每 dot `runppu(1)`；ppudead 期整帧一次跑完。
- `add_cycles` 同步：`tcount`（本指令累积，供 mapper IRQ hook）、`timestamp_`（绝对
  32 位时间戳，绕回由 `timestampbase` 兜底，`fceu.cpp:217`）、`sound_timestamp_`
  （**超频时不前进**——overclock 让 PPU 多跑行而 APU 时基不变）。

### 2.3 周期账目与 dummy read/write

- 基础周期 `CycTable[256]`（x6502.cpp:360-378）+ 分支 taken +1、跨页再 +1（`JR`
  宏 :128-143，页翻转判定 `(tmp^_PC)&0x100`）。
- 跨页 dummy read：`GetABIRD/GetIYRD` 补读 `target^0x100`；写指令恒 dummy read
  未修正地址；RMW 全系双写（读→写旧→写新）。
- 注意：blargg `cpu_dummy_writes_oam/ppu` 仍 FAIL——差距在 **$4014 OAM DMA 与 PPU
  区路径**，不在 RMW 宏。

### 2.4 中断模型（关键近似）

- 中断**只在 while 循环顶部（指令边界）检查**（x6502.cpp:515-579）：无指令内轮询、
  无 interrupt hijack、无 branch-delay——`cpu_int_2/3/4/5` 全部 FAIL 的直接原因。
- 优先级链：RESET（取 FFFC/FFFD）→ `FCEU_IQNMI2`（延迟一档的 NMI）→ NMI（7 周期，
  push `(_P&~B)|U`，取 FFFA/FFFB）→ IRQ（仅当上一条指令结束时的 P（`mooPI`）无
  I 标志）。
- NMI 一边界延迟由 `g_e1_nmi_fresh` 实现（latch 在边界 B 及之后置位 → 下一条边界
  可见；依据 e1_vbl 调查 04-nmi_control #11，x6502.cpp:416-447）。
- 每条指令前 `_PI = _P` 实现 I 标志延迟一拍；指令后以 `tcount` 调 mapper
  `MapIRQHook` 与 `FCEU_SoundCPUHook`。
- `X6502_IRQBegin/End(w)`（:385-393）= `_IRQlow` 置/清位；`FCEU_IQTEMP` 0x800 是
  调试器一次性 IRQ，主循环每轮清除。

### 2.5 非官方指令与 BCD

- 非官方指令覆盖完整（`ops.inc:324-492`）：含 KIL/JAM（`ADDCYC(0xFF)` + `_jammed=1`
  + PC 回退；`_jammed` 同时阻止 NMI/IRQ dispatch）、非官方 SBC 0xEB、
  ATX/OAL 的 `A|0xFF` 常数（blargg 实测 NES 上为 $FF）。
- 自认可疑点：**XAA 0x8B 用 `A|0xEE` 魔数**（「BIG QUESTION MARK」注释，
  ops.inc:490-491）；文件头自认该块「may be wrong」。
- **BCD 完全未实现**：ADC/SBC 无 D_FLAG 分支（x6502.cpp:156-172），SED/CLD 只动
  标志——NES 硬件本无 BCD，此为正确取舍。

### 2.6 Open bus

- 数据总线锁存 `_DB`（`x6502struct.h:27`），未映射地址读返回 `g_cpu.db()`
  （`bus.cpp:55-57`）；PPU 侧另有 600ms 衰减锁存，见 [ppu.md](ppu.md) §2.6。

---

## 三、经验与魔数（开发者备忘）

| 魔数 | 含义 | 位置 |
|---|---|---|
| `1789772.727272…` / `1662607.125` / `1773447.467` | NTSC/PAL/Dendy CPU 频率 | x6502.h:90-92 |
| `60.0988138974405…` / `50.0069779682683…` | NTSC/PAL 帧率（帧长 89342/35468 dots 的倒数） | MoviePlay.cpp:333、fceuWrapper.cpp:851 |
| `PAL?15:16` 与 `*48` | 1 PPU dot = 15/16 内部单位；1 CPU 周期 = 48 单位 | x6502.cpp:505、cpu.h:106 |
| `CycTable[256]` | 每 opcode 基础周期（调试器经 `X6502_GetOpcodeCycles` 读） | x6502.cpp:360-378 |
| `S=0xFD` | 上电栈指针 | x6502.cpp:492 |
| `ADDCYC(0xFF)` | KIL 的「吞周期」近似 | ops.inc:402 |
| `256+85` / `12` | 旧 PPU 行分段（可见+消隐）、VBL 后 12-dot NMI 延迟 | ppu_rendering.cpp:1211-1227 |
| `kLineTime=341` / `kFetchTime` | 新 PPU 主循环常数 | ppu_rendering.cpp:1362-1363 |
| `A|0xEE` | XAA 不稳定输入常数（自认 BIG QUESTION MARK） | ops.inc:490-491 |
| `Cpu::layout_` 偏移 0 + 64 字节对齐 | savestate 二进制兼容约束，**勿移动字段** | cpu.cpp:14-26 |
| `BRK_3BYTE_HACK` | 可选的 BRK 3 字节调试 hack（默认关） | types.h:38、x6502.cpp:639 |
| `opsize/optype/opwrite[256]` | 调试器/追踪用的指令元数据表，改 opcode 时同步 | x6502.cpp:638-730 |

## 四、CPU 侧已知精度限制

（详见 [precision.md](precision.md) 与 `tests/fixtures/blargg_known_fail.json` v1.16.0-P5.1）

- 指令级：`instr_v5_*`/`all_instrs`（组合挂、单组过的 accuracy gap）、`instr_timing`
  （周期表+边界加罚不精确）。
- 中断：`cpu_int_2/3/4/5`——指令边界轮询、无 hijack/branch-delay 的结构性近似；
  补齐需要把中断轮询下沉到指令内固定周期点。
- Dummy write：`cpu_dummy_writes_oam/ppu`、`cpu_exec_space_ppuio`、`instr_misc*`。
- 复位/RAM：`cpu_reset_ram/regs`。

## 五、相关文档

- 时钟单位与 PPU 交错：[ppu.md](ppu.md) §2.1
- APU hook（`FCEU_SoundCPUHook` 每指令逐周期推进帧计数器）：[apu.md](apu.md) §2.2-2.3
- 模数总表：[constants.md](constants.md)
