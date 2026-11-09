# FCEUX11 v2.0.3 · S1-a 最小可复现夹具 — 实施计划

> **STATUS: 待业主拍板**（本文是 v2.0.3 计划 §二 S1-a 的实施展开；**本文不改 v2.0.3 计划的任何结论**）
> **上游工作项**：`docs/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md` §二「S1-a 最小可复现夹具」/ §2.2 / §2.4
> **基线**：`723b559`（`wip203` 分支，工作树干净）· GBA 锁测试 **249** 项 · vendor **242** 项 · `ctest -LE perf` **36** 项
> **本工作项只造门禁，不改任何生产行为**——S2/S3 的修复是独立工作项。

---

## 〇、本工作项定位

### 范围内

1. **9 条最小可复现夹具**（合成 GBA ROM），每条能**独立复现**一条已定位缺陷。
2. 每条夹具对应的**缺陷复现器**（一条必然失败/必然 panic 的断言）。
3. 让复现器**不可能被遗忘**的常驻守卫 + 让**夹具工具本身**不可能是错的端到端自检。

### 范围外（排除项）

1. **不改任何生产代码**：`src/rust/src/gba/`、`src/rust/crates/gba-core/`、`src/gba_load.cpp` 一律不动。
2. **不修任何缺陷**。SWI 兜底（`bios.rs:134`）、panic 容错、EEPROM 模型、ROM 镜像——全是 S2/S3 的活。
3. **不做 S1-b 的探针改造**（SWI 陷阱 / panic 捕获 / 输入注入 / 长窗口 / 忙等开窗）——独立工作项。
4. **不放商业卡带进仓库**（v2.0.3 计划 §〇 范围外第 2 条）。

### 继承的不变式（AGENTS.md）

| # | 不变式 | 本工作项怎么守住 |
|---|---|---|
| 1 | NES 零回归 | 出口必跑 `ctest -LE perf` 36 项全绿 + F11QA `pass_to_fail = 0` + grade ≥ B |
| 2 | GBA 长期 Alpha，不阻塞本体发版 | 本工作项不设任何本体发版门槛 |
| 3 | 变异验证 | 每条复现器都要能「把修复前的值改回去 ⇒ 转红」 |
| 4 | **不凭记忆写位级格式** | OAM 位定义、EEPROM 命令协议、GPIO 语义**全部取自 GBATEK 权威快照或本仓代码逐字**；取不到就**停下记录**，不写 |
| 5 | vendor 树补丁登记 | 本工作项**不改** vendor ⇒ 无需改 `ATTRIBUTION.md` |
| 6 | instrument-first | 改行为前先有能看见它的读数——本工作项**通篇是读数** |
| 10 | **编不过的测试等于不存在** | 每加一条测试，当场确认它**落在哪条门禁命令的路径上**并**看到它的计数**（AGENTS.md 纪律 10 + r26 反向实例） |

---

## 一、开工前的事实更正（**本工作项最重要的一节**）

> v2.0.3 计划 §一 自称「**已确认根因的事实底座**」「别照着做」。本轮对 §1.2.2 做了一次
> **行号级复核**（只读：源码逐字阅读 + 子代理交叉验证 + 主会话逐条复验），发现 **6 处断言与代码不符**。
> 其中 2 处**直接推翻了 §1.2.2 的核心结论**。
>
> **下面每条都带可复核的行号与逐字代码。复核命令见 §九.3。**

### 1.1 更正 A（**推翻**）：`bus.rs` 的 11 处 `panic!` **全部不可达**

计划 §1.2.2 把它们列为「**可被游戏触达的 panic 点**」。实际不可达。

`bus.rs:1077` `read_raw` / `bus.rs:1152` `write_raw` 的 `match` 对各自 I/O 区间**已全覆盖**，
不存在能落进各 `*_raw` 函数兜底分支的地址。逐条：

| 行号 | 所在函数 | 上游调用点与传入区间 | 结论 |
|---|---|---|---|
| `bus.rs:291` | `read_keypad_raw` | `read_raw:1100` 传 `0x0400_0130..=0x0400_0133`（4 个地址全被 match 覆盖） | 不可达 |
| `bus.rs:302` | `write_keypad_raw` | `write_raw:1166` 同上区间 | 不可达 |
| `bus.rs:343` | `read_serial_raw` | `read_raw:1101` 传 `0x0400_0120..=0x0400_012F \| 0x0400_0134..=0x0400_01FF`，match 已全覆盖并显式列出兜底区间 | 不可达 |
| `bus.rs:422` | `read_timers_raw` | `read_raw:1099` 传 `0x0400_0100..=0x0400_011F`，match 覆盖 `0x100–0x10F` + `0x110–0x11F` | 不可达 |
| `bus.rs:520` | `write_timers_raw` | `write_raw:1162` 同上区间 | 不可达 |
| `bus.rs:538` / `bus.rs:558` | `read_dma_raw` | `read_raw:1098` 传 `0x0400_00B0..=0x0400_00FF`，match 覆盖到 `0x0400_00E0..=0x0400_00FF` | 不可达 |
| `bus.rs:576` | `write_dma_raw` 内闭包 | 同上 | 不可达 |
| **`bus.rs:596`** | `write_dma_raw` | `write_raw:1161` 传 `0x0400_00B0..=0x0400_00FF`，同上全覆盖 | **不可达** |
| `bus.rs:957` | `read_lcd_raw` | `read_raw:1096` 传 `0x0400_0000..=0x0400_005F`，match 覆盖到 `0x56..=0x5F` | 不可达 |
| `bus.rs:1069` | `write_lcd_raw` | `write_raw:1159` 传 `0x00..=0x5F`，match 覆盖到 `0x56..=0x5F` | 不可达 |

**「写一个未建模的 IO 寄存器」今天会发生什么**（这是 v2.0.3 计划 §2.2 的 `gba_io_unmapped.gba` 设想）：

```rust
// bus.rs:1203-1211
0x000_4000..=0x1FF_FFFF | 0x1000_0000..=0xFFFF_FFFF => {
    tracing::debug!("write on unused memory {address:x}");
    self.unused_region.insert(address, value);
}
_ => {
    tracing::debug!(
        "write on unimplemented memory address 0x{address:08X} with value 0x{value:02X}"
    );
}
```

⇒ **写未建模地址 = 静默忽略 + 一行 debug 日志，零 panic。**
⇒ **`gba_io_unmapped.gba` 这条夹具无法证伪任何东西**（它今天就是绿的）⇒ 见 §五 的替换决定。

### 1.2 更正 B（**推翻**）：`operations.rs:896` 的 PLD `todo!` 是**死代码**

计划 §1.2.2 把它列为 panic 点。实际有**两道独立防线**挡在前面：

1. `cpu/arm7tdmi.rs:458-463` 在调用 `single_data_transfer` **之前**就拦下 `Pld`：

   ```rust
   ArmModeInstruction::SingleDataTransfer {
       kind: arm::instructions::SingleDataTransferKind::Pld,
       ..
   } => {
       self.handle_exception(ExceptionType::UndefinedInstruction);
   }
   ```

   `single_data_transfer` 的**唯一**调用点是 `arm7tdmi.rs:474`（下一个 match 臂）⇒ 被抢先匹配 ⇒
   `operations.rs:896` 永远不会执行。

2. 条件门 `cpu/arm7tdmi.rs:339-343` + `cpu/../psr.rs:74`：
   `NV => false`（ARMv3+ 保留条件）⇒ **cond = `0b1111` 的指令永不执行**。

⇒ PLD 的真实后果是「**静默 no-op**」或（`cpu/arm/instructions.rs:691` 的 `Undefined` 规则）「**跳 `0x04` 向量自旋**」，
**都不是 panic**。⇒ S2 表里「`operations.rs:896` PLD → 按 NOP 语义实现」改的是**活代码之外的死代码**。

### 1.3 更正 C（**推翻**）：`object_attributes.rs` 四处里，**只有两处是真现场**

| 计划点的行号 | 该行逐字代码 | 真实 panic 现场 | 触发输入 |
|---|---|---|---|
| `262` | `gfx_mode: value.get_bits(10..=11).try_into().unwrap(),` | **`object_attributes.rs:144`** 的 `unreachable!()` | attr0 bits10–11 = `0b11` ⇒ 进 `GfxMode::try_from` 的 `_` 分支。**262 的 `.unwrap()` 只在值 = 4 时才响，而该字段是 2 bit（0..3）⇒ 取不到 4 ⇒ 262 是死分支** |
| `265` | `obj_shape: value.get_bits(14..=15).try_into().unwrap(),` | **265 本身**（Err 源在 `object_attributes.rs:201`：`3 => Err("Prohibited ObjShape")`） | attr0 bits14–15 = `0b11` |
| `389` | `let obj_attribute0 = value[0].try_into().unwrap();` | 389（**只向上转发**） | 同上 |
| `475` | `obj_attributes[idx] = [attribute0, attribute1, attribute2].try_into().unwrap();` | 475（**只向上转发**） | 同上 |

⇒ **shape = 3 与 gfx mode = 3 是两条不同的 panic 现场**（265 与 144）⇒ 按「一个夹具不混两件事」的纪律，
**必须拆成两条夹具**。

### 1.4 更正 D：OAM 的「禁用值」表述错了

| 字段 | attr0 位 | 真机语义 | vendor 行为 |
|---|---|---|---|
| obj mode（渲染模式） | bits 8–9 | **2 = Disabled（真机的「禁用精灵」）** | ✅ 正确（`object_attributes.rs:114` 的 `ObjMode::Disabled`，渲染时 `layer_obj.rs:234-236` 跳过） |
| gfx mode | bits 10–11 | **3 = prohibited（禁止）** | ❌ panic（`144`） |
| shape | bits 14–15 | **3 = prohibited（禁止）** | ❌ panic（`265`） |

⇒ 计划 §1.2.2 写的「**真机禁用值**」不准确：**真机上是「禁止值」，不是「禁用值」**；
真机对禁止值的处理是「精灵不可见」，vendor 是「崩溃」。
⇒ **S2-a 的修复语义应当是「映射为精灵禁用」**（与 obj mode = 2 的既有处理一致），**不是「忽略」**。

### 1.5 更正 E（**追加**）：`internal_memory.rs:660` 的 `unimplemented!` **同样不可达**

`read_at`（`internal_memory.rs:617`）与 `write_at`（`:665`）的**全部**调用方只有三处，全在 `bus.rs`：

```
bus.rs:1088   self.internal_memory.read_at(address)     // 仅 0x0000_0000..=0x0000_3FFF
bus.rs:1094   self.internal_memory.read_at(address)     // 仅 0x0200_0000..=0x03FF_FFFF | 0x0800_0000..=0x0FFF_FFFF
bus.rs:1157   self.internal_memory.write_at(address, value)  // 仅 0x0000_0000..=0x0000_3FFF | 0x0200_0000..=0x03FF_FFFF | 0x0800_0000..=0x0FFF_FFFF
```

而 `read_at` 的 match 缺口（`0x0400_0000–0x04FF_FFFF` 的 I/O 区、`0x0500_0000–0x07FF_FFFF` 的
调色板/VRAM/OAM 区）**恰好落在 `bus.rs` 自己接管、不转交 `internal_memory` 的区间** ⇒ `660` 不可达。

### 1.6 更正后的结论：**可达的 panic 面只有两条**

```
OAM attr0 shape = 3      → object_attributes.rs:265  (.unwrap() on Err "Prohibited ObjShape")
OAM attr0 gfx mode = 3   → object_attributes.rs:144  (unreachable!() in GfxMode::try_from)
```

⇒ **v2.0.3 计划 §1.2.2 的「闪退」根因需要收窄**：不是「vendor 核在未建模寄存器上 panic」，
而是「**vendor 核在 OAM 的两个禁止值上 panic**」。
⇒ 这**不推翻** §1.2.2 的结论方向（闪退确实来自 vendor 核 panic + `panic = "abort"`），
但**推翻了它的证据清单**——清单里 13 条中有 11 条不可达。

### 1.7 更正 F：EEPROM **有**位级模型（计划 §1.2.4 的「无真模型」不准确）

`internal_memory.rs` 里**存在**一套位级 EEPROM 模型：

| 符号 | 行号 | 作用 |
|---|---|---|
| `is_eeprom_access(address)` | `523` | 判定是否落在 EEPROM 窗口 |
| `eeprom_write(value)` | `536` | 移入 1 位 |
| `eeprom_read()` | `548` | 移出 1 位；空闲时返回 1（ready） |
| `eeprom_interpret_command()` | `566` | 解释命令：请求位 / 方向位 / 地址宽度（6 或 14 位，按位流长度推断） |
| `eeprom_block_offset(addr, width)` | `611` | 6 位宽 ⇒ 64 块 × 8 字节；14 位宽 ⇒ 1024 块 × 8 字节 |

⇒ 计划 §1.2.4 写的「**EEPROM 没有真模型**」不准确。**真正缺的是这三条**：

| # | 真实缺陷 | 证据 |
|---|---|---|
| **E1** | **访问窗口过宽 16 MB**：`is_eeprom_access` 对 ≤16 MB 的卡返回 `0x0D00_0000..=0x0DFF_FFFF`，而真机 EEPROM 只有 `0x0D00_0000..=0x0D00_FFFF`（64 KB）。⇒ `bus.rs:1638` / `bus.rs:1674` 会把 **0x0D010000–0x0DFFFFFF 的每一次半字读写劫持进 EEPROM 位流**——那是卡的 **ROM（wait state 2）区** ⇒ **EEPROM 卡读自己的 ROM 会拿到位流垃圾** | `internal_memory.rs:523-532` |
| **E2** | **芯片容量不建模**：`buffer_size()` 对 `Eeprom` 返回 `0x8000`（32 KB），真机是 **512 B 或 8 KB** ⇒ `.srm` 尺寸恒为 32768 | `internal_memory.rs:92-98` |
| **E3** | **Flash 状态寄存器不存在**：读 `0x0E000006/7` 落进「普通读」分支，返回**存档原始字节**而不是芯片状态（ready / write-enable / erase-complete） ⇒ 任何轮询 Flash 状态的商用卡会死等 | `internal_memory.rs:631-655` |

⇒ E1 是**独立于** E2/E3 的缺陷 ⇒ 按「一个夹具不混两件事」**必须拆成独立夹具**（§五）。

### 1.8 前置取证条件（**没有它，夹具 5 与夹具 9 写不出正确断言**）

计划 §一.5 承诺「GBATEK 快照固化成本地取证基线，关键条目的摘录写进 §附录 A」。
但**当前附录 A 只有三节**（DMA 控制寄存器 / 存档 ID 串 / 卡带头布局），
**没有 EEPROM 命令协议，也没有 GPIO/RTC**。

| 需要的内容 | 用在哪 | 状态 |
|---|---|---|
| EEPROM 命令协议（请求位 / 方向位 / 地址宽度 / 数据位序 / 停止位） | 夹具 5 的断言必须按**真机协议**写，否则测的是 vendor 的实现而不是缺陷 | ❌ **缺** |
| GPIO（`0x080000C4–C9`）在「游戏尚未写入」时的真机行为 | 夹具 9 的断言 | ❌ **缺** |

⇒ **S1-a 的前置条件 P1**：补抓 GBATEK 的 **EEPROM** 与 **GPIO/RTC** 两章进计划附录 A
（沿用 §一.5 已定的路径：代理 `127.0.0.1:7897` 抓 `https://mgba-emu.github.io/gbatek/`）。
**取不到就停下来记录，不许凭记忆写**（AGENTS.md 纪律 4，本项目已因此翻车两次）。

⇒ 另有一个**顺带发现**（不属本工作项，但必须登记）：vendor 的 `OperandKind` 判别与真 ARM **相反**
（`cpu/arm/instructions.rs:796`：`let op_kind: OperandKind = op_code.get_bit(25).into();`
—— bit25 = 1 被当成立即数，而真 ARM 的 I=1 是寄存器）。
其后果是 `bios.rs:78` 的 `ADD_LR_PC = 0xE28F_E000`（注释写 `ADD lr, pc, #0`）
**在 vendor 上恰好被读成注释想表达的立即数，而在真 ARM 上会被读成 `ADD lr, pc, r0`**。
本模拟器用 stub BIOS，所以当前**不受影响**；但它说明「按真 ARM 写的常量」在本仓 vendor 上**不可默认成立**。
⇒ 登记为观察项（见 §九.4），**本工作项不修**。

---

## 二、交付物清单

### 2.1 新增文件

| 文件 | 内容 | 行数估计 |
|---|---|---|
| `src/rust/src/gba/fixtures.rs` | 夹具工具 + 9 条夹具 + 9 条复现器 + 守卫 | ~700 |

### 2.2 修改文件

| 文件 | 改动 | 理由 |
|---|---|---|
| `src/rust/src/gba/mod.rs` | **+2 行**：`#[cfg(test)] pub mod fixtures;` | 挂载新模块 |
| `.gitignore` | **+2 行**：`*.gba`（全局忽略，**不加例外**） | 机械保证「卡带不进仓库」 |
| `docs/plans/FCEUX11-v2.0.3_GBA卡带兼容性优化计划.md` | §九 记 **r2**；改写 §2.2（夹具清单）与 §2.4（出口判据）；§1.2.2 加「更正」小节 | 纪律 2「先记变更记录，再改计划」 |

### 2.3 明确不碰

`src/rust/crates/gba-core/**`（vendor）、`src/rust/src/gba/bios.rs`、`src/rust/src/gba/swi/**`、
`src/rust/src/gba/frame.rs`、`src/gba_load.cpp`、`src/state.cpp`、任何 `src/**` 的 C/C++ 文件。

---

## 三、决策点 D1 — 夹具载体形态

### 3.1 两个方案

**方案 A（推荐）· Rust 内建夹具**

夹具是 `src/rust/src/gba/fixtures.rs` 里的 Rust 函数，直接返回一个 `Vec<u8>` 的卡带镜像；
锁测试在内存里构造机器、跑程序、断言读数。**不产生任何二进制文件。**

**方案 B（v2.0.3 计划 §2.2 的字面写法）· Python 生成 `.gba` 文件**

`tests/fixtures/generate_test_roms.py` 新增生成函数 → 产出 `tests/fixtures/gba/gba_*.gba`
→ `.gitignore` 加 `*.gba` + `!tests/fixtures/gba_*.gba` → Rust 侧用
`concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/gba/…")` 读文件。

### 3.2 对比

| 维度 | 方案 A（Rust 内建） | 方案 B（Python 生成文件） |
|---|---|---|
| 期望值可 review | ✅ **每个字节都能在代码里看到** | ❌ 二进制 blob，reviewer 无法核对它到底写了什么 |
| AGENTS.md 纪律 5「测试向量按字段拼装，不手写字面量」 | ✅ 天然满足 | ❌ **二进制 blob 就是「手写字面量」的二进制形态** |
| AGENTS.md 纪律 10「编不过的测试等于不存在」 | ✅ 无外部前置，文件不可能缺失 | ⚠️ 依赖仓库里一个二进制文件；**CI 侧根本没有 cargo 环节**（`f11qa.yml` 自陈），⇒ 文件丢了**没有任何一层会报警** |
| 变异验证 | ✅ 改代码，`git diff` 可读 | ⚠️ 改二进制 blob，diff 不可读 |
| 工具链依赖 | ✅ 无（不引入 Python 前置） | ⚠️ 需要 Python 跑生成器 |
| ARM 编码器写在哪 | ✅ 一处（Rust），测试与夹具共用 | ⚠️ **两处**（Python 生成器 + Rust 读文件）⇒ 编码器双份实现 = 双份可能出错 |
| 能喂给 C++ 装载器 / `cartprobe` 做端到端 | ⚠️ 需要额外导出一个文件 | ✅ 天然可以 |
| 仓库卫生 | ✅ 零新增二进制 | ⚠️ 新增 9 个二进制 blob 进 git |

### 3.3 推荐结论

**选方案 A**，理由三条：

1. **纪律 5 与纪律 10 的交集**：本仓的门禁链路上**没有任何 cargo 环节**（CI 侧），
   所以「测试依赖仓库里一个二进制文件」是**无监控的依赖**。方案 A 把这个依赖消灭掉。
2. **编码器只能有一份**。9 条夹具都要手写 ARM 指令；方案 B 会让同一套位级编码存在两个实现
   （Python 一个、Rust 一个）⇒ 这正是纪律 4（位级格式）翻车的高发形态。
3. **`.gitignore` 反而更简单**：方案 B 需要 `*.gba` + `!` 例外两条；方案 A 只需要 `*.gba` **一条**，
   而这一条**机械地保证商业卡带永远不会进仓库**（v2.0.3 计划 §〇 范围外第 2 条）——
   **例外规则是它的漏洞**。

**方案 A 的 C++ 端到端缺口怎么办**：需要时由 `fixtures.rs` 导出一份 `.gba` 到
`tests/fixtures/gba/`（**不提交**，`.gitignore` 已全局忽略）供人工或 `cartprobe` 把玩。
**导出是 S1-a 的可选项，不是必选项。**

---

## 四、决策点 D2 — 门禁口径：「全绿」与「会红」的矛盾

### 4.1 矛盾在哪

v2.0.3 计划里两句话互相打架：

- **§〇 范围内第 2 条**：「让每一条已定位缺陷都有**一条会红的锁测试**」
- **§2.4 第 1 条**：「8 条夹具全部生成，`cargo test …` **从 249 项增加到 257 项，全绿**」

一条断言「SWI 0x1A 之后机器不永久自旋」的测试，在 **S2-b 修复之前必然是红的**。
⇒ 「会红」与「全绿」不可能同时成立。

### 4.2 三个候选口径

| 口径 | 做法 | 问题 |
|---|---|---|
| **① 刻画式** | 断言写成「缺陷当前的行为」（如「SWI 之后 PC 停在 0x08」），今天全绿，修复时再把断言改写成目标行为 | ❌ **把缺陷锁成了预期行为**——v2.0.3 计划 §1.2.1 自己就点名批评过这个（`bios.rs:396-401` / `bios.rs:218-223`）。且门禁「全绿」给人「缺陷已修」的错觉 ⇒ 这正是 AGENTS.md 反复批评的「**门禁曾是错误信念的镜像**」 |
| **② 常驻红** | 8 条断言写成目标行为并常驻 ⇒ 从 S1-a 起门禁长期红 | ❌ 违反「NES 零回归 + 本地门禁须全绿」；且 CI 侧没有 cargo 环节 ⇒ 没人看得到红 |
| **③ `#[ignore]` 复现器 + 常驻守卫（推荐）** | 复现器写成**目标行为**并标 `#[ignore]`，附 `#[ignore = "S2-b: 修复后转常驻锁测试"]`；另加**常驻**守卫确保复现器不会烂在里面 | ✅ 门禁保持全绿；复现器**断言的是目标行为**（不是缺陷行为）；修复落地时把它转成常驻红→绿 |

### 4.3 推荐：口径 ③

**为什么 ③ 比 ① 好**：① 的断言「通过原因」是「缺陷还在」，与「它要验的东西」不是同一条路径
（AGENTS.md 纪律 3 的推论 + 纪律 12）。③ 的断言**从第一天起就写着目标行为**，
只是暂时不参与门禁计数——**修复落地的那一刻，它转常驻并且先红后绿**，变异验证天然成立。

**③ 的必备配套**（否则 `#[ignore]` 会腐烂——AGENTS.md 纪律 10 的同族）：

| 守卫 | 断言 | 何时会红 |
|---|---|---|
| **G1 孤儿守卫** | 每一条 `#[ignore]` 的复现器都必须带**非空的原因串**，且原因串必须指名它归属的 S2/S3 工作项 | 有人加了 `#[ignore]` 却没写归属 |
| **G2 夹具完整性** | 9 条夹具全部能构造出合法 ROM（长度 ≥ `0x200`、main unit code = 0、body 装得下） | 夹具本身被改坏 |
| **G3 编码器端到端自检** | 每条 ARM 编码器一条：程序写一个已知值到内存再读回，值必须相等 | 编码器被改坏 ⇒ **它正在给所有夹具发错误的指令** |
| **G4 复现器存在性** | 9 条复现器一条不少（按名字枚举核对） | 有人删了一条复现器来「让门禁变绿」 |

**G3 是本工作项最关键的一条**：编码器一旦错，**所有夹具都在测别的东西**，而且照样「全绿」。
（这与 AGENTS.md 纪律 12「相对比较型测试对共因变异结构性无感」是同一族，
但更严重——编码器是**全部夹具的共因**。）

### 4.4 门禁计数（口径 ③ 下的实际数字）

| 阶段 | 常驻（默认 `cargo test` 会跑） | `#[ignore]` 复现器 | 合计 |
|---|---|---|---|
| **S1-a 交付后** | 249 + **9**（G3 编码器自检） + 1（G1）+ 1（G2）+ 1（G4） = **261** | 9 | 270 |
| **S2/S3 每修一处** | +1（复现器转常驻） | −1 | — |
| **S3 全部修完** | **270** | 0 | **270** |

⇒ v2.0.3 计划 §2.4 的「249 → 257 全绿」需改写为「**249 → 261 全绿 + 9 条 `#[ignore]` 复现器就位**」
（§十一 的 r2 记录这次改写）。

---

## 五、夹具清单（修正后 **9 条**，替换掉计划里 1 条不可用的）

> 表格里的「当前行为」与「目标行为」两列**都是断言**，不是描述。
> 「归属」列指名该复现器在哪个工作项里转成常驻锁测试。

| # | 夹具 | 触发什么 | 当前行为（今天必然红/panic） | 目标行为 | 归属 |
|---|---|---|---|---|---|
| 1 | `swi_unimpl_1a` | ARM 程序执行 `SWI 0x1A`（SoundDriverInit） | dispatch 不认领 ⇒ core 走真异常路径 ⇒ PC = `0x08` ⇒ stub BIOS 的 `0x08` 是 `BRANCH_SELF`（`bios.rs:97`）⇒ **永久自旋** | SWI 返回并被记录；程序继续执行下一条指令 | **S2-b** |
| 2 | `swi_unimpl_16` | ARM 程序执行 `SWI 0x16`（Diff8BitUnFilterWram） | 同上 | 同上 | **S2-b**（兜底）+ **S3-a**（真实现） |
| 3 | `oam_bad_shape` | 程序用 `STRH` 把 OAM 精灵 0 的 attr0 写成 **shape = 3** | 跑到可见扫描线 ⇒ `object_attributes.rs:265` `.unwrap()` panic ⇒ `panic = "abort"` ⇒ **进程闪退** | 精灵按「禁用」处理；帧正常步进；无 panic | **S2-a** |
| 4 | `oam_gfx_mode3` | 同上，但 attr0 写成 **gfx mode = 3** | `object_attributes.rs:144` 的 `unreachable!()` panic | 同上 | **S2-a** |
| 5 | `eeprom_8k_protocol` | ROM 含 `EEPROM_V124` ⇒ `BackupType::Eeprom`；程序按 **GBATEK 协议**（前置条件 P1）写 8 KB EEPROM 的 0 号地址再读回 | vendor 的命令解释与真机协议不一致 ⇒ 读回不等于写入 | 读回等于写入 | **S3-b** |
| 6 | `eeprom_window_width` | ROM 含 `EEPROM_V124`；程序对 `0x0D010000`（真机是 **ROM 区**，不是 EEPROM）做一次半字写 | `is_eeprom_access` 覆盖到 `0x0DFFFFFF` ⇒ **该次写被劫持进 EEPROM 位流** ⇒ 卡的 ROM 区不可读 | 该地址不被劫持；读到的是 ROM 内容 | **S3-b** |
| 7 | `eeprom_srm_size` | ROM 含 `EEPROM_V124`；程序走一遍存档通路 | `buffer_size()` 对 Eeprom 返回 `0x8000` ⇒ `.srm` **恒 32768 字节** | `.srm` = 8192（8 KB 芯片） | **S3-b** |
| 8 | `flash_status_reg` | ROM 含 `FLASH_V124`；程序先往 `0x0E000000` 写 `0x00`，再读 `0x0E000006` | 读走「普通读」分支 ⇒ 返回**存档原始字节 `0x00`** ⇒ 状态位 bit0 = 0 | 返回**芯片状态**，bit0（ready）= 1 | **S3-b** |
| 9 | `rom_window_mirror` | ROM 只有 `0x200` 字节；程序读 `0x08000040`（ROM 内）与 `0x08000240`（超出容量） | 超出容量的读返回**开放总线垃圾** `((address >> 1) & 0xFFFF)` | 两个地址读到**同一个绝对金标准值** | **S3-d** |
| 10 | `gpio_read_before_write` | ROM 含 `EEPROM_V124`；程序**先读** `0x080000C4`，再写 `0x080000C6/C8`，再读一次 | 第一次读拿到 **ROM 原始字节**（`gpio_present` 初值 `false`，`internal_memory.rs:222`；只有 `write_at` 写过 `0xC4..=0xC9` 才置 `true`，`:689`） | 第一次读就该拿到 **GPIO 状态**（需前置条件 P1 的 GBATEK 依据） | **S3-c** |

### 5.1 三处与 v2.0.3 计划 §2.2 的差异（都要记进 r2）

| 差异 | 计划 §2.2 | 本计划 | 原因 |
|---|---|---|---|
| **D-a** | `gba_io_unmapped.gba` | **删除**，替换为 `oam_gfx_mode3` | 该缺陷**不可达**（§1.1）⇒ 夹具今天就是绿的 ⇒ 无牙门禁（AGENTS.md 纪律 13） |
| **D-b** | 8 条 | **9 条** | EEPROM 的三条缺陷（协议 / 窗口宽度 / `.srm` 尺寸）是**三条独立缺陷** ⇒ 按「一个夹具不混两件事」必须拆开 |
| **D-c** | `gba_eeprom8k.gba` 一条覆盖 EEPROM | 拆成 `eeprom_8k_protocol` / `eeprom_window_width` / `eeprom_srm_size` | 同上 |

### 5.2 每条夹具的 ROM 构造（要点）

所有夹具共用一个卡带构造器：

```
ROM 长度        0x200 字节（= `frame.rs:74` 的 MIN_CARTRIDGE，且 ≥ vendor 的 0xE4 下限）
offset 0x00     b <body>        （真实卡带的第一条指令就是一条分支）
offset 0xB3     0x00            （main unit code = GBA 卡带）
offset 0xC4     程序体          （与 `gate.rs:75` 的 CODE_OFFSET 一致）
其余            0
```

> **vendor 不校验 Nintendo logo**：`cartridge_header.rs:142-157` 只 `tracing::warn!`，
> `Gba::new` 不检查 ⇒ 合成 ROM 无需 logo。（`gate.rs` 的既有测试已在证明这一点。）

- 夹具 5/6/7：ROM 里放 ASCII **`EEPROM_V124`** ⇒ `BackupType::detect`（`internal_memory.rs:75-89`）识别为 Eeprom。
  **摆放位置**：避开 `0x00–0xC3`（header 区，含 logo / 固定值 / 校验和）与 `0xC4` 起的程序体 ⇒ 放 **`0x40`**。
- 夹具 8：放 ASCII **`FLASH_V124`** ⇒ Flash64。同样放 `0x40`。
- 夹具 9：ROM `0x40` 放**4 个已知字节**（例如 `5A 5B 5C 5D`）⇒ 断言读回的**字**等于
  `0x5D5C5B5A`（**绝对金标准**，不是「两个地址相等」的相对比较——AGENTS.md 纪律 12）。
- 夹具 10：ROM `0xC4` 是程序体第一条指令 ⇒ 第一次读 `0x080000C4` 读到的就是它 ⇒
  **期望值必须由程序体自己算出来**，不能手写字面量。

### 5.3 夹具 9 的「开放总线」期望值（今天的真实读数）

`internal_memory.rs:377-404`：`address >= rom.len()` 时返回

```rust
(((address >> 1) & 0xFFFF) as u16).get_byte((address & 0b1) as u8)
```

⇒ 读 `0x08000240` ⇒ offset `0x240` ⇒ `(0x240 >> 1) & 0xFFFF = 0x120` ⇒ 低字节 `0x20`、高字节 `0x01`
⇒ 读回的字 = `0x0120_xxxx` 形态，**与期望的 `0x5D5C5B5A` 完全不同** ⇒ 红 ✓

---

## 六、夹具工具的**语义契约**（不写位级字面量）

> **纪律 4 的正确用法**：本节**不写任何 ARM 编码的字面量**。
> 编码器由**语义**定义，位模式由实现者从**本仓已有的、已被既有测试验证过的常量**反推
> （`gate.rs:88-148` 的 `mov_r0_imm` / `swi`；`bios.rs:66-99` 的 `LDR_R2_R2` / `STRH_R1_R0` / `CMP_R2_ZERO`），
> **并由 §四 G3 的端到端自检逐条验证**。
>
> ⚠️ 已知的 vendor 偏差（必须绕开，见 §一.8 末）：
> - vendor 的 `SingleDataTransfer` **立即数偏移不乘 4**（真 ARM 乘 4）⇒ **只用零偏移**。
> - vendor 的 `HalfwordDataTransfer` 立即数偏移是 `imm4H:imm4L`（字节数，与真 ARM 一致）⇒ 同样只用零偏移。
> - vendor 的 `OperandKind` 判别与真 ARM 相反 ⇒ **不使用 I=0 的真 ARM 立即数形式**（ADD/SUB/AND 等），
>   只用 `MOV` / `ORR`（它们的真 ARM 立即数形式本来就是 I=1 ⇒ 两边一致）。

### 6.1 ARM 指令编码器（每个都要一条 G3 自检）

| 编码器 | 语义契约 | G3 自检怎么做 |
|---|---|---|
| `mov_imm(rd, value)` | 立即数 = **8 位常量 + 偶数右旋**（ARM 数据处理立即数规则）。**旋不进就报错，不许掩码**（照 `gate.rs:88-95` 的先例：掩码后的值仍是「一个合法立即数，只是另一个值」） | 程序 `mov r0,#V` → `swi 0x0D`（GetBiosChecksum）不测 V…… 改为：程序 `mov r0,#V` 后 `str r0,[r1]` → 读回内存 == V |
| `orr_imm(rd, rn, value)` | 同上的旋转立即数规则，作用在 `rn` 上 | 程序 `orr r0,r1,#V` → `str r0,[r2]` → 读回 == (初值 \| V) |
| `str(rd, rn)` | 零偏移的**字**存储 | 程序 `str` → 读回一个字 == 期望 |
| `strh(rd, rn)` | 零偏移的**半字**存储 | 同上，读回半字 |
| `ldr(rd, rn)` | 零偏移的**字**读取 | 程序 `ldr` → 再 `str` 到别处 → 读回 == 内存里的值 |
| `ldrh(rd, rn)` | 零偏移的**半字**读取 | 同上 |
| `swi(number)` | SWI 号码在 **bits 16–23**（ARM 模式），与 `gate.rs:142-148` 同源 | 复用 `gate.rs` 已有的 `swi_round_trip` 覆盖 |
| `branch_self()` / `mov_reg(rd, rn)` | 自旋且不改寄存器 | 自旋本身不需要自检 |

### 6.2 卡带构造器

| 接口 | 契约 |
|---|---|
| `Cartridge::new()` | 0x200 字节；offset 0 = `b <body>`；`0xB3` = 0x00；body 在 `0xC4` |
| `Cartridge::code(words)` | 追加 ARM 程序体 |
| `Cartridge::ascii(offset, bytes)` | 在指定偏移写 ASCII（存档型号标记） |
| `Cartridge::bytes(offset, bytes)` | 在指定偏移写原始字节（夹具 9 的金标准） |
| `Cartridge::build() -> Vec<u8>` | 产物；长度、body 装得下等断言在这里做 |

### 6.3 运行器

| 接口 | 契约 |
|---|---|
| `Fixture::new(rom)` | `Gba::new(bios::stub(), &rom)` + `install_swi_hook`（与 `gate.rs:229-230` 同款） |
| `Fixture::run_steps(n)` | 步进 n 步 |
| `Fixture::run_until_vblank() -> bool` | 步进到 `gba.step()` 返回 `true`（进入 VBlank）为止；带步数上限 |
| `Fixture::catch_panic(f) -> Result<(), String>` | **`std::panic::catch_unwind` + `AssertUnwindSafe`** 包住 `f`，把 panic 的 payload 变成字符串 |
| `Fixture::reg(n) -> u32` | 读 `gba.cpu.registers.register_at(n)` |
| `Fixture::read_byte/half/word(addr)` | 走 `gba.cpu.bus.read_*`（**与游戏指令同一条路径**） |
| `Fixture::mem_byte(addr) -> u8` | 同上，读内存 |

### 6.4 ⚠️ `catch_unwind` 与 `panic = "abort"` 的**硬约束**

`src/rust/Cargo.toml` 的 `[profile.release] panic = "abort"` ⇒ **release profile 下 `catch_unwind` 无效**，
一次 panic 会**带走整个测试二进制**（不只是那一条测试）。

⇒ **强制要求**：

1. 复现器里显式判 `cfg!(debug_assertions)`：
   - **debug**（门禁命令 `cargo test` 的默认 profile）⇒ 跑，`catch_unwind` 生效，测试如实红；
   - **release**（`cargo test --release`）⇒ **直接跳过**（`return Ok(())`），并在 `#[ignore]` 原因串里写明。
2. **门禁命令只用 debug profile**：`cargo test -p fceux11-rust --no-default-features --features gba --lib`
   （v2.0.3 计划 §八 的复验命令已是这样；**注意**同节的 `cartprobe` 那条用了 `--release`，
   ⇒ 那条命令在修复前会因 panic 直接 abort 整个进程——**这是预期行为**（它就是要看崩不崩），
   但要在 §八 的注释里写清楚，否则会被误判成「门禁挂了」）。

---

## 七、实施步骤（每条可单独执行与验收）

### Step 0 · 前置取证（P1）

**做什么**：把 GBATEK 的 **EEPROM 命令协议** 与 **GPIO/RTC** 两章抓下来，
摘录写进 v2.0.3 计划 §附录 A（新增 A.4 / A.5）。
沿用 §一.5 已定的路径：代理 `127.0.0.1:7897` 抓 `https://mgba-emu.github.io/gbatek/`。
**同时**给仓库加一条可重跑的抓取脚本（§一.5 已承诺）。

**验收**：附录 A 里能查到 EEPROM 命令的**每一位**与 GPIO 在「未写入前」的真机行为。

**取不到怎么办**：**停下**，在 v2.0.3 计划 §九 记录「取证受阻」，**不许凭记忆写协议**
（AGENTS.md 纪律 4；本项目已因此翻车两次：DMA 起始时序、FLASH1M 容量）。
此时夹具 5 与夹具 10 降级为「只做机制确认，不做协议断言」，并在 §十一 记录降级。

### Step 1 · 编码器与构造器 + G3 自检

**做什么**：写 `fixtures.rs` 的 §6.1 / §6.2 / §6.3；写 G3 的 8 条端到端自检。

**验收**：
- `cargo test -p fceux11-rust --no-default-features --features gba --lib` 通过，
  **计数从 249 增加到 257**（249 + 8 条编码器自检）。
- **G3 双向变异验证**：把编码器里的一个字段改错（例如 `str` 写成 `strh`）⇒ 至少一条 G3 自检转红。
  **只改一个字段** ⇒ 其它 7 条**必须仍然绿**（证明它们各测各的，不是共因）。

**这一步不过，不许做 Step 2。**

### Step 2 · 9 条夹具 + 9 条复现器

**做什么**：按 §五 逐条写夹具与复现器。

**验收**：
- 每条复现器在 `--ignored` 下**确实红**（或确实 panic 被捕获）⇒ 证明它复现的是真缺陷。
- 每条复现器都带 `#[ignore = "S2-x / S3-x: 修复后转常驻锁测试"]`。
- `cargo test … --lib`（默认，不带 `--ignored`）**全绿**，计数 257 → 266。

### Step 3 · 四条守卫（G1 / G2 / G4 + 夹具自检）

**验收**：计数 266 → **269**（G1 / G2 / G4 各 +1）。

> 计数说明：249（既有）+ 8（G3）+ 9（复现器，`--ignored` 不计入默认计数）+ 3（G1/G2/G4）
> ⇒ 默认 `cargo test` 计数 = **269**；`--ignored` 另有 **9** 条复现器。

### Step 4 · `.gitignore` 加 `*.gba`

**做什么**：在 `.gitignore` 的「Emulator ROMs」那一段加 `*.gba`，**不加例外**。

**验收**：`git check-ignore -v tests/fixtures/gba/whatever.gba` 命中；
`git status --porcelain` 在放了一个 `.gba` 后**不出现**该文件。

### Step 5 · 变异验证（逐条，双向）

对每条复现器：

| 方向 | 做法 | 期望 |
|---|---|---|
| **正向** | 把被断言的行为改回去（例如把 SWI 向量改回 `BRANCH_SELF`、把 OAM 禁止值改回 panic） | 该复现器**转红** |
| **反向** | 还原 | 该复现器**复红**（红 = 缺陷在场） |

**验收**：9 条复现器 × 2 向 = **18 次变异**，逐条留证据（贴命令与读数）。

### Step 6 · 门禁全跑

**验收**（全部必须绿）：

```
cargo test -p fceux11-rust --no-default-features --features gba --lib     # 269 项
cargo test -p gba-core --lib                                            # 242 项（不变）
cargo test -p fceux11-debug --lib                                        # 93 项（不变）
cargo test -p fceux11-media                                             # 52 项（不变）
cargo check --workspace --no-default-features                            # 不变式 8
cargo check --workspace
cmake --build build --config Release
ctest --test-dir build -j 4 -LE perf                                     # 36 项全绿
```

⚠️ **ctest 不要与构建/测试任务并行**（AGENTS.md 坑 12 + r54 ②）。

### Step 7 · 收口

**做什么**：
1. v2.0.3 计划 §九 记 **r2**（含 §一 的 6 处更正、§2.2 的 D-a/D-b/D-c、§2.4 的计数改写）。
2. v2.0.3 计划 §1.2.2 加「**更正**」小节，指向本文 §一。
3. v2.0.3 计划 §2.2 / §2.4 按 §五 / §4.4 改写。
4. 更新 AGENTS.md 的「当前进度」段：登记 S1-a 已完成 + 门禁计数变化。
5. CHANGELOG。

---

## 八、验收命令（与 v2.0.3 计划 §八 合并使用）

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# GBA 锁测试（含 9 条夹具复现器 —— 默认不跑复现器）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"

# 单独跑 9 条复现器（预期：全部红 —— 它们要证明缺陷在场）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib -- --ignored --nocapture"

# 计数核对（纪律 10：新增门禁要当场看到它的测试计数）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib -- --list | Measure-Object -Line"

# 其余四条 Rust 门禁 + 两态 feature 检查
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-debug --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-media"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace --no-default-features"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace"

# NES 零回归（必须等构建结束再跑）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf
```

---

## 九、风险与已知坑

### 9.1 本工作项自身的风险

| # | 风险 | 处置 |
|---|---|---|
| R1 | **编码器是全部夹具的共因** —— 它错了，9 条夹具都在测别的东西，而且照样「全绿」 | G3 端到端自检（§4.3）+ 8 条**互不共因**的变异验证（Step 5） |
| R2 | `catch_unwind` 在 release profile 下无效 ⇒ 一次 panic 带走整个测试二进制 | §6.4 的强制要求（`cfg!(debug_assertions)` 分支） |
| R3 | 复现器长期 `#[ignore]` ⇒ 腐烂 | G1 孤儿守卫 + G4 存在性守卫 + `#[ignore]` 原因串指名归属工作项 |
| R4 | 夹具 3/4 触发的 panic 在 release 下 abort ⇒ `cargo test --release` 整个套件死 | 同 R2；门禁命令固定用 debug profile |
| R5 | Step 1 之后 9 条夹具一起写 ⇒ 一次提交巨大、review 难 | 按 §五 的**编号分 3 批**提交（3+4+2），每批单独过门禁 |
| R6 | 有人为了让门禁变绿而**删掉**复现器 | G4 存在性守卫 |
| R7 | 「刻画式断言」诱惑 —— 把断言写成「缺陷当前的行为」以求全绿 | §4.2 的口径 ③；断言一律写**目标行为** |

### 9.2 本仓的既有坑（与本工作项相关）

| # | 坑 | 出处 |
|---|---|---|
| 1 | `cargo test -p fceux11-rust` **必须**加 `--no-default-features --features gba` | AGENTS.md 坑 2 |
| 2 | `ctest` 不在 PATH，在 `D:\Program Files\CMake\bin\` | AGENTS.md 坑 3 |
| 3 | 仓库在 HEAD 上**本来就不是 rustfmt-clean 的** ⇒ `cargo fmt --check` 不能当门禁；只保证**自己新写的行**符合 rustfmt | AGENTS.md 坑 5 |
| 4 | **行尾**：工作树是 CRLF（`core.autocrlf=true` 且无 `.gitattributes`）⇒ 新建文件必须写成 **CRLF**；改完用 `git diff --numstat` 核对（插入/删除远超真实改动量 ⇒ 行尾被动了） | AGENTS.md 坑 11 + 记忆条目「Edit 工具的行尾改写方向不固定」 |
| 5 | **编不过的测试等于不存在** ⇒ 每加一条门禁，当场看到它的计数 | AGENTS.md 纪律 10 |
| 6 | `cargo test -p fceux11-rust` **不跑依赖 crate 的测试** ⇒ 夹具测试必须落在**根 crate** 里（`src/rust/src/gba/`），不能放 `gba-core` | AGENTS.md r26 补录 |
| 7 | `ninja -j 1` 串行构建（LNK1104 并发归档冲突） | AGENTS.md 坑 12 |

### 9.3 本文的复核命令（别人要验 §一 的更正，照这些跑）

```powershell
# 更正 A：bus.rs 的 panic 可达性 —— 看 read_raw/write_raw 的 match 是否全覆盖
Select-String -Path "src\rust\crates\gba-core\src\bus.rs" -Pattern 'fn read_raw|fn write_raw' -Context 0,70
# 更正 B：PLD 的两道防线
Select-String -Path "src\rust\crates\gba-core\src\cpu\arm7tdmi.rs" -Pattern 'Pld' -Context 4,8
Select-String -Path "src\rust\crates\gba-core\src\cpu\arm\psr.rs" -Pattern 'NV =>'
# 更正 C：OAM 的两处真现场
Select-String -Path "src\rust\crates\gba-core\src\cpu\hardware\lcd\object_attributes.rs" -Pattern 'unreachable!|Prohibited ObjShape|Forbidden GfxMode' -Context 3,3
# 更正 E：read_at/write_at 的全部调用方
Get-ChildItem -Path "src\rust\crates\gba-core\src" -Recurse -Filter *.rs | Select-String -Pattern 'read_at\(|\.write_at\('
# 更正 F：EEPROM 模型与窗口
Select-String -Path "src\rust\crates\gba-core\src\cpu\hardware\internal_memory.rs" -Pattern 'fn is_eeprom_access|fn eeprom_write|fn eeprom_read|fn eeprom_interpret_command|fn buffer_size' -Context 0,12
```

### 9.4 登记但不修的观察项

| # | 观察项 | 归属 |
|---|---|---|
| O1 | vendor 的 `OperandKind` 判别与真 ARM 相反（`cpu/arm/instructions.rs:796`）⇒ `bios.rs:78` 的 `ADD_LR_PC` 在真 ARM 上是 `ADD lr, pc, r0` | 长尾（影响面：本模拟器的 stub BIOS；真机用真 BIOS） |
| O2 | vendor 的 `SingleDataTransfer` 立即数偏移**不乘 4**（真 ARM 乘 4） | 长尾（本工作项绕开：只用零偏移） |
| O3 | vendor 的 `HalfwordDataTransfer` 立即数偏移是 `imm4H:imm4L`（字节数）—— 与真 ARM 一致 | 仅登记 |
| O4 | `bus.rs` 的 11 处 panic 虽不可达，但是**防御性加固**的好目标 | **S2-a 顺带做**（v2.0.3 计划 §3.1 的表可以保留，但标注「防御性加固，非止血」） |

---

## 十、非目标

1. **不修缺陷**。9 条复现器转常驻是 S2/S3 各工作项的活。
2. **不改 v2.0.3 计划 §一 的结论方向** —— 只更正 §1.2.2 的**证据清单**（§1.6）。
3. **不做 S1-b 的探针改造**。
4. **不做精度对齐**（v2.0.3 计划 §〇 范围外第 3 条）。
5. **不引入新的构建依赖**（不引汇编器、不引 Python 前置）。
6. **不改 C++ 侧任何文件**。

---

## 十一、变更记录

| # | 日期 | 内容 |
|---|---|---|
| r1 | 2026-11-09 | **初稿**。只读调查 + 出 PLAN，未动任何生产代码。调查结论：**v2.0.3 计划 §1.2.2 的证据清单需要更正 6 处**，其中 2 处直接推翻原结论：①`bus.rs` 的 11 处 `panic!` **全部不可达**（`read_raw`/`write_raw` 的 match 对各 I/O 区间已全覆盖）；②`operations.rs:896` 的 PLD `todo!` 是**死代码**（`arm7tdmi.rs:458-463` 拦截 + `psr.rs:74` 的 NV 门）。更正后的**可达 panic 面只有 OAM 两条**：`object_attributes.rs:265`（shape=3）与 `:144`（gfx mode=3）。另更正：EEPROM **有**位级模型（`internal_memory.rs:523-611`），真正缺的是**访问窗口过宽 16 MB** / **芯片容量不建模（`.srm` 恒 32768）** / **Flash 状态寄存器不存在**。据此修正夹具清单：删 `gba_io_unmapped.gba`（不可达 ⇒ 无牙），EEPROM 拆 3 条 ⇒ **8 条变 9 条**。另发现两条**未被上游记录的**、vendor 与真 ARM 的编码偏差（`OperandKind` 判别相反、`SingleDataTransfer` 立即数偏移不乘 4）⇒ 登记为观察项 O1/O2。PLAN 给出两个待裁决点：**D1 夹具载体形态**（推荐 Rust 内建，非 Python 生成二进制入库）与 **D2 门禁口径**（推荐「`#[ignore]` 复现器 + 常驻守卫」，以消解「全绿」与「会红」的矛盾）。本文**未修改任何生产代码**，也**未修改 v2.0.3 计划**（待拍板后一并按纪律 2 处理） |
| r2 | 2026-11-09 | **业主拍板 D1 / D2 / D3，三个建议全部采纳**（2026-11-09 问卷）：**D1 载体 = Rust 内建**（不产生二进制入库；`.gitignore` 只加一条 `*.gba`，**不加例外**）；**D2 门禁口径 = 复现器标 `#[ignore]` + 3 条常驻守卫**（断言写**目标行为**；`-- --ignored` 下 9 条复现器必须逐条为红）；**D3 夹具条数 = 9 条**。⇒ 本文的 §三 / §四 / §五 三个决策点的推荐**成为已裁决结论**，本文其余部分**无需修改**（推荐本来就是这三项）。按纪律 2「先记入变更记录，再改计划」，**本条先记于本文与上游计划 §九（r2），随后才改写上游的 §1.2.2 / §1.2.4 / §2.2 / §2.4 / §一.5 / STATUS**。改动清单：①上游 §1.2.2 加「更正」小节 + 表格加判定列；②上游 §1.2.4 的「EEPROM 没有真模型」改为「模型存在、缺三条（E1/E2/E3）」；③上游 §2.2 改写为 9 条 + 载体形态说明 + 两条夹具纪律（OAM 必须半字/字写；断言必须能区分「状态」与「数据」）；④上游 §2.4 改写出口判据（249 → **269 全绿** + **9 条 ignore 复现器** + 前置条件）；⑤上游 §1.5 追加「附录 A 缺两章」的前置条件提示；⑥上游 STATUS 加 r2 警示。**仍未动任何生产代码**（工作树只有两个文档） |

---

*本文由独立只读调查 + code review 产生（2026-11-09）。实施按 Step 0..7 逐条执行，先 PLAN 后动手。*
