# 02 · vNESU11 架构

> **2026-08-10 修订**：本文件已按 `AUDIT_20260810.md` 修订——
> S1（CpuRegsLayout 字段顺序）、S2（SFORMAT savestate）、S3（时序模型决策 A）、
> S4（mapper per-range handler）、S9（影响面 68 站点）。修订点均标注 `[修订]`。

## 1. 设计总览

vNESU11 是一个 **Rust crate**，对外暴露 **C ABI**，把 NES 的三个核心芯片（Ricoh 2A03、2C02、APU）和它们之间的"片上总线"封装到一个 **`VNesSoc`** 结构体里。

```
┌─────────────────────────────────────────────────────────────┐
│  C++ 进程（fceux11.exe / kagami-qa-runner）                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Qt 驱动 / SDL 音视频 / Mapper(~250) / 调试器 / TAS    │ │
│  └─────────────────────────┬──────────────────────────────┘ │
│                            │  FFI（extern "C"）             │
│  ┌─────────────────────────▼──────────────────────────────┐ │
│  │ crates/vnesu11（Rust 静态库）                          │ │
│  │  ┌──────────────────────────────────────────────────┐  │ │
│  │  │ struct VNesSoc {                                │  │ │
│  │  │   cpu: CpuCore,    // 6502 解释器（budget 驱动） │  │ │
│  │  │   ppu: PpuCore,    // 2C02（scanline 段驱动）    │  │ │
│  │  │   apu: ApuCore,    // 5 通道 + DMC              │  │ │
│  │  │   dma: DmaCore,    // OAM DMA + DMC DMA         │  │ │
│  │  │   irq: IrqController,                            │  │ │
│  │  │   scheduler: Scheduler,  // scanline-budget 编排 │  │ │
│  │  │   wram: [u8; 2048], vram: [u8; 2048],          │  │ │
│  │  │   oam: [u8; 256],  palette: [u8; 32],          │  │ │
│  │  │   mapper: *mut c_void,  // FFI 边界（裸指针）  │  │ │
│  │  │   mapper_handlers: MapperRangeTable, // [修订]  │  │ │
│  │  │   xbuf: [u8; 61440],    // 256×240 帧缓冲      │  │ │
│  │  │   sbuf: Vec<i16>,        // 音频样本缓冲       │  │ │
│  │  │ }                                               │  │ │
│  │  │                                                 │  │ │
│  │  │ impl VNesSoc {                                  │  │ │
│  │  │   fn emulate_frame(&mut self) -> TickResult {...│  │ │
│  │  │   fn cpu_read(&mut self, addr: u16) -> u8 {...}│  │ │
│  │  │   fn ppu_read(&mut self, addr: u16) -> u8 {...}│  │ │
│  │  │ }                                               │  │ │
│  │  └──────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 模块边界：什么在 vNESU11 内 / 外

### 2.1 在 vNESU11 内（hot path）

| 模块 | 替换的 C++ | 每周期频率 |
|------|----------|----------|
| `CpuCore`（6502 解释器） | `x6502.cpp` + `cpu.cpp` | 每 CPU 周期（budget 内） |
| `PpuCore`（scanline 段 + 渲染） | `ppu_rendering.cpp`（newppu 路径） | 每 scanline 段 |
| `PpuSpriteLut`（512 KiB） | `ppu_sprite_lut.cpp` | 每 sprite eval |
| `PpuTileFetcher`（template） | `pputile_template.cpp` | 每 visible dot |
| `ApuCore`（5 通道 + DMC） | `apu.cpp` | 每 CPU 周期 |
| `DmaCore`（OAM + DMC） | 散落 `ppu.cpp`/`x6502.cpp`/`apu.cpp` | DMA 期间 |
| `IrqController`（NMI/IRQ + 外部源） | `x6502.cpp` IRQBegin/End | 每指令边界 |
| `Scheduler`（scanline-budget 编排） | `FCEUPPU_Loop`（newppu 主循环） | 每帧 |
| `BusMatrix`（match + mapper 区间表） | `bus.cpp` | 每 read/write |
| `WRAM/VRAM/OAM/Palette`（私有） | `bus.cpp` + `ppu.cpp` | 见上 |

### 2.2 在 vNESU11 外（FFI 边界）

| 模块 | 现有 C++ | 原因 |
|------|---------|------|
| ~250 个 Mapper | `boards/*.cpp` | 多实现、cold path；通过 **per-range handler 注册** 接入 |
| Save state 序列化 | `state.cpp` + `core_state.cpp` | cold path；通过 **SFORMAT tag 等价** 对接 |
| 输入设备 | `input/*.cpp` | 每帧 1 次 |
| 视频输出（Qt） | `drivers/Qt/*.cpp` | 每帧 1 次 |
| 音频输出（SDL） | `sdl-sound.cpp` | 每 audio buffer |
| 调试器 / Lua 钩子 | `debug.cpp` + `lua-engine.cpp` | 条件触发 |
| TAS Editor | `drivers/Qt/TasEditor/*.cpp` | cold path |
| **旧 PPU（newppu=0）** | `ppu.cpp`（旧路径） | **[修订] movie 兼容回退**（见 S5） |

### 2.3 边界判断准则

> **唯一标准**：模块是否在**每 CPU 周期或每 PPU dot** 的 hot path 上？
> - 是 → 进 `VNesSoc`（内联收益 > 抽象成本）
> - 否 → 留外面（FFI 调用频率低，可承受 thunk 开销）

### 2.4 [修订] 系统类型矩阵（S6）

vNESU11 替换 CPU/PPU 后，必须支持 FCEUX11 的全部四种系统类型（它们共用同一
`FCEUPPU_Loop → X6502_Run` 核心）：

| 系统类型 | 与普通 iNES 的差异 | vNESU11 支持方式 | 验证 ROM |
|---------|-------------------|-----------------|---------|
| **iNES** | 基准 | 默认路径 | blargg / 真实游戏 |
| **FDS** | 磁盘 IRQ（`FCEU_IQEXT/EXT2`）+ 额外 RAM | `IrqController` 支持外部 IRQ 源 + FDS 内存映射区 | fds 测试 ROM |
| **NSF** | 仅 CPU+APU，PPU 空转 | `Scheduler` 的"无 PPU"模式（PPU 走空转 stub） | nsf 测试文件 |
| **VS UniSystem** | coin 输入 + 特定 PPU 行为 | `Joypad` 扩展 + PPU 行为开关 | vs 测试 ROM |

**Phase 范围决策**（ADR-007）：v2.0 的 vNESU11 核心必须支持 iNES + FDS + NSF + VS；
NSF 的"无 PPU"模式在 Phase 3 一并交付（PPU 空转 stub 成本低）；FDS 外部 IRQ 在
Phase 4 的 `IrqController` 设计中预留。

---

## 3. 总线矩阵设计

### 3.1 设计决策：固定区 `match` + mapper 区间表（两层）

NES 总线分**两层**，不能只用 `match`：

1. **固定区**（$0000-$401F）：地址拓扑固定，用 `match`（jump table）
2. **mapper 区**（$4020-$FFFF）：mapper 运行时按区间注册 handler（`SetReadHandler`），
   用**小型 per-range 表**（见 §3.3）

```rust
#[inline(always)]
fn cpu_read(&mut self, addr: u16) -> u8 {
    match addr {
        0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
        0x2000..=0x3FFF => self.ppu.read_register((addr & 7) as u8),
        0x4000..=0x4015 => self.apu.read_register((addr & 0x1F) as u8),
        0x4016..=0x4017 => self.joypad.read(addr),
        // 固定区之外的地址全部走 mapper 区间表
        _ => self.mapper_read(addr),
    }
}

#[inline(always)]
fn mapper_read(&mut self, addr: u16) -> u8 {
    // 线性扫描注册的区间（通常 4-16 项，cache 内，~10 条指令）
    for h in &self.mapper_handlers.read_ranges[..self.mapper_handlers.read_count] {
        if addr >= h.start && addr <= h.end {
            return unsafe { (h.fn_ptr)(h.ctx, addr) };
        }
    }
    self.open_bus
}
```

### 3.2 PPU 总线

PPU 地址空间 $0000-$3FFF：

- `$0000-$1FFF`：CHR ROM/RAM（mapper 拥有——走 mapper 区间表）
- `$2000-$2FFF`：nametable（受 mirroring 影响）
- `$3000-$3FFF`：nametable 镜像
- `$3F00-$3FFF`：palette 镜像

nametable mirroring 由 `VNesSoc` 内部处理；mapper 只提供"4-screen VRAM"等决策。

### 3.3 [修订] MapperRangeTable（S4）

**关键事实**（来自 `bus.h:74-93` + `boards/mmc3.cpp:327-344`）：C++ mapper 通过
`SetReadHandler(start, end, fn)` 按**地址区间**注册 handler，单个 mapper 可注册
多个区间（MMC3 就 5+ 个）。因此 vNESU11 不能用"单个 cpu_read thunk"，必须提供
**区间注册表**：

```rust
#[repr(C)]
pub struct RangeHandler {
    pub start: u16,
    pub end: u16,
    pub fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,  // 读 handler
    pub ctx: *mut c_void,
}

pub struct MapperRangeTable {
    read_ranges: [RangeHandler; MAX_RANGES],   // MAX_RANGES = 64
    write_ranges: [RangeHandler; MAX_RANGES],
    read_count: usize,
    write_count: usize,
}
```

FFI 注册接口（替代原方案的 `vnesu11_attach_mapper` 单 vtable）：

```rust
#[no_mangle]
pub extern "C" fn vnesu11_set_read_handler(
    soc: *mut VNesSocOpaque,
    start: u16, end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,
    ctx: *mut c_void,
) -> i32;

#[no_mangle]
pub extern "C" fn vnesu11_set_write_handler(
    soc: *mut VNesSocOpaque,
    start: u16, end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16, u8),
    ctx: *mut c_void,
) -> i32;
```

C++ 侧 `SetReadHandler`/`SetWriteHandler` 的适配器：当 `VNESU11_CORE=ON` 时，
把注册转发到 `vnesu11_set_read_handler`（同一份 `*mut c_void` = mapper 实例指针），
这样 **~250 个 board 文件零改动**。

**Game Genie / cheat 兼容**：cheat 包装逻辑在 C++ 侧 `fceu.cpp::SetReadHandler`
中，适配器保留该包装（Rust 侧只看到被包装后的 handler 指针）。

---

## 4. [修订] 帧调度：scanline-budget 模型（决策 A，S3）

### 4.1 为什么必须复刻 budget 模型

**审计发现（S3）**：FCEUX 当前帧循环是 **PPU-master + CPU-budget** 模型，不是
dot 级紧交错：

- `fceu.cpp:841` 帧入口是 `FCEUPPU_Loop(skip)`
- `ppu_rendering.cpp` 主循环按 **scanline 段**推进，每个段通过 **`X6502_Run(N)`**
  给 CPU 发放周期预算：
  ```cpp
  X6502_Run(256);            // 可见段
  X6502_Run(85);             // pre-render 段
  X6502_Run(256 + 69);       // 可见 + sprite eval
  X6502_Run(16);             // 边界修正
  ```
- CPU 跑到 `tcount >= 预算` 后返回，PPU 继续渲染该段

blargg 81.4% 通过率是这套模型调出来的。**Rust 端若改成 dot 紧交错 = 行为重写**，
不是移植，风险不可控。

**决策 A（ADR-008）**：vNESU11 复刻 budget 模型：
- `CpuCore::run_budget(cycles)` 复刻 `X6502_Run` 语义（跑到 `tcount >= cycles` 返回）
- `PpuCore` 复刻 `FCEUPPU_Loop` 的 scanline 段结构
- `Scheduler` 复刻帧循环骨架（含 `X6502_Run` 的魔法常数，原样保留并注释来源）

**架构预留（决策 B 接口）**：`Scheduler` 的"段模型"内部就是 dot 推进
（`ppu.tick_one_dot`），未来若启用决策 B（dot 紧交错重写，v2.1+ 独立项目），
把段粒度细化为 dot 粒度是**增量改动**，不是推倒重来。段边界仍是 CPU 预算边界。
详见 `DECISIONS_S3_S5_analysis.md` §1.3。

### 4.2 Scheduler 骨架

```rust
impl VNesSoc {
    // 复刻 C++ FCEUPPU_Loop（newppu 路径）的 scanline-budget 结构
    pub fn emulate_frame(&mut self) -> FrameResult {
        self.scheduler.run_frame(self)
    }
}

pub struct Scheduler;

impl Scheduler {
    pub fn run_frame(&self, soc: &mut VNesSoc) -> FrameResult {
        loop {
            // PPU 主驱动：推进一个 scanline 段，产出 (CPU 预算, 渲染动作)
            let segment = soc.ppu.next_segment();
            match segment {
                Segment::Visible { cpu_budget, .. } => {
                    soc.cpu.run_budget(cpu_budget);      // X6502_Run(256)
                    soc.ppu.render_visible();            // 渲染该行
                    soc.apu.tick_budget(cpu_budget);
                }
                Segment::SpriteEval { cpu_budget, .. } => {
                    soc.cpu.run_budget(cpu_budget);      // X6502_Run(69)
                    soc.ppu.render_sprite_eval();
                }
                Segment::PreRender { cpu_budget, .. } => {
                    soc.cpu.run_budget(cpu_budget);      // X6502_Run(85)
                    soc.ppu.render_prerender();
                }
                Segment::VBlank { .. } => {
                    soc.ppu.enter_vblank();
                    soc.cpu.run_budget(vblank_budget);
                    if soc.ppu.frame_ready { return FrameResult::Complete; }
                }
                // ... 其余段（含 overclocking / dendy / skip 语义）
            }
            // DMA / IRQ 路由（每段边界检查）
            soc.dma.poll();
            soc.route_interrupts();
        }
    }
}
```

**关键约束**：
- `X6502_Run` 的魔法常数（256/85/69/16 等）**原样保留**，逐行对照 `ppu_rendering.cpp`
  抄录并注释来源（避免重推导时序）
- overclocking / dendy / frame-skip 语义在 Phase 6 一并复刻
- `emulate_frame(skip)` 的 `skip` 参数语义（跳过渲染/跳过音频）必须保留

### 4.3 帧驱动（`emulate_frame`）

```rust
pub fn emulate_frame(&mut self, skip: i32, xbuf: &mut [u8; 61440], sbuf: &mut Vec<i16>) {
    let result = self.scheduler.run_frame(self, skip);
    if !result.skipped_video {
        xbuf.copy_from_slice(&self.xbuf);
    }
    if !result.skipped_audio {
        sbuf.clear();
        sbuf.extend_from_slice(&self.sbuf);
    }
}
```

---

## 5. FFI 表面（extern "C"）

### 5.1 完整 API 清单

```rust
// === Lifecycle ===
#[no_mangle] pub extern "C" fn vnesu11_create() -> *mut VNesSocOpaque;
#[no_mangle] pub extern "C" fn vnesu11_destroy(soc: *mut VNesSocOpaque);
#[no_mangle] pub extern "C" fn vnesu11_power_on(soc: *mut VNesSocOpaque);
#[no_mangle] pub extern "C" fn vnesu11_reset(soc: *mut VNesSocOpaque);

// === Mapper handler 注册（[修订] 替代原单 vtable）===
#[no_mangle] pub extern "C" fn vnesu11_set_read_handler(soc, start, end, fn_ptr, ctx) -> i32;
#[no_mangle] pub extern "C" fn vnesu11_set_write_handler(soc, start, end, fn_ptr, ctx) -> i32;

// === 系统类型 ===
#[no_mangle] pub extern "C" fn vnesu11_set_system_type(soc: *mut VNesSocOpaque, t: u32) -> i32;  // [修订] S6
#[no_mangle] pub extern "C" fn vnesu11_set_external_irq(soc: *mut VNesSocOpaque, source: u32, on: bool);  // [修订] FDS

// === Emulation ===
#[no_mangle] pub extern "C" fn vnesu11_emulate_frame(
    soc: *mut VNesSocOpaque, skip: i32,
    xbuf: *mut u8, sbuf: *mut i16, sbuf_cap: usize, sbuf_written: *mut usize,
) -> i32;

// === Debugger / Lua / savestate peek/poke ===
#[no_mangle] pub extern "C" fn vnesu11_cpu_peek(soc: *const VNesSocOpaque, addr: u16) -> u8;
#[no_mangle] pub extern "C" fn vnesu11_cpu_poke(soc: *mut VNesSocOpaque, addr: u16, val: u8);
#[no_mangle] pub extern "C" fn vnesu11_cpu_peek_regs(soc: *const VNesSocOpaque, out: *mut CpuRegsLayout);
#[no_mangle] pub extern "C" fn vnesu11_cpu_poke_regs(soc: *mut VNesSocOpaque, regs: *const CpuRegsLayout);
#[no_mangle] pub extern "C" fn vnesu11_ppu_peek(soc: *const VNesSocOpaque, addr: u16) -> u8;
#[no_mangle] pub extern "C" fn vnesu11_joypad_set_button(soc: *mut VNesSocOpaque, pad: u8, btn: u32, pressed: bool);
#[no_mangle] pub extern "C" fn vnesu11_joypad_set_strobe(soc: *mut VNesSocOpaque, strobe: bool);

// === Lua memory hook control ===
#[no_mangle] pub extern "C" fn vnesu11_set_lua_mem_hook(active: bool);

// === Savestate（[修订] SFORMAT tag 驱动，S2）===
// C++ 侧 FCEUSS_SaveMS 保留，但 CPU/PPU/APU 的 SFORMAT 读写改为调用下列函数
#[no_mangle] pub extern "C" fn vnesu11_save_cpu_state(soc: *const VNesSocOpaque, sink: *mut c_void, write_fn: extern "C" fn(*mut c_void, *const u8, usize)) -> i32;
#[no_mangle] pub extern "C" fn vnesu11_load_cpu_state(soc: *mut VNesSocOpaque, source: *mut c_void, read_fn: extern "C" fn(*mut c_void, *mut u8, usize) -> usize) -> i32;
// （PPU/APU 同构，略）
```

### 5.2 [修订] Mapper 适配（S4）

**已废弃**：原方案的 9 函数 `MapperVtable` + `vnesu11_attach_mapper`（无法表达
per-range 注册）。

**新方案**：`vnesu11_set_read_handler` / `vnesu11_set_write_handler` 按区间注册，
见 §3.3。C++ 侧 `SetReadHandler`/`SetWriteHandler` 转发即可，~250 个 board 零改动。

保留的 mapper 元操作走单独窄 vtable：

```rust
#[repr(C)]
pub struct MapperMetaVtable {
    pub mirroring:   unsafe extern "C" fn(*mut c_void) -> u8,
    pub fill_audio:  unsafe extern "C" fn(*mut c_void, *mut i16, usize),
    pub tick_irq:    unsafe extern "C" fn(*mut c_void, *mut bool),
    pub save_state:  unsafe extern "C" fn(*mut c_void, *mut u8, usize, *mut usize) -> i32,
    pub load_state:  unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32,
}
```

### 5.3 [修订] CPU 寄存器布局（S1——字段顺序已修正）

**重要**：以下布局**必须与 `src/x6502struct.h` 逐字段一致**（含 `FCEUDEF_DEBUGGER`
条件字段）。实现时**禁止手写**——用 `offset_of!` 断言 + golden 二进制对比校验。

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpuRegsLayout {
    pub tcount: i32,       // offset 0
    pub PC: u16,           // offset 4
    pub A: u8,             // offset 6
    pub X: u8,             // offset 7
    pub Y: u8,             // offset 8
    pub S: u8,             // offset 9
    pub P: u8,             // offset 10
    pub moo_pi: u8,        // offset 11
    pub jammed: u8,        // offset 12
    // padding 13-15（count 需 4 字节对齐）
    pub count: i32,        // offset 16
    pub irq_low: u32,      // offset 20
    pub db: u8,            // offset 24
    // padding 25-27
    pub preexec: i32,      // offset 28
    // ---- 仅 FCEUDEF_DEBUGGER 构建（src/CMakeLists.txt:85 定义）----
    pub cpu_hook: *mut c_void,   // offset 32
    pub read_hook: *mut c_void,  // offset 40
    pub write_hook: *mut c_void, // offset 48
    // padding 56-63（alignas(64)）
}
```

**校验方式（Phase 0 强制）**：
```rust
// 逐字段 offset 断言（禁止依赖 repr(C) 自动布局的假设）
const _: () = {
    assert!(offset_of!(CpuRegsLayout, tcount)   == 0);
    assert!(offset_of!(CpuRegsLayout, PC)       == 4);
    assert!(offset_of!(CpuRegsLayout, A)        == 6);
    assert!(offset_of!(CpuRegsLayout, jammed)   == 12);
    assert!(offset_of!(CpuRegsLayout, count)    == 16);
    assert!(offset_of!(CpuRegsLayout, irq_low)  == 20);
    assert!(offset_of!(CpuRegsLayout, db)       == 24);
    assert!(offset_of!(CpuRegsLayout, preexec)  == 28);
    // FCEUDEF_DEBUGGER 构建下：
    assert!(offset_of!(CpuRegsLayout, cpu_hook) == 32);
    assert!(offset_of!(CpuRegsLayout, read_hook) == 40);
    assert!(offset_of!(CpuRegsLayout, write_hook) == 48);
    assert!(size_of::<CpuRegsLayout>() == 64);
};
```

---

## 6. 与现有 C++ 代码的集成

### 6.1 [修订] 影响面（S9）

**审计事实**：`x6502.h`（24 处）、`sound.h`（17）、`ppu.h`（16）、`bus.h`（8）、
`x6502struct.h`（3）——共 **68 个文件直接 include 核心头**，分布在 mappers /
drivers / 调试器 / movie / netplay，**不止 Qt 驱动**。

因此：
- `FCEUI_*` 兼容垫片保留（~150 个调用点零改动）——**但这是必要非充分条件**
- Phase 0 必须产出 **`core_headers_deps.md`**：68 个 include 站点逐一登记
  （依赖哪个头、读哪些符号、迁移后走哪条路径：FFI / 兼容别名 / 直接迁移）
- 每个站点在对应 phase 的 DoD 里有一条"迁移确认"项

### 6.2 [修订] Savestate 兼容（S2——SFORMAT tag 驱动）

**审计事实**：`state.cpp:122-142` 的 savestate 是 **V2 chunked 格式**：
`[4 字节 tag][4 字节 size][数据]`，CPU 拆成 `SFCPU` + `SFCPUC` 两块，逐字段带标签读写。
**不是 64 字节整块 memcpy**。

**兼容策略**：
1. **Phase 0 前置**：产出 `savestate_tags.md`——穷举全部 SFORMAT tag 清单：
   - `SFCPU`：PC / A / X / Y / S / P / DB / RAM（0x800 间接）
   - `SFCPUC`：JAMM / IQLB / ICoa / ICou / TSBS / MooP
   - `FCEUPPU_STATEINFO`（旧 PPU）、`FCEU_NEWPPU_STATEINFO`（新 PPU）
   - `FCEUSND_STATEINFO`（APU）、`FCEUCTRL_STATEINFO`（控制器）
   - mapper 各自的注册 chunk
2. **Rust 端按 tag 实现等价 save/load**：内部布局自由，序列化时按 tag 输出相同字节
3. **验证**：golden savestate 文件 round-trip 字节一致（不依赖结构体布局）
4. `Cpu::layout_` 的 `static_assert`（offset 0 / size 64）保护的是 **oldmovie 等旧代码**
   的整块复制路径，与 SFORMAT 序列化**无关**——两套兼容机制都要保持，但验证方法不同

### 6.3 Lua 内存钩子

```cpp
// C++ 端保留 Lua hook 注册
void FCEUI_LuaRegisterMemHook(...) { vnesu11_set_lua_mem_hook(true); /* 注册细节保持 C++ */ }
```

实际 hook 函数仍由 Lua 引擎（Rust mlua）通过 FFI 注册到 vNESU11。

### 6.4 [修订] newppu 策略（S5）

**审计事实**：`newppu` 运行时切换（`fceu.cpp:136-147`）；GUI 默认 0（旧 PPU）；
kagami-qa blargg baseline 用 1（新 PPU）；movie 记录 `PPUflag`（`movie_io.cpp:138`）。

**决策（ADR-009，选项③）**：
- vNESU11 的 Rust PPU **只实现 newppu=1 路径**（QA 验证的就是它）
- **C++ 旧 PPU（ppu.cpp 旧路径）保留**为 movie 兼容回退：当 `newppu=0` 时
  vNESU11 不接管 PPU（CPU/APU 仍接管，PPU 走 C++ 旧路径）
- 这意味着 vNESU11 的 `PpuCore` 按 `ppu_rendering.cpp`（新 PPU）移植；
  `ppu.cpp` 的旧 PPU 路径**不在迁移范围**，Phase 8 也不删除旧 PPU 代码
- 影响：`newppu=0` 的 movie 照常播放（走 C++ 旧 PPU）；性能收益只在 newppu=1 时体现
- 代价：Phase 8 "清理"目标改为"清理 newppu=1 路径的 C++ 代码"，旧 PPU 保留

### 6.5 [修订] FDS / NSF / VS（S6）

- `vnesu11_set_system_type(NSF)` 时：PPU 走空转 stub（CPU/APU 照常）
- FDS：`vnesu11_set_external_irq(EXT/EXT2, on)` 供磁盘 IRQ；FDS 的 32 KiB RAM
  映射区通过 mapper handler 注册（FDS 以虚拟 mapper 形式接入）
- VS：joypad 扩展 + coin 输入走 `vnesu11_joypad_*` 扩展参数

---

## 7. 内部 crate 结构

```
crates/vnesu11/
├── Cargo.toml                      # crate 元数据
├── build.rs                        # cbindgen 输出 vnesu11_ffi.h（C++ include 用）
├── cbindgen.toml                   # cbindgen 配置
├── src/
│   ├── lib.rs                      # pub mod 声明
│   ├── soc.rs                      # struct VNesSoc + emulate_frame()
│   ├── ffi.rs                      # extern "C" 表面 + Opaque
│   ├── scheduler.rs                # scanline-budget 编排（复刻 FCEUPPU_Loop）
│   ├── cpu/
│   │   ├── mod.rs                  # struct CpuCore + run_budget()
│   │   ├── decoder.rs              # 6502 opcode 表 + dispatch
│   │   ├── addressing.rs           # 寻址模式
│   │   ├── ops_*.rs                # 按指令族拆
│   │   └── regs.rs                 # CpuRegsLayout + offset_of 断言
│   ├── ppu/
│   │   ├── mod.rs                  # struct PpuCore + 段驱动
│   │   ├── segments.rs             # [修订] scanline 段枚举（S3）
│   │   ├── registers.rs            # $2000-$2007 + 内部 v/t/x/w
│   │   ├── rendering.rs            # background + sprite compositing
│   │   ├── sprite_lut.rs           # 512 KiB sprite LUT（LazyLock）
│   │   ├── tile_fetch.rs           # nametable/attr/pattern fetch
│   │   └── oam.rs                  # OAM + secondary OAM
│   ├── apu/
│   │   ├── mod.rs                  # struct ApuCore
│   │   ├── pulse.rs / triangle.rs / noise.rs / dmc.rs
│   │   └── frame_counter.rs
│   ├── dma/
│   │   ├── mod.rs                  # struct DmaCore
│   │   └── oam_dma.rs
│   ├── irq.rs                      # struct IrqController（含外部源 EXT/EXT2）
│   ├── bus.rs                      # 固定区 match + MapperRangeTable
│   ├── mapper.rs                   # RangeHandler / MapperMetaVtable
│   ├── ram.rs                      # WRAM/VRAM/OAM/Palette 私有
│   ├── joypad.rs                   # $4016/$4017
│   └── snapshot/
│       ├── mod.rs                  # [修订] SFORMAT tag 驱动序列化（S2）
│       └── regs.rs                 # CpuRegsLayout <-> X6502 值映射
├── tests/
│   ├── cpu_tests.rs                # blargg cpu_instrs 子集
│   ├── ppu_tests.rs                # blargg ppu_tests 子集（newppu 模式）
│   ├── apu_tests.rs
│   ├── bus_tests.rs
│   ├── savestate_roundtrip.rs      # [修订] golden tag round-trip（S2）
│   └── systems/
│       ├── fds_tests.rs            # [修订] S6
│       ├── nsf_tests.rs            # [修订] S6
│       └── vs_tests.rs             # [修订] S6
└── benches/
    ├── cpu_decode.rs
    ├── ppu_segment.rs              # [修订] 段渲染基准（S3）
    └── mapper_range_scan.rs        # [修订] 区间表线性扫描基准（S4）
```

---

## 8. 与 Phase 的对应

| 阶段 | 实现内容 |
|------|---------|
| Phase 0 | crate 骨架、FFI 表面空实现、**SFORMAT tag 清单 + golden round-trip**、**offset_of 布局校验**、**core_headers_deps.md** |
| Phase 1 | `cpu/` 全套（budget 驱动的解释器） |
| Phase 2 | `bus.rs` + `ram.rs` + **MapperRangeTable** + **RAM 随机源复刻** |
| Phase 3 | `ppu/` 全套（**newppu 段驱动** + 渲染 + 精灵 LUT） |
| Phase 4 | `apu/` + `dma/` + `irq.rs`（**含外部 IRQ 源**）+ scheduler |
| Phase 5 | mapper per-range 适配 + **Game Genie 包装保留** |
| Phase 6 | 接入主路径（shadow run，帧级三级 diff） |
| Phase 7 | 切换默认（`VNESU11_CORE=ON`） |
| Phase 8 | 清理 newppu=1 路径的 C++ 代码（**旧 PPU 保留**） |

下一步：[phase_0_foundation.md](./phase_0_foundation.md)
