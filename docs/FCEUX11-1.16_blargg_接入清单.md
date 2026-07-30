# blargg ROM 接入清单（Stage-2 §八 D-1）

> **生成日期**：2026-07-30（**S-1 修订同日**）
> **目的**：把"22/174"、"~80%"等口径不一的旧叙述统一到一个可复算的基线
> **基线**：`tests/fixtures/blargg_manifest.json` + `tests/fixtures/blargg/` 实际文件树

> ## ⚠️ S-1 修订（commit `bc7c1d8`）：本文档原「3 个 ROM 缺失」的结论已被推翻
>
> 原文判定 `all_instrs.nes` / `cpu_timing_test.nes` / `official_only.nes` 三个 ROM
> **不在仓库、需重新下载**。实测证明这是**命名错配的误判**：三个 ROM 早已在磁盘上，
> 用的是 `scripts/download_blargg_roms.ps1` 的命名规则，而且 manifest 里**已有对应的活条目**：
>
> | 原「死」路径 | 磁盘实际文件（manifest 已有活条目） | 上游来源 |
> |---|---|---|
> | `blargg/cpu/all_instrs.nes` | `blargg/cpu/instr_v5_all.nes` | `instr_test-v5/all_instrs.nes` |
> | `blargg/cpu/official_only.nes` | `blargg/cpu/instr_v5_official.nes` | `instr_test-v5/official_only.nes` |
> | `blargg/cpu/cpu_timing_test.nes` | `blargg/cpu/cpu_timing_test6.nes` | blargg `cpu_timing_test6` |
>
> 因此那 3 条是**指向不存在路径的纯重复条目**，已删除：manifest **180 → 177**，与磁盘完全一致，
> **死条目归零**、无需联网下载。原文「它们的缺席会显著压低覆盖率与权威性上限」的论断作废 ——
> 这三个核心权威 ROM 一直都在被 `blargg_suite` 跑。
>
> **两条独立条目压红的真因是帧预算，不是缺 ROM**（实测 `$6000`）：
>
> | ROM | frames=300 | frames=3000 |
> |---|---|---|
> | `cpu_timing_test6` | `0x00` **PASS** | — |
> | `instr_v5_official` | `0x80` "Running test 4 of 16" | `0x00` **PASS** (4.8s) |
> | `instr_v5_all` | `0x80` 仍在运行 | `0x00` **PASS** (4.8s) |
>
> `blargg_cpu_instrs` / `blargg_cpu_timing` 已据此转回 `blocking`，manifest 中两个 `instr_v5_*`
> 条目的 `frames` 提到 3000。这同时解释了 E-2 记录的「`official_only` 停在 4/16」。
>
> 以下正文保留原始记录（现象描述仍有价值），但**结论部分以本框为准**。

## 总览

| 口径 | 数量 | 来源 |
|---|---|---|
| 磁盘上的 ROM | **177** | `find tests/fixtures/blargg -name '*.nes' \| wc -l` |
| `blargg_manifest.json` 条目数 | **177**（S-1 前 180） | JSON 数组长度 |
| manifest 中引用不存在的文件 | **0**（S-1 前 3） | 见上方修订框 |
| tests.json 的 blargg 顶层条目 | **5** | `blargg_smoke` / `blargg_cpu_instrs` / `blargg_cpu_timing` / `blargg_ppu_vbl_nmi` / `blargg_suite` |
| `blargg_suite` 通过 `--manifest` 驱动的 ROM | **177** | 与磁盘 1:1 |

> **覆盖率分母 = 177**（磁盘上真实存在的 blargg ROM 数），分子亦为 177 —— manifest 与磁盘已完全对齐。

## 分类细分（S-1 前的原始快照）

| 类别 | manifest 条目 | 磁盘实际 | 缺口（死条目） |
|---|---|---|---|
| `apu` | 52 | 52 | 0 |
| `cpu` | 61 → **58** | 58 | **3 → 0** |
| `mmc3` | 18 | 18 | 0 |
| `ppu` | 49 | 49 | 0 |
| **合计** | **180 → 177** | **177** | **3 → 0** |

## 三个死条目（已于 `bc7c1d8` 删除）

| 名称 | manifest 声明路径 | 状态 |
|---|---|---|
| `all_instrs` | `fixtures/blargg/cpu/all_instrs.nes` | ⊘ 重复条目，已删（活条目 = `instr_v5_all`） |
| `cpu_timing_test` | `fixtures/blargg/cpu/cpu_timing_test.nes` | ⊘ 重复条目，已删（活条目 = `cpu_timing_test6`） |
| `official_only` | `fixtures/blargg/cpu/official_only.nes` | ⊘ 重复条目，已删（活条目 = `instr_v5_official`） |

这三个曾被 `tests.json` 中的 `blargg_cpu_instrs` / `blargg_cpu_timing` 两个**独立条目**（非 `blargg_suite`）直接引用而判 FAIL；两条已改指实际文件：
- `tests.json:blargg_cpu_instrs.input.args = ["--rom", "fixtures/blargg/cpu/instr_v5_all.nes", "--frames", "3000"]`
- `tests.json:blargg_cpu_timing.input.args = ["--rom", "fixtures/blargg/cpu/cpu_timing_test6.nes", "--frames", "500"]`


## 覆盖率口径

旧叙述中的几个数字，对应本表的解读：

| 旧叙述 | 真实口径 |
|---|---|
| "Oracle B 覆盖率 ≥80% / ≥140 ROM" | 80% × 177 = 142 ROM；≥140 是数学上的门槛值，但未与磁盘分母对齐 |
| "P2 22 ROM" | P2 阶段实际接入 manifest 的 ROM 数；远小于磁盘真实可用的 177 |
| "174 ROM"（误读） | 177 的舍入近似？来源不明，疑为 P4 文档笔误 |
| **本表口径** | **177 真实分母；目前 `blargg_suite` 已能批量跑全部 177 个（除去 3 个死条目）** |

## 接入状态机

```
磁盘文件          →  manifest 条目        →  tests.json 顶层条目    →  CTest 状态
─────────────────────────────────────────────────────────────────────────────
177 ROM          →  177 OK + 3 死条目    →  5 个顶层条目            →  33 PASS, 1 FAIL
                       (180 总计)              (blargg_suite 聚合)
```

- `blargg_suite` 通过 `fceux11_blargg_runner --manifest blargg_manifest.json` 跑**全部 177 个有效 ROM**。
- 其余 4 个顶层条目是"独立 manifest 条目"（每个跑单个 ROM），其中 2 个引用死文件 → 必然 FAIL。

## D-2 待办

按 D-1 清单扩充 manifest 接入 + 处理 3 个死条目：

1. ~~**重新拉取缺失的 3 个 CPU ROM**~~ → **不需要**：ROM 一直在磁盘上（命名错配，见顶部修订框）
2. ✅ **从 `blargg_manifest.json` 删除死条目** —— `bc7c1d8` 已删除 3 个重复条目，180 → 177
3. ✅ **`tests.json` 的 `blargg_cpu_instrs` / `blargg_cpu_timing`** —— 已改指实际文件、补足帧预算、
   实测 PASS 并转回 `blocking`

**D-2 待办已清空。**

## How to re-derive

```bash
# manifest 条目数
python -c "import json; print(len(json.load(open('tests/fixtures/blargg_manifest.json', encoding='utf-8-sig'))['roms']))"

# 磁盘 ROM 数
find tests/fixtures/blargg -name '*.nes' | wc -l

# 死条目清单
python -c "
import json, os
m = json.load(open('tests/fixtures/blargg_manifest.json', encoding='utf-8-sig'))
disk = set()
for r,_,fs in os.walk('tests/fixtures/blargg'):
    for f in fs:
        if f.endswith('.nes'):
            disk.add(os.path.relpath(os.path.join(r,f), 'tests').replace(os.sep, '/'))
for e in m['roms']:
    if e['path'] not in disk:
        print(e['name'], '->', e['path'])
"
```

## Why

阶段内一切"权威性"叙述必须落到可复算的数字上。177 这个分母之前从未在仓库内任何文档中出现——它本应是覆盖率公式的输入，却一直被人脑记忆里的某个 174 / 180 替代。

**How to apply**：后续所有覆盖率/权威性表述一律引用 `177`（磁盘 = manifest，S-1 后两者一致）这个具体值，禁止使用四舍五入或"约 170"等模糊措辞。S-1 之前的文档若写 `180（manifest 声明）`，那是含 3 个重复死条目的旧口径。

**另一条纪律（S-1 教训）**：判定「ROM 缺失」前必须先按**文件名**在整个 fixture 树里搜一遍，
而不是只看 manifest 声明的路径是否存在。`download_blargg_roms.ps1` 会重命名（`instr_test-v5/all_instrs.nes`
→ `instr_v5_all.nes`），照着 blargg 上游原名找必然找不到。同理，判定 ROM「失败」前先确认
`frames` 预算够 —— `$6000 == 0x80` 是**仍在运行**，不是错误码。