# FCEUX11 v1.16 — §十 R4「CI 实跑」诊断与整改（R4-0 / R4-1）

> **日期**：2026-07-31
> **触发**：用户按 `docs/FCEUX11-1.16_最终验收报告.md` §十 R4 步骤 1 推送 `wip_1.16`，`kagami-qa.yml` 自动触发
> **本文档性质**：全部结论均来自对 CI 日志原文的逐条核对，非采信。凡推断处显式标注。
>
> | 轮次 | CI run | commit | 结果 | 章节 |
> |---|---|---|---|---|
> | 第一轮 | `82956632293` | `10f1e05` | 配置步 45 min timeout 被取消，五步全 skip | §一~§五（整改 = **R4-0**） |
> | 第二轮 | `83046118885` | `efaa363` | **R4-0 全项生效**（配置步 48.4 min 成功、1151/1151 链接完成）；暴露 blargg ROM 缺失 + runner 路径两个新缺口 | §六（整改 = **R4-1**） |
> | 第三轮 | `83107636049`（推断，见 §7 注） | `1156ca1` | **R4 闭合、R4-1 全项实测生效**（配置步 47.88 min 冷编成功、177/177 ROM、Oracle A 33/33、Oracle B 121/56、R4 Gate `[OK]`） | §七（**R4 闭环**） |
>
> **R4 已由第三轮 CI（commit `1156ca1`）闭环**。`actions/cache@v4` 在 failed 作业下不保存 cache 已实测确认
> （第二轮 failed → 第三轮 cache miss 直接证实），cache 仅在成功作业下保存，故正确节奏是
> **三轮 CI：cold (45 min timeout) → cold + R4-0 (75 min, failed) → cold + R4-1 (78 min, succeeded) → 后续热跑 (~15 min)**。
> 三处方至此均升格为「实测结论」。

---

## 一、第一轮实测事实（全部有日志行号可查）

| # | 事实 | 证据 |
|---|------|------|
| 1 | 触发正常，跑的是正确 commit | `git rev-parse refs/remotes/origin/wip_1.16` → `10f1e05f431ec9314e8b7f5d3fa0e846f6508ae7`（`0_kagami-qa.txt:120`），即报告 §一.2 之后的 HEAD |
| 2 | runner 与配置符合预期 | `Requested labels: windows-2022`；`Job defined at: Laffinty/FCEUX11/.github/workflows/kagami-qa.yml@refs/heads/wip_1.16`（`system.txt`） |
| 3 | 前置步骤全部成功 | checkout / msvc-dev-cmd / get-cmake / **dtolnay/rust-toolchain@stable** 均正常（D-3 的 Rust 工具链步骤在 CI 上确实生效） |
| 4 | **死在 `Configure CMake` 步** | 该步 `00:04:09` 开始（`:1213`），`00:48:39` `##[error]The operation was canceled.`（`:2194`）。作业 `00:03:33` 起跑，撞 `timeout-minutes: 45` |
| 5 | 卡点是从源码编 Qt 6.8.0 | 取消前最后输出：`Installing 30/33 qtbase[brotli,concurrent,...]:x64-windows@6.8.0` → `-- Configuring x64-windows` → `-- Building x64-windows-dbg`（`:2170-2193`）。**debug 与 release 两份都编** |
| 6 | vcpkg 缓存从未存在过 | `Cache not found for input keys: vcpkg-13cac6ff...c5476, vcpkg-`（`:1144`）。连宽松 restore-key `vcpkg-` 都未命中 → 该 key 前缀下历史上**从未保存过任何条目** |
| 7 | 缓存路径缺陷 1 | 缓存步实测入参 `path: vcpkg_installed`（仓库根）。但 manifest 模式实际装到 **`build/vcpkg_installed`** —— 日志 `D:/a/FCEUX11/FCEUX11/build/vcpkg_installed/vcpkg/blds/dbus/src/...`（`:2154`）。缓存的是一个空目录 |
| 8 | 缓存路径缺陷 2 | 第二条路径实测渲染为字面量 **`\vcpkg\archives`**（`cat -A` 确认前缀无内容）。`${{ env.LOCALAPPDATA }}` 在 workflow 级 `env` 上下文中**展开为空串**（该上下文不含 runner 环境变量）→ vcpkg 真正的二进制缓存目录从未被缓存 |
| 9 | 后续四步全部未执行 | Build C++ / kagami-qa-runner / Oracle A ctest / Oracle B / Migration Matrix 均被 skip |
| 10 | matrix 未产出，且几乎无声 | `##[warning]No files were found with the provided path: build/kagamiqa_migration_matrix.json`；`Print Summary` 因 `Test-Path` 为假而**一行数字都没打印**，只输出了 `KagamiQA P5 CI Run Complete` 标题 |
| 11 | `ci.yml` 有同一缺陷 | `ci.yml:36-44`（整改前）缓存块与 `kagami-qa.yml` 逐字相同；差别仅在 `ci.yml` 未设 `timeout-minutes`（默认 360），故它是硬扛冷编而非被掐 |

---

## 二、根因

**`kagami-qa.yml` 在冷 runner 上从设计上不可能在 45 分钟内完成。**

因果链：

```
缓存 path 写错（两处，事实 7/8）
      ↓
GitHub Actions 缓存永远为空（事实 6）
      ↓
每轮 CI 都走 vcpkg manifest 模式，从源码编全部 33 个 port
（含 Qt 6.8.0 qtbase + qttools，且 debug/release 各一份，事实 5）
      ↓
配置步耗时远超 45 分钟 → 撞 timeout 被取消（事实 4）
      ↓
build / ctest / Oracle B / matrix 全 skip（事实 9）
      ↓
R4 的三项证伪判据（git_rev / passed=35 / fail_to_pass=0）无从核验
```

**这不是 KagamiQA 的缺陷**：Oracle A/B、判定链路、迁移矩阵、S-4 编译期 stamp 在本轮 CI 中根本没有获得执行机会。验收报告 §九.2 的通过判定不受本次影响。

### 一处顺带发现的潜伏脆弱

`tests/CMakeLists.txt:431` 的 Windows 测试运行时 PATH 注入硬编码的是**仓库根**路径
`${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows/bin`。而整改前 CI 的 manifest 模式装在 `build/` 下，
**这条注入在 CI 上一直指向一个不存在的目录**。28 个 ctest 之所以没有集体 `STATUS_DLL_NOT_FOUND`
（`tests/CMakeLists.txt:420-428` 注释所警告的 `0xc0000135`），是因为 vcpkg toolchain 的 applocal
部署把 DLL 拷到了每个 exe 旁边 —— 一条与 PATH 注入互相独立、且在「跳过 toolchain」路径下不存在的兜底。
下方整改的 `VCPKG_INSTALLED_DIR` 一并修掉了这一处。

---

## 三、整改（R4-0，本次落地）

新增 `cmake/triplets/x64-windows.cmake`，修改 `.github/workflows/{kagami-qa,ci}.yml`。**零代码变更**
（C++ / Rust / CMakeLists.txt / tests/CMakeLists.txt 均未触碰）。

| # | 改动 | 作用 |
|---|------|------|
| 1 | 配置步加 `-DVCPKG_INSTALLED_DIR=${{ github.workspace }}/vcpkg_installed` | **承重件，一箭三雕**：(a) 冷跑装到缓存步骤本来就打算缓存的仓库根路径；(b) 下轮命中缓存后 `CMakeLists.txt:7` 的 prefer-local 分支（`EXISTS .../vcpkg_installed/x64-windows`）直接命中，**完全跳过 vcpkg/manifest**；(c) 修掉上述 `tests/CMakeLists.txt:431` 的潜伏脆弱 |
| 2 | 新增 overlay triplet `cmake/triplets/x64-windows.cmake`（`VCPKG_BUILD_TYPE release`），配置步加 `-DVCPKG_OVERLAY_TRIPLETS=...` | 只编 release。本地实测 `vcpkg_installed/x64-windows` 共 **2.4 GB**，其中 `debug/` 占 **1.2 GB** → 冷编时间与缓存体积均对半砍。**故意沿用 `x64-windows` 这个名字**，因为安装目录名必须保持不变才能被第 1 项的 (b)(c) 命中 |
| 3 | 缓存 `path` 收窄为 `vcpkg_installed/x64-windows` + `vcpkg/{status,info}` + `vcpkg_bincache` | 删掉展开为空的 `${{ env.LOCALAPPDATA }}` 那条；**刻意排除** `vcpkg_installed/vcpkg/{blds,pkgs}`（buildtrees 数 GB）。`status`/`info` 约 500 KB，仅作 prefer-local 万一不触发时的保险 |
| 4 | job 级 `env: VCPKG_DEFAULT_BINARY_CACHE: ${{ github.workspace }}\vcpkg_bincache` + mkdir 步 | 给 vcpkg 二进制缓存一个确定性的、位于工作区内的落点 |
| 5 | `kagami-qa.yml` `timeout-minutes: 45` → `180` | 仅为容纳**一轮**冷预热跑。这是上限不是预留；预热后稳态约 15 分钟 |
| 6 | 缓存未命中时发 `::warning::` 冷跑告警 | 让「本轮要冷编 60+ 分钟」在日志顶部可见，而不是事后从 timeout 反推 |
| 7 | **`kagami-qa.yml` 新增 R4 gate 步** | 见下节 |
| 8 | `.gitignore` 加 `!cmake/triplets/*.cmake` 例外 | **必要修复**：`.gitignore:21` 的 `*.cmake`（本意是忽略 CMake 生成物）会把手写的 overlay triplet 一起排除，导致它进不了 commit、CI 上 `cmake/triplets/` 目录不存在、`-DVCPKG_OVERLAY_TRIPLETS` 指向空目录报错。**若不修，下一轮 CI 必然以一个全新的理由失败** |

### R4 gate —— 把证伪判据机器化

报告 §十 R4 的证伪判据此前只存在于散文里，而 workflow 中所有实质步骤都带 `continue-on-error: true`，
`Print Summary` 在 matrix 缺失时**一个字都不打印**。本轮 run 就是活证据：唯一的痕迹是一条被淹没的
`##[warning]No files were found`。这正是报告 §九.2 第 3 条所称「比构建失败更危险的『跑出来了但结论是错的』」型缺陷。

新增的 `R4 Gate` 步（`if: always()`，**置于产物上传之后**，使 gate 失败时产物仍可下载用于诊断）在以下任一条件成立时 `::error::` 并 `exit 1`：

- `build/kagamiqa_migration_matrix.json` 不存在
- `engine.git_rev` 为空或等于 `"unknown"`（S-4 编译期 stamp 在 CI 未生效）
- `summary.total < 39`（用例被静默跳过；用 `<` 而非 `!=`，避免将来合法新增测试时 gate 因错误理由报红）
- `transition_matrix.fail_to_pass != 0`（Phase 0.5-d 反 gaming 加固在 CI 失效）

**gate 逻辑已在本地用真实数据四向实测**（非纸面推导）：

| 用例 | 输入 | 结果 |
|------|------|------|
| good | 真实 S-4 矩阵（`git_rev=623dd39`, 39/35/4, `fail_to_pass=[]`） | `exit 0`，打印 `R4 gate passed: git_rev=623dd39, total=39, passed=35, failed=4, fail_to_pass=0 [OK]` |
| stale | 仓库内 `build/kagamiqa_migration_matrix.json`（2026-07-26 的 P1 期废快照，`runner: kagami-qa-p1`，30/30/0，无 `engine`/`transition_matrix`） | `exit 1`，**同时**报出 `git_rev is ''` 与 `total is 30, expected >= 39` |
| missing | 指向不存在的路径 | `exit 1`，报「未产出，去查前面的步骤」 |
| gamed | 在真实矩阵上人工注入 `fail_to_pass` 两条 | `exit 1`，报反 gaming 判据失守 |

> stale 用例的价值：那份废快照正是 S-4 立项要解决的「废快照与新产物不可区分」问题的实物样本，
> 而新 gate 能一眼把它判死。

> **实现注记**：gate 脚本刻意写成**纯 ASCII**。本地实测发现，脚本内的非 ASCII 字符（`✓`、中文）
> 在无 BOM 的 `.ps1` 被 Windows PowerShell 5.1 读取时会破坏字符串字面量导致 parse error。
> GitHub runner 用的是 pwsh 7（UTF-8 无 BOM 正常），但纯 ASCII 让这段逻辑**可在本地被实测验证**，
> 这比省几个符号重要。中文说明保留在 YAML 注释里（注释不被执行）。

---

## 四、下一轮 CI 的预期与验收判据

> **⚠️ 本节写于第一轮之后，其中「第一轮/第二轮」的编号预期已被 §六 的实测取代。**
> 第 1 条（预热跑 60-90 分钟）已由 run `83046118885` 实测证实（48.4 分钟配置步）；
> 第 2 条（缓存命中后 ~15 分钟）**仍待验证**。第 3-4 条的闭合判据保持有效。

1. **第一轮（预热跑）**：缓存未命中，配置步冷编 Qt release 半边，预计 **60-90 分钟**；日志顶部应出现
   `::warning::vcpkg cache miss`。此轮跑完后缓存条目 `vcpkg-<hash(vcpkg.json)>` 应被保存（约 1.2 GB + 二进制缓存）。
2. **第二轮**：缓存命中 → `CMakeLists.txt:7` prefer-local 分支触发 → 配置步秒级完成 → 全流程预计 **~15 分钟**，
   产出 `kagamiqa-results` artifact。
3. **R4 闭合判据**（gate 自动核验 + 人工复核）：
   - `engine.git_rev` = 触发该轮 CI 的真实 commit 短哈希（非 `unknown`）
   - `summary` = `{total:39, passed:35, failed:4}`
   - `transition_matrix.fail_to_pass` = 0
   - 4 项 FAIL 仍为 `blargg_ppu_vbl_nmi` / `blargg_suite` / `lua_joypad_test` / `lua_memory_test`
   - artifact 上传成功且可被后续 run 作为基线对比
4. 判据全绿后：验收报告 §九.1 标准 #5 由 🟡 转 ✅，§六 卫生门槛第 5 条由 ☐ 转 ☑，
   并按 §十 R2 **路径 A** 用该轮 CI 的真实 `git_rev` 统一刷新 `README.md` 中英两处与 `docs/tech/KagamiQA.md:5` 三处快照锚。

---

## 五、Contingency（若第一轮预热跑仍撞 180 分钟）

已知风险：**`actions/cache@v4` 在作业被取消（timeout 即取消）时不保存缓存**，故一旦预热跑再次超时，
本轮进度**零留存**，下一轮仍是冷跑。

当前 180 分钟的余量估算：整改前实测 44 分钟走到 30/33 个 port，且本次砍掉了 debug 半边 →
约 2 倍余量。判断为够用，因此**没有**预支更复杂的方案。

若判断失误，按以下顺序升级（未实施，仅记录）：

1. 新增 `workflow_dispatch` 专用预热 workflow（`timeout-minutes: 360`），主 workflow 加
   `if: steps.cache-vcpkg.outputs.cache-hit == 'true'` 硬门 —— 缓存缺失时**快速失败并明确报错**，
   而不是慢慢跑到超时再被误读成「CI 坏了」。
2. 改用预编译 Qt（`jurplel/install-qt-action`），把 `qtbase`/`qttools` 从 `vcpkg.json` 顶层依赖移进
   一个非默认 feature。冷启动可降到 ~15 分钟，代价是改动 `vcpkg.json` 语义、影响本地冷 bootstrap 流程。

---

## 六、第二轮实测（run `83046118885`，commit `efaa363`）—— R4-0 生效，暴露两个新缺口

> 本节全部结论来自对 `logs_83046118885.zip` 分步日志的逐条核对。
> 作业 `10:48:41` → `12:03:32`，约 **75 分钟**，最终**红灯**（`R4 Gate` 主动 `exit 1`）。

### 6.1 R4-0 的每一项都实测生效

| R4-0 改动 | 第一轮（`82956632293`） | 第二轮（`83046118885`） |
|---|---|---|
| 配置步 | 45 min 撞 timeout 被取消 | ✅ **成功**：`-- Configuring done (2905.4s)` = **48.4 分钟** |
| vcpkg 安装位置 | `build/vcpkg_installed`（缓存不到） | ✅ `Found LibArchive: D:/a/FCEUX11/FCEUX11/vcpkg_installed/x64-windows/lib/archive.lib`、`Found ZLIB: .../vcpkg_installed/x64-windows/lib/z.lib` —— **仓库根** |
| 测试 DLL PATH 注入 | 指向不存在目录（靠 applocal 兜底） | ✅ `FCEUX11 tests: PATH prepended for vcpkg runtime — D:\a\FCEUX11\FCEUX11\vcpkg_installed\x64-windows\bin` |
| Build C++ | 从未执行 | ✅ **`[1151/1151]` 全部链接**，含 `fceux11_blargg_runner.exe` |
| kagami-qa-runner | 从未执行 | ✅ `Finished release profile in 17.01s` |
| 冷跑告警 | 无此机制 | ✅ `::warning::vcpkg cache miss - ...` 如期出现于日志顶部 |
| 失败可见性 | 一条被淹没的 `##[warning]No files were found` | ✅ **`R4 Gate` 红灯并指名道姓**：`R4 gate: build/kagamiqa_migration_matrix.json was not produced. ... inspect the earlier steps` |

> **48.4 分钟这个数字直接证明了 timeout 抬升的必要性与充分性**：旧上限 45 分钟差约 3.4 分钟，
> 而 180 分钟留有约 3.7 倍余量。

### 6.2 缺口 A —— blargg ROM fixtures 在 CI 上根本不存在

**实测**：

- Oracle B：`Total: 177 / Passed: 0 / Failed: 177`，逐条为
  `{"value":"0xFE","diag":[229,246,127],"status":"FAIL","duration_ms":0}` ——
  `0xFE` + `duration_ms: 0` 是**加载不到 ROM** 的签名，不是精度失败
- Oracle A：`kagami_qa_direct_smoke` 红，
  `kagami_bridge_load_rom('fixtures/blargg/ppu/ppu_vbl_nmi.nes') failed: rc=-2`（4 项中 3 项 LOAD_ERROR）
  → CI 上 ctest 为 **32/33**，而同一 commit 本地是 34/34

**根因**：`.gitignore:108` 的 `*.nes` 把全部 ROM 排除出仓库（实测：本地磁盘 **177** 个 `.nes`，
`git ls-files tests/fixtures/blargg` 返回 **0**）。这本身是**有意设计**——ROM 从镜像拉取而非入库，
项目已备有 `scripts/download_blargg_roms.ps1`——**但两个 workflow 从未调用过它**。
即 CI 历史上一直在对着空的 fixture 树跑 Oracle B。

**修复（R4-1）**：新增三步——`Cache blargg ROMs`（key 跟随下载脚本哈希，因 ROM 清单声明在脚本里）、
`Fetch blargg test ROMs`、`Verify blargg ROM fixtures against manifest`。

第三步是必要的：下载脚本汇总失败数但**从不设非零退出码**，部分下载会被静默当成完整的 177 ROM 跑完。
校验步改为对着 `tests/fixtures/blargg_manifest.json` 的 177 条逐一核对磁盘存在性，缺一即 `::error::` + `exit 1`。

### 6.3 缺口 B —— runner 路径缺少 target 三元组

**实测**：矩阵步输出 `::warning::kagami-qa-runner not built; skipping migration matrix.`，
而紧邻的上一步 cargo 明确 `Finished release profile [optimized] target(s) in 17.01s`。

**根因**：`src/rust/.cargo/config.toml` 设 `build.target = "x86_64-pc-windows-msvc"`，
cargo 产物落在 `target/x86_64-pc-windows-msvc/release/`；而 workflow 检查的是
`src/rust/target/release/kagami-qa-runner.exe`。

**为何本地看不见**：本仓库 `target/release/` 下有一份 **2026-07-28、424,960 字节的陈旧副本**，
与三元组路径下 **2026-07-30、562,688 字节的真产物**并存。旧路径在开发机上恰好"能用"，
干净检出的 CI 上必然不存在。

> **顺带的严重隐患**：`docs/BuildGuide.md:363` 此前也写的是 `target/release/...`。
> 任何按该文档跑 runner 的人，用的都可能是过期二进制，产出的矩阵无法反映当前代码——
> 这正是 S-4 编译期 `git_rev` stamp 立项要防的问题的另一个入口。已随 R4-1 更正
> （`docs/tech/KagamiQA.md:28` 一直是对的）。

**修复（R4-1）**：按 `[三元组路径, 平路径]` 顺序解析，取第一个存在者；
**两者皆无时由 `::warning::` 升级为 `::error::` + `exit 1`** —— 静默跳过矩阵生成，
正是让本轮看起来像"基础设施抽风"而非真实缺口的原因。

### 6.4 R4-1 的本地实测（5 个用例，非纸面推导）

| # | 用例 | 结果 |
|---|------|------|
| A | ROM 校验：本地完整 177 个 | `blargg fixtures: 177 / 177 present`，exit 0 |
| B | ROM 校验：临时移走 3 个 | `174 / 177`，逐条 `::error::missing ROM: ...`，exit 1 |
| C | runner 解析：两份都在 | 选中**三元组**路径（即避开了陈旧副本） |
| D | runner 解析：模拟干净 CI（只有三元组） | 正确选中，exit 0 |
| E | runner 解析：两者皆无 | `::error::` 列出两个查找路径，exit 1 |

### 6.5 缓存是否已保存（未证实）

日志包中未包含 Post 步骤输出（分步文件从 `19_` 跳到 `38_Post Checkout`），
**无法从本轮日志证实 vcpkg 缓存条目已保存**。理论依据：`actions/cache@v4` 在作业
**failed** 时会执行保存（只有 **cancelled** 才跳过），而本轮是 `R4 Gate` 主动 `exit 1` 导致的 failed，
故大概率已存。**下一轮的 `Cache vcpkg` 步会直接给出答案**：若命中，配置步应从 48.4 分钟降至秒级。
在那之前，这条只是推断，不作为结论。

---

## 七、第三轮实测（run `83107636049`，commit `1156ca1`）—— R4 闭环

> 本节全部结论来自对 `logs_83107636049.zip` 日志原文的逐条核对（解压后仅余 `0_kagami-qa.txt` 单文件
> 7,598 行，分步文件未保留）。
>
> 作业 `2026-07-31T16:05:51Z` → `17:24:06Z`，约 **78 分钟**，最终**绿灯**
> （R4 Gate 输出 `R4 gate passed: ... [OK]`，未 `exit 1`）。
>
> > **§7 注 — Run ID 来源**：日志包文件名 `logs_83107636049.zip` 暗示 run id `83107636049`，
> > 但 `system.txt` 仅含 `Requested labels: windows-2022` + Job defined at，未保留 run id 行；
> > 实际 run id 由用户在 GitHub Actions 网页核对。本节凡引用 run id 处均按文件名推断，
> > 如有出入以网页为准。

### 7.1 R4 Gate 全项实测生效

| R4 闭合判据（前 4 条由 gate 自动核） | 本轮实测 | 判定 |
|---|---|---|
| matrix 产出 | `Report written to: build/kagamiqa_migration_matrix.json`（`17:23:16`） | ✅ |
| `engine.git_rev` 非空非 `unknown` | `git_rev=1156ca1`；与 `git rev-parse HEAD` 短哈希完全一致；`build/src/fceux_git_info.cpp` 内 `#define FCEUX_GIT_REV "1156ca1cf6a729af10265a1aec10717af7bcff8b"`（`17:03:08`，由配置步 echo 写入） | ✅ S-4 编译期 stamp 在 CI 生效 |
| `summary` = 39/35/4 | Print Summary 步输出 `Total: 39 / Passed: 35 / Failed: 4`（`17:23:17`）；`Oracle A: 25P / 2F \| Oracle B: 10P / 2F` → 27 + 12 = 39，与本地一致 | ✅ |
| `transition_matrix.fail_to_pass` = 0 | R4 Gate 输出 `fail_to_pass=0 [OK]` | ✅ Phase 0.5-d 反 gaming 加固在 CI 生效 |
| 4 FAIL 项性质 | per-oracle 拆分与 §十 R4 预期一一对应：`lua_joypad_test` / `lua_memory_test` 在 Oracle A（25P/2F）；`blargg_ppu_vbl_nmi` / `blargg_suite` 聚合在 Oracle B（10P/2F）。`test_id` 字符串未 echo 到 stdout 但 2+2=4 与预期吻合 | ✅ |
| `kagamiqa-results` artifact 上传 | 上传步 `if: always()` 且未与 R4 Gate 共用 exit code；矩阵已落地且 R4 Gate `[OK]` 未报 `##[warning]No files` → 高置信已上传 | ✅（高置信，非直接证据） |

### 7.2 R4-1 全项实测生效（每一项都针对第二轮暴露的缺口验证）

| 第二轮缺口 | 本轮实测 |
|---|---|
| **缺口 A**：blargg ROM 在 CI 不存在 → Oracle B 全 0xFE 加载失败 | `blargg fixtures: 177 / 177 present`（`17:20:35`，校验步零错零警告）；Oracle B `Total: 177 / Passed: 121 / Failed: 56`（`17:22:52`），每条 FAIL 带真实 `$6000` 码（`0x01/0x02/0x03/0x06/0x80/0x81/0x09/0xFE` 等）与 diag_string |
| **缺口 A 衍生**：`kagami_qa_direct_smoke` 同因失败 → CI ctest 32/33 | ctest `100% tests passed, 0 tests failed out of 33`（`17:20:56`）；`33/33 Test #34: kagami_qa_direct_smoke ........... Passed 6.49 sec` → 从 32/33 回到 33/33 |
| **缺口 B**：runner 三元组路径不存在 → 矩阵步被静默 skip | `Using runner: src/rust/target/x86_64-pc-windows-msvc/release/kagami-qa-runner.exe`（`17:22:52`，三元组路径优先命中，避开了 `target/release/` 下的陈旧副本） |

### 7.3 §六.5 的"缓存是否已保存"——结论落地

第二轮日志包**不含 Post 步骤输出**，无法证实 cache 是否已保存（§六.5 标注"未证实"）。
本轮日志**包含完整 Post**，直接给出答案：

```
16:06:19Z  Cache not found for input keys: vcpkg-13cac6ff...c5476, vcpkg-
16:54:13Z  -- Configuring done (2872.8s)               <- 配置步冷编 47.88 min
17:24:04Z  Cache saved with key: vcpkg-13cac6ff...c5476  <- cache saved
```

**§六.5 的"理论依据（failed 时 save，cancelled 时 skip）"由本轮实测推翻并改写**：
`actions/cache@v4` 在 **failed** 作业下**也**不保存 cache（run `83046118885` 是 R4 Gate 主动 `exit 1` 的 failed，
本轮 `Cache not found` 直接证实）。cache 仅在**成功**作业下保存。

故正确的「CI 节奏」是：
- 第二轮（failed，gate 主动 exit 1）→ cache 未保存 → 进度零留存
- 第三轮（cold，再次冷编 47.88 min，但 R4-1 已落地，**成功**）→ cache 保存 → 第四轮应 hot

**这不是工程错误**：预算上多一轮冷跑是已知代价，第三轮成功后即可稳定热跑。
**对 §四 contingency 评估的修正**：§五 contingency 提到的
`if: steps.cache-vcpkg.outputs.cache-hit == 'true'` 硬门，**实测下来更应该在第三轮之前就介入**——
不是"预热跑仍失败"时才用，而是**任何上一轮 failed 时都不应假设 cache 存在**。

### 7.4 顺带记录的小事实

- **总耗时 78 分钟**（16:05:51 → 17:24:06），与第二轮 75 分钟几乎一致——两次都是冷跑。
  下一次（第四轮）应降至 ~15 分钟
- **`cpu_interrupts.nes`** 在 Oracle B 输出 `value: 0xFE, diag: [222,176,97], duration_ms: 0`，
  与第二轮的 `0xFE + diag: [229,246,127]` **diag 不同** → 这是 **runner 超时签名**（测试 hang 后被 kill），
  不是 ROM 加载不到。其余 55 FAIL 都带真实 diag_string，不影响总数（121/56）
- **作业末尾 `##[warning]Node.js 20 is deprecated`**——纯运维提示，与产物无关
- **顶部 `##[warning]vcpkg cache miss`** 告警如期出现于日志顶部（`16:06:20`），证明 R4-0 的可观测性改造落地
- **`.gitignore` 例外生效**：`cmake/triplets/x64-windows.cmake` 被成功 checkout 并参与构建（无 fallback 报错），
  证明 R4-0 的 `.gitignore` 修复落地

### 7.5 R4 / R4-1 的诚实边界解除

| 处方 | 验证轮次 | 当前状态 |
|---|---|---|
| R4-0（workflow vcpkg 依赖治理） | 第二轮（`83046118885`）§六.1 | **已升格为实测结论** |
| R4-1（blargg ROM + runner 三元组路径） | **第三轮**（`1156ca1`）§7.2 | **已升格为实测结论** |
| R4（CI 产 matrix + git_rev + 35/4 + R4 Gate 绿灯） | **第三轮**（`1156ca1`）§7.1 | **已升格为实测结论** |

至此 §六.5 / §七 的"未经 CI 验证"标注全部解除。
## 八、本次未做（边界声明，§一~§七三轮合并视角）

> **🚧 2026-08-01 接管修订**：原 §七 编号迁至 §八，让位给第三轮实测记录（现 §七）。
> 下述条目**仍全部成立**——它们说的是 §一~§六 的事实边界，与第三轮无关。

- **未动 `vcpkg.json`**：未把 Qt 移入 feature、未引入 `install-qt-action`
- **零代码变更**：C++ / Rust / `CMakeLists.txt` / `tests/CMakeLists.txt` 均未修改
- **未触碰 §十 R5 / R6**（E-1 PPU VBL/NMI、E-3 APU 桶 C）：二者仍是 instrument-first 前置硬约束
- **未修 `ci.yml` 缺 `-DFCEUX11_ENABLE_RUST=ON` 的问题**：与本次 R4-0 无关，超出范围
- **R4-0 已由第二轮 CI 实测验证**（§六.1），不再是处方；**R4-1（§六.2/6.3）尚未经 CI 验证**（已于 2026-08-01 由第三轮实测解除，见 §七.2），
  只做了本地 YAML 解析、5 个用例的逻辑实测（§六.4）。R4-1 是否真能让矩阵产出，已由第三轮实测确认（§七.2）
- **未证实缓存是否已保存**（§六.5）—— **已由第三轮实测推翻为结论**：cache 在 failed 作业下也不保存（§七.3）
- **未处理 Oracle A 在 CI 与本地的口径差**：ROM 补齐后 CI 的 ctest 预期回到 33/33（`-LE perf`），
  **已由第三轮实测确认**：33/33，`kagami_qa_direct_smoke` 6.49s PASS（§七.2）

---

