# FCEUX11 v2.0.2 DEBUG 与性能优化 + v1.18 残留精度演进 · 已完成项归档

> **STATUS: ARCHIVED**（归档于 2026-11-08，经独立 code review 后拆分）
> **来源**：`docs/plans/FCEUX11-v2.0.2_DEBUG与性能优化计划.md`（已删除）
> ＋ `docs/plans/long-term-evolution/`（README / 主报告 / T0–T4 五张执行卡，已删除）。
> **本文件的定位**：两份计划中**已确认 100% 完成**的部分的事实源。内容冻结，不再续写；
> 未完成部分见 `docs/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`。
> **归档当刻 HEAD**：`2960998`（v2.0.2 收口批次，r26 之后）

---

## 〇、归档口径（三条同时满足才算「100% 完成」）

1. **交付物在树内可验** —— 代码 / 文档 / 锁测试 / 提交，逐项给出 `file:line` 与提交号；
2. **验收判据已满足**，或该项已由业主裁决**终态关闭**（abandon / closed / 永久封口）且无在办残留；
3. **该 ID 之下无残留工作项** —— 只要还有一个子步骤没做（哪怕主体已完工），就**不**进本文件。

> ⚠️ **判据 2 的一个易犯错误**：某项「用例仍 FAIL」**不等于**「未完成」。
> 若 FAIL 是**业主裁决记为已知限制**（advisory + baseline false），则该项**已终结**、进本文件；
> 若 FAIL 是**精度缺口本身**（用例该绿没绿），则该项**未完成**、进 review 报告。
> 本文件 §二 与 review 报告的分界就是这条线。

> ⚠️ **不要按「文件名/符号名」找实现**。归档过程中实测发现两处计划文档与代码树的**命名错位**
> （详见 §五）：A1 的门没有叫 `debugger_active`；kgmqa-121 的锁测试文件名不含 `kgmqa-121`。

---

## 一、归档当刻的门禁读数（实测，非转述）

| 门禁 | 读数 | 采集方式 |
|---|---|---|
| `ctest -LE perf` | **36 / 36 全绿**（9.57 s） | 本次**实跑**（构建完成后单跑，未与构建/测试并行） |
| `cargo test -p fceux11-rust --no-default-features --features gba --lib` | **249 passed / 0 failed / 19 ignored** | 本次**实跑** |
| `cargo test -p gba-core --lib` | **242 passed / 0 failed / 2 ignored** | 本次**实跑** |
| `cargo test -p fceux11-debug --lib` | **93 passed / 0 failed** | 本次**实跑** |
| `cargo test -p fceux11-media` | **52 passed / 0 failed** | 本次**实跑** |
| F11QA 迁移矩阵 | **122 total / 112 passed / 10 failed / 0 skipped / grade B / `pass_to_fail` 空**，`fail_to_pass` 3 项（049 / 050 / 051），`new_test` 1 项（121） | 本次**实跑** |
| └ 分层读数 | `A_regression`（内部逻辑检测）**42P / 1F**；`B_hardware` **70P / 9F**；`grade_reasons` = 「10 advisory known-limit failure(s), **all within frozen baseline**」 | 同上 |
| └ 迁移桶全量 | `fail_to_fail` 恰 10 条：**037 / 038 / 077 / 081 / 096 / 097 / 099 / 107 / 108 / 115** | 同上 |
| bench 基线 | 六档，`tolerance_pct` 10.0，`captured_at` 2026-11-07 | 读 `tests/fixtures/bench_baseline.json` |

**构建**：`cmake --build build --config Release` 在 HEAD 上 **exit 0**（48 个 target）。

> ✅ **重组后已复验**：本次删除原计划、修复全仓引用、重建并**再跑一遍全部门禁**，
> 读数与上表**逐项一致**（ctest 36/36；249 / 242 / 93 / 52；矩阵 122/112/10/grade B、
> `pass_to_fail` 仍为空、`fail_to_pass` 仍恰为 049/050/051）⇒ **本次文档重组零回归**。

> ⚠️ **采集纪律（本轮实测撞到，值得记）**：首轮 `ctest` 曾出现 2 项 `BAD_COMMAND`
> （`cpu_test` / `fds_load_test`，输出为空、耗时 0.00 s）。根因是**同一 build 树上并发跑着 ninja**，
> 两支 exe 的写入时刻正落在测试执行瞬间。**构建结束后复跑即 36/36 全绿。**
> ⇒ 复核者**不得在构建并行时采信 ctest 读数**（与 AGENTS.md r54 ② 同旨）。

> ⚠️ **`A_regression` 的 1 个 FAIL 不是回归**：唯一一条是 **kgmqa-115-bench-tolerance**
> ——墙钟类 bench，其 PASS/FAIL **不具跨机可比性**（基线采于 dev box，本次跑在更慢的机器上）。
> 该用例已在业主裁决 r15 中移出「精度回归」判定（`failure_means` blocking → advisory、
> 冻结基线 true → false），故**不判红、不影响 grade**（`grade_reasons` 明确记
> 「10 advisory known-limit failure(s), all within frozen baseline」）。
> ⇒ 「内部逻辑检测 42P / 0F」这条旧不变式**当前实为 42P / 1F**，且这 1F 是**设计内的**。

---

## 二、已完成工作项（代码已落地 · 验收已满足）

### 2.1 线 A —— NES 性能

| 项 | 提交 | 落点与证据 | 备注 |
|---|---|---|---|
| **P0/M1** bench `soundq=1` 测量位 + 基线重采 | `eb664a3` | `tests/fixtures/bench_baseline.json`：六档（`bench_cpu_frame` 34.049 / `bench_ppu_frame` 29.174 / `bench_full_frame` 32.135 / **`bench_full_frame_hq` 41.006** / `bench_apu_frame` 32.017 / `bench_bus_dispatch` 33.861），`tolerance_pct` 10.0，`captured_at` 2026-11-07 | A 线累计 cpu −27% / ppu −37% / full −34% / **hq −48%** |
| **P0/M2** GBA 帧导出计时探针 | — | `src/rust/src/gba/frame.rs:3352` `measure_frame_export`（`#[ignore]`） | 只作量级参照，**不进 ctest** |
| **A2** APU 帧计数器批量步进 | `062ce84` | 落位表 `src/sound.cpp:676-716`（PAL/NTSC × 4-step/5-step 四张表）；批量循环 `:808-841`，`:825` `const int32_t next = FcNextEventPos(fhcnt);`；`:839` `FrameCounterTick();` **保留为唯一点火权威**；锁测试 `tests/f11qa/core/apu_frame_timing_test.cpp`（运行期 **59** 断言 = 39 个静态 `FCEU11_EXPECT` + 21 处 `drive()` 各追加 1 条，`:246`）；门禁 `tests/CMakeLists.txt:192-198`（`TIMEOUT 300`）+ `tests.json` 的 `kgmqa-121-apu-frame-timing`（**blocking**） | ⚠️ 锁测试**文件名不含 kgmqa-121**，只在文件头注释与 `tests.json` 的 `kgmqa_id` |
| **A3** DoLine 每扫描线后处理合并 | `39b92db` | `src/ppu_rendering.cpp:764-765` dtarget 64 次循环 → `FCEU_dwmemset(...)`；`:751-757` deemphasis 三分支 64 次 RMW → 三行各一次 `fceu11::fceu_dwrmw_andor(target, mask, bits, 256)`；新 helper `src/utils/simd_fill.h:97-132`（AVX2 `loadu/and/or/storeu` + memcpy 语义标量回退） | **hotfix1 H-06 别名纪律保留**（未绕回 memcpy 中转坑）；三分支互斥 ⇒ 每扫描线实为一趟宽处理 |
| **A4-2** 帧级 WAVEDUMP 等价对拍 | — | `src/sound.cpp:1483-1508`，env `FCEUX11_A4_WAVEDUMP=<dir>`，dump `WaveHi[0..SOUNDTS)` 逐帧 | ⚠️ 只在 `FSettings.soundq>=1` 分支触发 ⇒ 低质量路径到不了，须走 `bench_full_frame_hq` 槽 |
| **A5 G0–G3** NeoFilterSound 无损 SIMD | `0da9ad4`（G0）+ `726e872`（G1/G2） | `src/rust/crates/fceux11-media/src/filter.rs:337` `unsafe fn fir_pair_avx2(...)`、`:336` `#[target_feature(enable = "avx2")]`、`:461` `is_x86_feature_detected!("avx2")` **运行时探测**（绕开 MSVC `__AVX2__` 宏陷阱）；锁 `:762` `lock_fir_pair_scalar_simd_bit_exact`、`:1101` `lock_resample_interpolation_golden_vector`（**绝对金标准向量**，独立参考实现，不比较两个实现） | 自建 AC fixture `tests/fixtures/apu_square.nes`（16400 B，**已 git 跟踪**，`.gitignore:140` 有白名单）；HQ 门禁非静默断言 `tests/core/apu_wav_hq_diff_test.cpp:336-351`、低质量 `:297-318`（`tests/core/apu_wav_diff_test.cpp`）——**两道 golden 各自有牙、覆盖不同代码路径** |

### 2.2 线 B —— NES 调试

| 项 | 提交 | 落点与证据 | 备注 |
|---|---|---|---|
| **B1** TraceLogger 无头入口 | `4c34cbf` | `src/headless_trace.cpp:104` `getenv("FCEUX11_TRACE")`；`:107` 未设/空 ⇒ 什么都不做；`:126` `path = (strcmp(env, "1") == 0) ? "fceux11-trace.log" : env;`；`:119-124` 槽已占则让位；`:135` 装采集器；`:137` `g_driver().trace_logging = 1`（**A1 快门的一条腿**）；唯一安装点 `src/fceu.cpp:622` | env 语义与计划**逐字一致**。计划称「纯剪切 452 行删除」，实测 diff 是 `TraceLogger.cpp` 65+/493−＋新建 `trace_common.cpp`（601 行），且该文件自陈「纯剪切**外加两处结构性抽出**」 |
| **B3** Lua 调试 API 补 range 系 | `34ce329` | `src/rust/crates/fceux11-lua/src/bindings/memory.rs` 导出 9 → **11**（`readbyterange` `:63` / `writebyterange` `:82`，另 9 个既有导出不变）；`tests/lua_scripts/test_memory.lua` 用例 **18 → 28**；Rust 单测 0 → 5 | ⚠️ 计划与提交说明写「+13 用例」，**实为 +10**；其中 2 条是恒真空断言（`check("writebyterange empty string", true)` 等）⇒ 有效新断言 8 条。功能本身已交付 |

### 2.3 线 C —— GBA

| 项 | 提交 | 落点与证据 | 备注 |
|---|---|---|---|
| **C1** 帧导出热路径 | `672bcf3` | `src/rust/src/gba/frame.rs:910` `static BGR555_TO_RGBA: [u32; 32768] = build_bgr555_to_rgba();`、`:912` `const fn` 构建、`:934` `write_frame_bytes` **直写 dst**、`:332` `gba_frame_buffer` 把调用方持久缓冲直接交给它（零中间 Vec、全帧写 1 次、分配 2→0）；穷举等价锁 `:1349` `the_bgr555_lut_matches_the_scalar_reference_exhaustively` | 标量 `scale5` 路径保留为**独立参照**（`:894/:948/:978`）⇒ 锁测试比的是两条独立路径 |
| **C2** savestate 延迟 | `6d88c56` | C++ 侧 `src/gba_load.cpp:272` 2 MB 首猜 + `:276-281` **单次** `GBA_ERR_CAPACITY` 重试（不再先 `gba_savestate_size` 量大小）；Rust 侧全 ROM FNV-1a **只在 `Machine::new` 算一次**并缓存（`frame.rs:91` 字段 / `:110` 计算 / `:422` encode 读缓存 / `:461` load 比对），`save.rs:143/:154` | `save.rs` 里 5 处 `RomFingerprint::of(...)` 全在 `#[cfg(test)] mod tests` 内，不属生产路径 |
| **C3** GBA 诊断工具文档化 + 计数更正 | `241860f` | `docs/tech/gba_probe_howto.md`（85 行，覆盖 `FCEUX11_DIAG_LOG`、`GBA_PROBE_*` 全族、r53/r56 探针、触发方式）；已登记于 `docs/tech/README.md:28`；`FCEUX11_DIAG_LOG` 实现 `src/drivers/Qt/main.cpp:325` / `src/rust/src/gba/mod.rs:77` | AGENTS.md 的 `#[ignore]` 计数**已是正确的 21 条**（root 19 = save.rs 2 + frame.rs 6 + waitprobe.rs 10 + cartprobe.rs 1，vendor 2 = gba-core timers） |
| **F-2** 无头回落反汇编器助记符表 | `ce89bc7` | `src/rust/crates/fceux11-debug/src/asm.rs:294-362` 按 opcode 升序重写（官方 151 + 非官方单字节 NOP）；`:317` NOP 臂；`:478` `lock_disasm_mnemonics_256` **256 项穷举 + 扁平独立参照表**（结构上与被测 `match` 互不相交） | 端到端验证脚本 `.zcode/d1s2/a1_checklist/verify_trace_f2.py`（按 ROM 字节推导期望值逐行比对官方矩阵，走 `trace_common.cpp:33` → `asm.rs:24` → Rust → 文本） |

### 2.4 线 D —— 长期难题（已终结项）

| 项 | 提交 | 落点与证据 | 终态 |
|---|---|---|---|
| **D4 / 050** OAM DMA dummy write | `b0a016a` + `c65c6a6` | 修复 `src/ppu_rendering.cpp:1651-1652` `if (PPU[1] & 0x18) PPU[3] = PPUSPL = 0;`（根因：VBL 入口无条件清 OAMADDR，硬件语义是仅渲染开启的 dots 257-320 清）；根因注释 `:1638-1650`；锁 `tests/f11qa/core/ppu_test.cpp:281-325` `test_oamaddr_preserved_vbl_rendering_off`（planted SPRAM `:285-286`、真 `$2003` 写路径 `:289`、真 `$2004` 读路径 `:308`）；帧预算 300→400（`tests/tests.json`） | **矩阵 PASS**（`fail_to_pass` 一项） |
| **D5 / 051** IO 空间取指 / bus dispatch | `4b45762`（S0+S1）+ `1eaf8cc`（S2）+ `7864b17`（预算） | `src/bus.h:160-165` `read_fetch()` **纯别名**（`return read(addr);`，分叉白名单**为空**，`:161-163` 注释记录「没有探针到任何 fetch/read 语义分叉」）；CPU 取指走此路 `src/x6502.cpp:74/:1069`；探针 `src/bus.cpp:42-51`；锁 `tests/f11qa/core/bus_test.cpp:493-636`，运行期 **85** 断言（70 静态 − 3 循环内 + 循环展开 18） | **前提被证据证伪**：IO 域取指 31 次 / 257 万（0.001%），与真机开放总线取指语义一致 ⇒ **无剩余行为工作**；矩阵 PASS |

### 2.5 v1.18 长期演进 —— 已终结项

| 项 | 提交 | 落点与证据 | 终态 |
|---|---|---|---|
| **056** `instr_timing` | `8010a3f` | `src/x6502.cpp:532` `CycTable` 行 `/*0xE0*/ 2,6,2,8,...` ⇒ **E2 = 2**（原 3）；`src/ops.inc:433` `case 0xBB: LD_ABY(...)`（原 `RMW_ABY`）⇒ 跨页 +1 由 `GetABIRD` 提供 ⇒ **BB 跨页 = 5**（原 4）；`src/ops_table.inc` 为 `scripts/generate_x6502_dispatch.py` 的**生成产物**，**内存重跑生成器逐行完全一致**（35278 字符 MATCH）；帧预算 300→3000（`tests/tests.json:1801-1802`） | **DONE** |
| **078** `serom` | v1.18.2 | `src/boards/mmc1.cpp:53` `#define MMC1_SUBMAPPER_FIXED_PRG 5`；`:448-458` `if (info->ines2 && info->submapper == 5) { if (ws == 0) ws = 8; ...; isFixedPRG = 1; MMC1PRG(); return; }`（**承载项**：PRG-RAM 为空回退 8 KB，`:449`）；`:69/:144-151/:368/:455` `isFixedPRG` 固定映射（**规范项**）；`:259` `int ws = 8;` 默认值**原样保留**（回退逻辑挪到调用点） | **DONE** |
| **097** `fme7ramtest` | — | 根因改判：**survey ROM**，不实现 blargg `$6000` 协议（PRG 对 `$6000` 绝对寻址 **0 处**、对 `$6900` **恰 2 处**）；`FCEUX11_FME7_PROBE=1` 实测 bank tag `C0 C1 C2 C3 C0 C1 C2 C3` 与上游 README 逐字节相同 ⇒ **FME-7 WRAM 映射本身是对的**；处置记于 `tests/tests.json:3390-3396`（`known_limit` + `provenance`，2026-09-28 人工授权） | **abandon**（终态） |
| **096** `fme7acktest` | — | 同样 **survey ROM**（`$6000` 是 PRG ROM bank 3，ROM 对 `$6000` 的绝对寻址指令数 = **0**）⇒ 改 mapper 也不会转 PASS。**但确有真实 ack bug**：`src/boards/69.cpp:130` `case 0xD: IRQa = V; X6502_IRQEnd(FCEU_IQEXT); break;` **无条件 ack**（硬件规则是 bit 0 == 0 才 ack，PowerPak / Everdrive / Nintendulator 三方一致）⇒ **代码一行未改**（业主裁定只 abandon、不改代码）；`tests/tests.json:3351-3357` 记录 known_limit + provenance | **abandon**（终态，**留一个已知不修的缺陷**） |
| **038** `instr_misc` | — | 根因改判：`Cpu::set_db()` **全仓库零调用点** ⇒ `DB` 永不赋值、`::ANull` 返回常量 ⇒ **open bus 是死的**。两次独立负结果：① 改 `GetABIRD` 加 dummy read ⇒ 无变化；② S0a 让 `DB` 活起来（`src/bus.h:45` / `src/bus.cpp:281` `FCEUX11_BUSDB`，**默认关**、零回归）⇒ 开关关/开、600/3000 帧结果完全相同 | **defer**（终态，**S2 需重新定义**） |

---

## 三、已终结（无代码交付）的决策项

这些项**没有代码交付**，但**已由业主裁决或规则终结**，不再有在办工作：

| 项 | 裁决 | 依据 |
|---|---|---|
| **A6** bus 直读快路径 | **关闭，不再立项**（业主裁决 r25） | 探针三连稳定但收益 3–8% 骑线；r12 排它后面的「D1 收口后再动」前提在方法论上不成立（收益是性能问题，静默探针测不出）；两个更便宜替代（**PGO** 开关现成 / **A7**）从未动。**S0+S2 的 85 条护甲断言转常驻**、零维持成本。实测确认：**无任何 A6 快路径落地**——`src/bus.h:115-131` 的 `read()` 仍是 64K 函数指针间接派发；`direct_read` 早于 A6 多年（`f44c3df` v1.4）且自述「debugger path, not the hot loop」。**⇒ 替代槽位是 PGO 或 A7**（A7 未动，见 review 报告） |
| **B5** 帧级 rewind 调试 | **本期不做，彻底封口**（不写路线图条目） | 成熟模拟器的「调试器内逐帧回溯」与 TAS Editor greenzone 是两套机制，新做属大件，与 v2.0.2 主题性价比不符 |
| **B4** 断点上限与条件表达式 | **不默认立项** | 64 断点上限与二元比较算术表达式是**产品取舍**；要做先问真实用户需求 |
| **D6 / 077** Holy Mapperel 聚合 | **不单独立项**（cascade） | 成本已收窄为「仅 L2 MMC3 IRQ 组」= 完全依赖 D1。**077 本身仍未做**，见 review 报告 |

---

## 四、指向 review 报告的遗留（**不在本文件**）

以下内容**未完成**，已移入 `docs/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`：

- **A1** 的手工 GUI 验收（代码已落地，**验收未闭合**）
- **A4-1** 的否决护栏（计划承诺写进 `src/video.cpp` 注释，**实际未写**）
- **A7** 指令 handler 别名/溢出障碍（未动工）
- **B2** profiler 的 `FCEU_SoundCPUHook` 探点 + Qt 菜单入口（未做）
- **D1 / 037** 的 S3–S5 与 round-2 两章程（转长尾）
- **D3 / 049** 的 M1–M3（read-buffer 实体工作）
- **D7 / 081** 的 Path B（FDS IRQ 状态机）
- **D9 / 107** 的端到端 0xFC、**D10 / 108** 全部
- **D8 / 099** 的 PAL/Dendy 产品决策
- **`tests/tests.json` 与计划正文的过期描述**（037 / 107 / 056 / 078 的 `known_limit` 等）

---

## 五、归档时的实测发现（计划正文与代码树的两处错位）

> 这两条**不是缺陷**，是「按计划字面去代码树里找会找不到」的陷阱。**接手人请先读这一节。**

1. **A1 的门没有叫 `debugger_active`。** 计划 §二 写「文件级 `debugger_active` 快速门」，
   但全仓 grep `debugger_active` **零命中**。实际实现是 `DebugCycle()` **函数体内的内联早退**：
   `src/debug.cpp:918-919`
   `if (!(numWPs || dbgstate.step || dbgstate.runline || dbgstate.stepout || watchpoint[64].flags || dbgstate.badopbreak || break_on_cycles || break_on_instructions || break_asap || debug_loggingCD || fceu11::g_driver().trace_logging)) return;`
   —— 11 路 OR（9 路 breakpoint 条件 + `debug_loggingCD` + `trace_logging`），与计划描述的判据**逐条一致**，
   只是没有抽成具名变量。`e1_last_pc` 写入已移入门内（`src/x6502.cpp:1066` `if (e1_cpu_trace_on()) e1_last_pc = (uint16)_PC;`）；
   `map_irq_hook` 热路径 load 已改 relaxed（`src/cpu.h:141`，store 仍 release `:144`）。

2. **A1 的覆盖面比计划 §二 标题小。** `ppu_view_open` 在 **core 侧只写不读**：
   全仓只有 `src/drivers/Qt/ppuViewer.cpp:62`（写）与 `src/driver_callbacks.h:126`（声明），
   **无任何 core 读点** ⇒ `FCEUD_UpdatePPUView`（`src/ppu_rendering.cpp:1319` 旧路径 / `:1800` 新路径）
   **仍是无门间接调用**。实际生效的门只有 `nt_view_open`（读点 `src/ppu_rendering.cpp:793`，**仅旧 PPU 路径**）。
   ⇒ **「viewer 已全部剪枝」不成立**（计划 §二 已自行记录此点，但提交信息 `55d82d0` 的措辞过时）。

---

## 六、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 门禁（不变式 1）—— 必须等构建结束再跑，不要与构建/测试并行
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf

# Rust 锁测试（四条都要跑：cargo test -p fceux11-rust 不跑依赖 crate 的测试）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-debug --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-media"

# F11QA 迁移矩阵（122 用例）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo build --release -p f11qa --bin f11qa-runner"
& src\rust\target\x86_64-pc-windows-msvc\release\f11qa-runner.exe --manifest tests\tests.json --bin-dir build\tests --output build\f11qa_migration_matrix.json --accuracy-table build\f11qa_accuracy_table.md --known-fail tests\fixtures\blargg_known_fail.json --baseline tests\fixtures\f11qa_baseline_frozen.json --save-baseline build\f11qa_baseline_next.json
```

---

## 附录 A · 环境变量开关全表（随本次归档一并保留）

> **来源**：原 v2.0.2 计划 §附录 A（**2026-11-06 全表重核**，r9）。该表是**已完成的核查工作**，
> 且是仓库里唯一的运行环境变量索引 —— 原计划删除后无其它去处，故并入本归档。
> **坐标口径**：统一写 **`getenv` 所在行**（不是函数定义行、不是 `#[ignore]` 属性行）。
> **三张表必须分开看**：A 表是**运行期**环境变量（设了就能用）；B 表是**编译期**开关
> （设环境变量开不了）；C 表是上游沿用的配置变量（非探针）。旧表把三者混列，
> 会让人「设了环境变量发现没反应」而误判表错了。

### A · 运行期环境变量（探针 / 诊断）

| 变量 | 取值形式 | 位置（getenv 行） | 作用 | 默认 / 触发条件 |
|---|---|---|---|---|
| `FCEUX11_BUSDB=1` | 严格 `"1"` | `src/bus.cpp:102` | 每次总线**读**把读值锁存进 CPU DB 字节 | 关。⚠️ **会改行为**（改 controller 读的 `DB&0xC0` 与每个 savestate 的 DB 字节） |
| `FCEUX11_E1_TRACE=1` | 严格 `"1"` | `src/ppu.cpp:353`、`ppu_rendering.cpp:1530`、`x6502.cpp:400` | PPU/NMI/CPU 事件 stderr trace（前缀 `E1`/`E1B`） | 关。多数点在 **new-PPU 路径**内；`$2000/$2001` 写与 CPU 侧不分路径 |
| `FCEUX11_E1_NMIDELAY=N` | 整数（`atoi`） | `src/ppu_rendering.cpp:1547` | NMI 锁存前授予的 CPU 预算（1 单位 = 1/3 dot） | 默认 `8`（= 生产值）。⚠️ **这是行为开关、不是日志开关**——调用点不在 `e1_trace_on()` 门内，单独设就改 NMI 注入时机；仅 new-PPU 路径生效 |
| `FCEUX11_OPENDECAY_PROBE=1` | 严格 `"1"` | `src/ppu.cpp:373` | PPUGenLatch open-bus 衰减观测 + SPR DMA 事件日志 + atexit 汇总 | 关 |
| `FCEUX11_E3_TRACE=1` | 严格 `"1"` | `src/sound.cpp:48` | APU 帧计数器状态机（每 quarter）+ `$4017` 写 + `$4015` 读 + IRQ 起止 + 复位事件（前缀 `E3`/`E3B`）——**远不止 `$4017` 写** | 关。进程启动后设无效（首次调用即缓存） |
| `FCEUX11_MMC1_PROBE=1` | 严格 `"1"` | `src/boards/mmc1.cpp:39` | ①初始化几何（声明 PRG/CHR/WRAM/BRAM vs 实际 ROM 大小、ines2/submapper/CRC32、PRG/CHR mask）②每次 MMC1PRG 同步后 `$8000`/`$C000` 的**实际** bank 映射 | 关 |
| `FCEUX11_MMC3_PROBE=1` | 严格 `"1"` | `src/boards/mmc3.cpp:52` | MMC3 扫描线 IRQ / A12 时钟事件 | 关 |
| `FCEUX11_FME7_PROBE=1` | 严格 `"1"` | `src/boards/69.cpp:41` | ①初始化几何（ines2/wram/battery_wram/实际 WRAMSIZE/submapper/CRC32）②寄存器 8 写事件——**「bank tag」是文档侧叫法，代码打的是这两类** | 关 |
| `FCEUX11_OAMBUS_PROBE=<dir>` | **非空即开；值 = 输出目录** | `src/ppu.cpp:627`（判定）、`:635`（取目录） | D4 取证：`<dir>/trace.bin`（SPR DMA 期间每条 CPU 总线事件，24 字节定长，`static_assert` 锁布局）+ `<dir>/spram_diff.log`（SPRAM shadow 比对 diff） | 关。未设或空 ⇒ **完全不激活**（零成本）；目录自动创建；打不开则自禁用并 stderr 报一次 |
| `FCEUX11_A3_XDUMP=<dir>` | **非空即开；值 = 输出目录** | `src/video.cpp:411` | A3 逐字节对拍：每渲染帧 dump `XBuf`/`XDBuf` 各 256×256 到 `<dir>/f%06d_xbuf.bin`/`f%06d_xdbuf.bin` | 关。落点在 `FCEU_PutImage()` 内 |
| `FCEUX11_A4_WAVEDUMP=<dir>` | **非空即开；值 = 输出目录** | `src/sound.cpp:1472` | A4-2 等价性对拍：dump `WaveHi[0..SOUNDTS)` 到 `<dir>/f%06u_wave.bin`（dump 点在 trailing memset 之后） | 关。⚠️ **只在 `FSettings.soundq>=1` 分支**触发 ⇒ 低质量路径（kgmqa-024 等）**永远到不了**，须走 `bench_full_frame_hq` 槽 |
| `FCEUX11_INTLEDGER=<dir>` | **非空即开；值 = 输出目录** | `src/x6502.cpp:451`（判定）、`:459`（取目录） | D1 取证：中断线**时刻账本**——`<dir>/intledger.bin`（8 字节头 `INTL1`+24 + 24 字节定长记录 `{u64 abs; i32 count; u32 irqlow; u16 pc; u8 kind; u8 edge; u8 decision; u8 pad}`，`static_assert` 锁布局）+ `<dir>/summary.log`；kind 0..7 = 八条中断线、0xF = 循环顶边界快照，edge 0/1/2 = release/assert/snapshot，decision 0..6 = 该边界实际走到的判定分支 | 关。**未设或空 ⇒ 探针完全不激活（零运行成本）**；目录自动创建；打不开则自禁用并 stderr 报一次；64 MiB 上限（超出后丢弃新记录、summary 继续）；held 快照连续折叠计数 `held_suppressed` |
| `FCEUX11_TRACE=1` 或 `<path>` | `"1"` ⇒ 默认文件名 `fceux11-trace.log`（相对 cwd）；其它非空值 ⇒ **该值即日志路径** | `src/headless_trace.cpp:104`（判定 `:107`、路径语义 `:126`、安装 `:135`、置位 `:137`）；唯一调用点 `src/fceu.cpp:622` | B1 无头 TraceLogger 入口（与 Qt TraceLogger 同采集体同格式） | 关（未设 ⇒ 零开销零产物）。仅当 Qt TraceLogger 未占槽才安装；装上后置 `trace_logging=1` ⇒ 打开 A1 的 `DebugCycle` 快门 |
| `FCEUX11_DIAG_LOG=<path>` | 完整文件路径 | **写入方** `src/drivers/Qt/main.cpp:325`；**读取方** `src/rust/src/gba/mod.rs:77` | C++ `std::terminate` 栈回溯 + Rust panic 追加写到**同一个** `fceux_diag.log` | ⚠️ **内部握手变量、不是用户开关**：外部设置会被 `main.cpp` 的 `install()` 无条件覆盖成 `<exe目录>\fceux_diag.log` |
| `FCEUX11_BENCH_BASELINE=<path>` | 路径 | `tests/f11qa/benchmarks/bench_tolerance_test.cpp:270` | bench 容差对照基线 JSON 路径（文件内另有 `tolerance_pct`，缺省 2.0%） | 缺省 `fixtures/bench_baseline.json`；CI 显式设它（`.github/workflows/ci.yml:385`） |
| `FCEUX11_GBA_PROBE_DIR=<dir>` | 目录（**必填**） | `src/rust/src/gba/cartprobe.rs:351` | 批量卡带老化报告（逐个 `.gba` 跑，打印 steps/frame、DMA 计数、逐帧哈希、WEDGED/FROZEN 判定） | 必填。未设 ⇒ **panic 并打印指引**（非静默跳过） |
| `GBA_PROBE_ROM=<path>` | 文件路径 | `src/rust/src/gba/frame.rs:2439` | 指定探针跑哪个 `.gba` | 未设 ⇒ 扫 `%USERPROFILE%\Desktop` 下第一个 `.gba`；都没有 ⇒ 打印提示并优雅返回 |
| `GBA_PROBE_STEPS=<n>` | 整数 | `frame.rs:2467` | 探针步数上限（`probe_a_real_cartridge` / `probe_why_a_game_stalls`） | 30,000,000。`probe_what_one_frame_changes`（`:2931`）**不读**它 |
| `GBA_PROBE_WARMUP=<n>` | 整数 | `frame.rs:2805` | `probe_trace_a_stuck_game` 追踪前的预热步数 | 5,000,000 |
| `GBA_PROBE_TRACESTEPS=<n>` | 整数 | `frame.rs:2814` | `probe_trace_a_stuck_game` 追踪的指令条数 | 600,000 |
| `GBA_PROBE_FRAMES=<n>` | 整数 | `frame.rs:3218`（dump，默认 1800）／`frame.rs:3360`（`measure_frame_export`，默认 300） | 帧数 | ⚠️ **两个探针默认值不同**，须分列 |
| `GBA_PROBE_OUT=<path>` | 文件路径 | `frame.rs:3222` | `dump_frames_like_the_reference` 的输出文件 | `ours.txt`（与 mGBA 参照核 dump 对拍用） |
| `GBA_PROBE_PERIOD=<cycles>` | 整数 | `frame.rs:3226` | 帧采样周期（cycles） | 280,896 |
| `GBA_PROBE_TRACE=<cycles>` | 整数（**cycle 数**；解析失败 panic） | `frame.rs:3253` | `dump_frames_like_the_reference` 切「逐步 trace」模式 | 不设 = 周期采样模式。⚠️ **只被该探针读**——`probe_trace_a_stuck_game` 用的是 WARMUP/TRACESTEPS |
| `GBA_PROBE_DIR=decode\|encode` | 枚举字符串 | `src/rust/src/gba/save.rs:760` | 栈探针的编解码方向 | `decode`（其它值一律按 decode 走） |
| `GBA_PROBE_STACK_BYTES=<bytes>` | 整数 | `save.rs:761` | 栈探针的线程栈大小 | 常量 `CODEC_STACK`。小栈下该探针**故意**把进程带崩（`#[ignore]`） |

### B · 编译期开关（**设环境变量开不了**）

| 开关 | 声明位置 | 默认 | 怎么开 |
|---|---|---|---|
| `FCEU_PROFILER_ENABLE` | `CMakeLists.txt:137`（**r6 才补上 `option()` 声明，此前从未声明 = 永远 OFF 且打不开**） | OFF | `cmake -DFCEU_PROFILER_ENABLE=ON`；**需独立 build 目录**，ON 的构建不受门禁 |
| Cargo feature `gba` | `src/rust/Cargo.toml:106` | 在 `default` 里 | 测试命令须 `--no-default-features --features gba` |
| Cargo feature `direct-adapter` | `src/rust/Cargo.toml:101` | 在 `default` 里 | 同上；**默认 feature 引 `kagami_bridge_*`（只有 C++ 链接才有），测试二进制直接 LNK2019** |
| CMake `FCEUX11_BUILD_TESTS` | `CMakeLists.txt:130` | ON | `-DFCEUX11_BUILD_TESTS=OFF` |
| CMake `FCEUX11_ENABLE_I18N` | `CMakeLists.txt:131` | ON | — |
| CMake `FCEUX11_ENABLE_RUST` | `src/rust/CMakeLists.txt:1` | ON | `-DFCEUX11_ENABLE_RUST=OFF` |
| CMake `FCEUX11_LUA_RUST_ENABLED` | `src/CMakeLists.txt:125` | ON | — |
| CMake `FCEUX11_WGI_BACKEND` | `src/CMakeLists.txt:126` | OFF | Windows.Gaming.Input 手柄后端 |
| CMake `FCEUX11_DIRECT_STORAGE_PROBE` | `src/CMakeLists.txt:132` | ON | DirectStorage 1.2 NVMe 探测脚手架 |
| CMake `FCEUX11_ASAN` / `FCEUX11_UBSAN` | `CMakeLists.txt:198` / `:199` | OFF | 仅 MSVC；须 Debug 构建 |
| CMake `FCEUX11_PGO` / `FCEUX11_PGO_USE` | `CMakeLists.txt:95` / `:96` | OFF | 仅 MSVC |
| CMake `FCEUX11_SHOW_DEPRECATION_WARNINGS` | `CMakeLists.txt:149` | OFF | 显示 `FCEUI_*` 废弃告警 |
| CMake `ENABLE_LINT_CPPCHECK` | `CMakeLists.txt:158` | OFF | cppcheck 静态分析 |
| **构建期** env `FCEUX11_GIT_REV` | `src/rust/crates/f11qa/build.rs:18`（→`cargo:rustc-env`，`:24`） | — | 编译期盖进 f11qa 报告的 `git_rev` 字段。**必须在构建/运行 cargo 之前设**，运行期设无效 |

### C · 上游沿用的配置类变量（非探针，登记以免漏读）

| 变量 | 位置 | 作用 |
|---|---|---|
| `FCEUX_HOME` | `src/drivers/Qt/config.cpp:449` | 覆盖应用基目录（Windows 默认是 exe 所在目录） |
| `FCEUX_CONFIG_DIR` | `src/drivers/Qt/config.cpp:450` | 覆盖配置目录（优先级高于 `FCEUX_HOME`） |
| `FCEUX_ROM_PATH` | `src/drivers/Qt/ConsoleFile.cpp:235`、`:328` | ROM 打开对话框的起始目录 |
| `LUA_PATH` | `src/drivers/Qt/LuaControl.cpp:323` | Lua 模块搜索路径 |

---

*本文件由独立 code review 产生：两份源计划已删除，本文件 + review 报告是它们的唯一继任者。*
*审查方法：4 个并行只读 agent 做逐项代码 / 提交 / 门禁取证，主会话复跑全部门禁并复核关键坐标。*
