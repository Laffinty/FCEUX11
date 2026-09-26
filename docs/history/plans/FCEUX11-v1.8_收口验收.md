# FCEUX11 v1.8 收口验收单

> **日期**：2026-09-26　**分支**：`wip1.8`　**R4 锚**：`f19fa7d`（grade B）
> **计划归档**：`docs/history/plans/FCEUX11-v1.8_F11QA-构建计划.md`（STATUS: CLOSED）

## §10.3 验收清单

| # | 项 | 结果 | 证据 |
|---|---|---|---|
| 1 | `tests.json` schema 1.8 / suite f11qa-v1.8 / 120 cases | ✅ | 120 × kgmqa-001~120 |
| 2 | active docs F11QA 改名完成 | ✅ | tech/ + BuildGuide；历史 docs 保留原名 |
| 3 | CI workflow 改名 + wip1.8 + mirror_snapshot_check 前置 + 单一 fetch | ✅ | `f11qa.yml` |
| 4 | fetch 脚本可拉取 ✅ vendored ROM | ✅ | 43/43 SHA + blargg 177 落盘 |
| 5 | `f11qa_mirror_pin.json` mirror_ref 非空有效 | ⚠️ | `v1.8.0-mirror` 已填；**tag 尚未在镜像源发布**（回退 mirror_commit_sha） |
| 6 | kgmqa-117 CI 稳定 PASS | ✅ | preflight 每轮绿 |
| 7 | 65 条 rom-suite 四元组填齐 | ✅ | 78 条（D 节 13 + F 节 65） |
| 8 | R4 gate 3 次连续 CI PASS | 🟡 **1/3** | `f19fa7d` grade B 全绿；本收口提交为第 2 次 |
| 9 | 计划归档 → `docs/history/plans/` | ✅ | 本文同目录 |
| 10 | `docs/tech/F11QA.md` 反映 v1.8 | ✅ | 头表 + 正文 f11qa 命名 |
| 11 | v1.17 frozen baseline 保留 | ✅ | `f11qa_baseline_frozen.v1.17.json` |

## 运行时快照（R4 green）

| 通道 | 数值 |
|---|---|
| 矩阵 | 120 总 / **106 PASS / 14 FAIL** / **Grade B** |
| Oracle A | 42P / 0F |
| Oracle B (blargg) | 145P / 32F |
| advisory-FAIL | 14/120 = 11.7%（cap 15%） |
| fail_to_pass / pass_to_fail | 0 / 0 |
| vendor_state | v=43 a=13 p=22 |

## 未尽事项（不阻塞 v1.8 标签，列出供跟踪）

1. **3× CI green**：还需 1 次（本提交后的 push 可计第 2 次）
2. **mirror tag `v1.8.0-mirror`**：OWNER 在 `Laffinty/f11qa-rom-mirror` 打 tag 后更新 pin
3. **精度 backlog** 14 项 advisory：`docs/tech/f11qa-accuracy-backlog.md`（055/093 已清零）
