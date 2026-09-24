// F11QA v1.8 — kgmqa-120 state_facade cargo test
//
// 对偶 C++ `tests/f11qa/core_state_test.cpp`：
//   `fceu11::State` 单例 facade 的 Rust 模拟实现。
//
// 跑：
//   cargo test -p f11qa --test state_facade -- --nocapture

use f11qa::state_facade::{global_state, StateFacade};

#[test]
fn global_state_is_singleton() {
    // 多次调用返回同一引用；地址稳定。
    let s1: &'static StateFacade = global_state();
    let s2: &'static StateFacade = global_state();
    let s3: &'static StateFacade = global_state();
    assert_eq!(
        s1 as *const _, s2 as *const _,
        "global_state() returned different addresses between call 1 and 2"
    );
    assert_eq!(
        s2 as *const _, s3 as *const _,
        "global_state() returned different addresses between call 2 and 3"
    );
    println!(
        "PASS: singleton identity at {:p} across 3 calls",
        s1 as *const _
    );
}

#[test]
fn accessor_references_are_valid() {
    // 所有子模块访问器返回有效引用；不返回 null/dangling。
    let s = global_state();
    let cpu = s.cpu.lock().expect("cpu mutex poisoned");
    let ppu = s.ppu.lock().expect("ppu mutex poisoned");
    let cart = s.cart.lock().expect("cart mutex poisoned");
    let sound = s.sound.lock().expect("sound mutex poisoned");
    let debug = s.debug.lock().expect("debug mutex poisoned");
    // 取地址（不是 null）
    let _ = &*cpu;
    let _ = &*ppu;
    let _ = &*cart;
    let _ = &*sound;
    let _ = &*debug;
    println!(
        "PASS: cpu/ppu/cart/sound/debug accessors return valid references at {:p}",
        s as *const _
    );
}

#[test]
fn sub_state_field_round_trip() {
    // 子状态字段可读写，跨调用保持；验证 facade 不是 read-only view。
    {
        let s = global_state();
        let mut cpu = s.cpu.lock().unwrap();
        cpu.pc = 0xC0DE;
        cpu.a = 0x42;
    }
    {
        let s = global_state();
        let cpu = s.cpu.lock().unwrap();
        assert_eq!(cpu.pc, 0xC0DE, "pc persists across lock cycles");
        assert_eq!(cpu.a, 0x42, "a persists across lock cycles");
    }
    {
        let mut ppu = global_state().ppu.lock().unwrap();
        ppu.scanline = 12345;
        ppu.frame_count = 7;
    }
    {
        let ppu = global_state().ppu.lock().unwrap();
        assert_eq!(ppu.scanline, 12345);
        assert_eq!(ppu.frame_count, 7);
    }
    println!("PASS: sub-state field round-trip across lock cycles");
}

#[test]
fn cart_and_sound_and_debug_mutable_independently() {
    // 各子状态独立持有（Mutex），互不阻塞字段读写。
    let s = global_state();
    {
        let mut cart = s.cart.lock().unwrap();
        cart.mapper_id = 4;
        cart.prg_banks = 16;
    }
    {
        let mut sound = s.sound.lock().unwrap();
        sound.sample_rate = 48000;
    }
    {
        let mut debug = s.debug.lock().unwrap();
        debug.trace_enabled = true;
        debug.breakpoint_count = 3;
    }
    let cart = s.cart.lock().unwrap();
    let sound = s.sound.lock().unwrap();
    let debug = s.debug.lock().unwrap();
    assert_eq!(cart.mapper_id, 4);
    assert_eq!(cart.prg_banks, 16);
    assert_eq!(sound.sample_rate, 48000);
    assert_eq!(sound.buffer_pos, 0); // 默认值
    assert!(debug.trace_enabled);
    assert_eq!(debug.breakpoint_count, 3);
    println!("PASS: cart/sound/debug independently mutable, defaults preserved");
}