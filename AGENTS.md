# FCEUX11 — 接手须知

> 本文件是**入口**。第一次接触本仓库、或换人接手时先读这里，再进治理文档。
> 里面每条「坑」都是实际卡过一次才写下来的，不是预防性提醒。

## 仓库在做什么

FCEUX11 是 NES 模拟器。`wip2.0` 分支上并行着 **GBAEUX11 v2.0**：把一个 GBA 硬件核心接进来，
做成 `.gba` 可玩的第二台机，同时 **NES 侧必须零回归**（`108P / 12F`，grade B）。

## 分支

| 分支 | 用途 |
|---|---|
| `main` | 发布线 |
| `wip2.0` | GBAEUX11 v2.0 的全部工作；推上去会触发外部门禁（f11qa.yml 已含 `wip2.0`） |

## 治理文档（事实源）

| 文档 | 管什么 |
|---|---|
| `docs/plans/FCEUX11-v2.0_GBAEUX11构建计划.md` | 范围、阶段编排（§八）、**已知限制（§9.1）**、不变式（§十）、变更记录（§十四 r1–r13） |
| `src/rust/crates/gba-core/ATTRIBUTION.md` | 上游出处、本地补丁集**逐处**说明、重新 vendor 流程 |
| `COPYRIGHT_AUDIT.md` §5 | Rust vendor 树在 `src/` C/C++ 扫描范围之外的归属声明 |

> `docs/plans/long-term-evolution/` 是 **NES 侧** F11QA 残留精度的长期演进，与 v2.0 无关，别混。

## 当前进度（2026-10-01）

| 阶段 | 状态 |
|---|---|
| S0 / S0' / S1a-0 / S1a-1 / S1b / S1c | ✅ 已完成（S1c：16 个号码，锁测试 132 项） |
| **S2-a** | **✅ 已完成**（r32）：生命周期 + 帧缓冲 + `BETA` 水印，**16** 个 C ABI 函数 |
| **S2-c** | **✅ 已完成**（r33）：5 条 SWI 往返在**真实 ARM 指令流**下通过 |
| **S2-b** | **进行中 · S2-b1（音频）✅ 落地，r35**。S2-b 其余三项（savestate 导出、RTC、Qt 接线）未开始 |
| S3 / S4 | 未开始 |

> ⚠️ **不变式 1 的判据是 F11QA 矩阵 `108P/12F`**，本机跑的是它的**本地近似**
> `ctest`（34 项）。当前状态 **`ctest` 33/34**，唯一失败的 `bench_tolerance_test`
> 比对的是 v0.3.0 的性能基线（跨了 11 个版本），**已在干净树复现、与改动无关**。
> GBA 锁测试 **185** 项全绿。基线刷新属性能治理，未做。
>
> ⚠️ **jsmolka 外部门禁在本环境不可执行**（用户裁决 2026-10-01：暂不处理）：
> 无 FASMARM、零 `.gba` 资产、CI 未接入，且它只用**屏幕文字**报结果，还需自写
> mode 4 文字读取器。**这是 S2 出口前必须解决的阻塞项**，L4 / L7 指向它。

v2.0 对 vendor 树的本地修改**集中在 `src/rust/crates/gba-core/src/cpu/arm7tdmi.rs` 一个文件，
共 9 处**（S0' 接线 5 处 + S1a-1 的 halt 机制 4 处），其余 44 个 vendor 文件零改动——
改动前先确认这一点是否仍成立。第 6–9 处**确实改变上游行为**（`step()` 新增守卫、
`Arm7tdmi::new` 清 CPSR 的 I 位），不再属于 R1 所说的「仅 SWI hook」，见计划 **R15**。

## 代码地图

```
src/rust/src/gba/            一手 GBA 逻辑（根 crate 的模块，不是独立 crate）
  mod.rs                     接线：install_swi_hook
  bios.rs                    stub BIOS，16 KB 镜像由 const fn 逐字写入
  ffi.rs                     C ABI 探针面（5+1 个符号）
  frame.rs                   生命周期 + 帧缓冲 + BETA 水印（10 个符号）
  audio.rs                   分数采样时钟 + 转换链 + 5 个符号（S2-b1）
  overlay.rs                 BETA 水印
  gate.rs                    真实 ARM 指令流的自建门禁（S2-c）
  save.rs                    savestate 地基（serde_json，导出面待做）
  swi/mod.rs                 dispatch：认领哪些 SWI 在这里决定
  swi/wait.rs                Halt/Stop/IntrWait 的纯逻辑
  swi/decompress.rs          LZ77/Huffman/RLE 解压器
src/rust/crates/gba-core/    vendor 的 GBA 硬件核心（clementine, MIT）
src/rust/build.rs            cbindgen → fceux11_rust.h
```

> 根 crate 的 `Cargo.toml` 里有 `rtrb = { optional = true }`：`Gba::init_audio` 返回
> `rtrb::Consumer<f32>`，而消费端必须存在**我们这侧**，所以根 crate 要能命名这个类型。
> 挂在 `gba` feature 下，`--no-default-features` 时不进图（不变式 8）。

## 门禁（这台机器上跑得通的写法）

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# GBA 锁测试
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"

# 两态 feature 检查（第二态同时复核不变式 8：关 feature 时 vendor 树退出编译图）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace --no-default-features"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo check --workspace"

# 全量构建 + NES 零回归
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4
```

### 七个坑

1. **不加载 MSVC 环境就连标准库头都找不到**（`cl.exe` 在，`INCLUDE` 不在）。先 `call vcvars64.bat`。
2. **`cargo test -p fceux11-rust` 必须加 `--no-default-features --features gba`**。默认 feature
   `direct-adapter` 引用只有 C++ 链接才有的 `kagami_bridge_*`，测试二进制直接 LNK2019 链不过。
3. **`ctest` 不在 PATH**，在 `D:\Program Files\CMake\bin\`。
4. **跑 `build/tests/*.exe` 要把 `vcpkg_installed\x64-windows\bin` 前置到 PATH**，否则 `0xC0000135`。
5. **仓库在 HEAD 上本来就不是 rustfmt-clean 的**，`cargo fmt --check` 不能当门禁。
   只保证**自己新写的行**符合 rustfmt，别整仓跑 `cargo fmt`。
6. **Ninja 对头文件变更不可靠**：改完 `src/version.h` 之类的「只被头文件引用」的值，
   可能不会触发重编，链接日志有 `Linking` 而二进制里还是旧值。改完要回**产物**里确认。
7. **`build/` 的 Ninja 定位可能漂移，且 MSVC 升级会让既有测试编不过**（2026-09-30 实测，
   记为 **L11**）。症状：`CMAKE_MAKE_PROGRAM-NOTFOUND` → 「Generator: build tool execution
   failed」；修好后又见 `C2220 以下警告被视为错误`，出自 **MSVC 14.51** 的
   `__msvc_ostream.hpp` / `chrono`。**这不是 v2.0 改动引起的** —— 判据是
   `git stash` 后在干净树上能否复现同样失败。Ninja 本体在
   `…\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe`（不在 PATH）。
8. **`build/` 下有多份同名 `fceux11_rust.lib`，查符号前先确认 CMake 用的是哪一份**
   （2026-10-01 实测）。S0 时期留了 4 个实验产物目录（`s0p_target` /
   `s0_exp3_target` / `s0_probe_target2` / `build\src\rust\probe`），都停在
   2026-09-28、只含 5 个探针符号；CMake 真正链接的是 `CMakeLists.txt:34` 定的
   `${CMAKE_CURRENT_BINARY_DIR}/target`（即 `build\src\rust\target\`，由
   `CMakeLists.txt:36` 的 `RUST_STATIC_LIB` 拼出）。**我按「找第一个 .lib」的写法查，
   挑中了 `s0p_target` 那份，差点报出一个不存在的缺陷。** 这是坑 6 的同族但更隐蔽：
   坑 6 是「改了不重编」，这个是「有 N 份产物、查错了那一份」——
   **两者都会让你对符号面/产物面得出完全相反的结论。**
   判据：先读 `CMakeLists.txt` 确认路径，再 `dumpbin /linkermember:1 <那一份>`。
9. **GBA 的 C ABI 是手写在 `build.rs` 里的，加导出函数必须同步两处**
   （2026-10-01 实测）。cbindgen 只对**成员 crate** 跑，根 crate 的 ABI 它从不
   指向，所以 `build.rs` 里那份 GBA 声明列表是人写的。两个后果：
   - **`build.rs` 的 `rerun-if-changed` 必须含 `src/gba`**（现已补）。cargo 在脚本
     声明过 `rerun-if-changed` 后就不再用「包内任何文件变化都重跑」的默认行为，
     漏了它就变成**库重编了、派生的 `fceux11_rust.h` 却没重生成** —— 符号在
     `.lib` 里、头文件里没有声明，C++ 侧能链接不能调用，全程零报错。
   - **`src/gba/ffi.rs` 的 `drift_guard` 靠扫源码发现导出，不要把它改回手写清单。**
     手写清单比手写生成的头文件，两边同源漏掉新项就互相抵消、断言照样全绿 ——
     这正是当初 S2-a 那 10 个导出溜过去的原因。改完 ABI 后自查：
     `dumpbin /linkermember:1 build\src\rust\target\x86_64-pc-windows-msvc\release\fceux11_rust.lib`
     的 `gba_*` 数量应与 `src/rust/fceux11_rust.h` 的声明数一致（当前 21）。

## 纪律（这几条都是被违反过之后写下来的）

1. **先只读调查 + 出 PLAN，用户拍板后再动文件。**
2. **先记入 §十四 变更记录，再改计划**（计划 §十四末尾的「定稿后的变更纪律」）。
   出口判据的改写也要记，不许静默放宽。
3. **变异验证**：每条新锁测试都要能把「修复前的值」改回去并看到它转红。全绿不算证据。
4. **不凭记忆写位级格式。** 取不到权威材料就**停下来记录**，不要写。已犯过两次
   （LZ77 头解析连续读错 GBA 与 NDS 两套布局，详见 §十四 r12 / r13）。
   外部参考实现只**读不抄**（mGBA 是 MPL-2.0，与本项目许可不相容）。
5. **测试向量按字段拼装，不手写字面量。** 手算错误在本项目已发生两次，
   而且两次都被自己写的测试盖章通过。
6. **`extern "C"` 出口面必须能被安全调用**：符号在 lib 里是最弱的一档判据；
   导出函数 panic 会在 `extern "C"` 帧里无法 unwind，直接带走整个进程。
7. **NES 侧零回归**是不可变式 1：任何 GBA 改动后都要跑 `ctest`。
