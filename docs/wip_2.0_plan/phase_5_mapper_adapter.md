# Phase 5 · Mapper 适配层

> **目标**：让 vNESU11 与 ~250 个 C++ mapper 通过 **per-range handler 注册**
> （`SetReadHandler`/`SetWriteHandler` 转发）对接，**不动 mapper 代码**。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S4 重写——原方案的
> 9 函数 `MapperVtable` + `vnesu11_attach_mapper` 废弃，改为区间注册表
> （真实机制是 `bus.h:74-93` 的 per-range handler，见 `boards/mmc3.cpp:327-344`）。

## 工期：3 周

---

## 1. 范围

### 1.1 ✅ 在范围内
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

---

## 2. 任务清单

### 2.1 [修订] Rust 侧：MapperRangeTable

```rust
// crates/vnesu11/src/mapper.rs
use std::ffi::c_void;

pub const MAX_RANGES: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RangeHandler {
    pub start: u16,
    pub end: u16,
    pub fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,   // 读 handler
    pub ctx: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WriteRangeHandler {
    pub start: u16,
    pub end: u16,
    pub fn_ptr: unsafe extern "C" fn(*mut c_void, u16, u8),
    pub ctx: *mut c_void,
}

pub struct MapperRangeTable {
    read_ranges: [RangeHandler; MAX_RANGES],
    write_ranges: [WriteRangeHandler; MAX_RANGES],
    read_count: usize,
    write_count: usize,
}

impl MapperRangeTable {
    #[inline(always)]
    pub fn read(&self, addr: u16) -> Option<u8> {
        for i in 0..self.read_count {
            let h = &self.read_ranges[i];
            if addr >= h.start && addr <= h.end {
                return Some(unsafe { (h.fn_ptr)(h.ctx, addr) });
            }
        }
        None  // 未命中 → open bus
    }

    #[inline(always)]
    pub fn write(&mut self, addr: u16, val: u8) -> bool {
        for i in 0..self.write_count {
            let h = &self.write_ranges[i];
            if addr >= h.start && addr <= h.end {
                unsafe { (h.fn_ptr)(h.ctx, addr, val); }
                return true;
            }
        }
        false
    }
}
```

**为什么不优化成哈希/二分**：区间数通常 4-16 个（MMC3 才 5+），线性扫描命中
首项即返回，分支预测完美，cache 内约 10 条指令。benchmark 目标见 §4。

### 2.2 [修订] FFI 注册接口

```rust
// crates/vnesu11/src/ffi.rs
#[no_mangle]
pub extern "C" fn vnesu11_set_read_handler(
    soc: *mut VNesSocOpaque,
    start: u16, end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,
    ctx: *mut c_void,
) -> i32 {
    let soc = unsafe { &mut *soc.0.as_ptr() };
    if soc.mapper.read_count >= MAX_RANGES { return -1; }
    soc.mapper.read_ranges[soc.mapper.read_count] = RangeHandler { start, end, fn_ptr, ctx };
    soc.mapper.read_count += 1;
    0
}

#[no_mangle]
pub extern "C" fn vnesu11_set_write_handler(
    soc: *mut VNesSocOpaque,
    start: u16, end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16, u8),
    ctx: *mut c_void,
) -> i32 {
    // 同构
}

// 清空（LoadGame 时重置）
#[no_mangle]
pub extern "C" fn vnesu11_clear_mapper_handlers(soc: *mut VNesSocOpaque) {
    let soc = unsafe { &mut *soc.0.as_ptr() };
    soc.mapper.read_count = 0;
    soc.mapper.write_count = 0;
}
```

### 2.3 [修订] 元操作 vtable

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

#[no_mangle]
pub extern "C" fn vnesu11_attach_mapper_meta(
    soc: *mut VNesSocOpaque,
    mapper: *mut c_void,
    vtable: *const MapperMetaVtable,
) -> i32;
```

### 2.4 C++ 侧：SetReadHandler/SetWriteHandler 转发

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

### 2.5 与 LoadGame 集成

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

### 2.6 savestate 通过 mapper

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

### 3.1 单元测试

```
crates/vnesu11/tests/mapper_tests.rs
├── test_null_mapper          // 无 handler 注册时读 $8000 返回 open bus
├── test_single_range         // 注册一个区间，读命中/未命中
├── test_overlapping_ranges   // 重叠区间按注册顺序优先
├── test_range_capacity       // 超 MAX_RANGES 返回 -1
└── test_clear_handlers       // 清空后再读回 open bus
```

### 3.2 集成测试：每个 mapper 跑一遍 SMB1

```
tests/integration_mapper.rs
└── 对 src/boards/ 所有 mapper，加载 SMB1，验证：
    - 启动画面正确
    - 帧缓冲 60 帧后 CRC 与 v1.17 一致
    - savestate round-trip 成功
```

### 3.3 mapper_byte_diff 测试

`kagami-qa::runner::mapper_byte_diff` 已有 175-case 测试；Phase 5 通过 vNESU11
mapper 适配器走通：

- 每个 mapper 跑 N 帧
- `save_mapper_state()` 字节 diff 与 C++ 基线对比
- 必须 100% parity

### 3.4 Game Genie 单测

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

bench: `cargo bench -p vnesu11 mapper_range_scan`

---

## 5. 关键技术决策

### 5.1 不在 Rust 里实现 mapper

mapper 是"卡带"的硬件描述，~250 个不同硬件实现，**永远不可能在 Rust 里重写**。
本 phase 只做适配层。

### 5.2 [修订] 为什么用区间表而不是 vtable

**审计事实（S4）**：真实 mapper 机制是 `SetReadHandler(start, end, fn)` 按区间
注册多个 handler（MMC3 注册 5+ 个），且是**动态注册**（`Power()` 回调时）。
单个 `cpu_read` vtable 无法表达"同一 mapper 不同地址区间走不同函数"。

### 5.3 双写 aread_ 表的理由

`newppu=0` 回退路径（C++ 旧 PPU）仍用 `ARead[]` 表。双写保证：
- vNESU11 接管（newppu=1）：走 Rust 区间表
- 回退（newppu=0）：走 C++ 表
- 切换发生在 LoadGame/Reset 边界，无并发问题

### 5.4 cbindgen 生成 vnesu11/mapper.h

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
| 区间表线性扫描性能不达标 | 🟠 中 | benchmark；命中首项路径是主路径，10ns 内 |
| 某个 mapper 注册超 64 区间 | 🟡 低 | MAX_RANGES=64 远超现状（MMC3 才 5+）；超限报错 |
| Game Genie 包装顺序错 | 🟠 中 | C++ 侧保留包装，Rust 只看到包装后 handler；单测覆盖 |
| 双写 aread_ 表内存翻倍 | 🟡 低 | 1 MiB 维持现状，可接受 |
| FDS 虚拟 mapper 时序错 | 🟠 中 | fds 测试 ROM 优先验证 |

---

## 7. DoD

- [ ] `MapperRangeTable` 实现 + 区间注册/清空 FFI
- [ ] `SetReadHandler`/`SetWriteHandler` 转发（含 Game Genie 包装保留）
- [ ] `MapperMetaVtable` + `vnesu11_attach_mapper_meta`
- [ ] LoadGame 集成（`vnesu11_clear_mapper_handlers` + 系统类型 + GI_POWER 自动注册）
- [ ] mapper_byte_diff 175-case 通过 vNESU11 全 PASS
- [ ] 主流 mapper (NROM/MMC1/MMC3/VRC2/VRC6/UxROM/CNROM) 跑 SMB1 一致
- [ ] savestate round-trip 100% parity
- [ ] 帧时间开销 ≤ 5% baseline
- [ ] `cargo test -p vnesu11 mapper_tests` 全绿
- [ ] FDS 虚拟 mapper 跑通 fds 测试 ROM

---

## 8. 关键文件交付

```
新增：
  src/vnesu11_mapper_adapter.h
  src/vnesu11_mapper_adapter.cpp
  crates/vnesu11/src/mapper.rs        # [修订] MapperRangeTable
  crates/vnesu11/tests/mapper_tests.rs

修改：
  src/bus.cpp                         # [修订] SetReadHandler/SetWriteHandler 转发
  src/fceu.cpp                        # LoadGame 集成
  src/CMakeLists.txt                  # vNESU11 mapper adapter 加入构建
  src/rust/crates/vnesu11/src/lib.rs  # 添加 mapper 模块
  src/rust/crates/vnesu11/src/ffi.rs  # [修订] 区间注册接口
```

下一步：[phase_6_integration.md](./phase_6_integration.md)
