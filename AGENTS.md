# FCEUX11 — 接手须知

> 本文件是**入口**。第一次接触本仓库、或换人接手时先读这里，再进治理文档。
> 里面每条「坑」都是实际卡过一次才写下来的，不是预防性提醒。

## 仓库在做什么

FCEUX11 是 NES 模拟器。`wip2.0` 分支上并行着 **GBAEUX11 v2.0**：把一个 GBA 硬件核心接进来，
做成 `.gba` 可玩的第二台机，同时 **NES 侧必须零回归**（`108P / 12F`，grade B）。

## 分支

| 分支 | 用途 |
|---|---|
| `main` | 发布线 |
| `wip2.0` | **已合入 `main`，远端分支已删除**（2026-11-09 核实：`git ls-remote --heads origin` 只有 `main` 与 `wip203`；GBAEUX11 v2.0 的全部成果——`src/gba_load.cpp`、`src/rust/src/gba/*`、v2.0/v2.0.1 归档计划——都已在 `main` 上） |
| `wip203` | **已合入 `main`，远端分支已删除**（2026-11-12，PR #13 真合并；v2.0.3 全部成果——三段式收口 r6–r19、S4/S5、复现器清零——都已在 `main` 上）；计划见 `docs/history/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md`（**已随 v2.0.3 发布归档**，r1–r20，内容冻结） |

## 治理文档（事实源）

| 文档 | 管什么 |
|---|---|
| `docs/history/plans/FCEUX11-v2.0_GBAEUX11构建计划.md` | **已收口归档（r57）**：范围、阶段编排（§八）、已知限制（§9.1）、**不变式（§十）**、变更记录（§十四 r1–r57）——历史与取证事实源，**内容冻结不再续写**；长尾承接见下方「长尾」节 |
| `docs/history/plans/FCEUX11-v2.0.1_GBAEUX11精度优化计划.md` | **已收口归档（用户裁决 2026-10-07）**：P0/线B/P4/线A/P3/P1 全部完成，**AGS 最终 30/33**；内容冻结不再续写。剩余 DMA DISPLAY START 与 DMA PRIORITY 两项 RE 见「长尾」节 |
| `docs/history/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md` | **已收口归档（2026-11-12）**：GBA **卡带兼容性**线——「商用手写卡带跑不起来」类问题。三段式：**S1 可复现门禁**（自建最小夹具 + 探针改造）→ **S2 止血**（vendor 核 panic 改容错、SWI 兜底）→ **S3 修根因**（SWI `0x16–0x2A`、存档芯片模型、GPIO/RTC、ROM 窗口镜像）。§一 是**已确认根因的事实底座**（含被否证的两个假设，**别照着做**）；附录 A 是 GBATEK 位级格式取证基线。⚠️ 随 v2.0.3 发布整体收口（PR #13 已并入 `main`），**内容冻结**；GBA 线的长尾项见下方「长尾」节与 v2.0.4 计划 §六 |
| `docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md` | **在途构建计划（2026-11-12 立项并升格，工作分支 `wip204`，NES 侧）**：v2.0.2 残留 review 报告的逐条复核结论 + **V1–V8 逐项解决方案**（目标/现状坐标/步骤/验收/风险）。**不纳入 v2.0.4 的项目已按业主指令删除**（不进理由与历史见 git 历史与 review 报告）。⚠️ 关键前提：**这些精度项的判红体系收益 = 0**（advisory + 冻结基线 False）⇒ 决策依据是「用户可感 / 解锁 / 防呆 / 成本」。**2026-11-13 追记（r7）**：本计划已追加**批 4/5/6 = V9–V25**（内存安全/线程安全/mapper 正确性/输入正确性面，由 `docs/history/plans/FCEUX11-v2.0.4_追加.md` 逐条核验后并入；原稿 12 项中 7 项属实 / 5 项部分属实，核验中**新发现 V24 / V25**；⚠️ **原稿有 15 处坐标与事实偏差，以主计划 §二 各节末的「文档偏差」为准，别照原稿做**；**r8 追记：V3 / V4 / V9 / V24 已落地**（批 2 全清 + 批 4 的两项），门禁 `ctest -LE perf` **36/36** + 六条 Rust 锁测试全绿（`fceux11-utils` 51/0 为 V24 新增）；**r9 追记（2026-11-14）：第一梯队四项落地——V11（VRC7 读档悬空指针）/ V13（AVI 四索引 atomic）/ V15（JSreturn 原子转移）/ V19（GUID C++ 兜底 + `guid_test` 入门禁 36→37）**，V9 变异腿实跑闭合（6145/8192 越界与数值推导逐字一致），门禁 `ctest -LE perf` **37/37** + 六条 Rust 锁测试全绿（与 r8 基线逐字一致）；⚠️ V19 的关键发现：**单腿变异测不出 C++ 半边**（双层零化互相掩盖）⇒ 变异验证须双腿齐拆；**r10 追记（2026-11-14）：V9 两条尾巴全清**——业主人工下载 nesdev 材料，#851 复现 ROM（mapper218bad.nes，SHA256 `efed56a8…`）在修复版装载 + 600 帧无崩溃，真 ROM 变异腿在本仓布局为**静默损坏**非段错误（布局无关越界证明由 mapper_load_test 承担）+ V6 信源取证（wiki `Standard controller` 只记 **bit2**，`Famicom microphone` 独立页不存在，「位 2/3」定性未证实；材料在 `Research_only/w204/`，gitignored）） |
| `docs/history/plans/FCEUX11-v2.0.4_追加.md` | **已归档（2026-11-13，r7）**：v2.0.4 的**静态分析技术方向稿**（上游 2.7.0 + master 对标的 12 项候选）。已逐条核验并并入 v2.0.4 构建计划（V9–V25）；**内容冻结，且其中的坐标/事实已被核验修正**（15 处偏差登记在主计划 §二）⇒ **别照原稿做** |
| `src/rust/crates/gba-core/ATTRIBUTION.md` | 上游出处、本地补丁集**逐处**说明、重新 vendor 流程 |
| `COPYRIGHT_AUDIT.md` §5 | Rust vendor 树在 `src/` C/C++ 扫描范围之外的归属声明 |

> **NES 侧**（v2.0.2 线）的计划文档已重组为两份，**均在 `docs/history/plans/`**：
> **已完成**部分见 `docs/history/plans/FCEUX11-v2.0.2_DEBUG与性能优化-已完成项归档.md`
> （事实源，内容冻结），
> **未完成**部分见 `docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`
> ——⚠️ **那份是 2026-11-08 的快照**，已在 v2.0.3 收口后逐条复核，
> **当前状态以 `docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md` 为准**。
> ⚠️ **`docs/plans/` 现在只留「在途」的计划**（v2.0.4），其余一律进 `docs/history/plans/`。
> 三者与 GBA 线无关，别混。

## GBA 与本体的关系（用户裁决 2026-10-01，动 GBA 之前先读这段）

这一节的每条都是**硬约束**，不是偏好。完整表述见计划 §十 不变式 9。

1. **GBA 不接入任何质量检测系统。** F11QA / KagamiQA / jsmolka 都不是它的对象。
   **它不是 S2 出口的门禁，2.0 GA 也不以 GBA 精度为条件**（原 §8.2 第 2 条已作废，见 r42）。
2. **长期 Alpha，不阻塞本体发版。** GBA 的进度、缺陷、半成品都不得成为本体发版的门槛。
3. **与 NES 侧最大化解耦。** 任何「为了 GBA 而改动 NES 既有行为」的做法一律不做。
   宁可**复制一小段**呈现代码，也不泛化 NES 那段（计划 §十 不变式 2 的 r41 改写）。
4. **它的出口判据只有两条**：`ctest` 不回归（33/34），以及它自己的锁测试。
5. **允许的耦合只有这五条**：① 同一进程 / 模拟线程 / 窗口 / 声卡；② 每帧循环里的那一个
   分支；③ `WriteSound` 写入接口（环按**设备**尺寸配，不按机器配）；④ 限速的基准帧率取值；
   ⑤ 打开 ROM 对话框的过滤器条目。
6. **红线**：NES 侧的帧池、纹理、缩放数学、调色板、调试器内存映射、cheats、TAS、NSF、
   录像**不得出现 GBA 引用**。GBA 会话下这些按不变式 7 显式声明为不可用。
7. **由 `src/rust/src/gba/decoupling.rs` 的三条守卫强制**：C ABI 的调用点必须恰为一个文件；
   任何提到 GBA 的 C/C++ 文件必须在显式白名单内；**限速与声卡的四个文件必须不含任何
   GBA 命名**。**加一个 GBA 引用就会变红** —— 新触碰点必须在那份白名单里显式登记，
   与加代码同一个提交。

## 当前进度（2026-10-07）

> ⚠️ **本节只覆盖 GBA（v2.0）线。NES 侧的工作在
> `docs/history/plans/FCEUX11-v2.0.2_DEBUG与性能优化-已完成项归档.md`**（v2.0.2 =
> DEBUG 与性能优化的**已完成**项，事实源）与
> `docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`（**未完成**项）——
> 独立排期，**不要**与本节的 GBA 阶段表混读。

| 阶段 | 状态 |
|---|---|
| S0 / S0' / S1a-0 / S1a-1 / S1b / S1c | ✅ 已完成（S1c：认领 16 个号码） |
| S2-a 生命周期 + 帧缓冲 + 水印 | ✅（r32） |
| S2-b1 音频 ABI | ✅（r35）—— 分数采样落地 |
| S2-b2 savestate 导出 | ✅（r37）—— 读档走自带 16 MB 栈的工作线程 |
| S2-b3 RTC | ✅（r38）—— 芯片级锁测试，7 个字节全断言 |
| S2-c 真实指令流自建门禁 | ✅（r33） |
| S2-b4 阶段 1 装载 / 输入 / emu 循环分叉 | ✅（r40） |
| S2-b4 阶段 2' 画面 | ✅（r44）—— GBA 自己的帧缓冲 + 纹理 + 3:2 信箱 |
| S2-b4 阶段 3' 音频排空 + 限速 | ✅（r45）—— 限速切到 59.7275 |
| S3-1 电池存档（`.srm`） | ✅（r46）—— 读写通路 + 手动覆盖存档类型 |
| S3-2 L/R 按键绑定 | ✅（r47）—— 读原始 scancode；绑定表仍硬编码 Z/X |
| S3-3 即时存档的 C++ 侧接线与菜单 | ✅（r48）—— 分支落在 `FCEUSS_Save`/`FCEUSS_Load` |
| S3 余项 | ⬜ RTC 的 C++ 接线（ABI 已有、零调用）、`gba_set_save_type` 无 UI |
| S4 手工实测 | 三轮已做（r49/r50/r51）：BIOS 启动序列、三 viewer GBA 分支、铺满与键位三个 P0 均已修。**SMA4 出画面**（语言选择，r51）。**MKSC 白屏已修（r56）**：根因是 SWI 停机唤醒的 IRQ 返回地址差一个流水线宽度、唤醒重执行 SWI 吞掉续接点（r53 ⑧ 第二卡点；SMA4 不受影响的原因——工作在 IRQ 处理器侧——亦已查明）；探针确认解除强制空白（DISPCNT 0x1E40、IE 0x2005、真调色板），**v2.0.1 P0 复跑已到标题画面**（harness 1200 帧，逐帧哈希每档都变 = 真跑非冻帧；ROM 由用户 2026-10-06 补路径，副本在 `Research_only\gbatech\mksc.gba`）。**进赛可玩性仍待手测确认**。清单内其余游戏未测 |
| v2.0.1 P0 | ✅ ARM 模式 SWI 编号修成 bits 16-23（此前读低字节 = 每次 ARM BIOS 调用都变 SoftReset，ARM 模式程序自复位卡死）。**修的同时发现本仓库两处 ARM SWI 编码器（`gate.rs` / `swi/mod.rs`）与缺陷同错**，246 项锁测试在缺陷存在的整段时间里全绿 —— 门禁曾是错误信念的镜像。两处编码器已一并改正，新增 2 条锁测试，两个方向都做过变异验证。详见 v2.0.1 计划 §二 / §十一 r1 |
| v2.0.1 P0–P4 | P0 ✅ ARM SWI 编号（r1）；P2 原范围取消、改向调查关闭（r2–r8）；线 B 指令计费 ✅（r12）；P4 定时器 ✅（r21/r22，AGS TIMER 三项全过）；线 A ✅（r26）；P5 模型侧关闭（r10/r11/r14）；**P3 预取 ✅（r34）**——预取命中判据从 `last_used_address` 换到 `last_opcode_fetch`（数据访问不再打断命中），AGS **24/33 → 25/33**，`PREFETCH BUFFER` 两档读数 24/51 精确。AGS 当前 **25/33** |
| v2.0.1 P1 | 🔧 **计时已落地并验收（r36）**：`TimeDmaToAndFromMemory_U16/U32` 检查 = 源固定读运行中的 TM0、逐半字等差步长校验（步长 = 源访问+目标访问）。缺陷 = DMA 期间定时器冻结 + 缺 +3 起始；修复 = `drain_clocked_peripherals()` 抽取 + `run_dma_block` 块首 +3、每单元前排水。**五个 MEMORY 测试全部转绿（MEMORY 9/9），AGS 25/33 → 30/33**。剩余 `DMA DISPLAY START`（timing=3 触发未实现）与 `DMA PRIORITY`（通道仲裁）是新失败面，各需自己的 RE，记入长尾 |

**导出符号 34 个**（S3-3 零新增）：探针 6 + 生命周期/帧 10 + 音频 5 + savestate 3 + RTC 2 + 输入 2 + 电池 6。
**GBA 锁测试 249 项全绿 + vendor 核心自测 242 项全绿**（**21 条 `#[ignore]` 的测量/诊断工具**：root crate 19 = save.rs 2 + frame.rs 6 + waitprobe.rs 10 + cartprobe.rs 1，vendor 2 = gba-core timers；r37 栈探针、r48 体积探针、S4 的探针与差分转储、v2.0.2 P0 的 M2 帧导出计时探针 `measure_frame_export` 等——完整触发方式见 `docs/tech/gba_probe_howto.md`；r56 新增 1 条唤醒返回地址锁测试，v2.0.1 P0 新增 2 条 ARM SWI 编号锁测试；P2 先行工作项在 `gba-core` 侧新增 2 条 WAIT 期望值测试 + r5 的 2 条半字载入宽度测试 + r12 的 4 条 fail_to_pass 指令周期测试 + r17 的 1 条 AGS 四档分频测试 + v2.0.1 P4 的定时器锁测试 + **r34 的 1 条预取窗口锁测试**——AGS 例程 48 字节内嵌、断言字面量 24/51、变异两向可红）。

> **C3 更正（2026-11-05，v2.0.2 P0/P1 后）**：旧文「7 条 `#[ignore]`」与「248 项」均为过时计数，实测 21 条 / 249 项。另注：decoupling/gate/waitprobe 的守卫与探针**全部 `#[cfg(test)]`**，常驻开销为零——它们不构成运行期强制。

> 🔴 **GBA 卡带兼容性线已在途（2026-11-08，v2.0.3）**：计划见
> `docs/history/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md`。动 GBA 兼容性的代码前**先读它的 §一**——
> 那里有**已确认的三条根因**（未实现 BIOS SWI 落到 stub 的 `b .` 自旋 / vendor 核 panic +
> release `panic="abort"` ⇒ 闪退 / 存档芯片模型不完整）与**被否证的两个假设**
> （「DMA 时序 3 没实现」「Flash 容量映射错」——**都别照着做**）。
> ⚠️ 计划里的位级格式一律取 **GBATEK 权威快照**（计划附录 A）——
> **不凭记忆写位级格式**，本轮已因此翻车两次。
>
> ✅ **S1-a 的 Step 0–5 已完成（2026-11-09，v2.0.3 r4/r5）**：9 条最小可复现夹具
> （**7 条会红** + 2 条绿锁）已落在 `src/rust/src/gba/fixtures/{swi,oam,backup,rom}.rs`
> （公共底座在 `fixtures.rs`），每条都做过**双向变异验证**；另有 3 条常驻守卫
> （G1 孤儿 ignore / G2 夹具集合完整性 / G4 复现器存在性与归属）。
> ⇒ **GBA 锁测试 249 → 262**（+8 编码器自检 + 2 绿锁 + 3 守卫），**26 ignored**
> （19 个测量工具 + **7 条复现器**）。
> ⇒ **复现器用 `--ignored` 跑，且必须逐条为红**；但**必须带夹具域过滤**
> （`cargo test … --lib gba::fixtures:: -- --ignored`）——不带过滤会把 19 个
> 需要真卡带的测量工具一起拉进来。
> ⇒ **夹具断言写「目标行为」**（修复后成立），**不是**缺陷当前行为；且取值必须让
> 差异可观测（区分半字/字存取要用**高半字非 0** 的值或哨兵）。
> ⇒ ⚠️ **两个坑**：① 变异验证的**还原腿会假红**（还原源文件不更新 mtime ⇒ cargo
> 跳过重建 ⇒ 跑的是上一轮变异版的二进制）⇒ 还原处必须刷新 mtime；
> ② **多个 worker 并发改同一个 vendor 文件会互撞** ⇒ 共享 vendor 树的变异验证
> 必须串行或用隔离副本。
>
> ✅ **S1-b 探针改造已完成（2026-11-10，v2.0.3 r6）**：§2.3 六项（verdict 通道化 / SWI
> 陷阱记录 / `gba_step_frame` catch_unwind / 输入注入 `GBA_PROBE_INPUT` + `GBA_PROBE_ONLY`
> / 窗口与「PC 连续 N 帧不变」可配 / `probe_what_one_frame_changes` 忙等开窗）全部落地，
> 只碰 `cartprobe.rs` / `frame.rs` / `swi/mod.rs`；fixtures/ 与 vendor 树零改动。
> ⇒ **GBA 锁测试 262 → 264**（+2 常驻锁：`special_timing_wording_is_channel_aware`、
> `a_core_panic_in_step_frame_is_caught_and_readable`——后者断言完整恢复周期：
> panic→STATE / 读数 / 下一帧 NO_ROM / 毒锁下重载成功且新机器跑通），**26 ignored 不变**；
> 复现器仍 7 红；vendor 242 / debug 93 / media 52 / `ctest -LE perf` 36/36 全绿。
> 7 张卡基线（240 帧窗口）**零 panic、零未实现 SWI**（故障点在更深处）；Dragon Ball
> 注入证明游戏对输入有响应、「按键后卡死」未复现（交 S3-b/S3-c 用同一工具追）。
> ⇒ ⚠️ **三个工具链/并发事实（细节见计划 §九 r6）**：① **`cargo test --release` 仍是
> unwind**（`panic="abort"` 不作用于测试目标）——fixtures/oam.rs 注释的相反说法被否证；
> ② **格式化 panic 的 payload 不可 downcast**（rustc 1.96，`is::<&str>()`/`is::<String>()`
> 双 false）⇒ panic 读数必须走 hook 的 `PanicHookInfo::payload_as_str()`（hook + thread-local
> 是 fixtures/oam.rs 已验证的模式）；
> ③ **match scrutinee 自死锁**——`match m.lock() { Err(_) => m.lock() }` 的 scrutinee
> （内含 guard）活到 match 结束 ⇒ Err 臂内对同一互斥量再 lock = 自己等自己；毒锁恢复一律
> **先 bind 再 `into_inner`**，不许在 match 臂内重入同一锁（初版 with_machine 正中此坑，
> 全套件 12+ 测试排队挂起；也解释了旧 `try_lock` 恢复为何是死代码）。
> ⇒ ⚠️ **bat 编码坑（补录）**：`.bat` 文件里写中文路径会被 cmd 按 OEM 码页读成乱码
> ⇒ 含非 ASCII 的环境变量（探针目录 / 过滤 / 注入）一律**从 bash 侧传入**，bat 内不放
> `set` 行。
>
> ✅ **S2-a 已落地（2026-11-10，v2.0.3 r7）**：vendor 的两个可达 panic 点
> （`object_attributes.rs` 的 OAM attr0 **shape=3 / gfx mode=3**）改为映射 **`ObjMode::Disabled`**
> （真机语义：精灵不显示）——§1.2.2 的「闪退」根因消除；§3.1 表其余条目按 r2 判定
> **不可达、不动**（业主裁决按 r2 收窄）。
> ⇒ vendor 补丁 **27 → 28 处、5 → 6 个源文件**（ATTRIBUTION §3.2.12，#30）。
> ⇒ **复现器 7 → 5**：`oam_bad_shape` / `oam_gfx_mode3` 摘 `#[ignore]` 转常驻绿锁
> （`assert_treated_as_disabled` 双对照设计直接成立），守卫 G4 集合同步收窄；
> **根 crate 264/26 → 266/24**，vendor 242 / debug 93 / media 52 / `ctest -LE perf` 36/36 全绿，
> 7 卡基线复跑零回归（verdict 与 r6 逐字一致）。
> ⇒ **S1-b 的 Lock B 改造为注入式机制锁**（业主裁决）：OAM 触发器随修复退役，改名
> `a_frame_panic_is_caught_and_the_machine_recovers`——`#[cfg(test)]` 的
> `INJECT_FRAME_PANIC` 在 `gba_step_frame` 的 machine body 内注入 panic（与 vendor panic
> 同一条 unwind 路径，release 编译期消失）；断言链不变（STATE / 读数 / NO_ROM /
> 毒锁下重载成功）。变异两腿双向可红（shape 映射改回 panic ⇒ oam_bad_shape 红；
> gfx 映射「只回 Normal 不置 Disabled」⇒ oam_gfx_mode3 红）。
>
> ✅ **S2-b 已落地（2026-11-11，v2.0.3 r12）**：stub BIOS 的 SWI 向量从 `b .` 自旋改为
> **`MOVS PC, LR`**（`0xE1B0F00E`）——未实现 SWI 从「机器冻结在 BIOS 0x08」变「返回 +
> 记录」，ARM 与 Thumb 双路夹具实证（Thumb 返回经 SPSR 恢复 THUMB 态）。
> ⚠️ **计划 §3.2「寄存器默认值按 mGBA」口径已修正**：mGBA 源码不在本机
> （`Research_only/` 已不存在），改按 **GBATEK SWI ABI 自建依据**（`:192058-192062`：
> R2,R4-R14 保持不变 ⇒ 全寄存器保留、零编造返回值）；GBATEK 行号链独立确认
> `MOVS PC, LR` 处方（ARM LR=PC+4 `:185686`、Thumb LR=PC+2 `:189222`）。
> ⇒ **两条把缺陷锁成预期行为的测试已按 §1.2.1 预告改写**：
> `abort_vectors_park_and_the_swi_vector_returns`（abort 向量仍 park——那是真异常）与
> `an_unclaimed_swi_returns_and_is_recorded`；**生产侧记录器** `note_unimplemented_swi`
> （每号首次 `eprintln!` + 常驻计数，vendor 自答 6 号不计入；跨 ABI 暴露为延后项）。
> ⇒ **复现器 5 → 3**（`swi_unimpl_1a`/`swi_unimpl_16` 转常驻绿锁），夹具 9 → 10
> （新增 Thumb 绿锁）；**根 crate 266/24 → 270/22**，vendor 242 / debug 93 / media 52 /
> formats 153 / `ctest -LE perf` 36/36 全绿；7 卡基线逐字一致；变异 M1–M4 逐腿双向可红。
> ⚠️ **新坑**：ARM `BX` 基字 bits 7-4 必须是 `0001`（`0xE12FFF10|rm`），写成
> `0xE12FFF00` 会被 vendor 解码劫走当 MSR——「注释写对位段、常量写错」要靠 disasm 实证抓。
>
> ✅ **S2-c 已落地（2026-11-11，v2.0.3 r13）**：`gba_load.cpp` 每帧步进的错误码从
> 「静默吞掉」改为**去重日志**（同码 streak 只报首帧、码变再报、干净帧/新会话清闩；
> 走 `FCEU_printf`＝stdout + 消息日志 dock，**刻意避开 `FCEU_PrintError`**——它下游是
> 弹窗）。不弹窗/不崩/不卸载，语义对齐 r6 的 catch_unwind 恢复机制；release
> （panic="abort"）下 STATE 不可达，日志读者是 debug/探针构建。**红线 2 的容错读数自此
> 在 C++ 侧也有落点。**
>
> ✅ **S3-a 第一批已落地（2026-11-11，v2.0.3 r15）**：BIOS SWI **`0x16`/`0x17`/`0x18`**
> （diff unfilter）**有真实现了**——新 bus-free 模块 `src/rust/src/gba/swi/unfilter.rs`
> + `swi/mod.rs` 的 `serve_diff_unfilter`。
> ⇒ `0x16` = 8 位单元/逐字节写；`0x17` = 8 位单元/逐半字写；`0x18` = 16 位单元/逐半字写。
> ⇒ 依据 = **GBATEK 快照**（`:192251-192253` 号段、`:192804-192867` 契约、
> **`:192826-192827` 它自己的示例向量**）**+ mGBA 的 `_unFilter`**（`src/gba/bios.c:903-946`，
> 经代理取回，**只读格式不抄代码**——唯一说清「哪号读 8 位单元、哪号存半字」的来源）。
> ⇒ 夹具 `swi_unimpl_16` **改写**为 `swi_diff8bit_unfilter_wram`（`0x16` 有了实现，
> 「未实现 SWI」的锁名与前提都不再成立），新增 `swi_diff8bit_unfilter_vram`（`0x17`）与
> `swi_diff16bit_unfilter`（`0x18`）；期望值取 **GBATEK 公布的 10 11 … 19** 那一行，
> **不是**拿实现的同一套算法重算（测试与实现同错是本项目栽过两次的坑）。
> ⇒ **GBA 锁测试 270 → 280**（+8 单元锁 +2 端到端夹具），夹具集合 **10 → 12**
> （红复现器仍 3 条：S3-b ×2 + S3-d ×1，本轮未修）。
> ⚠️ **两条如实登记**：① **size 字段按字节**在 GBATEK 里**没有明说**（它只写
> 「24bit size after decompression」）⇒ 依据是「与 LZ77 同款头部 ⇒ 同款口径」+ mGBA 的
> `remaining = header >> 8`；② `0x16` 的落点是 WRAM ⇒ **它的写宽度不可观测**
> （字节写与半字写在 WRAM 同结果），宽度由实现与 mGBA 参照背书 —— `0x17`/`0x18` 的落点
> 改用 **VRAM** 正是为了让它可观测（vendor 对 VRAM 字节写会复制/丢弃，bus.rs:1235-1262）。

> ✅ **S3 已收口（2026-11-12，r16 只读 RE + r17 实施批）——四条计划前提被权威来源证伪，实际落地比计划小而准**：
> ① **ROM 窗口「要镜像」证伪**（GBATEK `:14046` "Normal ROM Cartridges ... are NOT mirrored"，mGBA/melonDS 同判）
> ⇒ vendor 现状正确，`rom_window_mirror` 改写为**开放总线绿锁**（字节/半字/字四腿，期望值独立拼装）；
> ② **mode 4 index 0「缺陷」证伪**（GBATEK `:3198`：bitmap 模式 Color 0 = backdrop 透明，vendor 正确）⇒
> 改加 **OBJ-behind-BG2 防回归锁**（直接断言 index0==palette[0] 无牙：backdrop 色就是 bg_palette_ram[0]）；BG2 门控同为误报；
> ③ **flash 状态寄存器证伪**（GBATEK 命令集无 0x70、pret 反编译的 SDK 驱动 PollFlashStatus = 阵列字节读 + DQ5 超时位、
> mGBA/VBA-M 同）⇒ `0x0E000006/7` 真语义 = 回读，`flash_status_reg` 断言反转为绿锁；E2 EEPROM 走 **D1 动态容量**
> （ID 串 nnn = 库版本不携带容量 `:8521`；512 B 起步、高位访问懒扩 8 KB 封顶；旧 32 KB `.srm` 前 8 KB 无损迁入，>8KB 尾段 = 旧掩码下的 0xFF，一次性丢弃）；
> ④ **FireRed「GPIO 先读后写」线索证伪**——真根因 = **游戏自身 init 时序竞争**（影子寄存器 apply 链、DISPSTAT bit3
> 300 帧未开闸；加速时钟对照跑：apply 执行 ⇒ 游戏活；忙等地址是 `0x0300310C` 不是旧读数的 `0x030030FC`），
> 属精度范畴 **转长尾**。**✅ r19 追记：r17 的 `0x01` RegisterRamReset 强制空白（GBATEK `:193540-193541`）
> 被双向变异定位实证为让 FireRed 越过该竞争的确定性触发点**——7 卡基线里 6 张逐字不变、唯独 FireRed 从
> FROZEN 变 48 变化帧 + 音频 DMA 激活；摘掉 DISPCNT 写入立即回 FROZEN、还原即复现 ⇒ 机器判据「出画面」达成；
> **✅ r20（2026-11-12）：业主人工实测确认 FireRed 已兼容**（发布构建 `main` @ `45fec33`，
> 实测目录 `C:\Users\ikrx2\Desktop\v2.0.3实测依据`）——是**越过**该竞争，不是修好它，
> 竞态长尾条目保留。**真落地**：S3-a 第二批 4 号（`0x19`/`0x1F`/`0x25`/`0x27`，全 GBATEK 有据）+
> `0x01` RegisterRamReset 补 DISPCNT 强制空白；**mode 5 页面翻转修复**（vendor `layer_2.rs`）；**`apply_state` 潜伏缺陷**
> （读档后 ROM/BIOS/backup_type 三者全空 ⇒ 真实游戏读档致命；vendor 的 `redetect_backup_type` 全仓零调用）
> 修为三条语句 + 锁（**新已知限制**：手动 `gba_set_save_type` 覆盖随读档被 redetect 还原为检测值——vendor doc 既定语义）；
> `try_read_word` 越界臂快路径契约修复（总线级值前后逐位相同）。
> **声音家族 12 号（`0x1A`–`0x1E`/`0x20`–`0x24`/`0x28`/`0x29`）与 `0x2A` 维持 S2-b 兜底 + 已知限制**——
> mGBA HLE 也全是 no-op stub（`bios.c:607-608`），认领成 no-op 会违背红线 3 的「未实现必须可见」；
> `0x2A` 的 36 项函数表 GBATEK 零记载 ⇒ BLOCKED；`0x26` HardReset 结构障碍维持 ⇒ 挂起。
> ⇒ **门禁终读数（2026-11-12）**：GBA 根 crate **299 passed / 0 failed / 20 ignored**（提交口径 ignored = 19 测量工具，
> **复现器清零、`REPRODUCERS = []`**）；vendor **249**（242 + 5 单元锁）；debug 93 / media 52 / formats 153 全绿；
> 两态 `cargo check` EXIT=0；`ctest -LE perf` **36/36**。夹具集合 **12 → 14 条**（新 `fixtures/lcd.rs` 注册进 G2，
> 守卫扫描模块 4 → 5）。⚠️ **实施期新坑两条**：① **探针观测不得用 `bus.read_half_word`**——它加 `cycles_count`
> （`bus.rs:673/1302`），轮询探针给机器**加速时钟并改变行为**（§1.2.3 的旧读数即出自此类探针）⇒ 探针观测一律
> `read_raw`；② **bus 字写会把未对齐地址向下对齐再写 4 字节** ⇒ 夹具逐字节 word 写每 4 字节只落 1 个
> （曾致 mode4 夹具假红），造数据要么按对齐字拼、要么用字节/半字写。
>
> ✅ **S4 · FC 侧修复批次已落地（2026-11-10，v2.0.3 r9）**：GitHub issue #12 的两个 NES 侧
> 缺陷（iNES loader 硬拒非 2 幂 PRG / mapper 191、199 未注册灰屏）**均已修复**——
> ① loader 的 PRG 切片 / 文件边界检查 / hash 改用 **raw** 字节数（**分配与映射仍用舍入后的
> 2 幂尺寸**，`SetupCartPRGMapping` 的要求）；② 191/199 进 `MapperEntry` 注册表。
> ⚠️ **199 不能套一行式基类**：v1.15 B.1 把 MMC3 变体的 power 统一成基类的
> `GenMMC3Power()`，而 199 有自己的 `M199Power`（DBZ 扩展寄存器 + `0x8001` 写处理器）
> ⇒ `Mapper199Cart` 覆写 `on_power()`。
> ⚠️ **同因的既有问题未修（单独立项）**：已注册的 **17 个带自定义 power 的 MMC3 变体**
> （12/37/44/45/47/49/52/114/115/165/195/205/245/249/250/254/406）**同样只跑
> `GenMMC3Power()`**，它们自己的 `MxxPower` 从不执行 ⇒ 与 199 同因，本轮未动。
> 文件域（`fceux11-formats` / `ines_load.cpp` / `boards/` / `tests/f11qa`）与 GBA 线零交集；
> NES 零回归门禁对 S4 **加倍适用**。⚠️ **`fceux11-formats` 此前不在任何门禁里**
> （纪律 10 活例）——`cargo test -p fceux11-formats --lib`
> **151 项**已实测并登记进门禁节，动该 crate 必跑。
> ⚠️ **S4 未做的两项**：B4 的人工步骤**已做**（r10：业主用 9 张 NES 卡带实测 v2.0.2_hotfix1
> + 上游 FCEUX 2.6.6 / 2.7.0 同参数对照）；F11QA 矩阵 `pass_to_fail=0` / grade ≥ B
> **已于 2026-11-12 由 CI 复核通过**（`main` 发布提交 `45fec33` 上 CI #320 / F11QA #95 全绿）。

> ⚠️ **不变式 1 的判据是 F11QA 矩阵 `108P/12F`**，本机跑的是它的**本地近似**
> `ctest`。**本地确定性门禁 = `ctest -LE perf`（37 项，须全绿；计数随新增测试增长，
> 以 `ctest -N -LE perf` 的 `Total Tests` 为准）**；
> `bench_tolerance_test` 是 **advisory**（r55：full_frame 在本机随时钟上下文
> 漂移 +23% —— 同一构建热采 132.9 / 套件内 ~145 / 套件后 ~163 ms，2.5/5/10%
> 容差都实测假红过；基线随全量重链重采，读数只用来追大幅持续回退，历史真回归
> 量级是 +100%）。跑 ctest 时仍不要与构建/测试任务并行（并行会污染测量，r54 ②）。
>
> ⚠️ **GBA 的「画面与声音是否真的对」无法穷尽验证。** 现有资产是 MKSC 与 SMA4
> 两张商业卡带（S4 已用于实测）；差分参照 mGBA 0.10.5 在 `Research_only/`
> （gitignored，只做研究、不进构建）。已验到的是「接线自洽 + 守卫仍成立 +
> NES 未回归」。**`.srm` 跨模拟器字节级互通**同理未验 —— GA 门禁第 4 条已由
> r54（**用户裁决 2026-10-03：接受 GBA 模块不完美**）解除，验证转 S4 长尾；
> **GBA 长期 Alpha，不阻塞本体发版。**

v2.0 对 vendor 树的本地修改是 **33 处、跨 7 个源文件**（文件清单用
`git log --name-only --diff-filter=M -- src/rust/crates/gba-core/src/` 实测得出，
不是手数；`Cargo.toml` 不算源改动；r56 增 1 处，
v2.0.1 P0 增 1 处、P2 先行工作项增 1 处、r5 增 2 处、r12 计费修复增 1 处、
r17 定时器分频增 1 处、r21/r22 定时器两项增 2 处、r34 预取命中判据增 1 处、r36 DMA 逐单元排水与起始延迟增 1 处、
v2.0.3 r17 增 5 处（internal_memory.rs 4 + layer_2.rs 1）；
被回退的 #24（r17）与 #27（r29 DMA 计数）保留编号但不计数）：

| 文件 | 处数 | 内容 |
|---|---|---|
| `src/cpu/arm7tdmi.rs` | 13 | S0' 接线 5 处 + S1a-1 的 halt 机制 4 处 + r56 的唤醒 IRQ 返回地址 1 处（**改变行为**，见 ATTRIBUTION §3.2.2）+ v2.0.1 P0 的 ARM SWI 编号位 1 处（**改变行为**，见 ATTRIBUTION §3.2.3）+ r5 的 LDRH/LDRSH 半字载入宽度 2 条（**仅测试**，见 §3.2.5） |
| `src/bus.rs` | 4 | v2.0.1 P2 先行工作项：**只有测试**（AGS 老化卡的 WAIT 类期望值，28 项），未动任何生产代码 —— 见 ATTRIBUTION §3.2.4；r12 的**取指/执行体 S 周期双计费修复**（**改变每一条指令的代价**，见 §3.2.6）；r34 的**预取命中判据改用 `last_opcode_fetch`**（**改变预取开启时 ROM 取指的计价**，AGS PREFETCH 两档 24/51 精确，见 §3.2.10）；r36 的 **DMA 块逐单元排水定时器 + 3 周期起始延迟**（**改变 DMA 代价并让定时器在块内走动**，AGS MEMORY 五项转绿，见 §3.2.11） |
| `src/cpu/hardware/timers.rs` | 3 | r17 的**定时器分频取错寄存器**（读 `TMxCNT_H` ⇒ 恒为 ÷1）修复 + r21/r22 的 TM0 cascade 位与使能启动延迟（**改变行为**）—— 见 ATTRIBUTION §3.2.7/§3.2.8 |
| `src/cpu/hardware/rtc.rs` | 3 | S2-b3：固定时刻覆盖（**不改变上游行为**） |
| `src/cpu/hardware/internal_memory.rs` | 8 | S2-b3 的 `rtc()` / `rtc_mut()` 2 处 + S3-1 的 `backup_type()` / `set_backup_type()` 2 处 + v2.0.3 r17 的 **EEPROM D1 动态容量 3 处**（`buffer_size` 0x0200 / `ensure_eeprom_capacity` / `load_battery` 旧档迁移，**改变行为**：`.srm` 从恒 32 KB 变 512 B 起步懒扩 8 KB）+ **`try_read_word` 越界臂半字对快路径** 1 处（契约/性能，总线值不变）——见 ATTRIBUTION §3.2.13–§3.2.16 |
| `src/cpu/hardware/lcd/object_attributes.rs` | 1 | v2.0.3 S2-a（r7）：OAM 禁止值（attr0 shape=3 / gfx mode=3）映射为 `ObjMode::Disabled`（**改变行为**：原每扫描线 panic ⇒ 闪退根因；现为真机语义「精灵不显示」）——见 ATTRIBUTION §3.2.12；⚠️ 重新 vendor 时新版上游大概率仍是 panic，**合并会静默把闪退带回来** |
| `src/cpu/hardware/lcd/layers/layer_2.rs` | 1 | v2.0.3 S3-e（r17）：**mode 5 页面翻转**（`render_mode5` 补 `frame_offset = bit4 ? 0xA000 : 0`，与 mode 4 同式；**改变行为**：此前 mode 5 恒读第 0 页）——见 ATTRIBUTION §3.2.17；⚠️ GBATEK `:3198` 证实 mode 4 的 index 0 透明是**正确**行为，别当缺陷改（有防回归锁） |

**后三个文件里的改动都不改变「不覆盖时的行为」。** 但**补丁集已不再是「集中在一个
文件」**（r38 ④ / r46 ①）—— 改动前先确认这一点是否仍成立，`rtc.rs`、
`internal_memory.rs`、`bus.rs` 都要查。重新 vendor 时看 `ATTRIBUTION.md` §3.2 与
§3.2.1–§3.2.7、§3.5。

> ⚠️ **§3.2.3 那一处与上面 17 处性质不同：上游是错的，不是不完整。** 重新 vendor
> 时新版上游很可能仍读低字节，**合并会把修复悄悄覆盖回去且无人报警**，所以它是
> 重新 vendor 要**第一个**核对的项。另外它也是唯一一处「删掉它本仓库测试也发现不了」
> 的补丁 —— `gate.rs` 与 `swi/mod.rs` 的 ARM SWI 编码器当时与缺陷同错，两处已随本
> 次修复改正。

## 长尾（r57 收口后按需驱动——没有待办的主动清单）

v2.0 GBA 计划建设期已收口归档（r57 用户裁决）。剩余长尾全部**按需驱动**，
每项带触发条件；启动时以独立小任务立项，结果记 CHANGELOG 与本文件：

1. **RTC C++ 接线**（挂起）：`gba_rtc_set_time` / `gba_rtc_time` 两个导出在
   `frame.rs`，C++ 侧**零调用**。默认宿主时钟已让 RTC 游戏正常跑；接线只服务
   「钉时刻/调试」。触发条件：出现需要调时间的真实用例。接线前先想清楚凭什么
   判断卡带带 RTC（r38 刻意未导出该信息，防给非 RTC 卡带开假开关）。
2. **存档类型手动覆盖 UI**（挂起）：`gba_set_save_type` 有导出、无 UI。「类型
   不明时拒绝写入」核心侧已强制；UI 侧属产品决定。触发条件：某游戏存档类型
   误判。
3. **S4 兼容性**（报告驱动）：MKSC（r56 修复，用户已确认可运行）与 SMA4 已验
   主路径；其余游戏跑不动再立项排查。**先例教训**：r53、r56 两轮系统性缺陷
   均由真卡带发现、锁测试全绿——新的兼容性报告按 r53/r56 的探针+差分方法处理。
   **同族第三例已发生**：v2.0.1 P0 的 ARM SWI 编号缺陷同样在 246 项锁测试全绿的
   状态下存在（测试自己的编码器与缺陷同错），靠外部参照才暴露。**推论：新增
   门禁时先问「这条断言的通过原因，与它要验的那条路径是不是同一条」**——
   r33 记过一次、v2.0.1 P0 又记一次，两次都是「测试与实现同错，闭环自洽」。
4. **DMA 两项时序 RE**（报告驱动，v2.0.1 归档后移交）：`DMA DISPLAY START`
   （timing=3 触发未实现）与 `DMA PRIORITY`（通道仲裁）各需自己的 RE；
   方法与常数出处见归档计划 `docs/history/plans/FCEUX11-v2.0.1_GBAEUX11精度优化计划.md`。
   另有唯一取证待办：`start_delay = 2` 目前仍是以 AGS 为准绳的标定值，需要
   独立于 AGS 的来源（GBATEK TMxCNT_H 写入时序说明或第二套参照实现）。
5. **§9.1 八条开放限制**（L1/L2/L4/L6/L7/L10/L13/L15）：r42/r54 裁决 Alpha
   阶段长期开放，无需工作；全文见归档计划 §9.1。
6. **FireRed 的 init 时序竞争**（报告驱动，v2.0.3 r16 移交）：忙等轮询
   IWRAM `0x0300310C`（gMain+0x1C）的 bit0，写入方 = VBlank IRQ 回调
   （ROM `0x080007B4`）——但游戏自身的 IRQ 管理器把意图写进影子
   （`0x03000004=0x0008`）后「影子→硬件」apply 从未运行（DISPSTAT bit3 未开闸）；
   加速时钟 +25% 对照跑里 apply 执行、游戏活 ⇒ init 时序竞争，属 **GBA 精度范畴**
   （本轮范围外）。入口线索：`0x030000C0` 锁 + `0x03000060` 队列 + flusher
   `0x08000A18` 的调用方。取证读数：`docs/history/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md`
   §1.2.3 的 r16 收口注。**✅ r19：已知一个确定性「解楔」**——r17 落地的 `0x01`
   RegisterRamReset 强制空白（GBATEK `:193540-193541`）使 FireRed 在标准 240 帧
   探针窗口越过竞争（双向变异定位实证）；竞态本身未修，若未来要治本按入口线索立项。
7. **ROM 开放总线的 mGBA 两例外**（报告驱动，r17 未跟）：mGBA 0.10.5 对
   **1 MiB 卡的 4 倍镜像**与**非 2 幂 ROM（烧录卡）的补零**有额外行为，本仓一律
   开放总线公式；现有夹具对 1 MiB 类卡不设防。触发条件：出现实际受影响的卡带
   或差分证据。

## v2.0.2 线（NES 侧）的收口状态（2026-11-08，r26）

> 发版主线文档已重组为两份：**已完成**项的事实源是
> `docs/history/plans/FCEUX11-v2.0.2_DEBUG与性能优化-已完成项归档.md`（收口于 **r26**，
> 含当时实跑的 ctest / cargo / 矩阵读数），**未完成**项是
> `docs/history/plans/FCEUX11-v2.0.2_残留精度与调试待办-review报告.md`。
> 本节只记**下一任接手的人会踩到的**东西。

1. **A6/D5-S3-S5 已关闭（业主裁决 r25）** —— bus 直读快路径**不再立项**。理由三条：
   探针三连稳定但收益 3–8% 骑线；r12 排它后面的「D1 收口后再动」前提在方法论上
   不成立（收益是性能问题，静默探针测不出）；两个更便宜替代（**PGO** 开关现成 /
   **A7**）从未动。S0+S2 的 85 条护甲断言**转常驻**、零维持成本。
   ⇒ **A6 已关闭，不要照着旧计划立项**；替代槽位是 PGO（从未评估）或 A7（未动工），
   两者的状态见 review 报告 §1.3 / §1.11。
2. **F-2 已修（r26，`ce89bc7`）** —— 无头回落反汇编器
   （`src/rust/crates/fceux11-debug/src/asm.rs`）的助记符表原按列书写，**整个 zp
   列解码为 ORA**、0x8D/0x8E/0x8C/0x91/0x99/0x9D 落 `???`。现为按 opcode 升序的
   官方 6502 全集（151）+ 6 条单字节 NOP，锁测试 `lock_disasm_mnemonics_256`
   （256 项穷举、**扁平独立参照表**）。只污染 `FCEUX11_TRACE` 无头 trace 文本，
   Qt 调试器走独立实现不受影响。
   **⚠️ 它的端到端验证脚本在 `.zcode/d1s2/a1_checklist/verify_trace_f2.py`** ——
   按 ROM 字节推导期望值逐行比对官方矩阵，比单元锁多走一段真实链路
   （`trace_common.cpp:33` → `asm.cpp:24` → Rust → 文本）。
3. **两个 crate 的 Rust 锁测试曾不在任何门禁里** —— `cargo test -p fceux11-rust`
   **不跑依赖 crate 的测试**（`fceux11-debug` 93 项 / `fceux11-media` 52 项），
   且 **CI 里没有任何 cargo 环节**。⇒ F-2 的锁与 A5 G3 的绝对金标准向量锁
   （`lock_resample_interpolation_golden_vector`）**写了却一次都没跑过**。
   本轮已把两条命令补进「门禁」节。**新增锁测试时必须确认它落在哪条命令的路径上。**
4. **业主四项裁决（2026-11-08，收口前）**：D1 round-2 两项章程（(C) DMC 计费界定 /
   VBL 锚 K 扫参）**转长尾**；F-1（Lua `emu.frameadvance()` abort）**转长尾**；
   §八 出口判据的 107 条**改写**为「协议缺陷已修、端到端按 defer 记已知限制」；
   **不打 tag**（版本口径与 CHANGELOG 对齐即可，push/tag 另需明示）。
   ⇒ **int_4 / 037 / 050 / 107 在 `tests/tests.json` 里全是 `failure_means: advisory`
   且基线为 `false`** ⇒ 修它们**不改 grade、不改 pass_to_fail**，判红体系上零收益。
   这是「收益不大就不做」这条裁决的**硬依据**，下次评估精度项前先查这一列。

## v2.0.2_hotfix1 发版（2026-11-10）

> **v2.0.3 线尚未收口，但本体先发一个 hotfix**：业主裁决「S4 收尾 + GBA 侧在途工作
> 不影响 GBA 正常功能 ⇒ 可正常发布」。发版内容 = **v2.0.2 全部成果** +
> **S4 · FC 侧修复批次**（GitHub issue #12）+ GBA 侧 S1-a/S1-b/S2-a 的在途成果。

1. **版本口径**：`src/version.h` 的 `FCEU_HOTFIX_TAG` = `"_hotfix1"`，
   `FCEU_DISPLAY_VERSION` = `v2.0.2_hotfix1`。⚠️ **数值口径不动**
   （`FCEU_VERSION_NUMERIC` 仍 `20202`、`FCEU_VERSION_TWEAK` 不参与计算）——
   那个数进 FCSX 电影模式存档头（`state.cpp:397`）与录像头（`movie_record.cpp:591`），
   改了会让 v2.0.2 的存档/录像与本版**互读失败**。
   ⚠️ `CMakeLists.txt` 的 `project(FCEUX11 VERSION 2.0.2)` **故意不动** ——
   CMake 的 `VERSION` 只吃数字段，装不下 `_hotfix1` 后缀。
2. **本次发版对 GBA 侧的影响面**（v2.0.3 未完成部分是否影响正常功能）：
   ① S1-a 的夹具与守卫全在 `#[cfg(test)]` ⇒ **发布版零开销**；② S1-b 的探针改造
   （SWI 陷阱记录 / panic 捕获 / 输入注入）里**记数与注入器是 `#[cfg(test)]`**，
   生产路径只多了一层 `catch_unwind` 包装 —— release 是 `panic = "abort"`，
   **catch 不可能触发，行为与改前逐字相同**；③ S2-a 是**行为变更**（OAM 禁止值
   ⇒ 精灵禁用），它修的正是闪退根因 ⇒ 严格改善。
   ⇒ **结论：GBA 侧在途工作不改变 GBA 正常功能。**
3. **发版时随包登记的已知限制**（CHANGELOG `[2.0.3]` 段的 Known limitations 同款）：
   GBA 仍是 Alpha（SWI 声音家族 12 号 + `0x2A` 维持兜底、EEPROM 型号不建模但已走动态容量、
   GPIO/RTC 未做）· **4 张失败卡带仅 FireRed 经人工复核确认兼容，其余 3 张仍未确认**
   （机器判据全过 ≠ 兼容）· **17 个带自定义 power 的 MMC3 变体与 199 同因但未修**
   （只修了 198 与 199）· F11QA 矩阵**已于 2026-11-12 由 CI 复核通过**。
4. ⚠️ **发版不等于打 tag**：本地 `add`+`commit` 照做，**push / tag / 建分支一律等
   业主明确指示**。

> **🔴 2026-11-10 追加（S4 的 B4 人工步骤 + 上游对照实验的结果）**：业主用 9 张 NES 卡带
> 实测 v2.0.2_hotfix1，并与**上游 FCEUX 2.6.6 / 2.7.0** 做了同参数对照（抓模拟器窗口取
> 画面区像素统计）。结论：**S4 的两个修复都真实生效**（registry 让 3 张 191/199 卡能出画面；
> loader 让非 2 幂 PRG 卡从「被拒」变成「能装载」），**9 张里 8 张与上游行为一致**；
> **唯一不同的一张（吞食天地2）暴露了两个本仓真实缺陷**：
> ① **hash 作用域**——本仓按 **raw PRG** 算 CRC32/MD5，上游与 `src/ines-correct.h` 修正表
> 都按**舍入 + 0xFF 填充**后的缓冲区算 ⇒ 非 2 幂 PRG 的 ROM 必然查不中修正表
> （吞食天地2 的 raw CRC `0x50D82653` 查不到、padded CRC `0x28192599` 命中
> `{0x28192599, 198, -1}` —— **它实际是 mapper 198，不是头里的 4**）；
> ② **修正写回缺失**——`src/ines_load.cpp` 的 `ines_apply_corrections()` 把修正后的 mapper
> 写进**局部** `parseResult.mapper_no`，`tofix & 1` 分支是**空注释** ⇒ **整张 251 条修正表的
> mapper 修正在本仓整体失效**（镜像/电池/清 VROM 三类修正是写回了的）。
> ⇒ **S5 已落地**（计划 §四之三）：A1 hash 作用域改回 padded（**推翻 S4 的 A3 决策**——它漏了
> hash 同时是修正表的查找键）、A2 mapper 修正写回、A3 mapper 198 跑自己的 `M195Power`
> （否则 $5000 没有工作 RAM）、A4/A5 四条锁（formats **151 → 153**）。
> ⇒ **验证**：吞食天地2 **跑出与上游一致的游戏标题画面**；9 张里 **8 张与上游逐字一致**，
> 剩下 4 张（Q Boy / SD高达外传2 / SD高达外传3 两版）**双方都灰屏** ⇒ 上游级缺口，非本仓回归。
> 门禁：`ctest -LE perf` **36/36**；GBA 根 266/24、vendor 242、debug 93、media 52、formats 153 全绿。
> ⚠️ **口径更正**：本仓 fork 的上游基线是 **FCEUX 2.6.6**（`1c35642:src/version.h`），
> **「1.15」是本项目自己的版本线**——别再混。
> ⚠️ **本仓 exe 不吃命令行 ROM**（Qt 前端从不调 `Config::_parseArgs`）⇒ 自动化测试必须走
> GUI（Alt+F→O→**剪贴板粘贴**路径→Enter；中文路径无法用 SendKeys 直输）。

## S3-3 实施时踩到的三件事（给下一个接手的人）

1. **`FCEUSS_Save` / `FCEUSS_Load` 是所有即时存档入口的唯一收敛点**：10 个槽位、
   Save/Load State As、F8/F9（`ConsoleFile.cpp:389,399`）、热键
   （`src/input.cpp:1065,1082`）全部汇到这两个函数。分支插在「文件已开、序列化器
   未到」之间，文件名生成、备份拷贝、undo 记账、槽位标记**全部复用**，菜单因此
   免费获得。
2. **菜单门控此前是灰的，原因是结构性的**：`FCEU_IsValidUI` 判 `!GameInfo`，而
   `.gba` 装载时 `ResetGameLoaded()` → `FCEU_CloseGame()` 已把 `GameInfo` 置空
   （`fceu.cpp:201-202`），`GbaLoad` 从不构造它。必须拆组放行 savestate 六项，
   **录像三项与 `VIEWSLOTS` 保持 `GameInfo` 判据**（不变式 9 红线：录像须显式
   声明不可用）。
3. **一个我查到底后自己证伪的推断**：我以为 `FileBase` 会残留上一局 NES 的值，
   导致 GBA 存档覆盖别人的存档位。**不成立** —— `GetFileBase()` 在
   `fceu.cpp:459` 对每个文件（含 `.gba`）都调用。记在这里是因为它看起来非常可信，
   而「看起来可信」正是这类缺陷的特征。

## 代码地图

```
src/rust/src/gba/            一手 GBA 逻辑（根 crate 的模块，不是独立 crate）
  mod.rs                     接线：install_swi_hook
  bios.rs                    stub BIOS，16 KB 镜像由 const fn 逐字写入
  ffi.rs                     C ABI 探针面（6 个符号）+ 两个漂移守卫
  frame.rs                   生命周期 / 帧缓冲 / 水印 / 输入 / savestate / RTC / 电池存档的导出面
  audio.rs                   分数采样时钟 + 转换链 + 5 个符号
  overlay.rs                 BETA 水印
  gate.rs                    真实 ARM 指令流的自建门禁
  save.rs                    savestate 编解码（serde_json + 指纹 + 侧车 + 工作线程）
  rtc.rs                     RTC 固定时刻覆盖 + 芯片级端到端测试
  decoupling.rs              四条解耦守卫 + 两条自检（含 S3-3 的 state.cpp 位置守卫）
  swi/…                      dispatch 与各 SWI 的实现
src/gba_load.{h,cpp}         C++ 侧唯一的 GBA C ABI 调用点：装载器 + 会话状态 + 每帧步进 + 帧缓冲 + `.srm` / savestate 文件
src/state.cpp                即时存档：仅 `FCEUSS_Save` / `FCEUSS_Load` 两处分支（唯一收敛点）
src/drivers/Qt/ConsoleViewerSDL.{h,cpp}   GBA 的呈现分支（NES 那一侧逐字不动）
src/drivers/Qt/fceuWrapper.cpp            每帧分支 + 限速/音量/音频排空
src/rust/crates/gba-core/    vendor 的 GBA 硬件核心（clementine, MIT）
src/rust/build.rs            cbindgen → fceux11_rust.h（GBA 的 C ABI 是**手写**的）
```

> 根 crate 的 `Cargo.toml` 里有 `rtrb`、`serde`（带 `derive`）、`serde_json` 三个
> `{ optional = true }`：前两个是 `Gba::init_audio` / 派生宏要求**我们这侧**能命名这些
> 类型。全部挂在 `gba` feature 下，`--no-default-features` 时不进图（不变式 8）。

## 门禁（这台机器上跑得通的写法）

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# GBA 锁测试
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"
# vendor 核心自己的单元测试（r17 后 **249** 项；此前 242 = 244 − 2 ignored）。**这一条以前不存在**：
# vendoring 丢了 [dev-dependencies]，pretty_assertions 缺失使 `cargo test -p gba-core`
# 根本编不过，而门禁只跑根 crate，于是这 242 项一直「不存在」而不是「失败」。
# bus.rs 的时序测试全在这里，P2 之后要在时序模型上动手，先把它跑起来。
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib"
# **NES 侧两个 crate 的 Rust 锁测试（r26 补录）**：`cargo test -p fceux11-rust` **不跑
# 依赖 crate 的测试**（`fceux11-debug` / `fceux11-media` 是 path dependency，其
# `#[cfg(test)]` 模块在该命令下不参与编译）⇒ 这两条命令缺了，那 93 + 52 项
# **一次都不会跑**，且没有任何一层报警（A5 G3 的绝对金标准向量锁就在 media 里）。
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-debug --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-media"
# **FC loader/formats crate（r8 补录，2026-11-10）**：此前的门禁清单漏了它——
# `fceux11-formats` 的 150 项测试**一次都没被任何命令跑过**（纪律 10 的活例）。
# v2.0.3 S4（FC 侧修复批次）要动 iNES loader，这条是它的门禁；基线 **153 项全绿**
# （r9 加 1 条 S4 锁 + r11 加 2 条 S5 锁）。
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-formats --lib"

# 两态 feature 检查（第二态同时复核不变式 8：关 feature 时 vendor 树退出编译图）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace --no-default-features"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace"

# 全量构建 + NES 零回归
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4
```

### 十二个坑

1. **不加载 MSVC 环境就连标准库头都找不到**（`cl.exe` 在，`INCLUDE` 不在）。先 `call vcvars64.bat`。
2. **`cargo test -p fceux11-rust` 必须加 `--no-default-features --features gba`**。默认 feature
   `direct-adapter` 引用只有 C++ 链接才有的 `kagami_bridge_*`，测试二进制直接 LNK2019 链不过。
3. **`ctest` 不在 PATH**，在 `D:\Program Files\CMake\bin\`。
4. **跑 `build/tests/*.exe` 要把 `vcpkg_installed\x64-windows\bin` 前置到 PATH**，否则 `0xC0000135`。
5. **仓库在 HEAD 上本来就不是 rustfmt-clean 的**，`cargo fmt --check` 不能当门禁。
   只保证**自己新写的行**符合 rustfmt，别整仓跑 `cargo fmt`。
6. **Ninja 对头文件变更不可靠**：改完 `src/version.h` 之类的「只被头文件引用」的值，
   可能不会触发重编，链接日志有 `Linking` 而二进制里还是旧值。改完要回**产物**里确认。
7. **`build/` 的 Ninja 定位可能漂移，且 MSVC 升级会让既有测试编不过**（记为 **L11**）。
   症状：`CMAKE_MAKE_PROGRAM-NOTFOUND` → 「Generator: build tool execution
   failed」；修好后又见 `C2220 以下警告被视为错误`，出自 **MSVC 14.51** 的
   `__msvc_ostream.hpp` / `chrono`。**这不是 v2.0 改动引起的** —— 判据是
   `git stash` 后在干净树上能否复现。Ninja 本体在
   `…\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe`。
8. **查符号前先确认 CMake 用的是哪一份 `.lib`。** S0 时期 `build/` 下留过 4 份实验产物
   （`s0p_target` / `s0_exp3_target` / `s0_probe_target2` / `build\src\rust\probe`），
   都只含 5 个探针符号；我按「找第一个 .lib」的写法查，差点报出一个不存在的缺陷。
   CMake 真正链接的是 `src/rust/CMakeLists.txt:34` 的 `${CMAKE_CURRENT_BINARY_DIR}/target`
   下的 `release\fceux11_rust.lib`。*（那 4 份现已清理，当前 `build\` 下只有一份。）*
9. **GBA 的 C ABI 是手写在 `build.rs` 里的，加导出函数必须同步两处**。
   cbindgen 只对**成员 crate** 跑，根 crate 的 ABI 它从不指向。两个后果：
   - **`build.rs` 的 `rerun-if-changed` 必须含 `src/gba`**（现已补）。漏了它就变成
     **库重编了、派生的 `fceux11_rust.h` 却没重生成** —— 符号在 `.lib` 里、头文件里
     没有声明，C++ 侧能链接不能调用，全程零报错。
   - **`src/gba/ffi.rs` 的 `drift_guard` 靠扫源码发现导出，不要把它改回手写清单。**
     手写清单比手写生成的头文件，两边同源漏掉新项就互相抵消、断言照样全绿。改完 ABI
     后自查：`dumpbin /linkermember:1 build\src\rust\target\x86_64-pc-windows-msvc\release\fceux11_rust.lib`
     的 `gba_*` 数量应与 `src/rust/fceux11_rust.h` 的声明数一致（当前 **34**）。
10. **生成头 `src/rust/fceux11_rust.h` 被两个独立的 cargo target 目录共用**，
    **谁最后跑谁写这个文件**（`cargo test` 用 `src/rust/target`，CMake 用
    `build\src\rust\target`）。于是它可以**一边被重新生成、一边留着另一份旧的** ——
    我改完 `build.rs` 的错误码之后 `ctest` 绿、头里却还是 `GBA_ERR_STATE 4`。已补
    `the_error_codes_reach_the_generated_header` 守这一条。**改完 `build.rs` 请直接查
    头文件的内容，不要假定构建跑过就等于它更新了。**
11. **不要用 PowerShell 的 `WriteAllText` / `WriteAllLines` 改仓库里的源文件。**
    前者会剥掉 UTF-8 BOM，后两者按 `Environment.NewLine`（CRLF）重排行尾。
    **⚠️ 行尾口径已更正（2026-11-06 实测）**：「仓库是 LF」只对 **git blob 存储**成立
    （blob 内无 CR 字符）；**工作树文件因 `core.autocrlf=true` 且仓库无 `.gitattributes`
    ⇒ 检出后统一是 CRLF**（抽样 20 个 `src/**` 源文件：纯 CRLF 20、纯 LF 0）。
    当年计划文档被搞成 885 插入 / 876 删除，根因是**行尾被整体重写**（不是内容大改）。
    **判据**：改了文件后先看 `git diff --numstat` —— 插入/删除行数远超真实改动量
    ⇒ 就是行尾被动了。**用编辑工具原地替换，或改完立刻核对行尾**；
    `git apply` 移植补丁时注意源副本的行尾形态（CRLF 副本的补丁会把目标写成 CRLF）。
    今天为此返工三次。
12. **本机 `ninja -j 12`（默认并行 = CPU 数）会在归档阶段偶发 `LNK1104 无法打开 xxx.lib`。**
    2026-11-06 实测：症状是**连续失败在不同的 `.lib` 上**（先 `fceux11_utils.lib`、
    后 `fceux11_boards.lib`），且**手动单跑同一条 `lib.exe` 命令却成功**（EXIT=0）——
    即多个 `lib.exe` 并发归档时争写同一输出。**解法：`ninja -C build -j 1` 串行构建**，
    本轮所有构建均用 `-j 1`，稳定通过。**判据**：LNK1104 且手动跑同一命令能过 ⇒ 并发归档冲突，
    别去清缓存或改构建配置（**L11 的判据仍适用**：先在干净树上确认不是自己改动引起的）。

## 纪律（这几条都是被违反过之后写下来的）

1. **先只读调查 + 出 PLAN，用户拍板后再动文件。**
2. **先记入 §十四 变更记录，再改计划**（计划 §十四末尾的「定稿后的变更纪律」）。
   出口判据的改写也要记，不许静默放宽。
3. **变异验证**：每条新锁测试都要能把「修复前的值」改回去并看到它转红。全绿不算证据。
   **推论：一条「通过原因和断言名字不是同一条路径」的测试是无效的** —— 我写过两条：
   一条把被断言的守卫删掉它照样全绿，一条断言的是另一条代码路径（详见 §十四 r43 与
   S3-1 实施记录）。变异时留意这一类。
4. **不凭记忆写位级格式。** 取不到权威材料就**停下来记录**，不要写。已犯过两次
   （LZ77 头解析连续读错 GBA 与 NDS 两套布局，详见 §十四 r12 / r13），第三次是
   2026-10-01 想消 L6 而**取不到 SIO/声音寄存器的复位值表**（GBATEK 对本工具 403，
   镜像只给寄存器映射）—— 那一轮的失败过程记在 §十四 r43 ② 与 §9.1 的 L6 条里。
   外部参考实现只**读不抄**（mGBA 是 MPL-2.0，与本项目许可不相容）。
5. **测试向量按字段拼装，不手写字面量。** 手算错误在本项目已发生两次，
   而且两次都被自己写的测试盖章通过。
6. **`extern "C"` 出口面必须能被安全调用**：符号在 lib 里是最弱的一档判据；
   导出函数 panic 会在 `extern "C"` 帧里无法 unwind，直接带走整个进程。
7. **NES 侧零回归**是不可变式 1：任何 GBA 改动后都要跑 `ctest`。
8. **分层别越界**（2026-10-01 实测两次）：`gba_load.cpp` 属于 `fceux11_core`，
   **核心库不能依赖 Qt 驱动**。我曾在那里调 `WriteSound`，编译过、链接炸了 ——
   4 个 F11QA 测试可执行文件链接 `core` 而不链驱动，每一个都 `LNK2019`。
   **需要驱动的东西就写在驱动层。**
9. **会误报的守卫等于没有守卫** —— 它只会被关掉。`gba::decoupling` 的第一版把
   `Format_RGBA8888`、`PRGBanks`、`logBankNumCbox` 判成「提到 GBA」，报了 11 个文件；
   后来又发现它**太松的另一半**：`sdlGbaTexture` 匹配不上任何判据，**一个 GBA 专属
   成员可以完全躲开守卫**。判据要**大小写敏感 + 前缀锚定**，匹配前**先剥掉注释与
   字符串字面量**，并且**宁可漏也不要误报**。
10. **编不过的测试等于不存在，「全绿」也可能是从没跑过。** `gba-core` 的 242 项单元
   测试因为 vendor 时丢了 `[dev-dependencies]`（`pretty_assertions`）而**根本编不过**，
   而门禁只跑根 crate，于是它们「不存在」而不是「失败」，没有任何一层报警。**加一条
   门禁命令不等于它会跑** —— 新增门禁后要**当场看到它的测试计数**，计数为 0 或「跑不
   起来」都算没加。这一条与第 9 条同族：守卫失效的两种方式是**误报**与**根本不执行**，
   后者更隐蔽，因为它连红都不红。
   **⚠️ r26 补录的反向实例（2026-11-08）**：`cargo test -p fceux11-rust` **不跑依赖
   crate 的测试**（`fceux11-debug` / `fceux11-media` 是 path dependency，测试模块在
   该命令下不参与编译）⇒ F-2 的 `lock_disasm_mnemonics_256` 与 A5 G3 的
   `lock_resample_interpolation_golden_vector`（各在其 crate 内）**都写好了却一次都
   没被执行过**，且 CI 侧**完全没有 cargo 环节**（`f11qa.yml` 注释自陈「there is no a
   `cargo test` step」）。**「测试写了」与「测试会跑」是两件事** —— 新增锁测试后要确认
   它**落在哪条门禁命令的路径上**；若不在，登记到 AGENTS.md 门禁节（本轮已补两条）。
11. **测量窗口禁止绑 PC / 取指指针（推测态），必须绑架构精确态，并加 armed 标志。**
   2026-10-06 的 v2.0.1 调查里这一条**被同一个坑咬了三次**：AGBA / ROM 探针与单元测试
   都用「PC 到达循环之后那条指令」当退出条件，而 ARM7TDMI 的**取指指针会越过 `BNE`
   投机地取到循环之后的地址** —— 于是「出口」在第一次迭代就触发，读出「循环只跑了一次」。
   第三次的表现最有欺骗性：不是慢，是**报出 12 步/指令这种看起来像重大缺陷的数**。
   两条规矩：① 夹逼用**架构精确态**——寄存器值、内存值，**不要用 PC**；② 退出条件要
   **armed**：机器上电时寄存器常是 0，不先确认「它已经被程序写成目标值」就直接判退出，
   会在第 0 步命中。
   **判据**：一个测量若需要「多跑一会儿看看」才成立，它就没有夹住窗口。
   **同族**：变异验证里「测了改前的值」是同一类错误的另一面 —— 断言的**通过原因**必须
   与它**要验的状态**是同一个可观测量。
12. **「相对比较型」锁测试对「共因变异」结构性无感 —— 要验共享路径上的语义，必须用绝对金标准。**
   2026-11-06 的 A5 G3 实测：`filter.rs` 的三条锁（`lock_fir_pair_scalar_simd_bit_exact` /
   `lock_resample_end_to_end_scalar_simd_bit_exact` / `lock_domain_fallback_on_negative_sample`）
   **全是「此路对彼路」的相对比较**。而 `resample` 的插值公式位于 `if simd_ok{AVX2}else{标量}`
   分叉**之后** ⇒ 标量与 SIMD **共用同一条公式**。于是把插值权重翻转
   （`65536-frac` → `frac`）这条变异让**两边同步漂移**，三条锁**全部全绿**（51/51），
   而探针证明该变异**确实改变了真实输出**（非 no-op）。
   **判据**：断言若只比较「两条都经过被测代码的路径」，它对「该路径自身的语义错误」永远瞎。
   要验这种语义，期望值必须来自**独立参照实现**或**绝对金标准向量**（本轮新增
   `lock_resample_interpolation_golden_vector`：期望值由线性插值定义推导、不调用被测代码）。
   **这是纪律 3 的第三例**（前两例见 r43 与 S3-1 实施记录）—— 且这次连**代码注释里的
   覆盖声明**（「M-D 由 2b 抓住」）都是错的，注释写于实测之前。
   **推论**：写锁测试时，「这条测试声称覆盖 X」必须**实测验证**该覆盖，不能靠推理断言。
13. **门禁的「捕获内容」要验证它非空 —— 全零/静音的 golden 是无牙门禁。**
   2026-11-06 实测：`apu_wav_hq_diff_test` 与 kgmqa-024 低质量 golden 的**捕获样本全为 0**
   （各 799 样本）。真因**不是**「FIR 输入为零」——管线探针实测 FIR 输入是**恒定 DC 4185**
   （非零），是下游 `SexyFilter`（隔直 IIR）把 DC 衰减到 0，且**捕获帧早已收敛**；
   纯 DC ⇒ 任何保持纯 DC 特性的 FIR 变化都不可观测。实证：改 bias / 改系数表 ⇒ golden 仍绿；
   注入 **DC** 偏置 ⇒ 仍绿；注入 **AC** 分量 ⇒ **转红**（通路是活的，机制没坏，缺的是含 AC 的输入）。
   **判据**：新增 golden 门禁必须**断言捕获内容非零**（fail-fast），否则它会在某次
   fixture 更换后静默退化成「哑弹」，且没有任何一层报警。**这与第 10 条同族**：
   守卫失效的第三种方式是「**捕获不到任何东西**」。
   根因侧的可查线索：`tests/fixtures/generate_test_roms.py:251,296` 的 fixture PRG 是
   `0xEA` NOP sled（不写任何 APU 寄存器）⇒ 音频链全程静音。
