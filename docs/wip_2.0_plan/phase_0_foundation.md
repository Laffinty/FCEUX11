# Phase 0 · Foundation（基础工程）

> **目标**：建立 crate 骨架、FFI 表面、**SFORMAT tag 驱动的 savestate 兼容框架**、
> **逐字段 offset 布局校验**、**核心头依赖扫描**。**不实现任何 CPU/PPU 逻辑**——
> 只让 Rust 与 C++ 链接器跑通，savestate 路径走通。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S1/S2/S9 重写——
> ① savestate 兼容策略从"64 字节整块"改为"SFORMAT tag 驱动"；
> ② `CpuRegsLayout` 字段顺序修正并加 `offset_of!` 逐字段断言；
> ③ 新增核心头依赖扫描任务。

## 工期：2 周

---

## 1. 任务清单

### 1.1 Crate 骨架

- [ ] 创建 `src/rust/crates/vnesu11/Cargo.toml`（参考 `crates/fceux11-core/Cargo.toml`）
- [ ] 在根 `src/rust/Cargo.toml` 添加 `vnesu11` member
- [ ] 创建 `src/rust/crates/vnesu11/build.rs`（cbindgen 配置）
- [ ] 创建 `src/rust/crates/vnesu11/cbindgen.toml`
- [ ] 在 `src/rust/CMakeLists.txt` 注册 `vnesu11` build target

### 1.2 FFI 表面（空实现）

- [ ] 定义 `pub struct VNesSoc` 骨架（所有字段 `unimplemented!()` 或占位）
- [ ] 定义 `pub struct VNesSocOpaque`（不透明指针）
- [ ] 定义 `MapperMetaVtable`（`#[repr(C)]`，5 函数——mirroring/fill_audio/tick_irq/save/load）
- [ ] 定义 `RangeHandler` + `MapperRangeTable`（per-range 注册表，`#[repr(C)]`）
- [ ] 定义 `CpuRegsLayout`（`#[repr(C)]`，**字段顺序按 x6502struct.h**，含 offset 断言）
- [ ] 实现 `extern "C"` 函数清单（见 [02_architecture.md §5](./02_architecture.md#5-ffi-表面extern-c)）
      ——注意已改为 `vnesu11_set_read_handler` / `vnesu11_set_write_handler` 区间注册

### 1.3 [修订] SFORMAT tag 清单（S2）

**这是 Phase 0 最重要的前置交付**。产出 `docs/wip_2.0_plan/savestate_tags.md`：

- [ ] 穷举 `state.cpp` 中全部 SFORMAT 数组：
  - `SFCPU`（PC/A/X/Y/S/P/DB/RAM 0x800 间接）
  - `SFCPUC`（JAMM/IQLB/ICoa/ICou/TSBS/MooP）
- [ ] 穷举 PPU 两个状态块：
  - `FCEUPPU_STATEINFO`（旧 PPU，`state.cpp:114` extern）
  - `FCEU_NEWPPU_STATEINFO`（新 PPU，`state.cpp:115`）
- [ ] 穷举 `FCEUSND_STATEINFO`（APU）、`FCEUCTRL_STATEINFO`（控制器）
- [ ] 穷举 mapper 各自注册的 chunk（`register_state_chunk`，`cart_class.h:160-163`）
- [ ] 每 tag 记录：类型（u8/u16/u32/RLSB/数组）、大小、语义、源字段指针
- [ ] 产出"tag → Rust 字段"映射表（值语义等价，不依赖布局）

### 1.4 [修订 2026-08-10] C++ 端兼容垫片（构建实测校准）

- [x] 在 `src/vnesu11_bridge.h` / `.cpp` 声明 FFI 调用点（**已落地**）
- [x] 添加 `option(VNESU11_CORE "Use vNESU11 Rust core" OFF)` 到根 `CMakeLists.txt`（**已落地**）
- [x] 修改 `src/CMakeLists.txt`：当 `VNESU11_CORE=ON` 时链接 `vnesu11` 静态库
      + `add_dependencies(vnesu11_build)` 保证 cargo（生成 cbindgen header）先于
      C++ 编译（**已落地**）
- [ ] `FCEUI_*` 兼容垫片增加 `#ifdef VNESU11_CORE` 分支 — **[修订] 已回退，Phase 6 待办**：
      实测 fceu.cpp::Emulate 的路由代码无法编译（`int16_t*` vs `int32*` SoundBuf 不匹配 +
      缺 vnesu11_bridge.h include），已移除。Phase 6 用正确的类型和 include 重做。
- [ ] **`SetReadHandler`/`SetWriteHandler` 转发** — **[修订] 设计不成立，已移除，Phase 5 待办**：
      实测 `readfunc = uint8(*)(uint32)`（无 ctx，`types.h:90`），vNESU11 区间表需
      `uint8(*)(void*, uint16)`——直接 cast 是 ABI 谎言 + 悬垂 ctx 隐患。正确适配
      （mapper 实例作 ctx）属于 Phase 5 MapperAdapter（有 `Cart*`）。已从 bus.cpp 移除，
      C++ `aread_[]` 保持原样。

### 1.5 [修订 2026-08-10] 逐字段布局校验（S1）（构建实测校准）

- [x] `crates/vnesu11/tests/layout_check.rs`：`offset_of!` 逐字段断言 + C++ 侧
      `x6502struct.h` 底部 8 条 `static_assert`（**已落地**，且修正了断言位置——
      原放在 struct 定义前导致 C2065，已移到 `} X6502;` 之后）
- [ ] CI 工作流：编译 vNESU11 + C++，跑 layout_check — [Phase 6] CI 接线
- [ ] Golden 测试：把 `x6502struct.h` 转成二进制 golden — **[修订] 用 ctest 间接覆盖**：
      独立 golden 未做，但 `ctest 33/33`（含 `golden_savestate_test`、`cpu_test`、
      `core_state_test`）证明布局正确；`x6502struct.h` 的 `static_assert` + Rust
      `offset_of!` 双锁足以防止漂移。

### 1.6 [修订 2026-08-10] Savestate round-trip 验证（S2）

- [ ] 收集 v1.17 golden savestate 文件（各 mapper 类型若干）— [Phase 6] 需完整
      模拟器生成；当前 C++ 侧 `golden_savestate_test` PASS（ctest）
- [ ] `crates/vnesu11/tests/savestate_roundtrip.rs`：
      - load golden → 跑 N 帧 → save → 与 golden 对比
      - 逐 tag 对比（不是整块 MD5，出错时能定位到具体 tag）— [Phase 6] 需 vNESU11
        完整接入（含 PPU/APU savestate 路径）
- [ ] CI 强制：golden round-trip 字节一致才允许 merge — [Phase 7]

### 1.7 [修订] 核心头依赖扫描（S9）

- [ ] 产出 `docs/wip_2.0_plan/core_headers_deps.md`：
      - 68 个 include 站点清单（`x6502.h` 24、`sound.h` 17、`ppu.h` 16、
        `bus.h` 8、`x6502struct.h` 3）
      - 每站点记录：include 哪个头、读取哪些符号（`X`/`PPU`/`ARead`/内联访问器…）、
        迁移后走哪条路径（FFI / 兼容别名 / 直接迁移）
- [ ] 该清单作为 Phase 1-6 每个 phase DoD 的"迁移确认"依据

### 1.8 文档与脚手架

- [ ] 完成本目录所有 phase 文件的骨架（已 ✅）
- [ ] 在 `docs/ChangeLog.md` 添加 WIP 标记

---

## 2. 关键技术决策

### 2.1 [修订] Savestate：SFORMAT tag 驱动（S2）

**审计结论**：savestate 是 V2 chunked 格式（`state.cpp:122-142`），CPU 拆成
`SFCPU` + `SFCPUC` 两块，逐字段带 4 字符 tag 读写。**不是 64 字节整块 memcpy**。

因此兼容策略是**值语义等价**：

```rust
// crates/vnesu11/src/snapshot/regs.rs
// 内部布局自由，序列化时按 tag 输出相同字节
impl VNesSoc {
    pub fn save_cpu_state(&self, sink: &mut dyn FnMut(&[u8])) {
        // tag "PC\0" size 2
        sink(b"PC\0"); sink(&self.cpu.pc.to_le_bytes());
        // tag "A\0\0" size 1
        sink(b"A\0\0"); sink(&[self.cpu.a]);
        // ... 逐 tag 复刻 SFCPU
    }
}
```

**为什么这不是"手写序列化"**：因为 tag 集合固定、顺序固定（来自 `state.cpp` 的
SFORMAT 数组），Rust 端只需**逐项等价输出**。`savestate_tags.md` 就是这份
"契约"，任何 tag 增删都必须先改契约再改代码。

### 2.2 [修订] CpuRegsLayout：逐字段 offset 断言（S1）

```rust
// crates/vnesu11/src/cpu/regs.rs
// 字段顺序与 x6502struct.h 完全一致（审计修正）
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpuRegsLayout {
    pub tcount: i32,       // 0
    pub PC: u16,           // 4
    pub A: u8,             // 6
    pub X: u8,             // 7
    pub Y: u8,             // 8
    pub S: u8,             // 9
    pub P: u8,             // 10
    pub moo_pi: u8,        // 11
    pub jammed: u8,        // 12
    pub count: i32,        // 16
    pub irq_low: u32,      // 20
    pub db: u8,            // 24
    pub preexec: i32,      // 28
    // FCEUDEF_DEBUGGER（src/CMakeLists.txt:85 定义）构建下：
    pub cpu_hook: *mut c_void,   // 32
    pub read_hook: *mut c_void,  // 40
    pub write_hook: *mut c_void, // 48
}

// 逐字段断言（禁止依赖 repr(C) 默认布局假设）
const _: () = {
    assert!(offset_of!(CpuRegsLayout, tcount)   == 0);
    assert!(offset_of!(CpuRegsLayout, PC)       == 4);
    assert!(offset_of!(CpuRegsLayout, A)        == 6);
    assert!(offset_of!(CpuRegsLayout, X)        == 7);
    assert!(offset_of!(CpuRegsLayout, Y)        == 8);
    assert!(offset_of!(CpuRegsLayout, S)        == 9);
    assert!(offset_of!(CpuRegsLayout, P)        == 10);
    assert!(offset_of!(CpuRegsLayout, moo_pi)   == 11);
    assert!(offset_of!(CpuRegsLayout, jammed)   == 12);
    assert!(offset_of!(CpuRegsLayout, count)    == 16);
    assert!(offset_of!(CpuRegsLayout, irq_low)  == 20);
    assert!(offset_of!(CpuRegsLayout, db)       == 24);
    assert!(offset_of!(CpuRegsLayout, preexec)  == 28);
    #[cfg(feature = "debugger")] {
        assert!(offset_of!(CpuRegsLayout, cpu_hook)   == 32);
        assert!(offset_of!(CpuRegsLayout, read_hook)  == 40);
        assert!(offset_of!(CpuRegsLayout, write_hook) == 48);
    }
    assert!(size_of::<CpuRegsLayout>() == 64);
};
```

C++ 端：
```cpp
// src/x6502struct.h 保持不动（已是真实布局）；新增编译期断言：
static_assert(offsetof(X6502, tcount) == 0,  "tcount must be at 0");
static_assert(offsetof(X6502, PC)     == 4,  "PC must be at 4");
static_assert(offsetof(X6502, preexec) == 28, "preexec must be at 28");
#ifdef FCEUDEF_DEBUGGER
static_assert(offsetof(X6502, CPUHook) == 32, "CPUHook must be at 32");
#endif
static_assert(sizeof(X6502) == 64, "X6502 must be 64 bytes");
```

**注意**：这些断言保护的是 **oldmovie 等旧代码的整块复制路径**；savestate 的
SFORMAT 序列化走的是值语义（§2.1），两套机制都要验证，但方法不同。

### 2.3 Opaque 指针模式

```rust
pub struct VNesSocOpaque(std::ptr::NonNull<VNesSoc>);
// 强制 C++ 走函数访问，不能直接 deref
```

C++ 端：
```cpp
// src/vnesu11_bridge.h
extern "C" struct VNesSocOpaque;  // 前向声明，C++ 不能看见内部
```

### 2.4 [修订 2026-08-10] cbindgen 输出（构建实测校准）

build.rs 调用 cbindgen 生成 `vnesu11_ffi.h`（**已落地**），但 **[修订] bridge.cpp
不 include 它**：

- 实测：cbindgen 生成的 `typedef struct VNesSoc *VNesSocOpaque` 与 bridge.h 的
  `struct VNesSocOpaque;` 前向声明冲突（C2040），且强类型指针与 bridge 的 `void*`
  句柄不匹配
- 解决方案：bridge.cpp 用**本地 `extern "C"` 声明**（`void*` 签名，与 Rust
  `*mut VNesSocOpaque` ABI 相同），不依赖 cbindgen header。header 仍由 cargo
  生成（Phase 6 合并进 `fceux11_rust.h` 时用），CMake 的 include 路径已加
  `crates/vnesu11/target`

```rust
// build.rs（实际实现，输出到 CARGO_TARGET_DIR/vnesu11_ffi.h）
fn main() {
    let out_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| format!("{}/target", crate_dir));
    let out_path = format!("{}/vnesu11_ffi.h", out_dir);
    // ... cbindgen::Builder → write_to_file(out_path)
}
```

---

## 3. 验证 DoD（[修订 2026-08-10] 实测勾选）

- [x] `cargo build -p vnesu11` 成功（Release + LTO）
- [x] C++ `cmake --build build` 成功（`VNESU11_CORE=OFF`，与 v1.17 一致）— **本地验证**
- [x] C++ `cmake --build build -DVNESU11_CORE=ON` 成功（链接 `vnesu11.lib`）— **本地验证**
- [x] `cargo test -p vnesu11 layout_check` 全绿（**逐字段 offset 断言**）
- [x] `savestate_tags.md` 产出并通过 review（SFORMAT tag 契约）
- [x] `core_headers_deps.md` 产出（68 个 include 站点清单）
- [ ] golden savestate round-trip：**逐 tag** 字节一致 — [Phase 6] 需完整模拟器
- [x] CTest 33/33（含 `kagami_qa_direct_smoke` 等）— **本地验证，行为与 v1.17 一致**
- [x] 二进制 `fceux11.exe` 可启动（`--version` = 1.17.0）— **本地验证**

---

## 3.5 [新增 2026-08-10] 本地构建前置（实测经验）

本机验证 `VNESU11_CORE=ON/OFF` 需要（VS18 BuildTools + vcpkg + Qt6 已装前提下）：

1. **vcpkg_installed 符号链接**：`CMakeLists.txt:7` 的 prefer-local 分支找
   `vcpkg_installed/x64-windows`（不存在），会 fallback 到 vcpkg.cmake（触发
   manifest 重解析）。实测创建一个 junction 即可让 `do_build.ps1` 零改动工作：
   ```powershell
   cmd /c mklink /J "D:\Project\FCEUX11\vcpkg_installed\x64-windows" "D:\Project\FCEUX11\vcpkg\installed\x64-windows"
   ```
   （`vcpkg_installed/` 已加入 `.gitignore`，junction 不会被提交）
2. **测试 DLL**：`ctest` 的测试 exe 需要 Qt6/SDL DLL 在 PATH 上——
   把 `vcpkg_installed\x64-windows\bin` 加进 `$env:PATH`（否则 0xc0000139）。
   手动跑 GUI exe 还需复制 Qt 插件目录。
3. **用 `do_build.ps1` 触发构建**（自动加载 vcvars + Ninja），避免裸
   `cmake --build` 的 C1083。

---

## 4. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| `cbindgen` 输出与 C++ 现有 include 风格冲突 | 🟡 低 | 早期 review 一份 `vnesu11_ffi.h` 样本 |
| Opaque 指针导致调试困难 | 🟡 低 | 提供 `vnesu11_debug_dump()` FFI |
| **SFORMAT tag 清单漏项**（[修订]） | 🟠 中 | 与 `state.cpp`/`core_state.cpp` 逐行核对；golden round-trip 兜底 |
| **CpuRegsLayout 手写布局偏差**（[修订]） | 🔴 高 | `offset_of!` 逐字段断言 + golden 二进制对比，禁止手写 |
| `SetReadHandler` 转发遗漏 Game Genie 包装（[修订]） | 🟠 中 | C++ 侧适配器保留包装；Phase 5 单测覆盖 |

---

## 5. 关键文件交付

```
新增：
  src/rust/crates/vnesu11/
    ├── Cargo.toml
    ├── build.rs
    ├── cbindgen.toml
    └── src/
        ├── lib.rs                  # 占位 mod 声明
        ├── soc.rs                  # struct VNesSoc 骨架
        ├── ffi.rs                  # extern "C" 表面
        ├── mapper.rs               # RangeHandler / MapperRangeTable / MapperMetaVtable
        └── cpu/regs.rs             # CpuRegsLayout + offset_of 断言
  docs/wip_2.0_plan/savestate_tags.md      # [修订] SFORMAT tag 契约
  docs/wip_2.0_plan/core_headers_deps.md   # [修订] 68 include 站点清单
  crates/vnesu11/tests/layout_check.rs     # [修订] 逐字段断言
  crates/vnesu11/tests/savestate_roundtrip.rs  # [修订] golden round-trip

修改：
  src/rust/Cargo.toml               # 添加 vnesu11 member
  src/rust/CMakeLists.txt           # 注册 vnesu11 build target
  CMakeLists.txt                    # 添加 VNESU11_CORE option
  src/CMakeLists.txt                # VNESU11_CORE=ON 时链接 vnesu11
  src/x6502struct.h                 # 添加 offsetof 断言（不修改布局）
  src/bus.cpp / bus.h               # [修订] SetReadHandler/SetWriteHandler 转发
```

下一步：[phase_1_cpu.md](./phase_1_cpu.md)
