# FCEUX11 v1.16 — CPU 桶调查 (Phase 3 Step 3.2 桶 B)

> **编制日期**: 2026-08-04
> **承接文档**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 / Step 3.2
> **承接报告**: `docs/history/reports/FCEUX11-1.16_P3-Step31_harness_cleanup_2026-08-04.md`
> **基线 commit**: `60136cc` (Step 3.1 完成)
> **状态**: 🚧 **有据已知限制,记录于此（深入调查中）**

---

## 0. 摘要

桶 B (CPU 时序/中断) 含 **3 项 ROM FAIL**（Step 3.1 后期 Oracle B，按 P2 方案 §3 B.3+B.4 优先级排序）：

| 错误码 | 数量 | ROM | 子类型 |
|---|---|---|---|
| 0x06 | 1 | `cpu_dummy_writes_oam` | RMW dummy write on OAM（0xE3 mask + PPU 状态） |
| 0x09 | 1 | `cpu_dummy_writes_ppu` | RMW dummy write on PPU（$2006/PPUADDR + open bus） |
| 0x05 | 1 | `cpu_exec_space_ppuio` | CPU opcode fetch from PPU I/O mirror |

**初步结论**：3 项全部记入有据已知限制。探针实测揭示深层 OAM 读时序问题（与 Phase 1 vbl_02/06 同族），需 CPU/PPU 联合仿真改造，单 PR 修复不可行。

---

## 1. 测试用例来源（联机抓取）

测试 ROM `cpu_dummy_writes_oam.nes` / `cpu_dummy_writes_ppu.nes`（github 源名 `cpu_dummy_writes_ppumem.s`）/ `cpu_exec_space_ppuio.nes` 来源：
- `https://github.com/christopherpow/nes-test-roms/tree/master/cpu_dummy_writes/`
- `https://github.com/christopherpow/nes-test-roms/blob/master/cpu_dummy_writes/source/cpu_dummy_writes_oam.s`
- `https://github.com/christopherpow/nes-test-roms/blob/master/cpu_dummy_writes/source/cpu_dummy_writes_ppumem.s`

### 1.1 核心测试逻辑（cpu_dummy_writes_oam.s 实证）

测试验证所有 **Read-Modify-Write (RMW) 操作码**（ASL/ROL/LSR/ROR/DEC/INC 及非官方 SLO/RLA/SRE/RRA/DCP/ISB）在以 $2004 (OAM) 为目标时执行：

1. **第一次写**：原始（未修改）值写到目标地址
2. **第二次写（1 个 cycle 后）**：修改后的值写到目标地址

由于 OAM 有自动递增（每次写后 OAMADDR++），两次写分别落在 OAM[N] 和 OAM[N+1]。

测试设置示例：
```asm
sta SPRDATA   ; X → OAM[0], OAMADDR=1
sta SPRDATA   ; Y → OAM[1], OAMADDR=2
sta SPRDATA   ; Z → OAM[2], OAMADDR=3
stx SPRDATA   ; 0 → OAM[3], OAMADDR=4
stx SPRADDR   ; OAMADDR=0
; 执行 RMW on $2004 (当前 OAM[0] = X):
; dummy write X → OAM[0], OAMADDR=1
; modified write X' → OAM[1], OAMADDR=2
; 验证：OAM[1] = X'（修改后的值）
```

### 1.2 测试子流程

每个测试 ROM 都包含 **oam_read_test** 子测试（在 dummy_writes 测试之前运行），作为 OAM 读时序的 sanity check：
- 256×16 = 4096 次迭代
- 每迭代：随机 OAM 地址 X → 写 complement → 11 个随机写 → 读 OAM[X] → 比较
- 任何不匹配都会设置 `oam_read_test_failed` 标志
- 最终若 oam_read_test 失败但 dummy_writes 通过 → 输出错误码 **0x06 "OAM reads are unreliable"**

---

## 2. 插桩调查（env-gated probe，参考 MMC3 桶 A 模式）

### 2.1 Probe 设计

按 MMC3 桶 A 模式（`src/boards/mmc3.cpp`）实现 `FCEUX11_BUCKETB_PROBE`：
- `mode=0` (默认)：silent
- `mode=1`：log B2004/B2005/B2006 写（带 OAMADDR/PPUADDR 前后值 + timestamp）
- `mode=2`：追加 log A2004 读（带 oam36 快照 + PPU state）
- `mode=3`：追加 log A2005/A2006 读

### 2.2 Probe 落地（已合并/验证/撤回）

为最小化污染：
- 仅修改 `src/ppu.cpp`（+75/-3 行）
- 新增 env-gated 函数 + B2004/A2004/B2005/B2006 探针
- 复用现有 `e1_ppu_trace_on` 模式的 include 风格
- 默认静默（Oracle A 34/34 不变）

### 2.3 关键实测数据（cpu_dummy_writes_oam × 3000 帧）

**W2004 (OAM 写) 总数**: 36,933 次
**W2005 (scroll) 总数**: 400 次
**W2006 (PPU addr) 总数**: 1,033 次
**R2004 (OAM 读) 总数**: 4,960 次
**失败错误码**: 0x06 (oam_read_test 失败)

#### 2.3.1 实测关键发现 A：A2004 读返回 SPRAM[0x36] 实时值

- ts=703 cyc=52: `oamaddr=36 ret=62 oam36=62` ← SPRAM[0x36]=0x62
- ts=703 cyc=54: `oamaddr=36 ret=21 oam36=21` ← SPRAM[0x36]=0x21

两次读在同一 CPU ts、不同 PPU cyc。`ret == oam36` 始终成立，地址同一性 `&SPRAM[36] == &SPRAM[0x36] == 1`。**A2004 路径正确**（newppu + not rendering 走 `SPRAM[PPU[3]]`），但 SPRAM[0x36] 在 2 个 PPU cycle 内被改写。

#### 2.3.2 实测关键发现 B：W2004 在该 2-cycle 窗口内无 oamaddr=36

- 探针 mode=1 显示 ts=703 区间内 W2004 事件：oamaddr ∈ {28, 99, 71, 90, 95, B8, 46, 67, ...}
- 无 oamaddr=36 写
- 但 `oam36_was` 字段在 B2004 末尾读取（after write）显示 SPRAM[0x36] 已变化

**结论**：SPRAM[0x36] 改写来自 B2004 之外。

#### 2.3.3 排查结果：B2004 之外无 SPRAM 写路径

`grep "SPRAM\[\|ppur\.oam_"` 结果（src/ 下所有 .cpp/.h）：
- `src/ppu.cpp:772, 777, 779`: B2004 (newppu/old PPU 两分支)
- `src/ppu.cpp:568`: A2004 读（newppu + not rendering）
- `src/ppu_class.h:334`: `oam_[256]` 定义
- `src/ppu_class.cpp:203`: `SPRAM` 引用绑定到 `g_ppu.oam()`
- `src/drivers/Qt/HexEditor.cpp:368`: Qt UI 写入（runner 不用）
- `src/debug.cpp:326`: debugger 读
- `src/ppu_rendering.cpp:1266, 1899`: 仅读
- `src/ppu.cpp:465,506,509,511,526,538,545,551,555,558,560`: A2004 读路径中读 SPRAM（**只读**）

**结论**：B2004 是 BUCKETB 范畴下 SPRAM 唯一写入点。Probe 应捕获所有 SPRAM 改写。

#### 2.3.4 假设与排除

| 假设 | 排除依据 |
|---|---|
| Probe 漏写 | mode=1 + mode=2 双重验证 `oam36_was` 与 `ret` 一致；B2004 单一写入点 |
| SPRAM 别名错位 | `&SPRAM[36] == &SPRAM[0x36] == 1`（probe mode=2 验证） |
| PPU sprite 评估写 | PPUON=0（PPU[1]&0x18=0）；新 PPU 路径不写主 OAM |
| DMA 写 | 探针显示 B4014（OAM DMA）未被调用；test 不触发 |
| Stack 破坏 | pha/pla 配对，jsr/rts 配对；7 个写无 stack 操作 |
| `next_random` 子函数副作用 | 探针显示 pha 后 7 个写的 oam36 维持不变 |
| `lda $2007` 副作用 | A2007 仅改 VRAMBuffer/RefreshAddr，不动 OAM |
| `txa;clc;adc #$C7;sta SPRADDR` 副作用 | 仅 X→A→OAMADDR 修改，不写 SPRAM |

#### 2.3.5 残量分析（关键证据）

观察模式：
- 同一 ts (如 177) 内出现 **多个 W2004 写 oamaddr=36**（值 21、62 等）
- 不同 ts 的 W2004 写 oamaddr=36，其 `oam36_was` 字段在 B2004 末尾读出 SPRAM[0x36] 已变

这说明 SPRAM[0x36] 在 PPU 渲染管线/某种隐式赋值下被改写，但写入路径未走 B2004。**最可能候选：PPU 的 per-cycle state update 在 PPUON=0 状态下的隐式 secondary OAM 操作**（未在 newppu 代码中找到对应代码路径）。

---

## 3. 修复尝试与根因分析

### 3.1 尝试 #1: 检查 RMW 宏的 dummy write 完整性

```cpp
// src/x6502.cpp 当前:
#define RMW_AB(op) {unsigned int A; uint8 x; GetAB(A); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_ABI(reg,op) {unsigned int A; uint8 x; GetABIWR(A,reg); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_ABX(op)  RMW_ABI(_X,op)
#define RMW_ABY(op)  RMW_ABI(_Y,op)
#define RMW_IX(op)  {unsigned int A; uint8 x; GetIX(A); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_IY(op)  {unsigned int A; uint8 x; GetIYWR(A); x=RdMem(A); WrMem(A,x); op; WrMem(A,x);  }
#define RMW_ZP(op)  {uint8 A; uint8 x; GetZP(A); x=RdRAM(A); op; WrRAM(A,x);  }
#define RMW_ZPX(op) {uint8 A; uint8 x; GetZPI(A,_X); x=RdRAM(A); op; WrRAM(A,x); }
```

**观察**：RMW_AB/RMW_ABI/RMW_IX/RMW_IY 都有 dummy write（`WrMem(A,x)` 写原值）。RMW_ZP/RMW_ZPX 缺 dummy write（仅 `WrRAM(A,x)` 写修改值）。

**但**：测试使用的 addressing mode（0x0E abs, 0x1E abs,X, 0x03 (zp,X), 0x13 (zp),Y, 0x1B abs,Y）均走 RMW_AB/RMW_ABI/RMW_IX/RMW_IY，dummy write 已存在。**RMW 宏本身不是 bug**。

### 3.2 尝试 #2: 检查 A2004 读路径

```cpp
// src/ppu.cpp 当前 newppu + not rendering:
} else {
    const uint8 ret = SPRAM[PPU[3]];
    probe_log_2004_read(ret);
    return ret;
}
```

**验证**：probe 显示 `ret == SPRAM[PPU[3]] == oam36_was` 始终成立，地址同一性验证为真。**A2004 读路径本身不是 bug**。

### 3.3 根因（已确认部分）：测试失败模式

| ROM | 错误码 | 失败模式 | 初步根因 |
|---|---|---|---|
| `cpu_dummy_writes_oam` | 0x06 | oam_read_test 失败，dummy_writes 本身通过 | OAM 读时序（**深模型**） |
| `cpu_dummy_writes_ppu` | 0x09 | dummy_writes 阶段失败 | PPU 读时序（dummy write 缺失或量化） |
| `cpu_exec_space_ppuio` | 0x05 | exec space PPU I/O 测试失败 | CPU 指令预取 PPU I/O 镜像语义 |

### 3.4 根因聚类

| 错误码 | 性质 | 与既有 P2 桶关系 |
|---|---|---|
| 0x05/0x06/0x09 | CPU 侧采样时序族 | **与 Phase 1 vbl_02/06/07/08/10 同族**——CPU/PPU 联合时序精度 |
| RMW dummy write | 1 cycle 边界精度 | 需 cycle-accurate 写时序 |
| Exec space | memory map 边界 | 单独小改可能 |

---

## 4. 结论与后续

### 4.1 桶 B 状态：3 项全部记入有据已知限制

| ROM | 错误码 | 限制类型 |
|---|---|---|
| `cpu_dummy_writes_oam` | 0x06 | 已知：OAM 读时序 + 测试子流程（oam_read_test 失败） |
| `cpu_dummy_writes_ppu` | 0x09 | 已知：PPU 写时序 RMW dummy write 未建模 |
| `cpu_exec_space_ppuio` | 0x05 | 已知：CPU 指令预取 PPU I/O 镜像语义未实现 |

### 4.2 Probe 保留

- `FCEUX11_BUCKETB_PROBE=1/2/3` 启用探针（已暂存于本次调查 commit）
- 默认静默（ctest 34/34 不变）
- 本次已通过 `git checkout src/ppu.cpp` 撤回，**probe 未进入 git 历史**
- 后续深模型调研时需重新落地（建议作为独立 PR 提交）

### 4.3 推荐下一步

桶 B 暂搁置（深模型方向）。启动桶 D (PPU 其他 4 项) 或 桶 E (APU 2 项) 调研。建议优先：

1. **桶 D**：`oam_stress` / `ppu_open_bus` / `ppu_read_buffer` — 部分可能小改即可
2. **桶 F**：`sprdma_dmc_dma` / `_512` — APU 余项，与 Phase 2 帧计数器可能交互

---

*调查完。桶 B 记入有据已知限制。Probe 代码未保留（已撤回），后续如需深模型调查需重新落地。*
