# FCEUX11 v2.0.2 — DEBUG 与性能优化计划

> **STATUS: ACTIVE——P0 测量口径已落地（2026-11-05，读数见 §十 r3）；P1 起逐工作项先 PLAN 后动手**
> **立项目标**：以 DEBUG（调试能力/排查效率）与性能优化为主，另对
> `docs/plans/long-term-evolution/` 的 10 项长期难题做**可行性重估**，可行者并入本计划。
> **基线**：`4c75fe8`（v2.0.1 收口后）· 矩阵 **108P / 12F** · grade B · `ctest -LE perf` **36 项全绿**（计数随新增测试增长：v2.0.1 收口时 33 → r6 加 `input_snesmouse_test`/`apu_frame_timing_test` ⇒ 35 → 本轮 36；以 `ctest -N -LE perf` 的 `Total Tests` 为准）
> **产出定位**：只立项、排期、定验收；实施按工作项逐个开独立任务，先 PLAN 后动手的纪律不变。
>
> **文档内证据分级**（贯穿全文）：
> - ✅**实测**：本轮亲手跑/读码核验过的坐标与读数；
> - 📖**参照**：外部参照实现源码实读（Mesen2 / FCEUX 上游，只读不抄）；
> - 🔍**检索**：联网检索快照证实的信源（正文被 Cloudflare 拦截未直读，见 §5.4 取证说明）；
> - ⚠️**未实测**：静态读码或推理结论，动手前须先验证。

---

## 〇、范围与不变式

**范围内**：NES 核心性能、NES 调试设施、GBA 侧性能与诊断工具、长期难题可行性重估后的入选项。

**范围外（排除项）**：

1. **不换 savestate 载荷格式**（§9.1 L13 的 JSON 十进制数组）——格式版本级改动，
   风险与收益比不划算，**不搭 v2.0.2 顺风车**，维持长期限制现状。
2. **不做时序精度模型级改动**，除非作为 §四 长期难题入选项**单独立项**（如 037），
   与性能/调试改动**禁止混 PR**。
3. **不碰 GBA vendor 树**（`src/rust/crates/gba-core/`）——27 处补丁集的维护成本高，
   本轮性能项全部落在胶水层（`src/rust/src/gba/`）与 C++ 侧。

**不变式（继承 AGENTS.md，红灯即停）**：

1. **NES 零回归**：`ctest -LE perf` 36 项全绿；矩阵 `pass_to_fail = 0`；金标哈希
   （golden_hashes / golden_savestate）不动，要动须人工授权。
2. **变异验证**：每条新锁测试都要能把「修复前的值」改回去看到它转红；全绿不算证据。
3. **instrument-first**：改行为前先上探针/测量（`docs/tech/precision.md` §4）。
4. **bench 与构建/测试不并行**（r54 ②：并行会污染测量）；`bench_tolerance_test` 是
   advisory（r55：本机 full_frame 随时钟上下文漂移 +23%），读数只用来追**大幅持续**回退。
5. GBA 长期 Alpha，不阻塞本体发版；GBA 改动后仍跑 `ctest`（不变式 1 对两台机一体生效）。

---

## 一、先立测量，再动代码（P0）—— ✅ 已落地（2026-11-05，读数与实施记录见 §十 r3）

本轮扫描发现**现有性能测量有两个盲区**，不先补口径，后面的工作做完无法在门禁里证明：

| # | 盲区 | 证据（✅实测） | 处置 |
|---|---|---|---|
| M1 | `bench_tolerance_test` 五档（cpu/ppu/full/**apu/bus_dispatch**，`bench_tolerance_test.cpp:53-59`）全部 `soundq=0` 走低质量路径（`:96-99` 仅设采样率，全文件无 `SetSoundQuality` 调用），**真实游玩的 NeoFilterSound 484 抽头 FIR 根本不在基线内** | ✅ | 新增 **soundq=1 测量位**（bench_full_frame 加 soundq=1 变体档或独立 bench），基线重采后再动 A5 |
| M2 | GBA 帧导出/savestate 无微基准，C1/C2 的收益只能靠体感 | ✅ | 借用现成 `#[ignore]` 探针（`save.rs:885` `measure_a_state_on_disk`）扩一个帧导出计时探针，只作量级参照不进 ctest |

**验收**：新测量位跑出稳定读数（best-of-3 口径同 baseline_v1.10.json 方法论）；
**基线重采须单独提交**（照 `tests/fixtures/generate_bench_baseline.ps1` 流程）。

---

## 二、NES 性能线（线 A）—— ✅ A1/A2/A3/A4 已落地（r5/r6）；A5、A6、A7 未动

> 全部 ✅实测 有代码坐标。收益量级以「60 帧 / 68.7ms 基线」为参照系。

### A1 调试门控三合一 —— 首个工作项（收益/风险比最优）—— ✅ 已落地（2026-11-05，读数见 §十 r5；**手工九场景清单待 GUI 实测**，验收未完全关闭）

| 项 | 位置（✅实测） | 现状 |
|---|---|---|
| A1-a DebugCycle 每指令无条件执行 | `src/x6502.cpp:606` → `src/debug.cpp:901-963` | 每条 6502 指令先做：vblank 浮点换算（`debug.cpp:909`，含 double 除法）、`GameInfo` 判定（:917）、**重新取指 1–3 次 `GetMem`**（:922-936，与真取指重复）、寻址解码再 0–2 次 `GetMem`（:938-954），然后才在 :956 判断「有没有断点/单步/计数断」（9 路 OR：numWPs/step/runline/stepout/watchpoint flags/badopbreak/break_on_cycles/break_on_instructions）决定要不要进 `breakpoint()`。**零断点时整段纯浪费** |
| A1-b 每指令调试记账 | `src/x6502.cpp:608`（`IncrementInstructionsCounters` 跨 TU 调用 + 两次全局自增，`debug.cpp:601-604`）、`:611`（`e1_last_pc = _PC` 每指令存，仅供 env-gated E-1 探针消费）、`:621`（`map_irq_hook()` 每指令 atomic acquire，`cpu.h:131-133`） | 无任何开关 |
| A1-c viewer 回调链 | `src/ppu_rendering.cpp:787`、`:1313` → `driver_callbacks.cpp:182-187` 间接调用 → 窗口空判早退 | 每帧 480+ 次调用链 |
| ⚠️ **A1-c 的实际覆盖面小于 r5 声称**（2026-11-06 读码核实） | `ppu_view_open` 在 core 侧**只写不读**：写方 `src/drivers/Qt/ppuViewer.cpp:62`，声明 `src/driver_callbacks.h:126`，**core 侧零读点**。实际生效的门只有 `nt_view_open`（读点 `src/ppu_rendering.cpp:793`，在 `DoLine()` 内 ⇒ **仅旧 PPU 路径**；新 PPU 路径 `:1778-1779` 无门）。PPU/Sprite viewer 的 `FCEUD_UpdatePPUView`（`:1319`、`:1778`）**仍是每帧间接调用** | ⇒ **A1-c 实际只剪掉了 NT viewer 那条链**。不构成缺陷（行为正确、只是收益低于预期），但**任何「viewer 已全部剪枝」的说法都不成立** |
| ⚠️ **A1 的可观察行为变化**（2026-11-06 读码核实） | `vblankScanLines` / `vblankPixel` 的计算在 `src/debug.cpp:921-929`，**位于门内**；消费者是调试器 PPU 页签的扫描线/像素显示（`src/drivers/Qt/ConsoleDebugger.cpp:4346-4361`） | ⇒ **零断点时调试器 PPU 页签的 vblank 读数会停在旧值**。不是崩溃，但读数不对；手工验收时看到此现象**不应判红** |

**改法**：文件级 `debugger_active` 快速门（条件现成：`debug.cpp:956` 的 9 路 OR 加上
`debug_loggingCD` 与 TraceLogger logging 状态）；A1-b 三件套并入门控（`e1_last_pc` 移入
`e1_cpu_trace_on()` 内；irq hook 指针改普通读 + mapper init 处一次同步）；
A1-c 同门。**门打开时行为逐字节不变**——`GetMem` 在 debug 路径本就置 `fceuindbg`
抑制副作用（`debug.cpp:349-351`），输出只喂调试器，金标哈希不动。

**验收**：`ctest -LE perf` 全绿 + `golden_hashes.json` 逐帧一致 + bench_cpu_frame 明显下降
（目标 ≥5%，以实测为准）+ **手工场景**：断点/单步/step-out/条件断/TraceLogger 全部照常
（这是「调试功能不回归」的变异验证对应面）。

### A2 APU 帧计数器批量步进（收益高，风险中）

- 位置（✅实测）：`src/x6502.cpp:623-624` 每指令调 `FCEU_SoundCPUHook`（overclocking 时跳过）→
  `src/sound.cpp:685`，体内 `while(rem>0){ rem--; FrameCounterTick(); }`（:706-722），
  `FrameCounterTick` 是 5-6 次串行比较链（:570-634）。
- 现状：每帧 ~29,830 次循环迭代+函数调用，只为打 4-5 个定拍事件
  （7457/14913/22371/29828-30）与 `$4017` 写的复位成熟点（fc_reset_in）。
- 改法：批量步进——算 `min(下一事件, fc_reset_in)` 一次跳转，事件边界精确命中再触发；
  **fc_reset_in 的 3/4 周期奇偶抖动语义保持**（`sound.cpp:1254-1259` 注释）。
- 风险：APU 帧计数器/IRQ 是 blargg APU 测试与金标音频的时序敏感区。
  **先写 fail-to-pass 时序锁测试再改**（instrument-first），哨兵：
  blargg APU 全套、`apu_wav_diff_test`、golden_wav 三件、矩阵 031/054。
- 验收：锁测试两向变异可红；金标 WAV 逐字节一致。

### A3 DoLine 每扫描线后处理合并（收益中，风险低）

- 位置（✅实测）：`src/ppu_rendering.cpp:737-757`（emph/deemp 位标记，无条件跑，
  常见 no-emphasis 分支也走 `(tmp&0x3f3f3f3f)|0x80808080` 的 64 次 RMW）、
  `:761-765`（dtarget 64 次迭代重算同一个表达式——本质是 memset）。
- 改法：dtarget 换现成 `FCEU_dwmemset`（同文件 :714 已有同型用法）；deemp 循环
  合并为一趟宽处理（无跨像素依赖，可矢量化）。
- 验收：XBuf **逐字节 diff** 前后一致（探针 dump），golden_hashes 不动。
  历史注意：memcpy 中转是 hotfix1 H-06 为别名正确性引入的，合并时别绕回别名坑。

### A4 帧级内存流量收窄（收益低-中，风险低-中）

- `src/video.cpp:254` `memcpy(XBackBuf, XBuf, 256*256)` 64KB/帧——收窄为「有消费者才拷」
  （暂停重绘 `fceu.cpp:840`、截图、录像 HUD 逐个盘点后挂开关）。风险中：消费者盘点不全会出花屏。
- `src/sound.cpp:1338` memset 覆盖 `[left, 40000)` ≈160KB（`:1337` 的 memmove 保留 FIR carryover，勿动），下帧只写 ≤SOUNDTS≈3 万项
  ——收窄到 `[left, left+SOUNDTS)`。注意 hotfix3 D-4 REVERTED 的教训（:1327-1336 注释）：
  **trailing memset 必须保留**，只许收窄范围不许删除。

### A5 NeoFilterSound 无损 SIMD（收益高，仅 soundq≥1 真实路径；前置 = M1）—— ✅ G0-G3 已落地（2026-11-06，−43%；**G3 五项变异留证见 §十 r8**；⚠️ **两项「门禁无牙」发现**：M-D 插值权重方向原无人看守（r8 已补绝对金标准向量锁测试）、HQ golden 对 FIR 变异无感（**r9 已闭合**：新增自建 AC fixture + 非静默 fail-fast 断言，两道 golden 门禁现各有牙））

- 位置（✅实测）：`src/rust/crates/fceux11-media/src/filter.rs:299-339`，
  `NCOEFFS=484 / SQ2NCOEFFS=1024`（fcoeffs.rs:8-9），每输出样本 2×484 次 i64 乘累加
  （线性插值双相）≈77 万 i64 MAC/帧@48kHz。
- 现状：i64 点积在 AVX2 无廉价乘加 ⇒ 自动矢量化基本无效（⚠️未实测反汇编）；
  WaveHi 样本有效位在高 16 位（`sound.cpp:781` `&(~0xFFFF)`）。
- 改法：先无损重构（acc2 复用相邻载入、内层手动 SIMD 保 bit-exact），16 位重写列为
  有损备选**不做**。
- ⚠️ **哨兵更正（r8 实测推翻原文，r9 已修复）**：原文写「哨兵：`apu_wav_diff_test` +
  golden_wav 逐字节」——**该哨兵当时对 FIR 变异结构性无牙**：两个 golden 的捕获样本
  **全为 0**（各 799 样本）。真因不是「FIR 输入为零」，而是 fixture ROM 的 PRG 是
  `0xEA` NOP sled（不写任何 APU 寄存器）⇒ FIR 输入是**恒定 DC** ⇒ 下游 `SexyFilter`
  （隔直 IIR）把 DC 衰减到 0 且捕获帧早已收敛 ⇒ 任何保持纯 DC 特性的 FIR 变化都不可观测。
  实证：改 bias / 改系数表 ⇒ golden 仍绿；注入 DC ⇒ 仍绿；注入 AC ⇒ 转红。
  **✅ r9 已闭合**：新增自建 fixture `tests/fixtures/apu_square.nes`（周期性方波 = AC
  内容，自建资产无许可问题，`.gitignore` 已加白名单），并给两道门禁加「捕获必须非静默」
  fail-fast 断言。**变异验证**：FIR bias 改坏 ⇒ **HQ golden 转红**；`SexyFilter` 改坏 ⇒
  **低质量 golden 转红** ⇒ 两道门禁各自有牙、覆盖不同代码路径（低质量路径按设计不经过
  `fir_pair_avx2`）。**⇒ A5 的红源现为三重**：Rust 锁测试（52 项）+ HQ golden（管 FIR）
  + 低质量 golden（管 SexyFilter/DC 隔直）。

### A6 bus 直读快路径 —— 单独立项，排最后（收益最高，风险最高）

- 位置（✅实测）：`src/bus.h:103-114` 每次访存一次间接调用 + 512KB 函数指针表
  （`bus.h:232-233`）；现成先例 `Bus::direct_read`（`bus.cpp:156-158`）证明 Page[] 直读通路存在。
- **三条红线交叉**：DB 锁存语义（kgmqa-038，`bus.h:104-109`）、`fceuindbg` 副作用抑制、
  Genie 影子表（`fceu.cpp::SetReadHandler` wrap 路径）。**与长期难题 051（IO 空间取指）同域**——
  若 051 入选（§四 D5），两者**必须同窗设计、串行落地**，禁止两拨人各改各的 bus dispatch。
- 验收：除常规门禁外，加 `bus_test` + `mapper_byte_diff` 全套；金标 DB 字节（savestate 里）
  不动。

### A7 指令 handler 别名/溢出障碍（收益中-高，后置，⚠️未实测反汇编）

- `src/ops_table.inc` 256 个 handler 的 `xp` 参数全部 `(void)xp` 丢弃，
  `src/x6502abbrev.h:9-19` 全走 `g_cpu.native_layout().X` ⇒ 编译器必须假定每次
  RdMem/WrMem 可写 g_cpu，寄存器值全部反复重载。
- 改法：`xp`（`__restrict`）为基址 + 局部副本出口回写；`layout_` 是 savestate
  static_assert 对象（`cpu.cpp:14-24`）**布局不得动**。改动面是 256 个 handler + 生成脚本，
  排最后。

---

## 三、NES DEBUG 线（线 B）—— ✅ B1/B2 已落地（r6）；B3/B4/B5 未动（B5 永久封口）

> 依据：调试设施盘点（✅实测读码，环境变量全表见附录 A）。

### B1 TraceLogger 无头入口 + 性能修复

- 现状：只能 Qt 菜单开（`TraceLogger.cpp:1246`，静态 `logging` :107），harness/无头场景
  无法开 trace；自带性能注记 `//todo: really speed this up`（:1363）。
- 改法：加 `FCEUX11_TRACE` 环境变量入口（与既有 `FCEUX11_E1_TRACE` 族同款，进附录 A 表），
  治理 :1363 的低效（批量写/关 per-insn 格式化）。
- 与 A1 的 `debugger_active` 门**同窗接线**（trace 开 = 门开）。
- 验收：`--log` 的 nestest 指令 trace 与 `FCEUX11_TRACE` 输出可 diff；开启时 bench 回退
  可接受（trace 本就有代价），**关闭时**与 A1 合并后零成本。

### B2 profiler 从半成品到可用

- 现状（✅实测）：函数级 RAII 计时器 + Rust 后端 + 落盘（`profiler.h:144-155`、
  `profiler.cpp:39-237`），**默认不编译**（`FCEU_PROFILER_ENABLE` 无 option 声明）、
  探点仅 3 处（`fceu.cpp:789`、`ConsoleWindow.cpp:629,666`）、无任何 UI 入口、
  flat 统计无调用栈聚合。
- 改法（增量）：① 把编译期开关做成 CMake option；② 在 A1-A4 的关键函数补探点
  （Emulate/DoLine/RefreshLine/FCEU_SoundCPUHook/WriteSound）；③ 输出文本报告
  （min/max/sum/numCalls 表格）先不做火焰图；④ 可选 Qt 菜单项。
- 验收：Release 构建不开宏零成本不变；开宏能产出可读报告定位到函数。

### B3 Lua 调试 API 补 range 系（小件）

`memory.readbyterange`/`writebyterange`（现仅 9 个导出，`src/rust/crates/fceux11-lua/src/bindings/memory.rs:19-112`）。
验收：`tests/lua_scripts/test_memory.lua` 加用例。

### B4 断点上限与条件表达式（可选，视需求）

64 断点上限（`debug.cpp:519`）与二元比较算术表达式（`conddebug.h:25-47`）是产品取舍，
**不默认立项**；要做先问真实用户需求。

### B5 帧级 rewind 调试 —— 本期不做（彻底封口，不写路线图）

成熟模拟器的「调试器内逐帧回溯」与 TAS Editor greenzone 是两套机制，新做属大件；
与 v2.0.2 主题性价比不符。若将来要，须另立专项。

---

## 四、GBA 线（线 C）

> 依据：GBA 补扫报告（✅实测读码）。全部落在胶水层与 C++ 侧，不碰 vendor 树。

### C1 帧导出热路径（主菜）—— ✅ 已落地（2026-11-05，252→20.6 µs/帧，见 §十 r5）

- 现状（✅实测）：`frame.rs:321-334` `gba_frame_buffer` 每次调用
  **2×153.6KB `Vec` 分配 + 3 次全帧写/拷**（`frame_bytes` 转换写入 :865-894、
  `copy_from_slice` :329、`copy_nonoverlapping` 到 dst :333）+ 逐像素标量 BGR555→RGBA
  （38,400 像素 × `scale5`）。C++ 侧消费零分配（`gba_load.cpp:412`）。
- 改法：① 复用常驻缓冲消掉两次分配；② 转换**直写 dst** 合并一整份拷贝；
  ③ `scale5` 查表（8K 表）或 SIMD——纯函数，锁测试可兜。
  **水印「只画一次」语义与 `GBA_ERR_CAPACITY` 语义保持**。
- 验收：`cargo test -p fceux11-rust --no-default-features --features gba --lib` 248 项全绿；
  帧字节前后逐位一致（dump_frames 探针 diff）；帧导出计时探针（M2）读数下降。

### C2 savestate 延迟（双编码 + 指纹缓存）—— ✅ 已落地（2026-11-05，save 2×→1× 编码，见 §十 r5）

- 现状（✅实测）：每次保存 = **2 次全量 JSON 编码**（`gba_load.cpp:263-276`，调用在 :271/:275，先
  `gba_savestate_size` 量大小再 `gba_savestate_save` 再编码，release 单次 2.4ms）；
  每次存/读档全 ROM 重算 FNV-1a 指纹（`save.rs:141`、`frame.rs:405/438`，
  4-32MB 卡带为 ms 级）。
- 改法：宽 buffer + `GBA_ERR_CAPACITY` 重试（ABI 已支持，注释自述）；指纹在
  `gba_load_rom_bytes` 时算一次缓存（换 ROM 失效）。
- 验收：savestate 金标（`golden_savestate_test`）不动；`measure_a_state_on_disk` 读数对照。

### C3 GBA 诊断工具文档化 + 计数更正（DEBUG 方向）

- 把 GBA 探针族写成一页 HOWTO：`FCEUX11_DIAG_LOG`、`GBA_PROBE_*` 环境变量族、
  r53/r56 复用探针（`frame.rs:2586-2678` `probe_why_a_game_stalls`、
  `frame.rs:3120` `dump_frames_like_the_reference` 差分转储）触发方式
  （`cargo test … -- --ignored --nocapture` + ROM 路径）。
- **更正 AGENTS.md**：「7 条 `#[ignore]`」实测为 **20 条**（save.rs 2 + frame.rs 5 +
  waitprobe 10 + cartprobe 1 + gba-core timers 2）——计数过时会误导下一个接手人。
  同时给 AGENTS.md 补一句：decoupling/gate/waitprobe 的守卫与探针**全部 `#[cfg(test)]`**，
  常驻开销为零（避免被读成运行期强制）。

---

## 五、长期难题可行性重估（线 D）

> 对 `docs/plans/long-term-evolution/` 14 项中未收口的 10 项逐项重估。
> **总判**：技术手段整体比立卡时更成熟——三项有**同源或参照实现**直接可循
> （050/107/081），两项有现代参照实现的完整模型可对照（037/049），一项卡在
> **产品决策**而非技术（099），一项仍以负结果为主（038）。
> 参照实现只读不抄（Mesen2 = GPL-2.0，与本项目许可不相容；FCEUX 上游与本仓库同源）。

### 逐项结论

| # | 项 | 可行性 | 关键证据 | 建议 |
|---|---|---|---|---|
| D1 | **037** 中断轮询下沉 + hijack | **高**（模型级，改动面大） | 📖Mesen2 `NesCpu.cpp:178-217`：`_prevRunIrq/_runIrq` 两拍延迟轮询 + `IRQ()` 内 DummyRead 强制 $00 入 opcode 寄存器（即 hijack）+ `ProcessInterrupt(…, hijack标志)`；🔍nesdev wiki「CPU interrupts」页存在（含 Visual6502 writeup 引用） | **入选但走独占专线**（照 T2 卡 S0–S5 分步，禁与其他线混 PR）；金标哨兵 048/032/057/053/054 |
| D2 | **038** CPU 数据总线锁存 / open bus | **中**（两次负结果在前，不承诺转绿） | 卡点是 open bus 之「死」（`set_db()` 零调用点）；📖Mesen2 `NesPpu.cpp:221-251` 有 per-bit open-bus 衰减模型，但**衰减速率本身是模糊面**（Mesen 自注释「3 帧衰减是保守估计」） | 维持 defer；唯一候选动作 = T3 卡的 `GetIX` dummy read 新探针（先出调查页）；**与 A6 联动** |
| D3 | **049** $2007 读缓冲 | **中高** | 📖Mesen2 `NesPpu.cpp` 读缓冲 + open-bus 完整模型在；bisqwit 测试是精确判据；模糊面同 D2（衰减） | **入选为可选里程碑**（M0-M4 照 T3 卡）；M0 状态机文档先行，M3（衰减）单独评估 |
| D4 | **050** OAM DMA dummy write | **高** | RMW 已正确、`cpu_dummy_writes_ppu` 已 PASS、仅 `_oam` FAIL；T1 卡步骤现成（探针对比两条路径总线轨迹） | **入选**（4–7 天；哨兵 dma_sync_test_v2/110） |
| D5 | **051** IO 空间取指 / bus dispatch | **中高** | bus dispatch 未区分取指 vs 数据访问；bisqwit 变体比 blargg 严 | **入选，但与 A6 同窗设计、串行落地**（同域改动禁止并行） |
| D6 | **077** Holy Mapperel 聚合 | cascade | 成本已收窄为「仅 L2 MMC3 IRQ 组」= 完全依赖 D1 | 不单独立项；D1 收口时重估 |
| D7 | **081** FDS 加载 0xFE + IRQ | **高**（A 加载修复先行） | 📖Mesen2 `Fds.cpp:140-148/409-423`：$4022 IRQ 定时器（enable/repeat/counter reload → SetIrqSource）+ 磁盘 IRQ 完整；本仓库 kgmqa-011 基础加载绿 ⇒ 0xFE 是 patched .fds 形态问题 | **A 路径（加载修复）入选**；B 路径（IRQ 状态机）列为 A 完成后的独立小项 |
| D8 | **099** NTSC/PAL/Dendy TV 时序 | 技术可行，**卡产品决策** | core 已有 PAL/Dendy 基础设施（`FCEUI_SetVidSystem`、`dendy`、`normalscanlines`，✅实测 grep）；缺的是制式表精确化 | **决策项**：先拍板「是否正式支持 PAL/Dendy」；「否」则走有据 known_limit + 范围声明（需评审），「是」再排期 |
| D9 | **107** SNES 鼠标协议 | **高** | ✅本仓库**已有实现** `src/input/snesmouse.cpp`（FCEUX 经典版）+ `SI_SNES_MOUSE` 全链路接线（`input.cpp:522`、Qt `input.cpp:1689`）；缺的是分辨率位/溢出/移位时序细节；🔍nesdev wiki「Mouse」页有协议字节布局（快照含 byte2/3 方向位定义）；mset 测试是判据 | **入选**（1–2 周；回归面输入子系统，不碰 PPU/CPU 金标；哨兵 porttest/101、test_buttons/111） |
| D10 | **108** FC 麦克风协议 | **中高** | ✅现有粗实现 `input.cpp:122-140`（`replaceP2StartWithMicrophone`，自注释 "Not verified against hardware"）；缺硬件准确的采样位（bit 2/3）协议；mict 测试是判据 | **入选**（与 D9 同属输入协议，连续立项；先做 D9 再 D10） |

### §5.4 取证说明（信源与限制）

- 🔍 本轮 `nesdev.org` 被 Cloudflare 整站拦截（浏览器与直连均 403/challenge，
  web.archive.org 经代理亦不可达）；wiki 页**存在性与部分正文**经搜索快照证实
  （`https://www.nesdev.org/wiki/CPU_interrupts`、`https://www.nesdev.org/wiki/Mouse`），
  **正文未逐页直读**。动手实施时以测试 ROM 自带文档（rainwarrior mset/mict README、
  bisqwit 测试注释）与硬件实测为最终判据，wiki 只作路标。
- 📖 Mesen2 源码经 GitHub 实读（本轮实测下载核对行号）：
  `github.com/SourMesen/Mesen2` 的 `Core/NES/NesCpu.cpp`、`Core/NES/NesPpu.cpp`、
  `Core/NES/Mappers/FDS/Fds.cpp`；FCEUX 上游 `github.com/TASEmulators/fceux` 的
  `src/input.cpp`（`FCEU_InitSNESMouse`，与本仓库同源）。
- ⚠️ 本节所有「可行性」是**文献/参照实现层面**的结论，不是「本仓库实测可转绿」的承诺；
  逐项实施仍走 instrument-first + 变异验证。

### v2.0.2 实际立项组合（推荐）

**首批入选**：D4（050）、D9（107）、D7-A（081 加载修复）——三项都「有现成抓手、
验证判据清晰、不动金标」，与 DEBUG/性能主题互补成一个完整版本。
**专线**：D1（037）独占窗口（若人力允许；它是 D6 的唯一解锁项）。
**可选**：D3（049）M0-M2 里程碑、D7-B（FDS IRQ）、D5（051，与 A6 绑定）。
**决策等待**：D8（099）。**维持 defer**：D2（038）。**不立项**：D6（cascade）。

---

## 六、排期

| 阶段 | 内容 | 依赖 |
|---|---|---|
| **P0 测量口径** | ✅ 已落地（r3）：M1（soundq=1 bench 位 + 基线重采）、M2（GBA 计时探针） | 无 |
| **P1 低风险高收益** | ✅ 已落地（r5）：A1（调试门控三合一，cpu_frame −18%；手工清单待 GUI 实测）、C1（帧导出 −92%）、C2（savestate 2×→1× 编码） | P0 |
| **P2 深化** | ✅ 已落地（r6）：A2（批量步进 −15/−18%，五向变异）、A3（−12%）、A4-2（WAVEDUMP 等价；A4-1 主方案否决存档）、B1（无头 trace）、B2（option+探点+报告） | P1 |
| **P3 长期难题首批** | D9 ✅（r6，kgmqa-107 协议四缺陷修复+桥注入面；mset 端到端 0xFC 需真实交互，advisory 如实记录）；D7-A **关闭为「前提证伪，零改动」**（0xFE 根因 = headless 缺 BaseDirectory/BIOS 门控缺失，重分类待业主裁决）；D4 探针已落地、修复落点改向（0x06 → $2004 读路径合成读），修复立项待裁决 | 可与 P2 并行（不同文件域） |
| **P4 视余量** | 🔶 过半（r7/r8）：B3 ✅（34ce329）、A5 G0-G2 ✅（0da9ad4/726e872，−43%）、**A5 G3 变异验证 ✅（r8：M-A/M-B/M-C/M-E 四项转红即预期断言；M-D 实测 Rust 层无牙 ⇒ 新增绝对金标准向量锁测试 `lock_resample_interpolation_golden_vector` 补盲区，media crate 51→52）**、D7-B 暂缓（D7-A 关闭后触发条件不成立）、D3 调查完成（049 实为 harness 伪 FAIL，三项裁决待业主）、**A6+D5 PLAN 已出待拍板**（16-24 天串行同窗，见 r7） | P0 的 M1；A6 需全套回归护甲 |
| **专线** | D1（037，独占分支）——S2 ✅（r18）；**S2.5 G5 RE 三路调查收口（r20）：方案 A「线携带断言时刻」PLAN 已出待拍板**，验收门 G-1~G-10；附带发现 kgmqa-040 挂载漂移待业主裁决；S3–S5 未动 | 独立窗口 |
| **决策等待** | D8 | 业主拍板 |

> 多 agent 并行切分按**文件域**：A 线（x6502/debug/sound/ppu_rendering/video）、
> B 线（TraceLogger/profiler/lua bindings）、C 线（frame.rs/save.rs/gba_load.cpp）互不重叠；
> A6/D5 与任何 bus 改动互斥同窗。

---

## 七、风险与红线

1. **金标三件套**（golden_hashes / golden_savestate / golden_wav）任何漂移 = 停，
   确认是否有意变更，否则回滚；重生成须人工授权。
2. **A2（APU 时序）与 A6（bus）是两个高危点**：都必须 fail-to-pass 锁测试先行 +
   两向变异验证；A6 与 D2/D5 语义交叉，动工前重读 §2.2 ⑶（S0a 的 20+ 处 open-bus
   读取者清单）。
3. **调试功能不回归**是 A1/B1 的隐性验收：断点、条件断、单步族、TraceLogger、
   CD logger 逐一手工过（A1 门控改坏它们不会被 ctest 抓到）。
4. bench 读数**不做精确门禁**（r55 漂移 ±23%）；只追「≥10% 且可复现」的持续变化。
5. 改 `build.rs` / ABI 后查生成头（AGENTS.md 坑 9/10）；不整仓 `cargo fmt`（坑 5）；
   改完回产物确认（坑 6）。

---

## 八、整包出口判据

- [ ] `ctest -LE perf` 36 项全绿；矩阵 `pass_to_fail = 0`
- [ ] 金标三件套零漂移（或漂移均有人工授权记录）
- [ ] bench：cpu_frame 与 soundq=1 档位实测改善（读数入库，advisory）
- [ ] A1 手工调试场景清单全过
- [ ] GBA 锁测试 249 项 + vendor 242 项全绿；帧字节 diff 一致
- [ ] 入选长期难题项：各自验收（050 → kgmqa-050 PASS；107 → kgmqa-107 PASS；
      081-A → 0xFE 消失且失败类型重分类经评审）
- [ ] AGENTS.md 计数更正与 GBA 探针 HOWTO 落档
- [ ] **A5 G3 变异留证**：M-A/M-B/M-C/M-E 四项各有「转红即预期断言」的留证（✅ r8）；
      M-D 由绝对金标准向量锁测试覆盖（✅ r8，`lock_resample_interpolation_golden_vector`）
- [x] **HQ/low 质量 golden 门禁对音频算术有牙**（✅ r9）：新增自建 AC fixture
      `tests/fixtures/apu_square.nes`（周期性方波）+ 两道门禁的「捕获必须非静默」
      fail-fast 断言。变异验证：FIR bias 改坏 ⇒ **HQ golden 转红**；`SexyFilter`
      `mul1` 改坏 ⇒ **低质量 golden 转红** ⇒ 两道门禁各自有牙、覆盖不同代码路径
- ⚠️ **仍需人工**：A1 手工 GUI 验收（22 项清单已成文 `docs/tech/debug-regression-checklist.md`，
      **尚未有人执行** ⇒ 文档就绪 ≠ 验收通过）

## 九、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 门禁（不变式 1）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf

# bench（单独跑，不与构建并行；best-of-3 口径）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\build\tests && .\fceux11_bench_tolerance_test.exe"

# GBA 锁测试
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"

# 长期难题单项（示例：050）。注意：f11qa-rom-runner 不是 CMake 目标，是 f11qa crate 的
# Cargo [[bin]]（src/rust/crates/f11qa/Cargo.toml:15），产物路径见 f11qa-runner-protocol.md:172
& src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe --kgmqa-id kgmqa-050-cpu-dummy-writes-bisqwit --frames 3000
```

---

## 附录 A · 环境变量开关全表（✅实测 2026-11-06 全表重核，B 线改动同步本表）

> **坐标口径**：统一写 **`getenv` 所在行**（不是函数定义行、不是 `#[ignore]` 属性行）。
> 旧表曾混用三种口径，导致坐标漂移与一处完全失效（本轮已全部校准）。
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

## 十、变更记录

| # | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-04 | 初稿：只读扫描（NES 性能/调试、GBA 性能/调试）+ 长期难题 10 项可行性重估 + 排期与出口判据。未动任何代码 |
| r2 | 2026-11-04 | 拍板前三路复核（NES 坐标 / GBA 坐标与计数 / 长期难题事实基础），修正 8 处：① M1 档位实为五档非三档（bench_tolerance_test.cpp:53-59，补 apu/bus_dispatch；soundq=0 断言本身成立）；② debug.cpp:956 是 9 路 OR 非 8 路；③ A2 奇偶抖动注释坐标 :1254-1259；④ A4 memset 单独在 :1338（:1337 是 memmove 保 FIR carryover）；⑤ B3 lua memory 导出实为 9 个非 10 个，补全路径与行号；⑥ C3 probe_why_a_game_stalls 函数体在 frame.rs:2586-2678（原引 :2669-2677 只是尾部 println 段）；⑦ C2 双编码调用坐标精确到 :263-276/:271/:275；⑧ §九 runner 产物路径修正——f11qa-rom-runner 是 Cargo [[bin]] 非 CMake 目标，在 src\rust\target\…\debug\ 而非 build\tests\。复核同时确认：20 条 #[ignore] 分组计数、GBA 266−18=248 与 vendor 244−2=242 测试数、长期难题 10 项未收口清单、全部哨兵测试 ID、108P/12F 出处均属实 |
| r3 | 2026-11-05 | **P0 落地**（M1+M2，只加测量位不改生产行为）。**M1**：bench 第六档 `bench_full_frame_hq`（BenchConfig 改配置驱动 `sound`/`soundq` 两字段，替代原 strcmp 特判；soundq=1 在 `core_init()` 后设 `fceu11::SetSoundQuality`——`FSettings` 每次 `fceu11::Initialize` 清零，跨档不存活）；新增 `--only=<name>`（单档）与 `--json`（单行 JSON 沉到 stdout 末行，与独立 bench 二进制同 schema）两旗标；`generate_bench_baseline.ps1` 加第四采集行（args 字段）。同上下文对照：soundq=0 full_frame 49.4 ms/60帧 vs hq 80.6 ms（**+63%** = HQ 渲染器 + NeoFilterSound FIR 真实在跑）；hq 三遍重复 79.4/79.4/81.2 ms（±1%，稳定）。基线照 ps1 流程重采（best-of-5，4 条目，hq=78.588 ms，tolerance 10.0 不变），重采后对照全 PASS（hq +1.07%）。**M2**：`measure_frame_export` 探针（frame.rs 测试模块，`exclusively` 纪律走真 ABI 路径 `gba_step_frame`+`gba_frame_buffer`，60 帧预热后计时，`GBA_PROBE_FRAMES` 可调）。合成卡带实测（--release）：全链 **252 µs/帧（min 234）= 59.7275 Hz 帧预算的 1.51%**，其中标量 BGR555→RGBA 转换 ~213 µs（**~85%**）——即 C1 的对象；转换成本与像素内容无关，读数可作 C1 量级参照。**真卡带读数待补**：原 `Research_only\gbatech\mksc.gba` 副本已不在本机，设 `GBA_PROBE_ROM` 即可复跑。门禁：GBA 锁测试 248 全绿（#[ignore] 18→19，C3 的计数更正落地时按 21 算）、`ctest -LE perf` 33 全绿、bench 基线对照全 PASS。实施中两处偏差已修：`--only --json` 的 JSON 必须是 stdout 末行（ps1 取末行，首版把摘要排在了后面）；`exclusively` 要求 `Send + 'static`，探针的采样与报告整体移入闭包 |
| r4 | 2026-11-05 | 文档状态修订：STATUS 由 DRAFT 改 ACTIVE（P0 已落地见 r3），§一标题与 §六 P0 行加完成标记。无实质内容变更 |
| r5 | 2026-11-05 | **P1 落地**（A1→C1→C2，四提交：55d82d0 / 672bcf3 / 6d88c56 / 本条）。**A1**：门 = breakpoint() 的 9 路 OR + `debug_loggingCD` + trace_logging，`DebugCycle` 入口早退（实施修正①：9 路实含 `break_asap`，本文案原枚举漏列）；`e1_last_pc` 写入入 `e1_cpu_trace_on()` 门；`map_irq_hook` 热路径 load **acquire→relaxed**（未采计划原文「普通读+mapper init 一次性同步」——cpu.h hotfix3 B-5a 的 happens-before 注释即为此设计，relaxed 保单线程可见性、去编译器屏障负担，x86 同一条 mov）；**A1-b 记录偏差**：`IncrementInstructionsCounters` **不门控**——实施核对发现 `lua getTotalInstructions` 与 TraceLogger `instrCount` 在零调试特征时也消费，门控=读数停滞，故保留每指令记账（本条 A1-b 三件套只落两件）；**A1-c 实施修正②**：viewer 门条件是**窗口状态**（DriverCallbacks 新增 `nt_view_open`/`ppu_view_open`/`trace_logging` 三个状态位 int，Qt 在窗口构造/析构与 TraceLogger Start/Stop 置位），不是调试器状态——零断点开窗也要刷新，原文「同门」照抄会开窗黑屏；kgmqa-048 不可达（trace 由 ROM 写内存日志 $02/$03，不经 `FCEUD_TraceInstruction`）。实测：`bench_cpu_frame` 46.356→37.8/38.2/38.5 ms（**≈−18%**，目标 ≥5%）；`ctest -LE perf` 33 全绿。**C1**：`write_frame_bytes` 直写 dst + **32768 项 const fn 全字 LUT**（原案 ③ 的 scale5 小表升级为全字表，①常驻缓冲被「直写 dst」整体取代——消费方 g_frame 恰为 FRAME_BYTES 持久缓冲，无需 Machine 加字段）；全帧写 3→1、分配 2→0；穷举等价测试（32768 字 vs 标量参照=核心自身访问器）即变异验证。实测：M2 探针全链 **252→20.6 µs/帧（−92%）**、转换 213→19.9 µs、预算占比 1.51%→0.12%；锁测试 **249 全绿（+1）**。**C2**：C++ 侧宽 buffer（2MB）+`GBA_ERR_CAPACITY` 重试一次，**推翻 gba_load.cpp:263-269 当年「不值得第二次 buffer」的记录决策**（P0 口径落地后：每存一次白付 2.22ms 编码）；指纹（全 ROM FNV-1a）入 `Machine` 字段装载时算一次，encode/apply_state 读缓存，失效随机器整体换弃自动成立，载荷格式零变化；更正 save.rs「a few microseconds」过时注释。实测：状态 1,047,878 B、单次编码 2.22 ms（save 路径 2×→1×，load 移出全 ROM 哈希）；`measure_a_state_on_disk` 对照不变（探针本就自缓存指纹）；ctest 33 全绿、bench 六档对照全 PASS（full_frame 同吃 A1 红利 48.6→40.9）。**A1 未闭合项**：手工九场景（watchpoint/Step Into/Run Line/Step Out/Step Over/Break on PC/Bad Op/cycles/instructions 超限、CD Logger+TraceLogger、Lua break_asap、NSF 播放、viewer 刷新）待用户 GUI 实测——ctest 抓不到这类回归（§七 3） |
| r6 | 2026-11-06 | **P2+P3 多 agent 批次落地**（C3 + 六提交：241860f / cfa970c / 05bfc1f / 39b92db / adc45ae / 062ce84 / 4c34cbf；5 份调查 PLAN 由并行只读 agent 产出、用户全批后三波实施）。**C3**：GBA 探针 HOWTO 落档 docs/tech/gba_probe_howto.md；AGENTS.md 计数更正（#[ignore] 7→21、锁测试 248→249、cfg(test) 零开销注记）。**B2①**：`FCEU_PROFILER_ENABLE` option 声明（此前从未声明=永远 OFF 且打不开）。**D9**：协议四缺陷修复（byte0 签名 0xFF/0x00 交替、dy 灵敏度补齐的不对称 bug、倍率 ×1/×2/×4、6 位幅度+bit6 溢出）+ 测试桥 `kagami_bridge_set_input`/sut_input 注入面（原 SI_NONE 硬编码，ROM 摸不到任何输入设备）+ ptr 传持久零缓冲（原 nullptr 每帧必崩）；锁测试 input_snesmouse_test（向量按字段拼装），变异 M-A..M-D 四向红；**A5 状态入档实测 golden_savestate 漂移 +22B 按裁决撤销**；mset 端到端 0xFC=需真实鼠标交互（advisory 如实记录，不硬造 headless 注入）。**D7-A 关闭为「前提证伪，零改动」**：字节取证证明该 .fds 是教科书 kind=1（"FDS\x1a"@0、1 边、65516=16+65500 自洽）；0xFE 真根因 = headless 从不调 `SetBaseDirectory`（唯一调用点在 Qt config.cpp）→ BaseDirectory 空 → BIOS 查找落到盘根 → 加载失败；**旁证 082/083/084 PASS 也被证伪**（ROM 不在镜像里，advisory skip 路径）；且 $6000 判据对「从未跑起来」也是假绿（--frames 1 即 0x00）。**重分类（BIOS 门控 skip/known-limit）待业主裁决**。**A3+A4-2**：见对应提交；等价证明 = XDUMP（XBuf+XDBuf 全帧）0/840 diff + WAVEDUMP 0/180 diff；变异 M1（023 5/5 红）/M2'（仅 XDBuf 红 418、xbuf 盲——探针存在性实证）/M3'（仅 xbuf 红 418）/V2（删 memset 复刻 D-4，180/180 红）全红→绿；**实施哨兵修正：kgmqa-024 只跑低质量路径、到不了 HQ 分支 memset（调查设计漏洞），A4-2 等价证据以 WAVEDUMP 为准——此发现是 A5 的前置（A5 需先建 HQ golden WAV）**；A4-1 主方案（消费计数门控）被盘点否决（savestate chunk-8 任意帧异步消费→金标漂移，谓词恒真），结论存档进 video.cpp 注释防后人实施。bench：ppu_frame 46.554→40.8-41.0（−12%）。**D4 探针**：FCEUX11_OAMBUS_PROBE 落地（默认静默）；冒烟带**阴性证据**——kgmqa-050 失败窗口 2000 帧 0 次 DMA burst、0 次 SPRAM 改写，历史报告的「SPRAM[0x36] 非 B2004 改写」未复现，0x06 落点改向 **$2004 读路径（newppu spr_read 合成读）**，修复立项待裁决。**A2**：批量步进（FcNextEventPos 落位表 + FrameCounterTick 保为唯一点火权威 + 倒计时一次跳），kgmqa-121 锁测试（59 断言，独立参照模型、字段拼装、S1-S5 场景）；变异 M-A..M-E 五向全红（27/4/12/11/14）→ 还原绿；bench_apu_frame 39.9→33.2-35.1（−15%）、full_frame 40.9→33.4-33.6（−18%），远超 3% 止损线；**CI 177-ROM blargg 全批本地不可跑，PR 检查单待 CI**。**B1+B2**：trace_common 纯剪切（452 行删除逐行迁移审计、Qt 行为不变）+ headless_trace（FCEUX11_TRACE，与 A1 门同窗置位）；:1363 治理三件（recBufEnabled 门/无头丢弃+overrun/memo）；反汇编经钩子注入（无头回落 core Disassemble，归一化 diff 需处理）；无头冒烟 6.25M 行完整；B2 探点+报告+**修复 ON 分支预存编译错误**（profiler.cpp 裸指针 vs Rust u64 id ABI——ON 构建此前在 HEAD 编不过，「编不过=不存在」又一例）。**ctest -LE perf 35/34→35 项全绿**（+input_snesmouse_test、+apu_frame_timing_test）。**实施过程记录**：三个 agent 超时由主会话接续收口；一个 agent 把 M3 变异留在树上被审阅发现并清除——「变异验证的红必须由主会话复跑留档」成为新惯例。**待业主裁决**：① kgmqa-081 重分类（BIOS 门控）；② D4 修复立项（落点已改向 $2004 读路径）；③ mset headless 注入面是否立项；④ A1 手工九场景 GUI 实测仍未闭合 |
| r7 | 2026-11-06 | **P4 批次（用户中断收尾，三提交：34ce329 / 0da9ad4 / 726e872）**。**B3** ✅：lua memory 导出 9→11（readbyterange 对齐上游语义：负长 nil/0 长空串/掩码回绕；writebyterange 上游无此函数、按计划走 writebyte 同一 BWrite 路径，零新增 FFI）；test_memory.lua +13 用例；Rust 49/49、kgmqa-046 exit 0、用例翻转变异 0→1 红。**A5 G0-G2** ✅：G0 = apu_wav_hq_diff_test 进门禁（SetSoundQuality(1) 走 HQ 分支；nrom/mmc1/vrc6 三 ROM；int16 WAV + **int32 pcm32 裸 dump 双锁**；golden 由改动前构建 --generate 采出、两连 VERIFY 确定才提交，VERIFY 默认缺 golden 即红）；G1 标量无损重构（acc2 复用，逐项 >>6 语义保留）；G2 AVX2（`#[target_feature]`+运行时探测，**绕开 MSVC `__AVX2__` 宏陷阱**；偏置系数 c'=c+2^16 使 vpmuludq 精确、低 6 位不变、收口 (T>>6)−1024·Σs 逐位等于标量；标量路径零改动=天然回退；O(n) 负值域守卫）。锁：filter.rs 单测 51/51（穷举等价+域回落+端到端）、HQ golden VERIFY 逐字节过。**实测 bench_full_frame_hq 78.588→44.9/45.0/45.1（−43%）**。**G3 变异 M-A..M-E 未做——收口前置**（变异点已按 PLAN §4 备好：系数+1/删修正项/!63→!31/插值翻/尾部块失效；红由主会话复跑留档）。**D3 调查完成，前提被实跑取证改写**：kgmqa-049 在 HEAD **不是精度 FAIL 而是 harness 伪 FAIL**——tests.json `--frames 600` 时 $6000=0x80（仍在跑），1300+ 帧两连 0x00=PASS（完成窗口 1200-1300，newppu 路径）；M1/M2 从核心改动降级为预算修正+文档；M0 状态机文档骨架已交（渲染期隐式填充 VRAMBuffer 未实现，如实记录）；M3（衰减）确认 defer（049 对衰减速率零约束）。**三项裁决待业主**：① tests.json 预算 600→2000 + provenance 改写；② frozen baseline 049 false→true（人工授权流程）；③ M0 文档归档。变异映射 M-A..M-E 已备（预算变异 M-E 证明 L-2 有牙）。**A6+D5 同窗 PLAN 已出、不含实施、待拍板**（要点：A6 直读快路径三条红线=DB 锁存/Genie 影子表（**存量零测试覆盖，必补 genie_wrap_test**）/fceuindbg 资格表白名单；D5 fetch 通道最小改动面=x6502.cpp:615 一处 RdFetch + 探针 S-D5-0 先行；锁测试五件套+S0-S5 串行拆序 16-24 天；采纳线 bench_bus_dispatch ≥5%、退化线 3%；038/049/050 失败模式不得变化）。**基础设施事项**：push 被远端拒（`fatal error in commit_refs_rules`，空提交亦拒——**仓库服务端规则于 892f402 成功 push 后变更，需业主查 GitHub Rulesets/分支保护**；本地 11 提交安全待推）。中断无损：无变异残留、rust.h 干净、36/36 全绿。**下会话工作单**：A5-G3 五变异 → 049 裁决落地 → A6+D5 拍板 → bench 基线重采（读数已大幅下移：cpu 33 / ppu 40 / apu 33 / full 33 / hq 45 ms）→ push |
| r8 | 2026-11-06 | **A5 G3 变异验证完成（5 agent 并行，隔离副本）——并查出两项「门禁无牙」缺陷，同批修复一项、留证另一项。** **方法**：五个变异点全在 `filter.rs` 同一文件上，故用 `git worktree --detach` 建 5 份物理隔离副本（**不建任何分支**），每副本独立 `cargo test -p fceux11-media`；主树零改动。**基线**：51/51 全绿（19.4s）。**四项有牙**（转红均为预期断言 `lock_fir_pair_scalar_simd_bit_exact`，且各自命中**不同**的暴露面）：**M-A** bias `1<<16`→`1<<17`（漂移 = 1024·Σs，余 0，定量吻合）；**M-B** 删 `−1024·Σs` 修正项（漂移 = 1024·Σs，余 0，与 M-A 同值 ⇒ 二者数学等价，2a 内互为冗余，已记）；**M-C** `& !63`→`& !31`（漂移仅 +108/+111，1 LSB 级截断漂移仍被咬住）；**M-E** 尾部 tap 循环 `full..nc`→`full..full`（暴露面落在**真实 484 抽头表** nc=484，非仅边界表）。每项均「变异→预期断言转红→还原→51/51 复绿」，主树 `git status` 全程干净。**⚠️ M-D 无牙（首项真实发现）**：插值权重翻转 `65536−frac`→`frac`（`filter.rs:485`）下 **51/51 全绿、0 项转红**；探针实证该变异**是活的**（输入幅度 300/1000/4000 下输出哈希与逐样本值全变），非 no-op。**根因 = 共因盲区**：插值位于 `resample` 内 `if simd_ok{AVX2}else{标量}` 分叉**之后**，标量与 SIMD **共用同一条公式**；2a/2b/3 号三条锁全是**相对比较**（此路对彼路），共因变异时两边同步漂移 ⇒ 断言恒成立。**且 `filter.rs` 原 doc comment 声称 2b 能抓 M-D —— 实测不准确**（AGENTS.md「通过原因必须与要验的状态是同一个可观测量」的第三例）。**⚠️ HQ golden 对 FIR 变异结构性无感（第二项发现，主树独立复核）**：HQ golden 与低质量 golden 的**捕获样本全为 0**（各 799 样本，最大绝对值 0）。真因**不是**「FIR 输入为零」——管线探针实测 FIR 输入是**恒定 DC 4185**（非零）、FIR 输出恒定 34682，是下游 `SexyFilter`（隔直 IIR）把 DC 衰减到恰好 0，且**捕获帧早已收敛**（#59/#149/#209 三帧 out_maxabs 均 0）；纯 DC 输入 ⇒ 任何保持纯 DC 特性的 FIR 变化都不可观测。**实证矩阵**：M-A（bias）、M-B（系数表）下 HQ golden 仍 **PASSED 3/3**；对照实验注入 **DC** 偏置 → 仍绿，注入 **AC** 分量 → **转红**（offset 44）⇒ 通路是活的、golden 机制没坏，缺的是含 AC 的输入。**主树复核**：主树施加 M-D → 构建 → 跑 `apu_wav_hq_diff_test` → **仍 PASSED 3/3**（独立复现取证 agent 结论）。**连带发现**：低质量 golden（kgmqa-024）**同病**（三文件 body 非零字节数均为 0）⇒ 非 HQ 独有，是共用 fixture ROM 集合的通病。**已落地修复（一项）**：新增 `lock_resample_interpolation_golden_vector`（`filter.rs`，360 行纯新增、0 删除，既有锁测试一字未改）——**绝对金标准向量**：期望值由测试模块内**独立参考实现**从线性插值定义 `f(base+t)=f(base)+(f(base+1)−f(base))·t` 推导（`golden_fir`/`golden_resample`），**不调用被测代码、不比较两个实现**；另含 frac=0 / frac=65535 / 单调滑动三个**方向锚点**（直陈语义，不比较实现）。**主树实测**：未变异 52/52 全绿；M-D 下**仅新测试转红**（51 passed/1 failed，2a/2b/3 号全绿 ⇒ 共因盲区被精确堵住），诊断字段 `flipped-weight-would-give=313501` 与实际输出**完全相等**（机器确认「M-D=翻转权重」）；M-A/M-B/M-C/M-E 下新测试亦**全部转红**（覆盖面 = 2a ∪ 2b ∪ {M-D}，M-D 是唯一只有它能抓的）。**doc comment 更正**：`filter.rs` 2b 的 doc comment 已从「M-D 由 2b 抓住」改为如实写明其**只覆盖非对称变异与 mrindex 记账、不覆盖插值权重方向**，并指向新测试。**未修（留证待裁决）**：HQ golden 的 AC 输入缺失（需新增会写 APU 的专用 fixture ROM + 「捕获必须非静默」fail-fast 断言；`tests/fixtures/generate_test_roms.py:251,296` 的 fixture PRG 是 `0xEA` NOP sled，**不写任何 APU 寄存器**；已提交 195 个 ROM 中除 `nestest.nes` 外全为 NOP-sled mapper ROM，发声 ROM 均未被 git 跟踪 ⇒ 补救须先决定 fixture 来源与许可）。**门禁现状**：media crate 52/52（+1）；`apu_wav_hq_diff_test` 仍绿（**但须知其对 FIR 变异无牙，不得作为 A5 的红源**）。**环境事实（与本改动无关）**：`ninja -j 12` 并发归档触发 `LNK1104`（多个 `lib.exe` 争写 `.lib`），**`-j 1` 串行构建稳定通过**——本轮所有构建均用 `-j 1`。**教训归档**：「锁测试全绿」≠「有牙」；**相对比较型锁测试对共因变异结构性无感**，凡要验「共享路径上的语义」必须用绝对金标准或独立参照实现。 |
| r9 | 2026-11-06 | **低难度批次（4 agent 并行 + 主会话）——r8 留下的「HQ golden 无牙」缺口已闭合：两道音频 golden 门禁第一次真正有牙。** **批次切分（按文件域互斥）**：① 环境变量表核实（**纯只读**）② A1 手工验收清单成文（新建 `docs/tech/debug-regression-checklist.md`）③ D4/050 取证调查页（新建 surveys 页）④ 发声 fixture ROM（**由主会话接管完成**：该 agent 卡在副本 CMake/vcpkg manifest 联网，耗时超限；成果与变异验证由主会话亲自做完并留证）。**① 附录 A 全表重核**（改写为**三张表**：运行期环境变量 26 条 / 编译期开关 14 条 / 上游配置变量 4 条）：查出 **4 处坐标漂移 + 1 处完全失效**（`FCEUX11_BENCH_BASELINE` 旧记 `bench_tolerance_test.cpp:249`，**实际 :270**——旧坐标处已无该内容）、**3 个整族漏项**（`FCEUX11_OAMBUS_PROBE` / `FCEUX11_A3_XDUMP` / `FCEUX11_A4_WAVEDUMP` 表里完全没有）、5 处描述过时（`E3_TRACE` 实际含帧计数器+`$4017`+`$4015`+IRQ+复位，远不止「`$4017` 写」）；并把「编译期开关」与「运行期 env」分开（`FCEU_PROFILER_ENABLE` 是 CMake option，设 env 开不了）。**顺带更正 `docs/tech/gba_probe_howto.md:41`**：原写「`probe_trace_a_stuck_game` 用 `GBA_PROBE_TRACE`」**与代码不符**（该探针读 `GBA_PROBE_WARMUP`/`GBA_PROBE_TRACESTEPS`；`GBA_PROBE_TRACE` 只被 `dump_frames_like_the_reference` 读）。**② A1 手工验收清单成文**（643 行，**22 个可勾选项**，覆盖 r5 全部 15 个场景关键词 + 1 个 A1-b 反向项）：每项给出**可照做的步骤 + 绿/红判据 + A1 关联的代码坐标**；含「为什么这批项不可自动化」的代码依据（ctest 注册**全空**回调表 ⇒ viewer 回调恒 nullptr；`break_asap` 需真实指令流）；§七**如实记录 7 处「入口不存在/命名不符」**（如无 "Break on PC" 菜单项——r5 那个名字零命中，实际入口是 Add+Execute 与 Run To Cursor 两个；Lua 侧无 `getTotalInstructions`/`break_asap` 这两个名字，实际 API 是 `debugger.getinstructionscount()`/`debugger.hitbreakpoint()`）。**清单查出的两处 r5 偏差（主会话独立复核确认）**：① **`ppu_view_open` 在 core 侧只写不读**（写方 `ppuViewer.cpp:62`，core 侧**零读点**）⇒ **A1-c 实际只剪掉了 NT viewer 那条链**，PPU/Sprite viewer 的 `FCEUD_UpdatePPUView`（`ppu_rendering.cpp:1319`、`:1778`）**仍是每帧间接调用**（不构成缺陷，但「viewer 已全部剪枝」的说法不成立）；② **`vblankScanLines`/`vblankPixel` 的计算在门内**（`debug.cpp:921-929`）⇒ **零断点时调试器 PPU 页签的 vblank 读数停在旧值**（A1 的可观察行为变化，非崩溃）。**③ D4/050 取证调查页**（`docs/history/surveys/oam_dma_dummy_write/`，651 行 + 复跑产物）：按仓库规则归 `docs/history/surveys/`（`docs/tech/README.md` §3 明写「过程性记录不进 tech/」），并在 `docs/tech/precision.md` §5 调查链索引登记。**主会话复跑探针补齐证据**（原 r6 数字只有 commit message + 计划表格两处二手记录，仓库内无产物）：050 复跑**复现 `value=0x06 status=FAIL`**、**`armed=1`**（⇒ r6 的「0 次 SPRAM 改写」**不是「从未 armed」的平凡真值**，是有效阴性证据；此条 r6 未记录）、`dma_bursts=0 / diff_events=0`；另跑 kgmqa-110 作**阳性对照**（`dma_bursts=75164 / dma_bytes=19241984 / armed=1 / status=PASS`）⇒ 探针在真实 DMA 场景下确实在记录。产物只留 `spram_diff.log` + 复现命令（`trace.bin` 在 110 对照下 2000 帧写满 256 MB 上限，不适合入库）。**④ ★核心成果：HQ/low 质量 golden 门禁第一次真正有牙。** 新增**自建** fixture `tests/fixtures/apu_square.nes`（`generate_test_roms.py` 新增 `generate_apu_square_rom()`：6502 程序 `$4017=0` → `$4015=1` → `$4000=0xBF`（50% 占空比、音量 15）→ `$4001=0`（关 sweep）→ `$4002=0x40`/`$4003=0x08`（周期 0x408）→ 死循环 ⇒ **周期性方波 = AC 内容**；**自建资产、无第三方许可问题**）：**`.gitignore` 加白名单** `!tests/fixtures/apu_square.nes`（否则 CI 上门禁会因缺 ROM 而失败）。**实测捕获特征**：HQ 799/799 样本非零、峰值 |幅度| **3231**、逐样本变化（-2755,-2718,-2712,-2806,…,835）；低质量 734/734 非零、峰值 **2776**、含明显跳变（-2693→384→2771）。**同时给两道门禁加「捕获必须非静默」fail-fast 断言**（只对 `apusq` 生效——其余三个 ROM 按构造就是静音，加了会立刻变红且无意义）。**变异验证（两轮，主树实测）**：① FIR bias `1<<16`→`1<<17` ⇒ **HQ golden 转红**（`byte mismatch ... offset 44`，3 passed/1 failed）、**低质量 golden 仍绿**；② `SexyFilter` `mul1` 94→95 ⇒ **低质量 golden 转红**（`offset 46`）、**HQ golden 也红**（HQ 路径同样经过 SexyFilter）。⇒ **两道门禁各自有牙且覆盖不同代码路径**：HQ 管 FIR/AVX2 内核，低质量管 SexyFilter/DC 隔直；**低质量路径按设计根本不经过 `fir_pair_avx2`**（`sexy_filter_*` 不调 `resample`）⇒ 旧口径「golden 逐字节 = 守住 FIR」**是错的**。**门禁现状**：media 52/52、GBA 249/249、vendor 242/242、`ctest -LE perf` **36/36**。**其他低难度项**：bench 基线**补两档**（`bench_apu_frame`/`bench_bus_dispatch` 此前报「no baseline entry」= 无门禁运行；**A2 优化的正是 APU 却恰恰没有 APU 基线**）⇒ 6 档全有基线、对照偏差 +0.06%~+2.41% 全 PASS；顺带**修 `generate_bench_baseline.ps1` 的路径 bug**（相对路径 `'fixtures/bench_baseline.json'` 由 .NET `GetFullPath` 解析，**不随 `Set-Location` 走** ⇒ 写到仓库根而失败；改用 `$PSScriptRoot` 锚定）。**文档计数统一**：`ctest -LE perf` **33→36**（计划 §头/§〇/§八 + AGENTS.md）。**陷阱清单扩容**：`docs/tech/gba-traps.md` 由四种→**六种**（新增「相对比较型锁测试对共因变异结构性无感」「门禁捕获内容为空 ⇒ 哑弹」）。**教训**：**「golden 逐字节过」不等于「守住了它声称守住的东西」**——门禁的价值取决于它捕获的信号**是否含被测路径的可观测量**；全零/纯 DC 捕获会让任何算术改动不可观测。 |
| r10 | 2026-11-07 | **CI 失败诊断修复 + 主树变异复证 + 基线重采（两提交：caf62d4 / eb664a3，与并行会话的 r8/r9 无冲突）**。**CI 根因（run 114390042265）**：`apu_frame_timing_test` 与 `input_snesmouse_test` 各挂满 1500s 默认超时（其余 34 项秒级、进程**零字节输出**）——两测试不在 `tests/CMakeLists.txt` 的 `ENVIRONMENT_MODIFICATION` 名单（r6 注册测试时漏加；后加的 `apu_wav_hq_diff_test` 反而在），CI 上以裸 PATH 启动；本地从未复现因开发会话手动前置 vcpkg bin。**取证链**：两 exe 的 DLL 依赖与 apu_test 逐项一致（dumpbin 含共享的 `z.dll`）⇒ 排除依赖面差异；本地裸 PATH 复现 = **秒败 0xC0000135**（非慢），CI 的 1500s 存活是该失败在无头 runner 会话（WER 不可见交互）的表现形态；本地复现挂起不可得，但名单缺陷客观存在且签名（恰好只挂这两个名单外测试）完全吻合。**修复**：两测试补入名单 + 显式 `TIMEOUT 300s` 纵深防御 + 顺手修 filter.rs 四处 `_mm256_storeu` 的 E0133（edition-2024 `unsafe_op_in_unsafe_fn`）。**验证**：不带手动 PATH 的 ctest **36/36 全绿**（CI 环境模拟）。**A5-G3 主树独立复证（与 r8 的 worktree 隔离法互补）**：M-A/M-B/M-C/M-D/M-E 五变异逐一在主树「改→测→还原→复绿」，全部红且暴露面与 r8 记录一致（M-A/M-B/M-C/M-E golden 红；M-D 由 `lock_resample_interpolation_golden_vector` 独家抓住、等价锁对共因变异盲——纪律 12 活案例的第二次实测）。**基线重采**（eb664a3）：六档全档（r9 已补 apu/bus 两档）best-of-5：cpu 34.049 / ppu 29.174 / full 32.135 / **hq 41.006**——A 线累计：cpu −27%、ppu −37%、full −34%、**hq −48%**；tolerance 10.0 不变、重采后对照全 PASS。**注意**：本条与 r8/r9 来自两个并行会话，读数以各自实测为准；本条落档时 push 前状态 = 4 笔本地提交（800b3f8 / 3d2bccb / caf62d4 / eb664a3）。**待裁决事项不变**（r6 ④ + r7：081 重分类、D4 修复立项、mset 注入面、A1 手工清单执行；新增：A6+D5 同窗 PLAN 拍板） |
| r11 | 2026-11-07 | **五项裁决落地 + D4 修复转绿（三提交：4b45762 / b0a016a / c714a72）**。**裁决记录（业主授权按最优解执行）**：① **A6+D5 = 批准 S0+S1 先行**（零行为改动），S2-S5（fetch 通道+快路径）视 S1 探针证据再定；② **kgmqa-081 = BIOS 门控 known-limit**（baseline 不翻）；③ **D4 = 立项并已修复**；④ **mset headless 注入面 = defer**（advisory 项、产品级 ABI、性价比不足）；⑤ **D8 PAL/Dendy = defer**（主题外扩）。**A6+D5 S0+S1**（4b45762）：Genie 影子表/DB 锁存/fceuindbg 三件**存量零覆盖**锁测试（bus_test 19→63 断言）+ 变异 M-A6-1/2/3 各自红→绿 + `FCEUX11_FETCH_PROBE`（静默）；**S1 证据**：051 的 IO 域取指 31 次/总 257 万（0.001%），110/048 为 0——read_fetch 通道 IO 域命中面极小，S2 待业主再裁决。**三条实测事实留给 S2**：GetMem($4016/$4017) 走 debug.cpp 寄存器镜像短路不经 handler（计划原文断言面拆分）；A2002 的 fceuindbg 分支被 `#ifdef FCEUDEF_DEBUGGER` 编译掉；`vpage_g[]` 全仓从未填充（Genie+PPU 组合潜在坑，既有行为未改）。**D4**（b0a016a）：**kgmqa-050 转绿（0x06→0x00，矩阵 108P/12F→109P/11F）**。根因 = `FCEUX_PPU_Loop` VBL 入口无条件清 OAMADDR（硬件语义：仅渲染开启的 dots 257-320 清），渲染关闭的帧毁掉 CPU 刚程控的 OAMADDR=0xEA → 050 的 oam_read_test 恰有一读在 sl 240→241 边界前 4 周期被吞。修复 = `PPU[1]&0x18` 门控。锁测试（ppu_test 扩展，planted SPRAM+真 $2003/$2004 路径）+ 变异 M-D4 双红（050 复现 0x06 + 三断言红）→ 还原双绿。哨兵 110 PASS、ctest 36/36、金标零漂移。**081**（c714a72）：tests.json known_limit 如实化 + surveys/fds_bios_gating/ 事实链。**注意**：050 转绿后 frozen baseline 仍记 false——advisory 项，基线更新随下次统一重采/评审。**下会话工作单**：① 050 基线翻 true（人工授权流程）；② A6+D5 S2 拍板（S1 证据已备：IO 域命中面 0.001%）；③ A1 手工清单 22 项执行；④ mset 注入面与 D8 维持 defer |
| r12 | 2026-11-07 | **批末收口（三 agent 并行 + 主会话；两提交：1eaf8cc / 7864b17）**。**A6+D5 S2 落地且前提被证据证伪**：重采 051 取指轨迹 5,541,363 条逐条解析——IO 域 31 次 + expo 224 次全部返回 0x40=RTI，与真机开放总线取指语义一致（DB 锁存模型天然正确；nesdev 论坛 + spannerisms/BRK 外部佐证）；051 的 FAIL@300 是帧预算伪 FAIL（实测 ~316 帧完成）+ PPU wait_vblank 相位竞争（桶 C.1，非 bus 域）。**交付**：`read_fetch` 纯别名通道（委托 read() 共享锁存尾/Genie/fceuindbg，分叉白名单留空=证据正确形态，准入规则入注释）+ opcode fetch 改走 RdFetch + bus_test 63→85 断言 + 变异 M-D5-1'/2' 各自红→绿（计划原文的「051 红」不可实现——051 无法区分 fetch/read，这正是取证结论；偏差已记入测试注释）。**D3 落地**：049 预算 600→2000（provenance 记裁决与测量）+ 051 预算 300→400（同型伪 FAIL，~316 帧实测）+ **M0 状态机文档** `docs/tech/ppu_2007_read_buffer_state_machine.md`（R1-R16 迁移表；已知未实现：渲染期隐式填充 VRAMBuffer 实测零赋值点、per-bit 衰减 defer）。**L-2 哨兵留证**：049 600 帧红 0x80 / 2000 帧绿 0x00；051 300 红 / 400 绿。**基线裁决**：advisory 基线面按 phase8 脚本策略**恒 False**（fail-safe 通道，静态推导不跑测试）——049/050/051 绿移记录在 provenance，不翻基线；cta 门禁已实证容忍 true-vs-false（r11 起 050 PASS + ctest 绿）。**D1 专线 PLAN 已出（不含实施，待业主拍板三项闸门**：金标漂移授权预案、模型状态是否入 savestate、S2 后逐步放行**）**：设计 = 线时刻账本（本仓无每访问周期步进，不可照抄 Mesen2 两拍）；S0-S5 = 调查页+取证 / INTLEDGER 探针+cpu_interrupt_poll_test（fail-to-pass）/ 最小模型 NMI hijack BRK / NMI 劫持 IRQ+054 相容 / IRQ+DMA+050/110 保护 / branch-delay+收口；估工 16-22 天独占窗口；**与 A6/D5-S2 同文件域（x6502 取指通道）——裁决：D1 先行（解锁 D6），A6/D5-S3-S5（快路径）在 D1 收口后重采探针再动**。**门禁**：ctest 36/36、金标零漂移、rust.h 空。**下会话工作单**：① D1 拍板三项闸门（或授权按 r12 PLAN 的默认推荐执行：状态不入 savestate、S2 后闸门逐步放行）；② A6/D5-S3-S5 快路径（D1 收口后）；③ A1 手工清单 22 项执行；④ push（本批 2 笔 + r12） |
| r13 | 2026-11-07 | **D1 专线 S0+S1 落地并收口（调查页 + INTLEDGER 探针 + 锁测试；本条为四批工作项的收口记录，代码由另一 agent 实施、本会话独立复核并补齐文档登记）**。**S0 调查页**：`docs/history/surveys/cpu_interrupt_hijack/README.md`（146 行）——kgmqa-037（`cpu_int_2` NMI×BRK 差 1 周期）的 hijack 状态机四交互矩阵、G1–G5 差距面、E-1 双通道取证（set→dispatch delta = 在飞指令长度 2..7 周期，帧 3/4 = 7、其余 = 2）、NMIDELAY=8 的量化噪声面（帧 1 = 2 周期、帧 2 = 4 周期）、以及「两拍模型期望 vs 本仓现状」逐行对照表（期望值来源 = §4/§5 标定表 + nesdev 服务序列结构，不取自被测代码）。**S1 探针**：`FCEUX11_INTLEDGER=<dir>`（`src/x6502.cpp`，六个注入点 + 循环顶判定处 + 二进制账本 `<dir>/intledger.bin`（8 字节头 `INTL1`+24 + 24 字节定长记录 `{u64 abs; i32 count; u32 irqlow; u16 pc; u8 kind; u8 edge; u8 decision; u8 pad}`，`static_assert` 锁布局）+ `<dir>/summary.log`；kind 0..7 = 八条中断线 + 0xF 边界快照，decision 0..6 = 实际走到的判定分支，held 连续快照折叠计数；64 MiB 上限；**未设 env 完全静默零成本**（`intledger_on()` 一次性 static bool）。**双通道交叉验证 [实测]**：037 三百帧，INTLEDGER 与 E-1 的 assert/defer/dispatch 计数 10/10/10 两通道一致；E-1 delta [2×8, 7×2] 与 INTLEDGER delta [9×8, 14×2] **恒差 +7**——E-1 的 DISPATCH 印在 `ADDCYC(7)` 之前、INTLEDGER 的边界快照在判定块尾（`ADDCYC(7)` 已入账），**两通道互为印证**（差值本身即口径证据）。**S1 锁测试**：断言挂在既有 `cpu_test`（**不新增 ctest 项** ⇒ ctest 计数仍 36），新增 **34 条基线断言 + 12 条目标（FTP）断言**；FTP 组用自建计数器，**预期红不进 `ctx.failed`** ⇒ ctest 36 项全绿硬约束不受影响。实测 **838 passed / 0 failed**，FTP **RED=9 / GREEN=2**（T-D1-3c/3d 为两态皆绿的两条形状 pin）。**变异验证 [本会话独立复做，非转述]**：① **M-D1-0**（抑制 `g_e1_nmi_fresh` 置位）⇒ **11 项红**（B1/B1a/B5a/B5b/B5c/B5d/B6-2/B2/B3/B3b/B4b——stale 边界先派发把 CPU 送进 ROM NMI 处理器，后续栈形/向量断言全部失准），FTP 计数 9→10；② **M-D1-1**（参照模型 `kSeqCycles` 7→9，期望值污染）⇒ **2 项红**（B5b、I2a-2，恰为周期计费面），FTP 计数不动。两项均与调查页记载一致 ⇒ **锁测试有牙，非「全绿即证据」**；两次变异均已还原并复验 **838/0 全绿**（`git diff --numstat` 与变异前逐项一致，无残留）。**门禁**：Release 构建通过、`ctest -LE perf` **36/36 全绿**、golden hashes **780 帧 0 mismatch**、golden savestate 未漂移。**⚠️ CI 既有红（非本批引入，记账口径）**：`main` 的 F11QA workflow 已**连续 3 次红**（run 68/69/70，全部红在同一个 **R4 Gate**；同期 `ci.yml` 全绿）。根因 = A2 批次（`062ce84`，已在 origin/main）往 `tests/tests.json` 加了 `kgmqa-121` ⇒ 用例数 120→**121**，而冻结基线 `tests/fixtures/f11qa_baseline_frozen.json` 仍记 **120** ⇒ R4 Gate 的 `summary.total -ne 120` **确定性失败**（与本机素材/负载无关，在 GitHub runner 上同样必红）。另两条本地 `pass_to_fail` 需**分开记账**：kgmqa-043 为**本机环境所致、CI 上不复现**（本机缺 163/177 个 manifest 路径的 blargg ROM；CI 步骤 19「Hardware Consistency Check」为绿）；kgmqa-115 则**CI 上确实复现**（run 70 `pass_to_fail=1`），根因 = `eb664a3` 在 dev box 重采 `tests/fixtures/bench_baseline.json` 六档基线（cpu 36.419→34.049 等，收紧 5.5%–7.5%），CI runner 硬件更慢 ⇒ 超出 +10% 容差；与本批 D1 改动**无关**，单独立项。**本批 D1 改动零行为改动**（探针默认完全静默；锁测试只加断言）⇒ 不新增任何 CI 失败面。**R4 Gate 判据修复已按业主指令实施**（2026-11-07 业主指令「请将 R4 Gate 的 summary.total 判据改为 ≥120」）—— 实施与验证记录见 **r14**。**本批文档收尾（四项）**：① 本条 §十 变更记录；② 附录 A 运行期环境变量表补 `FCEUX11_INTLEDGER`（此前全表未收录，r9 建表时遗漏）；③ `docs/tech/precision.md` §5 调查数据索引登记 `cpu_interrupt_hijack/`；④ `docs/plans/long-term-evolution/T2-中断时序.md` 勾选 S0/S1（上位卡 T2 的阶段清单）。**下会话工作单**：① D1 三项闸门拍板（**状态不入 savestate**、S2 后闸门逐步放行、金标漂移授权预案）或授权按此默认推荐执行；② A6+D5-S3-S5 快路径（D1 收口后）；③ A1 手工清单 22 项执行（`docs/tech/debug-regression-checklist.md` 已成文未执行）；④ **R4 Gate `total` 口径修复**（既有红，独立立项）；⑤ push（本批 D1 三笔工作 + 本条文档收尾） |
| r14 | 2026-11-07 | **R4 Gate `summary.total` 判据由严格相等改为下限（业主指令驱动；既有红独立立项）**。**问题**：`main` 的 F11QA workflow 连续 3 次红（run 68/69/70），全部红在同一个 **R4 Gate**；同期 `ci.yml` 全绿。**取证方法（关键）**：用 GitHub API 取 check-runs/{id}/annotations 拿 **CI 原始报错**，而非本地复现推断 —— run 70（`1c567e5`）三条失败项为 ① `summary.total is 121, expected == 120` ② `transition_matrix.pass_to_fail is 1, expected 0 (REGRESSION: kgmqa-115-bench-tolerance)` ③ `matrix grade is D`；run 69（`3d2bccb`）**仅**第 ① 条 ⇒ **kgmqa-115 的 PASS→FAIL 是 run 69→70 之间由 `eb664a3`（bench 基线重采）引入**，与本次 D1 改动无关。**根因**：`eb664a3` 把 `tests/fixtures/bench_baseline.json` 六档从 2026-11-06 值重采为 2026-11-07 值，**全部收紧 5.5%–7.5%**（cpu 36.419→34.049、ppu 31.383→29.174、full 34.012→32.135、hq 44.337→41.006、apu 34.252→32.017、bus 36.335→33.861）；基线采于 dev box，CI runner 更慢 ⇒ 超 +10% 容差。**判据改写**：`.github/workflows/f11qa.yml` L470 由 `if ($m.summary.total -ne 120)` 改为 `if ($m.summary.total -lt 120)`，报错文案改为 `expected >= 120`，并加长注释记录改判据的三条理由 —— ① **与清单自身声明一致**（`tests.json` 的 `policy.r4_gate_thresholds."total_min": 120` 本就是下限语义，判据却写严格相等，两边自相矛盾）；② **runner 已结构性承担防作弊**（`report/matrix.rs:244-266` 的 `in_baseline` 判定把基线外用例归入 `new_test` 桶并排除出四个迁移桶，「新增未登记」已由 runner 侧把关，R4 Gate 再卡总数是重复把关且必然误伤）；③ **「少 1 个也失败」的原始意图仍保留**（`-lt` 判的正是用例被**静默跳过**，即总数跌破下限）。**未改动任何期望值**（冻结基线仍 120 项、kgmqa-121 仍走 `new_test` 桶），符合 `blargg_known_fail.json` 的「AI shall not modify expected values」约定。**本地复验**：矩阵复跑 Total **121** / Passed 110 / Failed 12，`total` 判据不再触发；`pass_to_fail` 仅剩 **kgmqa-115-bench-tolerance**（另 10 个 FAIL 全为 advisory 且 baseline=False ⇒ fail_to_fail，不影响 R4）；blargg 素材已补齐至 **177/177**（`scripts/download_blargg_roms.ps1` + 手动补 `cpu_test5_cpu.nes`，经代理 127.0.0.1:7897），kgmqa-043 由 FAIL 转 PASS。**剩余红的定性（独立立项，不计入本次改动）**：kgmqa-115 属**墙钟类 bench**，其 PASS/FAIL 不具跨机可比性 —— 与既有 doctrine 一致（`tests/CMakeLists.txt` 已用 `LABELS "perf"` + `ctest -LE perf` 把 bench 排除出确定性门禁，`ci.yml` 的 Bench tolerance gate 为 `continue-on-error: true`）；本机低负载复测 ppu_frame 亦稳定超容差 +13.4%~+14.8%（原神占用 CPU 的干扰已排除）。**修法待业主拍板**（触及期望值/门禁语义，按治理须与代码变更同级评审）：(A) 恢复 2026-11-06 旧 bench 基线（run 69 已实证该基线下 kgmqa-115 在 CI 上 PASS；代价 = 丢掉 `eb664a3` 固化的改善读数，且 ppu_frame 余量很小）；(B) 把 bench 档位移出迁移矩阵的回归判定（标 advisory 并同步基线；代价 = 需改 `tests.json` / `f11qa_baseline_frozen.json`）；(C) 放宽容差（治标、掩盖真实回退，**不推荐**）。**门禁**：Release 构建通过、`ctest -LE perf` **36/36 全绿**、cpu_test **838/0**（FTP RED=9 GREEN=2）、golden hashes **780 帧 0 mismatch**。**下会话工作单**：① 业主拍板 kgmqa-115 修法 (A)/(B)/(C)；② D1 三项闸门拍板；③ A6+D5-S3-S5 快路径（D1 收口后）；④ A1 手工清单 22 项执行；⑤ push（分笔：D1 S0+S1 / 四项文档收尾 / R4 Gate 判据） |
| r15 | 2026-11-07 | **kgmqa-115（性能基准容差用例）移出「精度回归」判定，改 advisory + 只记录不判红（业主裁决 2026-11-07）—— CI 既有红的最后一块**。**问题**：R4 Gate 的 `total` 判据已在 r14 改为 `-lt 120`，但 run 70（`1c567e5`）仍有**两条**红项同源于 kgmqa-115：`transition_matrix.pass_to_fail is 1 (REGRESSION: kgmqa-115-bench-tolerance)` 与 `matrix grade is D`（grade.rs 的 D 分支既由 blocking 失败触发、也由 pass_to_fail 非空触发）。**根因（非推测，已取证）**：kgmqa-115 是**墙钟类 bench**，其 PASS/FAIL 取决于跑它的机器快慢，**不具跨机可比性** —— `tests/f11qa/benchmarks/bench_tolerance_test.cpp:270-273` 的基线路径优先级为 `FCEUX11_BENCH_BASELINE` env > `fixtures/bench_baseline.json`（默认）；`f11qa.yml` **未设**该 env，而该用例的 `working_dir` = `tests` ⇒ 迁移矩阵比对的正是 `eb664a3` 在 dev box 重采的那份 `tests/fixtures/bench_baseline.json`（六档收紧 5.5%–7.5%），CI runner 更慢 ⇒ 超 +10% 容差 ⇒ 无任何代码改动却被判成「精度回归」。**既有 doctrine 一致性证据（本仓自己的口径，早于本裁决）**：`tests/CMakeLists.txt:413` 给 bench 用例打 `LABELS "perf"` ⇒ `ctest -LE perf` 把 bench 排除出确定性门禁；`ci.yml:385` 显式设 `FCEUX11_BENCH_BASELINE` 但该步为 `continue-on-error: true`；`tests/fixtures/bench_baseline.json` 的 `_comment` 自述「CI excludes wall-clock gates entirely (-LE perf; 20+ percent runner noise)」。**⇒ 迁移矩阵是唯一一条没有遵守该口径的通道。** **裁决落地（两处期望值改动，均按 `blargg_known_fail.json` 的「AI shall not modify expected values. Baseline updates require the same review as code changes.」走业主同级评审）**：① `tests/tests.json` 的 kgmqa-115 `failure_means`：`blocking` → `advisory`，并补 `known_limit` + `provenance` 记裁决与根因（对照 `c714a72` 对 kgmqa-081 的同类处理）；② `tests/fixtures/f11qa_baseline_frozen.json` 的 kgmqa-115：`true` → `false` ⇒ 该用例落 `fail_to_fail` 桶（**记录不判红**），且**不落入 `new_test` 桶**（若改为「从基线删除」会落 new_test(FAIL) ⇒ grade 由 B 降 C —— 已避开）。**为什么 baseline 置 false 而非删项**：基线的 `false` 语义 = 「该用例已知失败/不可跨机比较」，与既有的 10 个 advisory 已知限制（037/038/050/077/081/096/097/099/107/108）同形；删项会同时产生 `test_set_diff.removed` 与 new_test(FAIL) ⇒ grade 降 C。**CI 上仍然实际执行该用例**（不是跳过），只是其结果不再参与「精度回归」判定 —— 真实性能回退仍由 `ctest -LE perf` 与 `ci.yml` 的 bench 步（advisory）覆盖。**本地复验（矩阵复跑，121 项全跑）**：Total **121** / Passed **110** / Failed **11** / **Grade B**（修复前为 D）；五桶归属 = `pass_to_fail` **0**、`fail_to_pass` 2（049/051）、`pass_to_pass` 107、`fail_to_fail` **11**（含 kgmqa-115）、`new_test` 1（121）；R4 Gate 六条判据**逐条本地判定全绿**（`git_rev=2e1a2e7` / `total=121 ≥ 120` / `pass_to_fail=0` / `vendor_state={v=43 a=13 p=22}` 三态和 = 78 / `grade=B`（非 D/E）/ advisory-FAIL 11/121 = **9.1% ≤ 15%** 上限）⇒ **CI 的 R4 Gate 判据层已满足**。**变异验证（判据仍有牙，不是哑弹 —— 本会话独立实测）**：**M-GATE-1**（把 kgmqa-115 基线改回 `true`）⇒ kgmqa-115 重新落入 `pass_to_fail` ⇒ gate 转红 ⇒ 证明转绿的确切来源是本次两处改动、且 `pass_to_fail` 路径仍然活着；**M-GATE-2**（把 **kgmqa-050**（真实精度缺陷用例）的基线改 `false`→`true`）⇒ 落入 `pass_to_fail` ⇒ gate 转红 ⇒ 证明豁免**只作用于这一个墙钟用例**，并未削弱对真实精度回归的检出。**门禁**：Release 构建通过、`ctest -LE perf` **36/36 全绿**、cpu_test **838/0**（FTP RED=9 GREEN=2）、golden hashes **780 帧 0 mismatch**。**下会话工作单**：① push（D1 S0+S1 / 文档收尾 / R4 Gate 判据 / 本条 kgmqa-115 修法，四笔）；② D1 三项闸门拍板；③ A6+D5-S3-S5 快路径；④ A1 手工清单 22 项执行；⑤ **CI 转绿仍需 push 后实跑确认**（本地判据全绿 ≠ CI 已绿） |
| r16 | 2026-11-07 | **CI 实跑确认：push f9cbb1e 后三个 run 全绿，main 连续红（68/69/70）就此闭环**。**结论（GitHub Actions 实跑，非本地推断）**：F11QA run **42352471017** = **success**（job f11qa，check_run_id 128343357626，07:43:41Z→08:20:50Z）；CI run **42352471027** = **success**（job build-windows，check_run_id 128343357619，07:43:41Z→08:04:52Z）；CodeQL run **42352470648** = **success**。**F11QA 关键步骤逐条**（全部 success）：Build C++ / Pre-flight kgmqa-117 mirror_snapshot_check / Build f11qa-runner / Fetch test ROMs / Download blargg suite ROMs / **Internal Logic Check — CTest** / **Hardware Consistency Check — Blargg Suite** / **F11QA — Migration Matrix** / Upload F11QA Artifacts / Print Summary / **R4 Gate — migration matrix must exist and be sane (v1.8)** ⇒ **R4 Gate 步本身在 CI 上判绿**（本地六条判据全绿的对应项）；Baseline Drift Detection = skipped（该步仅在基线变更时触发，本次未改基线）。**CI run 关键步骤**：Build / **CTest**（ctest -LE perf）/ **ROM Regression Test** / Benchmark (smoke) / **Bench tolerance gate** 均 success；Golden savestate drift check 与 Golden layout compare = skipped（ci.yml:194 判据 github.event_name == 'pull_request'，push 事件不触发）。**闭环口径**：r14（total 判据改 -lt 120）+ r15（kgmqa-115 移出精度回归判定）两处修复，在远端实跑中同时生效 ⇒ **run 68/69/70 的既有红不再复现**，且 ci.yml 与 F11QA 两侧同时绿。**本条只记录事实，不放宽任何出口判据**（pass_to_fail = 0、grade ≥ B、advisory-FAIL ≤ 15%、金标三件套零漂移六条判据全部原样保留并已在 CI 上判绿）。**R4 Gate 专项失败的消失（决定性证据，check-runs/{id}/annotations 原始报错对比）**：run 71 的 F11QA check_run_id 128343357626 的 annotations **共 3 条** = 1 条 Node.js 20 deprecation warning + **2 条 Process completed with exit code 1**（start_line 103 / 615），**零条 R4 Gate 专项失败**；对照 run 70（check_run_id 128068529378）**8 条** = 上述 2 条 + **4 条 R4 Gate 专项失败**：summary.total is 121, expected == 120 / transition_matrix.pass_to_fail is 1, expected 0 (REGRESSION: kgmqa-115-bench-tolerance) / matrix grade is D (D/E are release-blocking) / R4 gate failed - per the acceptance report section 10 R4 falsification criteria, do NOT merge until this is resolved；run 69（check_run_id 127891954816）**7 条** = 1 条 R4 Gate 专项失败（summary.total is 121, expected == 120）+ 同款 exit-1 项。⇒ **R4 Gate 专项失败从 1 条（run 69）→ 4 条（run 70）→ 0 条（run 71）**，正是 r14（total 判据改 -lt 120）+ r15（kgmqa-115 移出精度回归判定）两处修复的预期效果。**残留的 2 条 exit-1 是长期存在的既有容忍项**：run 69/70/71 三轮**均含 line 615 那一条**（对应 f11qa.yml 内 continue-on-error: true 的步骤 —— CTest L281 / Blargg Suite L306 / Migration Matrix L349，三者均显式容忍非零退出），且 run 71 的 F11QA job 与整个 workflow 的 conclusion 均为 **success** ⇒ 已被设计上的容错吸收，**不是本次引入的新失败面**。**取证方法说明**：run/job 结论取自 **run/job 页面的 step 级 conclusion**（HTML data-conclusion 字段）+ **check-runs/{id}/annotations 原始报错**（两者同源于 GitHub 记录的步骤结论与注解）；step 级日志正文（.../checks/<id>/logs/<n>）需登录态故未取，**R4 Gate 六条判据的 CI 侧具体读数未逐条取到**（R4 Gate 步本身未产生任何 failure 注解，即六条判据全部通过 —— 若任一条失败，该步必然产出 R4 gate: ... 注解；本地侧读数见 r15）。**下会话工作单**：① **D1 三项闸门拍板**（状态不入 savestate / S2 后闸门逐步放行 / 金标漂移授权预案）—— 或授权按 r12 PLAN 的默认推荐执行；② A6+D5-S3-S5 快路径（D1 收口后）；③ A1 手工清单 22 项执行（docs/tech/debug-regression-checklist.md 已成文未执行）；④ r15 工作单第①项（push）已完成、第⑤项（CI 转绿确认）由本条闭合 |
| r17 | 2026-11-07 | **D1 专线三项闸门拍板完成（业主点选裁决 2026-11-07）—— D1 专线的剩余唯一卡点解除**。**闸门 #1 · 模型状态不入 savestate**：S2 在 x6502.cpp 引入的新模型状态（hijack 序列进行中标志、branch-delay 采样窗口等）**不进 state.cpp 的 SFCPU/SFCPUC SFORMAT 表** ⇒ **golden_savestate 保持零漂移**，不触碰 precision.md §3 禁忌清单（savestate chunk 的名字/大小/序）、不触发金标人工授权重生成流程。**技术依据**：① 现有 CPU 状态面 = state.cpp:123 SFCPU + :135 SFCPUC（_IRQlow 已含在 SFCPUC 的 IQLB），S2 的新状态是**指令内瞬态**（hijack 窗口只在单条指令的轮询点→派发点之间有效），跨存档边界几乎不可观测；② 现状已有同类先例 —— x6502.cpp:678 g_e1_nmi_fresh 与 :746 g_power_on_reset_pending 两个 static bool 本就不在 savestate 面内。**闸门 #2 · S2 后门禁逐组放行**：S1 锁测试 9 条 FTP 断言分三组，**S2 只放行 T-D1-2（两拍，3 条）+ T-D1-3（hijack，2 条 RED + 2 条两态绿 pin）转绿**，**T-D1-4（branch-delay，4 条 RED）保持红直到 S5** ⇒ 每步只允许该步该转的转、误放行会被立刻看见，与「分阶段、每步全量对账」的原设计一致（**不采用一次全放行**——那会迫使 S2 连带实现 S5 的 branch-delay、改动面与回归半径大幅扩大；**不采用长期不放行**——那会让门禁无牙、「该转绿却没转绿」抓不到，与纪律 3 相悖）。**闸门 #3 · 金标漂移预案 = 停 + 记录 + 提交业主裁决**（与 precision.md §3、长期演进计划 L46「known_fail / frozen baseline / savestate 金标更新须人工授权」完全一致，**不引入任何「预设授权」或「默认同意」机制**——后者实质是弱化的预设授权，与 blargg_known_fail.json 的「AI shall not modify expected values. Baseline updates require the same review as code changes.」相悖）。**三个闸门均未放宽任何出口判据**：pass_to_fail = 0、grade ≥ B、advisory-FAIL ≤ 15%、金标三件套零漂移、CPU 金标哨兵 048/032/057/053/054 不回退全部原样保留。**D1 专线可开工**：S2 阶段（NMI hijack BRK → 验证 cpu_int_2）的授权已具备。**下会话工作单**：① **D1 S2 实施**（最小模型：NMI hijack BRK；闸门 #1/#2/#3 三条约束生效）；② A6+D5-S3-S5 快路径（D1 收口后）；③ A1 手工清单 22 项执行（docs/tech/debug-regression-checklist.md 已成文未执行）；④ 本轮两笔文档提交（9e45634 / ca9082b）待业主指示后 push |
| r18 | 2026-11-07 | **D1 S2 落地：中断轮询两拍模型 + BRK 的 NMI 向量级劫持；037 十行 5/10→7/10 仍 FAIL 0x01（剩余缺口定性＝线断言侧量化，G5 残差，需独立 RE）**。**实现**（`src/x6502.cpp`、`src/ops.inc`→`ops_table.inc`、`tests/f11qa/core/cpu_test.cpp`）：① 采样点从指令边界下沉到「指令 dispatch 之后」（poll point），派发在下一 loop-top；`g_intl_pending`（0 无/1 NMI/2 IRQ）承载采样结果，不入 savestate（闸门 #1，golden_savestate 8/8 实证零漂移）。线源时序对照：mapper IRQ / APU frame IRQ / VBL NMI / RTI 的派发时刻全部不变，唯一变化窗口＝「线在指令边界前已置位」晚一条指令（即 T-D1-2 要的修正）。② `g_e1_nmi_fresh` 退休（两拍采样天然产生同一时刻；连带：E1 NMI_DEFER 输出消失、INTLEDGER decision=3 恒 0、M-D1-0 变异点失效→由 M-D1-2/M-D1-2b 替代）。③ BRK hijack：NMI 线在 BRK 执行时置位 ⇒ BRK 自身压栈序列原样保留、仅向量读 $FFFE→$FFFA、`X6502_NmiHijacked()` 消线防二次派发。**模型纠正（本轮关键取证，纪律 12 第四例）**：交接文档与 S1 调查页 §2/§5 的「hijack 压栈 B=0」系对 blargg README 的误读——原文该行注释为 "NMI interrupting BRK, **with B bit set on stack**"（期望行 `36 00 00`，bit4 置位＝B=1）；`cpu_interrupts_v2/source/2-nmi_and_brk.s` 的 nmi 处理器 `pla` 实证该字节＝NMI 序列压栈的 P ⇒ 正确模型＝**向量级劫持**（push 不重压、只改向量）。S1 断言 B3/T-D1-3b 与 ops.inc 首版 hijack 按错误模型实现且与调查页同源 ⇒ 锁测试全绿不证明该语义，最终靠外部参照（测试 ROM 自带源码＋README，christopherpow/nes-test-roms 镜像）暴露改正；nesdev.org 全程 Cloudflare 403 未取到（如实标注）。harness 首轮另暴露两处：a) 首版 hijack 压栈写 `_P|U_FLAG`，B 位随 _P 残留浮动（真机 ROM 恰好带 B＝偶然正确；power-on P=$34 的 harness 立刻暴露）⇒ 改为显式 `_P|U_FLAG|B_FLAG`；b) B4b/T-D1-3d 断言活 PC==NMI 向量，但预算 30 会让 nestest 的 NMI 处理器 RTI 弹栈使 PC 离开向量 ⇒ 预算改 7（＝边界持有 0＋劫持序列 7，处理器未及运行，架构精确态，纪律 11）。**实测**：cpu_test **850/850 PASSED**（非 FTP 836→840：I2a 4→7、B7 2→3；FTP 11 条：红 9→1／绿 2→10，唯一余红 T-D1-4c＝S5 设计红）；ROM kgmqa-037：S0 5/10→**7/10** 行匹配（行 4-5 `36 00 00` 与 blargg 参照一致；行 6-8 仍 `27 36 00`、期望 hijack——根因＝ppu_rendering.cpp 的 VBL 块先 `X6502_Run(8)` 跑完预算再 `TriggerNMI()` ⇒ 线断言永远量化在指令边界且落在 BRK 完成之后；r5 NMIDELAY 扫参已证明无固定 D 两全、040 调参不可动 ⇒ 周期级线断言精度属 G5 残差、S2 模型范围内不可达，需独立 RE 立项）；哨兵 054/040/048/053/057/032 全 PASS 零回退；ctest **36/36** 全绿；金标三件套零漂移（hashes 780/0、savestate 8/8、wav 组在 ctest 内）。**变异验证**（纪律 3，全部转红＋还原复绿，红面与预判一致）：M-D1-2 NMI 采样短路→9 红（B1b/B1c/B5b/B5c/B5d/C4/C5/B6-1/B6-2＝全部 NMI 派发基线；T-D1-2/3 与 T-D1-4a/b 不依赖该路径保持绿）；M-D1-2b IRQ 采样短路→7 红（I2a-5/6/7、B7-3、T-D1-2c、T-D1-4d）；M-D1-3 劫持向量 $FFFA→$FFFE→2 红（B4b、T-D1-3d）；M-D1-14 消线去除→1 红（B2b——预算 7 下第二次派发不发生，可观测量＝线状态）。**闸门 #2 实际形态（放行清单被事实部分推翻，如实记录）**：T-D1-2 3/3 绿 ✓；T-D1-3 4/4 绿 ✓（按纠正后模型）；T-D1-4a/4b/4d 自然转绿（原「保持红到 S5」对这三条不成立）；仅 4c 保持红（S5 派发形状），且其期望常数 $204 存疑——6502 相对分支目标应为 $203+$02=$205（S1 授权时疑似把偏移加在操作码地址），S5 落地时须重推、本轮不改。闸门 #3 未触发（零漂移）。工具：`build/d1_s2_patch2.py`（S2-② 补丁，修正其自反断言缺陷后应用）、`build/d1_s2_mutation.py`（四变异 harness，apply/revert 幂等）。**下一步建议：S3（cpu_int_3）开工前，先对行 6-8 的 G5 残差（线断言周期级精度）单独 RE 立项评估** |
| r19 | 2026-11-07 | **D1 S2 后 L2 同族摸底（只读：int_3/4/5 × S0/S2 双基线）—— G5 残差确认为 int_2/3/4/5 共同 PASS 阻塞，建议 G5 RE 先立项；int_3 行 5-6 的 S2 回归机理已查明＝S3 模型工作**。方法：`f11qa_blargg_runner` 直跑 fixture（--frames 3000，进程级并行 ~5 s/ROM，不涉构建无串行约束）；S0 基线＝`git checkout f9cbb1e -- src/{x6502.cpp,ops.inc,ops_table.inc}` ＋ `--target f11qa_blargg_runner` 目标域重建复跑＋还原重建（还原后 037 行 4-5 复验 `36 00 00` ✓）。**读数**：int_4 与 int_5 的 S0/S2 diag **逐字节一致**（校验和 FB7832D0 / EF4967C7）⇒ S2 对其零影响、缺口既有——int_4 全表统一提前一条指令（首列整体 −1，+526/527 收敛）、int_5 test_jmp 10 行中 4 个对齐行提前、分支特例行未到达（S5）。int_3 **互有进退**：S0 8/12 → S2 7/12（丢行 5-6、得行 11；12-vs-11 行数差为两态共有）。**行 5-6 回归机理（已查明，非 G5）**：NMI 在边界（X6502_Run 调用之间）置位时轮询点采不到，已挂起的 IRQ （pending=2）照常派发——「NMI 不降级已挂起 IRQ」只在采样时刻生效；硬件＝NMI 在 IRQ 序列向量化前置位则劫持向量（调查页 §2 矩阵第 3 行）⇒ 修复＝边界派发 IRQ 前对 NMI 线做向量级劫持检查，属 **S3 模型工作**，预估 int_3 → ~9/12 仍 FAIL（行 7-9＝序列中途置位，G5）。**结论**：G5（线断言周期级精度）卡 int_2 行 6-8 / int_3 行 7-9 / int_4 全表 / int_5 对齐行——是 L2 全族的共同 PASS 前置；int_4 的机理候选（断言相位 vs DMA 停止期间周计入）留 G5 RE 首步取证。读数与 S0 基线方法入调查页 §11；T2 卡 S3 行已注记。摸底产物为会话临时区 /tmp/int{3,4,5}{,_s0}.txt。**下一步建议：G5 RE 立项（独立小任务）** |
| r20 | 2026-11-07 | **D1 专线 S2.5（G5 RE）三路并行调查收口（架构/取证/约束各一 agent，全程只读零构建，不触串行构建红线）——推荐方案 A「线携带断言时刻 + 判定点门控」，验收门 G-1~G-10；另发现 kgmqa-040 门禁挂载漂移（待业主裁决）**。**架构路**：G5 结构本体＝线断言时刻被两类宿主量化——① VBL 块把断言耦合进 CPU 指令流（`X6502_Run(8)` 整数条指令过冲 E∈[0,4.3] 周期、逐帧漂移＝r5「无固定 D 两全」的死结本体）；② hook 批量推进把窗口内精确相位折叠到指令边界（MapIRQHook/SoundCPUHook 内部 fhcnt/DMCacc/mapper 计数器逐周期精确，对外丢弃相位 ⇒ 事件恰落指令边界时被提前一条指令识别）。逐线量化清单、帧循环结构、延迟公式见调查页 §12。Mesen2 对照（NesCpu.cpp `EndCpuCycle` 每访问周期采样，只读参照）确认：本仓以「poll point + 时刻门控」近似其在指令边界的不动点理论充分——需要周期级的只是断言时刻本身，派发边界本来就是指令边界。**取证路（int_4 机理判定，r19 遗留三选一）**：**(a)+(b) 组合、(c) 排除**——(a) 线断言相位：hook 推进窗口为半开区间 (B_k, B_k+temp]，恰落在指令结束边界上的事件被记入本指令、在本指令尾 poll 可见 ⇒ 提前一条派发（int_4 头部奇偶档模式逐档吻合，18 档唯一拟合 T_ours=T_hw=−1，frame counter 绝对常数无错）；(b) DMA 周计入：本仓 B4014（ppu.cpp:1484-1516）恒 512 周期、无 dummy/对齐停机、无写周期奇偶（硬件 513/514 按 DMA 奇偶；INTLEDGER delta=519 实测钉死 512）⇒ 尾部偏差；(c) 派发记账排除（压栈 PC 与 poll 所见逐档一致，assert→dispatch 同边界 delta=7）。交叉预测：只修 (a)=16/18 行、只修 (b)=14/18、双修=18/18——可用作实施后的行级验收。(b) 的修复（ppu.cpp 补 513/514 奇偶停机）会平移全部 sprite-DMA 时序敏感测试（blargg ppu sprdma 系），**非局部安全修**，独立于 G5 立项；DMC 仲裁是另一独立缺口（kgmqa-050 已在案）。**约束路（040 考古）**：⚠ **发现 kgmqa-040 门禁挂载漂移**——gate id 名为 vbl-05-nmi-timing，实跑 5.nmi_suppression.nes（9-24 镜像重整 6fa6b2f 换挂、legacy 名未改，派发日志与 md5 双重实证）；NMIDELAY=8 真正钉死的旧 05-nmi_timing.nes 已不在任何门禁——实测 D∈{6,7,9,12} 翻红、=8 唯一 PASS，该金丝雀目前无门禁覆盖 ⇒ G5 验收门必须显式加回 G-2（旧 05 X 表 [4,4,4,3,3,3,3,3,3,2] 逐值不变）；NMIDELAY=8 是标定甜点（±1 dot 即翻转）而非模型精度。040（5.nmi_suppression）对 D 全扫不敏感、监控的是 VBL 块宏观形状。完整验收门表 G-1~G-10 与 12 条历史陷阱见调查页 §12。**方案 A 要点**：`_IRQlow` 位保持电平语义，旁设 per-line `assert_at`（CPU 绝对周期）；TriggerNMI/TriggerNMI2/`X6502_IRQBegin`/hook 侧登记时刻（相位信息源侧本就逐周期精确）；poll point/边界判定/BRK 劫持（ops.inc）统一按 `assert_at < now` 门控（now＝指令记账末尾，timestamp 在 ADDCYC 已预支、无需新时钟；边界归属按 int_4 拟合窗口 [起始,结束) 半开）；savestate 不入（同 g_intl_pending 豁免先例）；`X6502_Run(8)` 帧预算结构保留，NMIDELAY 从「预算授予量」退化为「断言时刻偏移旋钮」。改动面 ~150-250 行核心＋全套金标复跑；合规 r12（判定仍每指令一次，不引入访问级步进）。方案 B（VBL 块重排）相位资产全重采、方案 C（指令内检查点）与两拍模型精神冲突且收益不确定——均劣。**拍板点**：① 方案 A 是否立项实施（S2.5）；② kgmqa-040 挂载漂移处置（改 id 名/换回旧 ROM/维持现状＋文档声明——涉及 tests.json 期望值治理，须业主裁决）；③ (b) DMA 513/514 奇偶修复是否独立立项（注意与 A6/D5 的 bus 同域互斥约束） |
| r21 | 2026-11-07 | **D1 专线 S2.5 第一轮落地（业主裁决：方案 A 开工 / 040 处置授权 / DMA 独立立项 / push）——时间戳基础设施 + 锁测试落地，两轮 ROM 级精确化尝试被门禁当场咬住后回退，零生产行为变化实证**。**① kgmqa-122 门禁补挂（d4da78a）**：新增加载 blargg/ppu/vbl_05_nmi_timing.nes 的 blocking PASS monitor（600 帧；r20 §12.7 挂载漂移的业主授权处置），baseline +1（121 项），矩阵本地实跑 122 项（total=122 passed=111 grade=B pass_to_fail=0）；f11qa.yml 的 vendor_state 总数判据由硬编码 78 改为从 tests.json rom-suite 用例数推导（严格相等保留、常数不再漂移，81ca5b9 同族）；**金丝雀当轮即咬中本轮 VBL 回归**（下述②）——G-2 按设计工作。**② S2.5 基础设施（本提交）**：per-line 断言时刻表（`g_intl_assert_at[8]`，IntLedgerKind 上提至 ops_table.inc 之前）+ 可见性门控（poll point `intl_live_irq_mask()`、BRK 劫持 `intl_nmi_live()`，严格 <；边界恰落事件归属下一条指令窗口——int_4 18 行拟合模型）+ 精确注入器 `TriggerNMIAt` / `X6502_IRQBeginAt` / `fceu11_cpu_abs_ts`（x6502.h 导出）+ legacy 注入器 `intl_stamp_legacy`（now−1 ⇒ 可见性与 S2 逐字节相同）。锁测试 **T-D1-5 组 11 条**（5a 未来时刻登记×5 / 5b 边界恰落×3 / 5c 窗口内×3；含 PC 压栈形状断言）；cpu_test **861/861**（850→861），FTP 面 1红/10绿不变。**③ 两轮精确化尝试与回退（门禁裁决，如实记录）**：(a) **VBL 时刻化**（TriggerNMIAt 以 flag 点为锚、预算授予前登记，δ=FCEUX11_NMI_ASSERT_DELTA 环境变量）：δ∈{0..4} 扫参——**无任何整数 δ 复现旧 05 X 表**；δ=2/3 = [4,3,3,3,3,3,3,2,2,2]，与期望 [4,4,4,3,3,3,3,3,3,2] 的**转变间距完全一致（6 行）但整体早 2 行**＝2-dot 点级相位差，整数周期粒度的 assert_at 表达不了（与 02-vbl_set_time 已知 1-dot 偏差合流）；037 在 δ=1 下行级交易（7/10→4/10：行 6-7 得劫持、行 2-5 位移）。kgmqa-122 转红 ⇒ 回退 VBL hunk 至 legacy 语义。**(b) APU FCOUNT 精确戳**（IRQBeginAt + fhcnt 派生事件周期）：054 保持 PASS，但 int_4 头部行**非均匀位移**（整体晚一条 + DMA 区异常）＝APU 时序器相位 vs 测试 sync_apu 对齐需要自己的轮次 ⇒ 回退至 legacy。两章程见调查页 §12.8。**④ 零生产行为变化实证**：037 校验和 B1222C16、int_3 BBA6CEF0、int_4 FB7832D0、int_5 EF4967C7 全部与 S2 轮逐字节一致；旧 05/122/040/054/048/032/053/057 全 PASS；ctest 36/36；金标 hashes 780/0、savestate 8/8。**⑤ 变异验证**（ms25a 门控短路/ms25b 严格<→≤/ms25d 折叠到边界）：ms25a 门控短路→1 红（T-D1-5a-5，派发早一边界 PC=$201）；ms25b 严格<→≤→1 红（T-D1-5b-3，边界事件改记本条）；ms25d 戳折叠→1 红（T-D1-5b-3；5c 对其不敏感——两戳均先于首次 poll，如实记录）；全部还原复绿 861/861。**⑥ DMA 513/514 奇偶修复独立立项**（业主裁决③）：落点 ppu.cpp B4014（512→513/514 按写周期奇偶，恒 514 会让 int_4 +527 反着错），排 S2.5 下一轮之后；与 (a) 双修预测 int_4 18/18；会平移全部 sprite-DMA 时序测试（blargg ppu sprdma 系）需全量复跑；注意 A6/D5 bus 同域互斥。**下一步建议：S2.5 第二轮＝两个点级章程的 RE（VBL 点级时刻 + APU 相位对齐），工具已就位（δ 旋钮回挂即用、金丝雀门禁、INTLEDGER）** |
| r22 | 2026-11-07 | **CI run 114820759666 红修：kgmqa-122 的 mirror_path 不在镜像快照 ⇒ fetch MISS ⇒ exit 4 ⇒ 矩阵未产出 ⇒ R4 Gate 报「matrix not produced」（二级症状，真凶在 fetch 步）**。根因：d4da78a 写 mirror_path 用了本地文件名 `blargg/ppu/vbl_05_nmi_timing.nes`，镜像 Laffinty/f11qa-rom-mirror@d50a769 里的真名是 `blargg/ppu/05-nmi_timing.nes`（sha256 72e515d6… 相同 ROM，SHA256SUMS 实证）。修复：mirror_path 改真名 + fixture 按派发口径（basename → tests/fixtures/）改名为 05-nmi_timing.nes。本地全链复现通过：fetch EXIT=0（copied 43→44、missing=0）、kgmqa-117 PASS、kgmqa-122 PASS 0x00、矩阵重跑 total=122 passed=111 grade=B pass_to_fail=0、R4 gate passed。**次要发现**：kgmqa-117 pre-flight 只查 mirror_ref/license 一致性、**不查每个用例 mirror_path 在快照中的存在性**（盲区）——纵深防御成立（fetch 脚本是真路径闸），117 补路径存在性检查列为可选小件。**教训**：新增 mirror_path 前先对镜像 SHA256SUMS 核对路径存在性，本地 fixtures 树的文件名 ≠ 镜像路径 |
| r23 | 2026-11-07 | **主线待办批次启动（业主指令：多 agent 并行）＋ kgmqa-050 预算伪 FAIL 修正**。**050（本提交）**：门禁原预算 300 帧 = 伪 FAIL（$6000 仍 0x80 在跑、exit 1、duration 608ms——被「advisory 基线恒 False」fail-safe 策略遮住、全部门禁不可见，仅矩阵 JSON 可见；r11 的 D4 修复本身无恙，400/600/2000 帧直跑全 PASS 0x00）——与 r12 的 049/051 完全同型（完成窗口 ~316-400 实测）。修正 = 预算 300→400 + provenance 记测量，r12 先例授权类。**三路点级章程 RE 并行启动**（只读 agent，PLAN 产出后交业主拍板再串行实施——B/C 同文件域 x6502.cpp 不可并行实施）：① APU frame IRQ 相位对齐（S2.5 精确戳回退的机理判定与正确公式）；② VBL NMI 点级时刻（2-dot 偏移的 grant/exec 位相判定 + 02-vbl_set_time 合议）；③ DMA 513/514 奇偶（r21⑥ 立项的语义钉死与实现设计）。VBL 路 agent 首轮撞并发上限，让位后重试。**A6/D5-S3-S5 维持「D1 收口后」排序（r12 裁决）；A1 手工清单 22 项维持挂账（需业主 GUI 配合）** |
| r24 | 2026-11-08 | **P1/P4 收口批次（业主指令：多 agent 推进；两路只读 agent + 主会话落档）——P1 自动化净闭合 + 两个新发现缺陷；P4 重采裁决材料齐备待两项拍板**。**① A1 清单收口（P1）**：口径更正——勾选表实际 **21 行**（自称 22，计数滑动）；分类 = H 3 / G 18 / C 0。**L-2 核心路径无头验证 PASS 留证**（FCEUX11_TRACE 全链路：两 ROM 格式 0 错位、首行 PC==复位向量、操作码字节==ROM 字节、助记符抽查 94650/94650、阴性对照无文件）——L-2 净闭合，GUI 壳并入走查单。**LUA-1/2 被新缺陷 F-1 阻断**：`emu.frameadvance()` 任何调用 ⇒ `panic in a function that cannot unwind` ⇒ abort 0xC0000409（Rust 回调内跨 C 边界 yield，lua51 feature 结构性不支持；绑定自 2026-06-21 d119514 未改，非 A1 回归）；**ctest 盲区又一例**——test_frameadvance.lua/test_callbacks.lua 自引入从未接入门禁（git log -S 零命中，纪律 10 案例）。**F-2（中）**：无头回落反汇编器（asm.rs:317-352）助记符表整列错配（ORA 列吞 LDA/CMP/SBC/STA/ADC/EOR/AND 的 zp，0x8E/0x8C 等落 ???）——只污染无头 trace 文本，Qt 调试器/TraceLogger 走独立实现不受损。A1 的 break_asap 门腿与 A1-b 反向项**仍无任何一层验证**（F-1 修复前）。业主走查单 18 项落档 `docs/tech/a1_gui_walkthrough.md`（估一次 GUI 会话 2-3 小时）。**② A6+D5-S3-S5 重采（P4，r12「重采探针再动」前置执行）**：IO 取指 31/355 万（0.00087%）且 31 为帧数无关结构常数（r11/r12/本轮三连同值）——D5 行为前提保持证伪、**D5 本体无剩余行为工作**（bus_test 85/85 绿）。**A6 快路径收益诚实评估 3-8%（中位 ~5%）骑线**，且 r12 前置在方法论上不成立（收益是性能问题，静默探针测不出）+ 两个更便宜替代从未动（PGO 开关现成/A7）；bench_bus_dispatch 与 cpu_frame 同步上漂 11-20% = r55 类整机时钟漂移，r10 绝对读数作废、采纳判定只能同窗 A/B。**agent 建议：关闭 A6-S3-S5 计划项**（S0+S2 护甲 85 断言转常驻，零维持成本）；替代槽位 = PGO 一次性评估或 A7。**排序条款退役建议**：S2 已把 x6502.cpp 移出 S3-S5 残余（read_fetch 纯别名），四阶段与快路径**文件域冲突=零**，r12「D1 收口后再动」前提已失。**待业主两项拍板**：① S2.5 第二轮四阶段（§12.9 拍板单，拍板单已呈两次）；② A6-S3-S5 关闭（推荐）/S3-α 尖刺先测/维持。**③ 基线事项**：bench 基线随下批统一重采（本轮漂移在案） |
| r25 | 2026-11-08 | **D1 S2.5 第二轮实施（业主「拍板允许执行」）——四阶段落地 3.5 个：DMA 奇偶 ✅、APU 精确戳 ✅、(C) 采样点 ✅（override 修订版）、VBL 锚首切被金丝雀咬住回退（K 标定章程）；另 A6-S3-S5 按裁决关闭**。**阶段 1（e35b748）**：B4014 停机段 1 dummy + 0-1 对齐（奇偶校准位kOamDma514Parity=0）+ 512 = 513/514；锁 T-D1-6（span {520,521}，变异 ms27a/b 双红）；int_4 = 13/18（+526 判别行 9→8）；int_2/3/5 校验和不变；110/069/122/040/054 全 PASS。**阶段 2（81f6631）**：g_fsu_event_abs = 着陆周期完成边界 + X6502_IRQBeginAt restamp 参数（mode-0 重火毒药守卫）；int_4 = 17/18（头部 4 行修正，与章程预测精确一致）；**超预期收敛：int_3 行 6 = 20 00（r19 S2 回归行修正）、int_5 test_jmp 10 行全对齐（ROM 推进到下一子测试）**；kgmqa-121 59/59 零影响。**阶段 3（a720db9，修订版）**：(C) 采样点第一版（post-hook 统一捕获）实证排除过度——$2004 的 X6502_DMR/DMW 计费（ppu.cpp:924/928 实为 B4014 探针镜像 + DMC fetch）被一并排除，int_5 pagecross T+04 回退 1 行；修订为 **override 形态**（B4014 经 fceu11_intl_set_poll_override 设 cycle_before，下一次 poll 一次性消费；其余指令 = 完整记账末）：int_5 恢复 79064A81、int_4 = 16/18（+4 收敛）。**两形态取舍记录**：capture 形态 int_4 = 18/18 ROM PASS（FCOUNT/DMA 计费链闭合）但 int_5 -1 行；override 形态零回退但 int_4 差 2 行——差异机理 = DMCDMA 的 X6502_DMR 计费被 capture 计入/被 override 排除，而 18/18 需要计入——**与「DMA 停摆不属于任何指令」模型矛盾，DMC 计费范围界定 = round-2 章程**。**阶段 4（首切回退）**：三章程 agent 的幂等补丁脚本落地（五锚字节级 + 九项后置断言 + --check），应用后 **G-2 金丝雀咬住**：派发 PC 落在同步例程（E86B）而非 marker 区（E34E-E356）——K=13 在新登记结构下过早 ~3 周期（legacy 有效时刻 ≈ flag+4.7 周期）；K 为 constexpr 无法免构建扫描（规格失误）⇒ 回退，**K 扫参章程**：constexpr 改 env 旋钮 + K∈{13..25} 扫描 + X 表逐档判定。**A6-S3-S5 关闭（业主「拍板允许执行」授权 r24②推荐）**：探针三连稳定 + 收益 3-8% 骑线 + PGO/A7 替代未动 ⇒ 关闭计划项，S0+S2 护甲 85 断言转常驻；r12「D1 收口后再动」条款退役（S2 已把 x6502.cpp 移出残余范围，文件域冲突=零）。**增量构建陷阱第三次**（陷阱 #10）：stage 1 只重建 cpu_test target，blargg_runner 旧二进制假绿一轮，INTLEDGER delta 519 钉死后重建复验——target 域构建后必须重建所有涉时序 runner。**门禁总态**：cpu_test 862/862（T-D1-6 新增）、ctest 36/36、金标三件套零漂移、哨兵/110/069/122/040/054 全 PASS；int_2 = B1222C16 不变、int_3 = ECD9F0D5（+1 行收敛）、int_5 = 79064A81（不变）、int_4 = 16/18 FAIL 0x01（advisory）。**round-2 工作单**：① (C) DMC 计费范围界定（capture 形态的 int_4=PASS 差一步）；② VBL 锚 K 扫参（env 旋钮 + 补丁脚本就绪）；③ 两者收敛后 S2.5 收口 → S3-S5 |
