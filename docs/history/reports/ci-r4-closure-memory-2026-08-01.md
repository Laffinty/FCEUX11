# Memory: ci-r4-closure-2026-08-01

> **性质**：本会话无独立的 memory 工具链写入接口（`~/.grok/memory/` 不存在），
> 故按 plan §2.3 降级为 `docs/history/` 落档。等价于 memory 条目 `ci-r4-closure-2026-08-01`。
> 与既有 `docs/history/reports/FCEUX11-1.16_CI-R4-实跑诊断.md`（条目 `ci-r4-diagnosis-2026-07-31` 的等价物）
> 配合使用：后者是处方与三轮实跑，本条是结论与对接下一个 PR/任务的提示。

---

## 背景

FCEUX11 v1.16 `docs/继续任务.txt` 把 R4（CI 闭环）的处方交给了独立接管方。
本会话接手时 HEAD = `1156ca1`，R4-1 已落地但**未经 CI 验证**。

---

## 关键事实

- **CI run**：commit `1156ca1`，文件名暗示 `83107636049`（用户需在 GitHub Actions 网页核对）
- **作业耗时**：78 分钟（16:05:51Z → 17:24:06Z）
- **结果**：R4 Gate 全项绿灯
  ```
  R4 gate passed: git_rev=1156ca1, total=39, passed=35, failed=4, fail_to_pass=0 [OK]
  ```
- **Oracle A**：33/33 ctest PASS（含 `kagami_qa_direct_smoke` 6.49s，从第二轮 32/33 修复）
- **Oracle B**：121/56（真实精度口径恢复，从第二轮 0/177 全 0xFE 加载失败修复）
- **blargg fixtures**：177/177 present（CI 校验步零错）
- **runner 路径**：`src/rust/target/x86_64-pc-windows-msvc/release/kagami-qa-runner.exe`（三元组优先）

---

## R4-1 三缺口实测生效

| 缺口 | 来源（第二轮暴露） | 第三轮实测 |
|---|---|---|
| A | blargg ROM 在 CI 不存在 → Oracle B 全 `0xFE` 加载失败 | 校验步 `177/177 present`；Oracle B `121/56` |
| A-衍生 | `kagami_qa_direct_smoke` 同因失败 → ctest `32/33` | ctest `33/33`，`kagami_qa_direct_smoke` PASS 6.49s |
| B | runner 三元组路径不存在 → 矩阵步被静默 skip | 三元组路径命中 |

---

## §六.5 假设落地为结论

`actions/cache@v4` 的保存行为：

| 作业状态 | cache 是否保存 |
|---|---|
| success | ✅ |
| failure（gate 主动 `exit 1` 之类） | ❌ |
| cancelled | ❌ |

第二轮 failed → 第三轮 cache miss 直接证实。
**本轮 cache 已 saved**（`17:24:04Z`），下次推送应秒级命中。

---

## 文档锚点刷新（R2 路径 A）

| 位置 | 旧 | 新 |
|---|---|---|
| `README.md` CN | `commit 623dd39, engine.git_rev=623dd39` | `commit 1156ca1, engine.git_rev=1156ca1` |
| `README.md` EN | 同上 | 同上 |
| `docs/tech/KagamiQA.md:5` | `commit 623dd39, engine.git_rev=623dd39` | `commit 1156ca1, engine.git_rev=1156ca1` |

34 / 39 / 177 三数字仍成立。

---

## 验收报告变更摘要

- §六：卫生门槛 #5 ☐ → ☑
- §九.1 标准 #5：🟡 → ✅
- §十 完成判据：R4 / R4-1 勾掉，R4-1 "未经 CI 验证" 标注解除
- §十 末尾追加 2026-08-01 第三轮接管修订块

---

## 与既有诊断文档的关系

- `docs/history/reports/FCEUX11-1.16_CI-R4-实跑诊断.md` 顶部表格加第三行（`1156ca1` → §七）
- `docs/history/reports/FCEUX11-1.16_CI-R4-实跑诊断.md` §七：本节是新加的第三轮实测记录

---

## 给下一个 PR / 任务的提示

1. **下次推送 CI 应 hot**：cache 已 saved，配置步应秒级（§七.3）。若再次冷编，先检查 `Cache vcpkg` 步输出 + 是否有未捕获的失败吞掉了 Post 步。
2. **§五 contingency 的硬门值得考虑实施**：原计划"若预热跑仍撞 180 分钟"才启用 `if: steps.cache-vcpkg.outputs.cache-hit == 'true'` 硬门。实测下来 failed 作业也不保存 cache——故硬门应在 R4-1 后任何时候启用，作为通用防线。
3. **不要顺手动 §十 R5 / R6**：briefing 明确这两处方已失信 / 未实测，instrument-first 前置硬约束未满足。
4. **R4 闭合 ≠ v1.16 验收通过**：v1.16 早已通过（`#2/#6/#7/#9/#10` 等 10 项标准已 ✅）；R4 闭合只解决 #5 一项，11 项全闭。
5. **下次推送的 cache 命中表现**也是 cache 行为的一个独立验证。若有偏差，回头检查是否新建了分支导致 key 漂移（`vcpkg-${{ hashFiles('vcpkg.json') }}`）。

---

*Memory 等价物完。*