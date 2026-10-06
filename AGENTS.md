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
| `wip2.0` | GBAEUX11 v2.0 的全部工作；推上去会触发外部门禁（f11qa.yml 已含 `wip2.0`） |

## 治理文档（事实源）

| 文档 | 管什么 |
|---|---|
| `docs/history/plans/FCEUX11-v2.0_GBAEUX11构建计划.md` | **已收口归档（r57）**：范围、阶段编排（§八）、已知限制（§9.1）、**不变式（§十）**、变更记录（§十四 r1–r57）——历史与取证事实源，**内容冻结不再续写**；长尾承接见下方「长尾」节 |
| `docs/plans/FCEUX11-v2.0.1_GBAEUX11精度优化计划.md` | **v2.0.1 精度优化（OPEN）**：按 Phase 独立立项（P0 已完成；P2→P3→P1/P4、P5 待做）。依据是外部测试报告 `docs/tech/gba-external-test-report-2026-10.md`（jsmolka 13 ROM + AGS 老化卡 v7.0），harness 与全部证据在 `Research_only/gbatech/`。**长尾的时序类精度缺口从那里接手** |
| `src/rust/crates/gba-core/ATTRIBUTION.md` | 上游出处、本地补丁集**逐处**说明、重新 vendor 流程 |
| `COPYRIGHT_AUDIT.md` §5 | Rust vendor 树在 `src/` C/C++ 扫描范围之外的归属声明 |

> `docs/plans/long-term-evolution/` 是 **NES 侧** F11QA 残留精度的长期演进，与 v2.0 无关，别混。

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

## 当前进度（2026-10-05）

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
| v2.0.1 P2–P5 | ⬜ 时序类精度（S/N 跟踪 → 预取 → DMA → 定时器 → LCD），每项独立立项，依序见 v2.0.1 计划 §八 |

**导出符号 34 个**（S3-3 零新增）：探针 6 + 生命周期/帧 10 + 音频 5 + savestate 3 + RTC 2 + 输入 2 + 电池 6。
**GBA 锁测试 246 项全绿 + vendor 核心自测 229 项全绿**（7 条 `#[ignore]` 的测量/诊断工具：r37 栈探针、r48 体积探针、S4 的探针与差分转储等；r56 新增 1 条唤醒返回地址锁测试，v2.0.1 P0 新增 2 条 ARM SWI 编号锁测试；P2 先行工作项在 `gba-core` 侧新增 2 条 WAIT 期望值测试 + r5 的 2 条半字载入宽度测试）。

> ⚠️ **不变式 1 的判据是 F11QA 矩阵 `108P/12F`**，本机跑的是它的**本地近似**
> `ctest`。**本地确定性门禁 = `ctest -LE perf`（33 项，须全绿）**；
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

v2.0 对 vendor 树的本地修改是 **21 处、跨 4 个源文件**（2026-10-02 用
`git log --name-only --diff-filter=M -- src/rust/crates/gba-core/src/` 实测得出，
不是手数；第 5 个文件是 `Cargo.toml`，不算源改动；r56 增 1 处，
v2.0.1 P0 增 1 处、P2 先行工作项增 1 处、r5 增 2 处）：

| 文件 | 处数 | 内容 |
|---|---|---|
| `src/cpu/arm7tdmi.rs` | 13 | S0' 接线 5 处 + S1a-1 的 halt 机制 4 处 + r56 的唤醒 IRQ 返回地址 1 处（**改变行为**，见 ATTRIBUTION §3.2.2）+ v2.0.1 P0 的 ARM SWI 编号位 1 处（**改变行为**，见 ATTRIBUTION §3.2.3）+ r5 的 LDRH/LDRSH 半字载入宽度 2 条（**仅测试**，见 §3.2.5） |
| `src/bus.rs` | 1 | v2.0.1 P2 先行工作项：**只有测试**（AGS 老化卡的 WAIT 类期望值，28 项），未动任何生产代码 —— 见 ATTRIBUTION §3.2.4 |
| `src/cpu/hardware/rtc.rs` | 3 | S2-b3：固定时刻覆盖（**不改变上游行为**） |
| `src/cpu/hardware/internal_memory.rs` | 4 | S2-b3 的 `rtc()` / `rtc_mut()` 2 处 + S3-1 的 `backup_type()` / `set_backup_type()` 2 处 |

**后三个文件里的改动都不改变「不覆盖时的行为」。** 但**补丁集已不再是「集中在一个
文件」**（r38 ④ / r46 ①）—— 改动前先确认这一点是否仍成立，`rtc.rs`、
`internal_memory.rs`、`bus.rs` 都要查。重新 vendor 时看 `ATTRIBUTION.md` §3.2 与
§3.2.1–§3.2.5、§3.5。

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
   时序类精度缺口改由 v2.0.1 计划接手，不再记在本节。
4. **§9.1 八条开放限制**（L1/L2/L4/L6/L7/L10/L13/L15）：r42/r54 裁决 Alpha
   阶段长期开放，无需工作；全文见归档计划 §9.1。

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
# vendor 核心自己的单元测试（227 项）。**这一条以前不存在**：vendoring 丢了
# [dev-dependencies]，pretty_assertions 缺失使 `cargo test -p gba-core` 根本编不过，
# 而门禁只跑根 crate，于是这 227 项一直「不存在」而不是「失败」。bus.rs 的时序测试
# 全在这里，P2 之后要在时序模型上动手，先把它跑起来。
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib"

# 两态 feature 检查（第二态同时复核不变式 8：关 feature 时 vendor 树退出编译图）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace --no-default-features"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace"

# 全量构建 + NES 零回归
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4
```

### 十一个坑

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
    前者会剥掉 UTF-8 BOM，后两者按 `Environment.NewLine`（CRLF）重排行尾 —— 仓库
    是 LF，于是**整个文件在 diff 里显示为改动**（计划文档被这么搞成 885 插入 / 876 删除，
    而真实改动只有 10 行）。今天为此返工三次。**用编辑工具，或改完立刻核对行尾。**

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
10. **编不过的测试等于不存在，「全绿」也可能是从没跑过。** `gba-core` 的 227 项单元
   测试因为 vendor 时丢了 `[dev-dependencies]`（`pretty_assertions`）而**根本编不过**，
   而门禁只跑根 crate，于是它们「不存在」而不是「失败」，没有任何一层报警。**加一条
   门禁命令不等于它会跑** —— 新增门禁后要**当场看到它的测试计数**，计数为 0 或「跑不
   起来」都算没加。这一条与第 9 条同族：守卫失效的两种方式是**误报**与**根本不执行**，
   后者更隐蔽，因为它连红都不红。
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
