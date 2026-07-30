# E-1 调查记录：PPU VBL 周期与 blargg `ppu_vbl_nmi`

状态：**调查阶段第 2 轮（fresh 构建）**。本文记录两次会话的实测与结论，
先前 §1/§3 多项已**被推翻**，接手前先读 §0 与 §6 的「不要照抄」清单。

## §-1. 修复尝试记录

### 2026-07-30 session：1-cycle NMI delay（已回退，未提交）

**假设**：blargg 05_nmi_timing 期望 NMI 在 VBL set 后**第 2 CPU cycle**
触发；当前实现是立即触发（cycle 0），导致所有行数值偏 1。

**实测**：在 `src/ppu_rendering.cpp` 的「Working config」（普通）VBL
路径给 `if (VBlankON) TriggerNMI()` 加 1 cycle delay。

| 测试 | 修复前 | 修复后 |
|---|---|---|
| 01_basics | PASS | PASS |
| 04_nmi_control | PASS | PASS |
| rom_regression_test | PASS | PASS |
| golden_savestate_test | PASS | PASS |
| **05_nmi_timing** | FAIL 行 00-06 = "1"、07-09 = "0" | FAIL 行 00-01 = "2"、02-07 = "1"、08-09 = "0" |

**结论**：**简单 linear shift 不能修 05**。该 ROM 每行测试不同 timing
组合，相同 1-cycle 偏移在不同行产生**不同**结果（行 00-01 满足、02-07
仍错）。"deliver at cycle X" 不是单一参数问题。

**修正后的修复策略**：
1. 先读 blargg 05 内部每行的具体 timing 序列
2. 把 NMI 触发时机作为显式 cycle+dot 参数（非「立即 / +1」二分）
3. 同时考虑 `$2002` 读点相位（blargg sync_vbl 用）
4. 每次改一处，独立回归 01 / 04 / Oracle A / 05 单条

完整记录见 `memory/e1-step2-attempt-2026-07-30.md`。
代码 HEAD = 当前 release branch tip，PPU 段无变化。



## 0. 最重要的一条：增量构建的 exe 可能比源码旧好几个 commit

`build-c1/` 是增量构建树。第一次会话的「`01-vbl_basics` 因 #8 回归」是
**错误结论**：`fceux11_blargg_runner.exe` 是用 P4-1 配置（`runppu(1)`
前置 + `delay=19`）编译的，而工作树早已被 `9998b2b` 改回 `delay=20` /
VBL 在 cycle 0。两者不一致 → 整个 §1 表读数失真、§3「sync_vbl 不收敛」
假设也立在错数据上。

> 教训：动 PPU 时序前，先 `do_build.ps1` 全量重建再取基线。
> 增量树里的 `.exe` 可能比源码旧好几个 commit。

## 1. 实测基线（fresh 构建，frames=600）

`build-c1/tests/fceux11_blargg_runner.exe --rom tests/fixtures/blargg/ppu/vbl_0*.nes --frames 600`。
原始日志：`docs/e1_survey/vbl_baseline_2026-07-30.txt`。
复测入口：`scripts/collect_diag.ps1`（需把 `vcpkg_installed/{x64-windows/bin,x64-windows/debug/bin}` 加入 PATH，否则 exe 不会启动）。

| ROM | $6000 | status | 首行失败模式 |
|---|---|---|---|
| 01-vbl_basics | 0x00 | **PASS** | — |
| 02-vbl_set_time | 0x01 | FAIL | 00-04 应 V 得 `-`，05-08 应 V 得 `V` → **VBL set 偏晚 1 row** |
| 03-vbl_clear_time | 0x00 | **PASS** | — |
| 04-nmi_control | 0x00 | **PASS** | — |
| 05-nmi_timing | 0x01 | FAIL | 00-06 应 `2` 实得 `1`，07-09 应 `0` 实得 `0` → **NMI delivery 偏晚 1 cycle** |
| 06-suppression | 0x01 | FAIL | 00-04 应 `-` 实得 `N` → **NMI 未被抑制**（VBL 提前 clear？） |
| 07-nmi_on_timing | 0x01 | FAIL | 00-05 全 `N` → **NMI enable 后多触发** |
| 08-nmi_off_timing | 0x01 | FAIL | 00-02 错过 NMI；05-0C 全 `N` → **前几帧 NMI 被吃掉** |
| 09-vbl_even_odd_frames | 0x00 | **PASS** | — |
| 10-vbl_even_odd_timing | 0x03 | FAIL #3 | 明文诊断："Clock is skipped too late, relative to enabling BG" |

> **历史注**：第一次会话记录此表时全 02~10 都写 `$6000=0x80 挂死`。
> 0x80 在 blargg 协议里是「仍运行」**不是错误码**；本次 fresh 构建
> 02/05/06/07/08 实际返回 `0x01`、10 返回 `0x03`，并附结构化对照表。
> 旧记录不是因为 VBL 周期算错，而是过期的二进制。

## 2. 插桩实测数据（保留）

临时在 `runppu()/runppu1_inline()` 加全局 dot 计数、VBL set/clear 与
`A2002` 读点打点（env `FCEUX11_E1_TRACE`，**已回滚，未提交**），跑
`vbl_01_basics` 400 帧：

- VBL 周期：`period=6820`，**398/398 帧全部一致**。
- 帧长：`89342`（渲染关）/ `89341`（渲染开奇数帧跳点）交替。

**结论（保留）**：VBL 周期本身没有错。E-1 在 hotfix4 计划里描述的
「`ppu_rendering.cpp:1547-1582` 周期过冲」是错描述 —— 周期对，
**真正的失败机制是边沿/NMI 时序**，见 §3。

## 3. 真正的失败机制（fresh 数据修正）

`sync_vbl` 在 fresh 构建上**是收敛的**：

- 03 / 09 返回 `0x00 PASS`，证明在该类 ROM 上 sync_vbl 能锁定 VBL 边沿。
- 02 输出的对照表是结构化的（前 4 行反相、后 4 行反相）而非乱码，
  说明 sync 锁到了一致的相位，只是相对 CPU 偏晚一格。
- 06/07/08 的对照表也是逐行 9 行有效输出，不是「卡住」。

所以真正的失败机制是 **VBL / NMI 边沿相位与 CPU 观察点不对齐**，按 ROM 分：

| ROM | 偏差点 | 推测方向 |
|---|---|---|
| 02_set_time | VBL set 沿 | sl 241 上偏晚 1 row（一帧 ≈ 341 PPU dot） |
| 05_nmi_timing | NMI delivery | 比 CPU 期望值晚 1 cycle（≈ 3 PPU dot） |
| 06_suppression | VBL clear 沿 | 提前 clear → 抑制位检测不到 V？ |
| 07_nmi_on_timing | NMI enable gate | enable 后多触发 ⇒ 检测沿偏早 |
| 08_nmi_off_timing | NMI delivery / VBL start | 前 3 行漏 NMI，后续补发 |
| 10_even_odd_timing | even/odd 翻转沿 | "clock is skipped too late, relative to enabling BG" |

每个都对应一个**单独的时序参数**，不是「sync_vbl 不收敛」这种系统性
问题。

> **推翻 §3 旧假设**：原 §3 称「sync_vbl 因指令原子读偏移 9 PPU dot
> 永远拿不到目标边沿」—— 错。03/09 通过即证 sync_vbl 不依赖 sub-cycle
> 精度；失败 ROM 上 sync 已经收敛到**某个**相位，只是该相位与该 ROM 的
> 时序期望值不一致。

## 4. 下一步建议（按性价比与「可证伪」排序）

1. **先做 05_nmi_timing**：失败模式最干净 —— 单 cycle 偏晚，归因
   直接。可能改一处 `ppu_rendering.cpp` 的 NMI 注入时刻。
   - 风险：会改 NMI 时序；必须回归 01/04/Oracle A。
2. **再做 02_set_time**：与 05 互补，验 VBL **set** 沿（sl 241 的 dot 0/1）。
3. **06/07/08 一组**：NMI gating 相关，验 VBL **clear** 沿 + NMI enable/
   disable 路径。
4. **10_even_odd_timing**：独立机制 —— even-odd tick 时序。
5. **每一步后做强制回归**（见 §5 判定口径）。

> 顺序选择：05 比 02 更易判定 —— 02 的对照表反向，5、6、7、8、9 看
> 行模式不易归因；05 仅一行数字（应是 2 实为 1），偏差明确就是 1 cycle。

## 5. 判定口径

`tests/tests.json` 里 `blargg_ppu_vbl_nmi` 目前是 `advisory`。
在 E-1 真正修好前**不要**改回 blocking，也不要为了让它变绿去动
`failure_means`。每改一处 PPU 时序，必须回归：

- `vbl_01_basics` (PASS 基线)
- `vbl_04_nmi_control` (NMI enable/disable 路径 PASS 基线)
- Oracle A: `fceux11_rom_regression_test` + `fceux11_golden_savestate_test`

## 6. 不要照抄的章节（接手必读）

- **§1 表**：第一次会话记录此表时全 02~10 都写 `$6000=0x80 挂死`。
  已被 §1 本版（fresh 数据）整体替换。
- **§3**：原版整段「sync_vbl 因指令原子读偏移 9 PPU dot 永远拿不到
  目标边沿」已被推翻（fresh 数据证明 sync 在 03/09 上收敛，
  02/05/06/07/08 输出结构化对照表）。
- **§4 第 2 条**：原文「验证 §3 假设，在 A2002 加相位补偿」无需再做，
  sync 已收敛，问题在 PPU 边的时序参数。
