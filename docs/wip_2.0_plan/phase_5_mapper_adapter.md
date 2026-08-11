# Phase 5 · 前置接线 + Mapper 适配层

> **目标**：先补完 Phase 0-4 复核发现的两项前置缺口（SoC 主循环未驱动
> APU/DMA/Joypad、精灵渲染未写 frame_buffer），再让 vNESU11 与 ~250 个
> C++ mapper 通过 **per-range handler 注册**（`SetReadHandler`/`SetWriteHandler`
> 转发）对接，**不动 mapper 代码**。
>
> **2026-08-10 修订（审计 S4）**：mapper 适配改为区间注册表（原 9 函数
> `MapperVtable` 废弃）。
>
> **2026-08-11 复核修订（新增阶段 0）**：Phase 0-4 精细复核确认——
> ① `run_frame()` 未调用 `apu.tick()`，`$4000-$4015` APU 寄存器未路由到
> `ApuCore`，`$4016/$4017` 仍用旧 joypad 字段，`$4014` 仍是简化拷贝；
> ② `sprite.rs::render_scanline` 用占位 pattern 数组、结果未写入
> frame_buffer。这两项是 mapper 适配的前提（mapper 的 `fill_audio`/`tick_irq`
> 依赖 APU/IRQ 被驱动；CHR handler 依赖 PPU 渲染真正消费图案数据），
> 故纳入本 phase 阶段 0 先行完成。

## 工期：4 周（阶段 0 接线 1 周 + 阶段 1 mapper 适配 3 周）

---

## 1. 范围

### 1.0 [新增 2026-08-11] 阶段 0 · 前置接线（在范围内）

- ✅ SoC 主循环驱动 APU / DMA / IRQ / Joypad（`run_frame` 每 segment tick）
- ✅ `$4000-$4015` APU 寄存器读写路由到 `ApuCore` 5 通道
- ✅ `$4014` OAM DMA 走 `DmaCore::oam.start()`（513/514 stall）
- ✅ `$4016/$4017` 走 `JoypadState`；删除旧 `joypad_latched` 等字段
- ✅ 精灵真实合成（`sprite.rs` 读 SoC CHR 缓存、结果写 frame_buffer）
- ✅ 接线端到端测试（APU 输出受寄存器写影响、精灵像素非零）

### 1.1 ✅ Mapper 适配（阶段 1，原 Phase 5 范围）

- vNESU11 侧 `MapperRangeTable`（read/write 区间注册表，`#[repr(C)]`）
- FFI：`vnesu11_set_read_handler` / `vnesu11_set_write_handler`
- C++ 侧 `SetReadHandler`/`SetWriteHandler` 转发（`bus.h:92-93`）
- **Game Genie / cheat 包装保留**（在 C++ 侧适配器，Rust 只看到包装后的 handler）
- mapper 元操作（mirroring / fill_audio / tick_irq / save / load）走窄 `MapperMetaVtable`
- mapper 的 savestate 序列化（`save_mapper_state` 已存在，`kagami_bridge` 有 FFI）
- FDS 虚拟 mapper（磁盘 IRQ + 32 KiB RAM 映射区）

### 1.2 ❌ 不在范围内

- mapper 实现本身（继续 C++，永远不做）
- mapper 重写
- 旧 PPU 路径的 mapper 适配（`newppu=0` 时 vNESU11 不接管 PPU，见 ADR-009）
- DMC DMA 完整仲裁（仍属 Phase 4 遗留，阶段 0 只接 OAM DMA；DMC 仲裁并入
  Phase 6 shadow-run 集成）

---

## 2. 任务清单

### 2.0 [新增 2026-08-11] 阶段 0 · 前置接线

#### 2.0.1 `run_frame()` 驱动 APU / DMA / IRQ / Joypad

```rust
// crates/vnesu11/src/soc.rs
pub fn run_frame(&mut self) -> FrameResult {
    loop {
        let segment = self.ppu.next_segment();
        match segment {
            Segment::FrameComplete => { result.completed = true; break; }
            _ => {
                let budget = segment.cpu_budget();
                // 1. DMA stall 优先：OAM DMA 期间 CPU 停摆（513/514 周期）
                if self.dma.is_stalling() {
                    self.dma.oam.step();          // 走完一字节
                    continue;                      // CPU 不推进
                }
                // 2. CPU 按预算运行
                let mut bus = unsafe { VNesBusContext::new(self) };
                self.cpu.run_budget(budget, &mut bus);
                // 3. APU 按预算 tick（含 frame counter / 5 通道 / mixer）
                self.apu.tick(budget as u32);
                // 4. PPU 渲染该段
                let frame_done = self.ppu.advance_to_next_segment();
                // 5. IRQ 路由：APU(帧计数/DMC) + mapper(元 vtable) + 外部(EXT/EXT2)
                self.route_interrupts();
                if frame_done { result.completed = true; break; }
            }
        }
    }
    // ... 帧缓冲拷贝 + NMI 桥接（保留现状）
}
```

**验收**：`cargo test -p vnesu11 apu_tests::soc_*` 端到端通过（见 §3.0）。

#### 2.0.2 `$4000-$4015` APU 寄存器路由

```rust
// crates/vnesu11/src/bus.rs
fn apu_io_write(&mut self, addr: u16, val: u8) {
    match addr {
        0x4000..=0x4003 => self.apu.pulse1.write_control(val),   // $4000
        // $4001 sweep / $4002 timer_lo / $4003 timer_hi
        0x4004..=0x4007 => self.apu.pulse2.write_...(val),
        0x4008 => self.apu.triangle.write_control(val),
        0x400A => self.apu.triangle.write_timer_lo(val),
        0x400B => self.apu.triangle.write_timer_hi(val),
        0x400C => self.apu.noise.write_envelope(val),
        0x400E => self.apu.noise.write_period(val),
        0x400F => self.apu.noise.write_length(val),
        0x4010 => self.apu.dmc.write_control(val),
        0x4011 => self.apu.dmc.write_load(val),
        0x4012 => self.apu.dmc.write_address(val),
        0x4013 => self.apu.dmc.write_length(val),
        0x4014 => self.oam_dma_start(val),        // → DmaCore
        0x4015 => self.apu_status_write(val),      // 通道 enable + IRQ 清位
        0x4017 => { self.apu.frame_counter.write(val); self.joypad.write_strobe(val); }
        _ => {}
    }
}
// $4016 写只走 joypad strobe（注意与 $4017 区分：$4016 写 strobe，
// $4017 写 APU frame counter + 只读 joypad 2 由 $4017 读返回）
```

**读侧**：`$4015` 返回通道 enable + IRQ flag；`$4016/$4017` 走 `JoypadState::read`；
其余寄存器读返回 open bus（硬件行为）。

#### 2.0.3 Joypad 旧字段清理

删除 `soc.rs` 中 `joypad_latched` / `joypad_strobe` / `joypad_strobe_latch` /
`joypad_shift` 四个旧字段，统一走 `self.joypad: JoypadState`。同时更新
`tests/bus_tests.rs` 中引用旧字段的测试（`joypad_strobe_clears_then_latches_first_button`
等改为走 `JoypadState` API）。

#### 2.0.4 OAM DMA 接入

```rust
fn oam_dma_start(&mut self, page: u8) {
    let cycle_odd = (self.cpu.tcount & 1) == 1;   // 触发周期奇偶 → 513/514
    self.dma.oam.start(page, cycle_odd);
    // 实际字节搬移由 run_frame 的 stall 路径执行：
    //   bus.read((page << 8) | offset) → ppu.oam[offset]
}
```

**注意**：`DmaCore::oam.step()` 当前返回占位 `Some(0)`，阶段 0 需改为真正
读总线并写 `ppu.oam`（Phase 2 的 `cpu_read_for_dma` 可复用）。

#### 2.0.5 精灵真实合成（Phase 3 (c) 补完）

```rust
// crates/vnesu11/src/ppu/sprite.rs
// 替换 render_scanline 中的占位：
//   let sprite_pattern_lo: [u8; 8192] = [0; 8192];   // ❌ 删除
// 改为读取 SoC CHR 缓存（PpuCore::pattern_lo/hi，Phase 3 (a) 已建字段）：
fn render_scanline(&mut self, scanline, frame_buffer, oam, ppuctrl, ppumask,
                   pattern_lo: &[u8], pattern_hi: &[u8],  // ← 新增参数
                   compositor) {
    // ... 原有评估逻辑保留 ...
    // 对每个 loaded sprite，用 pattern_lo/hi 取图案字节：
    let tile_offset = (tile_id as usize) * 16 + (row_in_tile as usize);
    let lo = pattern_lo[tile_offset];
    let hi = pattern_hi[tile_offset];
    let color = (bit_hi << 1) | bit_lo;
    // 合成结果写入 frame_buffer（用 Compositor::compose + 优先级）：
    if color != 0 {
        frame_buffer[target_offset + pixel_x as usize] = ...;
        // sprite 0 hit 检测（compositor.check_sprite_zero_hit）
    }
}
```

**注意**：PpuCore 调用处需把 `self.pattern_lo/hi` 传给 `spr.render_scanline`
（借 OR 复制，避免借用冲突——Phase 3 (a) 已用过 `let palette = *...` 模式）。

#### 2.0.6 端到端测试（新增）

```
crates/vnesu11/tests/apu_tests.rs（扩展）
├── soc_apu_register_write_drives_output   // 写 $4000/$4002/$4003 → run_frame →
│                                            // apu.output_buffer 有非零样本且随寄存器变化
├── soc_joypad_4016_reads_new_state         // soc.joypad.set_button + cpu_read($4016)
├── soc_oam_dma_via_4014                    // cpu_write($4014, page) → oam 搬移 + stall 计数
└── soc_apu_frame_irq_asserts_irq           // run_frame 多帧后 apu frame IRQ → irq.aggregate_mask

crates/vnesu11/tests/ppu_tests.rs（扩展）
└── sprite_render_produces_pixels           // 填 OAM + CHR 图案 → run_frame → 对应像素非零
```

### 2.1 [原，已实现需验证] Rust 侧：MapperRangeTable

> **状态**：`crates/vnesu11/src/mapper.rs` 已实现（Phase 0/2 落地）——
> `MAX_RANGES=64`、`ReadRangeHandler`/`WriteRangeHandler`、`read()/write()`
> 线性扫描、`clear()`、`MapperMetaVtable`。阶段 1 只需补
> `tests/mapper_tests.rs`（见 §3.1）并核对 `#[repr(C)]` 布局经 cbindgen 导出。

```rust
// crates/vnesu11/src/mapper.rs
pub const MAX_RANGES: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ReadRangeHandler {
    pub start: u16,
    pub end: u16,
    pub fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,
    pub ctx: *mut c_void,
}
// WriteRangeHandler 同构（fn_ptr: fn(*mut c_void, u16, u8)）
```

**为什么不优化成哈希/二分**：区间数通常 4-16 个（MMC3 才 5+），线性扫描命中
首项即返回，分支预测完美，cache 内约 10 条指令。benchmark 目标见 §4。

### 2.2 [原，已实现需验证] FFI 注册接口

> **状态**：`vnesu11_set_read_handler` / `vnesu11_set_write_handler` /
> `vnesu11_clear_mapper_handlers` / `vnesu11_attach_mapper_meta` 已在
> `ffi.rs` 实现并经 cbindgen 导出。阶段 1 验证 FFI 层 null 安全 + 容量上限
> 返回码（`-1` null、`-2` 超限）。

### 2.3 [原，已实现需验证] 元操作 vtable

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MapperMetaVtable {
    pub mirroring:   unsafe extern "C" fn(*mut c_void) -> u8,
    pub fill_audio:  unsafe extern "C" fn(*mut c_void, *mut i16, usize),
    pub tick_irq:    unsafe extern "C" fn(*mut c_void, *mut bool),
    pub save_state:  unsafe extern "C" fn(*mut c_void, *mut u8, usize, *mut usize) -> i32,
    pub load_state:  unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32,
}
```

**阶段 0 依赖**：`fill_audio` 需要 APU 已被 `run_frame` 驱动（2.0.1）；
`tick_irq` 需要 IRQ 路由接通。故接线先行。

### 2.4 [原] C++ 侧：SetReadHandler/SetWriteHandler 转发

**关键**：`bus.cpp` 的 `SetReadHandler` 是 mapper 注册的唯一入口（`bus.h:92-93`）。
让它在 `VNESU11_CORE=ON` 时转发：

```cpp
// src/bus.cpp
void Bus::set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept {
#ifdef VNESU11_CORE_ENABLED
    // vNESU11 接管：转发到 Rust 区间表
    vnesu11_set_read_handler(g_vnesu11_soc,
        static_cast<uint16_t>(start), static_cast<uint16_t>(end),
        fn, /* ctx */ g_cart.get());
    // 注意：C++ 侧 aread_ 表仍写入，供 newppu=0 回退路径使用
#endif
    for (uint32_t a = start; a <= end; ++a) {
        aread_[a] = fn;
    }
}
```

**Game Genie / cheat**：`fceu.cpp::SetReadHandler` 在调用 `bus.set_read_handler`
之前做 genie 包装。适配器**保留该包装顺序**——Rust 端只看到包装后的 `fn` 指针。
cheat 状态仍由 C++ 管理。

**双写策略**：`aread_[]` 表同时更新，保证 `newppu=0` 回退路径（C++ 旧 PPU +
C++ bus）仍然可用。vNESU11 接管时走 Rust 区间表；回退时走 C++ 表。两套并存
不冲突（内存开销 1 MiB 维持现状）。

### 2.5 [原] 与 LoadGame 集成

```cpp
// src/fceu.cpp 修改 LoadGame 路径
FCEUGI* fceu11::LoadGame(const char* path, int slot, bool silent) {
    // ... 现有 iNES 解析 ...
#ifdef VNESU11_CORE_ENABLED
    vnesu11_clear_mapper_handlers(g_vnesu11_soc);
    vnesu11_attach_mapper_meta(g_vnesu11_soc, g_cart.get(), &MapperMetaAdapter::vtable());
    vnesu11_set_system_type(g_vnesu11_soc, GameInfo->type);  // iNES/FDS/NSF/VS
#endif
    // mapper 的 Power() 内部会调用 SetReadHandler/SetWriteHandler →
    // 自动转发到 vNESU11 区间表
    GameInterface(GI_POWER);
    return gi;
}
```

**为什么不用显式遍历 mapper**：mapper 的 `Power()` 回调内部注册 handler，
转发机制让它们**自动落入** Rust 区间表，~250 个 board 零改动。

### 2.6 [原] savestate 通过 mapper

```cpp
// src/kagami_bridge.cpp 已有 kagami_bridge_save_mapper_state
// vNESU11 路径：走 MapperMetaVtable::save_state
int kagami_bridge_save_mapper_state(uint8_t* dst, uint32_t cap, uint32_t* written) {
#ifdef VNESU11_CORE_ENABLED
    return vnesu11_save_mapper_state(g_vnesu11_soc, dst, cap, written);
#else
    // 现有实现（save_mapper_state 已存在）
#endif
}
```

---

## 3. 验证策略

### 3.0 [新增 2026-08-11] 接线端到端测试

```
crates/vnesu11/tests/
├── apu_tests.rs（扩展）     # 见 §2.0.6 —— run_frame 驱动 APU 的端到端验证
└── ppu_tests.rs（扩展）     # 精灵真实像素输出（sprite_render_produces_pixels）
```

**关键断言**（阶段 0 完成标准）：
- 写 APU 寄存器 → `run_frame` 一帧 → `apu.output_buffer` 非空且随频率/音量变化
- 不写任何 APU 寄存器 → output_buffer 全 0（静音）
- `cpu_read($4016)` 反映 `JoypadState::set_button` 状态
- `cpu_write($4014, page)` → `ppu.oam` 256 字节搬移正确 + `dma.total_stall_cycles()==513/514`
- OAM 填精灵 + CHR 填图案 → `run_frame` → `frame_buffer` 对应像素非零

### 3.1 [原] 单元测试

```
crates/vnesu11/tests/mapper_tests.rs
├── test_null_mapper          // 无 handler 注册时读 $8000 返回 open bus
├── test_single_range         // 注册一个区间，读命中/未命中
├── test_overlapping_ranges   // 重叠区间按注册顺序优先
├── test_range_capacity       // 超 MAX_RANGES 返回 -1
└── test_clear_handlers       // 清空后再读回 open bus
```

### 3.2 [原] 集成测试：每个 mapper 跑一遍 SMB1

```
tests/integration_mapper.rs
└── 对 src/boards/ 所有 mapper，加载 SMB1，验证：
    - 启动画面正确
    - 帧缓冲 60 帧后 CRC 与 v1.17 一致
    - savestate round-trip 成功
```

### 3.3 [原] mapper_byte_diff 测试

`kagami-qa::runner::mapper_byte_diff` 已有 175-case 测试；Phase 5 通过 vNESU11
mapper 适配器走通：

- 每个 mapper 跑 N 帧
- `save_mapper_state()` 字节 diff 与 C++ 基线对比
- 必须 100% parity

### 3.4 [原] Game Genie 单测

- 启用一个 cheat code，确认 `SetReadHandler` 包装后的 handler 仍被正确转发
- 关掉 cheat，确认原始 handler 恢复

---

## 4. 性能基准

| 项目 | 目标 |
|------|------|
| 区间表 read（命中首项） | ≤ 10ns |
| 区间表 read（全扫未命中） | ≤ 20ns |
| 与 C++ `ARead[addr](addr)` 对比 | ≤ 现状（一次间接跳转） |
| mapper 整体开销 | ≤ 5% CPU cycles |
| **[新增] run_frame 全段（CPU+APU+PPU+DMA+IRQ）** | ≤ 16.67ms（60 FPS） |

bench: `cargo bench -p vnesu11 mapper_range_scan` + 阶段 0 新增 `soc_frame_bench`

---

## 5. 关键技术决策

### 5.0 [新增 2026-08-11] 接线先于适配的理由

mapper 适配的价值依赖"vNESU11 真正能跑一帧"：
- `fill_audio` 需要 APU 被 `run_frame` 驱动，否则扩展音频无输出
- `tick_irq` 需要 IRQ 路由接通，否则 mapper IRQ 无法注入 CPU
- CHR handler 需要 PPU 渲染真正消费图案数据，否则 mapper 的 CHR 映射无观察点
若跳过接线直接做适配，集成测试（§3.2）无法区分"mapper 错"还是"下游没接线"。

### 5.1 [原] 不在 Rust 里实现 mapper

mapper 是"卡带"的硬件描述，~250 个不同硬件实现，**永远不可能在 Rust 里重写**。
本 phase 只做适配层。

### 5.2 [原] 为什么用区间表而不是 vtable

**审计事实（S4）**：真实 mapper 机制是 `SetReadHandler(start, end, fn)` 按区间
注册多个 handler（MMC3 注册 5+ 个），且是**动态注册**（`Power()` 回调时）。
单个 `cpu_read` vtable 无法表达"同一 mapper 不同地址区间走不同函数"。

### 5.3 [原] 双写 aread_ 表的理由

`newppu=0` 回退路径（C++ 旧 PPU）仍用 `ARead[]` 表。双写保证：
- vNESU11 接管（newppu=1）：走 Rust 区间表
- 回退（newppu=0）：走 C++ 表
- 切换发生在 LoadGame/Reset 边界，无并发问题

### 5.4 [原] cbindgen 生成 vnesu11/mapper.h

```toml
# crates/vnesu11/cbindgen.toml
[parse]
include = ["src"]

[export]
include = ["RangeHandler", "WriteRangeHandler", "MapperMetaVtable", "Mirroring"]
```

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| **[新增] 接线后 APU 时序与 C++ 不同步** | 🟠 中 | 帧计数器对齐 C++ `X6502_Run` 预算边界；Phase 6 shadow run 兜底 |
| **[新增] 借用冲突（run_frame 内 APU/PPU 双可变借用）** | 🟠 中 | 阶段 0 用局部拷贝（`let palette = *...` 模式）或拆借；测试先行 |
| 区间表线性扫描性能不达标 | 🟠 中 | benchmark；命中首项路径是主路径，10ns 内 |
| 某个 mapper 注册超 64 区间 | 🟡 低 | MAX_RANGES=64 远超现状（MMC3 才 5+）；超限报错 |
| Game Genie 包装顺序错 | 🟠 中 | C++ 侧保留包装，Rust 只看到包装后 handler；单测覆盖 |
| 双写 aread_ 表内存翻倍 | 🟡 低 | 1 MiB 维持现状，可接受 |
| FDS 虚拟 mapper 时序错 | 🟠 中 | fds 测试 ROM 优先验证 |

---

## 7. DoD

### 阶段 0（接线）DoD — [新增 2026-08-11]

- [ ] `run_frame()` 每 segment 驱动 `apu.tick(budget)` + DMA stall + IRQ 路由
- [ ] `$4000-$4015` APU 寄存器读写全部路由到 `ApuCore` 5 通道
- [ ] `$4014` OAM DMA 走 `DmaCore::oam.start()`（513/514 stall + 实际搬移）
- [ ] `$4016/$4017` 走 `JoypadState`；旧 `joypad_latched` 等 4 字段删除
- [ ] `sprite.rs::render_scanline` 读 CHR 缓存、结果写入 frame_buffer
- [ ] 端到端测试：APU 输出受寄存器写影响 / 精灵像素非零（§3.0）
- [ ] `cargo test -p vnesu11` 全绿（含扩展的 apu_tests / ppu_tests / bus_tests）

### 阶段 1（mapper 适配）DoD — [原]

- [ ] `MapperRangeTable` 验证 + `tests/mapper_tests.rs` 全绿
- [ ] `SetReadHandler`/`SetWriteHandler` 转发（含 Game Genie 包装保留）
- [ ] `MapperMetaVtable` + `vnesu11_attach_mapper_meta` 验证
- [ ] LoadGame 集成（`vnesu11_clear_mapper_handlers` + 系统类型 + GI_POWER 自动注册）
- [ ] mapper_byte_diff 175-case 通过 vNESU11 全 PASS
- [ ] 主流 mapper (NROM/MMC1/MMC3/VRC2/VRC6/UxROM/CNROM) 跑 SMB1 一致
- [ ] savestate round-trip 100% parity
- [ ] 帧时间开销 ≤ 5% baseline
- [ ] FDS 虚拟 mapper 跑通 fds 测试 ROM

---

## 8. 关键文件交付

### 阶段 0（接线）— [新增 2026-08-11]

```
新增：
  crates/vnesu11/tests/mapper_tests.rs   # 阶段 1 用（见下）

修改：
  crates/vnesu11/src/soc.rs      # run_frame 驱动 APU/DMA/IRQ + route_interrupts()
  crates/vnesu11/src/bus.rs      # apu_io_read/write 路由 + $4014 DMA + $4016/17 joypad
  crates/vnesu11/src/ppu/sprite.rs # render_scanline 读 CHR + 写 frame_buffer
  crates/vnesu11/src/ppu/mod.rs  # 传 pattern_lo/hi 给 sprite.render_scanline
  crates/vnesu11/src/dma/oam_dma.rs # step() 真正读总线搬 OAM
  crates/vnesu11/src/soc.rs      # 删除旧 joypad 4 字段
  crates/vnesu11/tests/apu_tests.rs  # 端到端：寄存器→output_buffer
  crates/vnesu11/tests/ppu_tests.rs  # 端到端：精灵像素
  crates/vnesu11/tests/bus_tests.rs  # 适配 JoypadState 新 API
```

### 阶段 1（mapper 适配）— [原]

```
新增：
  src/vnesu11_mapper_adapter.h
  src/vnesu11_mapper_adapter.cpp
  crates/vnesu11/tests/mapper_tests.rs
  crates/vnesu11/benches/mapper_range_scan.rs

修改：
  src/bus.cpp                         # SetReadHandler/SetWriteHandler 转发
  src/fceu.cpp                        # LoadGame 集成
  src/CMakeLists.txt                  # vNESU11 mapper adapter 加入构建
  crates/vnesu11/src/mapper.rs        # [已验证存在] MapperRangeTable
  crates/vnesu11/src/ffi.rs           # [已验证存在] 区间注册接口
```

---

下一步（阶段 0 完成后）：[phase_6_integration.md](./phase_6_integration.md)
