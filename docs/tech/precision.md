# 精度治理：已知限制、禁忌事项与纪律

> **STATUS: ACTIVE**（知识库参考文档；最后核对 2026-09-19）
> 汇总各精度调查与迁移任务的**长期有效结论**；原始过程数据已归档至
> `docs/history/surveys/`（本文底部附索引）。改动 PPU/APU/CPU 时序前先读本文。

---

## 1. 当前已知失败面（blargg baseline v1.16.0-P5）

权威文件：`tests/fixtures/blargg_known_fail.json`（**治理规则：AI 不得修改期望值；
baseline 更新需与代码变更同等级 review**）。180 ROM：PASS 120 / FAIL 60。

### 1.1 PPU / runppu 相关（9 条，新 PPU 主战场）

| ROM | 根因 | 修复方向 |
|---|---|---|
| `vbl_02_set_time` | VBL 置位在 cy0 而非 cy1（「Cycle 0→1 shift deferred」，ppu_rendering.cpp:1609-1627） | 平移整个 VBL 事件链，连带检查 NMI 注入点 |
| `vbl_05_nmi_timing` | NMI 延迟 8 dot 为经验校准 | 重校准需新探针数据 |
| `vbl_06_suppression` / `vbl_07/08_nmi_*` | $2002 读抑制、$2000 写触发的 dot 级窗口 | 依赖 vbl_02 先修 |
| `vbl_10_even_odd_timing` | odd skip 只在 sl==0 判定 | **`ppu_rendering.cpp:2057` 跳点块是禁忌保留项，勿改** |
| `oam_stress` | sprite 评估/overflow 行为近似 | 补做需先复现硬件 overflow bug |
| `ppu_open_bus` | 已按 blargg 逐 test 修复但仍有残留 | 查 e1_vbl 链 decay probe 数据 |
| `ppu_vbl_nmi`（综合） | 上述各项组合 | — |

### 1.2 CPU 相关（21 条）

- 指令级 `instr_v5_*`/`all_instrs`/`instr_timing`：组合挂、单组过的 accuracy gap。
- `cpu_int_2/3/4/5`：**结构性近似**——中断只在指令边界轮询（无 hijack/branch-delay，
  `x6502.cpp:515-579`）；修复要把轮询下沉到指令内固定周期点。
- `cpu_dummy_writes_oam/ppu`、`cpu_exec_space_ppuio`、`instr_misc*`：差距在
  $4014 OAM DMA 与 PPU 区路径，不在 RMW 宏。
- `cpu_reset_ram/regs`：复位状态细节。
- `cpu_interrupts`：ROM 格式不兼容，**永久跳过**（eventually_pass=false）。

### 1.3 跨子系统的结构性近似

- **$4014 SPR DMA 无 DMC 总线仲裁**（`ppu.cpp:1134-1142`）：`sprdma_dmc_dma` FAIL；
  补齐需要 Mesen2 级逐周期 DMA 状态机（halt/对齐抖动/与 DMC fetch 交错）。
- DMC fetch 恒 4 周期 stall（`sound.cpp:659-683`），未做 halt/假读细分。
- sprite overflow bug 未复现（正确性判定而已）。

## 2. 黄金回归体系（改时序代码前的护栏）

| 体系 | 内容 | 注意 |
|---|---|---|
| `ppu_frame_diff_test` | XBuf 可见区 61440 字节裸 memcmp，金标 `tests/fixtures/golden_frames/` | 拒绝 PNG/PPM（理由见测试头注释）；**仅在有意的 PPU 变更落地时重生成** |
| `golden_hashes.json` | 多 ROM 帧 CRC/MD5 链 | R5/R6 若发生真实精度回归需追加 diff 行 |
| `golden_savestate_test` | savestate 字节金标 | 见 §3 禁忌 |
| blargg 180 ROM | KagamiQA 双 Oracle（A 回归 / B 硬件一致性） | 见 KagamiQA.md |
| 游戏级锚（注释级） | Knight Rider→ppudead、Super Donkey Kong→OAMADDR、3-D WorldRunner→dot257、SMB3/Crystalis→MMC3 hook、Bee 52→FRAMESKIP | 改注释锚指向的行为前先跑对应游戏 |

## 3. 禁忌清单（改了会碎东西）

1. **Savestate chunk 不可动**：`FHCN`/`FCNT`/`IQFM`/`PSG`/`LEN0..3`/`5ACC` 等的名字/
   大小/序（`sound.cpp:1303-1307, 1633-1688`）；`Cpu::layout_` 偏移 0 + 64 字节对齐
   （`cpu.cpp:14-26`）；运行期起始值同样危险（`wlcount=2048`、`nreg=1`、`DMC_7bit` hack）。
2. **`ppu_rendering.cpp:2057` 跳点块**：vbl_10 调查结论「有据已知限制」，保留勿改。
3. **`V=(V&0xC0)>>6` swap**：§4.4 P1 决策禁项（见 `docs/history/surveys/r5r6_v1.17/`）。
4. **`sound.cpp:1095` FIXME**：按 P3 处方处理，不与 P1/P2 同 commit。
5. **NMI 延迟量已证伪假设**（「延迟是根因」方向被探针否决过）：调整前必须重跑
   `FCEUX11_E1_NMIDELAY` 探针。
6. **blargg 期望值**：AI 不得改 `blargg_known_fail.json`。

## 4. 纪律（历史任务沉淀的通用规则）

- **Instrument-first**：改时序代码前，必须先以 env-gated 探针采集数据
  （`FCEUX11_E1_TRACE`/`E3_TRACE`/`E1_NMIDELAY`/`OPENDECAY_PROBE`，生产零开销）。
- **CI 数字回填纪律**：任何 CI 数字以 artifact `engine.git_rev` 为唯一可信来源，
  禁止手改（KagamiQA.md §0）。
- **迁移 parity 纪律**：Rust harness 必须逐字节镜像 C++ 驱动可观测行为（CLI、
  输出格式、退出码、CRC 链、watchdog）；任何 parity miss → 该测试留在 C++
  （Task1 Track C 结论）。
- **有据已知限制**：每个 FAIL 必须有探针数据支撑的根因描述，才能标「有据」；
  无据的 FAIL 视为 bug。

## 5. 调查数据索引（docs/history/surveys/）

| 链 | 内容 | 与本文的关系 |
|---|---|---|
| `e1_vbl/` | VBL/NMI 时序探针（vbl_05 反汇编、Step1.1-1.4、NMI 采样、E1B 探针） | §1.1 vbl_* 根因证据 |
| `e6_apu/` | APU 帧计数器链（r6_step*）+ R6 仪器数据 | §1.3 DMC/帧计数器证据 |
| `r5r6_v1.17/` | v1.17 R5/R6 迁移核查与仪器数据 | §2 黄金回归现状 |
| `p2_instrument/` | P2 精度收敛交接档案（8 份 survey 汇总） | §3/§4 禁忌与纪律的出处 |

## 6. 相关文档

- 测试框架与 CI 门禁：[KagamiQA.md](KagamiQA.md)
- 各模块细节：[cpu.md](cpu.md) §四、[ppu.md](ppu.md) §三、[apu.md](apu.md) §三
- 归档规则：[../history/README.md](../history/README.md)
