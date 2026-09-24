// F11QA v1.8 — kgmqa-119 pixbuf_pool cargo test
//
// 对偶 C++ `tests/f11qa/pixbuf_pool_test.cpp`：
//   PixBufPool（drivers/common/nes_shm.h）的 Rust 模拟实现。
//
// 跑：
//   cargo test -p f11qa --test pixbuf_pool -- --nocapture

use f11qa::pixbuf_pool::{PixBufPool, NUM_SLOTS};

#[test]
fn fresh_pool_is_empty_zero_generation() {
    let p = PixBufPool::new();
    assert_eq!(p.bytes(), 0);
    assert_eq!(p.generation(), 0);
    assert_eq!(p.dimensions(), (0, 0));
    // slot() on fresh pool is None (no allocation done).
    assert!(p.slot(0).is_none());
    assert!(p.slot(NUM_SLOTS - 1).is_none());
    println!("PASS: fresh pool: bytes()==0, generation()==0, slots None");
}

#[test]
fn resize_to_nes_resolution_bumps_generation() {
    let mut p = PixBufPool::new();
    p.resize(256, 240);
    assert_eq!(p.generation(), 1);
    let expected_bytes = 256u32 as usize * 240 * 4 * NUM_SLOTS;
    assert_eq!(p.bytes(), expected_bytes);
    assert_eq!(p.dimensions(), (256, 240));
    // Each slot non-null (i.e. Some) and distinct.
    let s0 = p.slot(0).expect("slot 0 must exist after resize");
    let s1 = p.slot(1).expect("slot 1 must exist after resize");
    assert_eq!(s0.len(), 256 * 240);
    assert_ne!(s0.as_ptr(), s1.as_ptr(), "slot 0 and slot 1 are distinct");
    println!(
        "PASS: resize(256, 240) -> bytes={} generation=1",
        expected_bytes
    );
}

#[test]
fn slot_strides_match_cap() {
    // slot(i+1).as_ptr() - slot(i).as_ptr() == ncol * nrow * sizeof(u32)
    // （slot(i+1) 的 [0] 地址 == slot(i) 的 [ncol*nrow] 地址）
    let cases: &[(u32, u32)] = &[(256, 240), (512, 480), (320, 240), (256, 224)];
    for &(ncol, nrow) in cases {
        let mut p = PixBufPool::new();
        p.resize(ncol, nrow);
        let cap = (ncol as usize) * (nrow as usize);
        let s0 = p.slot(0).unwrap();
        let s1 = p.slot(1).unwrap();
        let s0_end = unsafe { s0.as_ptr().add(cap) };
        assert_eq!(
            s1.as_ptr(),
            s0_end,
            "stride for {}x{}: slot(1) should equal slot(0)[ncol*nrow]",
            ncol,
            nrow
        );
        // 同样地 s2 是 s0 加 2*cap
        let s2 = p.slot(2).unwrap();
        let s0_end2 = unsafe { s0.as_ptr().add(2 * cap) };
        assert_eq!(s2.as_ptr(), s0_end2);
    }
    println!("PASS: slot strides match cap for {}/{} test cases", cases.len(), "size");
}

#[test]
fn clear_zeros_active_area() {
    let mut p = PixBufPool::new();
    p.resize(256, 240);
    {
        let s = p.slot_mut(0).unwrap();
        for px in s.iter_mut() {
            *px = 0xDEAD_BEEF;
        }
    }
    // 验证 fill
    assert_eq!(p.slot(0).unwrap()[0], 0xDEAD_BEEFu32);
    p.clear();
    for slot in 0..NUM_SLOTS {
        let s = p.slot(slot).unwrap();
        for (i, &px) in s.iter().enumerate() {
            assert_eq!(px, 0, "slot {} pixel {} not zeroed", slot, i);
        }
    }
    println!("PASS: clear() zeros all slot pixels");
}

#[test]
fn multiple_resizes_bump_generation_monotonically() {
    let mut p = PixBufPool::new();
    assert_eq!(p.generation(), 0);
    p.resize(256, 240);
    assert_eq!(p.generation(), 1);
    p.resize(256, 240); // same size, still bumps
    assert_eq!(p.generation(), 2);
    p.resize(512, 480);
    assert_eq!(p.generation(), 3);
    // bytes() reflects latest size
    assert_eq!(p.bytes(), 512 * 480 * 4 * NUM_SLOTS);
    println!("PASS: multiple resize() calls bump generation monotonically");
}

#[test]
fn bytes_zero_after_resize_then_clear_keeps_size() {
    // bytes() counts capacity, not zeroed state; clear() does not reset generation.
    let mut p = PixBufPool::new();
    p.resize(256, 240);
    let b = p.bytes();
    p.clear();
    assert_eq!(p.bytes(), b, "clear() preserves bytes()");
    assert_eq!(p.generation(), 1, "clear() preserves generation()");
    println!("PASS: clear() preserves bytes()/generation()");
}