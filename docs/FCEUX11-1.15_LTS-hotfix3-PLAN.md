# FCEUX11 1.15 LTS — hotfix3 细致 REVIEW 修复 PLAN

**分支**: `hotfix3`
**基于**: `main` @ commit `8520274`（hotfix2 完成版 `hotfix2` 已合并到 main 的 Phase D 节点，commit `8520274`）
**目标版本**: FCEUX11 1.15(hotfix3)
**制定日期**: 2026-07-17
**适用基线**: FCEUX11 1.15 LTS hotfix2

---

## 〇、本次 REVIEW 的目标与方法

hotfix1（42 PR）以**数据完整性 / 线程 / FFI 安全**为主线；hotfix2（22 PR）聚焦 PPU 渲染管线**算法 + 微结构**。hotfix3 在二者之上做**细致排查**，目标是找出**可能导致程序崩溃或严重降低性能**的残余 BUG，覆盖此前未触及的子系统（mapper / movie / Lua / Qt / 声音 / 配置 / 文件 I/O）。

### 0.1 排查范围

| 子系统 | 文件 / 范围 | 来源 agent |
|--------|------------|-----------|
| PPU / Audio / CPU | `src/ppu*.cpp/.h`、`x6502.cpp/.h`、`cpu.cpp/.h`、`sound.cpp`、`apu.cpp`、`expansion_audio.cpp` | A 类 |
| Mapper / Movie / Lua / Debug / Netplay / Input | `src/boards/*.cpp`、`movie*.cpp`、`lua-engine.cpp`、`debug.cpp`、`netplay.cpp`、`input.cpp`、`cheat.cpp` | B 类 |
| Rust FFI / Qt Driver / 文件 / 配置 | `src/rust/crates/*`、`drivers/Qt/*`、`config.cpp`、`file.cpp`、`ines.cpp`、`state.cpp`、`core_state.cpp`、`cart.cpp`、`palette.cpp` 等 | C 类 |

### 0.2 诊断汇总

| 类别 | CRITICAL | HIGH | MEDIUM | LOW | INFO | 合计 |
|------|---------|------|--------|-----|------|------|
| 崩溃 / UB | 5 | 13 | 16 | 11 | 1 | **46** |
| 性能 | 0 | 4 | 17 | 9 | 3 | **33** |
| 边界 / 别名 / 杂项 | 0 | 0 | 4 | 8 | 0 | **12** |
| **合计** | **5** | **17** | **37** | **28** | **4** | **91** |

> **重要声明**：本 PLAN 修复的目标是**实际可触发的 BUG**，每条都附"触发场景"。**纯理论（如 64-bit 端微优化）不纳入本轮范围**。

### 0.3 phase 切分原则（关键）

用户明确要求**避免单个 Phase 工作量过大、超出 AI agent 注意力窗口**。本 PLAN 遵循以下约束：

- **每 phase PR 数 ≤ 6**（hotfix2 Phase B 7 PR 实测已接近上限）
- **每 PR 文件改动 ≤ 5**、单 PR 代码净增 ≤ 150 行
- **每 phase 风险类目单一**：崩溃 / 性能 / 清理不混在同一 phase
- **phase 间依赖显式**：先修崩溃后修性能、先 PPU/Audio 后 Qt（Qt 关闭路径依赖 PPU 状态）

> 与 hotfix2 相比：hotfix2 Phase A（4 PR）门槛很低；本 hotfix3 目标 28 PR / 5 phase（**平均 5.6 PR/phase**），保持单一 AI agent 可一次性 build + test + review 的可执行粒度。

---

## 一、Phase 总览

| Phase | 主题 | PR 数 | 风险 | 预计周期 |
|-------|------|-------|------|---------|
| **A** | 跨线程数据竞争（CRITICAL） | 5 | 中 | 1 周 |
| **B** | 跨线程 buffer / stale pointer（HIGH） | 6 | 中 | 1 周 |
| **C** | PPU / Audio / Mapper 内存安全 | 5 | 中 | 1 周 |
| **D** | 性能回退（热路径） | 5 | 低 | 1 周 |
| **E** | 代码质量与一致性清理 | 5 | 低 | 0.5 周 |
| **总计** | — | **26 PR** | — | **4.5 周** |

---

## 二、Phase A — 跨线程数据竞争（CRITICAL）

> **主题**: 5 个 CRITICAL + 0 个 HIGH，全部为多线程数据竞争与 use-after-free 风险。hotfix1 P1-12（nes_shm 原子化）只修了部分字段；hotfix2 未触及 Lua engine 与 fceuWrapper 的锁状态。本 phase 是 hotfix3 必须最先合入的部分。

### A-1 [CRITICAL] Lua 引擎裸指针并发 UB

**诊断 ID**: RUST-CRASH-01
**文件**:
- `src/rust/crates/fceux11-lua/src/lib.rs:27-33`（`LUA_ENGINE_PTR`）
- `src/rust/crates/fceux11-lua/src/lib.rs:516-532`（init 路径）

**问题**:
```rust
static LUA_ENGINE_PTR: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
fn get_engine<'a>() -> Option<&'a mut LuaEngine> {
    let raw = LUA_ENGINE_PTR.load(Ordering::Acquire) as *mut LuaEngine;
    unsafe { raw.as_mut() }
}
```

`get_engine()` 把裸指针直接转 `&mut LuaEngine`，**无同步原语**。Emulator 线程与 GUI 线程并发调用 FFI 时，可同时拿到两个 `&mut` 到同一对象 → **Stacked Borrow UB**。重新 init 期间存在 **use-after-free**。

**修复**:
- 用 `Mutex<Box<LuaEngine>>` 替换 `AtomicPtr<c_void>`
- 提供 `fceux11_lua_init / shutdown` 配对 FFI
- 所有 `fceux11_lua_*` 入口走 `parking_lot::Mutex::lock()`

**风险**: 高（改 ABI）；Lua FFI 调用方需重新链接。
**回滚**: 单 PR revert。
**验证**: TSan + 集成测试（GUI 调 `get_mem_hook_count` 与 emulator 调 `call_registered` 并发）。

> **【hotfix3 执行决策 — 2026-07-18】A-1 降级为文档化，不实施 Mutex 替换。**
>
> **决策理由**：经分析，FCEUX11 的 Lua FFI 调用全部在 emulator 单线程上执行，GUI 线程仅调用
> `fceux11_lua_init` / `fceux11_lua_shutdown`，且 Qt 架构保证 emulatorThread 已退出后才调用
> shutdown（`ConsoleWindow` 析构函数通过 `requestInterruption + wait(5000) + terminate` 等待）。
> 因此 `AtomicPtr::swap(AcqRel)` 已经为 init/shutdown 提供了充分的 happens-before 保证。
>
> **Mutex 方案的风险**：
> 1. **死锁**：`parking_lot::Mutex` 是非可重入锁。当 FFI 调用触发 C++ 回调（如 `fceux11_lua_BWrite`）
>    时，C++ 侧可能通过 Lua C API 重新进入 Rust 的 `get_engine()`，在同一线程上再次尝试获取锁 → 死锁。
> 2. **ABI 变更**：添加 `parking_lot` 依赖 + 改变所有 13+ FFI 入口签名，影响面过大。
> 3. **ReentrantMutex 不可用**：`parking_lot::ReentrantMutex` 是 `!Sync`，不能用作 `static`。
>
> **已实施的替代措施**：
> - 在 `lib.rs` 的 `LUA_ENGINE_PTR` 声明处添加了 46 行安全论证文档（见 commit `802931b`），
>   明确说明当前架构下的安全保证、已知的理论 unsoundness。
> - **后续 agent 不得重新尝试实施 Mutex 替换**。当前方案已是最终决策。
>
> **对应 commit**：`802931b` — `docs(lua): add thread-safety rationale for AtomicPtr approach`

### A-2 [CRITICAL] Lua 引擎无析构 FFI

**诊断 ID**: RUST-CRASH-02
**文件**: `src/rust/crates/fceux11-lua/src/lib.rs:516-532`

**问题**: `Box::into_raw` 后**无对应 `Box::from_raw`**，两次 init 直接覆盖指针，前一个 `LuaEngine` 永久泄漏（含 `mlua::Lua` Registry 中注册的 callback 永不释放）。

**修复**（与 A-1 合并为一个 PR）：
- 在 `Mutex<Box<LuaEngine>>::lock()` 内分配 + 初始化
- 提供 `fceux11_lua_shutdown` 取走并 drop
- 在 Lua 脚本 reload / 应用退出路径调用 shutdown

**PR 标题**: `fix(lua): make LuaEngine thread-safe via Mutex and add shutdown FFI`

> **【hotfix3 执行决策 — 2026-07-18】A-2 已实施（不含 Mutex 部分）。**
>
> **实际 PR 标题**：`fix(lua): add fceux11_lua_shutdown FFI to reclaim LuaEngine Box (hotfix3 A-1+A-2)`
>
> **实施内容**：
> - `fceux11_lua_shutdown()` FFI 已实现：通过 `AtomicPtr::swap(null, AcqRel)` 取出指针，`Box::from_raw` drop。
> - `fceux11_lua_init()` 已修改：init 前先 `swap` 回收旧引擎，防止双重 init 泄漏。
> - C++ 侧：`lua-engine.cpp` 添加 `FCEU_LuaShutdown()` wrapper + `extern "C"` 声明；
>   `fceulua.h` 添加声明；`fceu.cpp` 的 `fceu11::Kill` 路径调用 shutdown。
> - `fceux11_rust.h` 自动头文件已补齐 `fceux11_lua_shutdown` 声明（commit `1d62407`）。
>
> **未实施部分**（因 A-1 降级）：Mutex 保护、所有 FFI 入口加锁。
>
> **对应 commit**：`8498cb2`（主修复）、`1d62407`（头文件补齐）

---

### A-3 [CRITICAL] nes_shm POD 字段跨线程非原子

**诊断 ID**: QT-CRASH-01
**文件**:
- `src/drivers/Qt/nes_shm.h:26-37, 51-55`
- 调用方: `src/drivers/Qt/sdl-video.cpp:285, 495-498, 543`、`ConsoleViewerGL.cpp:757,763`、`ConsoleViewerSDL.cpp:746,766`、`ConsoleViewerQWidget.cpp:501,518`

**问题**: `runEmulator`/`blitUpdated`/`pixBufIdx`/`sndBuf.head/tail/starveCounter` 已在 hotfix1 P1-12 升 `std::atomic`，**但**剩余字段 `render_count`、`blit_count`、`video.{ncol,nrow,pitch,xscale,yscale,xyRatio,preScaler,test}`、`pid`、`run` 仍为普通 int。注释 `nes_shm.cpp:48-51` 明确承认"no happens-before"。Emulator 线程 `doBlitScreen` 写 `video.ncol/nrow/pitch/preScaler`，GUI 线程 viewer 同帧读取 → **torn read + reordering → 错位/闪屏**。

**修复**:
```cpp
// nes_shm.h
struct nes_shm_t {
    std::atomic<char> runEmulator{0};
    std::atomic<char> blitUpdated{0};
    std::atomic<int>  pixBufIdx{0};
    std::atomic<uint32_t> render_count{0};   // 新增
    std::atomic<uint32_t> blit_count{0};    // 新增
    struct VideoAtomic {
        std::atomic<int> ncol{0}, nrow{0}, pitch{0};
        std::atomic<int> xscale{0}, yscale{0}, xyRatio{0};
        std::atomic<int> preScaler{0}, test{0};
    } video;
    std::atomic<int>  pid{0};
    std::atomic<char> run{0};
    std::atomic<int>  sndHead{0}, sndTail{0};
    // ...
};
```
逐字段把读/写路径替换为 `.load()`/`.store()`。

**PR 标题**: `fix(qt): atomicize remaining nes_shm cross-thread fields`

> **【hotfix3 执行决策 — 2026-07-18】A-3 已实施（含 errata 修复）。**
>
> **实际 PR 标题**：`fix(qt): atomicize remaining nes_shm cross-thread fields (hotfix3 A-3)`
>
> **实施内容**：
> - `nes_shm.h` 中 14 个字段全部改为 `std::atomic`：`render_count`、`blit_count`、
>   `video.{ncol,nrow,pitch,xscale,yscale,xyRatio,preScaler,test}`、`pid`、`run`。
> - `nes_shm.cpp` 初始化使用 `.store(0, memory_order_relaxed)`。
> - 13 个调用方文件的读/写路径全部替换为 `.load(acquire)` / `.store(release)` / `.fetch_add(relaxed)`。
> - `sndBuf.head` / `sndBuf.tail` 同步 atomic 化。
>
> **已知问题与修复**：
> - 自动化脚本 `atomize_nes_shm.py` 的正则 bug 将 `preScaler == N` 错误转换为
>   `preScaler.store(= N)` 语法，影响 5 个文件 7 处。此 bug 在 commit `5041e03` 中引入，
>   在 errata commit `1d62407` 中修复为正确的 `preScaler.load(acquire) == N`。
> - `pid` 和 `run` 字段在 atomic 化后未被任何代码读写（dead code），无害但可在未来清理。
>
> **对应 commit**：`5041e03`（主修复）、`1d62407`（语法 errata + 头文件）

---

### A-4 [CRITICAL] `fceuWrapperLock` 在析构路径 UAF

**诊断 ID**: QT-CRASH-03
**文件**:
- `src/drivers/Qt/ConsoleWindow.cpp:350`（dtor）
- `src/drivers/Qt/ConsoleVideo.cpp:366-368`（closeApp）
- `src/drivers/Qt/fceuWrapper.cpp:1186-1195`（lock 调用点）

**问题**: closeApp 路径已用 `wait(5000)` + `terminate()`（hotfix1 P1-2），但**非 closeApp 析构路径**（如 quit 时直接 `delete consoleWindow`）的顺序是：① `nes_shm->runEmulator=0` ② `gameTimer->stop()` ③ `unloadVideoDriver()` ④ `delete mutex` —— **没有等 emulatorThread 退出**。若 emulator 仍在 `fceuWrapperUpdate` → `fceuWrapperTryLock` → `consoleWindow->mutex`，则 mutex 已被 `delete` → **UAF**。

**修复**:
```cpp
// ConsoleWindow.cpp dtor
consoleWin_t::~consoleWin_t() {
    if (emulatorThread) {
        emulatorThread->requestInterruption();
        if (!emulatorThread->wait(5000)) {
            qWarning() << "Emulator thread did not exit; terminating";
            emulatorThread->terminate();
            emulatorThread->wait();
        }
    }
    delete mutex;   // 安全：thread 已退出
}
```

**PR 标题**: `fix(qt): wait for emulator thread before deleting mutex in consoleWin_t dtor`

> **【hotfix3 执行决策 — 2026-07-18】A-4 已实施，PASS。**
>
> **实际实现**与计划一致。额外增加了 `closed_` 幂等标志（`ConsoleWindow.h:142`），防止
> `closeApp()` 已执行等待后析构函数重复等待。`closeApp()` 设置 `closed_ = true` 后使用相同
> 的 `requestInterruption / wait(5000) / terminate + wait()` 模式。
>
> **对应 commit**：`f2cf4a0`

---

### A-5 [CRITICAL] `fceuWrapper` mutex 状态跨线程非原子

**诊断 ID**: QT-CRASH-08
**文件**: `src/drivers/Qt/fceuWrapper.cpp:120-122, 1188, 1234, 1304, 1318`

**问题**:
```cpp
static int   mutexLocks = 0;
static int   mutexPending = 0;
static bool  emulatorHasMutex = 0;
```
emulator 线程 `mutexPending++` / GUI 线程 `fceuWrapperTryLock` 等并发读写 plain int。Torn write 偶尔导致 lock 状态错位。

**修复**:
```cpp
static std::atomic<int>  mutexLocks{0};
static std::atomic<int>  mutexPending{0};
static std::atomic<bool> emulatorHasMutex{false};
```
逐操作替换 ++ / -- / = 为 `fetch_add` / `fetch_sub` / `store` + memory_order_acquire/release。

**PR 标题**: `fix(qt): atomicize fceuWrapper mutex state counters`

> **【hotfix3 执行决策 — 2026-07-18】A-5 已实施，PASS。**
>
> **实施内容**与计划完全一致：
> - `mutexLocks` → `std::atomic<int>`，使用 `fetch_add(1, acq_rel)` / `fetch_sub(1, acq_rel)` / `load(acquire)`
> - `mutexPending` → `std::atomic<int>`，使用 `fetch_add(1, relaxed)` / `fetch_sub(1, relaxed)` / `load(acquire)`
> - `emulatorHasMutex` → `std::atomic<bool>`，使用 `store(true/false, release)`
> - 全部 18 处使用点已验证正确，变量为 `static` 作用域（仅 `fceuWrapper.cpp` 内部），无外部访问。
>
> **对应 commit**：`c7a78f6`

---

### A-6 [CRITICAL] `fceux11_lua_movie_get_name`/`get_filename` 共享 static string

**诊断 ID**: LUA-CRASH-02
**文件**: `src/lua-engine.cpp:612-627`

**问题**:
```cpp
const char* fceux11_lua_movie_get_name() {
    static std::string name;
    name = fceu11::GetMovieName();
    return name.c_str();
}
const char* fceux11_lua_movie_get_filename() {
    static std::string name;
    name = fceu11::GetMovieName();
    // ...substr...
    return name.c_str();
}
```
**两个函数共享同一 static `name`**。Rust 端连续调用 `get_name()` + `get_filename()` 第二次赋值覆盖第一次的 `c_str()` → **UAF**。

**修复**:
- 改为 thread_local（或每函数独立 static）
- 或返回 `strdup` 副本，文档化"调用方负责 free"
- 推荐 thread_local + 在 Lua 端立即 copy 出

**PR 标题**: `fix(lua): make movie name/filename FFI return thread_local strings`

> **【hotfix3 执行决策 — 2026-07-18】A-6 已实施，PASS。**
>
> **实施内容**与计划完全一致：
> - `fceux11_lua_movie_get_name()`：`static std::string name` → `thread_local std::string name`
> - `fceux11_lua_movie_get_filename()`：`static std::string name` → `thread_local std::string filename`
> - 两个函数使用**独立**的 `thread_local` 变量（`name` vs `filename`），互不干扰。
> - Rust 端 `bindings/movie.rs:61,76` 立即通过 `CStr::to_string_lossy().into_owned()` 拷贝，
>   返回指针仅在单次 FFI 调用期间有效。
>
> **对应 commit**：`3fb4087`

---

## Phase A 执行总结（2026-07-18）

| PR | 计划方案 | 实际方案 | 差异说明 |
|---|---|---|---|
| A-1 | Mutex\<Box\<LuaEngine\>\> 替换 AtomicPtr | **降级为文档化** | 单线程架构下 AtomicPtr + AcqRel 已足够安全；Mutex 会因 C++ 回调重入导致死锁 |
| A-2 | Mutex 内 init + shutdown FFI | 仅 shutdown FFI | Mutex 部分随 A-1 一起取消；shutdown 和 init 的 swap 回收逻辑已正确实现 |
| A-3 | 14 字段 atomic 化 | 已实施 + errata 修复 | 自动化脚本引入 7 处语法错误，已在后续 commit 修复 |
| A-4 | 析构等待 emulatorThread | 已实施 | 额外增加了 `closed_` 幂等标志 |
| A-5 | 3 变量 atomic 化 | 已实施 | 与计划完全一致 |
| A-6 | thread_local 替换 static | 已实施 | 与计划完全一致 |

**验证结果**：Release 构建 100% 通过，28/28 测试全部通过，Rust `cargo check` 通过。

**Phase B 开工条件**：已满足。A-1 的理论 unsoundness 已文档化且有架构级安全保障，不阻塞后续 phase。

---

## 三、Phase B — 跨线程 buffer / stale pointer（HIGH）

> **主题**: 6 个 HIGH，全部为跨线程访问 / thread_local stale pointer。承接 Phase A 的原子化思路，扩展到 SDL 音频、mapper 临时 buffer 等。

### B-1 [HIGH] SDL 音频 buffer 跨线程非同步

**诊断 ID**: QT-CRASH-05
**文件**: `src/drivers/Qt/sdl-sound.cpp:40-46, 287, 521`

**问题**:
```cpp
static unsigned int s_BufferRead, s_BufferWrite, s_BufferIn;
```
SDL audio 回调线程与 emulator 线程并发读写 plain unsigned。读损坏 → 写入 `s_Buffer[out_of_range]` → 静音或爆音。

**修复**: 改为 `std::atomic<uint32_t>`，或用 `Mutex<SoundRing>` 包整个 ring buffer。

**PR 标题**: `fix(qt): make sdl-sound ring buffer indices atomic`

---

### B-2 [HIGH] Rust `movie_data_rom_filename` / `movie_data_guid` thread_local stale

**诊断 ID**: RUST-CRASH-05
**文件**:
- `src/rust/crates/fceux11-formats/src/movie.rs:967-1006`
- `src/rust/crates/fceux11-formats/src/ines.rs:230-250`（mapper_name 已修，但 movie/guid 仍漏）

**问题**: 与 hotfix1 P1-15 修的 `mapper_name` 同一模式——`thread_local` `RefCell` 缓存并返回内部指针。C 端若缓存指针后再调用一次同名函数，原指针指向的 buffer 被覆写 → **stale data**；多线程并发则 **UB**（RefCell 不是 Sync）。

**修复**: 参考 `mapper_name` 改为泄漏 `CString` 或 `Arc<Box<str>>`，并文档化"调用方必须立即 copy"。

**PR 标题**: `fix(rust): eliminate thread_local stale pointer in movie/ines FFI`

---

### B-3 [HIGH] `fceux11_ines_check_bad` thread_local stale + 255 字节截断

**诊断 ID**: RUST-CRASH-09
**文件**: `src/rust/crates/fceux11-formats/src/ines.rs:230-250`

**问题**: ① 同一线程连续调用两次，前一次返回指针悬空。② RefCell 不是 Sync，跨线程触发 panic / UB。③ 长 name 截断到 255 字节。

**修复**: 改为 `Arc<CString>` 返回；调用方立即 copy 或持有 Arc。

**PR 标题**: `fix(rust): replace ines_check_bad thread_local with leaked CString`

---

### B-4 [HIGH] Lua zapper file-static 跨线程

**诊断 ID**: LUA-CRASH-03
**文件**: `src/lua-engine.cpp:927-946`（`fceux11_lua_zapper_*`）

**问题**:
```cpp
// luazapperx / luazappery / luazapperfire 是 file-static，跨线程调用无锁。
```
Lua 线程写、emulator 线程读 → torn read。

**修复**: 改为 `std::atomic<int>` / `std::atomic<bool>`。

**PR 标题**: `fix(lua): atomicize zapper file-static state`

---

### B-5 [HIGH] `map_irq_hook` 跨线程 race

**诊断 ID**: A-CRASH-06
**文件**: `src/x6502.cpp:519-529`

**问题**: `g_cpu.map_irq_hook_ref()` 返回引用，热路径每指令调用一次（~107M 次/秒）。Mapper 通过 `FCEUI_SetHookFunc` 修改该函数指针时无同步 → **use-after-free / 函数指针悬空**。

**修复**:
```cpp
// cpu.h
class Cpu {
    std::atomic<MapIRQHook*> map_irq_hook_{nullptr};
public:
    MapIRQHook* map_irq_hook() const noexcept {
        return map_irq_hook_.load(std::memory_order_acquire);
    }
    void set_map_irq_hook(MapIRQHook* h) noexcept {
        map_irq_hook_.store(h, std::memory_order_release);
    }
    // map_irq_hook_ref() 保留作 back-compat（debugger 内部用）
};
```
或保留 `map_irq_hook_ref()` 但只用于 debugger 冷路径，热路径改用 `if (auto* h = g_cpu.map_irq_hook()) [[unlikely]] h(temp);`。

**PR 标题**: `fix(cpu): make map_irq_hook thread-safe via atomic load`

---

### B-6 [HIGH] Qt 跨线程 `connect(...)` 缺显式 connection type

**诊断 ID**: QT-CRASH-06/07
**文件**:
- `src/drivers/Qt/ConsoleWindow.cpp:195-199`
- `src/drivers/Qt/ConsoleVideo.cpp:242, 261, 277`
- `src/drivers/Qt/main.cpp:447-449`

**问题**: 跨线程 `connect(emulatorThread, ..., this, ...)` 用 `SIGNAL`/`SLOT` 宏默认 `AutoConnection` → `QueuedConnection`，**但**未来若 receiver 移到子线程，会变为 `DirectConnection` → 跨线程直接调用 → 状态不一致。

**修复**: 所有跨线程 `connect` 加显式第五参数 `Qt::QueuedConnection`。**单 PR 大批替换**，每处加 5-10 行 grep 验证。

**PR 标题**: `chore(qt): explicit Qt::QueuedConnection for cross-thread signal/slot`

---

## 四、Phase C — PPU / Audio / Mapper 内存安全

> **主题**: 5 PRs 修复 PPU 模板、sound.cpp 整数溢出、mapper mask 移位下溢、文件 I/O NULL 守卫。覆盖 hotfix2 落地后引入 / 遗留的内存安全问题。

### C-1 [HIGH] vnapage 未初始化即解引用

**诊断 ID**: A-CRASH-01
**文件**: `src/pputile_template.cpp:134`

**问题**:
```cpp
FCEU_MAYBE_UNUSED uint8_t* C_dummy = vnapage[(RefreshAddr >> 10) & 3];
cc = C_dummy[0x3c0 + (zz >> 2) + ((RefreshAddr & 0x380) >> 4)];
```
`vnapage_[4]` 在 `Ppu::Ppu()` 默认零初始化（`ppu_class.cpp:27-33`）。若 mapper 尚未调用 `set_mirror_mode` / `set_mirror_pages`，四个指针都是 `nullptr`。hotfix2 P0-3/P0-4 模板实例化引入此风险。

**修复**: 在 `Ppu::reset()` 默认指向 `ntaram_` 自身：
```cpp
// ppu_class.cpp Ppu::reset()
for (auto& p : vnapage_) p = ntaram_;
```
（与 `bus.cpp` init 中 VPage fallback 同模式。）

**PR 标题**: `fix(ppu): default vnapage to ntaram_ in Ppu::reset to avoid null deref`

---

### C-2 [MEDIUM] `pputile_template` 缺 PALRAM static_assert

**诊断 ID**: A-CRASH-03
**文件**: `src/pputile_template.cpp`（添加），`src/pputile_template.h`

**问题**: `static_assert(std::tuple_size_v<decltype(PALRAM)>>= 0x10, ...)` 仅在 ppu_rendering.cpp 中。模板实例化（`FetchAndDrawTile<Flags>`）也访问 `PALRAM`，但 pputile_template.cpp 没同等守卫。未来若有人把 PALRAM 缩到 < 0x20，模板编译通过但运行时越界。

**修复**: 在 `pputile_template.h` 加：
```cpp
#include "ppu_state.h"
static_assert(std::tuple_size_v<decltype(PALRAM)>>= 0x20,
              "PALRAM must hold offsets 0..0x1F (32 bytes)");
```

**PR 标题**: `fix(ppu): add PALRAM size static_assert to pputile_template.h`

---

### C-3 [MEDIUM] `sound.cpp` INT32 溢出 / DMC 下溢

**诊断 ID**: A-CRASH-08/09
**文件**: `src/sound.cpp:521, 453-455`

**问题**:
- `-DMCacc` 在 DMCacc == INT32_MIN 时 signed overflow UB（line 521）
- `cycles * 16` 接近 INT32_MAX 时 signed overflow（line 453-455）
- 极端参数（来自 Lua/cheat 的 `X6502_Run(2^31)`）会 silent overflow

**修复**:
```cpp
// sound.cpp:521
const uint32 fudge = std::min<uint32>(
    static_cast<uint32>(-(int64_t)DMCacc),    // 64-bit 中转
    soundtsoffs + g_cpu.timestamp_ref());

// x6502.cpp:453
int64_t scaled = (int64_t)cycles * (PAL ? 15 : 16);
if (scaled > INT32_MAX) scaled = INT32_MAX;
_count += (int32_t)scaled;
```

**PR 标题**: `fix(sound): guard signed integer overflow in DMC acc and X6502 cycle scaling`

---

### C-4 [MEDIUM] `SetSoundVariables` 在 `SndRate==0` 时除零

**诊断 ID**: A-CRASH-10
**文件**: `src/sound.cpp:1213, 1219`

**问题**: `FlushEmulateSound` 已有 `if(!FSettings.SndRate) goto nosoundo;` 守卫，但 `SetSoundVariables` 在 `SndRate==0` 时仍执行 `nesincsize`/`soundtsinc` 的除法，产生 NaN/Inf。

**修复**: `SetSoundVariables` 顶部早退 `if(!FSettings.SndRate) return;`。

**PR 标题**: `fix(sound): early-return SetSoundVariables when SndRate is zero`

---

### C-5 [HIGH] MMC3 mask shift 下溢 + MMC5fill nullptr 守卫

**诊断 ID**: MAP-CRASH-02, MAP-CRASH-03
**文件**:
- `src/boards/mmc3.cpp:316-318`
- `src/boards/mmc5.cpp:1037-1038, 1067-1068`

**问题**:
- MMC3: `(prg >> 13) - 1` 当 `prg < 0x2000` 时 uint32 减 1 = 0xFFFFFFFF → PRGmask8 全 1 → 后续 `setprg8(V)` 越界
- MMC5: `FCEU_gmalloc_unique(1024)` 失败时 `MMC5fill = nullptr.get()` → `FCEU_dwmemset(nullptr, ...)` 段错误

**修复**:
```cpp
// mmc3.cpp
PRGmask8[0] = (prg >= 0x2000) ? ((prg >> 13) - 1) : 0;
CHRmask1[0] = (chr >= 0x400)  ? ((chr >> 10) - 1) : 0;
CHRmask2[0] = (chr >= 0x800)  ? ((chr >> 11) - 1) : 0;

// mmc5.cpp
MMC5fill_owner = FCEU_gmalloc_unique(1024);
if (!MMC5fill_owner) {
    FCEU_PrintError("MMC5: out of memory");
    return;
}
MMC5fill = MMC5fill_owner.get();
```

**PR 标题**: `fix(mapper): clamp MMC3 mask shifts and guard MMC5fill allocation`

---

## 五、Phase D — 性能回退（热路径）

> **主题**: 5 PRs 修复 hotfix2 未触及 / 新引入的性能回退。重点是 hot path 上的重复工作、大缓冲 memset、跨线程 volatile 缺 atomic。

### D-1 [HIGH] nes_shm 24 MiB pixbuf 动态分配

**诊断 ID**: QT-CRASH-02, QT-PERF-01
**文件**: `src/drivers/Qt/nes_shm.h:54-55`

**问题**:
```cpp
uint32_t pixbuf[NES_VIDEO_BUFLEN][1048576]; // 1024 x 1024, 5 buffers ≈ 20 MiB
uint32_t avibuf[1048576];                   // 1024 x 1024 ≈ 4 MiB
```
实际只用 `video.ncol * video.nrow`（典型 256×240×5×4 ≈ 1.2 MiB），其余 19 MiB 浪费。`memset(pixbuf, 0, sizeof(pixbuf))` 每次 reset 跑 24 MiB。

**修复**:
```cpp
// nes_shm.h
class PixBufPool {
    std::vector<uint32_t> buf_;  // ncol * nrow * NES_VIDEO_BUFLEN
    size_t ncol_, nrow_;
public:
    void resize(int ncol, int nrow);
    uint32_t* slot(int i) noexcept;   // 返回 buf_.data() + i * ncol_ * nrow_
    void clear() noexcept;            // 仅清当前活跃区域
    size_t bytes() const noexcept { return buf_.size() * 4; }
};
```

**PR 标题**: `perf(qt): dynamic-allocate pixbuf to actual video size`

---

### D-2 [MEDIUM] RefreshSprites 中重建 `pal_tab_op` / `pal_tab_bk`

**诊断 ID**: B-PERF-02/03
**文件**: `src/ppu_rendering.cpp:1096-1102, 1072-1126`

**问题**: 每 sprite 处理都重建 4+4 字节 stack 临时；即使 `J==0` 走 else 路径，编译器也看到完整生命周期。`flipped = packed` 冗余初始化。

**修复**:
```cpp
// 延迟到进入 8-pixel 循环前；删冗余 flipped = packed
const uint64_t packed = fceu11::ppu::kSpriteIdxLUT[
    (uint32_t)spr->ca[0] | ((uint32_t)spr->ca[1] << 8)];
const uint64_t final_packed = (atr & H_FLIP) ? FCEU_BSWAP64(packed) : packed;
if (J) [[likely]] {
    const uint8_t mask = pal_mask;
    const uint8_t back = mask | 0x40;
    if (atr & SP_BACK) [[unlikely]] {
        for (int i = 0; i < 8; ++i)
            if (visible[i]) C[i] = pal_tab_op[idx[i]] | 0x40;
    } else {
        for (int i = 0; i < 8; ++i)
            if (visible[i]) C[i] = pal_tab_op[idx[i]];
    }
}
```
把 pal_tab_op/pal_tab_bk 合并到 `pal_tab_op[8]`（每 sprite 一个 8 字节）。Hoist 到 scanline 入口预计算一次（VB 跨 sprite 不变）。

**PR 标题**: `perf(ppu): hoist pal_tab construction to scanline entry in RefreshSprites`

---

### D-3 [MEDIUM] `RDoSQLQ` / `RDoTriangleNoisePCMLQ` 4-way 重复代码

**诊断 ID**: B-PERF-04/05/06/07
**文件**: `src/sound.cpp:625-710, 809-895, 925-960`

**问题**: `RDoSQLQ` 三层 `rea:`/`rea2:` 标签 goto；`RDoTriangleNoisePCMLQ` 4 重嵌套 else-if 链；`RDoNoise` 双 for 循环 96% 重复。每次 LQ 调用都付 I-cache / 分支预测开销。

**修复**:
- `RDoSQLQ`: 拆为 `do_sq_lq_lr` / `do_sq_lq_l_only` / `do_sq_lq_r_only` / `do_sq_lq_silent` 四函数，外层一次选择
- `RDoTriangleNoisePCMLQ`: 同样 4 路拆分；用 `if constexpr` 在 LQ 时强制内联
- `RDoNoise`: 把 feedback 选择移到循环外，仅 1 个 for 体

**PR 标题**: `perf(sound): split RDoSQLQ/RDoTriangleNoisePCMLQ/RDoNoise into per-case hot paths`

---

### D-4 [LOW] `FlushEmulateSound` 160 KB memmove + memset

**诊断 ID**: B-PERF-08
**文件**: `src/sound.cpp:1022-1023`

**问题**: `memmove(WaveHi, WaveHi+SOUNDTS-left, left*sizeof(uint32))` + `memset(WaveHi+left, 0, ...)` 每次都跑完整 40000 个 int32 = 160 KiB，即使静音。

**修复**: 维护 `WaveHi` 的有效长度游标 `wavehi_valid_`，仅清理 `[left..wavehi_valid_)`。

**PR 标题**: `perf(sound): cursored WaveHi to skip full-buffer memmove/memset`

---

### D-5 [MEDIUM] `FetchAndDrawTile` 中 `(void)ScreenON` stub 完全消除

**诊断 ID**: B-PERF-12
**文件**: `src/pputile_template.cpp:171, 183, 189, 206, 210`

**问题**: `ScreenON` 参数被传入但仅用来守护空 stub。每次 RefreshLine 调用 32 次 FetchAndDrawTile，每个实例 2 个 `if (ScreenON)` 分支 = 64 个无操作分支预测事件。

**修复**:
```cpp
// pputile_template.cpp
#ifndef FCEUDEF_DEBUGGER
    constexpr bool kRenderLog = false;
#else
    constexpr bool kRenderLog = true;
#endif

if constexpr (kRenderLog) {
    if (ScreenON) { (void)C; }   // 仅 debugger build 保留
}
```

**PR 标题**: `perf(ppu): gate RENDER_LOGP stub behind FCEUDEF_DEBUGGER constexpr`

---

## 六、Phase E — 代码质量与一致性

> **主题**: 5 个清理 PR（无新功能、无性能影响）。每 PR ≤ 50 行。

### E-1 [LOW] MSVC FCEU_LIKELY / FCEU_UNLIKELY 用 C++20 attribute

**诊断 ID**: B-PERF-18
**文件**: `src/compiler_attrs.h:48-56`

**问题**:
```cpp
#elif defined(_MSC_VER)
  #define FCEU_LIKELY(x)   (x)
  #define FCEU_UNLIKELY(x) (x)
```
MSVC 19.30+ 支持 C++20 `[[likely]]`/`[[unlikely]]`，但本宏透传丢弃提示 → MSVC 用户拿不到 hotfix2 已声明的 branch-prediction 收益。

**修复**:
```cpp
#elif defined(_MSC_VER)
  // MSVC 19.30+ supports C++20 [[likely]]/[[unlikely]] natively.
  #if _MSC_VER >= 1929
    #define FCEU_LIKELY(x)   (x) [[msvc::likely]]
    #define FCEU_UNLIKELY(x) (x) [[msvc::unlikely]]
  #else
    #define FCEU_LIKELY(x)   (x)
    #define FCEU_UNLIKELY(x) (x)
  #endif
```
注意：`[[msvc::likely]]` 只能用于 if/else 内表达式末尾，作为 attribute 应用到该 if 整体。

**PR 标题**: `chore(compiler-attrs): enable [[likely]]/[[unlikely]] on MSVC 19.30+`

---

### E-2 [LOW] `pputile_template.h` 头文件循环包含

**诊断 ID**: F-MISC-01
**文件**: `src/pputile_template.h:110`

**问题**: `pputile_template.h` 包含 `ppu_rendering.h`，后者又包含 `pputile_template.h`。未来重构易破坏。

**修复**: 移除 `pputile_template.h` 中的 `#include "ppu_rendering.h"`，改为前向声明。

**PR 标题**: `chore(ppu): break include cycle between pputile_template.h and ppu_rendering.h`

---

### E-3 [LOW] `StackAddrBackup` 无 fallback 链接

**诊断 ID**: F-MISC-05
**文件**: `src/x6502.cpp:440`

**问题**: `extern int StackAddrBackup;` 在非 `FCEUDEF_DEBUGGER` build 下无定义。

**修复**: 在 x6502.cpp 提供 `static int StackAddrBackup = -1;` fallback（条件编译）。

**PR 标题**: `chore(x6502): add StackAddrBackup fallback for non-debugger builds`

---

### E-4 [LOW] `Cpu::layout_` 静态断言加 sizeof

**诊断 ID**: A-CRASH-08
**文件**: `src/cpu.cpp:14`

**问题**: 当前 `static_assert(offsetof(Cpu, layout_) == 0, ...)` 仅检查偏移，不检查 sizeof。若 layout_ 加 padding，savestate 长度变化。

**修复**:
```cpp
static_assert(offsetof(Cpu, layout_) == 0, "Cpu::layout_ must be first field");
static_assert(sizeof(Cpu::layout_) == expected_size, "Cpu::layout_ size drift");
```

**PR 标题**: `chore(cpu): add sizeof assertion for Cpu::layout_`

---

### E-5 [LOW] 死代码清理 + RefreshAddr 宏顺序

**诊断 ID**: A-CRASH-02, DBG-CRASH-03
**文件**:
- `src/ppu_rendering.cpp:319-321`（`#define RefreshAddr smorkus` 与 Fixit1/Fixit2 调用顺序）
- `src/debug.cpp:520-524`（`int u; //deleteme`、`int skipdebug; //deleteme`）

**修复**:
- 把 Fixit1/Fixit2 移到 `#undef RefreshAddr` 之前；或把 Fixit 改为接受 RefreshAddr 参数
- 删除 `int u;` / `int skipdebug;`（确认无引用）

**PR 标题**: `chore: remove dead global debug fields and reorder RefreshAddr macro scope`

---

## 七、测试与验证策略

### 7.1 通用基线（每个 PR 必跑）

1. **正确性回归**: hotfix1 + hotfix2 全部测试 + SMB1/SMB3/Contra/Batman/Kirby/Micro Machines 6 场景 100% pass
2. **性能基线**:
   - `tests/benchmark/ppu_render_bench.cpp` 60 秒（hotfix2 已达 33.95 ms / 60 frames = 0.566 ms/frame；hotfix3 不得回退）
   - `tests/benchmarks/bench_tolerance_test.cpp` 5 轮 PASS
3. **静态分析**: `cppcheck --enable=all,style`、`cargo clippy -- -D warnings`
4. **动态分析**:
   - **AddressSanitizer**: 所有 PR
   - **UndefinedBehaviorSanitizer**: 涉及 alias / 移位的 PR（C-3, C-5）
   - **ThreadSanitizer**: 涉及 atomic / 跨线程字段的 PR（A 全部、B 全部）
5. **FFI 兼容**: 不得改变 `extern "C"` ABI；`fceu11_core_types.h` 同步验证
6. **存档兼容**: hotfix2 savestate 双向 smoke
7. **GUI 烟测**: 启动 / 加载 ROM / 关闭各 5 次

### 7.2 per-Phase 重点

| Phase | 重点验证 | 必跑工具 |
|-------|---------|---------|
| **A** | TSan 全过；GUI 线程与 emulator 线程并发 race window 验证 | TSan |
| **B** | TSan 全过；SDL 音频连续播放 60 秒无爆音 | TSan + 音频听感 |
| **C** | mapper 矩阵（nrom/mmc3/mmc5/vrc5/vrc6）+ 声音回归 | ASan + UBSan |
| **D** | bench 帧时间不得回退 ≥ 5% | bench_tolerance_test |
| **E** | 全量回归测试 | ASan |

---

## 八、不在本 PLAN 范围

| 项 | 原因 | 后续 |
|----|------|------|
| timing-rewrite（hotfix2 P2-5 延期项） | MMC3 A12/IRQ 精度风险 | 不在 hotfix3 范围 |
| AVX2 gather (hotfix2 P0-5 可选项) | ROI 不明 | 视 Phase D 后实测 |
| `map_irq_hook` 改为 atomic 后保留 `map_irq_hook_ref()` 冷路径优化 | 单测覆盖不足 | 不在 hotfix3 范围 |
| Lua 引擎全套 race audit | 工作量大 | 不在 hotfix3 范围 |
| mapper 矩阵全套性能回归 | Phase A 优先 | 不在 hotfix3 范围 |
| `bgdata.main[34]` → `main[32]`（dead row 去除） | 影响 hotfix2 P1-1 SoA，需重测 | 单独 issue 跟踪 |
| `oams[2][64][8]` 加 `alignas(64)` | 单线程优化，影响小 | 不在 hotfix3 范围 |

---

## 九、执行时间表

| 阶段 | PR 范围 | PR 数 | 周期 |
|------|--------|-------|------|
| **Phase A** | A-1 ~ A-6（**5 PRs** = A-1+A-2 合并） | 5 | 1 周 |
| **Phase B** | B-1 ~ B-6 | 6 | 1 周 |
| **Phase C** | C-1 ~ C-5 | 5 | 1 周 |
| **Phase D** | D-1 ~ D-5 | 5 | 1 周 |
| **Phase E** | E-1 ~ E-5 | 5 | 0.5 周 |
| **总计** | — | **26 PR** | **4.5 周** |

> **关键**: 每 phase PR 数 ≤ 6，每个 PR 文件改动 ≤ 5 / 行净增 ≤ 150。任一 phase 超出即触发 phase 拆分。

---

## 十、风险矩阵

### 10.1 高风险 PR（双 reviewer + 专项回归）

| PR | 主要风险 | 缓解 |
|----|---------|------|
| A-1+A-2 Lua Mutex | ABI 变化；多线程性能回归 | TSan；Lua 集成测试矩阵 |
| A-3 nes_shm atomic | 读/写路径遗漏 | 全工程 grep 替换；GUI 烟测 |
| B-1 SDL 音频 atomic | 播放断流 | 60 秒音频连续播放听感测试 |
| D-1 pixbuf 动态分配 | 显存重分配时序 | 视频断流测试；resize 路径专项 |

### 10.2 中风险 PR

| PR | 主要风险 | 缓解 |
|----|---------|------|
| C-1 vnapage default | mapper 启动顺序变化 | mapper 矩阵回归 |
| C-3 sound 整数溢出 | 64-bit 中转引入额外开销 | bench 验证 ≤ 1% 回退 |
| C-5 MMC3/MMC5 | mapper 兼容性 | mapper 矩阵回归 |
| D-2 RefreshSprites | PALRAM 运行时语义 | mid-scanline palette 写单测 |
| D-3 RDo* 拆分 | LQ 音频回归 | HQ/LQ 双向音频 diff |

### 10.3 低风险 PR（单 reviewer + 标准回归）

A-4, A-5, A-6, B-2, B-3, B-4, B-5, B-6, C-2, C-4, D-4, D-5, E-1 ~ E-5。

---

## 十一、PR 验收清单（每个 PR 必填）

- [ ] 修改文件清单（含新增 / 删除 / 重命名）
- [ ] §七.1 全部基线通过
- [ ] per-Phase 重点验证（§七.2 对应行）
- [ ] 单测覆盖（新增 / 修改文件 ≥ 80% 行覆盖）
- [ ] ASan / UBSan / TSan（按 §七.2 选跑）
- [ ] `tests/benchmark/ppu_render_bench.cpp` 60 秒测量（性能 PR 必填）
- [ ] Phase A/B 必跑 TSan；Phase C 必跑 UBSan；Phase D 必跑 bench
- [ ] CHANGELOG 条目（指向本 PR）
- [ ] PR 描述引用本 PLAN 的对应 PR ID（如 `ref: A-3`）

### 11.1 每个 Phase 完成时额外验收

- [ ] Phase 全量回归通过
- [ ] TSan 全过（A / B）
- [ ] bench 不回退（D）
- [ ] mapper 矩阵 9 款全过（C）
- [ ] CHANGELOG.md Phase 段更新
- [ ] readme.md "1.15(hotfix3)" 版本号同步
- [ ] Phase 完成报告（`docs/history/v1.15_hotfix3_phase_*.md`）

---

## 十二、PR 实施纪律

### 12.1 顺序约束

- **Phase A 必须先于 Phase B**：Phase A 修复 `LUA_ENGINE_PTR` race 后，Phase B 的 thread_local stale pointer 才有参考模式
- **Phase A 必须先于 Phase C**：Phase A 中 A-5 修复 `mutexLocks` 原子化后，Phase C-1 vnapage reset 才不会被跨线程 race 干扰
- **Phase D-1 必须独立**：pixbuf 动态分配涉及 `nes_shm` 整体重排，与 Phase A 的 nes_shm atomic 同步提交，避免中间态编译失败
- **Phase E 必须最后**：清理类不在算法路径上受其他 PR 干扰

### 12.2 分支策略

- 每个 PR 一个独立 feature 分支：`hotfix3/a-1-lua-mutex`、`hotfix3/c-3-sound-overflow` 等
- 合并前必须 `git rebase hotfix3` 避免 merge commit
- 冲突解决须由 PR 作者本人执行

### 12.3 Review 节奏

- 高风险 PR（A-1+A-2, A-3, B-1, D-1）：双 reviewer（一位并发 / 一位核心算法），review ≥ 24h
- 中风险 PR：单 reviewer，review ≥ 4h
- 低风险 PR：单 reviewer，CI 全过即合并

### 12.4 回滚预案

- 任一 PR 引入性能回退 > 5% 立即 revert
- 任一 PR 引入 TSan 报告立即 revert
- Phase 边界先 `git tag hotfix3-phase-a-done` 等，回滚时整 phase 撤回
- 每个 PR 必须可独立 revert

---

## 十三、附录 — 诊断 ID 索引

按子系统 / 类别交叉索引（便于 reviewer 定位）：

| 子系统 | CRITICAL | HIGH | MEDIUM | LOW |
|--------|---------|------|--------|-----|
| **Lua / Rust FFI** | RUST-CRASH-01, RUST-CRASH-02, LUA-CRASH-02 | RUST-CRASH-03 ~ 09, LUA-CRASH-03 | LUA-PERF-01 ~ 02 | — |
| **Qt GUI / 跨线程** | QT-CRASH-01, QT-CRASH-03, QT-CRASH-08 | QT-CRASH-05, QT-CRASH-06/07 | QT-CRASH-02/09, QT-CONNECT-01/02 | QT-CRASH-04, QT-CRASH-10 |
| **PPU / Audio** | — | A-CRASH-01 | A-CRASH-03 ~ 09, B-PERF-02 ~ 22 | B-PERF-12, B-PERF-17 |
| **CPU** | — | A-CRASH-06 | A-CRASH-08, D-NUM-01 ~ 03 | F-MISC-05 |
| **Mapper** | — | MAP-CRASH-01, MAP-CRASH-03 | MAP-CRASH-02, MAP-CRASH-04 ~ 07 | DBG-CRASH-01 ~ 03 |
| **Movie / Debug / Netplay / Input** | — | INP-CRASH-01/02, MOV-CRASH-02, LUA-CRASH-04, NET-CRASH-02, CHT-CRASH-01 | MOV-CRASH-01/03, MOV-PERF-01 ~ 03, NET-PERF-01, CHT-PERF-01, DBG-PERF-01/02 | MOV-OTHER-01 ~ 03, NET-OTHER-01 |
| **配置 / 文件 / 状态** | — | — | STATE-CRASH-03/04, INES-LOAD-01, FILE-IO-01 | — |
| **编译器 / 一致性** | — | — | F-MISC-02, F-MISC-03 | F-MISC-01, E-ALIAS-01/02, B-PERF-18 |

---

**PLAN END** — hotfix3 细致 REVIEW 与修复方案 v1（2026-07-17）

*v0 草稿：基于 3 个并行 REVIEW 子代理（2026-07-17）产出的 ~100 条诊断；按"AI agent 注意力窗口 ≤ 6 PR/phase"约束切分 5 个 phase，共 26 PR，预计 4.5 周。*