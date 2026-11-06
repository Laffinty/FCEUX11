# $2007 读缓冲状态机（M0 文档）

> **归属**: v2.0.2 计划 **D3**（kgmqa-049）裁决落地的 M0 交付物；骨架出处为 D3 PLAN §3
> （v2.0.2 计划 §十 r7 引用，2026-11-07 用户裁决落地）。
> **性质**: 纯文档，零代码改动。核心结论——`$2007` 读缓冲在 newppu 路径**已 conform**，
> kgmqa-049 在 HEAD 的 FAIL 是 **harness 伪 FAIL**（预算 600 帧 < 完成窗口 1200-1300 帧），
> 修正为 2000 帧（余量 ≥35%）。
> **坐标基准**: 2026-11-07 工作树实读（`grep`/逐行核对），非记忆转述；后续代码移动以
> 各节标注的符号名为准，行号仅供当时定位。
> **覆盖面**: 本文描述 **newppu 路径**（F11QA harness 经 `kagami_bridge_set_newppu(1)`
> 启用，`f11qa_bridge.cpp:198-202`）。old-PPU 分支（`newppu == 0`）的差异集中在 §3 R16，
> 仅存档、非验证对象。

---

## 1. 范围

`$2007`（PPUDATA）读走的不是实时 VRAM 访问，而是一个 **两级延迟缓冲**：CPU 读到的是
*上一次* 装进缓冲的字节，同时 *本次* 地址的字节被装进缓冲等待下一次读。围绕这个缓冲
的周边行为——palette 旁路、open bus 锁存、`$2006` 装订、衰减、reset 语义——共同构成
一个状态机。本文逐行记录该状态机在 FCEUX11 中的实现，并如实标注**已知未实现项**。

判据来源三族（§5 附录做迁移行映射）：

| 判据 | kgmqa id | vendor 状态 |
|---|---|---|
| blargg ppu_read_buffer | kgmqa-041 | pending-vendor |
| bisqwit ppu_read_buffer（怪兽级） | kgmqa-049 | vendored（本文主角） |
| blargg ppu_open_bus | kgmqa-060 | pending-vendor |

---

## 2. 寄存器与状态变量

### 2.1 缓冲与总线

| 变量 | 语义 | 坐标 |
|---|---|---|
| `VRAMBuffer` | `$2007` 内部读缓冲：存「下次 `$2007` 读要返回的字节」 | `ppu.cpp:125` 定义 |
| `PPUGenLatch` | PPU 数据总线锁存（open bus decay 寄存器的宿主）：任何 PPU 寄存器写刷新它；`$2002` 低 5 位、`$2000-$2006` 无读功能透传、`$2007` palette 读高 2 位都从这里取 | `ppu.cpp:125` 定义 |
| `PPU_status` | `#define PPU_status (PPU[2])` | `ppu.cpp:68` |
| `PPUGenLatch_last_refresh_cycle` | 衰减时间戳：最近一次 PPU 寄存器**写**的 CPU 周期 | `ppu.cpp:379-381` |
| `PPU_OPEN_BUS_DECAY_CYCLES` | `1073864`（≈600 ms NTSC，1.789773 MHz × 0.6 s） | `ppu.cpp:476` |

### 2.2 `ppur`（`PPUREGS`，`ppu_class.h:107-156`）——五级计数器

NESdev 滚动计数器五级链 + 锁存副本（`_` 前缀）+ 派生字段：

| 成员 | 位宽 | 角色 |
|---|---|---|
| `fv` / `_fv` | 3 | fine Y |
| `v` / `_v` | 1 | nametable Y |
| `h` / `_h` | 1 | nametable X |
| `vt` / `_vt` | 5 | coarse Y |
| `ht` / `_ht` | 5 | coarse X |
| `fh` | 3 | fine X（`$2005` 第一写） |
| `s` | 1 | BG/sprite pattern table 选择（`$2000` bit 4） |
| `par` | 8 | pattern fetch 期间属性暂存 |
| `status` | — | `sl`（scanline）/ `cycle` / `end_cycle` |

关键方法（本文引用的语义）：

- `get_2007access()`（`ppu_class.h:144`）: `((fv&3)<<12)|(v<<11)|(h<<10)|(vt<<5)|ht` ——
  五级计数器折叠成 14 位 VRAM 地址。`$2007` 读/写、`$2006` 第二写的地址源。
- `increment2007(rendering, by32)`（`ppu_class.h:147-155`）:
  - `rendering == true` → `increment_vs()`（`ppu_class.h:134-142`，y 方向递增链
    `fv→vt→v`，`vt==30` 回绕进位 `v`；**不动 ht/h**）；
  - 否则 `by32` → 仅 `vt++`（INC32 粗调，ht 不动）；`by32 == false` → `ht++`，
    `ht` 溢出进位 `vt`；随后 `vt→h→v→fv` 进位链收口并掩码。
- `install_latches()`（`ppu_class.h:130`）: `_fv/_v/_h/_vt/_ht` → live 五级（`$2006`
  第二写装订点）。
- `clear_latches()`（`ppu_class.h:132`）: 存在但唯一调用点被注释
  （`ppu.cpp:941-943`，A2002 内——清了会造成 midframe glitch，见该处上游注释）。

### 2.3 地址与开关

| 变量 | 语义 | 坐标 |
|---|---|---|
| `RefreshAddr` | 当前 VRAM 地址（NESdev 的 **v**）；类成员 `Ppu::vaddr_latch_` 的全局引用别名 | `ppu_class.h:304,384-387` |
| `TempAddr` | `$2005/$2006/$2000` 写入的锁存副本（NESdev 的 **t**）；类成员 `Ppu::vaddr_` 的别名 | `ppu_class.h:303,384-387` |
| `vtoggle` | `$2005/$2006` 双写状态位（0=第一写，1=第二写）；`Ppu::vtoggle_`（`uint8_t`）的别名 | `ppu_class.h:301` |
| `DummyRead` | `$2006` 第二写后置 1（`$2007` 首读前真机有一个 dummy 读；调试日志路径消费它） | `ppu.cpp:1424` 置位，`ppu.cpp:1157` 清 |
| `INC32` | `#define INC32 (PPU[0] & 0x04)` —— `$2000` bit 2，地址增量 1/32 | `ppu.cpp:58` |
| `PPUON` | `#define PPUON (PPU[1] & 0x18)` —— BG/sprite 任一使能 | `ppu.cpp:62` |
| `newppu` | PPU 实现选择开关（本文状态机的路径选择位） | `ppu.cpp:315` |
| `CALL_PPUREAD` / `CALL_PPUWRITE` | 经 mapper 的 PPUREAD/PPUWRITE 钩子（断言非空） | `ppu.cpp:310` |
| `READPAL` / `READUPAL` | palette 读宏，带 GRAYSCALE 掩码（`& 0x30`） | `ppu.cpp:70-72` |

### 2.4 渲染期判据

`$2007` 读/写的 `increment2007` 第一个参数
`ppur.status.sl >= 0 && ppur.status.sl < 241 && PPUON`（`ppu.cpp:1207/1451`）——
可见行（0-240）且 BG/sprite 使能时按渲染期语义递增。

---

## 3. 状态迁移表

> 「渲染中」= §2.4 判据为真。除 R16 外全部为 newppu 分支。
> 「**不变量 I-1**」：`VRAMBuffer` 只在 R1/R3（读侧填充）与 R15（reset 清零）被写；
> **任何写路径（R4）与装订路径（R5/R6/R7/R8）都不触碰它**。

### R1 — `$2007` 读 · 非渲染 · 非 palette

`newppu` 且 `(addr & 0x3F00) != 0x3F00`：

1. `ret = VRAMBuffer`（返回**旧**缓冲）；
2. `RefreshAddr = ppur.get_2007access() & 0x3FFF`（五级计数器 → v）；
3. `VRAMBuffer = CALL_PPUREAD(RefreshAddr)`（装进**新**字节）；
4. `ppur.increment2007(rendering, INC32)`；`RefreshAddr = ppur.get_2007access()`；
5. `PPUGenLatch = ret`（读值上总线、刷新 latch——blargg
   `ppu_read_buffer` TEST_PPU_OPENBUS_MUST_NOT_COPY_READBUFFER (0x13) 判据，
   修复注释 `ppu.cpp:1209-1217`）。

坐标 `ppu.cpp:1160-1218`（填充点 `:1205`，latch 点 `:1217`）。

### R2 — `$2007` 读 · 渲染中

与 R1 同一代码路径，唯一差异在第 4 步：`rendering == true` ⇒
`increment2007` 走 `increment_vs()`（y 链递增，ht/h 不动）。坐标 `ppu.cpp:1207`、
`ppu_class.h:147-148`。

> **注意**: 渲染期的**取指隐式填充**未实现——返回值仍是上一次 `$2007` 读的缓冲，
> 不是最近 BG/sprite 取指字节。见 §4 U1。

### R3 — `$2007` 读 · palette 旁路

`newppu` 且 `(addr & 0x3F00) == 0x3F00`：返回值**不走缓冲**：

1. 用 **live 地址**（`get_2007access()` 后的 `RefreshAddr`，非入口时捕获的 `tmp`——
   v1.16 P2 桶 C 修复，`ppu.cpp:1169-1179`）索引 palette RAM：
   `paddr = addr & 0x1F`，`!(paddr&3)` 且 `!(paddr&0xC)` → `READPAL(0x00)`；
   `!(paddr&3)` → `READUPAL(...)`；其余 `READPAL(paddr)`
   （`$3F10/$14/$18/$1C` 镜像进 `$3F00/$04/$08/$0C`）；
2. `ret |= PPUGenLatch & 0xC0` —— palette 读高 2 位是 open bus（blargg
   ppu_open_bus test 8，注释 `ppu.cpp:1188-1200`）；
3. 缓冲仍被填充，但填充 **`addr - 0x1000`**（palette 区下方 0x1000 的镜像读）：
   `VRAMBuffer = CALL_PPUREAD(RefreshAddr - 0x1000)`（`:1201`）；
4. 同 R1 第 4/5 步（递增 + `PPUGenLatch = ret`）。

坐标 `ppu.cpp:1163-1201`。

### R4 — `$2007` 写（不污染缓冲）

`newppu` 分支（`ppu.cpp:1446-1452`）：`PPUGenLatch = V`（`:1447`）→
`RefreshAddr = ppur.get_2007access() & 0x3FFF` → `CALL_PPUWRITE(RefreshAddr, V)` →
`increment2007(rendering, INC32)` → `RefreshAddr` 同步。**全程零 `VRAMBuffer` 触碰**
（不变量 I-1）——写后紧跟的读返回写前填充的旧缓冲，与真机一致。
old-PPU 分支在 `:1453-1481`（直写 VPage/vnapage/PALRAM + `INC32` 手工增量）。

### R5 — `$2006` 第一写（`vtoggle == 0`）

`PPUGenLatch = V`（`:1408`）+ `opendecay_log_write(V)`；`TempAddr &= 0x00FF`、
高 6 位 ← `V & 0x3F`；`ppur._vt/_h/_v/_fv` 按 bit 域装订
（`:1414-1418`）。坐标 `ppu.cpp:1405-1419`。

### R6 — `$2006` 第二写（`vtoggle == 1`）——装订

`PPUGenLatch = V`；`TempAddr &= 0xFF00 | V`；**`RefreshAddr = TempAddr`**（t → v，
`:1423`）；`DummyRead = 1`（`:1424`）；`ppur.install_latches()`（锁存副本 → live
五级，`:1432`）；`vtoggle ^= 1`。**装订不填也不清 `VRAMBuffer`**（I-1）。
坐标 `ppu.cpp:1419-1436`。

### R7 — `$2005` 第一写（`vtoggle == 0`）

`PPUGenLatch = V`；`TempAddr &= 0xFFE0`、coarse X ← `V >> 3`；`XOffset = V & 7`；
`ppur._ht = V >> 3`、`ppur.fh = V & 7`。坐标 `ppu.cpp:1382-1392`。

### R8 — `$2005` 第二写（`vtoggle == 1`）

`PPUGenLatch = V`；`TempAddr &= 0x8C1F` + fine Y 位域（`(V & ~7) << 2`、
`(V & 7) << 12`）；`ppur._vt = V >> 3`、`ppur._fv = V & 7`；`vtoggle ^= 1`。
不触碰 `RefreshAddr`（直到 `$2006` 第二写才装订）。坐标 `ppu.cpp:1393-1402`。

### R9 — `$2002` 读（低 5 位透传 + vtoggle 复位 + VBL 清）

1. `ret = PPU_status | (PPUGenLatch & 0x1F)` —— VBL/sprite0/overflow 高 3 位来自
   状态寄存器，低 5 位透传 open bus latch（`:973-974`）；
2. `vtoggle = 0`（复位 `$2005/$2006` 双写状态机，`:980`）；
3. `PPU_status &= 0x7F`（VBL 清，`:981`）；
4. `PPUGenLatch = ret`（读值上总线，`:982`）。
   另有 VBL set 抑制（Step 1.2：sl240 cy340 抑制置位、sl241 cy≤1 取消 NMI，
   `:950-971`）——与读缓冲无关，存档于此函数名下。

坐标 `ppu.cpp:939-986`。

### R10 — `$2004` 读 · 渲染中（整字节刷新 latch）

`spr_read` 逐周期 OAM 状态机仿真（`:993-1123`）；`cycle < 64` → `0xFF`
（OAM 缓冲初始化段，`:997-998`）。出口 **`PPUGenLatch = spr_read.ret`**
（`:1121`，blargg ppu_open_bus test 11「$2004 读整字节刷新衰减寄存器」）。

### R11 — `$2004` 读 · 非渲染

`ret = SPRAM[PPU[3]]`；同样 **`PPUGenLatch = ret`** 整字节刷新（`:1124-1131`）。

### R12 — `$2000/$2001/$2003/$2005/$2006` 读（无读功能透传）

`ret = PPUGenLatch`（`:1138-1143`，`A200x`）。**不刷新衰减时间戳**——blargg
readme：write-only 寄存器的读返回衰减值但**不刷新**衰减（`ppu.cpp:457-461` 注释）。

### R13 — 任意 PPU 寄存器写（B2000-B2007 全员）

`PPUGenLatch = V` + `opendecay_log_write(V)`（刷新衰减时间戳）。逐条坐标：
B2000 `:1295`、B2001 `:1329`、B2002 `:1337`、B2003 `:1342`、B2004 `:1355`、
B2005 `:1385`、B2006 `:1408`、B2007 `:1447`。

### R14 — 每帧 600ms 整字节衰减

每帧入口调用 `opendecay_log_decay_check()`（`FCEUPPU_Loop`
`ppu_rendering.cpp:1181` → `:1189`；`FCEUX_PPU_Loop` `:1557` → `:1560`），内部
`ppu_latch_decay_check()`（`ppu.cpp:482-496`）：

- `PPUGenLatch == 0` → 早退（已衰减/未写过）；
- `now - PPUGenLatch_last_refresh_cycle > 1073864`（≈600 ms NTSC）→ `PPUGenLatch = 0`。

**整字节归零**，非 per-bit（见 §4 U2）。每帧粒度足够：1 帧 ≈ 16.67 ms ≪ 600 ms。

### R15 — reset（`FCEUPPU_Reset`，不清 vnapage）

`ppu_core.cpp:88-105`：`VRAMBuffer = PPU[0..3] = 0`、`PPUGenLatch = 0`、
`RefreshAddr = TempAddr = 0`、`vtoggle = 0`、`ppudead = 2`、`new_ppu_reset = true`。
**`vnapage` 不清**——mirroring 是卡带硬件属性（iNES header），reset 清它曾让
blargg ppu_read_buffer 的 `TEST_NTA_MIRRORING_FAIL_1NTA`（FAIL #14 = 0x0E）
踩空（v1.16 P2 桶 C 修复 `863e9d7`，调查页
`docs/history/surveys/ppu_bucketC/stepC_investigation_2026-08-04.md` §1）；
需要重设的 mapper 在自身 `Power()` 中覆盖（`fceu.cpp:1045` 处 mapper Power
在 `FCEUPPU_Reset` 之后运行）。

### R16 — old-PPU 分支差异注记（存档，非验证对象）

`newppu == 0` 时 `A2007` 走 `ppu.cpp:1219-1291`：palette 直连返回且不加 `0xC0`
（`:1224-1241`）、`PPUGenLatch = VRAMBuffer`（`:1249`）、渲染期增量内联
（`ScreenON||SpriteON && sl<240` 的 x/y 增量 hack，`:1268-1291`）。kgmqa-049
在 newppu 路径验证（r7 取证），本行仅记录分支存在。

---

## 4. 已知未实现项（如实记录）

### U1 — 渲染期取指隐式填充 `VRAMBuffer` 未实现

真机上，渲染期间 BG/sprite 取指会把取到的字节压入 `$2007` 读缓冲；渲染期
`$2007` 读返回的是**最近一次内部取指**的字节。本仓库：

- `grep -c "VRAMBuffer" src/ppu_rendering.cpp` → **0**（2026-11-07 实测）；
- 全仓引用 `VRAMBuffer` 的文件仅 `ppu.cpp`（状态机本体）、`ppu_core.cpp`（reset）、
  `ppu_state.cpp`（savestate 序列化）、`ppu_core.h/ppu.h`（extern）、
  `debug.cpp/debug.h`（调试器显示）——**没有任何渲染期赋值点**。

因此渲染期 `$2007` 读（R2）返回的是上一次 `$2007` 读填充的缓冲值。
kgmqa-049 两连 PASS（完成窗口 1200-1300 帧确定性复现）说明该测试子测族对这一面
**无约束**——与其对衰减速率零约束（§4 U2 / M3 defer 裁决）同族。风险敞口：
blargg ppu_read_buffer 的渲染期填充子测若 vendor 后引入，此项可能暴露。

### U2 — per-bit 衰减未实现

blargg ppu_open_bus readme：衰减寄存器是 8-bit **每位独立**的（每 bit 若
600 ms 未被刷新为 1 则衰减为 0）。本仓库是**整字节**衰减（R14：整个
`PPUGenLatch` 过期归零）。历史判据（blargg ppu_open_bus 0x03 → 0x00 PASS、
bisqwit 049 0x00）都在该近似下通过；D3 M3（衰减精度）**defer**
（2026-11-07 裁决：049 对衰减速率零约束）。已知精度债，记于此供后续 RE 引用。

---

## 5. 判据映射附录（迁移行 → 测试；明细 vendor 后补）

| 测试 | 覆盖的迁移行 | 已知子测证据 | 状态 |
|---|---|---|---|
| blargg ppu_read_buffer（kgmqa-041） | R15（vnapage）+ R5/R6（装订） | `TEST_NTA_MIRRORING_FAIL_1NTA`（0x0E，FAIL #14）→ `863e9d7` 修复 | pending-vendor |
| 〃 | R3（live 地址索引） | `TEST_PALETTE_READS_UNRELIABLE`（0x30=48，`ppu.cpp:1169-1179` 注释） | 〃 |
| 〃 | R1（`PPUGenLatch = ret`） | `TEST_PPU_OPENBUS_MUST_NOT_COPY_READBUFFER`（0x13=19，`ppu.cpp:1209-1217` 注释） | 〃 |
| blargg ppu_open_bus（kgmqa-060） | R14（600 ms 衰减） | test 3（`setb $2002,$FF` → `delay_msec 1000` → `lda PPUCTRL`；探针取证 `opendecay_probe_2026-08-05.md`） | pending-vendor |
| 〃 | R3（高 2 位 open bus） | test 8（$2007 palette 读 `DD-- ----`） | 〃 |
| 〃 | R10/R11（$2004 整字节刷新） | test 11（$2004 第三字节读刷新全部 8 bit） | 〃 |
| bisqwit ppu_read_buffer（kgmqa-049） | 全族（黑盒 `$6000` 结果字节） | `0x00 @2000 帧`；完成窗口 1200-1300（两次确定性复现，r7 取证） | vendored |
| 〃 | —— | **子测族 → 迁移行的逐条映射需反汇编子测序号，明细 vendor 后补** | 〃 |
| 预算变异 M-E（本文 §6 实测） | L-2 预算哨兵 | 600 帧 → `0x80` FAIL / 2000 帧 → `0x00` PASS：哨兵有牙 | 2026-11-07 实测 |

> 按 r53/r56/v2.0.1 P0 的三例先例（「测试与实现同错、闭环自洽」），上表每条
> 「覆盖」声明应在 vendor 时**实测验证**（变异该迁移行看判据是否转红），不得
> 靠推理断言——本附录当前只列**已发生过的**修复-判据配对，不做覆盖承诺。

---

## 6. 预算变异留证（2026-11-07 实测）

`f11qa_blargg_runner.exe --rom tests/fixtures/test_ppu_read_buffer.nes
--kgmqa-id kgmqa-049`（仓库根 cwd，`vcpkg_installed/x64-windows/bin` 前置）：

```text
===== frames=600 =====
BLARGG_RESULT: rom=test_ppu_read_buffer.nes addr=0x6000 value=0x80 diag=[0xDE,0xB0,0x61] status=FAIL duration_ms=935
EXIT600=1

===== frames=2000 =====
BLARGG_RESULT: rom=test_ppu_read_buffer.nes addr=0x6000 value=0x00 diag=[0xDE,0xB0,0x61] status=PASS duration_ms=3148
EXIT2000=0
```

`0x80` = 测试仍在跑（harness 预算截断处的运行标志，非精度失败）；
`0x00` = PASS。两档输出即 r7 记录的完成窗口（1200-1300 帧）的外侧夹逼：
600（红）与 2000（绿）证明 tests.json 的 `--frames` 预算是**有牙的哨兵**——
预算不足会以 FAIL 形态暴露而不是静默放行。
