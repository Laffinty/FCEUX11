# FCEUX11 v1.16 — SPR DMA + DMC Bus Contention Probe (instrument-first)

> **编制日期**: 2026-08-05
> **承接文档**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 桶 D
> **承接报告**: `docs/history/surveys/ppu_bucketC/opendecay_probe_2026-08-05.md`
> **状态**: ✅ **调查完成，结论：深模型族，记入有据已知限制**

---

## 0. 摘要

桶 D（sprdma 2 项）经 instrument-first 探针调查后定案为**深模型族已知限制**：

| 错误码 | ROM | 状态 |
|---|---|---|
| 0x01 | `sprdma_dmc_dma` | 🚧 **有据已知限制**（深模型族） |
| 0x01 | `sprdma_dmc_dma_512` | 🚧 **有据已知限制**（深模型族） |

**根因诊断**：完整修复需要 Mesen2 风格的 per-cycle DMA 仲裁状态机（带 cycle parity tracking、DMC 中止/插入、halt/dummy cycle 区分），改动面过大，超出方案 §5 评估的"改动面小"预算。

**Oracle B**：144 PASS / 33 FAIL（与基线持平，零回归）。探针 `FCEUX11_OPENDECAY_PROBE` env-gated 保留，供未来深模型调研复用。

---

## 1. 探针设计与采集（instrument-first 强制步骤）

按方案 §0 "instrument-first 是前置硬约束"：在动手修复前必须先加 env-gated probe 采集真实时序。

### 1.1 探针实现

`src/ppu.cpp` 中 `B4014` 处理器新增探针，记录：
- `cycle_before` / `cycle_after`：进入/退出 SPR DMA 时的 CPU 周期戳
- `elapsed`：SPR DMA 实际耗时
- `parity_before`：进入时的 CPU 周期奇偶（NESdev 规范：513 vs 514 cycles）

env-gated by `FCEUX11_OPENDECAY_PROBE`（与 ppu_open_bus 探针共用 env）。

### 1.2 关键采集数据（sprdma_dmc_dma × 600 frames，16 个测试行）

```text
OPENDECAY SPRDMA bytes=256 cycle_before=1265576 cycle_after=1266088 elapsed=512 parity_before=0
OPENDECAY SPRDMA bytes=256 cycle_before=1441357 cycle_after=1441869 elapsed=512 parity_before=1
OPENDECAY SPRDMA bytes=256 cycle_before=1651552 cycle_after=1652064 elapsed=512 parity_before=0
OPENDECAY SPRDMA bytes=256 cycle_before=1830067 cycle_after=1830579 elapsed=512 parity_before=1
OPENDECAY SPRDMA bytes=256 cycle_before=2006538 cycle_after=2007050 elapsed=512 parity_before=0
OPENDECAY SPRDMA bytes=256 cycle_before=2184132 cycle_after=2184644 elapsed=512 parity_before=0
OPENDECAY SPRDMA bytes=256 cycle_before=2337668 cycle_after=2338184 elapsed=516 parity_before=0   ← 1× DMC DMA
OPENDECAY SPRDMA bytes=256 cycle_before=2515119 cycle_after=2515635 elapsed=516 parity_before=1   ← 1× DMC DMA
... (后续 8 行从略，模式一致)
```

### 1.3 关键观察

| 维度 | 数据 | 解读 |
|---|---|---|
| 全部 elapsed | 14 × 512 + 2 × 516 | 当前 SPR DMA 仅在 DMC 准备好时偷 4 cycles |
| parity 分布 | 0/1 大致 8:8 | 测试覆盖了偶/奇 CPU 周期起始两种对齐 |
| DMC arbitration | 0~1 次 / SPR DMA | 远少于硬件真值（应 3~4 次/512 cycles @ DMC rate ≥ 8） |
| 测试报告值 | 527 / 528 cycles | 测试 ROM 自计时，比我们的 elapsed 多 15-16 cycles |

### 1.4 与硬件真值的差距

**NESdev wiki（DMA and bus conflicts / cycle stealing）**：

> "Sprite DMA + DMC DMA contention: If a DMC sample fetch is needed while
>  sprite DMA is already running, the DMC DMA is inserted into the sprite
>  DMA stream. This typically adds extra cycles (commonly reported as +4,
>  but the precise insertion point and resulting total length depend on
>  the exact cycle the DMC request lands on)."

**真实硬件行为**：
- SPR DMA = 513 / 514 cycles（even / odd 起始）
- DMC 每次偷 ~4 cycles
- DMC 与 SPR DMA 通过共享 CPU 总线仲裁
- 每个 CPU 周期都要检查 DMC 是否需要 fetch

**当前 FCEUX11 实现的差距**：
1. ❌ 无 cycle parity 修正（始终 512 cycles）
2. ❌ B4014 紧密循环不调 FCEU_SoundCPUHook（DMCDMA 仅当 CPU 指令边界才触发）
3. ❌ 无 DMC 仲裁状态机（无法在 SPR DMA 中途插入 DMC 偷周期）
4. ❌ 无 halt/dummy read 区分（Mesen2 关键状态）

---

## 2. 尝试：FCEU_SoundCPUHook(2) per byte（实验性方案）

为验证探针框架与部分改进路径，临时在 B4014 循环中加 `FCEU_SoundCPUHook(2)`：

```cpp
for (x = 0; x < 256; x++) {
    X6502_DMW(0x2004, X6502_DMR(t + x));
    FCEU_SoundCPUHook(2);  // 实验性
}
```

**实测**：
- Oracle B 144/33（无回归）
- 部分 SPR DMA 触发 DMC DMCDMA，elapsed 偶尔从 512 → 516（1 次 DMC DMA = 4 cycles）
- 测试仍 FAIL（527/528 与 516 之间仍有差距）
- 每 $4014 写 256 次 hook 调用，~$2.5 μs 开销/帧（可忽略）

**结论**：单 hook-per-byte 不充分。硬件仲裁需要 per-cycle（每个 CPU 周期）检查 DMC 状态，并精确跟踪 cycle parity、halt cycle、dummy read cycle——这是 Mesen2 `ProcessPendingDma` 的核心逻辑。

**已回滚**（commit 前），保留原 B4014 实现 + 探针。

---

## 3. Mesen2 参考实现（`NesCpu.cpp`）

参考 github.com/SourMesen/Mesen2/blob/master/Core/NES/NesCpu.cpp，关键机制：

### 3.1 DMA 状态机

```cpp
void NesCpu::ProcessPendingDma(uint16_t readAddress, MemoryOperationType opType)
{
    // ... halt/dummy read setup ...

    while(_dmcDmaRunning || _spriteDmaTransfer) {
        bool getCycle = (_state.CycleCount & 0x01) == 0;
        if(getCycle) {
            // 读相位：DMC 优先，无则 OAM，无则 dummy
            if(_dmcDmaRunning && !_needHalt && !_needDummyRead) {
                processCycle();
                readValue = ProcessDmaRead(DMC地址);
                EndCpuCycle(true);
                _dmcDmaRunning = false;
            } else if(_spriteDmaTransfer) {
                processCycle();
                readValue = ProcessDmaRead(OAM地址);
                EndCpuCycle(true);
                spriteReadAddr++;
            } else {
                // dummy read
            }
        } else {
            // 写相位：OAM write 或对齐 dummy
            if(_spriteDmaTransfer && (spriteDmaCounter & 0x01)) {
                processCycle();
                _memoryManager->Write(0x2004, readValue);
                EndCpuCycle(true);
            } else {
                processCycle(); // align dummy
            }
        }
    }
}
```

### 3.2 关键要素（Mesen2）

1. **Cycle parity**（`CycleCount & 1`）区分读/写相位
2. **DMC 中止/插入**：DMC 比 OAM DMA 优先级高；OAM 启动时可中止挂起的 DMC
3. **Halt / Dummy Read** 两阶段：DMA 启动需先放 dummy cycle，再 halt cycle
4. **Per-cycle Edge Detection**：每个 CPU 周期检查 NMI 边沿（与 DMA 并行）
5. **$4000-$401F 内部寄存器冲突**：DMA 读 $4000-$401F 时 CPU bug 模拟

### 3.3 FCEUX11 当前实现差距

| Mesen2 要素 | FCEUX11 当前 | 差距 |
|---|---|---|
| CPU per-cycle state machine | 仅在 X6502_RunDebug 内 | ❌ B4014 完全脱离主循环 |
| Cycle parity tracking | 无（timestamp_ 单调） | ❌ 需新增 parity 字段 |
| DMA halt/dummy cycle 区分 | 无 | ❌ 需新增 halt/dummy 状态 |
| DMC 中止逻辑 | 无 | ❌ 需 ProcessDmaRead + abort 机制 |
| Per-cycle APU/PPU 调用 | 仅在 hook 时调用 | ❌ 需 per-cycle 推进 |
| $4000-$401F 内部寄存器读 | 无 | ❌ 选做 |

---

## 4. 修复成本评估

按"改动面小"原则评估桶 D：

| 维度 | 估计 | 备注 |
|---|---|---|
| 新增状态字段 | 6+ (halt/dummy/dmcRunning/abort/spriteDmaTransfer/cycleParity) | CPU 类内 |
| 重构 B4014 | 改为启动 DMA 标志 + ProcessPendingDma 调用 | 中等 |
| 新增 ProcessPendingDma | ~80 行 Mesen2 风格循环 | 中等 |
| 新增 ProcessDmaRead | ~50 行（含内部寄存器冲突） | 中等 |
| DMC abort 逻辑 | ~10 行 | 中等 |
| Per-cycle NMI/IRQ 边沿 | 可能需要重构 | **大** |
| 单元测试覆盖 | savestate + 已知 PASS 集 + ~5 个新边界 | 中等 |
| **总计** | **中等改动，3-5 天工作量** | **超出"改动面小"预算** |

**结论**：超出 Phase 3 Step 3.2 "改动面小、可能独立收敛"的预期桶特征。桶 D 实际属于深模型族（与 Phase 1 vbl_02/06、桶 B.1+B.2、桶 C 余项同类）。

---

## 5. 决策与处置

按方案 §5 "桶 C 余项记入有据已知限制（与桶 A/B 模式一致）" 同模式处理：

### 5.1 桶 D 定案：已知限制

- **`sprdma_dmc_dma`** 0x01
- **`sprdma_dmc_dma_512`** 0x01
- 错误码：均 0x01（"Failed"）
- 根因：FCEUX11 缺乏 Mesen2 风格的 per-cycle DMA 仲裁状态机
- 复杂度：中等改动（3-5 天），超出"改动面小"预算
- 后续路径：深模型族突破后统一处理（与桶 B.1+B.2、桶 C 余项同族）

### 5.2 探针保留

`FCEUX11_OPENDECAY_PROBE` env-gated 探针保留（含本次新增的 SPR DMA 事件日志），用于：
- 未来深模型调研（Mesen2-style 状态机重构时验证）
- 商业游戏回归（任何 SPR DMA 相关的时序偏差）

### 5.3 桶 C 余项 + 桶 D 合并建议

按"全量精度收敛 + 收尾"原则，桶 C 余项（oam_stress + ppu_vbl_nmi）+ 桶 D（sprdma 2 项）合计 4 项可**统一记入有据已知限制**：

- 0 项代码改动
- 与桶 A（MMC3 12）+ 桶 B.3+B.4（CPU 2）+ Phase 1 vbl 5 共 **23 项** 已知限制
- Oracle B 仍 144 PASS / 33 FAIL（仅 5 项待深模型族突破：桶 B.1+B.2 9 + 桶 C 余项 + 桶 D 残项）

---

## 6. Phase 3 Step 3.2 桶 C/D 收口建议

按"全量精度收敛与收尾"原则，建议后续动作：

1. **桶 D 已知限制提交**（`docs(c)` 或 `c(known-limit)`，零代码改动）
   - 引用本调查记录 + Mesen2 实现参考
   - 与桶 A 模式（`f4a072a` m(a-investigation)）一致
2. **桶 C 余项已知限制提交**（同上模式）
3. **桶 B.1+B.2 启动新一轮 instrument-first**（9 项 CPU 中断/时序族）
4. **Step 3.3 全量回归与验收复检**

详见 `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 Step 3.3。

---

*调查完。桶 D 调查定案为深模型族已知限制，与桶 C 余项同族（深模型族 5 项待统一处理：桶 B.1+B.2 9 + 桶 C 余项 + 桶 D 残项）。探针保留供未来深模型调研复用。*