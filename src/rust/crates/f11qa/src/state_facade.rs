// F11QA v1.8 — kgmqa-120 state_facade
//
// Rust 端 `fceux11::State` 双身，对应 C++ `tests/f11qa/core_state_test.cpp`
// 测的 `fceu11::State` (core_state.h)。
//
// C++ 原版：
//   - `fceu11::global_state()` 返回 `fceu11::State&` 单例
//   - State 持有多个子模块引用 (CPU / PPU / Cart / Sound / Debug 等)
//   - 多次调用 `global_state()` 必须返回同一地址 (singleton identity)
//   - 访问器不能返回 null/bogus 引用
//
// Rust 端没有 C++ 全局符号；这里用 `OnceLock<StateFacade>` 实现 singleton，
// 行为契约：
//   1. global_state() 多次调用返回同一 `&'static StateFacade` 引用
//   2. 各子组件访问器返回 `&'static SubState` 引用，永不 null
//   3. 子组件内部字段可读写（mock 实现，便于 f11qa runner 测试集成）
//
// 设计取舍：
//   - 不依赖 C++ externs；用 in-memory SubState 占位
//   - 全局可变子状态通过 `Mutex<SubState>` 包装，简化后续接入真 C++ 状态时的迁移路径

use std::sync::{Mutex, OnceLock};

/// CPU 子状态（mock）。
#[derive(Default, Debug)]
pub struct CpuSubState {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
}

/// PPU 子状态（mock）。
#[derive(Default, Debug)]
pub struct PpuSubState {
    pub scanline: u16,
    pub dot: u16,
    pub frame_count: u64,
}

/// Cart 子状态（mock）。
#[derive(Default, Debug)]
pub struct CartSubState {
    pub mapper_id: u8,
    pub prg_banks: u16,
    pub chr_banks: u16,
}

/// Sound 子状态（mock）。
#[derive(Default, Debug)]
pub struct SoundSubState {
    pub sample_rate: u32,
    pub buffer_pos: usize,
}

/// Debug 子状态（mock）。
#[derive(Default, Debug)]
pub struct DebugSubState {
    pub trace_enabled: bool,
    pub breakpoint_count: u32,
}

/// 引擎全局状态 facade。
#[derive(Debug)]
pub struct StateFacade {
    pub cpu: Mutex<CpuSubState>,
    pub ppu: Mutex<PpuSubState>,
    pub cart: Mutex<CartSubState>,
    pub sound: Mutex<SoundSubState>,
    pub debug: Mutex<DebugSubState>,
}

impl Default for StateFacade {
    fn default() -> Self {
        Self {
            cpu: Mutex::new(CpuSubState::default()),
            ppu: Mutex::new(PpuSubState::default()),
            cart: Mutex::new(CartSubState::default()),
            sound: Mutex::new(SoundSubState::default()),
            debug: Mutex::new(DebugSubState::default()),
        }
    }
}

static GLOBAL_STATE: OnceLock<StateFacade> = OnceLock::new();

/// 返回全局 StateFacade 单例引用。
pub fn global_state() -> &'static StateFacade {
    GLOBAL_STATE.get_or_init(StateFacade::default)
}