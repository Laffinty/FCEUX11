// F11QA v1.8 — kgmqa-119 pixbuf_pool
//
// Rust 端 `PixBufPool` 双身，对应 C++ `tests/f11qa/pixbuf_pool_test.cpp`
// 测的 `PixBufPool` (drivers/common/nes_shm.h)。
//
// C++ 原版：
//   - 5 个 slot，每 slot ncol*nrow 个 u32 (= 4 bytes)，连续分配
//   - total bytes = ncol * nrow * 4 * 5
//   - generation atomic u32，每次 resize 自增
//
// 契约：
//   resize(ncol, nrow)    -- 分配新 buffer，generation 自增
//   slot(i)               -- 返回 slot i 的只读 slice
//   slot_mut(i)           -- 返回 slot i 的可变 slice
//   clear()               -- 清零所有 slot 的有效区域
//   bytes()               -- 总字节数 (ncol*nrow*4*5)；未 resize 时为 0
//   generation()          -- resize 次数
//
// 设计要点：5 个 slot 必须保证 `slot(i+1).as_ptr() == slot(i).as_ptr() + ncol*nrow`，
// 这是 C++ 测试 `slot_strides_match_cap` 的硬约束。Vec<Vec<u32>> 会分配 5 个独立的
// heap 块，**不连续**。这里改用单次 Vec<u32> 平铺分配，按 slot i 索引到 [i*cap, (i+1)*cap)。

/// NES 视频 buffer slot 数（与 C++ `NES_VIDEO_BUFLEN` 一致）。
pub const NUM_SLOTS: usize = 5;

#[derive(Clone, Debug)]
pub struct PixBufPool {
    ncol: u32,
    nrow: u32,
    /// 平铺 u32 buffer：[slot0][slot1]...[slot4]，每个 slot 长度 = cap。
    /// 未 resize 时为空（`Vec::new()`），bytes() / slot() 据此判断。
    storage: Vec<u32>,
    generation: u32,
}

impl Default for PixBufPool {
    fn default() -> Self {
        Self::new()
    }
}

impl PixBufPool {
    pub fn new() -> Self {
        Self {
            ncol: 0,
            nrow: 0,
            storage: Vec::new(),
            generation: 0,
        }
    }

    pub fn resize(&mut self, ncol: u32, nrow: u32) {
        let cap = (ncol as usize) * (nrow as usize);
        let total = cap * NUM_SLOTS;
        self.storage = vec![0u32; total];
        self.ncol = ncol;
        self.nrow = nrow;
        self.generation = self.generation.saturating_add(1);
    }

    /// 返回 slot i 的只读 slice；未 resize 或 i 越界时为 None。
    pub fn slot(&self, i: usize) -> Option<&[u32]> {
        if i >= NUM_SLOTS || self.storage.is_empty() {
            return None;
        }
        let cap = self.slot_cap();
        Some(&self.storage[i * cap..(i + 1) * cap])
    }

    /// 返回 slot i 的可变 slice；未 resize 或 i 越界时为 None。
    pub fn slot_mut(&mut self, i: usize) -> Option<&mut [u32]> {
        if i >= NUM_SLOTS || self.storage.is_empty() {
            return None;
        }
        let cap = self.slot_cap();
        Some(&mut self.storage[i * cap..(i + 1) * cap])
    }

    /// 清零所有 slot 的有效区域；未 resize 时 no-op。
    pub fn clear(&mut self) {
        for px in self.storage.iter_mut() {
            *px = 0;
        }
    }

    /// 总字节数 (ncol*nrow*4*5)；未 resize 时为 0。
    pub fn bytes(&self) -> usize {
        if self.generation == 0 {
            0
        } else {
            (self.ncol as usize) * (self.nrow as usize) * 4 * NUM_SLOTS
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.ncol, self.nrow)
    }

    fn slot_cap(&self) -> usize {
        (self.ncol as usize) * (self.nrow as usize)
    }
}