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
| **专线** | D1（037，独占分支） | 独立窗口 |
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
