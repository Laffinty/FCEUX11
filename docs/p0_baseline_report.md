# P0 基线报告 — topic/v0.3.10-api-convergence

## 分支信息
- 分支: topic/v0.3.10-api-convergence
- 基线 commit: f48a408053232db0c69ed6f0d58da60150af3609
- 时间: 2026-06-13T08:25:18+08:00
- 构建目录: build/（Visual Studio 18 2026 生成器，x64，Release）

## 符号计数基线
- `FCEUI_*` 出现次数: 1118
- `EMUFILE` 出现次数: 291
- `EMUFILE_MEMORY` 出现次数: 97

## EMUFILE 调用方清单（按文件，行数）
| 行数 | 文件 |
|------|------|
| 36 | src/emufile.cpp |
| 19 | src/emufile.h |
| 17 | src/movie.cpp |
| 16 | src/utils/endian.h |
| 16 | src/state.cpp |
| 10 | src/movie.h |
| 9 | src/utils/endian.cpp |
| 7 | src/drivers/Qt/fceuWrapper.cpp |
| 5 | src/utils/xstring.h |
| 5 | src/file.h |
| 5 | src/drivers/Qt/TasEditor/selection.h |
| 5 | src/drivers/Qt/TasEditor/selection.cpp |
| 5 | src/drivers/Qt/TasEditor/TasEditorWindow.cpp |
| 4 | src/file.cpp |
| 4 | src/drivers/Qt/TasEditor/greenzone.cpp |
| 3 | src/utils/fceu11_expected.cpp |
| 3 | src/io_api.h |
| 3 | src/emufile_types.h |
| 3 | src/drivers/Qt/TasEditor/taseditor_project.cpp |
| 3 | src/drivers/Qt/TasEditor/snapshot.h |
| 3 | src/drivers/Qt/TasEditor/snapshot.cpp |
| 3 | src/drivers/Qt/TasEditor/markers.h |
| 3 | src/drivers/Qt/TasEditor/markers.cpp |
| 3 | src/drivers/Qt/TasEditor/laglog.h |
| 3 | src/drivers/Qt/TasEditor/laglog.cpp |
| 3 | src/drivers/Qt/TasEditor/inputlog.h |
| 3 | src/drivers/Qt/TasEditor/inputlog.cpp |
| 3 | src/drivers/Qt/TasEditor/bookmark.h |
| 3 | src/drivers/Qt/TasEditor/bookmark.cpp |
| 2 | src/state.h |
| 2 | src/lua-engine.cpp |
| 2 | src/drivers/Qt/TasEditor/markers_manager.h |
| 2 | src/drivers/Qt/TasEditor/markers_manager.cpp |
| 2 | src/drivers/Qt/TasEditor/history.h |
| 2 | src/drivers/Qt/TasEditor/history.cpp |
| 2 | src/drivers/Qt/TasEditor/greenzone.h |
| 2 | src/drivers/Qt/TasEditor/branches.h |
| 2 | src/drivers/Qt/TasEditor/branches.cpp |
| 2 | src/drivers/Qt/TasEditor/bookmarks.h |
| 2 | src/drivers/Qt/TasEditor/bookmarks.cpp |
| 2 | src/drivers/Qt/TasEditor/TasEditorWindow.h |
| 1 | src/utils/xstring.cpp |
| 1 | src/utils/fceu11_expected.h |
| 1 | src/rust/target/fceux11_rust_formats.h |
| 1 | src/rust/fceux11_rust.h |
| 1 | src/oldmovie.cpp |
| 1 | src/drivers/Qt/StateRecorderConf.cpp |

## 测试闸结果

| 闸 | 内容 | 状态 | 备注 |
|---|------|------|------|
| 闸 1 | Release 全量构建 | ✅ 通过 | 0 errors；Visual Studio 18 2026；vcpkg 本地依赖 |
| 闸 2 | `ctest --test-dir build` | ✅ 通过 | 6/6 通过 |
| 闸 3 | `cargo test --workspace` | ❌ 未通过 | `fceux11-lua` / `fceux11-rust` 对 C++ FFI 符号存在未解析外部引用 |
| 闸 4 | `rom_regression_test` 5 ROM 哈希 | ✅ 通过 | 比较 720 帧，0 mismatch，RESULT: PASSED |

### ctest 详情
```
Test project C:/Users/ikrx2/Desktop/project/FCEUX11/build
    Start 1: smoke_test
1/6 Test #1: smoke_test .......................   Passed    0.42 sec
    Start 2: mapper_load_test
2/6 Test #2: mapper_load_test .................   Passed    0.10 sec
    Start 3: mapper_reset_test
3/6 Test #3: mapper_reset_test ................   Passed    0.11 sec
    Start 4: rom_regression_test
4/6 Test #4: rom_regression_test ..............   Passed    0.66 sec
    Start 5: expected_api_test
5/6 Test #5: expected_api_test ................   Passed    0.11 sec
    Start 6: enum_class_bitflags_test
6/6 Test #6: enum_class_bitflags_test .........   Passed    0.01 sec

100% tests passed, 0 tests failed out of 6
```

### rom_regression_test 详情
- 模式: VERIFY against golden hashes
- 测试 ROM 数: 13（含规划要求的 5 ROM）
- 比较帧数: 720
- 不匹配帧数: 0
- 结果: **PASSED**

### cargo test --workspace 说明
当前 `cargo test --workspace` 因 `fceux11-lua` / `fceux11-rust` crate 对 C++ FFI 符号（`fceux11_lua_*`，实现于 `src/lua-engine.cpp`）存在未解析外部引用，无法直接通过。非 FFI 依赖 crate 的测试可独立通过：

```bash
cd src/rust
cargo test -p fceux11-utils -p fceux11-media -p fceux11-formats -p fceux11-debug -p fceux11-core
```

该问题属于 P5（Rust 侧同步与 FFI 边界检查）的前置现状，P0 阶段记录为基线偏差。项目 CI（`.github/workflows/ci.yml`）当前未将 `cargo test --workspace` 纳入自动验证。

## fceux11_rust.h 冻结声明
自本分支起，`src/rust/fceux11_rust.h` 由 cbindgen 自动生成，任何签名变更需遵循 `docs/tech/05_Project_Development_Guide.txt` 中的 RFC 流程，并在 P5 阶段同步所有 C++ 调用方。

## 结论
P0 已完成分支切出、基线数据落盘、Release 构建、ctest 与 ROM 回归验证。唯一偏差为 `cargo test --workspace` 因既有 FFI 链接问题未通过，建议在 P5 阶段处理。
