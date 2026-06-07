# Mapper Rust 化可行性研究报告

> **版本**：v0.2.30  
> **范围**：`src/boards/` 下全部 Mapper 实现  
> **目标**：分析现有 C++ Mapper 宏系统，提出 Rust 替代方案，为 v0.3.x 决策提供依据

---

## 一、现状分析

### 1.1 规模统计

```bash
$ ls src/boards/*.cpp | wc -l
175

$ wc -l src/boards/*.cpp | tail -1
  ~48,000 行
```

Mapper 是 FCEUX 代码库中**最大的单一模块**（约 48,000 行 C++），分布在 175 个 `.cpp` 文件中。

### 1.2 核心机制

每个 Mapper 文件遵循高度一致的模板：

```cpp
#include "mapinc.h"

static uint8 reg[4], cmd;
static SFORMAT StateRegs[] = {
    { reg, 4, "REGS" },
    { &cmd, 1, "CMD" },
    { 0 }
};

static void Sync(void) {
    setprg32(0x8000, (reg[2] >> 2) & 1);
    setchr8(reg[2] & 3);
}

static DECLFW(UNL22211WriteLo) { reg[A & 3] = V; }
static DECLFW(UNL22211WriteHi) { cmd = V; Sync(); }
static DECLFR(UNL22211ReadLo) { return (reg[1] ^ reg[2]) | 0x40; }

static void UNL22211Power(void) {
    Sync();
    SetReadHandler(0x8000, 0xFFFF, CartBR);
    SetReadHandler(0x4100, 0x4100, UNL22211ReadLo);
    SetWriteHandler(0x4100, 0x4103, UNL22211WriteLo);
    SetWriteHandler(0x8000, 0xFFFF, UNL22211WriteHi);
}

void UNL22211_Init(CartInfo *info) {
    info->Power = UNL22211Power;
    info->Reset = UNL22211Reset;   // optional
    AddExState(StateRegs, ~0, 0, 0);
}
```

### 1.3 关键宏与全局设施

| 设施 | 作用 | 出现频率 |
|------|------|----------|
| `DECLFW(name)` | 声明 CPU write handler | 100% |
| `DECLFR(name)` | 声明 CPU read handler | ~80% |
| `SetReadHandler(s,e,fn)` | 注册地址范围读回调 | 100% |
| `SetWriteHandler(s,e,fn)` | 注册地址范围写回调 | 100% |
| `setprg{2,4,8,16,32}(A,V)` | PRG ROM 分块映射 | ~90% |
| `setchr{1,2,4,8}(A,V)` | CHR ROM/RAM 分块映射 | ~85% |
| `SFORMAT` + `AddExState` | 保存状态注册 | ~95% |
| `CartBR` / `CartBW` | 默认 cart 读/写 | ~70% |
| `MapIRQHook` | 扫描线/周期 IRQ | ~30% |

### 1.4 当前问题

1. **全局函数指针数组**：`ARead[65536]` / `BWrite[65536]` 保存每个地址的读写回调。每次 `SetReadHandler` 都会遍历范围写入函数指针，开销大。
2. **无类型安全**：`DECLFW` 的 `A` 和 `V` 是全局宏定义的参数名，拼写错误在编译期无法捕获。
3. **状态分散**：每个 Mapper 的静态变量（`reg[]`, `cmd` 等）是全局的，不支持多实例。
4. **SFORMAT 脆弱性**：宏反射式序列化，字段改名后描述字符串不匹配会导致旧存档无法加载。

---

## 二、Rust 替代方案对比

### 方案 A：直接 1:1 翻译（不推荐）

将每个 Mapper `.cpp` 翻译为对应的 `.rs`，保留函数指针表。

**优点**：
- 工作量可预估（175 个文件逐一翻译）。
- 行为等价性容易验证（同地址同回调）。

**缺点**：
- 无法解决全局状态、SFORMAT 脆弱性等根本问题。
- 48,000 行 C++ → 估计 55,000+ 行 Rust，维护负担巨大。
- 函数指针表在 Rust 中需要 `unsafe` 或 `dyn Fn` 动态分发，性能下降。

**结论**：仅在极端保守场景下采用，不推荐作为默认策略。

---

### 方案 B：Proc-Macro DSL（推荐）

设计一个声明式 DSL，用 Rust proc-macro 生成 Mapper 实现。

**示例设计**：

```rust
#[mapper(id = 222, name = "UNL-22211")]
struct Mapper222 {
    #[state]
    reg: [u8; 4],
    #[state]
    cmd: u8,
}

impl Mapper for Mapper222 {
    fn power(&mut self) {
        self.sync();
        self.set_read_handler(0x8000..=0xFFFF, CartBR);
        self.set_read_handler(0x4100..=0x4100, Self::read_lo);
        self.set_write_handler(0x4100..=0x4103, Self::write_lo);
        self.set_write_handler(0x8000..=0xFFFF, Self::write_hi);
    }

    fn sync(&mut self) {
        self.set_prg32(0x8000, (self.reg[2] >> 2) & 1);
        self.set_chr8(self.reg[2] & 3);
    }

    #[read]
    fn read_lo(&self, addr: u16) -> u8 {
        (self.reg[1] ^ self.reg[2]) | 0x40
    }

    #[write]
    fn write_lo(&mut self, addr: u16, val: u8) {
        self.reg[(addr & 3) as usize] = val;
    }

    #[write]
    fn write_hi(&mut self, _addr: u16, val: u8) {
        self.cmd = val;
        self.sync();
    }
}
```

**优点**：
- `#[state]` 属性自动生成 `serde` / savestate 支持，替代 SFORMAT。
- 每个 Mapper 是独立 `struct`，天然支持多实例。
- 编译期生成 dispatch 表，可用 `match` 或跳转表替代函数指针。
- 类型安全：编译器检查地址范围、寄存器访问。

**缺点**：
- 需要设计和维护 proc-macro crate（约 1,000 行宏代码）。
- 开发者需要学习 DSL 语法。
- 复杂 Mapper（MMC3 变体、FDS 等）可能超出 DSL 表达能力。

**可行性评估**：**高**。约 80% 的 Mapper 逻辑简单（寄存器 + banking），完全适合 DSL。剩余 20%（MMC3 家族、VRC 家族、Sunsoft FME-7 等）可作为“手写特例”。

---

### 方案 C：解释器 / 字节码 VM

将 Mapper 行为描述为小型字节码，Rust 侧实现 VM 解释器。

**优点**：
- 极致的代码复用：所有 Mapper 共享同一个 VM。
- 可动态加载新 Mapper（无需重新编译）。

**缺点**：
- 性能风险：VM 解释开销对于 cycle-accurate 模拟可能是致命的。
- 需要为每个 Mapper 编写字节码，工作量大且不直观。
- 调试困难：无法单步进 Mapper 代码。

**结论**：**不推荐**。模拟器对 Mapper 有严格的时序要求，VM 解释难以满足。

---

### 方案 D：混合策略（最终推荐）

** Tier 1：DSL 生成**（覆盖 ~140 个简单 Mapper）
- 使用 proc-macro 生成标准 banking + IRQ 逻辑的 Mapper。

** Tier 2：手写 Rust trait**（覆盖 ~25 个中等复杂 Mapper）
- MMC3、VRC6、VRC7、Sunsoft FME-7 等。
- 直接实现 `Mapper` trait，保留手动优化的灵活性。

** Tier 3：C++ 桥接**（覆盖 ~10 个极端复杂 Mapper）
- FDS、NSF、带 coprocessor 的 Mapper（如 SA-1）。
- 通过 FFI 保留 C++ 实现，逐步替换。

---

## 三、Proc-Macro DSL 技术预研

### 3.1 核心语法草案

```rust
// crates/fceux11-core/src/mapper_dsl.rs (未来文件)

#[mapper(id = 0, name = "NROM")]
struct MapperNrom {
    #[state]
    prg_ram: [u8; 0x2000], // optional WRAM at $6000
}

impl Mapper for MapperNrom {
    fn power(&mut self) {
        // NROM has fixed PRG, no banking
        self.set_read_handler(0x8000..=0xFFFF, Self::prg_read);
        self.set_write_handler(0x6000..=0x7FFF, Self::prg_ram_write);
    }

    #[read]
    fn prg_read(&self, addr: u16) -> u8 {
        self.prg_rom[(addr as usize - 0x8000) % self.prg_rom.len()]
    }

    #[write]
    fn prg_ram_write(&mut self, addr: u16, val: u8) {
        self.prg_ram[(addr - 0x6000) as usize] = val;
    }
}
```

### 3.2 生成的代码骨架

proc-macro 将为 `#[mapper]` 生成：

1. `impl Mapper for MapperNrom` 的默认方法（`mapper_number`、`mirroring` 等）。
2. `snapshot()` / `restore_snapshot()` 基于 `#[state]` 字段自动生成。
3. 地址 dispatch 表（编译期 `match` 或跳转表）。

### 3.3 与 C++ 的兼容性

- `fceux11-core` 的 `Mapper` trait 已是 FFI 友好的设计。
- C++ 侧通过 `Box<dyn Mapper>` 的 opaque pointer 调用 Rust 方法。
- 保存状态：Rust `snapshot()` 返回 `Vec<u8>`（bincode），C++ 侧原样存入 savestate chunk。

---

## 四、风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Proc-macro 开发耗时 | 中 | v0.3.0 前完成原型，先用 5 个简单 Mapper 验证 |
| 复杂 Mapper 超出 DSL | 中 | Tier 2/3 兜底，不追求 100% DSL 覆盖 |
| 性能退化（dispatch） | 中低 | `match` + `#[inline]` 预期 ≥ C++ 函数指针 |
| 旧存档兼容性 | 高 | `snapshot()` 格式需与旧 SFORMAT 逐字节兼容 |
| 多实例需求 | 低 | `struct` 化天然支持，Netplay 不再依赖全局状态 |

---

## 五、建议路线图

### v0.3.0 — Mapper DSL 原型
- 实现 `fceux11-core/mapper_dsl` proc-macro。
- 翻译 5 个最简单的 Mapper（NROM, CNROM, AxROM, UxROM, MMC1）。
- 建立 `Mapper` trait ↔ C++ 的 FFI 桥接。
- nestest + 5 个商业 ROM 回归测试。

### v0.3.1–v0.3.5 — Tier 1 批量迁移
- 用 DSL 覆盖 ~50 个简单 banking Mapper。
- 每批 10 个，每批配回归测试。

### v0.3.6–v0.3.10 — Tier 2 手写迁移
- MMC3 家族、VRC 家族、Sunsoft。
- 保留 cycle-accurate 的精细控制。

### v0.3.11+ — Tier 3 桥接清理
- FDS、NSF 等极端复杂 Mapper 逐步替换或永久桥接。

---

## 六、结论

**Mapper Rust 化不仅是可行的，而且是高回报的。**

- 当前 C++ Mapper 系统的全局状态和 SFORMAT 宏是技术债的核心来源。
- Rust 的 `struct` + `trait` + proc-macro 可以一次性解决类型安全、多实例、序列化三大问题。
- 推荐采用 **Tier 1 (DSL) + Tier 2 (手写) + Tier 3 (桥接)** 的混合策略。
- 预计 v0.3.x 系列可以完成 90% 的 Mapper 迁移，剩余 10% 保留 C++ 桥接不影响整体架构。

---

*本报告随 v0.2.30 发布，作为 v0.3.x Mapper 迁移的决策基线。*
