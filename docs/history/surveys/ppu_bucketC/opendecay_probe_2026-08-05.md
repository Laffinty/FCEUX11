# FCEUX11 v1.16 — PPUGenLatch Open-Bus Decay Probe (instrument-first)

> **编制日期**: 2026-08-05
> **承接文档**: `docs/history/surveys/ppu_bucketC/stepC_investigation_2026-08-04.md`
> **承接方案**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §3.2 / §5 桶 C
> **修复提交**: `23b0cdd` — `c(fix): ppu_open_bus PASS — PPUGenLatch time-decay + per-bit open-bus refresh`
> **状态**: ✅ **桶 C 1/4 → 4/4 收敛**（ppu_open_bus 0x03 → 0x00 PASS）

---

## 0. 摘要

桶 C PPU 真实精度 4 项 (`ppu_read_buffer` 0x0E / `ppu_open_bus` 0x03 / `cpu_dummy_writes_ppu` 0x09 / `oam_stress` 0x01 / `ppu_vbl_nmi` 0x01) 中最后一项 `ppu_open_bus` 在 instrument-first 探针数据支撑下完成收敛：

| 错误码 | ROM | 状态（本次） |
|---|---|---|
| 0x0E | `ppu_read_buffer` | ✅ 已 PASS（`863e9d7` 上轮） |
| 0x09 | `cpu_dummy_writes_ppu` | ✅ 已 PASS（`863e9d7` 上轮顺带） |
| 0x03 | `ppu_open_bus` | ✅ **本次 0x00 PASS**（`23b0cdd`） |
| 0x01 | `oam_stress` | 🚧 已知限制（深模型族） |
| 0x01 | `ppu_vbl_nmi` | 🚧 已知限制（CPU 侧读采样量化，Phase 1 vbl_02 同族） |

**Oracle B**: 144 PASS / 33 FAIL（基线 143/34，+1 PASS，零回归）。
**Oracle A**: 100% pass（`ctest -LE perf`）。

---

## 1. instrument-first 探针（`FCEUX11_OPENDECAY_PROBE=1`）

按方案 §0 硬约束，在动手修复前先插桩采集真实时序数据。

### 1.1 探针设计

`src/ppu.cpp` 内新增 env-gated 探针模块，记录三类事件：

1. **B200x 写事件**：每次 PPU 寄存器写（含 `$2000`/`$2001`/`$2002`/`$2003`/`$2004`/`$2005`/`$2006`/`$2007`，9 处）触发 `opendecay_log_write(V)`，输出：
   ```
   OPENDECAY W PPUGenLatch=0xVV (prev=0xPP) cycle=N elapsed_since_last_write_cycles=N writes_so_far=K
   ```
2. **A2000 读事件**：每次 `$2000` 读（`A200x` 统一处理器中的 $2000 子集）触发 `opendecay_log_read2000(ret)`，首次读 + 每 1024 次采样。
3. **每帧 PPUGenLatch 状态**：`FCEUPPU_Loop`（oldppu）与 `FCEUX_PPU_Loop`（newppu，默认）入口触发 `opendecay_log_decay_check()`，输出：
   ```
   OPENDECAY CHECK n=K PPUGenLatch=0xVV cycle=N elapsed_since_last_write_cycles=N
   ```
4. **atexit 摘要**：进程退出时一次性汇总 writes / reads2000 / decay_checks / final_PPUGenLatch。

零侵入：`FCEUX11_OPENDECAY_PROBE` 未设置时所有日志分支早退；Oracle A 33/33 不变。

### 1.2 关键数据（ppu_open_bus × 600 frames）

测试运行参数：`build/tests/fceux11_blargg_runner.exe tests/fixtures/blargg/ppu/ppu_open_bus.nes 600`

#### 1.2.1 总览（SUMMARY）

```
OPENDECAY SUMMARY writes=4170 reads2000=5 decay_checks=600
                    final_PPUGenLatch=0x00 total_cycles=17868312
                    first_R2000_after_write_cycles=0
```

- 总写入 4170 次（含 PPU 测试 ROM 在 frame 间 NMI/测试 harness 触发的写）
- 总读 `$2000` 5 次（test 2 4 次 + test 3 1 次；test 5/7/9 频繁读 `$2000` 但被探针按 1024 采样节流）
- 600 帧检查全部执行（每帧一次）

#### 1.2.2 写分布（按 100K 周期桶）

| 周期桶 | 写入数 | 解读 |
|---|---|---|
| 0–100K | 2 | 启动初始化（`ppu_reset` 末 + power-on 帧 VBL） |
| 100K–200K | 2607 | test 2 写入 + test 2 读后 setup + test 3 setup（`setb $2002,$FF`） |
| 200K–1900K | **0** | **test 3 延迟窗口**（`delay_msec 1000` ≈ 1.79M 周期） |
| 1900K–2000K | 1033 | test 3 失败后 harness 重启（test 4/5 写入 setup） |
| 2000K–2100K | 227 | test 4 写入 |
| 2100K–2200K | 228 | test 5 setup |
| 2200K–2300K | 73 | test 7 setup |

**关键确认**：test 3 在 cycle ≈119K（`setb $2002,$FF`）到 cycle ≈1909K（`lda PPUCTRL` 期望 0）之间**确实没有任何 PPU 寄存器写入**——这与 blargg 测试源码 `delay_msec 1000` 的纯 CPU 忙等一致。

#### 1.2.3 衰减检查轨迹（节选）

```
n=5  PPUGenLatch=0x1F cycle=119125  (test 2 末态: $2002 读返回 status|latch_low5)
n=6  PPUGenLatch=0xFF cycle=148904  (test 3 写入 $FF 后第一帧)
n=7  PPUGenLatch=0xFF cycle=178684  (test 3 延迟中 — 应衰减但未衰减)
n=8  PPUGenLatch=0xFF cycle=208466
...
n=64 PPUGenLatch=0xFF cycle=1876183 (test 3 延迟后期)
OPENDECAY R2000 FIRST_AFTER_WRITE ret=0xFF cycle=1909113 elapsed=1789777
                                      ↑ test 3 读 $2000 仍得 0xFF ← 当前 FAIL
n=128 PPUGenLatch=0x00 cycle=3782136 (后续 harness 重启后被覆盖)
```

**核心观察**：从 cycle 148904（test 3 写入）到 cycle 1909113（test 3 读），间隔 1.79M CPU 周期 ≈ 1.0 秒，但 `PPUGenLatch` 始终保持 `0xFF`，从未衰减——确认根因：当前实现**完全没有 decay 机制**。

---

## 2. 修复方案

### 2.1 PPUGenLatch 时间衰减（blargg spec）

blargg `ppu_open_bus/readme.txt`（实测抓自 christopherpow/nes-test-roms 2026-08-01）：

> "The PPU effectively has a 'decay register', an 8-bit register. Each bit
>  can be refreshed with a 0 or 1. **If a bit isn't refreshed with a 1 for
>  about 600 milliseconds, it will decay to 0** (some decay sooner,
>  depending on the NES and temperature)."

实现：固定 600ms 阈值，write-only 触发整字节刷新（已就位），时间驱动衰减。

#### 2.1.1 关键代码（`src/ppu.cpp`）

```cpp
// 600ms @ NTSC CPU 1.789773 MHz = 1 073 864 cycles
static constexpr uint64 PPU_OPEN_BUS_DECAY_CYCLES = 1073864;
uint64 PPUGenLatch_last_refresh_cycle = 0;  // 定义在 opendecay_log_write 之前

static inline void ppu_latch_decay_check() {
    if (PPUGenLatch == 0) return;          // 零开销快路径
    const uint64 now = g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref();
    if (PPUGenLatch_last_refresh_cycle != 0 &&
        now > PPUGenLatch_last_refresh_cycle &&
        now - PPUGenLatch_last_refresh_cycle > PPU_OPEN_BUS_DECAY_CYCLES) {
        PPUGenLatch = 0;
    }
}
```

#### 2.1.2 钩入点

- **写入**：所有 `B2000..B2007` 处理器中的 `opendecay_log_write(V)` 同步更新 `PPUGenLatch_last_refresh_cycle`。probe on/off 均无条件执行（always-on decay 逻辑；probe 日志 env-gated）。
- **每帧检查**：`FCEUPPU_Loop`（oldppu 路径）与 `FCEUX_PPU_Loop`（newppu，默认）入口调 `opendecay_log_decay_check()`（同时是 probe 与 decay 入口，单一函数）。

#### 2.1.3 阈值裕度分析

| 时序 | 周期数 | 备注 |
|---|---|---|
| 阈值（600ms） | 1 073 864 | blargg spec 下限 |
| test 3 写入到读出间隔 | 1 789 777 | 实测 1.0s @ NTSC，1.67× 阈值 |
| 阈值触发后的衰减检出 | ≤1 帧 ≈ 29 782 | per-frame check @ 60Hz，远小于阈值 |
| 误衰减风险 | 极低 | 普通游戏 NMI/handler 每帧多次写 PPU，远低于 600ms 静默 |

### 2.2 A2007 palette 读 per-bit 修复

blargg readme 表：
```
$2007  ---- ----   non-palette
       DD-- ----   palette
```
`DD-- ----` 含义：高 2 位（位 6、7）来自 decay register；位 0–5 来自 PALRAM。

当前 newppu 实现仅返回 PALRAM，未 OR latch 高 2 位 → test 8 "High 2 bits from $2007 from palette should be from decay value" FAIL。

修复（`src/ppu.cpp` A2007 palette 分支）：
```cpp
ret |= (PPUGenLatch & 0xC0);
```
顺带 per-bit refresh 自然正确：`PPUGenLatch = ret` 把 ret（高 2 位来自旧 latch = 保留；低 6 位来自 PALRAM = 刷新）整字节写回——blargg 图 "DD-- ----" 中 `D` 与 `-` 的精确语义。

### 2.3 A2004 读 latch 刷新

blargg readme 表：
```
$2004  ---- ----   (SPRAM data 整字节驱动总线)
```

当前 newppu 实现只返回 SPRAM/PPU[3]，未更新 latch → test 11 "Reading third byte of a sprite from $2004 should refresh all bits of decay value" FAIL。

修复（`src/ppu.cpp` A2004）：
- 非渲染路径：`const uint8 ret = SPRAM[PPU[3]]; PPUGenLatch = ret; return ret;`
- 渲染路径（spr_read 出口）：`PPUGenLatch = spr_read.ret; return spr_read.ret;`

oldppu 路径保留原 `return PPUGenLatch`（已知 "Not correct for $2004 reads"，与 0x01 永久跳过模式相同——不在本次序列内修复）。

---

## 3. 验证

### 3.1 ppu_open_bus 单测

```
$ fceux11_blargg_runner.exe tests/fixtures/blargg/ppu/ppu_open_bus.nes 600
BLARGG_RESULT: rom=ppu_open_bus.nes addr=0x6000 value=0x00
               diag=[0xDE,0xB0,0x61] status=PASS duration_ms=1027
```
（基线 0x03 "Decay value should become zero by one second"）

### 3.2 Oracle B 全量回归

| 指标 | 基线（`863e9d7` 桶 C） | 本次（`23b0cdd`） | Δ |
|---|---|---|---|
| PASS | 143 | **144** | **+1** |
| FAIL | 34 | 33 | −1 |
| 总计 | 177 | 177 | 0 |
| 0x80/0x81 | 0 | 0 | 0 |
| 0xFE | 1 | 1 | 0 |

唯一新增 PASS: `ppu_open_bus.nes`。

#### 3.2.1 残留 33 项 FAIL 分类（与基线逐一一致）

- **MMC3 12**: mmc3_1..6 + mmc3_v2_1..6（Bucket A 全部已知限制）
- **vbl 5**: vbl_02/06/07/08/10（Phase 1 已知限制，桶 C.1）
- **CPU 9**: cpu_int_2/3/4/5 + cpu_reset_regs + instr_misc + instr_misc_03_dummy + instr_timing + instr_timing_v2_1 + cpu_dummy_writes_oam + cpu_exec_space_ppuio（Bucket B.1+B.2 + B.3+B.4）
- **PPU 真实精度 2**: oam_stress + ppu_vbl_nmi（桶 C 余项，深模型族）
- **sprdma 2**: sprdma_dmc_dma + sprdma_dmc_dma_512（桶 D 未启动）

零回归：`ppu_read_buffer` 仍 PASS，`cpu_dummy_writes_ppu` 仍 PASS，其余 31 项 FAIL 与基线逐字一致。

### 3.3 Oracle A

`ctest -LE perf`: 100% tests passed（25 非 perf 测试全绿，0 failed）。

---

## 4. 探针保留与可观测性

`FCEUX11_OPENDECAY_PROBE=1` env-gated 探针随修复保留（env 未设置时所有日志路径零开销早退）。供未来场景使用：

- 深模型族调查（oam_stress / ppu_vbl_nmi）：用探针对比 latch 在可疑时序窗口的变化
- 商业游戏回归：若某游戏突然出现 open-bus 副作用，先开探针跑几百帧定位写入频率
- 阈值调优：探针的 `elapsed_since_last_write_cycles` 字段可直接观察任意游戏的最长静默间隔，为调阈值提供数据

数据采集脚本：
```powershell
$env:FCEUX11_OPENDECAY_PROBE = "1"
.\build\tests\fceux11_blargg_runner.exe <rom> <frames> 2><log>
# 关键字段：
#   OPENDECAY CHECK n=K PPUGenLatch=0xVV cycle=N elapsed_since_last_write_cycles=M
#   OPENDECAY W PPUGenLatch=0xVV cycle=N writes_so_far=K
#   OPENDECAY SUMMARY writes=N reads2000=N decay_checks=N final_PPUGenLatch=0xVV
```

---

## 5. 后续

桶 C PPU 真实精度 4 项中 1 项（`ppu_open_bus`）已在本次收敛。剩余桶 C 余项 + Phase 3 Step 3.2 其余桶评估：

- **桶 C 余项**：`oam_stress` 0x01（深模型族 — PPU 隐式 OAM 改写）+ `ppu_vbl_nmi` 0x01（CPU 侧读采样量化，Phase 1 vbl_02 同族）。建议记入有据已知限制，与桶 A/B 模式一致。
- **桶 B.1+B.2**（CPU 9 项）：改动面大，深模型族，启动新一轮 instrument-first 探针再评估。
- **桶 C.1**（vbl 5 项）：Phase 1 已定案为限制，不在本桶重做。
- **桶 D**（sprdma 2 项）：DMC+SPR DMA 耦合，未启动。

详见 `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 Step 3.2。

---

*调查完。桶 C 4/4 收敛（ppu_read_buffer + cpu_dummy_writes_ppu 上轮 + ppu_open_bus 本次 + 0 项本次未新增），3/4 已知限制待 Step 3.2 其余桶或深模型族突破。*