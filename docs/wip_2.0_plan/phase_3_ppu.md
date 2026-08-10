# Phase 3 · PPU 迁移

> **目标**：用 Rust 实现 **newppu=1 路径**的 2C02 PPU（scanline 段驱动 + 渲染 + 精灵 LUT），达到与 C++ `ppu_rendering.cpp`（新 PPU）**逐段、逐帧**等价。**这是工程量最大的阶段**。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S3/S5 修订——
> ① 明确只迁移 **newppu=1 路径**（旧 PPU 保留为 movie 兼容回退，ADR-009）；
> ② 时序驱动改为 **scanline 段模型**（复刻 `FCEUPPU_Loop` + `X6502_Run` 预算，
>    决策 A），废弃原 dot 紧交错 `tick()` 设计。

## 工期：6 周（含 1.5x buffer）

---

## 1. 范围

### 1.1 ✅ 在范围内（newppu=1 路径）
- **scanline 段驱动**：复刻 `FCEUPPU_Loop`（newppu 分支）的段结构
  （visible / sprite-eval / pre-render / vblank，各段带 CPU 预算）
- **背景渲染**：nametable fetch → attribute fetch → pattern low/high fetch
- **精灵渲染**：OAM 评估（次级 OAM）→ sprite fetch → 优先级合成
- **Sprite 0 hit**：精确 dot 对齐
- **Sprite overflow**：8 个精灵限制检测
- **VBlank NMI**：scanline 241 dot 0 触发
- **奇数帧跳点**：odd frame 时 scanline 261 少一个 dot
- **寄存器读缓冲**：$2002 / $2007
- **灰度 / 强调色**（$2001）
- **精灵 LUT**（512 KiB）
- **Tile fetcher 模板**（按 Flags 特化）

### 1.2 ❌ 不在范围内（[修订] S5）
- **旧 PPU（newppu=0）路径**（`ppu.cpp` 旧渲染）——**不迁移**，保留为
  movie 兼容回退（ADR-009）。`newppu=0` 时 vNESU11 不接管 PPU
- 调色板生成（`drawing.cpp`）—— Phase 6 接入 Qt 输出时处理
- PPU Viewer / NameTable Viewer（Qt 调试器）—— Phase 8
- CHR ROM 银行切换的 mapper 行为—— Phase 5

### 1.3 [修订] NSF 空转 PPU（S6）

Phase 3 一并交付 **PPU 空转 stub**（`system_type=NSF` 时 PPU 不渲染、只走
最小时序），成本低但保证 NSF 播放可用。

---

## 2. 任务清单

### 2.1 模块结构

```
crates/vnesu11/src/ppu/
├── mod.rs              # struct PpuCore + run_dots()
├── registers.rs        # $2000-$2007 + 内部 v/t/x/w
├── dot_clock.rs        # 341×262 时序、odd frame 跳点
├── background.rs       # 背景 fetch + 移位寄存器
├── sprite.rs           # OAM 评估、次级 OAM、精灵合成
├── sprite_lut.rs       # 512 KiB sprite LUT（LazyLock）
├── tile_fetch.rs       # nametable/attr/pattern fetch
├── compositing.rs      # 背景+精灵合成 + sprite 0 hit
├── nmi.rs              # VBlank NMI 触发
└── oam.rs              # OAM + 次级 OAM DMA 缓冲
```

### 2.2 [修订] scanline 段驱动（S3——废弃 dot 紧交错）

**审计事实**：FCEUX 的帧循环是 **PPU-master + CPU-budget** 模型
（`fceu.cpp:841` 入口 `FCEUPPU_Loop`；`ppu_rendering.cpp:690-703` 用
`X6502_Run(256)` / `X6502_Run(85)` / `X6502_Run(256+69)` 发放预算）。
**Rust 端必须复刻该段结构**，而不是 dot 紧交错 `tick()`。

```rust
// crates/vnesu11/src/ppu/mod.rs
pub enum Segment {
    Visible { cpu_budget: i32 },      // X6502_Run(256)
    SpriteEval { cpu_budget: i32 },   // X6502_Run(69)
    PreRender { cpu_budget: i32 },    // X6502_Run(85)
    VBlank { cpu_budget: i32 },
    Idle { cpu_budget: i32 },         // 空转 / 无 PPU（NSF）
}

pub struct PpuCore {
    // 渲染状态（background）
    pub bg_next_tile_id: u8,
    pub bg_next_tile_attr: u8,
    pub bg_next_tile_lsb: u8,
    pub bg_next_tile_msb: u8,
    pub bg_shifter_pattern_lo: u16,
    pub bg_shifter_pattern_hi: u16,
    pub bg_shifter_attr_lo: u16,
    pub bg_shifter_attr_hi: u16,

    // 渲染状态（sprite）
    pub oam: [u8; 256],
    pub secondary_oam: [u8; 32],
    pub sprite_count: u8,
    pub sprite_shifter_pattern_lo: [u8; 8],
    pub sprite_shifter_pattern_hi: [u8; 8],
    pub sprite_attributes: [u8; 8],
    pub sprite_x_counter: [u8; 8],

    // 输出
    pub frame_buffer: [u8; 61440],
    pub frame_ready: bool,
    pub nmi_pending: bool,
    pub sprite_zero_hit_pending: bool,
    pub sprite_zero_being_rendered: bool,
}

impl PpuCore {
    /// 产出下一个 scanline 段（含 CPU 预算）；scheduler 据此调用 cpu.run_budget
    pub fn next_segment(&mut self) -> Segment { ... }
    pub fn render_visible(&mut self, bus: &mut impl PpuBus) { ... }
    pub fn render_sprite_eval(&mut self, bus: &mut impl PpuBus) { ... }
    pub fn render_prerender(&mut self, bus: &mut impl PpuBus) { ... }
}
```

**关键约束**：
- 段内的**魔法常数**（256/85/69/16 等）逐行对照 `ppu_rendering.cpp` 抄录并注释来源，
  禁止重推导
- 段边界是 **CPU 预算边界**，也是 shadow run 的可比点
- overclocking / dendy / frame-skip 语义 Phase 6 复刻

### 2.3 run_dots 主循环（[修订] 段驱动，S3）

> **已废弃**：原 dot 紧交错 `tick_one_dot`（341×262 逐 dot 循环）与现状
> budget 模型不符。改为段驱动（§2.2），段的内部仍逐 dot 推进渲染状态，
> 但**段边界**是 CPU 预算边界。

```rust
impl PpuCore {
    // 段内部：逐 dot 推进渲染状态（不驱动 CPU）
    #[inline(always)]
    fn tick_one_dot(&mut self, bus: &mut impl PpuBus) {
        match (self.scanline, self.dot) {
            (-1, 0..=339) => self.tick_preline(),
            (0..=239, 0..=339) => self.tick_visible(),
            (240, 0..=339) => self.tick_postline(),
            (241, 0..=339) => self.tick_vblank_start(),
            (242..=260, _) => {},
            (261, 0..=339) => self.tick_prerender(),
            _ => {},
        }
        self.dot += 1;
        if self.dot > 340 {
            self.dot = 0;
            self.scanline += 1;
            if self.scanline > 261 {
                self.scanline = -1;
                self.frame_ready = true;
                self.odd_frame = !self.odd_frame;
            }
        }
    }
}
```

`tick_one_dot` 只推进 PPU 内部状态；CPU 由 `scheduler` 在**段边界**按预算驱动。

### 2.4 精灵 LUT（512 KiB）

```rust
// crates/vnesu11/src/ppu/sprite_lut.rs
use std::sync::LazyLock;

static SPRITE_LUT: LazyLock<Box<[u8; 524288]>> = LazyLock::new(|| {
    let mut lut = Box::new([0u8; 524288]);
    for y in 0..256 {
        for tile in 0..256 {
            for row in 0..8 {
                let base = ((y as usize) << 16) | ((tile as usize) << 3) | (row as usize);
                lut[base] = compute_sprite_pattern_byte(...);
            }
        }
    }
    lut
});
```

**为什么 `LazyLock`**：避免 512 KiB 在 BSS 段静态占用（启动时占用 ~512 KiB 进程内存），按需懒加载；运行时只一次初始化。

**LTCG 兼容**：在独立子模块（不在 `vnesu11` 顶层），避免 c2.dll 物化崩溃（参考 `ppu_sprite_lut.cpp` 的 `/GL-` 处理）。

### 2.5 Tile Fetcher（template 化）

```rust
// crates/vnesu11/src/ppu/tile_fetch.rs

pub trait TileFlags {
    const IS_SPRITE: bool;
    const FLIP_HORIZONTAL: bool;
}

pub struct Background;
pub struct SpriteNormal;
pub struct SpriteFlipH;

impl TileFlags for Background {
    const IS_SPRITE = false;
    const FLIP_HORIZONTAL = false;
}
// ...

#[inline(always)]
pub fn fetch_and_draw_tile<F: TileFlags>(
    ppu: &mut PpuCore,
    bus: &mut impl PpuBus,
    scanline: i16,
    dot: u16,
) {
    if F::IS_SPRITE {
        // sprite-specific 路径
    } else {
        // background 路径
    }
}
```

**关键**：编译器对每种 `TileFlags` 实例化一份，零开销。

---

## 3. 验证策略（[修订] 全部在 newppu=1 模式下验证）

### 3.1 blargg PPU 测试（必须全绿，newppu=1）

| ROM | 测试 |
|-----|------|
| `ppu_vbl_nmi.nes` | VBlank NMI 时序 |
| `ppu_open_bus.nes` | 总线行为 |
| `vbl_clear_time.nes` | VBlank flag clear 时序 |
| `oam_read.nes` | OAM read 时序 |
| `oam_stress.nes` | OAM stress |
| `ppu_misc.nes` | PPU misc 行为 |
| `vram_access_test.nes` | VRAM 访问时序 |
| `pal_ppu_test.nes` | 调色板 |

> **注意**：kagami-qa blargg baseline 就是 newppu=1（`tests/fixtures/blargg_full_baseline.json`），
> 与本次迁移目标一致。

### 3.2 自定义 sprite 测试（newppu=1）

- sprite 0 hit 时序（精确 dot）
- sprite overflow（8 精灵限制）
- 8×16 sprite 模式
- sprite priority（背景/精灵优先级合成）

### 3.3 Shadow run 与 C++ newppu PPU 对比

**段边界**对比（budget 模型可比点）：
- 每段结束时的 CPU tcount
- 当前 scanline/dot
- 当前 `v`/`t`/`x`/`w`
- 帧缓冲特定像素（每帧采样 256 个）

### 3.4 视觉回归（newppu=1）

- 跑 SMB1 第一帧，diff PNG 与 v1.17 baseline（newppu=1 模式）
- 跑 Zelda title screen，diff PNG
- 跑 MMC3 IRQ 测试 ROM（mapper 行为正确性）

---

## 4. 性能基准（这是最关键的阶段）

| 项目 | 目标 |
|------|------|
| 每 dot CPU cycles（visible） | ≤ 200ns |
| 每 scanline CPU cycles | ≤ 70µs |
| 每帧 CPU cycles（visible 240 行） | ≤ 17ms |
| 实际帧时间（Release build） | ≤ 16.67ms（60 FPS） |

### 4.1 性能 hot path

最热的几条路径：
1. **背景 fetch**（每 dot 一次）：nametable read → attribute read → pattern low → pattern high
2. **背景 shift + 输出**（每 visible dot 一次）：shifter 移位 + 像素合成
3. **精灵评估**（每 scanline dot 256-319）：次级 OAM 填充
4. **精灵合成**（每 visible dot 一次）：与背景合成 + sprite 0 hit

### 4.2 优化技巧

- **shifter 寄存器**用 `u16`（不是 `[u8; 2]`）—— LLVM 能放进一个寄存器
- **精灵 LUT**预先计算所有 `(y, tile, row)` 组合 → 256×256×8 = 524 288 项，每项 1 字节
- **`#[inline(always)]` 所有 tick_one_dot 子函数**——让 LLVM 看穿 dispatch
- **避免 `match` 深嵌套**——PPU dot-step 内部状态机用 if-else（branch 预测友好）
- **profile-guided**：Phase 3 末用 `cargo flamegraph` 看热点

### 4.3 性能不达标时的备选

- 退路 1：**单态化特定游戏**——为热点 ROM 生成专用代码（不通用）
- 退路 2：**SIMD tile fetch**——AVX2 一次处理 8 个 dot（侵入式，仅 Release）
- 退路 3：**dynarec 部分渲染**——极端手段，Phase 8 才考虑

---

## 5. 关键技术决策

### 5.1 [修订] 时序源 = scanline × dot（段内），段边界 = CPU 预算

每 dot = 1 PPU cycle = 1/3 CPU cycle。PPU 内部按 dot 推进，但**CPU 交互
发生在段边界**（复刻 `X6502_Run` 预算语义）。不做比这更细的 CPU/PPU 交错。

### 5.2 VBlank NMI 时序

- **Pre-render scanline (261)**：NMI flag 在 VBlank set 时（241 dot 1）置位；如 NMI 输出使能则触发
- **Visible scanline 241**：第 1 dot 触发 VBlank
- **Odd frame**：pre-render scanline 缩短 1 dot（340 → 339）

### 5.3 Sprite 0 hit 严格对齐

Sprite 0 hit 必须在 sprite 0 第一个非透明像素与背景第一个非透明像素**同一个 dot 重叠**时触发，**且**不在屏幕左 8 像素列（clipping）。代码：

```rust
fn check_sprite_zero_hit(&mut self, x: u8) {
    if self.sprite_zero_being_rendered
        && self.bg_pixel_opaque
        && self.sprite_pixel_opaque
        && x >= 8  // clipping
    {
        self.status |= SPRITE_ZERO_HIT;
    }
}
```

### 5.4 PPU 寄存器读缓冲

- `$2002` 读 = `status` 当前值，但 VBlank flag 读后清零
- `$2007` 读 = 缓冲值，然后预读下个地址（地址自动 +1 或 +32）

### 5.5 [修订] newppu 策略（S5，ADR-009——已定，选项③）

- Rust PPU **只实现 newppu=1**（QA 验证的就是它）
- **C++ 旧 PPU（newppu=0）保留**为 movie 兼容回退：`newppu=0` 时 vNESU11
  不接管 PPU（CPU/APU 仍接管）
- Phase 8 只清理 newppu=1 路径的 C++ 代码，旧 PPU 不删
- **行为差异保留**：newppu=0 与 newppu=1 的行为差异（`FCEU_TogglePPU` 切换）
  必须原样保留——旧 movie（PPUflag=0）按旧 PPU 行为播放，新 TAS 按 newppu=1
  行为。不得"统一"两路径（TAS 兼容的硬要求，见 `DECISIONS_S3_S5_analysis.md` §2）
- 理论依据：FCEUX 上游保留双 PPU 正是为了避免"一夜之间毁掉所有旧 movie"；
  TASVideos 指南标准实践 = "新 TAS 用 new PPU，旧 movie 用 old PPU"

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| PPU 性能回归（最关键） | 🔴 高 | 提前 P0 阶段做 benchmark；Phase 3 末必须 ≤ baseline（预期持平，见 A_performance_model） |
| Sprite 0 hit 时序偏差 | 🟠 中 | blargg sprite_hit_tests + 自定义 |
| VBlank NMI 延迟 | 🟠 中 | blargg vbl_clear_time + ppu_vbl_nmi |
| OAM 评估边界 bug | 🟠 中 | blargg oam_stress |
| 精灵 LUT LTCG 崩溃 | 🟠 中 | 独立子模块 + `LazyLock`（避免静态初始化） |
| 帧缓冲 diff 像素差 | 🟠 中 | 调试期每段 snapshot |
| **[修订] 段魔法常数抄录错（S3）** | 🔴 高 | 逐行对照 `ppu_rendering.cpp`；段边界 shadow diff |
| **[修订] newppu=0 movie 兼容回退失效（S5）** | 🟠 中 | C++ 旧 PPU 保留；Phase 6 录 old-PPU movie round-trip |

---

## 7. DoD

- [ ] **newppu=1** 段驱动 PPU（visible / sprite-eval / pre-render / vblank）完整
- [ ] 段魔法常数与 `ppu_rendering.cpp` 逐行对照一致
- [ ] 背景渲染完整（移位寄存器、属性）
- [ ] 精灵渲染完整（OAM 评估、合成、8×16）
- [ ] Sprite 0 hit / sprite overflow / VBlank NMI 全部正确
- [ ] blargg PPU 测试（newppu=1）25+ ROM 全 PASS
- [ ] shadow run（段边界）与 C++ newppu PPU 100 帧零 diff
- [ ] 视觉回归：SMB1/Zelda title 像素一致（newppu=1）
- [ ] NSF 空转 PPU stub 可用（S6）
- [ ] 帧时间 ≤ v1.17 × 1.05（60 FPS）
- [ ] `cargo test -p vnesu11 ppu_tests` 全绿
- [ ] `cargo bench -p vnesu11 ppu_segment` ≤ 对照 1.05x

---

## 8. 关键文件交付

```
新增：
  src/rust/crates/vnesu11/src/ppu/        # 整个目录（含 segments.rs，[修订]）
  src/rust/crates/vnesu11/src/ppu/idle.rs # [修订] NSF 空转 stub（S6）
  src/rust/crates/vnesu11/tests/ppu_tests.rs
  src/rust/crates/vnesu11/tests/shadow_ppu.rs
  src/rust/crates/vnesu11/benches/ppu_segment.rs   # [修订] 段渲染基准

修改：
  src/rust/crates/vnesu11/src/soc.rs      # 集成 PpuCore
  src/rust/crates/vnesu11/Cargo.toml      # 必要 deps（bitflags 等）
```

下一步：[phase_4_apu_dma.md](./phase_4_apu_dma.md)
