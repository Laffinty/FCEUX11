# kgmqa-050 OAM DMA dummy write —— 取证调查页（D4）

> **STATUS: OPEN**（阴性证据已取得，修复落点待业主裁决）
> 落地：2026-11-06（v2.0.2 计划 P3 批次 D4，提交 `adc45ae`）
> 本页是 **kgmqa-050 的唯一 RE 入口页**：把散在 v2.0.2 计划变更记录 §十 r6 / r7
> 两处表格单元格里的 D4 取证结论，整理成可复核的一页，并补上「怎么跑、怎么读、
> 下一步从哪切入」。
>
> **证据强度标注**（全页统一）：
> - `[实测]` = 有实测读数支撑（注明出处与提交/文档）
> - `[读码]` = 读源码推断，**未实测**
> - `[历史]` = 历史文档记录，本轮未复核
> - `[未确定]` = 不知道，需 RE 验证
>
> ⚠️ **本页不含任何修复立项建议**。修复归属与优先级是业主的决定，本页只给事实与
> 依赖关系（§5）。

---

## 1. 问题陈述：050 是什么

### 1.1 用例身份

| 字段 | 值 | 出处 |
|---|---|---|
| kgmqa id | `kgmqa-050-cpu-dummy-writes-bisqwit` | `tests/tests.json:1511` |
| 标题 | bisqwit cpu_dummy_writes | `tests/tests.json:1513` |
| ROM | `bisqwit/cpu_dummy_writes_oam.nes` | `tests/tests.json:1544`（`mirror_path`） |
| 镜像源 | `v1.8.0-mirror`，`vendor_state: vendored` | `tests/tests.json:1543,1545` |
| 协议 | `$6000`（跑 N 帧 → `$6000`==0x00 即 PASS） | `tests/tests.json:1546` |
| 预期 | `exit_code: 0` | `tests/tests.json:1533` |
| 帧预算 | `--frames 300` | `tests/tests.json:1528-1529` |
| 超时 | 60 秒 | `tests/tests.json:1535` |
| 失败严重度 | `failure_means: advisory` | `tests/tests.json:1541` |
| known_limit | `dummy writes to OAM DMA / PPU region; RMW macro OK, DMA path not` | `tests/tests.json:1547` |

**上位口径**：F11QA 矩阵 `108P / 12F`（grade B）不变；050 属「长期难题」，
不阻塞本体发版（`docs/plans/long-term-evolution/` 长期演进计划 §八 / T1 卡）。

### 1.2 缺陷本体（判据是什么）

050 **不是** 一个单一断言，而是一组 bisqwit 子测试打包成一个 ROM：

- **主体**：`dummy_writes` —— 验证所有 RMW 操作码（ASL/ROL/LSR/ROR/DEC/INC 与
  非官方 SLO/RLA/SRE/RRA/DCP/ISB）以 `$2004`（OAM）为目标时的**两次写**语义：
  第一次写原值（dummy write），第二次写修改后的值。由于 OAM 写后自动递增，
  两次写分别落到 `OAM[N]` 与 `OAM[N+1]`。`[历史]`
  （`docs/history/surveys/cpu_bucketB/stepB_investigation_2026-08-04.md` §1.1）
- **前置子测试**：`oam_read_test` —— 256×16 = 4096 次迭代，每迭代「随机 OAM 地址
  X → 写 complement → 11 个随机写 → 读 OAM[X] → 比较」，任何不匹配置
  `oam_read_test_failed`。`[历史]`（同上 §1.2）
- **失败错误码 `0x06`** = **`oam_read_test` 失败**（即「OAM 读不可靠」），
  **不是** dummy_writes 本身失败。`[历史]`（同上 §3.3、§2.3）
  > 原文：`| cpu_dummy_writes_oam | 0x06 | oam_read_test 失败，dummy_writes 本身通过 | OAM 读时序（**深模型**） |`

⚠️ **这条判据口径是本页最容易被误读的地方**：`known_limit` 写的是
「dummy writes to OAM DMA」，但**实测失败的是 OAM 读子测试**。历史调查也早已
记录「探针显示 B4014（OAM DMA）未被调用；test 不触发」
（`stepB_investigation_2026-08-04.md` §2.3.4 假设排除表「DMA 写」一行）。
`[历史]`

### 1.3 现状：三个组件的错位

| 组件 | 状态 | 出处 |
|---|---|---|
| RMW 宏本身 | **已正确**（dummy write 齐全），**不是 bug** | `stepB_investigation_2026-08-04.md` §3.1 |
| `cpu_dummy_writes_ppu` | **已 PASS**（桶 C 修复 `863e9d7`） | 同上 §0 摘要 / §4.1 |
| `cpu_dummy_writes_oam` | **仍 FAIL（0x06）**，记入有据已知限制 | 同上 §4.1；`docs/tech/precision.md:33-34` |

---

## 2. 探针：`FCEUX11_OAMBUS_PROBE`

> 实现：`src/ppu.cpp:600-937`（设计说明 doc comment 在 `:600-624`）。
> 提交 `adc45ae`「feat(ppu): D4 取证探针 FCEUX11_OAMBUS_PROBE（默认静默，不改行为）」。
> **默认完全静默**：不设环境变量时代码路径零动作、零开销。

### 2.1 开关与产物

| 项 | 值 | 出处 |
|---|---|---|
| 环境变量 | `FCEUX11_OAMBUS_PROBE=<输出目录>` | `src/ppu.cpp:625-631` |
| 判定 | 值非空即开启；**空串或未设 = 完全不激活** | `src/ppu.cpp:627-628` |
| 读取时机 | `static const bool` 的一次性初始化（进程内只读一次 env） | `src/ppu.cpp:626-629` |
| 输出目录 | 不存在则 `std::filesystem::create_directories` 自建 | `src/ppu.cpp:728-729` |
| 产物 1 | `<dir>/trace.bin`（二进制，`"wb"` **截断**） | `src/ppu.cpp:731-732` |
| 产物 2 | `<dir>/spram_diff.log`（文本，`"w"` **截断**） | `src/ppu.cpp:733-734` |
| 打不开时 | 探针**自禁用**并在 stderr 打一行错误（宁死勿噪） | `src/ppu.cpp:735-742` |
| 收尾 | `atexit` 钩子写结束汇总行并关闭两个文件 | `src/ppu.cpp:755-770, 752` |

⚠️ 两个文件都是**截断模式**：同一目录重复跑会覆盖上一次的证据。要保留多轮
对比必须换目录。

### 2.2 `trace.bin` 格式

**文件头（8 字节）** —— `src/ppu.cpp:745-746`

| 偏移 | 内容 |
|---|---|
| 0..5 | 魔数 ASCII `"OAMB1"` |
| 6 | 记录长度 = `24`（0x18） |
| 7 | 保留 = `0` |

**记录（24 字节定宽，x64 native alignment ⇒ 小端）** —— 结构体
`OamBusTraceRec`，`src/ppu.cpp:646-656`（`static_assert(sizeof == 24)`）

| 偏移 | 字段 | 类型 | 含义 | 出处 |
|---|---|---|---|---|
| +0 | `cpu_ts` | u64 | `g_cpu.timestamp_base() + g_cpu.timestamp_ref()` | `ppu.cpp:714-716` |
| +8 | `ppu_dot` | i32 | `ppur.status.cycle` = **扫描线内的 dot 计数（0..340）** | `ppu.cpp:788` |
| +12 | `ppu_sl` | i32 | `ppur.status.sl` = 扫描线号（0..261） | `ppu.cpp:789` |
| +16 | `addr` | u32 | 访问地址 | `ppu.cpp:790` |
| +20 | `rw` | u8 | 操作码，见下 | `ppu.cpp:791` |
| +21 | `data` | u8 | 数据字节 | `ppu.cpp:792` |
| +22 | `oamaddr_before` | u8 | 该次总线操作**前**的 OAMADDR（`PPU[3]`） | `ppu.cpp:793` |
| +23 | `oamaddr_after` | u8 | 该次总线操作**后**的 OAMADDR | `ppu.cpp:794` |

`rw` 操作码（`src/ppu.cpp:641-645`）：

| 值 | 含义 | 附加约定 |
|---|---|---|
| `0` | CPU 读（DMA 的读半） | `addr` = 被读地址，`data` = 读到的字节 |
| `1` | CPU 写 `$2004`（DMA 的写半） | `addr` = 0x2004，`data` = 写入字节 |
| `2` | DMA burst **开始** | `addr` = 页基址（`V<<8`），`data` = 页号 `V` |
| `3` | DMA burst **结束** | `addr` = 0，`data` = 0 |

⚠️ **三个必须知道的采样约定**（`src/ppu.cpp:641-645`）：

1. `cpu_ts` / `ppu_dot` / `ppu_sl` 都在**总线操作完成之后**采样
   （`X6502_DMR` / `X6502_DMW` 各自经 `ADDCYC` 计一个 CPU 周期）。
2. `oamaddr_before/after` **夹住**该次操作本身。
3. `ppu_dot` / `ppu_sl` 取自 `ppur.status`，是**架构精确态**——不绑 PC、也不绑
   取指指针（仓库纪律 11：绑推测态会让出口条件在第 0 步误命中）。

**⚠️ 最关键的一条：`trace.bin` 只记录 OAM DMA 的总线事件，别无其它。**
写 `trace.bin` 的调用点只有 4 处，全部在 DMA 路径上
（`ppu.cpp:915` / `:925` / `:929` / `:935`）。它**不是**通用 CPU 总线轨迹。
因此 T1 卡里「对比 `_ppu`（PASS）vs `_oam`（FAIL）总线轨迹」这一步
（`docs/plans/long-term-evolution/T1-Mapper与DMA.md:163`）**用现有探针满足不了**
——需要新探针或换取证方法。见 §4.5。

**体积自检（离线解析时的硬判据）**：`[读码]`

```
每次 burst = 1(start) + 256×2(每字节一读一写) + 1(end) = 514 条记录
trace.bin 大小 = 8 + 24 × 514 × burst 数      （未触顶时）
```
上限 `OAMBUS_TRACE_CAP_BYTES = 256 MiB`（`ppu.cpp:704`）。触顶后**丢弃后续记录**
并在 stderr 提示，`spram_diff.log` 继续写（`ppu.cpp:772-782`）。
对照实测：`kgmqa-110` 11211 次 burst ⇒ 8 + 24×514×11211 ≈ 132 MB（未触顶），
`dma_bytes` = 11211×256 = 2.87 M（`adc45ae` 提交消息）。

### 2.3 `spram_diff.log` 格式

纯文本，**一行一条**，字段以空格分隔。行内含 UTF-8 em-dash `—`（解析时注意）。
六种行型：

| 行首标记 | 出现条件 | 字段 |
|---|---|---|
| `OAMBUS PROBE begin` | 文件初始化时（探针首次激活） | `dir=<目录> trace_cap=<字节>` |
| `OAMBUS POWER RESET` | `FCEUPPU_Power`（`ppu.cpp:1533`） | `cpu_ts=<u64> — armed=0, shadow resynced` |
| `OAMBUS ARMED` | armed 条件首次成立 | `cpu_ts dot sl oamaddr=0x%02X PPUON=0 — shadow synced, monitoring SPRAM rewrites` |
| `OAMBUS DIFF` | SPRAM 与 shadow 不一致（**每个变动字节一行**） | `event where cpu_ts dot sl oamaddr idx old new last_writer lw_idx lw_val lw_ts lw_dot lw_sl` |
| `OAMBUS STATE_LOAD` | savestate 载入 | `cpu_ts dot sl oamaddr bytes_changed_vs_shadow=<u32> (shadow resynced)` |
| `OAMBUS PROBE end` | `atexit` | `dma_bursts=<u32> dma_bytes=<u32> diff_events=<u32> armed=<0\|1> trace_bytes=<u64>` |

出处：`ppu.cpp:748-750`（begin）、`:903-906`（power reset）、`:809-813`（armed）、
`:831-842`（DIFF）、`:884-888`（state load）、`:758-761`（end）。

枚举值：

| `where` | 含义 | 出处 |
|---|---|---|
| `dma_byte` | DMA 每字节边界比对点 | `ppu.cpp:675` |
| `b2004_pre` | `$2004` 写**入口**（catch-up 返回点） | `ppu.cpp:676` |
| `b2004_read` | `$2004` 读**入口** | `ppu.cpp:677` |
| `b2003_write` | `$2003`（OAMADDR）写**出口** | `ppu.cpp:678` |
| `state_load` | savestate 载入 | `ppu.cpp:679` |

| `last_writer` | 含义 | 出处 |
|---|---|---|
| `NONE` | 无带标签写者 | `ppu.cpp:690` |
| `B2004_NEWPPU` | newppu 分支的 `$2004` 写 | `ppu.cpp:686` |
| `B2004_LEGACY` | 旧 PPU 分支的 `$2004` 写 | `ppu.cpp:687` |
| `STATE_LOAD` | savestate 整块恢复 | `ppu.cpp:688` |

**「无标签 diff」是探针的核心设计**：带标签的写者（B2004 两个分支 + state load）
会同步更新 shadow，**永远不会被报成 diff**；因此任何 `OAMBUS DIFF` 行都是
**隐式写者的直接证据**（`ppu.cpp:818-820`、`:848-850`）。

### 2.4 触发条件（判定「探针到底有没有在工作」）

这一节是接手者最需要的信息。**探针的四个触发面互不相同**，混淆会得出错误结论。

| # | 触发面 | 条件 | 产物 | 出处 |
|---|---|---|---|---|
| T1 | **探针激活** | `FCEUX11_OAMBUS_PROBE` 非空 | 两个文件被创建 | `ppu.cpp:625-631, 723-753` |
| T2 | **DMA 轨迹记录** | CPU 写 `$4014`（B4014），**不看 armed** | `trace.bin` 追加 514 条 | `ppu.cpp:1502-1507` |
| T3 | **SPRAM 改写监视** | 需 **armed** 成立 | `spram_diff.log` 的 DIFF 行 | `ppu.cpp:801-816, 821-846` |
| T4 | **savestate 载入** | `FCEUPPU_LoadState` 被调 | `STATE_LOAD` 行 | `ppu.cpp:870-891`；`src/ppu_state.cpp:52-56` |

**armed 条件（架构精确态、sticky）** `[读码]`：

```cpp
// src/ppu.cpp:801-816
if (s_oambus_armed) return;
if (PPU[3] == 0x36 && (PPU[1] & 0x18) == 0) {   // OAMADDR==0x36 且 PPUON==0
    s_oambus_armed = true;
    std::memcpy(s_oambus_shadow, SPRAM, 0x100);  // 从此刻起 shadow 与 SPRAM 同步
    ... 写 "OAMBUS ARMED" 行 ...
}
```

- `PPU[3]` = **OAMADDR**（`$2003`），`PPU[1] & 0x18` = **PPUON**
  （BG + sprite 使能，`ppu.cpp:62`）。`[读码]`
- armed 一旦成立**不再复位**（除 `FCEUPPU_Power` 的 power reset，`ppu.cpp:896-908`）。
- armed 的检查点在：`oambus_compare`（4 个比对点）与 `oambus_tag_spram_write`
  （每次带标签写之后）。`[读码]`
- **未 armed 时 `oambus_compare` 直接 return**（`ppu.cpp:826`）⇒ 未 armed 期间
  SPRAM 被改写**不会留下任何记录**。

**SPRAM 比对的 4 个比对点** `[读码]`：

| 比对点 | 位置 | 说明 |
|---|---|---|
| `$2003` 写出口 | `ppu.cpp:1348` | OAMADDR 改写后 |
| `$2004` 读入口 | `ppu.cpp:991` | **返回值出来之前**抓 diff（让「读数不可靠」与「SPRAM 被改」可对上时刻） |
| `$2004` 写入口 | `ppu.cpp:1354` | catch-up 返回点语义：抓「上次比对之后、本次带标签写落地之前」的隐式改写 |
| DMA 每字节边界 | `ppu.cpp:922` | 在本字节的带标签写落地**之前**比 |

**带标签写者（3 个，永不报 diff）** `[读码]`：

| 写者 | 位置 |
|---|---|
| B2004 newppu 分支 | `ppu.cpp:1365`（`oambus_tag_spram_write((uint8_t)PPU[3], V, OAMBUS_WR_B2004_NEWPPU)`） |
| B2004 legacy 分支 | `ppu.cpp:1371`、`:1375`（`OAMBUS_WR_B2004_LEGACY`） |
| savestate 整块载入 | `ppu.cpp:870`（`oambus_probe_notify_state_load`，由 `src/ppu_state.cpp:55` 调用） |

**探针不改行为** `[读码]`：B4014 的探针分支与原循环逐字节等价
（先读后写、同样的 `X6502_DMR` / `X6502_DMW` 与计费），见 `ppu.cpp:1499-1511`
的注释与 `:919-932` 的实现；探针关闭时走原循环（`ppu.cpp:1509-1510`）。
`adc45ae` 提交消息记录「静默态 ctest 34/34 全绿，零行为变化」。

### 2.5 复现步骤

```powershell
$ErrorActionPreference = 'Stop'
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 1) 构建 runner —— f11qa-rom-runner 是 f11qa crate 的 Cargo [[bin]]，
#    不是 CMake 目标，产物在 src\rust\target\…\debug\ 下
#    （见 docs/tech/f11qa-runner-protocol.md:172）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo build -p f11qa --bin f11qa-rom-runner"

# 2) ROM 必须先落到 tests/fixtures\
#    mirror_path = bisqwit/cpu_dummy_writes_oam.nes
#    → 本地路径 = tests/fixtures/cpu_dummy_writes_oam.nes
#    （rom_runner.rs:388-392 的 rom_local_path 只取 basename）
#    取法：python scripts/fetch_roms_from_mirror.py
#    缺 ROM 时 runner 打印 hint 并 exit 2（rom_runner.rs:227-236）

# 3) 开探针跑（--frames 决定窗口；tests.json 预算是 300，
#    D4 冒烟用的是 2000 帧）
$env:FCEUX11_OAMBUS_PROBE = "D:\Project\FCEUX11\build\oambus-050"
& src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe `
    --kgmqa-id kgmqa-050-cpu-dummy-writes-bisqwit `
    --tests-json tests/tests.json `
    --frames 2000
$LASTEXITCODE
$env:FCEUX11_OAMBUS_PROBE = $null      # 记得清掉，否则后续所有运行都会开探针
```

要点：

1. **`f11qa_blargg_runner` 必须在 PATH 上**。`f11qa-rom-runner` 是 dispatcher，
   真正跑 ROM 的是它 spawn 出来的 `f11qa_blargg_runner`
   （`rom_runner.rs:35`、`:201-219`）。spawn 用 `std::process::Command`，
   **继承父进程环境** ⇒ `FCEUX11_OAMBUS_PROBE` 会自动传到子进程。`[读码]`
2. **ROM 必须真实到位**。`vendor_state: vendored` 但文件缺失时，runner 走
   exit 2（缺 ROM）而不是跑测试——这会伪装成「探针没抓到东西」。
   **判据：先看 runner 的 exit code 与 stderr，别把 exit 2 当阴性证据。**
3. **退出码语义**（`rom_runner.rs`）：0 = PASS/skip；1 = 测试跑完但 FAIL；
   2 = 前置检查失败（缺 ROM / 未知 protocol）；3 = 环境失败（spawn 不到 runner）。
4. **探针的收尾行是唯一自证**：`OAMBUS PROBE end dma_bursts=… dma_bytes=…
   diff_events=… armed=… trace_bytes=…`。没有这行 = 探针没激活（env 没传下去
   或进程被提前杀掉），**不能**解读为阴性结果。
5. **别与构建/测试任务并行**（会污染测量，r54 ②）。

---

## 3. 已有取证结论

### 3.1 结论表

| 项 | 内容 | 强度 | 出处 |
|---|---|---|---|
| 探针已落地 | `FCEUX11_OAMBUS_PROBE`，默认静默，提交 `adc45ae` | `[实测]` | `adc45ae`；v2.0.2 计划 §十 r6 |
| **阴性证据（050）** | kgmqa-050 失败窗口（**2000 帧**）内 **0 次 DMA burst、0 次 SPRAM 改写** | `[实测]` | 同上 |
| **阴性结论** | 历史报告的「SPRAM[0x36] 非 B2004 改写」**未复现** | `[实测]` | 同上 |
| **落点改向** | 0x06 不再指向「OAM 内容损坏 / OAM DMA 路径」，改指 **`$2004` 读路径（newppu `spr_read` 合成读）** | `[实测]`（现象）+ `[读码]`（落点定位） | 同上 |
| 修复立项 | **待业主裁决**（本轮未做、也未否决） | — | v2.0.2 计划 §十 r6 / r7 |
| **阳性对照（110）** | kgmqa-110（`dma_sync_test_v2`，回归哨兵）轨迹：**11211 次 DMA burst / 2.87 M 字节 / diff = 0** | `[实测]` | `adc45ae` 提交消息 |
| 静默态无回归 | 探针关闭时 ctest **34/34** 全绿，零行为变化 | `[实测]` | `adc45ae` 提交消息 |
| T1 卡步骤更新 | 050 的步骤「现成（探针对比两条路径总线轨迹）」 | `[历史]` | v2.0.2 计划 §十 r7；T1 卡 §⑥ |

### 3.2 与历史报告的差异

**历史报告**（`docs/history/surveys/cpu_bucketB/stepB_investigation_2026-08-04.md`，
基线 commit `60136cc`）当时的实测：`[历史]`

| 历史观察（2026-08-04，3000 帧） | 数值 |
|---|---|
| W2004（OAM 写）总数 | 36,933 次 |
| W2005（scroll 写） | 400 次 |
| W2006（PPU addr 写） | 1,033 次 |
| R2004（OAM 读）总数 | 4,960 次 |
| 失败错误码 | 0x06（`oam_read_test` 失败） |

历史的两条核心结论：

1. §2.3.1「A2004 读返回 SPRAM[0x36] 实时值」：同一 CPU ts、相差 2 个 PPU dot
   的两次读，`ret == oam36` 恒成立 ⇒ **A2004 路径本身正确**（地址同一性
   `&SPRAM[36] == &SPRAM[0x36] == 1`），但 **SPRAM[0x36] 在 2 个 PPU dot 窗口内
   被改写**。
2. §2.3.2「W2004 在该 2-cycle 窗口内无 oamaddr=36 写」⇒ 改写**来自 B2004
   之外**，但 §2.3.3 的 grep 证明 **B2004 是 SPRAM 的唯一写入点**，写者未定位
   （§2.3.5：最可能候选是「PPU 的 per-cycle state update 在 PPUON=0 状态下的
   隐式 secondary OAM 操作」）。
3. §2.3.4 假设排除表里已有一行：**「DMA 写 | 探针显示 B4014（OAM DMA）未被
   调用；test 不触发」**。

**⇒ 两次独立探针互相印证**：D4（2026-11-06）与桶 B（2026-08-04）在**不同
代码基线、不同探针实现**上得出同一事实——**这个 ROM 根本不触发 OAM DMA**。
`[实测]` + `[历史]` 交叉支持。

**⇒ 差异的真正落点**：历史报告在 2026-08 记录的是「**SPRAM 被谁改了**」
（一个 writer 身份问题）；D4 探针问的是「**SPRAM 有没有被非 B2004 的路径改
写**」，在 2000 帧窗口内答案是**没有**。两者不是互相矛盾，而是**同一个悬案
在两个不同代码基线上的两次否定**——2026-08 之后 PPU 侧经历过桶 C 修复
（`863e9d7`）与 `ppu.cpp` 2304→800 的拆分重构（`ppu_rendering.cpp`），
**历史观察所依据的那条隐式写路径，在今天的代码里可能已不复存在**。
`[读码]`（基于 `git log -- src/ppu.cpp` 的重构轨迹 + 桶 C 修复记录）

### 3.3 ⚠️ 证据的存放状态（接手者必须知道的一个缺口）

**原始探针产物没有进仓库。** `git ls-files` 搜 `spram_diff` / `trace.bin` /
`oambus` **零命中**；`docs/history/surveys/` 下也没有 D4 探针的产物归档。
`[实测]`

⇒ §3.1 的所有数字目前**只有两处二手记录**：
① 提交 `adc45ae` 的 commit message；② v2.0.2 计划 §十 r6 的表格单元格。

**这意味着**：本轮结论**无法在仓库内离线复核**，只能重跑（§2.5）复现。
下一个接手的人若要引用这组数字，应当**重跑一次并把 `spram_diff.log` 与
`trace.bin` 归档到 `docs/history/surveys/`**（或类似目录），否则同一问题会被
第三次「重新摸索」。

---

## 4. 下一步的调查入口：`$2004` 读路径

> ⚠️ **本节全部是 `[读码]` 推断，没有实测支撑。** 每条都标了「需 RE 验证」。

### 4.1 先纠正一个寄存器编号

`$2004` 是 **OAMDATA（OAM 数据寄存器）**，**不是** VRAM 地址寄存器。
VRAM 地址寄存器是 `$2006`（PPUADDR）；OAM 地址寄存器是 `$2003`（OAMADDR）。
本仓库的 `PPU[]` 数组按 `$2000 + n` 索引：`PPU[0]`=PPUCTRL、`PPU[1]`=PPUMASK、
`PPU[2]`=PPUSTATUS、`PPU[3]`=OAMADDR（`src/ppu.cpp:54-68`、`:1344`）。`[读码]`

### 4.2 `A2004` 的三条分支

`src/ppu.cpp:988` `static DECLFR(A2004)`：

| 分支 | 条件 | 返回值 | 行号 |
|---|---|---|---|
| **① newppu + 渲染期（合成读）** | `newppu && ppur.status.sl < 241 && PPUON` | `spr_read.ret` —— 由状态机**合成** | `ppu.cpp:993-1123` |
| **② newppu + 非渲染期（直读）** | `newppu && !(sl<241 && PPUON)` | `SPRAM[PPU[3]]` —— **直接读** | `ppu.cpp:1124-1131` |
| **③ 旧 PPU** | `!newppu` | `PPUGenLatch` | `ppu.cpp:1132-1135` |

分支 ① 内部还有一层：dot 0..63 期间直接返回 `0xFF`（32 字节 OAM buffer 初始化）
（`ppu.cpp:997-998`），dot ≥ 64 才进状态机。

### 4.3 「合成读」的含义

分支 ① 的返回值**不是从 SPRAM 数组直接取**的，而是由一个**按 dot 推进的
sprite 评估状态机**算出来的： `[读码]`

```cpp
// src/ppu.cpp:1000-1001
for (int i = spr_read.last; i != ppur.status.cycle; ++i) {
    if (i < 256) { switch (spr_read.mode) { case 0..4: ... } }   // 逐 dot 推进评估
    else if (i < 320) { ... }                                     // secondary OAM 装配
    else { ...; break; }                                          // dot ≥ 320 提前退出
}
spr_read.last = ppur.status.cycle;     // ppu.cpp:1110
PPUGenLatch = spr_read.ret;            // ppu.cpp:1121（open-bus 衰减刷新）
return spr_read.ret;                   // ppu.cpp:1122
```

状态结构 `SPRITE_READ`：`num / count / fetch / found / found_pos[8] / ret /
last / mode`（`src/ppu_class.h:76-95`），全局实例 `src/ppu.cpp:83`，
savestate 持久化 `src/ppu_state.cpp:77-96`。

**「合成读」= 返回值由这个状态机按 dot 递推合成，而非 `SPRAM[PPU[3]]` 直读。**
`mode` 0→4 是 sprite 评估的五个阶段（扫描 64 个 sprite / 取 8 个 / 溢出检测 /
HBLK 段读 OAM[n][0]）。

### 4.4 三个候选落点（读码推断，需 RE 验证）

**⚠️ 先看这条结构性事实，它决定了后面所有事** `[读码]`：

> D4 探针的 armed 条件要求 `PPUON == 0`（`ppu.cpp:803`），而 `PPUON == 0`
> ⇒ `A2004` **必然走分支 ②（直读）**，**永远到不了分支 ①（合成读）**。
>
> **⇒ 现有 D4 探针在架构上就观测不到「合成读」路径的行为。**
> 落点既然改向合成读，**第一步必须是扩展探针的覆盖面**，而不是直接改 PPU。

在此前提下，三个候选落点：

**候选 A：合成读的推进量依赖「读节奏」** `[读码]`

`spr_read.last` **只在 `A2004` 内被更新**（`ppu.cpp:1110`），除此之外全仓库
**无第二处写入**（grep `spr_read.<field> =` 全量核对：写入点集中在
`ppu.cpp:998-1110`，另有 `spr_read.reset()` 于 `src/ppu_rendering.cpp:1565`
——只在 `new_ppu_reset` 即复位后第一帧触发一次）。
`spr_read.last` **不跨帧清零**。

⇒ 合成读的语义是「**按读事件推进**的评估」，而不是「PPU 连续评估」。
一次 `$2004` 读会一口气把 `last → cycle` 的所有 dot 跑完；
若两次读相隔 > 320 dot，循环在 `i ≥ 320` 的 `else` 分支 `break` 提前退出
（`ppu.cpp:1102-1107`），中间的 dot 被跳过；
若 `spr_read.last > 当前 cycle`（跨扫描线回退），循环从 `last` 递增到 320 就
退出，实际推进量少于真实经过的 dot 数。

> **需 RE 验证**：bisqwit 的 `oam_read_test` 读 `$2004` 的节奏是什么？
> 合成值是否随读节奏变化？与硬件「渲染期读 `$2004` 返回 secondary OAM
> 快照」(`docs/tech/ppu.md:86-87`) 的语义差多少？

**候选 B：dot 0..63 的 `0xFF` 早退** `[读码]`

`ppu.cpp:997-998`：`ppur.status.cycle < 64` 时无条件 `return 0xFF`，
**不推进 `spr_read.last`**。⇒ 在帧首 64 个 dot 内连续读 `$2004`，
`spr_read.last` 仍停在旧值，下一次进入状态机时会跨过一个很大的 dot 区间
（叠加候选 A 的提前退出）。且 `PPUGenLatch` 在这条早退路径上**不被更新**
（`return` 直接返回，`ppu.cpp:998` 没有 `PPUGenLatch = ...`）⇒ open-bus 锁存
在该路径上保持旧值。

> **需 RE 验证**：测试是否在帧首 64 dot 内读 `$2004`？

**候选 C：`PPUGenLatch` 刷新的口径** `[读码]`

分支 ① 的 `PPUGenLatch = spr_read.ret`（`ppu.cpp:1121`）与分支 ② 的
`PPUGenLatch = ret`（`ppu.cpp:1129`）都刷新了 open-bus 锁存；
但分支 ① 的 dot<64 早退路径**没刷**（见候选 B）。
若 `oam_read_test` 用「先写 `$2002` 再读 `$2004` 再读某个 open-bus 寄存器」
的组合读法，锁存陈旧会污染读数。

> **需 RE 验证**：这三条与 049（`$2007` 读缓冲）共享 `PPUGenLatch` 这一个全局
> —— 见 §5 的依赖分析。

### 4.5 最省力的切入点（建议）

**建议从这里切入（而不是直接改 PPU）**：

> **在 `A2004` 分支 ① 的 `return spr_read.ret;`（`src/ppu.cpp:1122`）之前，
> 同时记录两个值：`spr_read.ret`（合成读返回值）与 `SPRAM[PPU[3]]`（直读
> 参考值），并记录 `spr_read.last` 的前值与本次循环的实际迭代次数。**
>
> **理由**：一次运行就能把「0x06 是合成读模型错」与「0x06 是 OAM 内容错」
> 这两个假设**一次性切开**——若两者相等，则合成读不是问题，落点必须再往
> 别处走；若不等，直接得到错值样本。
>
> **改动面**：仅 `src/ppu.cpp:1121-1122` 一处加记录 + 一处 env 门，
> 与现有 D4 探针同一套 env 开关风格，**不碰生产行为**（门控内只在探针开启时
> 记录）。

**配套的前置动作**（否则做不了上面这步）：

1. **放宽/增开 armed 条件**。现有 armed 强制 `PPUON == 0`（`ppu.cpp:803`），
   把观测锁死在分支 ②。需要一个允许 `PPUON != 0` 的臂（**改探针，不改 PPU**）。
2. **补齐 T1 卡第 1 步的取证手段**。T1 卡 §⑥ 步骤写的是「对比 `_ppu`（PASS）
   vs `_oam`（FAIL）总线轨迹」（`T1-Mapper与DMA.md:163`），
   但 `trace.bin` **只记录 DMA 事件**（§2.2）⇒ 这一步**当前无工具可用**。
   要么新写一个通用 CPU 总线轨迹探针，要么把这一步改写成「对比两个 ROM 的
   `$2004` 读序列」——后者更省力，且与落点改向后的方向一致。
3. **归档探针产物**（§3.3 的缺口）。下一轮跑完必须把
   `spram_diff.log` + `trace.bin` 落进 `docs/history/`，否则结论再次不可复核。

### 4.6 与「历史观察」的对照表（下一轮要填的空）

| 历史观察（2026-08-04） | D4 实测（2026-11-06） | 待验证的差异来源 |
|---|---|---|
| SPRAM[0x36] 在 2 个 PPU dot 内被改写 | 2000 帧内 0 次 SPRAM 改写 | 桶 C 修复 / `ppu.cpp` 拆分重构？基线不同？观察窗口不同（3000 vs 2000 帧）？ |
| 写者未定位（疑「PPUON=0 下的隐式 secondary OAM 操作」） | 无带标签 diff ⇒ 无隐式写者 | 若历史观察为真，则今天的代码里已无此路径 |
| 「探针显示 B4014 未被调用；test 不触发」 | 0 次 DMA burst | **一致**（两次独立探针互证） |

---

## 5. 与其它长期难题的边界

> 只陈述事实与依赖关系，**不做修复立项决策**。

### 5.1 三项的定位

| 项 | kgmqa id | ROM | 主题 | 层 | 当前状态 |
|---|---|---|---|---|---|
| **038** | `kgmqa-038-instr-misc-blargg` | `blargg/cpu/instr_misc.nes`（L2：`instr_misc_03_dummy`） | CPU 数据总线锁存（open bus） | Core | **defer**（2026-09-28 由 T0 移入 T3） |
| **049** | `kgmqa-049-ppu-read-buffer-bisqwit` | `bisqwit/test_ppu_read_buffer.nes` | `$2007` 读缓冲 + open-bus 衰减 | Core | TODO（★5，2–4 周） |
| **050** | `kgmqa-050-cpu-dummy-writes-bisqwit` | `bisqwit/cpu_dummy_writes_oam.nes` | OAM DMA dummy write → **实为 `$2004` 读路径** | Core | TODO（★3，4–7 天） |

出处：T3 卡 §3′/§⑨（`docs/plans/long-term-evolution/T3-PPU读缓冲与外设时序.md`）、
T1 卡 §⑥（`T1-Mapper与DMA.md`）、长期演进主报告 §八。

### 5.2 共享与独立（事实陈述）

**共享「open bus / 总线模型」前提的部分** `[读码 + 历史]`：

1. **049 与 050 共享 `PPUGenLatch` 这一个全局 open-bus 锁存**。
   `$2004` 读刷新它（`ppu.cpp:1121` 合成读 / `:1129` 直读），
   `$2007` 读也刷新它（`ppu.cpp:1217`），`$2002` 读返回它（`ppu.cpp:973-974`）。
   ⇒ **改 050 的 `$2004` 读路径必然触碰 049 的读缓冲语义**。
   （`docs/tech/ppu.md:82-87` 亦记载 `$2007` 缓冲 / palette 高 2 位来自
   `PPUGenLatch`。）
2. **038 与 049 共享「CPU 总线锁存活起来」这一层**：038 的 S0 是「让 `DB` 活起来
   —— 每次总线访问记录它放到总线上的值，`::ANull` 回读它」
   （长期演进主报告 §三③ 的 S0 分解）；主报告同时写明「038 的 S0 有可能顺带给
   049 铺路」。038 已 defer；049 不必等 038，可直接开。
3. **038 与 049 共享 open-bus **衰减**这个模糊面**：049 的 M3（open-bus 位衰减）
   与 038 的衰减速率是同一个「模型模糊」问题；049 的调查（v2.0.2 §十 r7 / D3）
   已确认 **049 对衰减速率零约束** ⇒ M3 defer。038 的衰减同样未被任何用例约束。

**彼此独立的部分** `[历史]`：

1. **038 与 050 无代码耦合**：038 的阻塞点是 CPU 侧未映射读的 open-bus（`DB`
   恒为常量），050 的阻塞点在 PPU 侧 `$2004` 读路径。T1 卡 §依赖行写明
   「050 与 defer 中的 038 同属 dummy 语义，若将来做 038-S0+S1 可先于 050」——
   这是**排期建议**，不是代码依赖。
2. **050 与 051（IO 空间取指）无关**：051 是 bus dispatch 不区分取指 vs 数据访问
   （T1 卡 §⑦），与 OAM 读路径不共面。
3. **049 与 050 共用 `PPUGenLatch` 但触发面不同**：049 卡在 `$2007` 的缓冲状态机
   与 VRAM 增量；050 卡在 `$2004` 的 sprite 评估合成。共用的是一个全局变量，
   不是同一条代码路径。

### 5.3 一条对 050 立项有利的既成事实

`kgmqa-110-dma-sync-test-v2-rahsennor`（`dma_sync_test_v2`）是 **050 的回归哨兵**
（T1 卡 §⑥ 验收：「kgmqa-050 PASS；`dma_sync_test_v2`（kgmqa-110）保持 PASS」）。
D4 探针已在 kgmqa-110 上跑出 **11211 次 DMA burst / 2.87 M 字节 / diff = 0**
（`adc45ae` 提交消息）⇒ **探针在「DMA 真的发生」的条件下是有牙的**
（能记录、能计数、shadow 同步无误报）。`[实测]`

⇒ 050 的阴性证据不是因为「探针坏了」——**阳性对照已排除这个解释**。

---

## 6. 不确定项清单

### 6.1 有实测数据支撑 `[实测]`

| 结论 | 出处 |
|---|---|
| 探针已落地、默认静默、提交 `adc45ae` | commit `adc45ae` |
| kgmqa-050 失败窗口 2000 帧：**0 次 DMA burst、0 次 SPRAM 改写** | 同上；v2.0.2 §十 r6 |
| 历史报告「SPRAM[0x36] 非 B2004 改写」**未复现** | 同上 |
| kgmqa-110：**11211 次 DMA burst / 2.87 M 字节 / diff = 0**（阳性对照） | 同上 |
| 探针静默态 ctest **34/34** 全绿、零行为变化 | 同上 |
| 探针产物**未进仓库**（`git ls-files` 零命中） | 本轮 `git ls-files` |
| `cpu_dummy_writes_oam` 失败码 0x06 = `oam_read_test` 失败 | 历史文档 §2.3 / §3.3 |

### 6.2 读码推断，**未实测** `[读码]`

| 推断 | 依据 | 需验证方式 |
|---|---|---|
| `trace.bin` 只记录 DMA 事件，非通用总线轨迹 | 写点仅 4 处，全在 DMA 路径（`ppu.cpp:915/925/929/935`） | grep 复核（已做）+ 跑一次看字节数 |
| armed 条件（`PPU[3]==0x36 && PPUON==0`）把观测锁在分支 ② | `ppu.cpp:803` + `:993` / `:1124` | 读码推得；跑一次确认 ARMED 行出现时的读走直读分支 |
| `spr_read.last` 只在 A2004 内更新、**不跨帧清零** | grep `spr_read.<field> =` 全量核对 + `ppu_rendering.cpp:1565` | 读码推得；需加日志实测 |
| 合成读的推进量依赖**读节奏**（> 320 dot 提前 break；`last > cycle` 时推进不足） | `ppu.cpp:1000-1008` | 需记录迭代次数实测 |
| dot < 64 早退路径**不更新 `PPUGenLatch`、不推进 `spr_read.last`** | `ppu.cpp:997-998` | 读码推得；需实测 |
| `PPUON` / `PPU[3]` 的寄存器映射 | `ppu.cpp:54-68`、`:1344` | 读码推得 |
| 2026-08 历史观察所依据的隐式写路径**可能已不复存在** | 桶 C 修复 `863e9d7` + `ppu.cpp` 拆分重构的 git 轨迹 | 需按 §4.6 对照表实测 |

### 6.3 完全未知 / 无数据 `[未确定]`

> **2026-11-06 复跑已解开其中 3 项**（见下方划掉项与 `probe_2026-11-06/` 产物目录）：
> ① 原始产物内容 —— 已复跑并归档文本汇总；② 冒烟是否 armed 过 —— **实测 `armed=1`**，
> 「0 次 SPRAM 改写」因此**不是平凡真值**；③ `cpu_dummy_writes_ppu` 对照 —— 未跑，
> 但**换成了 kgmqa-110 做阳性对照**（`dma_bursts=75164`，证明探针在真实 DMA 下确实在记录）。

| 未知项 |
|---|
| ~~原始 `trace.bin` / `spram_diff.log` 的内容~~ → **已解**：`spram_diff.log` 文本汇总归档于 `probe_2026-11-06/`；`trace.bin` 因 110 对照在 2000 帧内写满 256 MB 上限而不适合入库，改由复现命令重现 |
| ~~D4 那次冒烟具体跑的是不是 `cpu_dummy_writes_oam.nes`~~ → **已解**：本次复跑用的就是该 ROM（`mirror_path="bisqwit/cpu_dummy_writes_oam.nes"`），且**复现了 `value=0x06 status=FAIL`** |
| ~~那次冒烟是否 armed 过~~ → **已解**：2026-11-06 复跑实测 **`armed=1`**（`OAMBUS ARMED cpu_ts=90192 dot=152 sl=248 oamaddr=0x36 PPUON=0`）。⇒ r6 的「0 次 SPRAM 改写」是**有效阴性证据**，不是「从未 armed」的平凡真值。⚠️ **r6 未记录 armed 状态**——这一条是复跑补上的 |
| ~~`cpu_dummy_writes_ppu`（PASS 对照组）在同一探针下的读数~~ → **未跑该 ROM**，改用 kgmqa-110（`kgmqa-110-dma-sync-test-v2-rahsennor`）作阳性对照：`dma_bursts=75164 / dma_bytes=19241984 / armed=1 / status=PASS` ⇒ 探针在真实 DMA 场景下确实在记录 |
| bisqwit `oam_read_test` 的**读 `$2004` 的节奏与时刻分布**（决定候选 A/B/C 哪个成立） |
| `oam_read_test` 到底因为**哪一次读**失败（4096 次迭代里第几次）——**这仍是最关键的空白** |
| 2026-08 历史探针的原始日志（同样未归档） |
| 硬件在「渲染期读 `$2004`」时的精确语义（本页引用的 `docs/tech/ppu.md:86-87` 是仓库既有结论，未复核硬件原文） |

---

## 7. 相关文档与源码索引

**本页引用的文档**

| 文档 | 用途 |
|---|---|
| `docs/plans/FCEUX11-v2.0.2_DEBUG与性能优化计划.md` §十 r6 / r7 | D4 结论的原始出处（变更记录表格） |
| `docs/plans/long-term-evolution/T1-Mapper与DMA.md` §⑥ | 050 执行卡（步骤 / 验收 / 风险） |
| `docs/plans/long-term-evolution/T3-PPU读缓冲与外设时序.md` §3′ / §⑨ | 038 / 049 执行卡 |
| `docs/plans/long-term-evolution/FCEUX11-v1.18_F11QA残留精度长期演进计划.md` §三③ / §八 | 038 的完整论证与难度重估；长期难题总表 |
| `docs/history/surveys/cpu_bucketB/stepB_investigation_2026-08-04.md` | **历史深查报告**（SPRAM[0x36] 观察的原始出处，基线 `60136cc`） |
| `docs/tech/f11qa-runner-protocol.md:167-190` | runner 调用骨架与产物路径 |
| `docs/tech/precision.md:33-34` | `cpu_dummy_writes_oam` 在已知失败面里的位置 |
| `docs/tech/ppu.md:82-90` | `$2002/$2004/$2007` 的 quirk 速查（open-bus / 合成读 / 读抑制） |
| `docs/tech/gba_probe_howto.md` | 探针 HOWTO 的同族写法（本页的结构参照） |
| `docs/history/surveys/oam_dma_dummy_write/probe_2026-11-06/` | **本链的复跑产物**（2026-11-06）：`spram_diff.log` 探针文本汇总 + `README.md`（复现命令、050 阴性读数、110 阳性对照读数）。⇒ §6.3 的「armed 过否」「原始产物内容」两问已由此解开 |

**本页引用的源码锚点**

| 位置 | 内容 |
|---|---|
| `src/ppu.cpp:600-624` | D4 探针设计 doc comment（事实源） |
| `src/ppu.cpp:625-639` | env 开关（一次性 static bool） |
| `src/ppu.cpp:641-691` | trace 记录结构 + 两组枚举 + 名称表 |
| `src/ppu.cpp:693-712` | 探针状态变量 |
| `src/ppu.cpp:714-716` | `oambus_now_ts()` |
| `src/ppu.cpp:723-753` | 文件初始化 + 8 字节头 |
| `src/ppu.cpp:755-770` | atexit 收尾（结束汇总行） |
| `src/ppu.cpp:772-796` | trace 写入（256 MiB 封顶）+ 记录构造 |
| `src/ppu.cpp:798-816` | **armed 检查（架构精确态、sticky）** |
| `src/ppu.cpp:818-846` | SPRAM 比对与 DIFF 输出 |
| `src/ppu.cpp:848-862` | 带标签写者登记 |
| `src/ppu.cpp:864-891` | savestate 载入通知 |
| `src/ppu.cpp:893-908` | power reset |
| `src/ppu.cpp:910-937` | DMA burst 三件套（start / byte / end） |
| `src/ppu.cpp:988-1136` | **`A2004` 三条分支（合成读 / 直读 / 旧 PPU）** |
| `src/ppu.cpp:1341-1349` | `B2003` 写出口（比对点） |
| `src/ppu.cpp:1351-1380` | `B2004`（比对点 + 两个带标签写者） |
| `src/ppu.cpp:1484-1516` | **`B4014` OAM DMA（探针分支 vs 原循环）** |
| `src/ppu.cpp:1524-1533` | `FCEUPPU_Power`（power reset 挂钩点） |
| `src/ppu_class.h:76-95` | `SPRITE_READ` 结构定义 |
| `src/ppu.cpp:83` | `spr_read` 全局实例 |
| `src/ppu_rendering.cpp:1562-1567` | `spr_read.reset()` 的唯一调用点 |
| `src/ppu_state.cpp:46-56` | savestate 载入 → 探针通知（`ppu.cpp` 之外唯一触碰点） |
| `src/rust/crates/f11qa/src/bin/rom_runner.rs:35, 201-248, 388-392` | runner 常量 / spawn / ROM 本地路径派生 |
| `tests/tests.json:1510-1548` | kgmqa-050 条目 |

**相关提交**

| 提交 | 内容 |
|---|---|
| `adc45ae` | D4 探针 `FCEUX11_OAMBUS_PROBE` 落地（含冒烟证据） |
| `863e9d7` | 桶 C：`cpu_dummy_writes_ppu` 修复（对照组，PASS） |
| `4a7f7e2` | `sprdma_dmc_dma` 深模型族调查（记入有据已知限制） |
| `60136cc` | 桶 B 深查时的基线 commit（历史报告锚点） |
