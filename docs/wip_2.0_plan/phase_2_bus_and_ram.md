# Phase 2 · Bus Matrix + 私有内存

> **目标**：实现 CPU 总线矩阵（固定区 `match` + mapper 区间表）和 vNESU11 内部私有内存（WRAM/VRAM/OAM/Palette），为 Phase 3-4 的 PPU/APU 提供基础设施。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S4/S7 修订——
> ① 总线改为"固定区 match + mapper 区间表"两层（mapper 是 per-range 注册）；
> ② 新增 RAM 初始化随机源复刻任务（`xoroshiro128plus`，shadow run 前提）。

## 工期：2 周

---

## 1. 范围

### 1.1 ✅ 在范围内
- CPU 侧 `cpu_read`/`cpu_write`（固定区 `match` + mapper 区间表 fallthrough）
- PPU 侧 `ppu_read`/`ppu_write`（含 nametable mirroring）
- WRAM（2 KiB）
- VRAM（2 KiB，含 nametable mirroring）
- OAM（256 字节）
- Palette RAM（32 字节，含镜像）
- Open bus 行为（未映射地址返回"上次总线值"）
- PPU data read buffer（$2007 read 滞后）
- **[修订] MapperRangeTable**（`$4020-$FFFF` 走区间表，Phase 5 接通 handler）
- **[修订] RAM 初始化随机源复刻**（`splitmix64` + `xoroshiro128plus` + `RAMInitOption`）

### 1.2 ❌ 不在范围内
- CHR ROM/RAM 访问（mapper FFI，Phase 5 接通）
- PRG-RAM / PRG-ROM 访问（mapper FFI）
- Joypad strobe（Phase 4 一并实现）
- APU register read（Phase 4）

---

## 2. 任务清单

### 2.1 模块结构

```
crates/vnesu11/src/
├── bus.rs              # CPU/PPU 总线 match（非 trait）
├── ram.rs              # WRAM/VRAM/OAM/Palette 私有结构
├── ppu/
│   └── nametable.rs    # nametable mirroring 决策
└── snapshot/
    └── mem.rs          # savestate 序列化（WRAM/VRAM/OAM/Palette）
```

### 2.2 CPU 总线（[修订] 两层：固定区 match + mapper 区间表）

```rust
// crates/vnesu11/src/bus.rs
impl VNesSoc {
    #[inline(always)]
    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => self.ppu.read_register((addr & 7) as u8),
            0x4000..=0x4015 => self.apu.read_register((addr & 0x1F) as u8),
            0x4016..=0x4017 => self.joypad.read(addr),
            0x4018..=0x401F => self.read_open_bus(),
            // $4020-$FFFF：mapper 区间表（Phase 5 接通 handler）
            _ => self.mapper.read(addr).unwrap_or_else(|| self.read_open_bus()),
        }
    }

    #[inline(always)]
    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize] = val,
            0x2000..=0x3FFF => self.ppu.write_register((addr & 7) as u8, val),
            0x4000..=0x4015 => self.apu.write_register((addr & 0x1F) as u8, val),
            0x4016..=0x4017 => self.joypad.write(addr, val),
            0x4018..=0x401F => { /* open bus */ }
            _ => { self.mapper.write(addr, val); }
        }
    }
}
```

**为什么不能纯 `match`**：mapper 在运行时通过 `SetReadHandler(start,end,fn)` 按
区间注册 handler（`bus.h:74-93`、`boards/mmc3.cpp:327-344`），$4020-$FFFF 的
映射是**动态的**。固定区用 `match`（jump table），mapper 区用区间表
（4-16 项线性扫描，见 `phase_5_mapper_adapter.md`）。

### 2.3 [修订] RAM 初始化随机源复刻（S7）

**审计事实**：`fceu.cpp:910-954` 用 `splitmix64` + `xoroshiro128plus` 生成
可复现的 RAM 填充（`RAMInitOption` 0-3 + `RAMInitSeed`）。shadow run 要字节
等价，vNESU11 必须逐位复刻：

```rust
// crates/vnesu11/src/ram.rs
pub struct RamRng {
    s: [u64; 2],
}

impl RamRng {
    // 复刻 fceu.cpp:918-954 的 splitmix64 + xoroshiro128plus
    pub fn seed(&mut self, input: u32) {
        let mut z = input as u64 + 0x9e3779b97f4a7c15;
        z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9;
        z = (z ^ (z >> 27)) * 0x94d049bb133111eb;
        self.s[0] = z ^ (z >> 31);
        // ... 完整复刻
    }

    pub fn next(&mut self) -> u64 {
        let s0 = self.s[0];
        let mut s1 = self.s[1];
        let result = s0 + s1;
        s1 ^= s0;
        self.s[0] = s0.rotate_left(55) ^ s1 ^ (s1 << 14);
        self.s[1] = s1.rotate_left(36);
        result
    }
}

impl VNesSoc {
    /// 复刻 FCEU_MemoryRand：按 RAMInitOption 填充 WRAM
    pub fn memory_rand(&mut self, default_zero: bool) {
        // RAMInitOption 0-3 各分支逐行对照 fceu.cpp:958-975
    }
}
```

**注意**：`RAMInitOption`/`RAMInitSeed` 的值来自 C++ 配置层，通过 FFI 传入
（`vnesu11_set_ram_init(option, seed)`）。**不能**在 Rust 侧重写算法
（哪怕结果"看起来一样"）——必须逐位等价。

### 2.4 PPU 总线（mirroring，[修订] CHR 走区间表）

```rust
impl VNesSoc {
    #[inline(always)]
    pub fn ppu_read(&mut self, addr: u16) -> u8 {
        match addr {
            // $0000-$1FFF CHR ROM/RAM：mapper 区间表（Phase 5 接通）
            0x0000..=0x1FFF => self.mapper.read_chr(addr).unwrap_or(0),
            0x2000..=0x2FFF => {
                let mirrored = self.nametable_mirror(addr & 0x0FFF);
                self.vram[mirrored as usize]
            }
            0x3000..=0x3EFF => {
                // 0x3000-0x3EFF mirrors 0x2000-0x2EFF
                let mirrored = self.nametable_mirror((addr - 0x1000) & 0x0FFF);
                self.vram[mirrored as usize]
            }
            0x3F00..=0x3FFF => {
                let mirrored = addr & 0x1F;
                let val = self.palette[mirrored as usize];
                // 0x3F10/0x3F14/0x3F18/0x3F1C mirror 0x3F00/0x3F04/0x3F08/0x3F0C
                if mirrored >= 0x10 && mirrored & 0x3 == 0 {
                    self.palette[(mirrored & 0x0F) as usize]
                } else {
                    val
                }
            }
            _ => 0,
        }
    }
}
```

### 2.5 私有内存

```rust
// crates/vnesu11/src/ram.rs
pub struct InternalRam {
    pub wram: [u8; 2048],
    pub vram: [u8; 2048],
    pub oam: [u8; 256],
    pub palette: [u8; 32],
    pub ppu_read_buffer: u8,  // $2007 read 滞后
    pub open_bus: u8,         // 上次总线值
}
```

### 2.6 PPU data read buffer

```rust
impl VNesSoc {
    #[inline(always)]
    pub fn ppu_read_data(&mut self) -> u8 {
        let addr = self.ppu.v & 0x3FFF;
        let val = self.ppu_read_buffer;
        self.ppu_read_buffer = self.ppu_read(addr);
        // 自动 increment v (PPUCTRL $2000 bit 2 控制 +1 或 +32)
        self.ppu.v = self.ppu.v.wrapping_add(self.ppu.get_vram_increment());
        // 非强制 blanking 时 palette 屏蔽低 6 位
        val
    }
}
```

---

## 3. 验证策略

### 3.1 单元测试

```
crates/vnesu11/tests/bus_tests.rs
├── test_wram_mirror    // 写入 0x0000，读 0x0800/0x1000/0x1800 一致
├── test_ppu_reg_mirror // $2000 写 → $2008/$2010/$2018/$2000 同样生效
├── test_nametable_mirror_horizontal
├── test_nametable_mirror_vertical
├── test_nametable_mirror_single_screen
├── test_nametable_mirror_four_screen
├── test_palette_mirror
└── test_ppu_read_buffer // $2007 read 滞后
```

### 3.2 blargg 总线测试

- `blargg_nametable_timing.nes`（如果存在）
- 自定义 bus_confusion ROM（同时访问 mirror 区域）

---

## 4. 性能基准

| 项目 | 目标 |
|------|------|
| `cpu_read(0x0000)` (WRAM hot) | ≤ 5ns |
| `cpu_read(0x8000)` (mapper thunk) | ≤ 30ns（含 thunk 间接跳转） |
| `ppu_read(0x2000)` (VRAM) | ≤ 5ns |
| `ppu_read(0x3F00)` (palette) | ≤ 10ns（含镜像判定） |

---

## 5. 关键技术决策

### 5.1 Open Bus 实现

NES 的真实行为：未映射地址返回"上次访问的总线值"。在 vNESU11 中：

```rust
#[inline(always)]
pub fn cpu_read(&mut self, addr: u16) -> u8 {
    let val = match addr {
        // ... 实际解码
        _ => 0,  // 暂时返回 0，Phase 4 加 open_bus 跟踪
    };
    self.open_bus = val;
    val
}
```

Phase 2 先实现**正确的** open bus（不靠默认值 0），避免后续 PPU 测试踩坑。

### 5.2 Mirroring 决策缓存

nametable mirroring 在 PPU reset 时由 mapper 写入：

```rust
pub fn set_mirroring(&mut self, m: Mirroring) {
    self.nametable_mirror_fn = match m {
        Mirroring::Horizontal => Self::mirror_horizontal,
        Mirroring::Vertical => Self::mirror_vertical,
        // ...
    };
}

// 用函数指针（非 dyn）避免 vtable
type MirrorFn = fn(u16) -> u16;
nametable_mirror_fn: MirrorFn,
```

**关键**：函数指针而非 `dyn`——单态函数指针调用开销可预测。

### 5.3 不做通用总线协议

NES 总线只有一种拓扑。**绝对不做**：
- `trait Bus`
- `&mut dyn BusAgent`
- 任何"可配置"的地址路由

性能成本（每次访问多一次间接跳转）超过抽象收益。

**例外（[修订] S4）**：mapper 区（$4020-$FFFF）必须是**区间表**——这不是
"通用总线"，是 mapper 动态注册机制的忠实复刻（`SetReadHandler` 语义）。
区间表是冷路径（通常 4-16 项、cache 内），与固定区 `match` 不冲突。

### 5.4 [修订] RAM 初始化等价性（S7）

RAM 填充是 shadow run 字节等价的前提，**不是可选项**。`splitmix64` +
`xoroshiro128plus` + `RAMInitOption` 的完整状态机必须在 Phase 2 复刻并
用固定 seed 的 golden 输出测试锁定（同一 seed → 同一字节序列）。

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| Mirroring 函数指针影响内联 | 🟡 低 | 函数指针只用于 cold path；hot path 用 match |
| Open bus 实现漏写 | 🟠 中 | PPU 测试覆盖 |
| Palette 镜像位运算 bug | 🟡 低 | 显式 if 链而非位运算 |
| Mapper FFI stub 不存在导致链接错 | 🟡 低 | Phase 2 提供 mock mapper（始终返回 0xFF） |
| **[修订] RAM 随机源逐位不等价（S7）** | 🔴 高 | 固定 seed golden 输出测试锁定；禁用"看起来一样"的实现 |
| **[修订] mapper 区间表未命中时 open bus 行为错（S4）** | 🟠 中 | 未命中返回 open bus；blargg 总线测试覆盖 |

---

## 7. DoD

- [ ] WRAM/VRAM/OAM/Palette 实现 + 镜像测试
- [ ] CPU 总线 `cpu_read`/`cpu_write` 完整（固定区 match + mapper 区间表）
- [ ] PPU 总线 `ppu_read`/`ppu_write` 完整
- [ ] PPU data read buffer 实现
- [ ] Open bus 实现（正确行为，非默认 0）
- [ ] **[修订] RAM 随机源复刻 + 固定 seed golden 输出测试（S7）**
- [ ] **[修订] MapperRangeTable 接口就绪（handler 由 Phase 5 注册）**
- [ ] `cargo test -p vnesu11 bus_tests` 全绿
- [ ] benchmark hot path ≤ 5ns
- [ ] shadow run 与 C++ bus 100 帧零 diff（不含 PPU/APU）

---

## 8. 关键文件交付

```
新增：
  src/rust/crates/vnesu11/src/bus.rs
  src/rust/crates/vnesu11/src/ram.rs
  src/rust/crates/vnesu11/src/ppu/nametable.rs
  src/rust/crates/vnesu11/src/snapshot/mem.rs
  src/rust/crates/vnesu11/tests/bus_tests.rs

修改：
  src/rust/crates/vnesu11/src/soc.rs    # 集成总线
```

下一步：[phase_3_ppu.md](./phase_3_ppu.md)
