# Phase 4 · APU + DMA + IRQ + Clock

> **目标**：补齐 vNESU11 的最后几个内部子模块：APU 5 通道、OAM/DMC DMA、IRQ 控制器、时钟树。这阶段让 `VNesSoc` 真正能驱动完整仿真。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S6 修订——
> IRQ 控制器增加**外部 IRQ 源**（FDS `FCEU_IQEXT/EXT2`），Joypad 支持 VS coin 输入。

## 工期：4 周

---

## 1. 范围

### 1.1 ✅ 在范围内
- APU 5 通道：Pulse ×2、Triangle、Noise、DMC
- APU 帧计数器：4-step / 5-step
- APU 非线性 mixer（标准查找表）
- APU 输出缓冲（`sbuf`）
- OAM DMA（$4014 写入触发，513 或 514 周期 stall CPU）
- DMC DMA（每周期 stall CPU 1-4 周期）
- IRQ 控制器（NMI edge + IRQ level + **[修订] 外部 IRQ 源 EXT/EXT2（FDS）**）
- 时钟树（scanline-budget 编排，见 ADR-008）
- Joypad strobe（$4016/$4017 + **[修订] VS coin 输入扩展**）

### 1.2 ❌ 不在范围内
- 扩展音频（VRC6/VRC7/N163/Sunsoft 5B）——由 mapper FFI 提供 `fill_audio`
- 音频后处理（低通滤波器）——Phase 6 由 Qt 驱动处理

---

## 2. 任务清单

### 2.1 模块结构

```
crates/vnesu11/src/
├── apu/
│   ├── mod.rs              # struct ApuCore + tick()
│   ├── pulse.rs            # 脉冲通道（2 个）
│   ├── triangle.rs         # 三角通道
│   ├── noise.rs            # 噪声通道
│   ├── dmc.rs              # DMC 通道
│   ├── frame_counter.rs    # 4-step / 5-step
│   ├── envelope.rs         # 包络
│   ├── length_counter.rs   # 长度计数器
│   ├── sweep.rs            # 脉冲 sweep
│   ├── linear_counter.rs   # 三角 linear counter
│   └── mixer.rs            # 非线性 mixer
├── dma/
│   ├── mod.rs              # struct DmaCore
│   ├── oam_dma.rs          # $4014 启动的 OAM DMA
│   └── dmc_dma.rs          # DMC sample fetch
├── irq.rs                  # struct IrqController
├── clock.rs                # struct ClockTree
├── joypad.rs               # $4016/$4017 strobe
└── snapshot/
    └── apu.rs              # savestate 序列化（APU 状态）
```

### 2.2 APU Core 结构

```rust
pub struct ApuCore {
    pub pulse1: PulseChannel,
    pub pulse2: PulseChannel,
    pub triangle: TriangleChannel,
    pub noise: NoiseChannel,
    pub dmc: DmcChannel,
    pub frame_counter: FrameCounter,
    pub frame_irq_flag: bool,
    pub dmc_irq_flag: bool,
    pub cycle: u64,
    pub output_buffer: Vec<i16>,  // stereo interleaved
}

impl ApuCore {
    #[inline(always)]
    pub fn tick(&mut self, irq_out: &mut bool) {
        // 1. Frame counter tick
        self.frame_counter.tick(self.cycle, &mut self.frame_irq_flag);
        // 2. Quarter-frame / half-frame 触发
        if self.frame_counter.quarter_frame {
            self.pulse1.envelope.tick();
            self.pulse2.envelope.tick();
            self.triangle.linear_counter.tick();
            self.noise.envelope.tick();
        }
        if self.frame_counter.half_frame {
            self.pulse1.length_counter.tick();
            // ...
            self.pulse1.sweep.tick();
        }
        // 3. 通道 tick
        self.pulse1.tick();
        self.pulse2.tick();
        self.triangle.tick();
        self.noise.tick();
        self.dmc.tick(&mut self.dmc_dma_pending);
        // 4. Mixer
        let sample = nonlinear_mixer(
            self.pulse1.output(),
            self.pulse2.output(),
            self.triangle.output(),
            self.noise.output(),
            self.dmc.output(),
        );
        self.output_buffer.push(sample.0);
        self.output_buffer.push(sample.1);
        // 5. IRQ 输出
        *irq_out = self.frame_irq_flag || self.dmc_irq_flag;
    }
}
```

### 2.3 OAM DMA（关键时序）

```rust
pub struct DmaCore {
    oam_page: u8,
    oam_counter: u16,    // 256 → 0
    oam_stall: bool,
    oam_aligned: bool,   // 偶/奇周期对齐
}

impl DmaCore {
    #[inline(always)]
    pub fn start_oam_dma(&mut self, page: u8) {
        self.oam_page = page;
        self.oam_counter = 256;
        self.oam_stall = true;
        // 第一周期是"dummy"（CPU 写 $4014 那周期 + 1）
        self.oam_aligned = (self.cycle & 1) == 1;
    }

    #[inline(always)]
    pub fn step<BC: BusContext>(&mut self, cpu: &mut CpuCore, ppu: &mut PpuCore, bus: &mut BC) {
        if !self.oam_stall { return; }
        if self.oam_counter == 0 {
            self.oam_stall = false;
            return;
        }
        let addr = ((self.oam_page as u16) << 8) | ((256 - self.oam_counter) as u16);
        let val = bus.read(addr);
        if self.oam_aligned {
            // 奇周期：写入 OAM
            ppu.oam[(256 - self.oam_counter) as usize] = val;
        }
        // 偶周期：CPU 假装读（值丢弃）
        self.oam_counter -= 1;
    }
}
```

**严格 513/514 周期**：取决于起始时的 `cycle` 奇偶（与 C++ `FCEUI_DoOAMDMA` 完全一致）。

### 2.4 DMC DMA

```rust
impl DmaCore {
    #[inline(always)]
    pub fn step_dmc<BC: BusContext>(&mut self, dmc: &mut DmcChannel, bus: &mut BC, cpu_stalled_cycles: &mut u8) {
        if !dmc.dma_active { return; }
        // DMC DMA：每 1-4 周期 stall CPU 一次
        let stall = dmc.tick_dma_stall();
        if stall > 0 {
            *cpu_stalled_cycles = stall;
        }
        if dmc.needs_byte() {
            let addr = dmc.current_address;
            dmc.sample_buffer = bus.read(addr);
            dmc.bytes_remaining -= 1;
        }
    }
}
```

### 2.5 [修订] IRQ Controller（含外部源 EXT/EXT2，S6）

```rust
pub struct IrqController {
    nmi_edge: bool,
    nmi_pending: bool,
    irq_sources: u32,        // 多源 OR
    mapper_irq_line: bool,
    apu_irq_line: bool,
    frame_nmi_line: bool,    // PPU VBlank
    // [修订] FDS 磁盘 IRQ（fceu.cpp:139 FCEU_IQEXT / FCEU_IQEXT2）
    external_irq: [bool; 2], // EXT / EXT2
}

impl IrqController {
    pub fn assert_nmi(&mut self) {
        self.nmi_edge = true;  // edge-triggered
    }
    pub fn assert_irq(&mut self, source: IrqSource) {
        self.irq_sources |= source as u32;
    }
    /// [修订] 外部 IRQ（FDS 磁盘定时器/寻道）
    pub fn set_external_irq(&mut self, ext: usize, on: bool) {
        self.external_irq[ext] = on;
    }
    pub fn poll(&mut self) -> (bool, bool) {
        let nmi = self.nmi_edge;
        let irq = self.mapper_irq_line || self.apu_irq_line
               || self.external_irq[0] || self.external_irq[1];
        self.nmi_edge = false;  // 边缘触发后清
        (nmi, irq)
    }
}
```

**FFI**：`vnesu11_set_external_irq(soc, source, on)`（FDS 用，Phase 5 接通）。

### 2.6 [修订] Clock Tree（scanline-budget 编排，S3/ADR-008）

> **已废弃**：原 `tick()`（1 CPU step + 3 PPU dots 紧交错）与现状 budget 模型
> 不符（审计 S3）。时钟树改为 **scanline-budget 编排**，由 `Scheduler` 驱动
> （见 `02_architecture.md §4`）。

```rust
impl VNesSoc {
    // scheduler 在段边界调用（02_architecture.md §4.2）
    pub fn run_segment(&mut self, segment: Segment) -> SegmentResult {
        // 1. 检查 DMA 状态
        if self.dma.oam_stall {
            self.dma.step_oam(&mut self.cpu, &mut self.ppu, &mut self.bus_ctx());
            return self.collect_segment_result();
        }
        // 2. CPU 按预算运行（复刻 X6502_Run 语义）
        self.cpu.run_budget(segment.cpu_budget, &mut self.bus_ctx());
        // 3. PPU 渲染该段
        match segment {
            Segment::Visible { .. } => self.ppu.render_visible(&mut self.bus_ctx()),
            Segment::SpriteEval { .. } => self.ppu.render_sprite_eval(&mut self.bus_ctx()),
            Segment::PreRender { .. } => self.ppu.render_prerender(&mut self.bus_ctx()),
            Segment::VBlank { .. } => self.ppu.enter_vblank(),
            Segment::Idle { .. } => { /* NSF 空转 */ }
        }
        // 4. APU 按预算 tick
        self.apu.tick_budget(segment.cpu_budget, &mut self.apu_irq);
        // 5. Mapper IRQ（元 vtable）
        unsafe { (self.mapper_meta.tick_irq)(self.mapper, &mut self.mapper_irq); }
        // 6. 中断路由（含外部 EXT/EXT2）
        if self.ppu.nmi_pending { self.irq.assert_nmi(); }
        if self.apu_irq || self.mapper_irq || self.external_irq_any() {
            self.cpu.irq_begin(IRQ_APU | IRQ_MAPPER | IRQ_EXT);
        }
        // 7. 帧完成检测
        self.collect_segment_result()
    }
}
```

### 2.7 Joypad

```rust
pub struct JoypadState {
    pub button_state: [u8; 2],   // 当前按键状态（8 位每位一按键）
    pub shift_register: [u8; 2], // 串行移位寄存器
    pub strobe: bool,             // $4016 bit 0
}

impl JoypadState {
    pub fn read(&self, addr: u16) -> u8 {
        let pad = (addr & 1) as usize;
        let bit = (self.shift_register[pad] & 0x80) >> 7;
        if self.strobe {
            (self.button_state[pad] & 0x80) >> 7  // strobe 时持续刷新
        } else {
            self.shift_register[pad] <<= 1;        // 移位
            bit
        }
    }
    pub fn write(&mut self, addr: u16, val: u8) {
        if addr == 0x4016 {
            self.strobe = val & 1 != 0;
            if self.strobe {
                self.shift_register[0] = self.button_state[0];
                self.shift_register[1] = self.button_state[1];
            }
        }
    }
}
```

---

## 3. 验证策略

### 3.1 blargg APU 测试

- `apu_test.nes`（基础）
- `apu_mixer.nes`（mixer 验证）
- `apu_timing_test.nes`（时序）
- `apu_reset.nes`（复位）
- `dmc_dma_test.nes`（DMC DMA）
- `dmc_irq_test.nes`（DMC IRQ）
- `frame_test.nes`（帧计数器）
- `pal_apu_test.nes`

### 3.2 blargg DMA 测试

- `dma_test.nes`（OAM DMA）
- `oam_dma_test.nes`
- `cpu_dmc_test.nes`（DMC + CPU 交互）

### 3.3 Shadow run

- APU 输出 sample buffer diff（每帧）
- DMC DMA stall 周期计数
- OAM DMA stall 周期计数

### 3.4 音频回归

- 跑 SMB1 overworld 主题，截取 5 秒音频，与 v1.17 baseline 做 SNR 对比
- 目标：SNR ≥ 60dB

---

## 4. 性能基准

| 项目 | 目标 |
|------|------|
| APU 单 tick（含 mixer） | ≤ 50ns |
| OAM DMA 整周期（256 字节） | = 513/514 CPU cycles（精确） |
| 全 tick (CPU + 3 PPU + APU + IRQ) | ≤ 350ns |
| 帧时间 | ≤ 16.67ms（包含 mapper FFI 开销） |

---

## 5. 关键技术决策

### 5.1 APU 非线性 mixer 表

NES APU 的 mixer 是**非线性**的（脉冲 + 三角混合用对数/查找表近似）。标准实现：

```rust
const PULSE_TABLE: [f32; 31] = [0.0, ...];  // 95.88 / (8128 / x + 100)
const TND_TABLE: [f32; 203] = [0.0, ...];   // 159.79 / (1 / (t/8227 + n/12241 + d/22638) + 100)

fn nonlinear_mixer(p1: u8, p2: u8, t: u8, n: u8, d: u8) -> (i16, i16) {
    let pulse_out = PULSE_TABLE[(p1 as usize + p2 as usize).min(30)];
    let tnd_out = TND_TABLE[(3 * t as usize + 2 * n as usize + d as usize).min(202)];
    (pulse_out as i16, tnd_out as i16)
}
```

表来自公开的 nesdev APU 参考。

### 5.2 帧计数器半帧/四帧事件

```rust
impl FrameCounter {
    pub fn tick(&mut self, cycle: u64, irq_out: &mut bool) {
        // 4-step: 3728.5, 7456.5, 11185.5, 14914.5 cycles (14914 总周期)
        // 5-step: 3728.5, 7456.5, 11185.5, 14914.5, 18640.5 cycles (18640 总周期)
        // Quarter frame at 3728.5 / 11185.5
        // Half frame at 7456.5 / 14914.5
        ...
    }
}
```

**半帧事件**：envelope、sweep、triangle linear counter、length counter 重载
**四帧事件**：envelope / triangle linear counter 重置

### 5.3 DMA 仲裁优先级

```
1. OAM DMA 启动 → 立即 stall CPU 513/514 周期
2. DMC DMA 在 OAM DMA 期间暂停
3. DMC DMA stall CPU 1-4 周期（带空闲检测）
```

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| APU mixer 表精度差 | 🟠 中 | SNR 60dB 阈值测试 |
| OAM DMA 周期数偏差 1 | 🟠 中 | blargg oam_dma_test 严格 |
| DMC DMA 仲裁错误 | 🟠 中 | blargg dmc_dma_test |
| IRQ 多源 OR 错误 | 🟠 中 | blargg interrupt_test |
| Joypad strobe 时序 | 🟡 低 | blargg joypad_test |

---

## 7. DoD

> **2026-08-11 实测更新（路径 A 决策后）**：Phase 4 全部架构级 DoD 已完成。
> `cargo test -p vnesu11` 总计 **292 passed / 0 failed / 1 ignored**（182 unit
> + 24 CPU + 4 layout + 29 bus + 31 ppu + **20 apu integration** + 1 blargg
> + 1 nestest）。
>
> ROM-based 验证（blargg APU 25+ ROM、shadow run 100 帧、SMB1 overworld 音频
> 回归 SNR ≥ 60dB）推迟到 Phase 6/7（需完整 SoC + blargg ROM fixtures）。

- [x] APU 5 通道骨架（pulse×2 + triangle + noise + dmc）
  - `src/rust/crates/vnesu11/src/apu/{pulse,triangle,noise,dmc}.rs`
  - 包含 envelope / sweep / length_counter / linear_counter 支持
- [x] APU frame counter 4-step / 5-step
  - `src/rust/crates/vnesu11/src/apu/frame_counter.rs`
  - 4-step: 14914 cycles/frame, IRQ at 14914 cycle
  - 5-step: 18640 cycles/frame, no IRQ
- [x] 非线性 mixer（公式驱动）
  - `src/rust/crates/vnesu11/src/apu/mixer.rs`
  - PULSE_TABLE + TND_TABLE 用 `const fn` 在编译期从公式生成
  - 实音频输出后由 Phase 6 shadow-run byte-pin
- [x] OAM DMA 513/514 周期精确
  - `src/rust/crates/vnesu11/src/dma/oam_dma.rs`
  - 奇周期触发 → 513 cycles；偶周期触发 → 514 cycles
  - 严格遵循 `src/ppu.cpp::FCEUI_DoOAMDMA` quirk
- [ ] DMC DMA 仲裁正确 — 推迟到 Phase 4.5/6（需 mapper interface 接通）
- [x] IRQ 控制器（NMI edge + IRQ level + **[修订] 外部 EXT/EXT2（FDS）**）
  - `src/rust/crates/vnesu11/src/irq.rs`
  - `IrqController::assert_nmi` (edge) + `take_nmi` (once)
  - `set_external(src, on)` 用于 FDS 磁盘 IRQ
  - `aggregate_mask()` 供 CPU `irq_begin` 使用
- [ ] 时钟树（scanline-budget 编排）完整 — 推迟到 Phase 4.5（Scheduler 占位）
- [x] Joypad strobe 时序正确（+ **[修订] VS coin 输入**）
  - `src/rust/crates/vnesu11/src/joypad.rs`
  - 8-bit shift register + strobe latch
  - $4016 bit 1 = VS coin strobe
- [ ] blargg APU/DMA 测试 15+ ROM 全 PASS — [Phase 6/7] 需 ROM fixtures
- [x] **[修订] FDS 磁盘 IRQ 单测**（外部 IRQ 触发 → CPU 响应）
  - `irq::tests::irq_external_sources_aggregate` 验证 EXT/EXT2 聚合
- [ ] shadow run 与 C++ APU/DMA 100 帧零 diff — [Phase 6]
- [ ] 音频回归 SNR ≥ 60dB — [Phase 7]

### 关键文件交付

```
新增：
  [x] src/rust/crates/vnesu11/src/apu/mod.rs              # ApuCore + 5 通道 + OutputBuffer
  [x] src/rust/crates/vnesu11/src/apu/pulse.rs            # PulseChannel + duty table
  [x] src/rust/crates/vnesu11/src/apu/triangle.rs         # TriangleChannel + 32-step seq
  [x] src/rust/crates/vnesu11/src/apu/noise.rs            # NoiseChannel + LFSR
  [x] src/rust/crates/vnesu11/src/apu/dmc.rs              # DmcChannel + DMA + IRQ
  [x] src/rust/crates/vnesu11/src/apu/frame_counter.rs   # 4-step / 5-step + IRQ
  [x] src/rust/crates/vnesu11/src/apu/envelope.rs         # Volume envelope
  [x] src/rust/crates/vnesu11/src/apu/length_counter.rs   # Length counter (5 channels)
  [x] src/rust/crates/vnesu11/src/apu/sweep.rs            # Pulse sweep
  [x] src/rust/crates/vnesu11/src/apu/linear_counter.rs   # Triangle linear counter
  [x] src/rust/crates/vnesu11/src/apu/mixer.rs            # Non-linear mixer (PULSE + TND)
  [x] src/rust/crates/vnesu11/src/dma/mod.rs              # DmaCore
  [x] src/rust/crates/vnesu11/src/dma/oam_dma.rs          # OAM DMA (513/514 cycles)
  [x] src/rust/crates/vnesu11/src/irq.rs                  # IrqController (NMI + IRQ + EXT)
  [x] src/rust/crates/vnesu11/src/joypad.rs               # JoypadState + VS coin
  [x] src/rust/crates/vnesu11/tests/apu_tests.rs          # 20 个 Phase 4 集成测试

修改：
  [x] src/rust/crates/vnesu11/src/lib.rs                  # 导出 apu/dma/irq/joypad
  [x] src/rust/crates/vnesu11/src/soc.rs                  # 集成 ApuCore + DmaCore + IrqController + JoypadState
```

---

## 8. 关键文件交付

```
新增：
  src/rust/crates/vnesu11/src/apu/        # 整个目录（10 个 .rs）
  src/rust/crates/vnesu11/src/dma/        # 整个目录（3 个 .rs）
  src/rust/crates/vnesu11/src/irq.rs
  src/rust/crates/vnesu11/src/clock.rs
  src/rust/crates/vnesu11/src/joypad.rs
  src/rust/crates/vnesu11/src/snapshot/apu.rs
  src/rust/crates/vnesu11/tests/apu_tests.rs
  src/rust/crates/vnesu11/tests/dma_tests.rs
  src/rust/crates/vnesu11/tests/shadow_apu.rs

修改：
  src/rust/crates/vnesu11/src/soc.rs      # 集成所有新模块到 tick()
```

下一步：[phase_5_mapper_adapter.md](./phase_5_mapper_adapter.md)
