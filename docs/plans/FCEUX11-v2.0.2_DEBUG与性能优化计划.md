# FCEUX11 v2.0.2 — DEBUG 与性能优化计划

> **STATUS: DRAFT（待拍板，未动任何代码）**
> **立项目标**：以 DEBUG（调试能力/排查效率）与性能优化为主，另对
> `docs/plans/long-term-evolution/` 的 10 项长期难题做**可行性重估**，可行者并入本计划。
> **基线**：`4c75fe8`（v2.0.1 收口后）· 矩阵 **108P / 12F** · grade B · `ctest -LE perf` **33 项全绿**
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

1. **NES 零回归**：`ctest -LE perf` 33 项全绿；矩阵 `pass_to_fail = 0`；金标哈希
   （golden_hashes / golden_savestate）不动，要动须人工授权。
2. **变异验证**：每条新锁测试都要能把「修复前的值」改回去看到它转红；全绿不算证据。
3. **instrument-first**：改行为前先上探针/测量（`docs/tech/precision.md` §4）。
4. **bench 与构建/测试不并行**（r54 ②：并行会污染测量）；`bench_tolerance_test` 是
   advisory（r55：本机 full_frame 随时钟上下文漂移 +23%），读数只用来追**大幅持续**回退。
5. GBA 长期 Alpha，不阻塞本体发版；GBA 改动后仍跑 `ctest`（不变式 1 对两台机一体生效）。

---

## 一、先立测量，再动代码（P0）

本轮扫描发现**现有性能测量有两个盲区**，不先补口径，后面的工作做完无法在门禁里证明：

| # | 盲区 | 证据（✅实测） | 处置 |
|---|---|---|---|
| M1 | `bench_tolerance_test` 三档（cpu/ppu/full）全部 `soundq=0` 走低质量路径，**真实游玩的 NeoFilterSound 484 抽头 FIR 根本不在基线内**（`tests/f11qa/benchmarks/bench_tolerance_test.cpp:96-99`） | ✅ | 新增 **soundq=1 测量位**（bench_full_frame 加第四档或独立 bench），基线重采后再动 A5 |
| M2 | GBA 帧导出/savestate 无微基准，C1/C2 的收益只能靠体感 | ✅ | 借用现成 `#[ignore]` 探针（`save.rs:885` `measure_a_state_on_disk`）扩一个帧导出计时探针，只作量级参照不进 ctest |

**验收**：新测量位跑出稳定读数（best-of-3 口径同 baseline_v1.10.json 方法论）；
**基线重采须单独提交**（照 `tests/fixtures/generate_bench_baseline.ps1` 流程）。

---

## 二、NES 性能线（线 A）

> 全部 ✅实测 有代码坐标。收益量级以「60 帧 / 68.7ms 基线」为参照系。

### A1 调试门控三合一 —— 首个工作项（收益/风险比最优）

| 项 | 位置（✅实测） | 现状 |
|---|---|---|
| A1-a DebugCycle 每指令无条件执行 | `src/x6502.cpp:606` → `src/debug.cpp:901-963` | 每条 6502 指令先做：vblank 浮点换算（`debug.cpp:909`，含 double 除法）、`GameInfo` 判定（:917）、**重新取指 1–3 次 `GetMem`**（:922-936，与真取指重复）、寻址解码再 0–2 次 `GetMem`（:938-954），然后才在 :956 判断「有没有断点/单步/计数断」决定要不要进 `breakpoint()`。**零断点时整段纯浪费** |
| A1-b 每指令调试记账 | `src/x6502.cpp:608`（`IncrementInstructionsCounters` 跨 TU 调用 + 两次全局自增，`debug.cpp:601-604`）、`:611`（`e1_last_pc = _PC` 每指令存，仅供 env-gated E-1 探针消费）、`:621`（`map_irq_hook()` 每指令 atomic acquire，`cpu.h:131-133`） | 无任何开关 |
| A1-c viewer 回调链 | `src/ppu_rendering.cpp:787`、`:1313` → `driver_callbacks.cpp:182-187` 间接调用 → 窗口空判早退 | 每帧 480+ 次调用链 |

**改法**：文件级 `debugger_active` 快速门（条件现成：`debug.cpp:956` 的 8 路 OR 加上
`debug_loggingCD` 与 TraceLogger logging 状态）；A1-b 三件套并入门控（`e1_last_pc` 移入
`e1_cpu_trace_on()` 内；irq hook 指针改普通读 + mapper init 处一次同步）；
A1-c 同门。**门打开时行为逐字节不变**——`GetMem` 在 debug 路径本就置 `fceuindbg`
抑制副作用（`debug.cpp:349-351`），输出只喂调试器，金标哈希不动。

**验收**：`ctest -LE perf` 全绿 + `golden_hashes.json` 逐帧一致 + bench_cpu_frame 明显下降
（目标 ≥5%，以实测为准）+ **手工场景**：断点/单步/step-out/条件断/TraceLogger 全部照常
（这是「调试功能不回归」的变异验证对应面）。

### A2 APU 帧计数器批量步进（收益高，风险中）

- 位置（✅实测）：`src/x6502.cpp:623-624` 每指令调 `FCEU_SoundCPUHook` →
  `src/sound.cpp:685`，体内 `while(rem>0){ rem--; FrameCounterTick(); }`（:706-722），
  `FrameCounterTick` 是 5-6 次串行比较链（:570-634）。
- 现状：每帧 ~29,830 次循环迭代+函数调用，只为打 4-5 个定拍事件
  （7457/14913/22371/29828-30）与 `$4017` 写的复位成熟点（fc_reset_in）。
- 改法：批量步进——算 `min(下一事件, fc_reset_in)` 一次跳转，事件边界精确命中再触发；
  **fc_reset_in 的 3/4 周期奇偶抖动语义保持**（`sound.cpp:1256-1262` 注释）。
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
- `src/sound.cpp:1337-1338` memset 覆盖 `[left, 40000)` ≈160KB，下帧只写 ≤SOUNDTS≈3 万项
  ——收窄到 `[left, left+SOUNDTS)`。注意 hotfix3 D-4 REVERTED 的教训（:1327-1336 注释）：
  **trailing memset 必须保留**，只许收窄范围不许删除。

### A5 NeoFilterSound 无损 SIMD（收益高，仅 soundq≥1 真实路径；前置 = M1）

- 位置（✅实测）：`src/rust/crates/fceux11-media/src/filter.rs:299-339`，
  `NCOEFFS=484 / SQ2NCOEFFS=1024`（fcoeffs.rs:8-9），每输出样本 2×484 次 i64 乘累加
  （线性插值双相）≈77 万 i64 MAC/帧@48kHz。
- 现状：i64 点积在 AVX2 无廉价乘加 ⇒ 自动矢量化基本无效（⚠️未实测反汇编）；
  WaveHi 样本有效位在高 16 位（`sound.cpp:781` `&(~0xFFFF)`）。
- 改法：先无损重构（acc2 复用相邻载入、内层手动 SIMD 保 bit-exact），16 位重写列为
  有损备选**不做**。哨兵：`apu_wav_diff_test` + golden_wav 逐字节。

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

## 三、NES DEBUG 线（线 B）

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

`memory.readbyterange`/`writebyterange`（现仅 10 个导出，`bindings/memory.rs:20-104`）。
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

### C1 帧导出热路径（主菜）

- 现状（✅实测）：`frame.rs:321-334` `gba_frame_buffer` 每次调用
  **2×153.6KB `Vec` 分配 + 3 次全帧写/拷**（`frame_bytes` 转换写入 :865-893、
  `copy_from_slice` :329、`copy_nonoverlapping` 到 dst :333）+ 逐像素标量 BGR555→RGBA
  （38,400 像素 × `scale5`）。C++ 侧消费零分配（`gba_load.cpp:412`）。
- 改法：① 复用常驻缓冲消掉两次分配；② 转换**直写 dst** 合并一整份拷贝；
  ③ `scale5` 查表（8K 表）或 SIMD——纯函数，锁测试可兜。
  **水印「只画一次」语义与 `GBA_ERR_CAPACITY` 语义保持**。
- 验收：`cargo test -p fceux11-rust --no-default-features --features gba --lib` 248 项全绿；
  帧字节前后逐位一致（dump_frames 探针 diff）；帧导出计时探针（M2）读数下降。

### C2 savestate 延迟（双编码 + 指纹缓存）

- 现状（✅实测）：每次保存 = **2 次全量 JSON 编码**（`gba_load.cpp:267-275` 先
  `gba_savestate_size` 量大小再 `gba_savestate_save` 再编码，release 单次 2.4ms）；
  每次存/读档全 ROM 重算 FNV-1a 指纹（`save.rs:141`、`frame.rs:405/438`，
  4-32MB 卡带为 ms 级）。
- 改法：宽 buffer + `GBA_ERR_CAPACITY` 重试（ABI 已支持，注释自述）；指纹在
  `gba_load_rom_bytes` 时算一次缓存（换 ROM 失效）。
- 验收：savestate 金标（`golden_savestate_test`）不动；`measure_a_state_on_disk` 读数对照。

### C3 GBA 诊断工具文档化 + 计数更正（DEBUG 方向）

- 把 GBA 探针族写成一页 HOWTO：`FCEUX11_DIAG_LOG`、`GBA_PROBE_*` 环境变量族、
  r53/r56 复用探针（`frame.rs:2669-2677` `probe_why_a_game_stalls`、
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
| **P0 测量口径** | M1（soundq=1 bench 位 + 基线重采）、M2（GBA 计时探针） | 无 |
| **P1 低风险高收益** | A1（调试门控三合一）、C1（GBA 帧导出）、C2（savestate 延迟） | P0 |
| **P2 深化** | A2（APU 批量步进，先锁测试）、A3、A4、B1、B2 | P1 |
| **P3 长期难题首批** | D4、D9、D7-A | 可与 P2 并行（不同文件域） |
| **P4 视余量** | A5（FIR SIMD）、B3、D7-B、D3 里程碑、D5+A6（同窗） | P0 的 M1；A6 需全套回归护甲 |
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

- [ ] `ctest -LE perf` 33 项全绿；矩阵 `pass_to_fail = 0`
- [ ] 金标三件套零漂移（或漂移均有人工授权记录）
- [ ] bench：cpu_frame 与 soundq=1 档位实测改善（读数入库，advisory）
- [ ] A1 手工调试场景清单全过
- [ ] GBA 锁测试 248 项 + vendor 242 项全绿；帧字节 diff 一致
- [ ] 入选长期难题项：各自验收（050 → kgmqa-050 PASS；107 → kgmqa-107 PASS；
      081-A → 0xFE 消失且失败类型重分类经评审）
- [ ] AGENTS.md 计数更正与 GBA 探针 HOWTO 落档

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

# 长期难题单项（示例：050）
& build\tests\f11qa-rom-runner.exe --kgmqa-id kgmqa-050-cpu-dummy-writes-bisqwit --tests-json tests\tests.json --frames 3000
```

---

## 附录 A · 环境变量开关全表（✅实测，B 线改动同步本表）

| 变量 | 位置 | 作用 |
|---|---|---|
| `FCEUX11_BUSDB=1` | `src/bus.cpp:102` | CPU 数据总线 DB 记录（kgmqa-038 实验，默认关） |
| `FCEUX11_E1_TRACE=1` | `src/ppu.cpp:352`、`ppu_rendering.cpp:1524`、`x6502.cpp:400` | PPU/NMI/CPU 事件 trace |
| `FCEUX11_E1_NMIDELAY=N` | `src/ppu_rendering.cpp:1541` | NMI 注入延迟，默认 8 |
| `FCEUX11_OPENDECAY_PROBE=1` | `src/ppu.cpp:372` | open-bus 衰减 + SPR DMA 事件日志 |
| `FCEUX11_E3_TRACE=1` | `src/sound.cpp:48` | APU $4017 写 trace |
| `FCEUX11_MMC1_PROBE=1` | `src/boards/mmc1.cpp:39` | MMC1 板几何/映射 dump |
| `FCEUX11_MMC3_PROBE=1` | `src/boards/mmc3.cpp:52` | MMC3 扫描线 IRQ 探针 |
| `FCEUX11_FME7_PROBE=1` | `src/boards/69.cpp:41` | FME-7 bank tag dump |
| `FCEUX11_DIAG_LOG=<path>` | `src/drivers/Qt/main.cpp:325` / `gba/mod.rs:77` | Rust panic 落盘 |
| `FCEUX11_GBA_PROBE_DIR` + `GBA_PROBE_*` 族 | `cartprobe.rs:351`、`frame.rs:2343-3157`、`save.rs:758` | GBA 卡带/时序/帧差分探针 |
| `FCEUX11_BENCH_BASELINE` | `bench_tolerance_test.cpp:249` | bench 基线 JSON |
| （B1 新增）`FCEUX11_TRACE` | — | TraceLogger 无头入口 |

## 十、变更记录

| # | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-04 | 初稿：只读扫描（NES 性能/调试、GBA 性能/调试）+ 长期难题 10 项可行性重估 + 排期与出口判据。未动任何代码 |
