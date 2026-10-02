# 跨 FFI 工程约束

> 面向的读者：要在 Rust ↔ C++ 之间搬运数据、加导出、或排查「符号在库里但调不到」
> 的人。全部是 FCEUX11 的实测结论，不是通则。GBA 模块是本仓库第一个手写 C ABI 的
> Rust 模块，所以这些约束的来源都是它。

---

## 一、手写 C ABI 与 cbindgen 的分工

`cbindgen` 只对**成员 crate** 跑，根 crate 的 ABI 它从不指向。所以：

- **成员 crate**（`fceux11-core` 等）：导出由 `#[no_mangle] extern "C"` 决定，
  cbindgen 自动生成头文件。
- **根 crate**（`src/lib.rs`，GBA 在此）：`build.rs` 里**手写**声明列表，
  cbindgen 只负责把那些声明写进 `src/rust/fceux11_rust.h`。

**加导出函数必须同步两处**：`build.rs` 的手写声明 + 头文件的生成时机。

### 头文件会在你不改它的时候变化

`build.rs` 的 `rerun-if-changed` 列表**必须包含整个新源码目录**。漏了它的
症状极其隐蔽：库重编了，派生头没重生成 → 符号在 `.lib` 里，头文件里没有声明 →
C++ 侧**能链接、不能调用，全程零报错**。

而 GBA 当时的默认行为是 `build.rs` 只看自己，于是「在 `src/gba/` 新增导出函数
不会触发 `build.rs`」。已补。

## 二、生成头被两个 target 目录共用

`build.rs` 被两个独立的 cargo target 目录各跑一次：

| 目录 | 由谁触发 |
|---|---|
| `src/rust/target` | `cargo test` |
| `build/src/rust/target` | CMake 的 `add_custom_command` |

**谁最后跑，谁写 `src/rust/fceux11_rust.h`。** 于是它可以一边被重新生成、
一边留着另一份旧的。真实发生过：改完 `build.rs` 的错误码之后，`ctest` 绿、
头里还是 `GBA_ERR_STATE 4`。

**规则：改完 `build.rs`，直接查头文件的内容，不要假定构建跑过就等于它更新了。**
锁测试 `the_error_codes_reach_the_generated_header` 守的就是这一条。

## 三、大对象一律出参 + 显式 free

FCEUX11 在 `core_api.h` 与 `fceux11-core/src/state_file.rs` 里已经定过一次
（`FceuStateChunkOutput { data, len, cap }`、五个出参、所有权转移）。GBA 沿用它。

要点：

- **不把大对象按值跨 FFI 返回**。`Arm7tdmi` 实测 82,032 字节（`gba_probe_cpu_size`，
  其中 76,800 是 240×160 帧缓冲）；这个尺寸的值在 FFI 边界上要materialize 到
  **调用方的栈**上，而调用方的栈不由我们控制，也不能要求它一定够。
- **`cap` 必须外传**，不是冗余。`Vec::as_mut_ptr` 交出去后用 `from_raw` 回收，
  只传 `len` 不传真 `capacity`，释放走的就是错误布局。
- free 用 `drop(Box::from_raw(..))`，**不要** `Vec::from_raw_parts(ptr, len, len)`
  —— `cap > len` 时是未定义行为。

## 四、栈约束必须实测，且只能在调用方解决

`gba_savestate_load` 的解码路径实测需要 **2 MB（Release）/ 6–7 MB（Debug）** 栈，
而 Qt 的 `QThread` 只有 1 MB（Windows 默认）。

三种可能做法里只有一种能接受：

| 做法 | 评价 |
|---|---|
| 要求调用方提供 16 MB 栈 | ✗ 谁都不会遵守的隐式契约 |
| 在 `extern "C"` 帧上解 | ✗ 栈由调用方提供，控制不了 |
| **自带栈的工作线程** | ✓ 已采用（`save.rs` 的 `load_on_worker`，栈大小为实测峰值的 8 倍） |

同一条经验的第二半：**测试 harness 的栈不是产品栈**。Rust 测试线程默认 2 MB，
所以锁测试必须显式跑在 16 MB 线程上。**写测试时用的环境和产品不是同一个环境，
这是「自建测试全绿、实机崩」的一类常见来源。**

## 五、`extern "C"` 出口面必须能被安全调用

符号存在 `.lib` 里只是最弱的一档证据。两条硬要求：

- **不能 unwind。** `panic = "abort"` 之外，还要避免 `extern "C"` 函数里做可能
  panic 的操作（数组越界、算术溢出、`unwrap()`）。
- 指针参数要在函数入口判空并返回错误码，不要解引用后再判。

## 六、并发共享的全局状态

Rust 侧用 `Mutex<Option<Machine>>` + `OnceLock`，**不用** `static mut`。
后者在 r32 造成过 `STATUS_HEAP_CORRUPTION`：测试 harness 并行跑用例，
多个用例同时装卸 `Box<Gba>`，`rtrb` 环的所有权跨 `Box` 移动 → 堆损坏。
单跑某条测试则通过。

**互斥锁解决了「安全」，没解决「隔离」。** 用例之间仍会逻辑干扰（关水印那条会
读到另一条用例刚取的帧），所以另有 `exclusively(|| …)` 把 9 条用例串行化。

**两个词不要混用**：safe（并发访问不损坏内存）与 isolated（用例互不干扰）是
两件事，只做前者会在第一条失败的测试之后引发连锁失败。

## 七、导出数量的自检

改完 ABI 后核对导出数：

```powershell
dumpbin /linkermember:1 build\src\rust\target\x86_64-pc-windows-msvc\release\fceux11_rust.lib |
  Select-String "gba_"
```

应与 `src/rust/fceux11_rust.h` 的声明数一致。**当前 34 个。**

注意只查 CMake 真正链接的那一份 `.lib`（`src/rust/CMakeLists.txt` 里的
`${CMAKE_CURRENT_BINARY_DIR}/target/release/`），S0 时期 `build/` 下曾并存
4 份实验产物，按「找第一个 `.lib`」查会得出错误结论。

## 八、相关文档

- [gba-traps.md](gba-traps.md) §四 — 「合成夹具」在 FFI 场景下的特殊形态
- [multi-machine-integration.md](multi-machine-integration.md) — 跨 FFI 的分层约束
- [../audit/gba-third-party.md](../audit/gba-third-party.md) — 上游代码的许可边界
- `src/rust/build.rs` — 手写 ABI 声明的唯一事实源
