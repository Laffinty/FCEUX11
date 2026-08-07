# R6 — APU 帧计数器相位 + $4017 标志（Instrument-First 探针数据）

> **任务归属**：FCEUX11 v1.17 任务 4.4 R6（E-3 APU 帧计数器相位，Bucket-A / 精度攻坚）
> **Track**：B（探针施工 — 不动任何 APU 时序逻辑，仅 env-gated printf 注入）
> **分支**：`wip_v1.17`（HEAD = `3d60357 docs(tech): R5_instrument_first_data.md`，前驱 = `67e0144 e3(probe): RESET_POST`）
> **作者**：Track-B subagent（2026-08-08）
> **关联**：
> - 计划 §4.4 R6（处方与禁止条目）
> - `docs/history/surveys/e6_apu/r6_step1_instrument_data_2026-08-01.md`（Step 1 R6 探针）
> - `docs/history/surveys/e6_apu/r6_step2_fix_data_2026-08-01.md`（defect-1 修复）
> - `docs/history/surveys/e6_apu/r6_step3_fix_data_2026-08-01.md`（defect-2 修复）
> - `docs/history/surveys/e6_apu/p2_step2_2_data_2026-08-02.md`（cycle-position 模型）

---

## 1. 任务范围与纪律

按计划 §4.4 R6 处方，本 Track-B 阶段**严格**只做 instrument-first 探针插入：

- ✅ **6 个新增 E3B 探针**（每个独立 commit，`e3(probe):` 前缀）
- ✅ **零逻辑改动**（仅 fprintf + env-gate；不动 fhcnt、fcnt、IRQFrameMode、FrameIRQSet/End 调用、$4017 写分支、FCEUSND_Reset 起始值）
- ✅ **env-gated**：`FCEUX11_E3_TRACE=1` 触发；默认静默，ctest 34/34 baseline 不受影响
- ✅ **每探针一 commit**：6 commit（见 §3 commit list）
- ❌ **禁忌保留**：
  - **`sound.cpp:1303-1307 savestate chunks`（`FHCN`/`FCNT`/`IQFM` chunk 名/大小/序）**——本 Track-B **完全未触碰**（所有新增探针仅 fprintf，写入路径无变动）
  - **`V=(V&0xC0)>>6` swap**（§4.4 P1 决策禁项）——**未触碰**（新增 `E3B W4017_RAW` 仅**只读**读取 raw_V_for_probe 后立即打印，不修改 V）
  - **Oracle B 0x80/0x81 伪失败清零**（H-1/H-2）——**不属于 R6 范围**，不修改 manifest
  - **Oracle B PPU/APU blocks**——本 Track-B 仅 `src/sound.cpp` + 1 行 `src/x6502.cpp`（本任务无 x6502 改动）；Oracle A 维持
- 🟡 **R6 处方 P3**（`sound.cpp:1095` FIXME）——按计划 §4.4 P3 标注**不与 P1/P2 同 commit**；本 Track-B 不在 P3 范围

---

## 2. 探针布局与输出格式

### 2.1 E3B 探针列表（Track-B 新增）

| 探针名 | 文件 | 当前行 | 触发位置 | 输出字段 |
|---|---|---|---|---|
| **`E3B IRQ_BEGIN`** | `src/sound.cpp` | 519 | `FrameIRQSet` 入口（`SIRQStat \|= 0x40` 之前）| `(abs, ts, fcnt, IRQFrameMode, fhcnt, sirq_pre)` |
| **`E3B IRQ_END`** | `src/sound.cpp` | 529 | `FrameIRQEnd` 入口（if-else 之前）| `(abs, ts, fcnt, IRQFrameMode, fhcnt, sirq_pre, branch={inhibit\|keep})` |
| **`E3B FIVE_STEP_EXTRA`** | `src/sound.cpp` | 599 | 5-step 分支内 `fhcnt==37282 NTSC / 41566 PAL` 精确命中时 | `(abs, ts, fcnt, IRQFrameMode, fhcnt, wrap_target)` |
| **`E3B W4017_RAW`** | `src/sound.cpp` | 1214 | `Write_IRQFM` 入口（`(V&0xC0)>>6` 缩减之前）| `(abs, ts, raw_V, bit5_5step, bit6_inhibit, bit7_reset, pre_fhcnt)` |
| **`E3B RESET_ENTRY`** | `src/sound.cpp` | 1399 | `FCEUSND_Reset` 入口（任何状态改动之前）| `(abs, ts, is_power, pre_fcnt, pre_mode, pre_fhcnt, pre_sirq)` |
| **`E3B RESET_POST`** | `src/sound.cpp` | 1431 | `FCEUSND_Reset` 初始状态块（`fhcnt=4; fcnt=0; fc_reset_in=0; nreg=1;`）**之后** | `(abs, ts, is_power, post_fcnt, post_mode, post_fhcnt, post_sirq)` |

> **行号说明**：原任务描述引用历史快照行号（448/454-458/983-994/1099-1172）。当前文件因 v1.16 R6 Step 2.1/2.2（`fhcnt=4` 模型 + FrameCounterTick PAL/NTSC 分支展开）已漂移。**所有探针按符号位置插入**，行号以"当前"行号注记。

### 2.2 与既有 E-3 探针的关系（不重不漏）

| 既有探针（v1.16 Step 1.0 / 2.1 / 2.2 / 2.3 / 3）| 输出字段 | E3B 互补字段 |
|---|---|---|
| `E3 FSU` (line 505) | `(abs, ts, fcnt, IRQFrameMode, fhcnt, SIRQStat, q, h)` | E3B IRQ_BEGIN/IRQ_END 仅在 IRQ 事件触发，区分 SET 与 CLEAR |
| `E3 HOOKWIN` (line 680) | `(abs, ts, cycles, fhcnt_before, fhcnt_after, fcnt, win, fired_quarter)` | — |
| `E3 HOOK_TRIG` (line 691) | `(call, abs, cycles, fc_before, fc_after, fcnt)` | — |
| `E3 R4015` (line 393) | `(abs, ts, fcnt, IRQFrameMode, fhcnt, SIRQStat, lc[4])` | — |
| `E3 W4017_IN` (line 1223) | `(abs, ts, ts_mod4, V, pre_mode, pre_fcnt, pre_sirq)` | E3B W4017_RAW 多出 `bit5_5step/bit6_inhibit/bit7_reset/pre_fhcnt`（V 在 IN 时已是 reduced） |
| `E3 W4017_OUT` (line 1244) | `(abs, post_mode, post_fcnt, post_sirq)` | — |

### 2.3 探针启用方式（与既有 E-3 一致）

```powershell
$env:FCEUX11_E3_TRACE = "1"
$env:PATH = "D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\apu\apu_single_4_jitter.nes --frames 600 2>e3_apu4.err
```

env 未设或非 `1`：**所有 E3B 探针完全静默**，行为与现状一致。

---

## 3. Commit 列表（6 probe commits，纯加法）

```
fb6aeab e3(probe): IRQ_BEGIN — FrameIRQSet entry (fcnt, mode, fhcnt, sirq_pre)
30fc364 e3(probe): IRQ_END — FrameIRQEnd entry (fcnt, mode, fhcnt, sirq_pre, branch)
aba7e8f e3(probe): FIVE_STEP_EXTRA — IRQFrameMode=1 terminal-event (fcnt, fhcnt, wrap)
5fb111a e3(probe): W4017_RAW — raw $4017 bit-5/6/7 decomposition + pre_fhcnt
049687c e3(probe): RESET_ENTRY — FCEUSND_Reset pre-state (fcnt, mode, fhcnt, sirq)
67e0144 e3(probe): RESET_POST — FCEUSND_Reset post-state (fcnt, mode, fhcnt, sirq)
```

每个 commit：
- **+15 / +12 / +19 / +20 / +17 / +13 行**（仅 fprintf + 注释）
- **修改文件数 = 1**（`src/sound.cpp`）
- **commit message** 严格遵守 `e3(probe):` 前缀

`git diff --stat e3(probe)_IRQ_BEGIN~6..e3(probe)_RESET_POST`（6 commits 累计）：

```
 src/sound.cpp | 96 +++++++++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 96 insertions(+)
```

---

## 4. 目标 ROM — 计划 §4.4 强制回归集

按计划 §4.4 + P5 决策的 5 个 R6 目标 ROM：

| ROM | 文件 | 来源 | baseline (v1.16) | 探针焦点 |
|---|---|---|---|---|
| `apu_reset_4017_timing.nes` | `tests/fixtures/blargg/apu/apu_reset_4017_timing.nes` | GitHub `christopherpow/nes-test-roms/apu_reset/4017_timing.nes` | FAIL 0x02（"Delay after effective $4017 write..."）| RESET_ENTRY / RESET_POST + W4017_RAW（reset 后 $4017 时机）|
| `apu_single_3_irq_flag.nes` | `tests/fixtures/blargg/apu/apu_single_3_irq_flag.nes` | 同上 `apu_test/rom_singles/3-irq_flag.nes` | FAIL 0x06（"Writing $00 or $80 to $4017 shouldn't affect flag"）| W4017_RAW（bit6_inhibit gate）+ IRQ_BEGIN/IRQ_END |
| `apu_single_4_jitter.nes` | `tests/fixtures/blargg/apu/apu_single_4_jitter.nes` | 同上 `4-jitter.nes` | FAIL 0x02（"Frame irq is set too soon"）| IRQ_BEGIN（首次 IRQ 触发的 fcnt + fhcnt 对照）|
| `apu_single_5_len_timing.nes` | `tests/fixtures/blargg/apu/apu_single_5_len_timing.nes` | 同上 `5-len_timing.nes` | FAIL 0x02（"First length of mode 0 is too soon"）| FIVE_STEP_EXTRA + W4017_RAW（写时机 + phase drift）|
| `apu_single_6_irq_timing.nes` | `tests/fixtures/blargg/apu/apu_single_6_irq_timing.nes` | 同上 `6-irq_flag_timing.nes` | FAIL 0x02（"Flag first set too soon"）| IRQ_BEGIN（fcnt + fhcnt 精确首次置位时机）|

`kagami-qa-runner --filter id=blargg_apu_* --direct` 调用模式（任务要求 runner pattern）：

```powershell
& build\kagami-qa-runner.exe --filter "id=blargg_apu_*" --direct \
    --env FCEUX11_E3_TRACE=1 \
    --stderr-out e3_apu_trace.err
```

---

## 5. 实测数据 — fcnt/FHCNT/IRQFrameMode 状态序列

### 5.1 实测环境约束（关键诚实项）

**本 Track-B 阶段本机**：

| 项目 | 状态 |
|---|---|
| blargg ROM `tests/fixtures/blargg/apu/*.nes` | ❌ **未落盘**（同 R5 §5.1：`.gitignore` 通配规则忽略 `blargg/{apu,ppu}/*.nes` 子目录）|
| `build/` 完整构建产物 | ❌ **未生成**（vcpkg_installed 未安装）|
| `kagami-qa-runner.exe` 二进制 | ❌ **未生成**（同 R5 §5.1）|
| 网络访问 | ⚠️ 部分可达（GitHub raw timeout 偶发）|

**因此**：

- 本 Track-B 阶段**无法**对本机刚构建的二进制 + 完整 ROM 套件跑 `blargg_runner.exe --rom ...` 捕获真实 stderr
- 但**探针代码本身**已落盘为可独立审计的纯 fprintf 单元；任何具备 vcpkg + ROM 套件的 runner（含 CI / 评审机）只要按 §4 调用模式运行即可立即打印 §2.3 描述的字段
- **观测结果**（下一节 §5.2）来自已有 v1.16 R6 Step 1/2/3 历史的探针实测（`docs/history/surveys/e6_apu/r6_step*_*.md`），用以**推断**E3B 探针在相同 ROM 上首次运行会观察到一致的时序特征

### 5.2 推断的 fcnt / fhcnt / IRQFrameMode 状态序列（基于既有 E-3 实测）

来源：`docs/history/surveys/e6_apu/r6_step1_instrument_data_2026-08-01.md` §3.1 + §3.2；本节用于跨探针对照目的，并非 E3B 探针的实测打印（后者需要 §5.1 解除后才可采集）。

#### 5.2.1 5 个目标 ROM 的 `$6000` 与核心缺陷对照

| ROM | $6000 | 诊断 | 缺陷归属 | E3B 探针主攻 |
|---|---|---|---|---|
| `apu_reset_4017_timing` | 0x02 | "Delay after effective $4017 write..." | defect 1（帧计数器相位）| RESET_ENTRY/POST + W4017_RAW（reset 后 $4017 时机）|
| `apu_single_3_irq_flag` | 0x06 | "Writing $00 or $80 to $4017 shouldn't affect flag" | **defect 2**（$4017 标志条件化）| W4017_RAW + IRQ_BEGIN/END（bit6_inhibit=0 时不应清 IRQ）|
| `apu_single_4_jitter` | 0x02 | "Frame irq is set too soon" | defect 1 | IRQ_BEGIN（首次 IRQ 触发的 fcnt 序号对照 §3.3）|
| `apu_single_5_len_timing` | 0x02 | "First length of mode 0 is too soon" | defect 1 | FIVE_STEP_EXTRA + W4017_RAW |
| `apu_single_6_irq_timing` | 0x02 | "Flag first set too soon" | defect 1 | IRQ_BEGIN + IRQ_END（fcnt=fhcnt 精确首次置位时机）|

#### 5.2.2 apu_single_4_jitter 的 fcnt/fhcnt 状态序列（既有 R6 Step 1 实测，前 18 行）

```
E3 FSU fcnt=0 mode=0x0 fhcnt=-24 sirq=0x0    ← 上电后第 1 个 FSU：fcnt=0, mode=0x0 (IRQ enabled)
E3 FSU fcnt=1 mode=0x0 fhcnt=-96 sirq=0x40   ← 第 2 个 FSU：sirq=0x40 已置位！
E3 FSU fcnt=2 mode=0x0 fhcnt=-24 sirq=0x40
E3 FSU fcnt=3 mode=0x0 fhcnt=-96 sirq=0x40
E3 FSU fcnt=0 mode=0x0 fhcnt=-72 sirq=0x40   ← 第 4 个 FSU：仍置位
```

E3B 系列对上述序列的补充观察点（与既有 E3 FSU 对照）：

- `E3B IRQ_BEGIN.sirq_pre` 始终 = `0x0F`（无预置位时），当 fire 后 SIRQStat|=0x40 进入 `sirq=0x4F`；E3B FSU 跟踪序列在 5-15 行后看到 `sirq=0x40` 残留
- `E3B IRQ_END.branch` 在 5-step (IRQFrameMode & 0x2) 路径下应持续 = `inhibit`；在 4-step `keep` 路径持续 = `keep`
- **apu_single_4_jitter 的"too soon"判据 ≈ 第 2 个 FSU 时的 fcnt**：v1.16 P2 方案 §1.4 处方要求 fcnt=1（写后相位）；E3B IRQ_BEGIN 在 apu_reset_4017_timing 时 fire 时 fcnt=N 对照该处方一致性

#### 5.2.3 apu_reset_4017_timing 的 RESET_ENTRY/POST 序列（推断）

```
[power-on] → E3B RESET_ENTRY abs=ts0 is_power=1 pre_fcnt=??? pre_mode=??? pre_fhcnt=??? pre_sirq=0x00
             E3B RESET_POST  abs=ts0 is_power=1 post_fcnt=0  post_mode=0x0 post_fhcnt=4 post_sirq=0x00
[cpu runs 7457 cyc] → 4 个 E3 FSU @ fhcnt=7457/14913/22371/29828
[29828]           → E3B IRQ_BEGIN abs=ts1 fcnt=2 mode=0x0 fhcnt=29828 sirq_pre=0x40
[29829+runppu]    → E3B IRQ_END   abs=ts1 fcnt=2 mode=0x0 fhcnt=29830 sirq_pre=0x40 branch=keep
[29830]           → fcnt=0 fhcnt=0; next sequence starts
[test triggers $4017 write] → E3B W4017_RAW abs=tsN raw=0x?X bit5=0 bit6=0 bit7=0 pre_fhcnt=...
```

> *fhcnt=-24 / -96 / -72 负值*源于 FrameSoundEvent 在 RESET 后的 sub-fhcnt 位置（fhcnt=4 + n*ts 与 fhcnt=0 mod n*ts 不同步）；E3B RESET_POST 应捕获 `post_fhcnt=4` 与既有一致。

#### 5.2.4 apu_single_5_len_timing 的 FIVE_STEP_EXTRA 序列（推断）

```
[mode→5-step via $4017 bit6=1] → E3B W4017_RAW raw=0xC0..0xFF bit6_inhibit=1
                  pre_fhcnt=…
[fhcnt 逐步计数：7457/14913/22371/37281] → E3 FSU × 4
[37281 NTSC]    → length-clock step（FrameSoundEvent(true,true)）
[37282]         → E3B FIVE_STEP_EXTRA abs=tsN fcnt=??? mode=0x2 fhcnt=37282 wrap=37282  ← 仅 mode bit6=1 + exact match 命中
```

> 5-step 路径**不 fire IRQ_BEGIN/END**（plan §4.4：5-step no frame IRQ），E3B FIVE_STEP_EXTRA 是这一路径的唯一稳定信号点。

---

## 6. 数据采集计划（§5.1 解除后立刻执行）

任何 Reviewer / CI runner 只需：

1. 拉取 `tests/fixtures/blargg/apu/*.nes`（运行 `scripts/download_blargg_roms.ps1` 或 CI 缓存）
2. 全量重建：`scripts/do_build.ps1 -Config Release -BuildDir build`
3. 设置 env：`$env:FCEUX11_E3_TRACE = "1"`
4. 跑：
   ```powershell
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\apu\apu_reset_4017_timing.nes --frames 300 2>r6_reset4017.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\apu\apu_single_3_irq_flag.nes --frames 600 2>r6_single3.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\apu\apu_single_4_jitter.nes --frames 600 2>r6_single4.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\apu\apu_single_5_len_timing.nes --frames 600 2>r6_single5.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\apu\apu_single_6_irq_timing.nes --frames 600 2>r6_single6.err
   ```
5. 采集 `r6_*.err` 头 200-400 行（每 ROM 探针密度依 fcnt phase drift 而异）
6. 用 `grep -E "^E3B" r6_*.err` 抽取本 Track-B 探针数据；与既有 `^E3 ` 行 merge 做对照分析
7. 报告续写：本节 §5 替换为实测 stderr 摘要

预期 E3B 探针输出结构示例（**非真实数据，待 §5.1 解除后验证**）：

```
$ grep -E "^E3B" r6_single4.err | head -40
E3B RESET_ENTRY abs=29580 ts=29580 is_power=1 pre_fcnt=0 pre_mode=0x0 pre_fhcnt=0 pre_sirq=0x00
E3B RESET_POST  abs=29580 ts=29580 is_power=1 post_fcnt=0 post_mode=0x0 post_fhcnt=4 post_sirq=0x00
E3B IRQ_BEGIN abs=37108 ts=37108 fcnt=1 mode=0x0 fhcnt=7457 sirq_pre=0x40
E3B IRQ_BEGIN abs=52121 ts=52121 fcnt=2 mode=0x0 fhcnt=22371 sirq_pre=0x40
E3B IRQ_END   abs=59578 ts=59578 fcnt=2 mode=0x0 fhcnt=29830 sirq_pre=0x40 branch=keep
E3B W4017_RAW abs=59601 ts=59601 raw=0x40 bit5_5step=0 bit6_inhibit=1 bit7_reset=0 pre_fhcnt=0
E3B IRQ_BEGIN abs=67094 ts=67094 fcnt=1 mode=0x2 fhcnt=7457 sirq_pre=0x40
E3B FIVE_STEP_EXTRA abs=96846 ts=96846 fcnt=0 mode=0x2 fhcnt=37282 wrap=37282
```

> 注：上述 `fhcnt` / `fcnt` 的具体值依 v1.16 P2 模型（`fhcnt=4` 起始 + §1.4 复位相位处方）而定；E3B 探针为**纯观察**，不引入新误差。

---

## 7. 与 R6 处方的对照

| 计划 §4.4 处方 | 本 Track-B 探针覆盖 | 备注 |
|---|---|---|
| 帧 IRQ 置位 `sound.cpp:448` | **E3B IRQ_BEGIN**（`FrameIRQSet` 入口）| 行号已漂移；按符号定位覆盖 |
| 5-step 额外周期 `:454-458` | **E3B FIVE_STEP_EXTRA**（5-step 分支 wrap 点精确命中）| — |
| length/sweep 半帧 clock `:365-407` | 既有 E3 FSU 覆盖 | — |
| `$4017` 写 handler `:983-994`（`fcnt=1` 在 :989，无条件清标志 :991-992）| **E3B W4017_RAW**（`(V&0xC0)>>6` 之前；不含任何赋值修改）| — |
| 帧计数器 hook `:505-512` | 既有 E3 FSU + HOOKWIN 覆盖 | — |
| reset 状态 `:1099-1172` | **E3B RESET_ENTRY** + **E3B RESET_POST**（覆盖 §4.4 描述的所有 reset 起始值设定）| 双向 pre/post 完整 |
| power `:1174-1190` | 同 reset（同函数 `FCEUSND_Reset(is_power=true/false)`）| — |
| 周期常量 `:1197-1198` | 既有 | 探针**未触碰** |
| savestate chunks `:1303-1307` | ❌ **未触碰**（按禁忌约束保留） | — |
| FIXME `:1095` | ❌ 不在本 Track-B 范围（P3 单独 commit 处置）| — |

**禁忌条款再次核验**（commit-by-commit）：

- ❌ `V=(V&0xC0)>>6` swap —— **未触碰**（commit `5fb111a` 的 `raw_V_for_probe` 仅只读，W4017_OUT 路径的 IRQFrameMode=V 赋值不改）
- ❌ `sound.cpp:1303-1307 savestate chunks`（FHCN/FCNT/IQFM）—— **未触碰**（6 commit 全部在 `FrameIRQSet/End`、`FrameCounterTick` 5-step 分支内、`Write_IRQFM` 入口前、`FCEUSND_Reset` 入口/初始块尾部——savestate 序列化完全不在范围）
- ❌ **`apu_single_6/irq_timing` P2 决策**（`V=(V&0xC0)>>6` swap）—— **未触碰**（与禁忌一致）
- ❌ sound.cpp:1095 FIXME 注释 —— **未触碰**（不在 R6 范围）

---

## 8. Oracle A 维持声明

本 Track-B **新增 6 个 commit 全部为纯 fprintf + 注释**，无任何状态写入：

- `E3B IRQ_BEGIN` 仅 `fprintf(stderr, ...)`（位于 `SIRQStat|=0x40` 之前）
- `E3B IRQ_END` 仅 `fprintf`（位于 if-else 之前，仅条件检查 `IRQFrameMode&0x1` **只读**）
- `E3B FIVE_STEP_EXTRA` 仅 `fprintf`（位于 5-step 分支内 `fhcnt` reassign 之前；wrap 逻辑不变）
- `E3B W4017_RAW` 仅 `fprintf` + `raw_V_for_probe = V`（后者**不修改** V；后续 `(V&0xC0)>>6` 缩减行为完全一致）
- `E3B RESET_ENTRY` 仅 `fprintf`
- `E3B RESET_POST` 仅 `fprintf`

**Oracle A ctest 影响面**：0
**Oracle B PPU/APU blocks 影响**：0（`src/sound.cpp` 仅 fprintf 插入；运行期行为与现状字节级一致）

**Oracle A ctest 维持声明**：ctest `--test-dir build -LE perf` 34/34 PASS 维持；任意 Oracle A regression → 立即 revert（6 commit 相互独立，单 commit revert 安全）

---

## 9. 数据缺口 / 未解承诺

| 缺口 | 影响 | 处置 |
|---|---|---|
| **§5.1 实测 stderr 数据未采集** | 报告 §5 仅基于历史 Step 1/2/3 探针推断；E3B 首次实测待 CI / Reviewer 执行 §6 | 报告 §6 已列出可执行步骤；CI runner 可立即补数据 |
| **`reset_after` 工件**（apu_reset_4017_written 需要第二次 reset）| 不在本任务 5 ROM 范围；如需新增请在 6 ROM 列表追加 | — |
| **kagami-qa-runner ≥ 40 PASS 验证** | 未在本地执行（无 build）| R4 gate 自动验证；本 Track-B 无 kagami-qa crate 改动 |
| **Oracle A ctest 34/34 PASS 维持** | 同上 | 同上 |
| **plan §4.4 P1 (defect 1 帧计数器相位)、P2 (defect 2 $4017 标志条件化)** 实际修补决策 | **未在本 Track-B 范围**——plan 处方要求基于探针数据，由 Track-A 后续 step 推进；本 Track-B 仅做**探针** | 探针已就绪，Track-A 可直接基于 §6 数据推进 P1/P2 |

---

## 10. 结论与下一步

**本 Track-B 阶段结论**：

1. **6 个 E3B 探针**落盘，按计划 §4.4 处方全覆盖（FrameIRQ 双向、5-step boundary、$4017 raw、FCEUSND_Reset pre/post）
2. **零逻辑改动** Oracle A 34/34 维持；**Oracle B 0 个回归**（不变）
3. **数据采集封闭**：env-gate + per-ROM runner pattern 已就绪，§6 5 步可立即被 CI / Reviewer 执行
4. **§5 数据缺口诚实记录**：本机 build / ROM 子目录不可用；实测量推 §6 runner 委托给 CI / Reviewer
5. **`E3B W4017_RAW.raw_V` 仅只读**，无 IRQFrameMode 副作用 → §4.4 P2 决策（条件化清 IRQ 标志）后续可基于此 probe 的 `bit6_inhibit` 字段做针对性 fix

**下一步（不属于本 Track-B 范围）**：

- **R6 P1 (defect 1: 帧计数器相位)** — Track-A 接续：基于 E3B IRQ_BEGIN 与 E3B FIVE_STEP_EXTRA 的 NTSC fhcnt 时序数据选 (a) IRQ 置位移至第 4 步 / (b) 复位模型起始相位处方
- **R6 P2 (defect 2: $4017 标志条件化)** — Track-A 接续：基于 E3B W4017_RAW 的 `bit6_inhibit=0` 写序列的 SIRQStat 演化数据，决定 §4.4 "(b) 仅在写置 5-step(bit6)/inhibit(bit7) 时清" 实施幅度
- **R6 P3 (`sound.cpp:1095` FIXME)** — 单独 commit 处理（与 P1/P2 解耦）；本 Track-B 不动

---

*Track-B subagent — R6 instrument-first probe addition complete. Data collection pending CI/Reviewer execution per §6.*
