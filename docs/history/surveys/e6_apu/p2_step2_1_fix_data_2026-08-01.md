# E-3 APU R6 延续 — P2 Phase 2 Step 2.1 实测记录 (2026-08-01)

> **目的**：P2 精度收敛 Phase 2 Step 2.1（power-on/reset 相位分离）的实测记录：
> instrument 验证 + 强制回归 + golden 重生验证。延续 `r6_step1/2/3` 调查链。
> **状态**：✅ 已落地（commit `e3(step2.1)`）。相位修正经探针证实；Step 2.1 **未单独闭合**缺陷 1 ROM
> （符合方案 §1.4 三步根因预期，剩余"写→IRQ 延迟 + jitter"属 Step 2.2）。
> **配套**：`docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` Phase 2

---

## 1. 改动内容

- `src/sound.h` / `src/sound.cpp`：`FCEUSND_Reset(void)` → `FCEUSND_Reset(bool is_power)`
  - **power**（`is_power=true`）：`IRQFrameMode=0x0`（等效 $4017=$00）+ `fcnt=1`
  - **reset**（`is_power=false`）：**保留 `IRQFrameMode` 最后写入值**（bit7 模式不变；bit6 inhibit 先按保留实测，
    用 `apu_reset_4017_written` 判据校准）+ `fcnt=1`
  - `fcnt=1`（写后相位）：R6-2b（2026-08-01）曾试 `fcnt=1` 被 golden 碎裂误伤回滚；P2 方案 §1.4 复核判定
    **方向正确（只修了一半）**，本步恢复并补齐 reset 语义
- `src/fceu.cpp:900`：`ResetNES`（软复位）→ `FCEUSND_Reset(false)`
- `src/sound.cpp:1284`：`FCEUSND_Power`（上电）→ `FCEUSND_Reset(true)`
- 同时改写 `sound.cpp:1200-1209` 处 R6-2b 遗留的误导注释（"fcnt 不是根因" → 修正为三步根因表述）

## 2. Instrument 验证（`FCEUX11_E3_TRACE=1`，ROM: `apu_reset_4017_timing`）

**改动前**（r6_step1 基线）：power-on 后 fcnt 序列 `0,1,2,3` → IRQ 于**首个 quarter≈7459** 置位（过早，缺陷 1）。

**改动后**（本步实测，power-on 首个 FSU 序列）：

```
E3 FSU fcnt=1 mode=0x0 fhcnt=-24 sirq=0x0
E3 FSU fcnt=2 mode=0x0 fhcnt=0   sirq=0x0
E3 FSU fcnt=3 mode=0x0 fhcnt=-72 sirq=0x0
E3 FSU fcnt=0 mode=0x0 fhcnt=0   sirq=0x0   ← IRQ 于第 4 个 quarter 置位（sirq=0x40）
```

✅ **相位修正证实**：fcnt 序列 `1,2,3,0`，IRQ 从首个 quarter 移至**第 4 个 quarter=29830**，
符合硬件真值（blargg readme：IRQ flag 可见点 ≈29831，handler ≥29833）。

## 3. APU 强制回归（40 ROM，零 PASS→FAIL）

| 组 | 结果 |
|---|---|
| `apu_01`~`apu_11`（含 apu_07/08 稳态，inter-IRQ 周期） | **11/11 PASS** |
| `apu_single_1/2/3/7/8` | **5/5 PASS**（**`apu_single_3` 缺陷 2 修复保留**） |
| `pal_apu_01`~`08`,`10`,`11` | **10/10 PASS** |
| `apu_mixer_*`（`--frames 2400`） | **4/4 PASS** |
| `apu_reset_4015` / `apu_reset_irq_cleared` / `apu_reset_len_ctrs` | **3/3 PASS** |
| `apu_single_4_jitter` | FAIL 0x02（未闭合，需 Step 2.2 jitter） |
| `apu_single_5_len_timing` | FAIL 0x02（diag "First length of mode 0 is too soon" → Step 2.2 延迟） |
| `apu_single_6_irq_timing` | FAIL 0x02（diag "Flag first set too soon" → Step 2.2 延迟） |
| `apu_reset_4017_timing` | FAIL **0x02→0x03**（diag "Delay after effective \$4017 write: ~2-3" → **写→IRQ ~2-3 周期延迟，正是 Step 2.2 目标**） |
| `apu_reset_4017_written` | FAIL 0x02（diag "At power, \$4017 should be written with \$00"；reset 语义待校准，作 Step 2.1/2.2 校准 oracle） |
| `apu_reset_works_imm` | FAIL 0x02（"At power, writes should work immediately"） |
| `apu_test`（组合套件） | FAIL 0x01（停在 sub-test 4 jitter，与 `apu_single_4` 同源） |

**结论**：40 个 APU ROM **零 PASS→FAIL 回归**；未闭合项全部指向方案 §1.4 三步根因的剩余两块
（② W4017→IRQ ~1-2 周期延迟；③ jitter），与方案预期一致。

## 4. Oracle A / B

- **Oracle A**：`ctest -LE perf` → **34/34**（golden 重生后）
- **Oracle B**：全量 177 ROM → **PASS=121 FAIL=56**，与 CI-R4 基线（commit `1156ca1`）**完全一致，零计数回归**
  （`rom_regression_test` 视觉 CRC 亦 PASS → 无渲染回归）
- 注：manifest 批量模式不带 `--reset-after`/mixer 2400，`apu_reset_*` 显示 0x81、`apu_mixer_*` 显示 0x80
  属已知 harness 类 FAIL（方案 §5 Step 3.1 分桶），非精度回归

## 5. Golden 重生（预期流程，`tests/CMakeLists.txt:348`）

- `fceux11_golden_savestate_test --generate`：7 个 .fc0 重写 + `golden_index.json` MD5 更新；
  **`fds_bios.fc0` 未变**（不受 APU 帧计数器影响，幂等验证）
- `fceux11_savestate_regression_test --generate`：12 哈希重写 `golden_savestate_hashes.json`
- **字节级验证**（`nrom_smb_title.fc0`，87536 字节，HEAD 版 vs 新版）：仅 **5 字节**变化
  - 偏移 5170-5173：`fhcnt`（32 位计时值，相位不同）
  - 偏移 5194：`fcnt`（0x03→0x00，相位偏移 1 quarter 的直接证据）
  - 其余 CPU/PPU/RAM/其他 APU 状态在 30 帧后**完全一致** → 仅运行期起始值变化，无 chunk 结构漂移

## 6. 结论与下一步

- Step 2.1 相位修正**正确、零回归**，已独立 commit（`e3(step2.1)`）
- 未闭合项（`apu_reset_4017_timing` 0x03 的 ~2-3 周期延迟、`apu_single_4/5/6` 0x02 的 too-soon）全部指向
  **Step 2.2（`Write_IRQFM` 计时器重置延迟 + IRQ 可见点 29831 + jitter）**；`apu_reset_4017_written`
  作 reset 语义校准 oracle
- **下一步建议**：Step 2.2（`src/sound.cpp:1050-1090` `Write_IRQFM` + `:543-579` `FCEU_SoundCPUHook`）
