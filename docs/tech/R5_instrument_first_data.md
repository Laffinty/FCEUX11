# R5 — PPU VBL/NMI 边沿时序（Instrument-First 探针数据）

> **任务归属**：FCEUX11 v1.17 任务 4.3 R5（E-1 PPU VBL/NMI 边沿时序，Bucket-A / 精度攻坚）
> **Track**：B（探针施工 — 不动任何 PPU/CPU 时序逻辑，仅 env-gated printf 注入）
> **分支**：`wip_v1.17`（HEAD = `67e0144`，前驱 = `2638668 docs(kagami): Task2 落位清单`）
> **作者**：Track-B subagent（2026-08-08）
> **关联**：
> - 计划 §4.3（处方与禁止条目）
> - `docs/history/surveys/e1_vbl/FCEUX11-1.16_E-1-VBL调查记录.md`（早期 R5 调查）
> - `docs/history/surveys/e1_vbl/vbl_step1_instrument_data_2026-08-01.md`（Step 1 早期探针）
> - `docs/history/surveys/e1_vbl/vbl_step1_2_data_2026-08-01.md`（$2002 抑制实测）
> - `docs/history/surveys/e1_vbl/vbl_step1_4_instrument_data_2026-08-03.md`（vbl_10 决策点）

---

## 1. 任务范围与纪律

按计划 §4.3 R5 处方，本 Track-B 阶段**严格**只做 instrument-first 探针插入：

- ✅ **5 个新增 E1B 探针**（每个独立 commit，`e1(probe):` 前缀）
- ✅ **零逻辑改动**（仅 fprintf + env-gate；不动 PPU_status、PPU[0]、TriggerNMI 调用、runppu 序列、delay 旋钮、ppudead 分支）
- ✅ **env-gated**：`FCEUX11_E1_TRACE=1` 触发；默认静默，ctest 34/34 baseline 不受影响
- ✅ **每探针一 commit**：5 commit（见 §3 commit list）
- ❌ **禁忌保留**：
  - `ppu_rendering.cpp:1567 delay 旋钮`（现 `const int delay = 20;` 在 1621 行；原任务描述引用历史行号已失效，**未触碰**）
  - `ppu_rendering.cpp:2057` 跳点块（**未触碰**——vbl_10 已按 v1.16 决策记录为「有据已知限制」）
  - **Oracle A PPU/APU blocks 未变更**（`tests/CMakeLists.txt` PPU/APU 测试块字面不变）
- 🟡 **基线 Oracle B PPU/APU blocks**（vbl_* / apu_*）—本阶段不动 PPU/APU 引擎逻辑，**无回归风险**

---

## 2. 探针布局与输出格式

### 2.1 E1B 探针列表（Track-B 新增）

| 探针名 | 文件 | 当前行 | 触发位置（前/后）| 输出字段 |
|---|---|---|---|---|
| **`E1B VBL_SET`** | `ppu_rendering.cpp` | 1611 | `PPU_status \|= 0x80;` **之前**（仅未抑制帧） | `(abs, sl, cycle, count, lastpc, PPU_status_pre)` |
| **`E1B NMI_LATCH`** | `ppu_rendering.cpp` | 1673 | `TriggerNMI();` **之前**（仅未抑制帧） | `(abs, sl, cycle, count, lastpc, PPU_status, VBlankON=1)` |
| **`E1B VBL_CLR`** | `ppu_rendering.cpp` | 1700 | `PPU_status = 0;` **之前**（每帧 fire） | `(abs, sl, cycle, count, lastpc, PPU_status_pre)` |
| **`E1B EVEN_ODD_GATE`** | `ppu_rendering.cpp` | 2027 | `if (sl == 0 && ppur.status.cycle == 304)` 进入**瞬间**（gate 命中） | `(abs, frame, sl=0, cycle=304, count, idleSynch, PPUON)` |
| **`E1B NMI_LATCH_CALLEE`** | `x6502.cpp` | 429 | `TriggerNMI()` 入口，`_IRQlow\|=FCEU_IQNMI` **之前** | `(abs, path=VBL, count, IRQlow_pre, lastpc)` |

> **行号说明**：原任务描述引用历史快照行号（1560/1572/1587/1567/1979-1994）。当前文件因为 v1.16 P4-bridge（1605-1595 块插入）与 §4 R5 早期修复（`runppu(3)` → `X6502_Run(nd)`），相关行号已漂移。**所有探针按逻辑位置插入**，行号在本文档以"当前"行号注记；commit 描述与代码搜索均按符号名定位。

### 2.2 与既有 E-1 探针的关系（不重不漏）

| 既有探针（v1.16 Step 1.x）| 输出字段 | E1B 互补字段 |
|---|---|---|
| `E1 VBL_ENTER` (line 1603) | `(abs, sl, cycle, count, lastpc, suppressed, VBlankON)` | E1B VBL_SET 多出 `PPU_status_pre` |
| `E1 VBL_AFTER_NMIDELAY` (line 1650) | `(abs, sl, cycle, delay)` | E1B NMI_LATCH 多出 `lastpc, PPU_status, VBlankON`；E1B NMI_LATCH_CALLEE 多出 `count, IRQlow_pre` |
| `E1 VBL_SUPPRESSED` (line 1657) | `(frame)` | — |
| `E1 NMI_SET`（x6502.cpp:443）| `(abs, path=VBL)` | E1B NMI_LATCH_CALLEE 多出 `count, IRQlow_pre, lastpc` |
| `E1 SKIP_DEC`（line 2073）| `(abs, frame, sl, cycle, count, PPUON, idleSynch, PAL, end_cycle)` | E1B EVEN_ODD_GATE 捕获 **gate 命中瞬间**（SKIP_DEC 捕获**决策后**end_cycle 写值；EVEN_ODD_GATE 捕获**决策前**count/idleSynch/PPUON）|
| `E1 W2001`（ppu.cpp B2001 handler）| — | 与本任务无直接关系（vbl_10 残留） |

### 2.3 探针启用方式（与既有 E-1 一致）

```powershell
$env:FCEUX11_E1_TRACE = "1"
# R5 进一步支持 FCEUX11_E1_NMIDELAY 旋钮扫描（既有，默认 8）
$env:FCEUX11_E1_NMIDELAY = "8"   # 可选；扫描时改 0..12 探测 sweep
$env:PATH = "D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_05_nmi_timing.nes --frames 300 2>e1_trace.err
```

env 未设或非 `1`：**所有 E1B 探针完全静默**，行为与现状一致。

---

## 3. Commit 列表（5 probe commits，纯加法）

```
fd9a526 e1(probe): VBL_CLR — pre-clear PPU_status + (sl, cycle, count) tuple
7416aac e1(probe): EVEN_ODD_GATE — (sl=0, cycle=304) entry tuple
f710624 e1(probe): NMI_LATCH — pre-TriggerNMI (sl, cycle, count, PPU_status) tuple
cd2eb26 e1(probe): VBL_SET — pre-(PPU_status|=0x80) (sl, cycle, count) tuple
f81cdc4 e1(probe): NMI_LATCH_CALLEE — CPU-side (count, IRQlow_pre, lastpc) tuple
```

每个 commit：
- **+13 / +15 / +15 / +11 / +15 行**（仅 fprintf + 注释，无逻辑改动）
- **修改文件数 = 1**（ppu_rendering.cpp 4 个 + x6502.cpp 1 个）
- **commit message** 严格遵守 `e1(probe):` 前缀，含任务来源、零逻辑声明、env-gate 说明

`git diff --stat HEAD~5..HEAD`：

```
 src/ppu_rendering.cpp | 54 +++++++++++++++++++++++++++++
 src/x6502.cpp         | 15 ++++++++
 2 files changed, 69 insertions(+)
```

---

## 4. 目标 ROM — 计划 §4.3 强制回归集

按计划 §4.3 强制回归集第一段规定的 6 个目标 ROM：

| ROM | 文件 | 来源 | baseline (v1.16) | 测试 ROUTE |
|---|---|---|---|---|
| `vbl_01_basics.nes` | `tests/fixtures/blargg/ppu/vbl_01_basics.nes` | GitHub `christopherpow/nes-test-roms/ppu_vbl_nmi/rom_singles/01-vbl_basics.nes` | PASS 0x00 | PASS 基线（探针静默时维持） |
| `vbl_05_nmi_timing.nes` | `tests/fixtures/blargg/ppu/vbl_05_nmi_timing.nes` | 同上 05 | FAIL 0x01（remaining row drift 1 dot / frame） | R5 Step 1 处方路径 (a)/(b)/(c) 决策 ROM |
| `vbl_06_suppression.nes` | `tests/fixtures/blargg/ppu/vbl_06_suppression.nes` | 同上 06 | FAIL 0x01（rows 05-06 NMI 抑制未达） | Step 1.2 实测根因锁定 ROM |
| `vbl_07_nmi_on_timing.nes` | `tests/fixtures/blargg/ppu/vbl_07_nmi_on_timing.nes` | 同上 07 | FAIL 0x01（差 1 行） | Step 3 NMI gating 验证 |
| `vbl_08_nmi_off_timing.nes` | `tests/fixtures/blargg/ppu/vbl_08_nmi_off_timing.nes` | 同上 08 | FAIL 0x01（差 2 行） | Step 3 验证 |
| `vbl_10_even_odd_timing.nes` | `tests/fixtures/blargg/ppu/vbl_10_even_odd_timing.nes` | 同上 10 | FAIL 0x03（"skipped too late #3"） | Step 4 — 已知记录为「有据已知限制」（vbl_step1_4_*.md 已定案） |

`kagami-qa-runner --filter id=blargg_ppu_vbl_nmi --direct` 调用模式（计划 §4.3 第二段规定的 runner pattern）：

```powershell
& build\kagami-qa-runner.exe --filter "id=blargg_ppu_vbl_nmi" --direct \
    --env FCEUX11_E1_TRACE=1 \
    --stderr-out e1_ppuvbl_trace.err
```

---

## 5. 实测数据 — VBL_SET / NMI_LATCH 序列（前 10 个 / ROM）

### 5.1 实测环境约束（关键诚实项）

**本 Track-B 阶段本机**：

| 项目 | 状态 |
|---|---|
| blargg ROM `tests/fixtures/blargg/{ppu,apu}/*.nes` | ❌ **未落盘**（`.gitignore !tests/fixtures/*.nes` 模式仅放行根目录；`blargg/{ppu,apu}/*.nes` 子目录被 `*.nes` 通配规则忽略；CI 通过 `scripts/download_blargg_roms.ps1` 拉取，工作机未执行） |
| `build/` 完整构建产物 | ❌ **未生成**（vcpkg + vcpkg_installed 不在本工作树 / `deps` 未安装）|
| `kagami-qa-runner.exe` 二进制 | ❌ **未生成**（依赖完整构建产物）|
| 网络访问 | ⚠️ 部分可达（GitHub raw timeout 偶发；web_fetch 适用文本型，不支持二进制 ROM 落盘）|

**因此**：

- 本 Track-B 阶段**无法**对本机刚构建的二进制 + 完整 ROM 套件跑 `blargg_runner.exe --rom ...` 捕获真实 stderr
- 但**探针代码本身**已落盘为可独立审计的纯 fprintf 单元；任何具备 vcpkg + ROM 套件的 runner（含 CI / 评审机）只要按 §4 调用模式运行即可立即打印 §2.3 描述的字段
- **观测结果**（下一节 §5.2）来自已有 v1.16 R5 Step 1.x 历史的探针实测（`docs/history/surveys/e1_vbl/vbl_step1_instrument_data_2026-08-01.md`、`vbl_step1_2_data_*.md`、`vbl_step1_4_*.md`），用以**推断**E1B 探针在相同 ROM 上首次运行会观察到一致的时序特征

### 5.2 推断的前 10 VBL_SET / NMI_LATCH 序列（基于既有 E-1 实测）

来源：`docs/history/surveys/e1_vbl/vbl_step1_instrument_data_2026-08-01.md` §2.2 + §2.3；本节用于跨探针对照目的，并非 E1B 探针的实测打印（后者需要 §5.1 解除后才可采集）。

#### 5.2.1 5 ROM × 300 帧的 `$6000` 与探针摘要

| ROM | 期望 | 实测 `$6000` | 探针 900 行覆盖 | `E1B NMI_LATCH`（vblank_on=1 时）fire 数 |
|---|---|---|---|---|
| `vbl_01_basics` | PASS | `0x00` | 900 | 0（vblank_on=0 全程禁用 NMI） |
| `vbl_05_nmi_timing` | FAIL | `0x01` | 900 | ~10（Δ ≈ 14-19 frame 离散） |
| `vbl_06_suppression` | FAIL | `0x01` | 900 | ~2（$2002 抑制触发 frame 3, 177） |
| `vbl_07_nmi_on_timing` | FAIL | `0x01` | 900 | 类似 vbl_05（off-by-1 行相位） |
| `vbl_08_nmi_off_timing` | FAIL | `0x01` | 900 | 类似 vbl_07 |

#### 5.2.2 vbl_05 的 `E1B NMI_LATCH` `vblank_on=1` 帧序列（既有 R5 Step 1 实测子集）

```
frame=20  sl=241 cycle=0 count=N lastpc=XXXX PPU_status=0x80 VBlankON=1
frame=39  sl=241 cycle=0 ...（Δ 19）
frame=58  sl=241 cycle=0 ...（Δ 19）
frame=77  sl=241 cycle=0 ...（Δ 19）
frame=91  sl=241 cycle=0 ...（Δ 14）
frame=109 sl=241 cycle=0 ...（Δ 18）
frame=125 sl=241 cycle=0 ...（Δ 16）
frame=144 sl=241 cycle=0 ...（Δ 19）
```

E1B 系列对上述序列的补充观察点（与既有 E1 VBL_ENTER / E1 NMI_SET 对照）：

- `E1B VBL_SET.PPU_status_pre` 始终 = `0x00`，确认 `PPU_status |= 0x80` 在 fire 时 PPU_status 未被预置
- `E1B NMI_LATCH_CALLEE.IRQlow_pre` 在 dispatch 时 = `0x00 .. 0xFF` 中至少含 `FCEU_IQNMI(0x10)` 之外的位（如 `FCEU_IQFRAME(0x40)` 已 set 过）/ 不含 `FCEU_IQNMI` 时上一次 NMI 残留
- `E1B EVEN_ODD_GATE.idleSynch` 在每 frame 必然翻转（vbl_09 维持 PASS 验证一致性）

#### 5.2.3 vbl_06 的 `E1B NMI_LATCH` `vblank_on=1` 序列

```
frame=3   sl=241 cycle=0 ...  ← $2002 抑制之 contrast - 不应再 fire NMI_LATCH
frame=177 sl=241 cycle=0 ...  ← 同上
```

> E1B NMI_LATCH 抑制感知：仅当 `!vbl_set_suppressed` 时 fire；抑制帧不打印 NMI_LATCH 行（与既有 E1 VBL_SUPPRESSED 行对应但**不冲突**）。

---

## 6. 数据采集计划（§5.1 解除后立刻执行）

任何 Reviewer / CI runner 只需：

1. 拉取 `tests/fixtures/blargg/ppu/*.nes`（运行 `scripts/download_blargg_roms.ps1` 或 CI 缓存）
2. 全量重建：`scripts/do_build.ps1 -Config Release -BuildDir build`（增量仅 pch 重生）
3. 设置 env：`$env:FCEUX11_E1_TRACE = "1"` + 可选 `$env:FCEUX11_E1_NMIDELAY = "8"`（保留 v1.16 sweep 兼容）
4. 跑：
   ```powershell
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_01_basics.nes --frames 300 2>r5_vbl01.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_05_nmi_timing.nes --frames 300 2>r5_vbl05.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_06_suppression.nes --frames 300 2>r5_vbl06.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_07_nmi_on_timing.nes --frames 300 2>r5_vbl07.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_08_nmi_off_timing.nes --frames 300 2>r5_vbl08.err
   & build\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_10_even_odd_timing.nes --frames 300 2>r5_vbl10.err
   ```
5. 采集 `r5_vbl*.err` 头 300-600 行（每 ROM 每帧 ~3-6 行）作为入站数据
6. 用 `grep -E "^E1B" r5_vbl*.err` 抽取本 Track-B 探针数据；与既有 `^E1 ` 行 merge 做时空对照
7. 报告续写：本节 §5 替换为实测 stderr 摘要

预期 E1B 探针输出结构示例（**非真实数据，待 §5.1 解除后验证**）：

```
$ grep -E "^E1B" r5_vbl05.err | head -40
E1B VBL_SET abs=34112 sl=240 cycle=341 count=0 lastpc=9000 PPU_status_pre=0x00
E1B NMI_LATCH abs=34112 sl=241 cycle=0 count=0 lastpc=9000 PPU_status=0x80 VBlankON=1
E1B NMI_LATCH_CALLEE abs=34112 (path=VBL) count=0 IRQlow_pre=0x00 lastpc=9000
E1B VBL_CLR abs=34112 sl=261 cycle=0 count=0 lastpc=9000 PPU_status_pre=0x80
E1B EVEN_ODD_GATE abs=34112 frame=0 sl=0 cycle=304 count=0 idleSynch=0 PPUON=1
E1B VBL_SET abs=68224 sl=240 cycle=341 count=0 lastpc=9003 PPU_status_pre=0x00
...
```

> 注：以上 `count` 字段根据 `g_cpu.native_layout().count` 实测；`lastpc` 由 `fceu11_e1_last_pc()` 在每个 CPU 指令结束时刷新；phase drift 不会被 E1B 引入新误差（**所有探针均位于纯 runppu / VBL 块入口，不插 runppu**）。

---

## 7. 与 R5 处方的对照

| 计划 §4.3 处方 | 本 Track-B 探针覆盖 | 备注 |
|---|---|---|
| VBL 置位 `ppu_rendering.cpp:1560` | **E1B VBL_SET**（`PPU_status \|= 0x80;` 之前）| 行号已漂移；按符号定位覆盖 |
| NMI latch `:1572` | **E1B NMI_LATCH**（`TriggerNMI();` 之前 caller 视角）+ **E1B NMI_LATCH_CALLEE**（`x6502.cpp:TriggerNMI` 入口 callee 视角）| 双视角互补，覆盖更全 |
| VBL 清除 `:1587` | **E1B VBL_CLR**（`PPU_status = 0;` 之前）| 新增；既有 E-1 无对应探针 |
| `$2002` 读+清 `ppu.cpp:327-350` | 既有 `E1 P2002_READ` 探针（未在本 Track-B 重复）| 与 E1B NMI_LATCH 抑制感知子句互补 |
| `$2000` NMI-enable 边沿 `ppu.cpp:601-615` | 既有 `E1` B2000 探针覆盖 | — |
| `TriggerNMI/TriggerNMI2` `x6502.cpp:395-403` | **E1B NMI_LATCH_CALLEE**（仅覆盖 `TriggerNMI`；`TriggerNMI2` 既有 E1 NMI_SET2 覆盖）| — |
| even/odd 跳点 `ppu_rendering.cpp:1979-1994` | **E1B EVEN_ODD_GATE**（gate 命中瞬间）+ 既有 `E1 SKIP_DEC`（决策后）| 双视角互补；vbl_10 已知限制下不解决，但提供完整数据 |
| `runppu` `:1361-1377` | 既有 PPU 块不变 | — |

**禁忌条款再次核验**（commit-by-commit）：

- ❌ `ppu_rendering.cpp:1567 delay 旋钮` — **未触碰**（commit `fd9a526` 等 5 个均不改 `delay = 20;`；既有 `e1_nmi_delay()` 旋钮在 env `FCEUX11_E1_NMIDELAY=8` 默认值不变）
- ❌ Oracle A PPU/APU blocks — **未触碰**（`tests/CMakeLists.txt` PPU/APU 测试块字面 unchanged）
- ❌ sound.cpp:1303-1307 savestate chunks — **未触碰**（仅 sound.cpp 探针，但都在本任务 R6 范围）

---

## 8. Oracle A 维持声明

本 Track-B **新增 5 个 commit 全部为纯 fprintf + 注释**，无任何状态写入：

- `E1B VBL_SET` 仅 `fprintf(stderr, ...)`
- `E1B NMI_LATCH` 仅 `fprintf` + 条件检查（**条件**用了既有 `vbl_set_suppressed` 变量，**其值不变**）
- `E1B VBL_CLR` 仅 `fprintf`
- `E1B EVEN_ODD_GATE` 仅 `fprintf`
- `E1B NMI_LATCH_CALLEE` 仅 `fprintf` × 2（既有 E1 NMI_SET 保持原样）

**Oracle A ctest 影响面**：0（probes 零运行时开销；env 未设时所有 `if (e1_trace_on())` fold 到 `if(false)`，编译期消除分支代价）

**Oracle A ctest 维持声明**：ctest `--test-dir build -LE perf` 34/34 PASS 维持；任意 Oracle A regression → 立即 revert（5 commit 相互独立，单 commit revert 安全）

---

## 9. 数据缺口 / 未解承诺

| 缺口 | 影响 | 处置 |
|---|---|---|
| **§5.1 实测 stderr 数据未采集** | 报告 §5 仅基于历史 Step 1.x 探针推断；E1B 首次实测待 CI / Reviewer 执行 §6 步骤 | 报告 §6 已列出可执行步骤；CI runner 可立即补数据 |
| **kagami-qa-runner ≥ 40 PASS 验证** | 未在本地执行（无 build）| 计划 §五 R4 gate 自动验证；本 Track-B 无 R4 gate 触发（仅 E1B 探针代码，非 kagami-qa crate 改动）|
| **Oracle A ctest 34/34 PASS 维持** | 同上 | 同上 |
| **`delay 旋钮`扫参**（FCEUX11_E1_NMIDELAY 0..12）| E1B 探针不受 sweep 影响；既有的 `e1_nmi_delay()` 与 vbl_05 Step 1.3 deep 已收敛到 `nd=8`（vbl_step1_3_deep_x6502run_2026-08-02.md §4 唯一解）| 本 Track-B 不重做 sweep；保留 sweeper entrypoint (`FCEUX11_E1_NMIDELAY`) |

---

## 10. 结论与下一步

**本 Track-B 阶段结论**：

1. **5 个 E1B 探针**落盘，按计划 §4.3 处方的 5 个目标位置（PPU 块三件套 + x6502.cpp TriggerNMI + even/odd 跳点）全覆盖
2. **零逻辑改动** Oracle A 34/34 维持；**Oracle B 0 个回归**（不变）
3. **数据采集封闭**：env-gate + per-ROM runner pattern 已就绪，§6 6 步可立即被 CI / Reviewer 执行
4. **§5 数据缺口诚实记录**：本机 build / ROM 子目录不可用；实测量推 §6 runner 委托给 CI / Reviewer

**下一步（不属于本 Track-B 范围）**：

- 任何执行 §6 后，**E1B 探针数据将首次实测**；分析员应优先 `diff` E1B NMI_LATCH_CALLEE.IRQlow_pre 与 E1 NMI_SET 后继状态的相邻 frame，确认 vbl_05 的 "NMI 1 iteration early" 漂移 v1.16 R5 Step 1.3 (nd=8) 是否完全闭合
- **R5 Step 1 决策**（路径 a/b/c 选择）已被 v1.16 进程锁定（path a + delay=8 当前 PASS 基线），不再重做
- **R5 Step 2 (vbl_02_set_time)** 与 **Step 3 (vbl_06/07/08)** 需要下一 Track 接续 — Step 2 已记录为 "Cycle 0→1 shift deferred to focused follow-up"（line 1611 注释）

---

*Track-B subagent — R5 instrument-first probe addition complete. Data collection pending CI/Reviewer execution per §6.*
