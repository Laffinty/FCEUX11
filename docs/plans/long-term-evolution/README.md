# FCEUX11 v1.18 — F11QA 残留精度长期演进 · 文档索引

> **STATUS: OPEN**
> **基线**：`f11qa.yml` run **98252605787** · `git_rev=b0658c9` · **106P / 14F** · grade **B**
> **当前**：v1.18.2 / 矩阵 **108P / 12F** / grade B · **2 DONE**（① 056、② 078，T0 收口 2/2）
> · **2 abandon**（④ 097、⑤ 096，均为 survey ROM，非精度债）· **1 defer**（③ 038 → T3）
> · 9 项未开始 · **T1 仅 2 项可攻关**（050 / 051）
> **用途**：未来一段时期**独立、逐步**清零 F11QA 残留精度缺口。

---

## 文档集

| 文件 | 角色 | 何时用 |
|---|---|---|
| [`FCEUX11-v1.18_F11QA残留精度长期演进计划.md`](FCEUX11-v1.18_F11QA残留精度长期演进计划.md) | **主报告**：筛选口径、14 项独立分析、难度总表、**§2.2 三项重估裁定**、阶段编排、风险 | 立项前通读 |
| [`T0-指令时序与板级约束.md`](T0-指令时序与板级约束.md) | T0 执行卡（056 / 078） | **已收口 2/2** |
| [`T1-Mapper与DMA.md`](T1-Mapper与DMA.md) | T1 执行卡（050 / 051；097 / 096 已 abandon） | T0 后，可并行切分 |
| [`T2-中断时序.md`](T2-中断时序.md) | T2 执行卡（037） | 独占分支，1–2 周 |
| [`T3-PPU读缓冲与外设时序.md`](T3-PPU读缓冲与外设时序.md) | T3 执行卡（**038 defer** / 049 / 099 / 107 / 108） | 长周期，按项立项 |
| [`T4-聚合与FDS.md`](T4-聚合与FDS.md) | T4 执行卡（077 / 081） | 随 T1–T3 cascade |

---

## 推荐推进顺序

```
T0 指令时序与板约束（056→078）                2/2 DONE ✅ 收口（038 已按 §2.2 defer 移出）
    ↓
T1 Mapper/DMA（050→051）                       仅 2 项可攻关（097 与 096 均已按 §2.2 abandon）
    ↓
T2 中断时序（037，独占）                       ← 077 现在完全依赖本阶段的 MMC3 IRQ 组
    ↓
T3 总线模型/PPU读缓冲/外设（038-S0+S1 → 049 / 099 / 107 / 108）
    ↓
T4 聚合与 FDS（081 先加载后 IRQ；077 成本 = 仅 L2 MMC3 IRQ 组）
    ↓
ζ 评级：L2 eventually_pass 清零 → grade A
```

> **defer / abandon 是与 DONE / TODO 并列的终态**，口径与两项重估的完整论证见
> 主报告 **§2.2**。矩阵预期值不变（仍 108P/12F、grade B）。

---

## 完成度速查

> 截至 v1.18.2：**2 / 14 DONE（14%）**，**2 abandon**（097、096）、
> **1 defer**（038）、**9 项未开始**。矩阵与 grade 均未变（108P/12F、grade B）——
> defer / abandon **不改动矩阵预期值**。**T1 只剩 2 项可攻关**（050 / 051）。

| # | kgmqa | 难度 | Tier | 主题 | 预估 | 状态 |
|---|---|---|---|---|---|---|
| 1 | 056 | ★☆☆☆☆ | T0 | 指令周期表 E2/BB | 1–3 天 | **DONE** |
| 2 | 078 | ★★☆☆☆ | T0 | MMC1 SEROM/SHROM 板约束 | 2–4 天 | **DONE** |
| 3 | 038 | ★★→**★★★★☆** | T0→**T3** | CPU 数据总线锁存（049 前哨） | **1–2 周** | **defer** |
| 4 | 097 | —（无缺陷） | T1→**移出** | FME-7 WRAM 映射 | — | **abandon** |
| 5 | 096 | —（无缺陷） | T1→**移出** | FME-7 IRQ ack | — | **abandon**（**留有真实 ack bug 不修**） |
| 6 | 050 | ★★★☆☆ | T1 | OAM DMA dummy write | 4–7 天 | TODO |
| 7 | 051 | ★★★☆☆ | T1 | IO 空间取指 / bus dispatch | 4–7 天 | TODO |
| 8 | 037 | ★★★★☆ | T2 | 中断轮询下沉 + hijack | 1–2 周 | TODO |
| 9 | 049 | ★★★★★ | T3 | `$2007` 读缓冲 | 2–4 周 | TODO |
| 10 | 099 | ★★★★★ | T3 | NTSC/PAL/Dendy TV 时序 | 2–4 周 | TODO |
| 11 | 107 | ★★★★★ | T3 | SNES 鼠标扫描协议 | 1–2 周 | TODO |
| 12 | 108 | ★★★★★ | T3 | FC 麦克风扫描协议 | 1–2 周 | TODO |
| 13 | 077 | ★★★★★（**派生**） | T4 | Holy Mapperel 聚合 | **仅 L2 MMC3 IRQ 组** | TODO |
| 14 | 081 | ★★★★★ | T4 | FDS 加载 `0xFE` + IRQ | 2–4 周 | TODO |

**两处排序/纪律修正**（相对主报告初版）：

1. **⑬ 077 的成本是派生的**，不按 star 数独立排期 —— 它 = ②+④+⑤ 剩余 + L2 MMC3 IRQ 组，
   排期改为「随 β 顺带推进，**β 收口即重估**」。详见主报告 §2.1。
2. **R4 gate 方向已修正**（`b6fe92a`）：现守 `pass_to_fail`（回归，红线），
3. **② 078 的原根因判断是错的**（v1.18.2 实测）：计划写「bank 宽度约束未建模」，
   实际有**两个独立缺陷且 1 掩盖 2**。承载项是 **`$6000` 未被映射**——
   ROM 头声明 `PRG-RAM=0`，而 `DetectMMC1WRAMSize()` 只对 NES 2.0 板信头部，
   同一 ROM 写成 iNES 1.0 则会拿到默认 8 KB；blargg 结果寄存器写不进去，
   于是无论 mapper 怎么译码都报同一个 `0xC3`。
   submapper 5「Fixed PRG」语义（缺陷 2）符合规范且改变了真实映射，
   **但本 ROM 测不出**（32 KB PRG 时 `PRGmask16=1`，普通 MMC1 与固定映射数值相同）。
   详见 T0 卡 ② 与主报告 §三② 执行记录。

> **可复用教训**：探针读回一个**恒定**字节（且整个 diag 区同值）时，
4. **③ 038 的根因改判（v1.18.3 任务，未修复）**：计划写「`LDA abs,x` dummy read 不完整」，
   实测不成立。把 `GetABIRD` 改成硬件行为（未修正地址无条件读）后，
   **测试结果一点没变** —— 探针显示该测试的 33 次 abs 索引读**全部不跨页**，
   此时 dummy 地址 == 真实地址，多读一次观察上完全等价。
   真正阻塞点是 **FCEUX11 没有 CPU 数据总线模型**：
   `Cpu::set_db()` 全仓库零调用点，`DB` 永不赋值，`::ANull` 因此返回常量；
   `PPUGenLatch` 也不跟随通用 CPU 总线访问。
   一个"观察 dummy read 往总线放了什么"的测试当前**无物可观察**。
   需要实现总线锁存 —— 与 ⑨ 049 同族，**本项预估（3–5 天）明显偏低，建议重估**。
   已把探针实测数据与结论记入 T0 卡 ③ 与主报告 §三③。

5. **③ 038 已重估并裁定 defer：★★☆☆☆ / 3–5 天 → ★★★★☆ / 1–2 周，风险低 → 中，
   Tier T0 → T3（排在 ⑨ 049 之前）。**
   上一条里「需要实现 CPU 数据总线锁存…与 049 同族」说重了：查证 2C02 的 PPU latch
   语义（由**端口访问**填充，正是 FCEUX 现有 `PPUGenLatch` 模型）后，
   「CPU↔PPU 耦合」并非已证实的阻塞点。
   真正确证的是：`set_db()` 零调用点 → `::ANull` 返回常量，**open bus 是死的**。
   最小可判别实验 = **S0+S1（1–3 天）**：让 `DB` 活起来，然后让 03_dummy 自己
   分辨走哪条通道。
   **裁定 defer（留待以后解决）**：038 不阻塞 release，也不占独立窗口；
   它的价值是**给 ⑨ 049 铺路**（两者共用「活的总线值」这一层），
   故性价比最高的顺序是 **038-S0+S1 → 再开 049**。
   **T0 随之收口 2/2 DONE**（056 + 078），出口指标 14F→12F、advisory→10.0% 已达成。
   完整论证见主报告 §2.2，执行卡已移入 [`T3-PPU读缓冲与外设时序.md`](T3-PPU读缓冲与外设时序.md) 3′ 段。

6. **④ 097 已改判并裁定 abandon：彻底放弃。**
   `fme7ramtest` 是 **survey ROM**，不实现 blargg 的 `$6000` 结果协议 ——
   静态扫描 32 KiB PRG，对 `$6000` 的绝对寻址指令 **0 处**，
   对 `$6900` **恰 2 处**（`STA`/`LDA $6900`，与上游 `main.s` 的
   `check_bank_numbers` 逐字吻合）。`value=0x01 diag=[0x02,0x04,0x08]`
   就是 `check_for_wram` 写的 9 字节 RAM 图样前 4 字节
   （`01 02 04 08 10 20 40 80 00`），**不是结果码**。
   而 harness 判 PASS 的唯一条件是 `probe_addr` 读到 `0x00`（`blargg.rs:316`），
   `probe_addr` 恒为 `0x6000` 且 `tests.json` 无 per-case 覆盖；
   换读 `$6900` 得 `0xC0`，同样不是 `0x00` —— **没有可行地址**。
   反过来，`FCEUX11_FME7_PROBE=1` 实测 bank tag 为
   `C0 C1 C2 C3 C0 C1 C2 C3`（两组），与上游 README「With 62256 (32Kx8)」
   期望输出**逐字节相同** —— **FME-7 WRAM 映射本身是对的**。
   **裁定 abandon（彻底放弃攻关）**：不是精度债，无目标投入。
   已按人工授权 A 改 `tests/tests.json` 的 `known_limit` + `provenance`；
   **注意 `known_limit` 字段不参与任何机械判定**
   （`grade.rs::compute_grade` 只读 `failure_means` / `pass_to_fail` / `new_test` /
   `test_set_diff` / `summary.failed`），所以矩阵仍 108P/12F、grade 仍 B ——
   改动买的是语义，不是数字。
   「abandon 攻关」≠「从矩阵删用例」：删掉会让 `test_set_diff.removed` 非空
   （矩阵变 119 / 108P-11F，**grade 仍 B**），属评审范围，本次未做。
   ⑬ 077 的派生成本随之收窄为 **⑤ 096 + L2 MMC3 IRQ 组**。
   完整论证见主报告 §2.2 与 §三④ 调查记录。

7. **⑤ 096 已改判并裁定 abandon，代码不动；留有一个真实 ack bug 明确不修。**
   `fme7acktest` **也是 survey ROM**。矩阵报的 `value=0xA2 diag=[0x0F,0x8E,0x00]`
   与 **PRG ROM bank 3 offset 0-3 逐字节相同**（那 4 个字节本身就是 6502 指令
   `LDX #imm` / `BPL` / `STX abs` / `BRK`）。成因：上游 `fme7.s` 的 `init_fme7`
   把 reg 8 写成 3 → `preg[3]&0xC0==0` → `setprg8(0x6000, 3)` → `$6000` 是 PRG ROM。
   **按硬件这是对的**（reg 8 的 bit7=+CE、bit6=RAM/ROM select，`$03` 两位皆 0 → 选 PRG ROM），
   不是模拟器 bug。ROM 对 `$6000` 的绝对寻址指令数 = **0**，结果只写零页并画到屏幕上。
   → 改 mapper 也不会转 PASS。

   **但确有真实 bug**：硬件规则是 **bit 0 == 0 才 ack**（PowerPak / Everdrive 两个独立
   硬件实现 + Nintendulator 三方一致）。现状 `case 0xD: IRQa = V; X6502_IRQEnd(...); break;`
   无条件 ack → 6 个用例里 **`$0D=$01` 与 `$0D=$81` 两项错**（应为 No ack）。
   > ⚠️ nesdev wiki 现写着「All writes to this register acknowledge an active IRQ」，
   > 那是照 FCEUX 行为改的，与硬件实测相反。**以硬件为准。**

   **人工裁定只 abandon、不改代码**，理由：该 ROM 报不了 pass/fail → 改完验证不了
   （038 纪律）；要验证需新增 ctest 用例（`AWrite[]` 驱动 `$8000`/`$A000`、
   读 `X.IRQlow & FCEU_IQEXT` 断言 ack 表），属独立工作项；且 `IRQa = V` 的使能位语义
   按规范应是 bit 0/bit 7，但 **bit 7 有争议**（wiki 说 0=停计数器，与 PowerPak 实测矛盾），
   改它会动到 Batman: Return of the Joker / Gimmick! 的真实 IRQ 时序。
   **一行可修的最小改法已留档在主报告 §2.2 ⑸**：
   `case 0xD: IRQa = V; if (!(V & 1)) X6502_IRQEnd(FCEU_IQEXT); break;`
   该 bug 影响真实 FME-7 游戏的 IRQ 应答行为，**但不体现在任何 F11QA 指标上**。
   完整论证见主报告 §2.2 ⑸。

   **连带后果**：T1 只剩 **050 / 051** 两项可攻关（出口 4/4 → 2/2）；
   ⑬ 077 的派生成本两次收窄为 **仅 L2 MMC3 IRQ 组**。

> **方法论**：把「实测确证」和「未验证假设」分开写，比给一个干脆的结论有用。
> 上一轮就是因为把推理当结论，才需要事后收回。

> 先怀疑**寄存器本身没被映射**，而不是去改被测逻辑；
> 改 ROM 头部一个字节做 A/B 是零成本分辨手段。
   `fail_to_pass`（精度进展）不再阻断 —— 否则「修好一项」会被门禁拦红，与本计划目的相反。
   **推论：`tests/fixtures/f11qa_baseline_frozen.json` 须随每项修复同 PR 更新**，
   否则已修项会被下一项重复计入 `fail_to_pass`。详见主报告 §7.1。

---

## 不变式（每个分项计划通用）

1. **内部逻辑检测保持 42P / 0F**，红灯即停。
2. **`pass_to_fail` 必须为 0**，出现即回滚 PR。
3. Instrument-first：改时序前先上探针（`docs/tech/precision.md` §4）。
4. known_fail / frozen baseline / savestate 金标更新 **须人工授权**。
5. 禁止无据 `known_limit`；禁止把 blocking 降级刷绿。

---

## 状态回写

单项完成后：

1. 分项计划内 checklist 勾选；
2. 主报告 §八 状态总表更新；
3. `docs/tech/f11qa-accuracy-backlog.md` 对应条目划掉；
4. PR 描述附：根因、探针数据、矩阵数字（fail_to_pass / pass_to_fail）。
