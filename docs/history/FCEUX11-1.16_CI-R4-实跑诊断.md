# FCEUX11 v1.16 — §十 R4「CI 实跑」第一轮诊断与整改（R4-0）

> **日期**：2026-07-31
> **触发**：用户按 `docs/FCEUX11-1.16_最终验收报告.md` §十 R4 步骤 1 推送 `wip_1.16`，`kagami-qa.yml` 自动触发
> **CI run**：`82956632293`（日志包 `logs_82956632293.zip`，含 `0_kagami-qa.txt` 2354 行 + `kagami-qa/system.txt`）
> **结论**：**该轮 CI 失败，R4 未闭合**。失败原因**不在 KagamiQA**，而在两个 workflow 共有的 vcpkg 依赖治理缺陷。
> **本文档性质**：全部结论均来自对 CI 日志原文的逐行核对，非采信。凡推断处显式标注。

---

## 一、实测事实（全部有日志行号可查）

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

## 六、本次未做（边界声明）

- **未动 `vcpkg.json`**：未把 Qt 移入 feature、未引入 `install-qt-action`
- **零代码变更**：C++ / Rust / `CMakeLists.txt` / `tests/CMakeLists.txt` 均未修改
- **未触碰 §十 R5 / R6**（E-1 PPU VBL/NMI、E-3 APU 桶 C）：二者仍是 instrument-first 前置硬约束
- **未修 `ci.yml` 缺 `-DFCEUX11_ENABLE_RUST=ON` 的问题**：与本次 R4-0 无关，超出范围
- **本次整改本身未经 CI 实跑验证**：只能在本地做 YAML 解析、grep 复核、gate 逻辑四向实测、
  以及「本地构建不受影响」的回归。**整改是否真能让 CI 跑通，须待用户推送后的下一轮 CI 判定** ——
  在那之前，本文档第三节的修法应被理解为**有依据的处方，而非已验证的结论**。
