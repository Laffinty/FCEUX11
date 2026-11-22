# FCEUX11 v2.0.8 批 2 — 崩溃韧性（SRAM 周期落盘 + 本地 minidump）· 构建计划

> **STATUS: 在途**（2026-11-22 立项，r1）。本文件是主计划 §三 的**专项细化文档**
> （业主指示：批 2 相对复杂，独立立项）。主计划 =
> `docs/plans/FCEUX11-v2.0.8_许可证合规与崩溃韧性-构建计划.md`（r1–r3）。
> 业主裁决 **D1 = 本地 minidump**（不接 Sentry，§0.3 权威口径已记主计划）。
>
> **坐标口径**：以 `main` @ `29560c7`（批 1 收口提交）实测为准；过期以源码为准。

---

## 〇、定位与范围

| 项 | 内容 | 现状 |
|---|---|---|
| **C1** | NES 电池存档周期落盘（根治「崩溃丢电池进度」） | 未实现——`FCEU_SaveGameSave` 仅 GI_CLOSE 调用 |
| **C2** | 崩溃转储（SEH + 本地 minidump + 崩溃摘要） | 未实现——全仓零 SEH/`MiniDumpWriteDump` |
| **C3** | GBA `.srm` 周期落盘 | **已存在**（r1 调查证实，见 §四）——转验证项 |

**明确排除**：Sentry/云端遥测（D1 裁决）、「核心自愈/降级」（语义不成立，主计划 §1.2
定性——落地物 = 转储 + 落盘）、日志分级体系（批 3）。

---

## 一、事实底座（2026-11-22 逐条实测）

### 1.1 GI 分发器全貌（C1 的路由面）

`GameInterface` 是**全局函数指针**（`src/fceu.h:126`），GI 枚举在 `src/fceu.h:120-123`
（`GI_RESETM2=1 / GI_POWER=2 / GI_CLOSE=3 / GI_RESETSAVE=4`）。装载链
（`src/fceu.cpp:499-516`：iNES → NSF → UNIF → FDS → GBA）各 loader 各自注册：

| 分发器 | 定义 | 注册点 | 电池？ | GI_SAVESRAM 处置 |
|---|---|---|---|---|
| `iNESGI` | `ines_gi.cpp:58` | `ines.cpp:76` | ✔（GI_CLOSE→`FCEU_SaveGameSave(&iNESCart)` `ines_gi.cpp:57`） | **加 case** |
| `UNIFGI` | `unif.cpp:43` | `unif.cpp:67` | ✔（`unif.cpp:33` 同款） | **加 case** |
| `FDSGI` | `fds.cpp:78` | `fds.cpp:274` | ✘（GI_CLOSE=`FDSClose` 写 `.fds` 磁盘镜像，非电池 `.sav`） | default 无操作 |
| `NSFGI` | `nsf.cpp:68` | `nsf.cpp:97` | ✘ | default 无操作 |
| `FCEUXGameInterface` | `fceu.cpp:1515` | `fceu.cpp:1571`（`FCEUXLoad`，现行装载链**未调用**，备用路径） | ✘ | default 无操作 |

### 1.2 写盘路径（C1 的原子写落点）

- `FCEU_SaveGameSave`（`src/cart.cpp:430`）→ 组装 `FceuSaveGameEntry[]` →
  `fceux11_rust_cart_battery_save`（`src/rust/crates/fceux11-formats/src/cart.rs:193`）
  → **`File::create(path)` 直写**（`cart.rs:205`）——**非原子**：崩溃写一半 ⇒
  `.sav` 截断损坏。`battery_load`（`cart.rs:237`）同文件系。
- 调用方仅 GI_CLOSE 两处（§1.1）⇒ 运行期从未写盘。

### 1.3 emu 循环与配置（C1 的触发面）

- 每帧入口 `fceuWrapperUpdate()`（`src/drivers/Qt/fceuWrapper.cpp:1390`）——
  **模拟线程**持 `fceuWrapperTryLock`，内含 GBA 分支 / NES `DoFun` 分支 + 会话翻转检测
  （V15 输入快照注释块为界标）。**flush tick 落点 = 帧步完成后、锁内**：
  SaveGame 缓冲的写者就是本线程 ⇒ 快照一致，**零新增锁**；暂停 = 无帧 = 不刷
  （语义正确）；turbo = 按墙钟而非帧数（用 `std::steady_clock`）。
- 配置项模式：`src/drivers/Qt/config.cpp:515`
  `config->addOption('s', "sound", "SDL.Sound", 1)`（末参 = 默认值）。
  新项：`config->addOption("sramFlushInterval", "SDL.SRAMFlushInterval", 30)`。
- GBA 同款已存在（§四）——NES 侧 tick 写在 `fceuWrapper.cpp`（驱动层读配置、
  调 core 的 flush；纪律 8 分层）。

### 1.4 崩溃处理现状（C2 的安装面）

- 全仓零 `SetUnhandledExceptionFilter` / `MiniDumpWriteDump` / CRT 补丁（grep 零命中）。
- 入口 `main()`（`src/drivers/Qt/main.cpp:332`）首行已是 `fceux_diag::install()`
  （**仅版本串**，`diag_api.h`）⇒ crash 安装调用插其后；安装必须早于 QApplication。
- 核心源清单 `set(SRC_CORE ...)`（`src/CMakeLists.txt:160`）；链接行
  `target_link_libraries(fceux11_core ...)`（`:592`）——需加 `Dbghelp`（Windows）。
- 路径设施：`FCEUMKF_*` 枚举（`src/file.h:166-179`）、`FCEU_MakeFName`
  （`src/file.cpp:621`，case → `odirs[x]` 覆盖或 `BaseDirectory + PSS + 子目录`，
  如 `"fcs"`）、`fceu11::GetBaseDirectory()`（`file.cpp:480`）⇒ 新增
  `FCEUMKF_CRASH` → `crashes/`。
- 测试基建约束：核心仍引用 ~15 个 Qt 驱动符号（`tests/f11qa/headless_smoke_test.cpp:8`
  注释）⇒ **直载 ROM 的 ctest 不可行**；可行面 = ① Rust formats 单元锁（注入失败
  writer）② C++ 分发级 ctest（`FCEUGI` 可裸构造 `fceu.cpp:111`；`GameInterface`
  可指 stub）③ GUI 人工 e2e。

### 1.5 解耦与红线落点

- `decoupling.rs` 白名单含 `fceuWrapper.cpp`（tick 落点合法，无需新登记）；
  新文件 `crash_handler.{h,cpp}` **不得出现 "GBA" 字串**（守卫会红）。
- GBA 的 flush 机制（§四）**零改动**——C3 是验证不是实现。

---

## 二、C1 · SRAM 周期落盘

### 2.1 设计（五步链路）

```
帧步完成（emu 线程，锁内）
  → fceuWrapper.cpp: sram_flush_tick()         [驱动层：steady_clock 间隔判定 + 读 SDL.SRAMFlushInterval]
    → FCEU_FlushGameSave()                     [cart.cpp：GameInfo/GameInterface 判空]
      → GameInterface(GI_SAVESRAM)             [fceu.h:120 新枚举值 = 5]
        → iNESGI/UNIFGI: FCEU_SaveGameSave(...) [既有函数复用]
          → fceux11_rust_cart_battery_save      [cart.rs：改原子写]
```

1. **枚举**：`GI_SAVESRAM = 5`（`fceu.h:123` 后）；`iNESGI` / `UNIFGI` 各加
   `case GI_SAVESRAM: FCEU_SaveGameSave(&xxxCart); break;`——FDS/NSF/FCEUX
   分发器 default 已是无操作（显式不处理，注释说明为何不加：无电池 `.sav` 面）。
2. **flush 入口**：`cart.cpp` 新函数
   `void FCEU_FlushGameSave(void) { if (GameInfo && GameInterface) GameInterface(GI_SAVESRAM); }`
   （声明进 `cart.h`，与 `FCEU_SaveGameSave` 并排）。
3. **tick**（`fceuWrapper.cpp`，文件级 static）：
   `steady_clock` 上次刷写时刻；每帧步后判 `elapsed >= interval`；`interval`
   每帧读 `g_config->getOption("SDL.SRAMFlushInterval")`（map 查找，开销可忽略），
   `0 = 关`；仅在 `GameInfo` 存在（NES 分支语义）时调用——GBA 走既有机制。
   首帧记基准（armed 语义：装载后首次刷写 = interval 到点，不是第 0 帧）。
4. **原子写**（`cart.rs`）：抽内部助手
   `fn atomic_write_file(path, write_fn) -> bool`——先写 `<path>.tmp`
   （同卷 ⇒ rename 原子），成功后 `fs::rename`；失败清理 tmp 并保持原文件不动。
   `fceux11_rust_cart_battery_save` 改走该助手。**失败注入测试**（见 2.2）——
   这是原子性语义的锁，生产代码不经特殊化参数。
5. **配置**：`config.cpp` 加 `SDL.SRAMFlushInterval`（默认 30，秒；0 = 关）。

### 2.2 验收与变异腿（纪律 3）

| # | 锁 | 断言 | 变异腿 |
|---|---|---|---|
| M1 | formats `atomic_write_failing_writer` | 注入第 N 字节失败的 writer ⇒ 目标文件内容不变、无 tmp 残留、返回 false | 助手改直写目标 ⇒ 目标被截断 ⇒ **红** |
| M2 | formats `atomic_write_success` | 正常写入后最终文件字节正确、tmp 不存在 | 改 rename 为 copy ⇒ 腿 1 才红，本条作绿对照 |
| M3 | ctest `sram_flush_test` ① | 无游戏（GameInfo==NULL）调 `FCEU_FlushGameSave()` ⇒ 安全 no-op 不崩 | —（负腿常绿） |
| M4 | 同 ② | `GameInfo = new FCEUGI()` + `GameInterface = stub` ⇒ stub 收到 **GI_SAVESRAM**（值 5） | flush 改发 `GI_CLOSE` ⇒ stub 记录不符 ⇒ **红** |
| M5 | 同 ③④ | `iNESGI(GI_SAVESRAM)`：`battery=0` ⇒ 不写盘；`battery=1` + 手工填 `iNESCart.SaveGame` 缓冲 ⇒ `.sav` 生成且字节正确（路径经 `FCEU_MakeFName`，file base 指向临时目录；若 file base 不可控则降级为「写出被调用 + 无异常」，e2e 由人工清单兜底） | 删 case ⇒ M5 无写盘 ⇒ **红** |
| 人工 | GUI e2e | 电池 ROM + `SDL.SRAMFlushInterval=5` ⇒ 玩 10 秒 `.sav` mtime 更新；interval=0 ⇒ 不更新 | — |

行为变更登记（CHANGELOG）：游戏运行中 `.sav` 的 mtime 周期更新（默认开）；
`interval=0` 时行为与改前逐字相同（反向守卫）。

### 2.3 风险

- 分发器遗漏（某个 loader 的 GI 函数没加 case）⇒ flush 对该类 ROM 静默无效
  ⇒ M4/M5 的 stub 断言按分发器逐一覆盖；NSF/FDS default 显式 `break` 注释。
- `GameInterface` 为 NULL 的窗口（load 失败中段）⇒ 判空（M3 同源）。
- flush 与崩溃竞争 ⇒ tmp+rename 即解（写一半只损 tmp）。
- LTCG 链接病态（坑 13）⇒ 新 TU 若中招按先例 `/GL-`。

---

## 三、C2 · 崩溃转储（本地 minidump，D1 裁决）

### 3.1 设计

新文件 `src/crash_handler.{h,cpp}`（**core 层，零 Qt 依赖**，不含 "GBA" 字串）：

1. **安装**（`main.cpp:332`，`fceux_diag::install()` 之后、QApplication 之前）：
   `SetUnhandledExceptionFilter` + CRT 补丁族（`_set_invalid_parameter_handler` /
   `_set_purecall_handler` / `std::set_terminate` / `signal(SIGABRT)`）统一汇入
   一个写入口；一次性重入标志防递归。
2. **写入口**（可测形态）：`write_crash_dump(EXCEPTION_POINTERS*, const char* dir)`
   —— `MiniDumpWriteDump`（`MiniDumpNormal`，链 `Dbghelp`）写
   `crashes/fceux11_<yyyymmdd_hhmmss>_<pid>.dmp`；随后 `crashes/crash_log.txt`
   追加一行：时间戳 / `FCEU_DISPLAY_VERSION` / 异常码 / 异常地址 /
   当前 ROM 名（各读取 try-guard；filter 内**零 UI、最小分配**）；SEH filter 返回
   `EXCEPTION_CONTINUE_SEARCH`（交还 WER，不吞进程）。
   terminate/abort/purecall 路径无异常上下文 ⇒ 传 NULL 上下文仍写 dump
   （当前线程快照）+ 摘要（来源标记 "terminate"/"abort"/...）。
3. **目录**：`FCEUMKF_CRASH`（`file.h` 枚举 + `file.cpp` case，默认
   `BaseDirectory + "crashes"`）；`.gitignore` 加 `crashes/`（转储不进 git）。
4. **探针**：`FCEUX11_TEST_CRASH=1` ⇒ `main()` 安装后注入一次受控崩溃
   （`RaiseException`）——debug 与 release 均可用（人工验收证据）。

### 3.2 验收与变异腿

| # | 锁 | 断言 | 变异腿 |
|---|---|---|---|
| M6 | ctest `crash_dump_test` | 直调 `write_crash_dump`（栈上构造 `EXCEPTION_RECORD`+`CONTEXT` 的 fabricated `EXCEPTION_POINTERS`）⇒ `.dmp` 存在且 **>0 字节**、`crash_log.txt` 追加行含版本与异常码（非零捕获，纪律 13） | dump 写入函数提前 return ⇒ 无 dmp ⇒ **红** |
| M7 | 同 | 无效路径（不可写目录）⇒ 返回 false、不崩、主流程存活 | 失败吞异常不返回 ⇒ **红** |
| 人工 | 探针 | release 构建 + `FCEUX11_TEST_CRASH=1` 启动 ⇒ crashes/ 出 dmp + 摘要行 + 进程按 WER 语义终止 | — |
| 人工 | 真崩溃 | 人工触发一次真实崩溃（如有）⇒ dmp 可被 WinDbg 打开出调用栈 | — |

### 3.3 风险

- Qt 进程内 SEH filter 被第三方覆盖（Qt 自身不装）⇒ 探针实证；CRT 补丁族覆盖
  非 SEH 退出面。
- `MiniDumpWriteDump` 本身崩溃 ⇒ 重入标志 + 失败即放弃（不二次崩溃）。
- dump 尺寸：`MiniDumpNormal` 最小级，典型几百 KB～数 MB，可接受。
- 转储含用户 ROM 数据 ⇒ 本地文件不上传（D1 口径），`crashes/` 不进 git。
- 多实例并发写同名 ⇒ 文件名含 pid。

---

## 四、C3 · GBA `.srm`（验证项——机制已存在）

**r1 调查证实**（推翻主计划 §三 C3「另行实现」的前提）：`gba_load.cpp:241`
`fceu11_gba_flush_battery(bool force = false)` 已实现完整机制——

- **脏标志**：`gba_battery_take_dirty()`（`:258`）——游戏写存档内存才置位；
- **限频**：最小写入间隔常量（`:226`，毫秒）；
- **每帧调用**：`fceu11_gba_frame()` 内（`:536`，注释明言「一帧一个标志测试，
  限频写入」）——与 C1 的 tick 同语义；
- **关闭强制**：`fceu11_gba_deactivate()`（`:350`）`force=true` 刷最后一秒。

⇒ **C3 = 验证收尾，零新代码**：
① GBA 根 crate 锁测试计数不减（303+20i 基线）；② ctest 全绿；③ 人工：GBA ROM
玩 30 秒 `.srm` mtime 更新（若该卡带存档类型判明且写入）；④ 文档核对——
`AGENTS.md`/CHANGELOG 把「GBA .srm 周期落盘」记为**既有行为**（本批只登记不实现）。

---

## 五、门禁与验收总表

| 门禁 | 基线 | 期望 |
|---|---|---|
| 全量 Release 构建（`--parallel 1`） | BUILD_EXIT=0 | EXIT=0 |
| `ctest -LE perf` | 49 | **51**（+`sram_flush_test` +`crash_dump_test`） |
| `cargo test -p fceux11-formats --lib` | 158 | **160**（+2 原子写锁） |
| 其余 Rust crate | 303+20i / 249 / 94 / 52 / 51 / 54 / 184 | 计数不减、全绿 |
| `cargo check --workspace` 两态 | EXIT=0 | EXIT=0（cart.rs 改动后重跑） |
| `check.py`（批 1 门禁） | EXIT=0 | EXIT=0（新文件带头！） |
| decoupling 三守卫 | 绿 | 绿（fceuWrapper.cpp 已白名单；新文件无 GBA 字串） |
| 人工清单 | — | 探针崩溃出 dmp / `.sav` mtime 观察 / interval=0 反向守卫 / GBA `.srm` mtime 观察 |

新 ctest 注册：`tests/CMakeLists.txt`（`fceux11_add_test_executable` + `add_test`
模式，参照 `smoke_test` 行 `:81-85`）。

---

## 六、不变式与红线

1. **NES 零回归**（不变式 1）：C1 触碰 GI 分发链与 cart.rs ⇒ 每步 ctest。
2. **GBA 解耦**：C1/C2 新代码零 GBA 引用；C3 零代码改动；fceuWrapper.cpp tick
   在既有白名单内。
3. **vendor 树**：cart.rs 是 `fceux11-formats`（本仓 crate）非 vendor ⇒ 可改；
   gba-core 依旧零触碰。
4. **纪律 8 分层**：flush tick 读配置 = 驱动层；`FCEU_FlushGameSave` = core；
   `crash_handler` = core（无 Qt）；dump 目录 = core 路径设施。
5. **纪律 2**：每阶段开工前先记 r；出口判据不静默放宽。
6. **坑 11**：cart.rs 改动用编辑工具原地替换；改后 `git diff --numstat` 核对。

---

## 七、风险与止损

- **止损线**：M1 原子写锁写不出注入点 ⇒ 停止，如实登记「原子性仅靠 rename 语义
  人工评审」；M6 在 LTCG/Qt 环境不稳定 ⇒ 降级为探针人工验收 + 登记。
- **回滚**：C1/C2 分开提交；任一项出问题单独 revert 不牵连。
- **行为兼容**：FCSX/FM2/`.sav` 文件格式零变化（只加写盘时机）⇒ 存档互读不受影响。

---

## 八、阶段编排

| 阶段 | 内容 | 依赖 |
|---|---|---|
| P1 | C1 步骤 1-2（枚举 + flush 入口 + M3/M4/M5 ctest） | 无 |
| P2 | C1 步骤 4（cart.rs 原子写 + M1/M2） | 无（可与 P1 并行） |
| P3 | C1 步骤 3+5（tick + 配置项）+ 人工 e2e | P1 |
| P4 | C2 全部（crash_handler + 路径 + 探针 + M6/M7） | 无 |
| P5 | C3 验证收尾 + CHANGELOG/AGENTS 登记 + 门禁全表 | P1-P4 |

每阶段收口跑 §五 全表；**push / 发版动作等业主明示**。

---

## 九、明确不做（批 2 范围外）

- Sentry / crashpad / breakpad（D1：本地零依赖）。
- 自愈/降级/看门狗重启（语义不成立）。
- .sav 备份历史链（undo 已有即时存档层管）。
- 崩溃收集上报 UI（批 3 诊断对话框只读展示 crashes/ 计数）。
- FDS `.fds` 磁盘镜像的周期落盘（磁盘镜像量大且 FDSGI 语义不同，维持 GI_CLOSE）。

---

## 十四、变更记录

- **r1（2026-11-22）立项**：业主指示批 2 独立建档。只读调查完成——
  **C3 机制证实现已存在**（`fceu11_gba_flush_battery` 脏标志+限频+每帧+关闭强刷，
  主计划 §三 C3 的「另行实现」前提作废，转验证项）；GI 分发器五路全貌钉死
  （iNESGI/UNIFGI 有电池面，FDS/NSF/FCEUX default 无操作）；Rust 侧
  `File::create` 直写非原子实证（cart.rs:205）；emu 线程锁内 tick 设计定型
  （零新增锁、暂停不刷、墙钟限频）；测试策略按「核心直载 ROM 不可行」约束定为
  分发级 ctest + formats 注入锁 + 人工 e2e。C1 五步链路 / C2 四件套设计 / M1–M7
  变异腿与门禁总表（ctest 49→51、formats 158→160）落档。
- **r2（2026-11-22）P1 开工**：业主改为**串行作业**（每 PHASE 完成即汇报）。P1 =
  C1 步骤 1-2（GI_SAVESRAM 枚举 + iNESGI/UNIFGI case + `FCEU_FlushGameSave` +
  M3/M4/M5 分发级 ctest）。实施备忘：M4/M5 变异腿用 **`cmake --build --target
  fceux11_sram_flush_test` 目标级构建**降本（LTCG 全量重链 30–60 min，单目标
  5–25 min）；M5 的 `.sav` 路径依赖 `FCEU_MakeFName`/FileBase，实施时钉锚点，
  不可控则按计划降级为「写出被调用」断言 + 人工 e2e 兜底。
- **r3（2026-11-22）P1 完成**：
  - **落地五文件**：`fceu.h`（`GI_SAVESRAM=5` + 分发器处置注释）/ `cart.cpp`
    （`FCEU_FlushGameSave`——`GameInfo && GameInterface` 判空后路由）/
    `cart.h`（声明）/ `ines_gi.cpp` + `unif.cpp`（case GI_SAVESRAM = 仅存档、
    无 Close teardown）；`tests/f11qa/sram_flush_test.cpp` + tests/CMakeLists
    注册（smoke_test 模式同款）。numstat 全纯插入（fceu.h +7/−1 枚举体）。
  - **正向绿**：目标构建 BUILD_EXIT=0（806 步，fceu.h 宽重编被单目标构建一次
    吃下）；测试六腿全 PASS（M3 / M4 / M5a / M5b / M5c，TEST_EXIT=0）。
  - **变异腿双闭合**（目标级构建实跑，每腿红/绿边界与预登记一致）：
    M4（flush 改发 GI_CLOSE）⇒ **M4 红**、M3/M5 绿、EXIT=1；
    M5（iNESGI 摘 case）⇒ **M5b 红**（`.sav` 不写出）、M4/M3/M5a 绿、EXIT=1。
    还原后全绿。还原用 Edit 工具回放（不用 `git checkout`——HEAD 尚无 P1 改动，
    checkout 会连批内成果一起丢，[[git-checkout-drops-uncommitted-work]]）。
  - **终门禁**：全量构建 BUILD_EXIT=0（278 步收尾重链）+ `ctest -LE perf`
    **50/50 全绿**（49→50，`sram_flush_test` 入列），CTEST_EXIT=0。
  - **P1 未含**：P2（cart.rs 原子写）与 P3（tick + 配置项）未动——当前 flush
    只有入口与分发，尚无周期触发与原子性，属预期中间态。
