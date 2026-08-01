# E-1 PPU VBL/NMI Step 1.1 — 路径 (a) 置位点修正 证伪记录 (2026-08-01)

> **目的**：按 `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` Phase 1 Step 1.1 执行路径 (a)（VBL flag 置位点 cycle0→dot1），实测证伪。
>
> **依据**：NESdev Wiki「PPU frame timing」——VBL flag 硬件置位于 sl 241 dot 1（NTSC）；blargg `ppu_vbl_nmi` 套件源码。
> **分支**：`wip_1.16`（HEAD `f50573a`，工作树直接改）
> **作者**：独立执行（2026-08-01）

---

## 1. 改动内容（已回滚）

`src/ppu_rendering.cpp` working-config VBL 块（原 `:1556-1595`）：
- 前置 `runppu(1)`（cycle 0→1），`PPU_status |= 0x80` 于 dot 1 置位
- 删除 R5 Step 3 的 `runppu(3)` hack，NMI 与 flag 同 dot latch
- S=0 主循环上限 `kLineTime` → `(kLineTime-1)`（320 dots），保持块总长 6820
- 附带 env-gated `FCEUX11_E1_TRACE` 探针（VBL_ENTRY / VBL_SET / VBL_CLR）

**周期核算（错误所在）**：块总长 = 1+20+320+19×341 = **6820** ✓，但 **VBL 置位→清除 = 6820−1 = 6819** ✗
（flag 于块内 dot 1 置位，清除在块末 dot 6820）。`vbl_01` 的 6820 不变量是 **set→clear** 周期，不是块总长。

## 2. 实测结果（10 ROM × 600 帧）

| ROM | 改动前 | 改动后 | 变化 | diag |
|---|---|---|---|---|
| `vbl_01_basics` | 0x00 PASS | **0x07 FAIL** | 🔴 回归 | "VBL period is too short with BG off" |
| `vbl_02_set_time` | 0x01 FAIL | 0x01 FAIL | ⚪（模式变差：全行 "V -"） | — |
| `vbl_03_clear_time` | 0x00 PASS | **0x01 FAIL** | 🔴 回归 | 全行 "-" |
| `vbl_04_nmi_control` | 0x00 PASS | 0x00 PASS | ✅ 保持 | — |
| `vbl_05_nmi_timing` | 0x01 FAIL | 0x01 FAIL | ⚪（变差：全行 X=1） | — |
| `vbl_06_suppression` | 0x01 FAIL | 0x01 FAIL | ⚪ | 全行 "V N" |
| `vbl_07_nmi_on_timing` | 0x01 FAIL | 0x01 FAIL | ⚪ | 全行 "-" |
| `vbl_08_nmi_off_timing` | 0x01 FAIL | 0x01 FAIL | ⚪ | 全行 "N" |
| `vbl_09_even_odd_frames` | 0x00 PASS | **0x02 FAIL** | 🔴 回归 | "Pattern ----- should not skip any clocks" |
| `vbl_10_even_odd_timing` | 0x03 FAIL | 0x02 FAIL | ⚪（码变） | — |

**Oracle A**：`ctest -LE perf` **34/34 PASS**（无 Oracle A 回归——golden savestate 未碎，说明 savestate 捕获点不敏感于 VBL 置位 dot）。

## 3. 结论

1. **置位点 cycle0→dot1 单独调整是净负**：3 个 PASS ROM（01/03/09）翻红，0 个 FAIL ROM 转 PASS。
2. **周期不变量是 set→clear = 6820**，不是块总长。flag 于 dot 1 置位则清除必须于 dot 6821——需重构 VBL 块 + idle（`runppu(kLineTime)`）边界，本次未做。
3. **vbl_02 置位点假设证伪**：改动后全行 "V -"（更差），印证 `vbl_step1_instrument_data` §3.2/§5.2 结论——vbl_02 真因是 **$2002 读抑制缺失**（row 04 应 "- -" 实为 "- V"），非置位点。
4. **vbl_05 置位点假设证伪**：改动后全行 X=1（比 runppu(3) 的 [2,2,2,2,1,1,1,1,1,1] 更差），印证单参数 PPU 侧调整无法修复——NMI 早 ~2 指令是 **CPU 侧 latch 相对指令流的时序**问题。
5. **vbl_09 回归**：置位点移动扰动 even/odd 帧交互（`sl==0` 的 `end_cycle=340` 逻辑与帧内 dot 相位耦合），验证了"vbl_09 依赖跳点位置、盲目移动必回归"的预案。

## 4. 处置

- ✅ 已 `git checkout -- src/ppu_rendering.cpp` 回滚（HEAD `f50573a` 状态，runppu(3) 保留）。
- ✅ Oracle A 34/34 状态由改动本身保证（已实测），回滚后无残留。
- 📝 方案文档 Phase 1 Step 1.1 已就地标注证伪，修订为 Step 1.2（$2002 抑制）优先 + vbl_05 转 CPU 侧 latch 调查。

## 5. 修订后的 Phase 1 方向

| 步骤 | 内容 | 依据 |
|---|---|---|
| **Step 1.2（优先）** | $2002 读抑制窗口（vbl_02 row 04 / vbl_06） | step1/step2 probe：vbl_02 仅 row 04 错，置位点正确；A2002 无抑制逻辑 |
| **Step 1.3** | CPU 侧 NMI latch 时序（vbl_05，~2 指令早） | step2 probe：NMI_ENTRY X 序列 [2,1,1,1,1,1,1,0,0,0]；runppu(N) 单参数不可行 |
| **Step 1.4** | even/odd 跳点（vbl_10，独立机制） | 保持原预案 |
| ~~Step 1.1~~ | ~~置位点 dot1~~ | **证伪**（本文档） |

---

*证伪记录完。下一步：Step 1.2（$2002 抑制）。*
