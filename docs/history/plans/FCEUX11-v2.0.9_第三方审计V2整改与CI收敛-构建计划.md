# FCEUX11 v2.0.9 — 第三方二次审计 V2 整改 + CI 收敛 · 构建计划

> **STATUS: 在途**（2026-11-25 立项，r1）。收口即移 `docs/history/plans/`。
> **触发源**：`docs/third_party_report/FCEUX11二次审计报告V2_2026-11-24.md`（未入库的第三方报告）。
> **坐标口径**：以 `main` @ `efedd83`（v2.0.8 发布提交）实测为准；**过期以源码为准**。
> **业主裁决（2026-11-25）**：① 核实报告问题，确实存在且值得整改的则整改；
> ② **果断移除 CI 侧重复实现**，把 `ci.yml` 独有的检查移植进 `f11qa.yml`；
> ③ 产出 **v2.0.9** 构建计划。

---

## 〇、定位与范围

| 项 | 内容 | 现状（核实结论） |
|---|---|---|
| **P1** | Rust crate 单测从未在 CI 跑 | **属实**——两个 workflow 均无 `cargo test` |
| **P2** | 崩溃报告重入 / 重复 | **属实**——一次崩溃事件产出 2 条摘要 + 第一份转储被覆盖 |
| **P3** | （报告未提）v2.0.8 静默废掉 `fceux_diag` 背迹 | **属实**——`std::set_terminate` 单槽被顶替 |
| **P4** | CI 与 F11QA 重复构建 | **属实**——同一份构建做两遍；业主要求收敛为一次 |

**明确排除**：CodeQL workflow（GitHub 侧 default setup 已启用，仓库内无可整改项）；
`sccache` / 关 LTCG（真编译速度杠杆，但会改变 CI 编出的二进制并掩盖 AGENTS.md 坑 13
那类 LTCG 病态 ⇒ 独立立项）；**tag / GitHub Release / push**（须业主另行明示）。

---

## 一、事实底座（2026-11-24/25 逐条实测，非引用）

### 1.1 P1 · Rust crate 单测缺失（属实）

| 位置 | 实测 |
|---|---|
| `.github/workflows/ci.yml` | 无任何 `cargo test`（只有 `cmake --build`，它编 Rust 静态库但不编测试） |
| `.github/workflows/f11qa.yml:19` | 自陈「There is no `cargo test` step」；唯一 cargo 调用是 `cargo build --release -p f11qa` |
| `.github/workflows/release.yml` | 无 cargo |

**缺口规模**（CI 锁定版工具链 **1.100.0**，本机 `+nightly` 实测，全绿）：

| crate | 通过 / 忽略 | 备注 |
|---|---|---|
| fceux11-rust（根，`--features gba`） | 303 / 20i | GBA 锁测试所在 |
| gba-core | 249 / 2i | vendor 核时序 / 总线 / 定时器 |
| fceux11-formats | 160 / 0 | iNES loader 锁（S4/S5） |
| f11qa | 199 / 0 | 184 lib + **15 集成测试**（`crates/f11qa/tests/{config_store,pixbuf_pool,state_facade}.rs`） |
| fceux11-debug | 94 / 0 | 含 `lock_disasm_mnemonics_256`（F-2 抓出「整个 zp 列解码成 ORA」） |
| fceux11-lua | 54 / 0 | V22 / V25 |
| fceux11-media | 52 / 0 | 含纪律 12 的绝对金标准向量锁 |
| fceux11-utils | 51 / 0 | V24 |
| fceux11-core | 33 / 0 | **此前不在任何门禁**（savestate 序列化 / 总线） |
| fceux11-rom-tests | 2 / 0 | 孤儿 workspace 成员，仓内无消费者 |
| **合计** | **1197 通过 / 22 忽略 / 0 失败** | 热编译总墙钟 ≈ 2 分 09 秒 |

⇒ 与 AGENTS.md 纪律 10 的活例同源（该纪律已记两次：gba-core 的 242 项因 vendoring
丢 `[dev-dependencies]` 而**根本编不过**；`fceux11-debug` / `fceux11-media` 不在任何
门禁命令里）。

### 1.2 P2 · 崩溃报告重复触发（属实，已实测复现）

`src/crash_handler.cpp:53` 的 `bool g_inCrash` 是**普通 bool**，只在 `seh_filter` 与
`write_crash_dump` 置位，**`report()` 本身无任何保护** ⇒ terminate / abort /
purecall / invalid-parameter 四条路径全裸，且非线程安全。

用**真实 `src/crash_handler.cpp`** 实测（探针 `Research_only/audit_v2_probe/`）：

| 子进程动作 | .dmp 文件 | crash_log.txt 行数 | 来源标记 |
|---|---|---|---|
| `std::terminate()` | 1 | **2** | `terminate` + `abort` |
| `abort()` | 1 | 1 | `abort` |
| main 内未捕获 C++ 异常 | 1 | 1 | `seh`（0xE06D7363） |
| 线程内 throw | 1 | 1 | `abort` |

⇒ **一次崩溃事件产生 2 条摘要**；两次写盘同名（秒级时间戳 + PID，`CREATE_ALWAYS`）
⇒ **第一份转储被静默覆盖**。

机制钉死：MSVC 的 `abort()` **确实**调用已注册的 SIGABRT 处理器（实测 sig=22、
退出码 3）⇒ `terminate → abort → abort_handler → report("abort")` 是真实的二次上报。

### 1.3 P3 · 崩溃路径路由表（本次新发现，决定 §三 阶段 2 的设计）

用三个各打不同标记的处理器（SEH / terminate / SIGABRT）实测，**装了 SEH filter
之后**的路由是：

| 触发 | 实际命中的处理器 | 是否经过 `std::terminate` |
|---|---|---|
| main 内逃逸的 C++ 异常 | **SEH**（0xE06D7363） | **否** |
| 线程内逃逸的 C++ 异常 | **SIGABRT** | **否** |
| 显式 `std::terminate()` | terminate → SIGABRT | 是 |
| `abort()` | SIGABRT | — |
| `RaiseException`（非 C++ 异常） | **SEH** | — |

⇒ **SEH filter 会抢在 terminate 之前**（逃逸异常根本不进 terminate），且线程里的
逃逸异常直接走 SIGABRT。**因此「把 terminate handler 串起来」不足以恢复背迹**：
最常见的两条路径都不经过它 ⇒ 必须有一条**所有路径都经过**的钩子（§三 阶段 2 的
pre-hook）。

### 1.4 P4 · CI 与 F11QA 的重复面（收敛的事实依据）

| 步骤 | ci.yml | f11qa.yml | 重复? |
|---|---|---|---|
| checkout / MSVC / CMake / Rust / vcpkg cache / prepare / announce | ✓ | ✓ | 是（廉价） |
| **Configure CMake** | ✓ | ✓ | **是**（同一 `build/` 树） |
| **Build C++**（~62 个测试 exe + Rust 静态库，Release 全开 LTCG） | ✓ | ✓ | **是 ← 最贵** |
| **`ctest --build-config Release -LE perf`**（同一串命令、同 54 项） | ✓ | ✓ | **是** |
| license / 5 个 `git grep` 漂移检查 / golden drift | ✓ | — | ci 独有（**本来就不需构建**） |
| ROM Regression / bench smoke / bench tolerance | ✓ | — | ci 独有 |
| kgmqa-117 pre-flight / `cargo build -p f11qa` / ROM 拉取 / blargg 套件 / **迁移矩阵 + grade + baseline drift + R4 gate + artifact** | — | ✓ | f11qa 独有（发布门禁） |

⇒ **测试内容零重复，重复的是同一份构建做两遍。** 两个 job 并行 ⇒ 墙钟 =
max ≈ 35 分钟（F11QA run #99 实测 34m37s），被翻倍的是 **CI 分钟**。

**为什么 f11qa 那一遍不能省**：迁移矩阵的 runner 是 Rust 的 `f11qa-runner.exe`，
但它经 SubprocessAdapter **spawn `build/tests` 里的 C++ 测试 exe**
（`--bin-dir build/tests`），blargg runner 本身也是 C++ exe ⇒ f11qa 对 C++ 构建
是**硬依赖**。⇒ 唯一出路是**构建收敛为一次**。

**vcpkg 缓存不丢**：两个 workflow 的 cache key 逐字相同
（`vcpkg-${{ hashFiles('vcpkg.json') }}` + restore-keys `vcpkg-`）⇒ 删 ci.yml 不影响
缓存命中。

### 1.5 报告其余裁定的抽查 —— 全部属实

许可证门禁 `check.py` EXIT=0（669 扫描 / 58 白名单，与报告数字一致）；ctest PATH 注入
确由 `tests/CMakeLists.txt:1133` 的目录属性扫描**覆盖全部测试**；12 个 `.ts`；
SRAM 周期落盘链路在位。

**CodeQL 之争**：报告的依据（GitHub 侧 default setup，Run #420）比开发侧
「翻 `.github/workflows/` 找没找到」的反驳更可靠 ⇒ **仓库内无可整改项**（属 GitHub
仓库设置），本次不动。

---

## 二、业主裁决与计划修订（纪律 2：出口判据与设计的改动必须记）

| # | 裁决/修订 | 内容 |
|---|---|---|
| **D1** | 报告两项均属实且整改 | P1 进 CI 门禁；P2 一次性化 + 文件名唯一 |
| **D2** | **CI 收敛** | 删除 `.github/workflows/ci.yml`，独有检查移植进 `f11qa.yml` 的现有 job；**不新建任何 workflow** |
| **D3** | CTest 语义只变严 | f11qa.yml 旧的 ctest 步带 `continue-on-error: true`（**吞掉所有失败**）⇒ 换成 ci.yml 的严格版（失败即 job 失败）+ step summary 镜像 + `::error::FAILED-TEST` 注解 |
| **D4** | Rust 门禁落点 | **放在已有 Rust 构建的同一个 job 内**并与 CMake **共享 `CARGO_TARGET_DIR`** ⇒ 依赖 crate 只编一次；**不新开第三个构建** |
| **D5** | 触发分支收窄 | f11qa.yml 的 `[main, wip1.8, wip2.0, wip203, wip204]` → `[main]`（前提已核实：`git ls-remote --heads origin` 只回 `refs/heads/main`） |
| **R1** | **阶段 2 的设计修订** | 初版设计是「把 terminate handler 串起来」。§1.3 的路由表证明这**不够**（SEH 抢在 terminate 前、线程路径直走 SIGABRT）⇒ 改为 **pre-report hook**（所有路径共享的唯一入口）+ terminate 链式。**这是本计划对已批准方案的设计修订**，须登记。 |

---

## 三、阶段编排

### 阶段 1 · 崩溃报告一次性化 + 转储文件名不覆盖（`src/crash_handler.{h,cpp}`）

- `bool g_inCrash` → `std::atomic<bool> g_reported` + `std::mutex g_reportMutex`
- **一次性标志下沉进 `report()`**：`if (g_reported.exchange(true)) return;` —— 覆盖全部
  崩溃路径且跨线程唯一赢家；标志在**写盘之前**置位 ⇒ `MiniDumpWriteDump` 自身崩溃
  也不会递归
- `terminate_handler` 保持 `report("terminate") → abort()`；后续 `abort_handler` 的
  `report("abort")` 被挡下（仍 `_exit(3)`，退出码不变）
- `write_crash_dump()`（公开 API，M6/M7 依赖其可重复调用）**不消费**一次性标志，只取
  `g_reportMutex`，语义不变
- 文件名唯一化：新增 `open_unique_dump()`，`CREATE_NEW` 打开；`ERROR_FILE_EXISTS`
  时追加 `_2`/`_3`…（上限 100）⇒ **绝不覆盖既有转储**，同时保留文档化基名
  `fceux11_<yyyymmdd_hhmmss>_<pid>.dmp`
- `crash_dump_dir()` 增加环境变量覆盖 `FCEUX11_CRASH_DIR`（测试隔离 + 人工探针）
- `src/crash_handler.h` 同步文档

### 阶段 2 · 恢复 `fceux_diag` 背迹（pre-hook + terminate 链式 + 安装顺序）

- `crash_handler.h/.cpp`：新增 `CrashPreHook` 与 `set_crash_pre_hook()`；`report()` 在
  写盘前调用一次（由一次性标志保证只调一次）
- `src/drivers/Qt/main.cpp`：
  - `fceux_diag::install()`：注册 `write_crash_context` 为 pre-hook（写头行 + 符号化
    背迹）
  - `fceux_diag::terminate_handler()`：保留在飞异常的捕获（仅显式 terminate 路径
    有意义），**不再自己写栈**（改由 pre-hook 写）⇒ 末尾改为**链到**前一个 handler
  - `main()`：**先** `fceux11::install_crash_handler()`、**后** `fceux_diag::install()`
  - 人工探针：`FCEUX11_TEST_CRASH=1` 保持原义；新增 `FCEUX11_TEST_CRASH=terminate`

### 阶段 3 · 新门禁 `crash_once_test`（`tests/f11qa/crash_once_test.cpp`）

父进程 spawn 自己（`_popen` + `FCEUX11_CRASH_DIR`），逐模式检查崩溃目录：

- C1 `--terminate` ⇒ 恰好 **1** 个 `.dmp`（>0 字节）+ **1** 行摘要
- C2 `--abort` ⇒ 恰好 1 + 1
- C3 `--seh` ⇒ 恰好 1 + 1
- C4 `write_crash_dump()` 同秒内连调两次 ⇒ **2 个不同名** `.dmp`（都非空）+ 2 行摘要
- C5 捕获内容非零（纪律 13）
- C6 **pre-report hook 只被调用一次**（子进程注册一个写计数的 hook ⇒ 父进程数行数）
  —— 这条锁住「背迹挂载点」本身：hook 是所有崩溃路径共享的唯一入口，一次崩溃
  事件必须只写一次背迹

**子进程模式刻意全部走显式退出**（`_exit`），**不走操作系统的未处理异常路径** ——
那条路径可能弹 WER 对话框，在无交互桌面的 CI runner 上会永久阻塞（AGENTS.md 已记
两次同类事故）。SEH 腿由子进程**包装** reporter 的 filter：包装器先调用真正的
filter（它才是干活的），然后自行 `_exit(70)`。

**变异验证（纪律 3，三腿各须转红再还原）**：
- **M-A** 去掉一次性标志（`g_reported.exchange(true)` → 无）⇒ **C1 红**（terminate
  那条路径出 2 行摘要）
- **M-B** `CREATE_NEW` 改回 `CREATE_ALWAYS` ⇒ **C4 红**（同秒两次只留下 1 个文件）
- **M-C** 去掉 `report()` 里的 pre-hook 调用 ⇒ **C6 红**（hook 一次都没被调）

### 阶段 4 · CI 收敛（删 ci.yml，独有步骤移植进 f11qa.yml）

处置表见 §1.4；要点：

- **删除 `.github/workflows/ci.yml`**
- 迁入：license header check（放在 `Setup Python` 之后，早于 vcpkg 恢复）、blargg ROM
  cache、golden savestate drift check / golden layout compare / bus dispatch /
  hot-path alias / board-file 五个 PR-only 检查、ROM Regression Test、Benchmark
  (smoke)、Bench tolerance gate（**保留 `continue-on-error: true`**）
- **不迁**：Setup MSVC / CMake / Rust / vcpkg cache / prepare / announce / Configure /
  Build（f11qa.yml 全部已有；vcpkg cache key 逐字相同 ⇒ 不丢缓存；Configure 以
  f11qa 版为准，它多一个 `-DFCEUX11_ENABLE_RUST=ON`）
- **不迁**：ci.yml 的 blargg manifest 校验（f11qa 的下载步已内含等价的 python 校验，
  且失败即 `exit 1`）
- **CTest 换成严格版**（D3）
- 触发分支收窄为 `[main]`（D5）

⚠️ **连带必须改的一处（实施时才发现）**：`scripts/check_release_gates.py` 的
`CI_WORKFLOW = "CI"` 是**发布门禁的一部分** —— `release.yml` 的 `gate-ci` job 靠它
轮询「本次提交有一个绿色的 CI workflow run」。删掉 ci.yml 后**永远不会有 CI run**
⇒ 发布门禁会一路等到超时、**每次发版都被卡死**。⇒ 该脚本改为
`BUILD_TEST_WORKFLOWS = ["F11QA"]`（F11QA 已是唯一的构建+测试 workflow），并同步
`release.yml` 的 job/step 名与输入描述。**这一处若不改，CI 收敛就是半成品。**

### 阶段 5 · Rust crate 门禁进 f11qa.yml

在 job **末尾**（R4 gate 之后）加两步，`shell: bash`（GitHub 默认 `-e -o pipefail`
⇒ 失败即 job 失败）：

```
# (a) 依赖 crate（含 f11qa 的集成测试 —— 不能加 --lib）
cargo test -p gba-core -p fceux11-core -p fceux11-formats -p fceux11-media \
          -p fceux11-debug -p fceux11-utils -p fceux11-lua -p f11qa \
          -p fceux11-rom-tests
# (b) 根 crate 的 features 必须与 CMake 不同（direct-adapter 的 C++ 符号会 LNK2019
#     —— AGENTS.md 坑 2）
cargo test -p fceux11-rust --no-default-features --features gba
```

⚠️ **实施时实测并二选一 —— 实测结论：不能用 `--release`**。初版设计是「用 release
profile 与 CMake 的 cargo 产物共享同一 `CARGO_TARGET_DIR` ⇒ 依赖 crate 只编一次」。
实测（本机 `+nightly` = CI 锁定的 1.100.0，跑**完全相同的命令**）：

| profile | 结果 |
|---|---|
| `--release` | **gba-core 245 passed / 4 FAILED** |
| dev（默认） | gba-core **249 passed / 0 failed**（其余九个 crate 全绿） |

失败的 4 条是 `#[should_panic]` 断在 `debug_assert!` 边界检查上
（`crates/gba-core/src/bitwise.rs:18/25/32` 与 `cpu/arm7tdmi.rs:1933`）：
release profile 下 `debug_assertions` 关闭 ⇒ 根本不 panic ⇒ 「test did not panic as
expected」。**这四条测试在它们被写出来的那个 profile 里是对的**；改 vendor 树
（把 `debug_assert!` 换成 `assert!`）会改变发布二进制的边界检查行为 ⇒ 本批不做。

⇒ **门禁改用 dev（默认）profile**，与 AGENTS.md 门禁节的本地命令**逐字一致**；其产物
目录 `src/rust/target` **本来就在** "Cache Rust" 步骤的缓存路径里 ⇒ 首轮之后几乎免费。

**附带修一处缓存失效**：`Cache Rust` 步骤缓存的是 `src/rust/target`，而 CMake 的
cargo 构建用的是 `build/src/rust/target`（`src/rust/CMakeLists.txt:34` 把
`CARGO_TARGET_DIR` 设成 `${CMAKE_CURRENT_BINARY_DIR}/target`）⇒ **缓存从来没覆盖到
它** ⇒ 每次 CI 都在从零重编整个 Rust 依赖图（gba-core 一家 22K 行）。⇒ 把
`build/src/rust/target` 加进缓存路径。

⚠️ **登记的副作用**：(b) 以不同 features 重编根 crate ⇒ **覆写**
`build/src/rust/target/.../release/fceux11_rust.lib` 并让 build.rs 重生成
`src/rust/fceux11_rust.h`（gba-only 版）。因该步骤排在**所有 C++ 编译与链接之后**
且其后无 C++ 编译，无实际影响 ⇒ **它必须排在 job 的最后**。

### 阶段 6 · v2.0.9 版本口径与文档收口

- `src/version.h`：`FCEU_VERSION_PATCH` 8 → 9 ⇒ NUMERIC 20009；
  `FCEU_DISPLAY_VERSION` 是**硬编码串**，必须一并改 ⇒ ⚠️ **回产物查字节**确认 exe
  报 v2.0.9
- `CMakeLists.txt` 的 `project(FCEUX11 VERSION 2.0.7)` **不动**（v2.0.8 也没动；
  CMake 的 VERSION 只作信息性元数据，不参与版本口径）
- `CHANGELOG.md`：`## [2.0.9]` 段 + Known limitations
- `readme.md`：徽章 + 功能表（崩溃报告与 CI 门禁两处）
- web 12 语言页版本号 v2.0.8 → v2.0.9 + 本次更新段
- `AGENTS.md`：门禁节（Rust 门禁进 CI + `cargo test -p fceux11-core --lib` +
  `cargo test -p f11qa`（含集成测试）+ ctest 计数 **54 → 55**）、崩溃处理器契约变更、
  ci.yml 删除记录、CI 收敛记录
- **tag / GitHub Release / push 须业主另行明示**

---

## 四、门禁与验收

**实测读数（2026-11-25，全部在本机实跑）**

| 项 | 期望 | 实测 |
|---|---|---|
| `ctest -LE perf` | 55/55 全绿 | **55/55（100% passed, 0 failed, 23.95 s）** ✅ |
| `crash_once_test`（新门禁） | 16 腿全绿 | **16/16**：C1/C2/C3 各「1 个非空 dump + 1 行摘要 + hook 恰好 1 次」；C4a–d ✅ |
| Rust 十条门禁（CI 锁定版 1.100.0） | 1197 通过 / 22 忽略 / 0 失败 | **303+249+160+199+94+54+52+51+33+2 = 1197 通过 / 22 忽略 / 0 失败** ✅ |
| `cargo check --workspace` / `--no-default-features` 两态 | EXIT=0 | **EXIT=0 / EXIT=0** ✅ |
| 变异腿 M-A / M-B / M-C | 各红一次 + 还原复绿 | **M-A 红 3 腿**（C1 dump/摘要/hook）+ 还原绿；**M-B 红 1 腿**（C4b）+ 还原绿；**M-C 红 3 腿**（C1/C2/C3 hook）+ 还原绿 ✅ |
| `python scripts/license_headers/check.py` | EXIT=0 | **EXIT=0**（669 扫描 / 58 白名单）✅ |
| 产物版本 | exe 报 v2.0.9 | **v2.0.9**（崩溃摘要行 `version=v2.0.9` 实测）✅ |
| 人工探针 `FCEUX11_TEST_CRASH=1` | 1 dump + 1 摘要 | **1 dump + 1 摘要**（`source=seh` / `code=0xE0000001` / 退出码 0xC0000001）✅ |
| 人工探针 `FCEUX11_TEST_CRASH=terminate` | 1 dump + 1 摘要 + 背迹 | **1 dump + 1 摘要**（`source=terminate` / 退出码 3）+ `fceux_diag.log` 有符号化背迹（SEH 腿 15 帧、terminate 腿 9 帧）✅ |

> **M-A 的额外读数**：去掉一次性标志后终止路径留下的是 **2 个 dump 文件**（`N.dmp` 与
> `N_2.dmp`）而不是「第二个覆盖第一个」——**这正是唯一文件名逻辑在起作用**；若没有
> 它，被覆盖的正是第一份带异常上下文的转储。

**注意**：ctest 须与构建/测试任务**不并行**跑（AGENTS.md r54 ②，bench 容差会被污染）。

---

## 五、风险与止损

| 风险 | 止损 |
|---|---|
| **分支保护卡死 PR**（check 名从 `CI / build-windows` 变 `F11QA / f11qa`） | 须业主在 GitHub 设置里改名/重建 required check；**本机无 gh CLI，代劳不了** ⇒ 在 workflow 头部与 CHANGELOG 显著登记 |
| Rust 门禁引入新红项（`fceux11-core` / `fceux11-rom-tests` 首次被覆盖） | 先本地实测；红则先修或显式登记已知限制，**不带红上线** |
| `--release` LTO 链接过慢 | 实测后改 `--profile dev` + 独立 target dir |
| 共享 `CARGO_TARGET_DIR` 覆写 `.lib` / 生成头 | 步骤固定排在 job 最后；已登记副作用 |
| 崩溃处理器改动影响既有 54 项 ctest | 每步跑 ctest；SEH filter 仍返回 `EXCEPTION_CONTINUE_SEARCH`（不吞进程） |
| 删 ci.yml 后某项检查漏迁 | §1.4 的处置表逐项对照；`ci.yml` 的 22 个 step 名已逐一登记 |

---

## 六、不变式与红线

1. **NES 零回归**（不变式 1）：每阶段跑 `ctest -LE perf`。
2. **GBA 解耦**：新代码零 GBA 引用；`src/rust/crates/gba-core` 零改动（vendor 树不动）。
3. **纪律 8 分层**：`crash_handler` = core（零 Qt）；pre-hook 的实现落在驱动层
   （`main.cpp`），core 只提供一个函数指针槽。
4. **纪律 2**：出口判据与设计变更先记（§二 D1–D5 / R1）。
5. **纪律 3**：每条新锁做双向变异验证。
6. **纪律 13**：新捕获必须断言内容非零。
7. **坑 11**：改文件用编辑工具原地替换，改后核 `git diff --numstat` 与 CRLF/LF 计数。
8. **坑 12/13**：`ninja -j 1` 串行；被取消的构建要查孤儿 `ninja.exe`/`link.exe`。
9. **不改用户的既有工作树**：`M COPYRIGHT_AUDIT.md` 与未入库的审计报告原样保留。

---

## 七、变更记录

| r | 日期 | 内容 |
|---|---|---|
| **r1** | 2026-11-25 | 立项。§一 事实底座全部为实测（探针脚本在 `Research_only/audit_v2_probe/`，gitignored）；业主裁决 D1–D5 记入 §二；**R1 设计修订**（阶段 2 由「串 terminate」改为「pre-hook + 串 terminate」，依据 §1.3 的路由表）。 |
| **r2** | 2026-11-25 | 实施 + 验收完成。**实施期新发现两条**（已并入阶段 4/5）：① **发布门禁会连带失效**——`scripts/check_release_gates.py` 的 `CI_WORKFLOW="CI"` 是 `release.yml` 的 `gate-ci` job 的轮询对象，删掉 ci.yml 后每次发版都会卡到超时 ⇒ 已改为只等 F11QA；② **Rust 门禁不能用 `--release`**——gba-core 有 4 条 `#[should_panic]` 断在 `debug_assert!` 边界检查上，release profile 下 `debug_assertions` 关闭 ⇒ 恒红（实测 245/4 failed vs dev 249/0）⇒ 改用 dev profile（与本地门禁逐字一致）。另修 `Cache Rust` 的缓存失效（缓存的是 `src/rust/target`，而 CMake 的 cargo 构建用 `build/src/rust/target`）。**验收读数见 §四**（ctest 55/55、crash_once_test 16/16、Rust 1197/0 失败、两态 check EXIT=0、license EXIT=0、产物 v2.0.9、两个探针各 1 dump + 1 摘要 + 背迹、三条变异腿各红后复绿）。**提交**：`16feab2`（fix(crash)）、`1e837f5`（ci）、docs 收口提交。**未做**：tag / GitHub Release / push（须业主另行明示）。⚠️ **给业主的两项待办**：① 分支保护的 required check 需从 `CI / build-windows` 改名为 `F11QA / f11qa`（否则 PR 永久卡住）；② `M COPYRIGHT_AUDIT.md` 与未入库的审计报告**原样保留**，未混入本批提交。 |
| **r3** | 2026-11-25 | **首次 CI 运行后的整改**（流水线 `#…`，日志已解包存档于 `Research_only/ci_logs_123135242419/`）。**整条流水线只红在新加的 Rust 门禁一步**（exit 101）：ctest **55/55 通过**、R4 gate **通过**（grade B / 123 项 / 113P/10F / `pass_to_fail=0`）、blargg 的 28 个 FAIL **全在** known-fail 基线内（且 4 个基线项这轮反而通过）⇒ 其余步骤均为既有状态，无回归。**两个根因**（均已本地复现并修掉）：① **`shell: bash` 让 Git 的 `usr\bin\link.exe`（其实是 MSYS 的 `ln`）遮蔽了 MSVC 的链接器** ⇒ 每个带 build script 的 crate 链接失败（`note: /usr/bin/link: extra operand …`）⇒ 两步都改回**默认 pwsh shell**（本地按同样 PATH 顺序复现：Git 的 usr/bin 在前 ⇒ 逐字相同的错误；MSVC 在前 ⇒ `Finished test profile`）；② **`cargo test` 还会跑 doctest**，而 vendor 树 gba-core 的 doc 示例引用了 vendored 布局里不存在的 `emu::` 模块 ⇒ `error[E0433]` ⇒ 依赖 crate 那一步改用 **`--all-targets`**（单元 + 集成，不含 doctest；这组里只有 gba-core 有 doc 示例 ⇒ 无覆盖损失），根 crate 那步保留 doctest（它的是我们的，通过）。本地复核：依赖 crate **894 通过 / 0 失败 / 2 忽略**，根 crate **303 / 0 / 20 忽略** ⇒ 合计仍 1197。**新增 AGENTS.md 坑 17**（MSYS 同名工具遮蔽 Windows 工具，`link.exe` 是最贵的一个）。 |
