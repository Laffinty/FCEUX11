# blargg ROM 接入清单（Stage-2 §八 D-1）

> **生成日期**：2026-07-30
> **目的**：把"22/174"、"~80%"等口径不一的旧叙述统一到一个可复算的基线
> **基线**：`tests/fixtures/blargg_manifest.json` + `tests/fixtures/blargg/` 实际文件树

## 总览

| 口径 | 数量 | 来源 |
|---|---|---|
| 磁盘上的 ROM | **177** | `find tests/fixtures/blargg -name '*.nes' \| wc -l` |
| `blargg_manifest.json` 条目数 | **180** | JSON 数组长度 |
| manifest 中引用不存在的文件 | **3** | 全部位于 `fixtures/blargg/cpu/` |
| tests.json 的 blargg 顶层条目 | **5** | `blargg_smoke` / `blargg_cpu_instrs` / `blargg_cpu_timing` / `blargg_ppu_vbl_nmi` / `blargg_suite` |
| `blargg_suite` 通过 `--manifest` 驱动的 ROM | **177** | 仅磁盘实际存在的 |

> **覆盖率分母 = 177**（磁盘上真实存在的 blargg ROM 数）。
> 180-177=3 个 manifest 条目是**死条目**（指向不存在的文件，运行时会 loadrom 失败）。

## 分类细分

| 类别 | manifest 条目 | 磁盘实际 | 缺口（死条目） |
|---|---|---|---|
| `apu` | 52 | 52 | 0 |
| `cpu` | 61 | 58 | **3** |
| `mmc3` | 18 | 18 | 0 |
| `ppu` | 49 | 49 | 0 |
| **合计** | **180** | **177** | **3** |

## 三个死条目

| 名称 | manifest 声明路径 | 状态 |
|---|---|---|
| `all_instrs` | `fixtures/blargg/cpu/all_instrs.nes` | ❌ 文件不存在 |
| `cpu_timing_test` | `fixtures/blargg/cpu/cpu_timing_test.nes` | ❌ 文件不存在 |
| `official_only` | `fixtures/blargg/cpu/official_only.nes` | ❌ 文件不存在 |

这三个被 `tests.json` 中的 `blargg_cpu_instrs` / `blargg_cpu_timing` 两个**独立条目**（非 `blargg_suite`）直接引用，构建时 CTest 标 FAIL：
- `tests.json:blargg_cpu_instrs.input.args = ["--rom", "fixtures/blargg/cpu/all_instrs.nes", "--frames", "300"]`
- `tests.json:blargg_cpu_timing.input.args = ["--rom", "fixtures/blargg/cpu/cpu_timing_test.nes", "--frames", "300"]`

`all_instrs.nes` 与 `cpu_timing_test.nes` 是 blargg 公开测试套件中的**核心权威 ROM**（CPU 完整指令覆盖 + 时序），它们的缺席会显著压低覆盖率与权威性上限。

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

1. **重新拉取缺失的 3 个 CPU ROM**（`all_instrs.nes` / `cpu_timing_test.6` / `official_only`）
   - 或：从 `scripts/download_blargg_roms.ps1` 重新跑（参见脚本注释）
2. **从 `blargg_manifest.json` 删除死条目**（或保留但加 `"missing"` 标记，让 runner 跳过并记录已知失败）
3. **`tests.json` 的 `blargg_cpu_instrs` / `blargg_cpu_timing` 在 ROM 补齐后即可转 PASS**

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

**How to apply**：后续所有覆盖率/权威性表述一律引用 `177（实际）或 180（manifest 声明）` 这两个具体值，禁止使用四舍五入或"约 170"等模糊措辞。