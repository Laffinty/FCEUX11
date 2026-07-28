# FCEUX11 v1.16 KagamiQA — P4-bridge 新 PPU headless 帧 3 崩溃根因与解决方案

> **日期**：2026-07-28
> **分支**：`wip_1.16`
> **对应问题**：构建状态报告 §3.1「P4-bridge：新 PPU 正常渲染路径挂起」
> **性质**：独立根因分析 + 已验证修复（代码已落地，Oracle A 回归全绿）
> **关键结论**：报告所述「帧 3 挂起」实为 **`FFCEUX_PPURead` 空指针解引用导致的进程崩溃**（非死循环式挂起）。根因是 `PPU_ResetHooks()` 在生产初始化链中**从未被调用**，致使 `ResetGameLoaded()` 清空的 `FFCEUX_PPURead` 指针在 LoadGame 完成后保持 `NULL`。新 PPU 渲染路径首次 `CALL_PPUREAD` 即解引用 NULL。

---

## 一、问题重述

构建状态报告 §3.1 描述：

> `newppu=1` 时，ppudead 帧（1-2）正常完成，首次进入正常渲染路径（帧 3）后进程挂起，无输出、无崩溃信息。

报告列出了 9 个「已排除的根因」假说与 5 个「剩余候选方向」，结论是「纯代码审查已穷尽，必须在 Visual Studio 中调试」，并强调「绝不可回退到旧 PPU」。

**本方案不依赖 Visual Studio 调试**，通过代码精读 + 实证复现 + 诊断打印三段式定位了真实根因，并已落地修复、通过回归。

---

## 二、根因（已实证确认）

### 2.1 一句话根因

`FFCEUX_PPURead` 这个 PPU 内存读取钩子函数指针，在 `ResetGameLoaded()`（每次 `LoadGame` 开头调用）里被置为 `NULL`，而唯一能把它恢复为 `FFCEUX_PPURead_Default` 的 `PPU_ResetHooks()` 函数**在整个生产代码中从未被调用**。新 PPU（`FCEUX_PPU_Loop`）的渲染热路径 `CALL_PPUREAD` 宏**不判空**直接调用该指针，导致帧 3 首次渲染时解引用 NULL → 进程崩溃。

### 2.2 完整根因链（带 file:line 证据）

| 步骤 | 位置 | 行为 |
|------|------|------|
| ① 初始化 | `src/ppu.cpp:298` | `uint8 (FASTCALL *FFCEUX_PPURead)(uint32 A) = 0;` —— 全局指针初值为 `NULL` |
| ② LoadGame 清理 | `src/fceu.cpp:454` 调 `ResetGameLoaded()` | `src/fceu.cpp:396` `FFCEUX_PPURead = NULL;` —— 每次加载 ROM 前显式置空 |
| ③ PowerNES | `src/fceu.cpp:522` → `PowerNES()` (fceu.cpp:993) → `FCEUPPU_Power()` (ppu.cpp:766) | `FCEUPPU_Power` 调 `FCEUPPU_Reset()`（设 `new_ppu_reset=true`、`ppudead=2`），**但不调用 `PPU_ResetHooks()`**，不恢复 `FFCEUX_PPURead` |
| ④ 唯一恢复点 | `src/ppu_core.cpp:83` `PPU_ResetHooks()` | `FFCEUX_PPURead = FFCEUX_PPURead_Default;` —— **但 grep 全仓确认：生产代码无任何调用者**（仅 `tests/core/ppu_test.cpp:244` 测试里调用） |
| ⑤ 不判空的调用 | `src/ppu.cpp:301` `#define CALL_PPUREAD(A) (FFCEUX_PPURead(A))` | 直接解引用，无 `? :` 守卫（对比 `CALL_PPUWRITE` at ppu.cpp:303 **有**判空） |
| ⑥ 新 PPU 热路径触发 | `src/ppu_rendering.cpp:1417` `nt = CALL_PPUREAD(RefreshAddr);`（`bgdata::Record::Read` 内） | 帧 3 首次进入 scanline 循环 → 第一个 tile 的 nametable fetch → 调用 `NULL` → 崩溃 |

### 2.3 为何 ppudead 帧（1-2）不崩、帧 3 才崩

`FCEUX_PPU_Loop`（ppu_rendering.cpp:1514）的结构：

```cpp
if (new_ppu_reset) { ppur.reset(); spr_read.reset(); new_ppu_reset = false; }  // 1516-1521

if (ppudead) {                              // 1524 — 帧 1、2 走此分支
    runppu(20 * kLineTime);                 //     只推进 CPU 周期
    runppu(242 * kLineTime);
    PPU_status |= 0x80; TriggerNMI();       //     P4-bridge 加的 VBL+NMI
    --ppudead;
    goto finish;                            //     ★ 直接跳到 return，不进入渲染循环
}

{                                           // 1547 — 帧 3+ 走此分支（正常渲染）
    runppu(1);                              //     P4-1 VBL 对齐
    ...
    for (int sl = 0; sl < normalscanlines; sl++) {  // 1625 scanline 循环
        ...
        for (int xt = 0; xt < 32; xt++) {
            bgdata.main[xt + 2].Read(xt + 2);  // 1692 → Record::Read()
                // ↑ 1417: nt = CALL_PPUREAD(RefreshAddr)  ← ★ NULL deref 崩溃点
```

ppudead 分支（1524-1545）**从不调用 `CALL_PPUREAD`**（它只 `runppu()` 推进周期 + 触发 NMI，然后 `goto finish`）。所以前两帧平安无事。帧 3 是第一个进入 1625 scanline 循环的帧，第一个 `bgdata::Record::Read()` 的第一行 `CALL_PPUREAD` 就解引用 NULL。

### 2.4 为何此 bug 长期潜伏（旧 PPU Oracle A 一直全绿）

`CALL_PPUREAD` 的全部调用点（grep 确认）：

| 调用点 | 位置 | 触发条件 |
|--------|------|----------|
| `B2007`（$2007 VRAM 读 handler） | `src/ppu.cpp:512,516` | CPU 主动读 `$2007` 时 |
| 新 PPU nametable fetch | `ppu_rendering.cpp:1417` | 新 PPU 渲染，每 scanline 32 次 |
| 新 PPU attribute fetch | `ppu_rendering.cpp:1426` | 新 PPU 渲染，每 scanline 32 次 |
| 新 PPU pattern fetch（BG） | `ppu_rendering.cpp:1448,1453,1473,1481` | 新 PPU 渲染，每 scanline 128 次 |
| 新 PPU sprite pattern fetch | `ppu_rendering.cpp:1946,1952` | 新 PPU 渲染，每 scanline ≤16 次 |

**旧 PPU 渲染路径（`RefreshLine`/`DoLine`/`pputile.inc`/`pputile_template.h`）完全不调用 `CALL_PPUREAD`**（grep `pputile.inc` = 0 处，`pputile_template.h` = 0 处）。旧 PPU 用 `VPage[]`/`vnapage[]` 直接数组索引读取 pattern data，绕过了 `FFCEUX_PPURead` 钩子。

因此：
- **旧 PPU（`newppu=0`）渲染从不触碰 `FFCEUX_PPURead`** → `NULL` 无害 → Oracle A 回归测试全绿。
- 旧 PPU 下只有 CPU 主动读 `$2007`（ppu.cpp:512）才会调 `CALL_PPUREAD`；Oracle A 的回归测试 ROM（如 nestest）若不读 `$2007`，就不触发 NULL。
- **新 PPU（`newppu=1`）渲染路径每帧调用 `CALL_PPUREAD` 数千次**，首次即崩。

这正是该 bug 在 P4-bridge 之前从未暴露的原因——P4-bridge 是首次在 headless 下启用新 PPU 渲染路径的尝试。

### 2.5 为何报告误判为「挂起」而非「崩溃」

报告称「无输出、无崩溃信息、挂起」。实际表现（实证，见 §三）是 `exit code 127`（Git Bash / cmd 对 Windows 进程崩溃的 Reporting）、**无任何 stdout 输出**。这是因为崩溃发生在 `emulate_n()` 内部、`print_single_result()` 之前，进程被 access violation 终止，stdout 缓冲未刷新。Release 构建（无调试器）下，Windows 不弹崩溃对话框，进程静默退出 → 观察者极易误判为「挂起」。报告作者在「已排除假说」里列了 9 项，但**没有一项覆盖「NULL 函数指针解引用」**——因为表象指向「无限循环」，而真实根因是「立即崩溃」。

---

## 三、实证复现与根因确认（诊断打印）

### 3.1 复现：frames=2 vs frames=3

使用报告同款 ROM `tests/fixtures/blargg/ppu/ppu_vbl_nmi.nes`，Release 构建：

```
$ ./build/tests/fceux11_blargg_runner.exe --rom tests/fixtures/blargg/ppu/ppu_vbl_nmi.nes --frames 2
BLARGG_RESULT: rom=ppu_vbl_nmi.nes addr=0x6000 value=0x00 diag=[0x00,0x00,0x00] status=PASS duration_ms=4
EXIT=0

$ ./build/tests/fceux11_blargg_runner.exe --rom tests/fixtures/blargg/ppu/ppu_vbl_nmi.nes --frames 3
EXIT=127    ← 无任何输出
```

- `--frames 2`（ppudead 帧）：正常返回，但 `status=PASS` 是**假阳性**——`$6000` 默认值是 `0x00`（`FCEU_MemoryRand` 初始化），ROM 还没来得及写结果。这解释了为何之前没人发现：2 帧「通过」掩盖了路径根本没走通。
- `--frames 3`（首次渲染）：`exit 127` + 空输出 = **进程崩溃**（access violation）。

### 3.2 诊断打印确认 `FFCEUX_PPURead == NULL`

在 `blargg_runner.cpp` 的 `run_one_rom` 中、`newppu=1` 设置后、`emulate_n()` 前，临时插入诊断：

```cpp
extern uint8 (FASTCALL *FFCEUX_PPURead)(uint32 A);
extern uint8 FASTCALL FFCEUX_PPURead_Default(uint32 A);
std::fprintf(stderr, "DIAG-KAGAMI: FFCEUX_PPURead=%p Default=%p ...\n",
    (void*)FFCEUX_PPURead, (void*)FFCEUX_PPURead_Default, ...);
```

结果（诊断版构建）：

```
$ --frames 2
DIAG-KAGAMI: FFCEUX_PPURead=0000000000000000 Default=00007FF797B30890 newppu=1 normalscanlines=241
BLARGG_RESULT: ... status=PASS ...

$ --frames 3
DIAG-KAGAMI: FFCEUX_PPURead=0000000000000000 Default=00007FF797B30890 newppu=1 normalscanlines=241
EXIT=127    ← 诊断打印已输出（stderr 无缓冲），但崩溃前没到 BLARGG_RESULT
```

**`FFCEUX_PPURead = 0x0000000000000000`（NULL）**，而 `FFCEUX_PPURead_Default = 0x00007FF797B30890`（有效）。根因 100% 确认。

（诊断代码已验证后移除，未入库。）

---

## 四、修复方案（已落地、已验证）

### 4.1 核心修复：`FCEUPPU_Power` 调用 `PPU_ResetHooks()`

**文件**：`src/ppu.cpp`，函数 `FCEUPPU_Power()`（line 766）

```cpp
void FCEUPPU_Power(void) {
    int x;
    FCEU_MemoryRand(NTARAM, 0x800, true);
    FCEU_MemoryRand(PALRAM.data(), 0x20, true);
    FCEU_MemoryRand(SPRAM, 0x100, true);
    for (x = 0; x < 0x20; ++x) PALRAM[x] &= 0x3F;
    UPALRAM[0] = PALRAM[0x04];
    UPALRAM[1] = PALRAM[0x08];
    UPALRAM[2] = PALRAM[0x0C];
    PALRAM[0x0C] = PALRAM[0x08] = PALRAM[0x04] = PALRAM[0x00];
    PALRAM[0x1C] = PALRAM[0x18] = PALRAM[0x14] = PALRAM[0x10];
+   // Restore the default PPU read/write hooks before resetting. This is
+   // essential for the new-PPU rendering path: CALL_PPUREAD dereferences
+   // FFCEUX_PPURead without a NULL guard, and the only other writer of
+   // this pointer besides PPU_ResetHooks is ResetGameLoaded() (which
+   // NULLs it on every LoadGame). Without this call, the first
+   // bgdata::Record::Read() of the first rendered frame calls a NULL
+   // function pointer and crashes. Board Power() handlers (e.g. MMC5)
+   // run after FCEUPPU_Power via GameInterface(GI_POWER) and may
+   // override the hook themselves.
+   PPU_ResetHooks();
    FCEUPPU_Reset();
    ...
}
```

**为何放在这里**（而非 `FCEUPPU_Reset` 或 runner 侧）：

1. **调用顺序正确**：`PowerNES()`（fceu.cpp:993）的顺序是 `FCEUPPU_Power()` (line 1039) → `GameInterface(GI_POWER)` (line 1042)。在 `FCEUPPU_Power` 里恢复 default hook，之后 mapper 的 `GI_POWER` handler 可以覆盖（如 MMC5 在 `src/boards/mmc5.cpp:1090` 设 `FFCEUX_PPURead = mmc5_PPURead`）。先恢复 default、再让 mapper 覆盖，语义正确。
2. **覆盖所有入口**：`FCEUPPU_Power` 是 ROM 加载后 PPU 上电的唯一入口（PowerNES → FCEUPPU_Power）。在此处修复，无论 GUI、headless、还是测试 runner 都受益，无需逐个调用点打补丁。
3. **不破坏 savestate**：`FFCEUX_PPURead` 是函数指针，不参与 savestate（`ppu_state.cpp` 的 SFORMAT 表不含它），恢复它对存档加载/保存无影响。

### 4.2 完整性修复：`PPU_ResetHooks` 同时恢复 Write 钩子

**文件**：`src/ppu_core.cpp`，函数 `PPU_ResetHooks()`（line 83）

```cpp
void PPU_ResetHooks() {
    FFCEUX_PPURead = FFCEUX_PPURead_Default;
+   FFCEUX_PPUWrite = FFCEUX_PPUWrite_Default;
}
```

`ResetGameLoaded()`（fceu.cpp:396-397）同时清空了 `FFCEUX_PPURead` 和 `FFCEUX_PPUWrite`。虽然 `CALL_PPUWRITE`（ppu.cpp:303）**有**判空（`FFCEUX_PPUWrite ? ... : ...Default`），NULL 时安全走 Default，但为一致性（`PPU_ResetHooks` 的语义是「恢复默认钩子」）应同时恢复两者。`FFCEUX_PPUWrite_Default` 已在 `src/ppu.cpp:172` 定义、`src/ppu.h:63` 声明。

### 4.3 修复验证

修复后重新构建（无诊断版），运行：

```
$ --frames 3
BLARGG_RESULT: rom=ppu_vbl_nmi.nes addr=0x6000 value=0x80 diag=[0xDE,0xB0,0x61] status=FAIL duration_ms=6
EXIT=1    ← 不再是 127 崩溃；exit 1 = 测试失败（正常的 Oracle B FAIL 退出码）

$ --frames 300
BLARGG_RESULT: rom=ppu_vbl_nmi.nes addr=0x6000 value=0x01 diag=[0xDE,0xB0,0x61] status=FAIL duration_ms=493
diag_string="
VBL period is too long with BG off
01-vbl_basics
Failed #8
While running test 1 of 10
"
EXIT=1    ← 300 帧正常完成（493ms），输出完整诊断字符串
```

**核心阻塞问题（崩溃）已彻底消除**。新 PPU headless 渲染路径首次跑通，Oracle B 现在能拿到真实的 ROM 诊断输出——这正是 P4-bridge 期待的状态。

### 4.4 Oracle A 回归验证（关键约束 #3）

修复后运行 ctest + 关键测试单独验证：

| 测试 | 结果 | 说明 |
|------|------|------|
| `rom_regression_test`（Oracle A 核心） | **PASSED** | 720 帧对比，0 mismatch。nestest 13/13 ROM 全过 |
| `ppu_test`（含 `PPU_ResetHooks` 断言） | **24/24 PASSED** | 包括 `PPU_ResetHooks restores FFCEUX_PPURead to default` 与 `engine runs 1 frame after PPU_ResetHooks` |
| `smoke_test` / `headless_smoke_test` / `blargg_smoke` | PASSED | |
| `savestate_regression_test` / `golden_savestate_test` | PASSED | 存档兼容性无影响 |
| `ppu_frame_diff_test` / `ppu_phase_c_test` / `ppu_phase_d_test` / `ppu_rendering_lut_test` | PASSED | PPU 渲染相关全过 |
| `mapper_*_test`（load/reset/core/byte_diff） | PASSED | mapper 兼容性无影响 |
| `cpu_test` / `apu_test` / `bus_test` / `cart_class_test` | PASSED | |
| `ppu_test`（ctest 内 BAD_COMMAND） | ⚠ 既有配置问题 | ctest 从 `build/` 运行时 WORKING_DIRECTORY 解析错误；手动从 `tests/` 运行 24/24 PASS。**与本次修改无关** |
| `lua_bit_test_headless` | ⚠ 既有配置问题 + 真实 Lua 库 bug | CMake 配置中 `WORKING_DIRECTORY=tests` 与参数 `tests/lua_scripts/test_bit.lua` 路径前缀重复，解析为 `tests/tests/lua_scripts/...` 找不到。**与本次修改无关**（修改仅触及 ppu.cpp + ppu_core.cpp） |

**结论**：修复对 Oracle A 零回归。ctest 报告的 2 个「失败」中 ppu_test 为既有工作目录配置问题，lua_bit_test_headless 为路径前缀 + Lua bit 库 bug 双重问题（P5 M2 修复后 5 个 FAIL 将被正确检测）。均非 P4-bridge 代码回归（手动从正确目录运行均通过）。

---

## 五、修复后浮现的新问题：P4-1 VBL 时序「too long #8」

崩溃修复后，`ppu_vbl_nmi.nes` 首次在新 PPU 下产出真实诊断：

```
"VBL period is too long with BG off\n01-vbl_basics\nFailed #8\nWhile running test 1 of 10"
```

### 5.1 与报告记录的对比

| 阶段 | 诊断 | 子测试 | 含义 |
|------|------|--------|------|
| 报告 §3.3（旧 PPU，crash 前） | `VBL period is too short with BG off` | `Failed #7` | VBL 周期**太短** |
| 本次（新 PPU，crash 修复后） | `VBL period is too long with BG off` | `Failed #8` | VBL 周期**太长** |

P4-1 的修复（`ppu_rendering.cpp:1552 runppu(1)`，把 VBL flag 从 cycle 0 移到 cycle 1）在 crash 修复前**从未在新 PPU 下真正被验证过**（因为帧 3 在到达诊断输出前就 crash 了）。报告 §3.3 的「too short #7」是在旧 PPU 下测得的（旧 PPU 无 PPU dot 概念，无法通过）。现在新 PPU 路径打通后，P4-1 的 `runppu(1)` 实际效果是**过度补偿**——把 VBL 周期从「太短」推成了「太长」。

### 5.2 时序分析（VBL 段 cycle 计数）

`FCEUX_PPU_Loop` 正常渲染分支的 VBL 段（ppu_rendering.cpp:1547-1582，NTSC）：

```
runppu(1);                         // 1552: P4-1, cycle 0→1, VBL flag 置位 (dot 1 of sl 241)
PPU_status |= 0x80; TriggerNMI();
delay = 19;                        // 1561: 原 20，因上面消耗 1
for dot in 0..delay: runppu(1);    // 1569: sl 241 跑到 cycle 1+19 = 20
sltodo = 20;
for S in 0..20:                    // 1574: 20 条 VBL scanline
    for dot in (S==0?19:0)..341:   //       第一行从 19 起算
        runppu(1);
    ppur.status.sl++;
PPU_status = 0;                    // 1582: VBL 清零
```

「too long」提示 VBL 置位到清零之间的总周期数比真机多。可疑点：

1. **`delay=19` 与 scanline 边界**：sl 241 第一行跑了 `1(P4-1) + 19(delay) + (341-19)(S==0 余)` = `1 + 341` = 342 cycle，比标准 341 多 1。这个多出来的 1 cycle 可能把 VBL 拉长。
2. **NMI 在 cycle 1 触发 vs cycle 0**：P4-1 把 NMI 从 cycle 0 移到 cycle 1。若 blargg 测试测的是「NMI 到 VBL 清零」的时长，cycle 1 起算会让区间看起来更长。
3. **`PPU_status = 0` 的时机**：1582 行在 20 条 VBL scanline 跑完后清零 VBL flag。真机在 sl 261（prerender）cycle 1 清零。若新 PPU 的 scanline 计数与真机差 1，VBL 区间会偏长。

### 5.3 这是下一阶段（P4-1 精度调优）的工作，非阻塞

此问题与本次 crash 修复**性质不同**：
- crash 修复解决的是「路径根本走不通」（阻塞性，Oracle B 完全无法运行）。
- 「too long #8」是「路径走通后的精度偏差」（非阻塞，Oracle B 已能产出诊断，可迭代）。

这正是报告「优先级 2：渲染路径通后立即验证 P4-1」所期待的状态——现在路径通了，可以基于真实诊断迭代时序。建议的下一步见 §六。

---

## 六、后续行动建议

### 6.1 优先级 1（已完成）：crash 修复

`PPU_ResetHooks()` 调用补入 `FCEUPPU_Power()`。**已落地、已验证、Oracle A 零回归**。建议立即提交（commit message 示例）：

```
fix(ppu): restore FFCEUX_PPURead/Write hooks in FCEUPPU_Power

PPU_ResetHooks() — the only function that restores FFCEUX_PPURead
from NULL back to FFCEUX_PPURead_Default — was never called from the
production init chain (only from tests/core/ppu_test.cpp). Since
ResetGameLoaded() NULLs FFCEUX_PPURead on every LoadGame, the pointer
stayed NULL after load. The old-PPU render path never calls
CALL_PPUREAD (it uses VPage[]/vnapage[] directly), so this was latent.
The new-PPU render path (FCEUX_PPU_Loop) calls CALL_PPUREAD hundreds
of times per scanline via bgdata::Record::Read(); the first call on
frame 3 dereferenced NULL and crashed (reported as "frame 3 hang").

Fix: call PPU_ResetHooks() in FCEUPPU_Power() before FCEUPPU_Reset(),
so the default read/write hooks are restored on every power-on. Board
Power() handlers (e.g. MMC5) run afterwards via GameInterface(GI_POWER)
and may override. Also restore FFCEUX_PPUWrite in PPU_ResetHooks for
completeness (CALL_PPUWRITE already guards NULL, but consistency).

Verified: blargg ppu_vbl_nmi --frames 300 no longer crashes (was
exit 127, now completes with real diagnostic). Oracle A regression
0 mismatches (rom_regression_test 720 frames). ppu_test 24/24 PASS.
```

### 6.2 优先级 2：P4-1 VBL 时序「too long #8」调优

基于 §5.2 的时序分析，建议的实验顺序（每步用 `--frames 300` 验证诊断变化）：

1. **实验 A**：将 `delay` 改回 20，去掉 1552 的 `runppu(1)`（即回退 P4-1），确认是否回到「too short #7」。这能确认 P4-1 的方向正确但幅度过大。
2. **实验 B**：保留 `runppu(1)`，但调整 sl 241 第一行的 cycle 计算，消除 §5.2 可疑点 1 的「342 vs 341」偏差。可能需要让 `S==0` 的内层循环从 `delay` 改为 `delay-1`，或调整 `runppu(1)` 与 `delay` 的关系。
3. **实验 C**：核查 `PPU_status = 0`（1582）的时机是否对应真机 prerender scanline cycle 1，而非 scanline 边界。

每次实验后，Oracle A 回归（rom_regression_test）必须保持 0 mismatch。

### 6.3 优先级 3：headless runner 的 newppu 设置时序（次要）

`blargg_runner.cpp:169` 在 `LoadGame` **之后**设置 `newppu=1`。这导致 `FCEU_ResetVidSys()`（fceu.cpp:1062，在 LoadGame 内 line 513 调用）用 `newppu=0` 计算扫描线参数：

- `normalscanlines`：runner 手动设 241（line 170）绕过，新 PPU 循环边界正确。
- `overclock_enabled`：`if(newppu) overclock_enabled=0`（fceu.cpp:1078）在 LoadGame 时 newppu=0 未执行，若用户之前开过超频则残留非 0。**新 PPU 路径不读 `overclock_enabled`/`totalscanlines`**（grep 确认这些仅在旧 PPU `FCEUPPU_Loop` 的 687/1222/1293/1303 行使用），故当前无实际影响，但是潜在不一致。
- `FCEUPPU_SetVideoSystem` / `SetSoundVariables`：在 LoadGame 时已按 NTSC 调过，`scanlines_per_frame=262` 对 NTSC 无论 newppu 都相同。

**建议**：将 `newppu = 1` 移到 `LoadGame` **之前**（runner 的 `run_one_rom` 里调整顺序），或在设置后调用 `FCEU_ResetVidSys()` 重新传播。这能让 `overclock_enabled` 被正确清零、`normalscanlines` 由引擎而非 runner 硬编码。非阻塞，但消除了「runner 与引擎状态不一致」的隐患。

### 6.4 优先级 4：增强 `CALL_PPUREAD` 的防御性（可选，长期）

`CALL_PPUREAD`（ppu.cpp:301）不判空，而 `CALL_PPUWRITE`（ppu.cpp:303）判空。这种不对称是本次 bug 的放大因素——即便 `PPU_ResetHooks` 漏调，判空也能让 NULL 退化为「返回 0」而非崩溃。可考虑统一为判空宏：

```cpp
#define CALL_PPUREAD(A) (FFCEUX_PPURead ? FFCEUX_PPURead(A) : FFCEUX_PPURead_Default(A))
```

但这会改变 NULL 时的语义（从「崩溃暴露 bug」变为「静默返回 0」），可能掩盖未来的类似问题。**本次修复已从根本上解决 NULL 来源**，故此条仅作长期健壮性记录，不强制。

---

## 七、对报告「剩余候选方向」的回应

报告 §3.1 列出 5 个「剩余候选方向」并推荐 VS 调试。本方案的结论对它们的回应：

| 报告候选 | 本方案结论 |
|----------|-----------|
| ① per-scanline sprite evaluation（`spr_read.start_scanline()` → OAM 读取） | **非根因**。`A2004` 的 `for(i=spr_read.last; i!=ppur.status.cycle; ++i)`（ppu.cpp:355）在 `last > cycle` 时确有死循环风险，但崩溃发生在它**之前**（`bgdata::Record::Read` 的 `CALL_PPUREAD` 先触发）。crash 修复后未观察到 spr_read 死循环。仍建议作为后续健壮性加固（加 `last > cycle` 的 break guard）。 |
| ② per-tile BG fetch（`bgdata.main[xt+2].Read()` → mapper 回调） | **方向正确，定位精确**。`bgdata::Record::Read` 确实是崩溃入口，但根因不是「mapper 回调」，而是其中的 `CALL_PPUREAD` 解引用 NULL。 |
| ③ `CALL_PPUREAD` → `ARead[addr]` 在 headless 下的行为 | **`CALL_PPUREAD` 是崩溃点**，但不是 `ARead` 的问题——是 `FFCEUX_PPURead` 本身为 NULL。 |
| ④ `runppu1_inline()` 中 `X6502_Run(1)` — CPU NMI handler 再入 PPU | **非根因**。`X6502_Run` 因 `CycTable[op]≥2` 必然终止（已证明），CPU 不会在帧 3 空转。 |
| ⑤ 正常渲染路径对 Qt driver 回调的隐式依赖 | **非根因**。null driver 全 nullptr 且转发层判空，安全 no-op。问题不在 driver 回调，在 PPU 内部的 `FFCEUX_PPURead` 钩子（不受 `DriverCallbacks` 覆盖）。 |

报告推荐的「VS 调试」**非必需**——本方案通过代码精读 + 诊断打印已定位并修复。但若未来需调试渲染时序类问题（如 §6.2 的「too long」），VS 逐 scanline 断点仍有价值。

---

## 八、变更清单

本次修改共 2 个文件、+11 行，已通过全部回归：

```
src/ppu.cpp      | 10 ++++++++++
src/ppu_core.cpp |  1 +
2 files changed, 11 insertions(+)
```

- `src/ppu.cpp` `FCEUPPU_Power()`：`FCEUPPU_Reset()` 前加 `PPU_ResetHooks();`（+9 行含注释，+1 行调用）
- `src/ppu_core.cpp` `PPU_ResetHooks()`：加 `FFCEUX_PPUWrite = FFCEUX_PPUWrite_Default;`（+1 行）

诊断代码（`blargg_runner.cpp` 的 DIAG-KAGAMI 打印 + `#include "ppu.h"`）已验证后移除，未入库。

---

## 九、关键约束复核

> 报告 §六「关键约束重申」

1. **新 PPU 是唯一方向** ✅ —— 本修复**不回退旧 PPU**，而是让新 PPU headless 路径首次跑通。旧 PPU 因不调 `CALL_PPUREAD` 而免于此 bug，但这不构成「用旧 PPU」的理由——新 PPU 的 cycle 级精度是 Oracle B 的权威性基础。
2. **headless 是 Oracle B 的基础** ✅ —— 修复后 headless 新 PPU 可完整运行 300 帧，Oracle B 不再依赖 GUI。
3. **Oracle A 全绿是每次修改的前置条件** ✅ —— `rom_regression_test` 720 帧 0 mismatch，`ppu_test` 24/24 PASS。
4. **AI 不得修改已入库的 expected 值** ✅ —— 本修复不触及任何 baseline / golden hash / expected 值，仅修复初始化链的函数指针恢复遗漏。

---

## 附录 A：关键文件与行号速查

| 主题 | 位置 |
|------|------|
| `FFCEUX_PPURead` 定义（初值 NULL） | `src/ppu.cpp:298` |
| `FFCEUX_PPURead_Default` 实现 | `src/ppu.cpp:273` |
| `CALL_PPUREAD` 宏（不判空） | `src/ppu.cpp:301` |
| `CALL_PPUWRITE` 宏（判空） | `src/ppu.cpp:303` |
| `ResetGameLoaded()` 置 NULL | `src/fceu.cpp:396-397` |
| `ResetGameLoaded()` 调用点 | `src/fceu.cpp:454`（LoadGameVirtual 开头） |
| `PPU_ResetHooks()` 定义 | `src/ppu_core.cpp:83` |
| `PPU_ResetHooks()` 唯一生产调用（本次新增） | `src/ppu.cpp` `FCEUPPU_Power()` 内 |
| `FCEUPPU_Power()` | `src/ppu.cpp:766` |
| `FCEUPPU_Reset()`（设 `new_ppu_reset`/`ppudead`） | `src/ppu_core.cpp:87` |
| `PowerNES()` 调用 `FCEUPPU_Power` | `src/fceu.cpp:1039` |
| `GameInterface(GI_POWER)`（mapper 覆盖机会） | `src/fceu.cpp:1042` |
| MMC5 覆盖 `FFCEUX_PPURead` | `src/boards/mmc5.cpp:1090` |
| `FCEUX_PPU_Loop` ppudead 分支（不调 CALL_PPUREAD） | `src/ppu_rendering.cpp:1524-1545` |
| `FCEUX_PPU_Loop` 正常渲染分支 | `src/ppu_rendering.cpp:1547-2000` |
| 崩溃点 `CALL_PPUREAD`（nametable fetch） | `src/ppu_rendering.cpp:1417` |
| `bgdata::Record::Read()` | `src/ppu_rendering.cpp:1408` |
| `runppu` / `runppu1_inline` | `src/ppu_rendering.cpp:1359 / 1386` |
| `A2004` spr_read 循环（后续健壮性候选） | `src/ppu.cpp:346-474` |
| blargg_runner `newppu=1` 设置点 | `tests/blargg_runner.cpp:169` |
| `FCEU_ResetVidSys` newppu 分支 | `src/fceu.cpp:1078` |
