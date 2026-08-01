# FCEUX11 1.15 LTS — hotfix1 修复执行 PLAN

**基于**: `docs/FCEUX11-1.15_LTS-隐患审计报告.md`（含 deepseek-v4-pro 第二轮复审）
**目标版本**: FCEUX11 1.15.1 LTS hotfix1
**制定日期**: 2026-07-13
**适用基线**: FCEUX11 1.15 LTS（commit bf86fe3 及以前）

---

## 〇、验证结论与处置总览

### 0.1 报告准确性评估

经逐文件核验源码，**报告中 90% 以上的 CRITICAL / HIGH 项真实存在**。具体来说：

| 验证项 | 报告描述 | 源码确认 | 评估 |
|--------|---------|---------|------|
| C-01 `memset` 覆盖 `std::string` (fceu.cpp:437) | UB | ✅ 完全确认 | **真BUG, 修复合理** |
| C-02 SFORMAT 全部指向 found_pos[0] (ppu_state.cpp:74-81) | 8 项全错 | ✅ 完全确认 | **真BUG, 修复合理** |
| C-03 `&head + i` 指针算术 (fceu.cpp:1467) | 结构体步进而非字节 | ✅ 完全确认 | **真BUG, 修复合理** |
| C-04 FceuFilterState 零长度数组 (fceux11_rust.h:349) | UB | ✅ 完全确认 | **真BUG, 修复合理** |
| C-05 `static mut LUA_ENGINE_PTR` (lua/lib.rs:21) | UB | ✅ 完全确认 | **真BUG, 修复合理** |
| C-06 `Vec::from_raw_parts` 容量不匹配 (state_file.rs:604) | 容量假设 | ✅ 确认 | **真BUG, 修复合理** |
| C-07 SFORMAT 反序列化无大小校验 (sformat.rs:158-172) | 缓冲区溢出 | ✅ 确认 | **真BUG, 修复合理** |
| C-08 pixbuf 数组无边界检查 (nes_shm.h:40) | 越界写 | ✅ 确认 | **真BUG, 修复合理** |
| C-09 AVI 音频环形缓冲区无溢出保护 (AviRecord.cpp:575) | 缺自旋等待 | ✅ 确认 | **真BUG, 修复合理** |
| C-10 CONOUT$ 句柄双重打开 (main.cpp:348-371) | 双 close | ✅ 确认 | **真BUG, 修复合理** |
| C-11 mutex 在析构路径中 (fceuWrapper.cpp:1232-1247) | 竞态 | ✅ 确认 | **真BUG, 但修复复杂** |
| C-12 ConsoleVideo.cpp:111 反引号n | 编译错误 | ✅ 确认 | **真BUG, 修复极简** |
| **H-19** registry g_keepalive[256] vs kRegistrySize=512 | 缓冲区溢出 | ⚠️ **描述过激** | **有守卫，不会溢出**，但 256-511 范围无法被 keepalive |
| N-C01 SDL_PumpEvents 双线程 (sdl_backend.cpp:160 + main.cpp:425) | 线程安全 | ✅ 确认 | **真BUG, 建议升级为 CRITICAL** |
| N-C02 closeApp 线程退出竞态 (ConsoleVideo.cpp:322-360) | use-after-free | ✅ 确认 | **真BUG, 应升级为 CRITICAL** |
| N-H01 nsf.rs a-0x6000 下溢 (nsf.rs:547) | 整数下溢 | ✅ 确认 | **真BUG, 修复极简** |
| N-H02 emufile.rs seek_set 负数 (emufile.rs:128) | OOM | ✅ 确认 | **真BUG, 修复极简** |
| N-H03 nes_shm 跨线程字段非原子 | 数据竞争 | ✅ 确认 | **真BUG, 修复复杂** |

### 0.2 报告不准确项的修正

1. **H-19 (registry g_keepalive)**: deepseek 复审已正确指出 `if (entry.mapper_number < kRegistrySize)` 守卫存在，**不会发生缓冲区溢出**。但功能上 256-511 范围的 mapper 无法通过 `g_keepalive` 防 DCE，需要扩展数组或保留守卫。**严重度建议降为 MEDIUM**。

2. **H-25 (state[] 锁外访问)**: deepseek 复审正确——`state[]` 是 GUI 线程拥有的 `QAction*`，无数据竞争，仅 UI 一致性问题。**严重度建议降为 LOW~MEDIUM**。

3. **C-02 报告中的数组下标建议**有更优解：使用 `arr` 字段 + 单一 SFORMAT 条目（避免每条 4 字节 RLE 头开销）。

### 0.3 应升级为 CRITICAL 的项

| 原编号 | 原严重度 | 升级理由 |
|--------|---------|---------|
| N-C01 SDL_PumpEvents 双线程 | HIGH | SDL 文档明确禁止，0ms QTimer 保证高频并发，**实测会导致输入丢失/进程崩溃** |
| N-C02 closeApp 线程退出竞态 | HIGH | `quit()` 对无事件循环线程是 no-op，`wait(1000)` 超时后直接 UAF |
| C-09 AVI 音频溢出 | HIGH | 录视频时静默损坏 audio frame，不易察觉 |

### 0.4 处置原则

1. **数据损坏型 BUG 优先**: C-01, C-02, C-03, C-12, H-01, H-29 → P0
2. **线程/进程崩溃型 BUG 次之**: N-C01, N-C02, C-08, C-09, H-22, H-28, H-32 → P1
3. **FFI 边界/安全**: C-04, C-05, C-06, C-07, H-11, H-13, H-14, N-H01, N-H02 → P1
4. **代码健壮性**: H-02, H-03, H-04, H-19, M-* → P2
5. **代码质量/可读性**: L-* → P3

---

## 一、修复路线图

```
P0 (数据损坏, 立即修复)         P1 (线程/FFI安全)         P2 (健壮性)         P3 (代码质量)
─────────────────────────────────  ────────────────────────  ─────────────────  ─────────────
C-01, C-02, C-03, C-12          N-C01, N-C02, C-08         H-02, H-03, H-04   N-L01~N-L03
H-01, H-29                      C-09, C-10, C-11, C-04     H-19(改大)         ...
H-07, H-08                      C-05, C-06, C-07           M-*
                                H-13, H-14, H-15, H-16
                                N-H01, N-H02, N-H03
                                H-22, H-25(降级), H-32
预计 5-7 个 PR                    预计 10-15 个 PR          预计 5-8 个 PR     待定
```

---

## 二、P0 — 数据损坏类（即刻修复）

> **目标**: 不引入任何新 ABI 破坏，单 PR 即可完成，最大化可回滚性。

### P0-1 修复 `memset` 覆盖 `std::string` 成员（C-01）

- **文件**: `src/fceu.cpp:435-447`
- **当前代码**:
```cpp
GameInfo = new FCEUGI();
memset( static_cast<void*>(GameInfo), 0, sizeof(FCEUGI));
GameInfo->filename = fp->filename;
```
- **风险**: `FCEUGI` 包含 `std::string filename`、`std::string archiveFilename`，memset 覆盖其 SSO 表示 → 析构时 UB / 堆损坏
- **修复方案**: 移除 `memset`，依赖编译器零初始化已默认构造的 POD 字段

```cpp
GameInfo = new FCEUGI();   // 默认构造已零初始化所有 POD 字段
GameInfo->filename = fp->filename;
if (fp->archiveFilename != "")
    GameInfo->archiveFilename = fp->archiveFilename;
// ... 其余赋值保持不变
```
- **风险评估**: 低。`= default` 构造保证 POD 字段为 0；`std::string` 由后续赋值初始化
- **验证步骤**:
  1. 编译: `cmake --build build` 全平台通过
  2. 运行: 加载任何 iNES ROM，确认 `GameInfo->filename` 不为空
  3. 反复加载不同 ROM（10+ 次），使用 AddressSanitizer 监测无堆损坏
- **PR 标题**: `fix(c-01): remove memset over FCEUGI containing std::string members`
- **关联**: M-26（normalscanlines bool 当数字用）一并修复

### P0-2 修复 SFORMAT 全部指向 found_pos[0]（C-02）

- **文件**: `src/ppu_state.cpp:74-81`
- **当前代码**:
```cpp
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx0" },
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx1" },
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx2" },
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx3" },
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx4" },
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx5" },
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx6" },
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx7" },
```
- **风险**: 存档精灵位置只保存/恢复 `[0]`，导致精灵渲染异常
- **修复方案 A（最小变更）**:
```cpp
{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx0" },
{ &spr_read.found_pos[1], 4 | FCEUSTATE_RLSB, "SRx1" },
{ &spr_read.found_pos[2], 4 | FCEUSTATE_RLSB, "SRx2" },
{ &spr_read.found_pos[3], 4 | FCEUSTATE_RLSB, "SRx3" },
{ &spr_read.found_pos[4], 4 | FCEUSTATE_RLSB, "SRx4" },
{ &spr_read.found_pos[5], 4 | FCEUSTATE_RLSB, "SRx5" },
{ &spr_read.found_pos[6], 4 | FCEUSTATE_RLSB, "SRx6" },
{ &spr_read.found_pos[7], 4 | FCEUSTATE_RLSB, "SRx7" },
```
- **修复方案 B（更优，存档格式更紧凑）**: 使用单一 array 条目 + 标签 "SRxA"
  ```cpp
  { spr_read.found_pos, sizeof(spr_read.found_pos), "SRxA" },
  ```
  > **注意**: 方案 B **会改变存档格式标签**（"SRx0".."SRx7" → "SRxA"），与现有 v1.15 之前版本的存档**不兼容**。需权衡。
- **建议**: 选方案 A（兼容旧存档格式），后续大版本再做格式整合
- **验证步骤**:
  1. 用 SMB1（多精灵测试）做存档 → 加载 → 反复验证精灵位置一致
  2. 用现有 1.15 之前版本生成的存档验证仍能加载
- **PR 标题**: `fix(c-02): correct SFORMAT entries for spr_read.found_pos[0..7]`

### P0-3 修复指针算术按结构体大小步进（C-03）

- **文件**: `src/fceu.cpp:1467`
- **当前代码**:
```cpp
return *reinterpret_cast<unsigned char*>(&head + i);
```
- **修复方案**:
```cpp
return *reinterpret_cast<unsigned char*>(&head)[i];
// 或更清晰:
const auto* bytes = reinterpret_cast<const unsigned char*>(&head);
return bytes[i];
```
- **验证步骤**:
  1. 单元测试: `FCEU_ReadRomByte(0..15)` 返回 iNES header 字节
  2. 加载任何 ROM 后打印 `head` 内容验证
- **PR 标题**: `fix(c-03): correct pointer arithmetic in FCEU_ReadRomByte`

### P0-4 修复 ConsoleVideo.cpp 文件损坏（C-12）

- **文件**: `src/drivers/Qt/ConsoleVideo.cpp:111`
- **当前代码**:
```cpp
#include "Qt/ConsoleVideo.h"`n#include "Qt/ConsoleWindow.h"
```
- **修复方案**: 替换为正确换行
```cpp
#include "Qt/ConsoleVideo.h"
#include "Qt/ConsoleWindow.h"
```
- **额外发现**: 反引号-n 不是 `\`n，因此可能是从某种 Windows-1252 编码文件转换错误。建议检查整个 ConsoleVideo.cpp 的编码。
- **验证步骤**: 编译通过；`ConsoleWindow.h` 中的符号在 `ConsoleVideo.cpp` 中可用
- **PR 标题**: `fix(c-12): repair corrupted newline in ConsoleVideo.cpp`

### P0-5 修复 FCEU_WriteRomByte 缺少 `else`（H-01）

- **文件**: `src/fceu.cpp:1475-1481`
- **当前代码**:
```cpp
if (i < 16)
    printf("Sorry, you can't edit the ROM header.\n");
if (i < 16 + PRGsize[0])
    PRGptr[0][i - 16] = value;
```
- **问题**: 当 `i < 16` 时第二个 `if` 仍命中，`i - 16`（uint32）回绕为巨大值 → 越界写
- **修复方案**:
```cpp
if (i < 16) {
    printf("Sorry, you can't edit the ROM header.\n");
    return;
}
if (i < 16 + PRGsize[0])
    PRGptr[0][i - 16] = value;
else if (i < 16 + PRGsize[0] + CHRsize[0])
    CHRptr[0][i - 16 - PRGsize[0]] = value;
```
- **验证步骤**:
  1. 单元测试: `FCEU_WriteRomByte(0, 0xFF)` 不修改 PRG 缓冲区
  2. ASan 检测无越界写
- **PR 标题**: `fix(h-01): add early return in FCEU_WriteRomByte for header writes`

### P0-6 修复 chunk-8 memcpy 无上界检查（H-29）

- **文件**: `src/state.cpp:621-628`
- **当前代码**:
```cpp
case 8:
{
    extern uint8 *XBackBuf;
    if (size == 256 * 256 + 8) {
        memcpy(XBackBuf, data, 256 * 256);
    } else {
        memcpy(XBackBuf, data, size);  // ← 无上界检查
    }
}
```
- **修复方案**: 拒绝非法大小或限制为已知上限
```cpp
case 8:
{
    extern uint8 *XBackBuf;
    constexpr size_t kXBackBufSize = 256 * 256;
    if (size == kXBackBufSize + 8) {
        memcpy(XBackBuf, data, kXBackBufSize);
    } else if (size == kXBackBufSize) {
        // 兼容无 8 字节 trailer 的旧存档
        memcpy(XBackBuf, data, kXBackBufSize);
    } else {
        // 损坏的存档：拒绝
        ret = false;
        if (!warned) {
            // ... warn
        }
    }
}
```
- **PR 标题**: `fix(h-29): add bounds check to chunk-8 memcpy in state.cpp`

### P0-7 修复 CheatRPtrs 数组越界（H-07）

- **文件**: `src/cheat.cpp:71-78`
- **当前代码**:
```cpp
void FCEU_CheatAddRAM(int s, uint32 A, uint8 *p) {
    uint32 AB=A>>10;
    for(x=s-1;x>=0;x--)
        CheatRPtrs[AB+x]=p-A;
}
```
- **问题**: 当 `AB + (s-1) >= 64` 时写越界
- **修复方案**:
```cpp
void FCEU_CheatAddRAM(int s, uint32 A, uint8 *p) {
    uint32 AB = A >> 10;
    if (AB >= 64 || AB + s > 64) {
        // s 太大或 AB 越界；记录并返回
        return;
    }
    for (int32 x = (int32)(s - 1); x >= 0; x--)
        CheatRPtrs[AB + x] = p - A;
}
```
- **PR 标题**: `fix(h-07): add bounds check to FCEU_CheatAddRAM`

### P0-8 修复 SubCheats 数组无上限（H-08）

- **文件**: `src/cheat.cpp:154`
- **当前代码**:
```cpp
SubCheats[numsubcheats].PrevRead = ...;
numsubcheats++;
```
- **修复方案**:
```cpp
if (numsubcheats >= 256) return;  // 已满
SubCheats[numsubcheats].PrevRead = ...;
numsubcheats++;
```
- **PR 标题**: `fix(h-08): add upper bound to SubCheats in cheat.cpp`

---

## 三、P1 — 线程安全与 FFI 安全

### P1-1 修复 SDL_PumpEvents 双线程并发（N-C01，**升级 CRITICAL**）

- **文件**: `src/drivers/Qt/main.cpp:418-427`、`src/drivers/Qt/input/sdl_backend.cpp:156-161`
- **问题**: `SDL_PumpEvents()` 不是线程安全的，但被主线程 0ms QTimer 和模拟器线程同时调用
- **修复方案**: SDL2.0.22+ 提供 `SDL_PumpEvents` 的线程安全替代：将 SDL 事件泵完全移到主线程，模拟器线程通过共享无锁队列获取事件
  - 方案 A: 移除 `sdl_backend.cpp:160` 的 `SDL_PumpEvents()` 调用，仅在主线程 pump
  - 方案 B: 改用 SDL2 `SDL_SetEventFilter` + 主线程专 pump + 跨线程队列
- **建议**: 采用方案 A（最小侵入）：
  ```cpp
  // sdl_backend.cpp 改为只更新 joystick state，不调用 pump
  void SDLBackend::pollAll() {
      // 主线程 0ms QTimer 已在调用 SDL_PumpEvents，此处不需要
      // 仅触发 SDL joystick 状态查询（如有需要可调用 SDL_JoystickUpdate）
  }
  ```
- **验证步骤**:
  1. 重负载测试（拖动窗口、连续按键）观察无事件丢失
  2. Windows + Linux 双平台测试
  3. 启用 TSan 编译验证无数据竞争
- **PR 标题**: `fix(critical): remove duplicate SDL_PumpEvents call from emulator thread`

### P1-2 修复 closeApp 线程退出 use-after-free（N-C02，**升级 CRITICAL**）

- **文件**: `src/drivers/Qt/ConsoleVideo.cpp:322-355`
- **问题**: `quit()` 对无事件循环线程是 no-op；`wait(1000)` 超时后仍执行 `fceuWrapperClose()`
- **修复方案**:
```cpp
void consoleWin_t::closeApp(void)
{
    nes_shm->runEmulator = 0;       // 应改为 std::atomic<char>

    gameTimer->stop();

    closeGamePadConfWindow();

    emulatorThread->requestInterruption();    // 替代无效的 quit()
    if (!emulatorThread->wait(5000)) {        // 5 秒而非 1 秒
        qWarning() << "Emulator thread did not exit cleanly; terminating";
        emulatorThread->terminate();
        emulatorThread->wait();
    }

    // ... 其余保持
}
```
- **配套修复**: 将 `nes_shm->runEmulator` 改为 `std::atomic<char>`
- **PR 标题**: `fix(critical): prevent use-after-free in closeApp on slow frames`

### P1-3 修复 LUA_ENGINE_PTR static mut（C-05）

- **文件**: `src/rust/crates/fceux11-lua/src/lib.rs:21-31`
- **修复方案**: 使用 `AtomicPtr`
```rust
use std::sync::atomic::{AtomicPtr, Ordering};

static LUA_ENGINE_PTR: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

#[inline(always)]
fn get_engine<'a>() -> Option<&'a mut LuaEngine> {
    unsafe { (LUA_ENGINE_PTR.load(Ordering::Acquire) as *mut LuaEngine).as_mut() }
}

// 初始化处
LUA_ENGINE_PTR.store(Box::into_raw(boxed) as *mut c_void, Ordering::Release);
// 销毁处
let ptr = LUA_ENGINE_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
if !ptr.is_null() {
    unsafe { drop(Box::from_raw(ptr as *mut LuaEngine)); }
}
```
- **PR 标题**: `fix(c-05): replace static mut LUA_ENGINE_PTR with AtomicPtr`

### P1-4 修复 Vec::from_raw_parts 容量不匹配（C-06）

- **文件**: `src/rust/crates/fceux11-core/src/state_file.rs`
- **修复方案**: 在 `FceuStateChunkOutput` 中存储 `cap` 字段
```rust
#[repr(C)]
pub struct FceuStateChunkOutput {
    pub chunk_type: u32,
    pub data: *mut u8,
    pub len: usize,
    pub cap: usize,    // 新增
}

// state_file.rs:607 改为
drop(Vec::from_raw_parts(chunk.data, chunk.len, chunk.cap));

// load 函数处
c_chunks.push(FceuStateChunkOutput {
    chunk_type: chunk.chunk_type,
    data: data.as_mut_ptr(),
    len: data.len(),
    cap: data.capacity(),   // 记录实际容量
});
```
- **配套修改**: `fceux11_rust.h` 中对应的 C 结构体同步新增 `cap` 字段
- **PR 标题**: `fix(c-06): store actual Vec capacity in FceuStateChunkOutput`

### P1-5 修复 SFORMAT 反序列化无大小校验（C-07）

- **文件**: `src/rust/crates/fceux11-core/src/sformat.rs:158-172`
- **修复方案**: 添加每条 SFORMAT 条目的最大边界（由编译期生成的常量表提供）
```rust
const MAX_ENTRY_SIZE: usize = 1 << 20;   // 1MB 单条目上限

// ... copy 前检查:
if entry_size > MAX_ENTRY_SIZE {
    return Err(SformatError::EntryTooLarge);
}
```
- **PR 标题**: `fix(c-07): add size bounds check to SFORMAT deserialization`

### P1-6 修复 FceuFilterState 零长度数组（C-04）

- **文件**: `src/rust/fceux11_rust.h:349-351`
- **修复方案 A**: 改为不完整类型
```c
typedef struct FceuFilterState FceuFilterState;   // 前向声明
```
- **修复方案 B（推荐）**: 改为 void* 不透明句柄
```c
typedef void* FceuFilterState;
```
  并在 C++ 端对应修改为
  ```cpp
  using FceuFilterState = void*;
  // 分配/释放通过新 FFI: fceuux11_filter_state_new/free
  ```
- **PR 标题**: `fix(c-04): replace zero-length array with opaque handle in FceuFilterState`

### P1-7 修复 pixbuf 数组无边界检查（C-08）

- **文件**: `src/nes_shm.h`、`src/video.cpp`
- **修复方案**: 在 `CalcVideoDimensions` 中添加边界
```cpp
if (ncol * nrow > 1048576) {
    ncol = 1024;
    nrow = 1024;
    // 或返回 false 阻止启动
}
```
- **PR 标题**: `fix(c-08): add runtime bounds check in CalcVideoDimensions`

### P1-8 修复 AVI 音频环形缓冲区溢出（C-09）

- **文件**: `src/drivers/Qt/AviRecord.cpp:563-583`
- **修复方案**:
```cpp
int aviRecordAddAudioFrame(int32_t *buf, int numSamples)
{
    if (!recordEnable || !recordAudio) return -1;

    for (int i = 0; i < numSamples; i++) {
        int next = (abufHead + 1) % abufSize;
        if (next == abufTail) {
            // 缓冲区满：等待 AVI 线程消费
            msleep(1);
            // 重新检查（可能 abufTail 已推进）
            next = (abufHead + 1) % abufSize;
            if (next == abufTail) {
                // 仍满：丢弃当前样本（接受音频截断而非损坏视频）
                continue;
            }
        }
        rawAudioBuf[abufHead] = buf[i];
        abufHead = next;
    }
    return 0;
}
```
- **PR 标题**: `fix(c-09): add overflow protection to AVI audio ring buffer`

### P1-9 修复 CONOUT$ 句柄双重所有权（C-10）

- **文件**: `src/drivers/Qt/main.cpp:348-371`
- **修复方案**: 使用 `DuplicateHandle`
```cpp
HANDLE hConOut = CreateFileA("CONOUT$", GENERIC_WRITE, FILE_SHARE_WRITE,
                              NULL, OPEN_EXISTING, 0, NULL);
if (hConOut != INVALID_HANDLE_VALUE) {
    HANDLE hStdout, hStderr;
    DuplicateHandle(GetCurrentProcess(), hConOut,
                    GetCurrentProcess(), &hStdout, 0, TRUE, DUPLICATE_SAME_ACCESS);
    DuplicateHandle(GetCurrentProcess(), hConOut,
                    GetCurrentProcess(), &hStderr, 0, TRUE, DUPLICATE_SAME_ACCESS);
    CloseHandle(hConOut);
    
    int conFd = _open_osfhandle((intptr_t)hStdout, _O_TEXT);
    // ... 使用 hStdout
    
    int conFd2 = _open_osfhandle((intptr_t)hStderr, _O_TEXT);
    // ... 使用 hStderr
}
```
- **PR 标题**: `fix(c-10): use DuplicateHandle for stdout/stderr wrappers`

### P1-10 修复 nsf.rs a-0x6000 下溢（N-H01）

- **文件**: `src/rust/crates/fceux11-formats/src/nsf.rs:547`
- **修复方案**:
```rust
if a >= 0x6000 {
    let dst = unsafe { ex_wram.add((a - 0x6000) as usize) };
    // ...
} else {
    // 地址非法，跳过
    return;
}
```
- **PR 标题**: `fix(n-h01): guard against a < 0x6000 underflow in nsf.rs`

### P1-11 修复 emufile.rs seek_set 负数（N-H02）

- **文件**: `src/rust/crates/fceux11-formats/src/emufile.rs:128`
- **修复方案**:
```rust
pub fn seek_set(&mut self, offset: isize) -> bool {
    if offset < 0 { return false; }
    let offset = offset as usize;
    if offset > self.buf.len() {
        self.buf.resize(offset, 0);
    }
    self.pos = offset;
    true
}
```
- **PR 标题**: `fix(n-h02): reject negative offset in seek_set`

### P1-12 修复 nes_shm 跨线程字段原子性（N-H03）

- **文件**: `src/drivers/Qt/nes_shm.h:36-61`
- **修复方案**: 将跨线程字段改为 `std::atomic<T>`
```cpp
#include <atomic>

struct nes_shm_t {
    std::atomic<char> runEmulator{0};
    std::atomic<char> blitUpdated{0};
    std::atomic<int>  pixBufIdx{0};
    // ... video 字段也可改为 atomic 但访问模式更复杂，分步实施
    std::atomic<int> sndHead{0};
    std::atomic<int> sndTail{0};
};
```
- **注意**: 需要重新审视所有 `nes_shm->runEmulator`、`pixBufIdx` 等的读写点，确保使用 `load()`/`store()`
- **PR 标题**: `fix(n-h03): make nes_shm_t cross-thread fields atomic`
- **关联 PR**: 还需修改 N-H04（transfer2LocalBuffer 的 TOCTOU 修复）

### P1-13 Rust FFI null 检查（H-13）

- **文件**: `src/rust/crates/fceux11-utils/src/md5.rs:162`、`guid.rs:30`
- **修复方案**:
```rust
pub unsafe extern "C" fn fceux11_rust_md5_starts(ctx: *mut Md5Context) {
    if ctx.is_null() { return; }
    let ctx = unsafe { &mut *ctx };
    // ...
}
```
- **PR 标题**: `fix(h-13): add null checks in Rust FFI entry points`

### P1-14 Rust Mutex unwrap 在 FFI 上下文（H-14）

- **文件**: `src/rust/crates/.../wave.rs`、`video.rs`
- **修复方案**: 使用 `lock().unwrap_or_else(|e| e.into_inner())` 或返回错误码
```rust
let mut state = match WAVE_STATE.lock() {
    Ok(g) => g,
    Err(poisoned) => poisoned.into_inner(),
};
```
- **PR 标题**: `fix(h-14): handle poisoned mutexes gracefully in FFI`

### P1-15 线程局部缓冲区失效（H-15）

- **文件**: `src/rust/crates/fceux11-formats/src/ines.rs:87-97`
- **修复方案**: 改为分配新缓冲区，由 C++ 端负责释放
```rust
// 改用 Box::leak + 配套 free FFI
```
- **PR 标题**: `fix(h-15): eliminate thread-local pointer TOCTOU in ines.rs`

### P1-16 Profiler 裸指针生命周期（H-16）

- **文件**: `src/rust/crates/fceux11-utils/src/profiler.rs:82-95`
- **修复方案**: 改用 `Weak<...>` 或在 Rust 端维护所有权
- **PR 标题**: `fix(h-16): use weak references for profiler records`

### P1-17 SDL resize 资源重建（H-22）

- **文件**: `src/drivers/Qt/ConsoleViewerSDL.cpp:478-491`
- **修复方案**: 添加防抖（debounce）或节流，仅在 resize 停止后 100ms 重建
- **PR 标题**: `fix(h-22): debounce SDL resize to avoid resource thrashing`

---

## 四、P2 — 代码健壮性

### P2-1 SetReadHandler/WriteHandler start>end 保护（H-02）

- **文件**: `src/fceu.cpp:284-318`
- **修复方案**: 添加 guard
```cpp
if (end < start) return;
```
- **PR 标题**: `fix(h-02): guard against start>end in SetRead/WriteHandler`

### P2-2 FCEUXCart 全局指针未 delete（H-03）

- **文件**: `src/fceu.cpp:1347-1371, 1427`
- **修复方案**:
  1. 将 FCEUXCart 析构函数改为 `virtual`
  2. 将 `NROM` 继承改为 `public`
  3. 在 `FCEU_CloseGame` 中 `delete cart; cart = nullptr;`
- **PR 标题**: `fix(h-03): virtualize FCEUXCart destructor and delete on close`

### P2-3 setup_prg_mapping size==0 保护（H-04）

- **文件**: `src/bus.cpp:333-342`
- **修复方案**:
```cpp
if (size == 0) {
    prg_size_[chip] = 0;
    prg_mask2_[chip] = 0;
    prg_mask4_[chip] = 0;
    // ...
    return;
}
```
- **PR 标题**: `fix(h-04): guard setup_prg_mapping against size==0`

### P2-4 set_mirror_pages 越界（H-05）

- **文件**: `src/ppu_class.cpp:121-127`
- **修复方案**: 参数掩码
```cpp
vnapage_[0] = nt + (a & 1) * 0x400;
vnapage_[1] = nt + (b & 1) * 0x400;
// ...
```
- **PR 标题**: `fix(h-05): mask set_mirror_pages parameters to valid range`

### P2-5 PPU 严格别名（H-06）

- **文件**: `src/ppu_rendering.cpp`
- **修复方案**: 使用 `memcpy` 替代 `*reinterpret_cast<uint32*>`
```cpp
uint32_t tmp;
std::memcpy(&tmp, Plinef + 4, 4);
std::memcpy(Plinef, &tmp, 4);
```
- **PR 标题**: `fix(h-06): replace strict-aliasing reinterpret_cast with memcpy`

### P2-6 registry g_keepalive 大小调整（H-19）

- **文件**: `src/boards/registry.cpp:57`
- **修复方案**: 数组改为 512
```cpp
volatile const MapperEntryRegister* g_keepalive[kRegistrySize] = {};
```
- **PR 标题**: `fix(h-19): grow g_keepalive to match kRegistrySize`

### P2-7 LQ 方波音量互换（H-20）

- **文件**: `src/sound.cpp:654`
- **修复方案**: 与 HQ 路径对齐
- **PR 标题**: `fix(h-20): align LQ square volume mapping with HQ`

### P2-8 AutoFirePatternLength 钳位（H-21）

- **文件**: `src/fceu.cpp:683`
- **修复方案**: 恢复钳位
```cpp
int len = onframes + offframes;
if (len < 2) len = 2;
AutoFirePatternLength = len;
```
- **PR 标题**: `fix(h-21): clamp AutoFirePatternLength to >= 2`

### P2-9 MMC5 缺 Close 处理器（H-17、H-18）

- **文件**: `src/boards/mmc5.cpp:1006-1063`
- **修复方案**: 添加 `info->Close = MMC5_Close;`，引入 RAII 包装
- **PR 标题**: `fix(h-17-18): add MMC5 Close handler and RAII wrap`

### P2-10 模拟线程无限自旋（H-24）

- **文件**: `src/drivers/Qt/fceuWrapper.cpp:1254-1322`
- **修复方案**: 增加渐进退避（16ms → 32ms → 64ms ...）
- **PR 标题**: `fix(h-24): add progressive backoff to emulator lock spin`

### P2-11 chunk-8 memcpy（H-29，已在 P0-6 涵盖）

### P2-12 fwrite 返回值未检查（H-12、H-30）

- **文件**: `src/ines_save.cpp:55-58`
- **修复方案**: 检查返回值并报告
- **PR 标题**: `fix(h-12-30): check fwrite return values`

### P2-13 netplay alloca（H-09）

- **文件**: `src/netplay.cpp:89`
- **修复方案**: 改为 `std::vector<uint8_t>` 或限制大小
- **PR 标题**: `fix(h-09): replace netplay alloca with bounded allocation`

### P2-14 FCEU_malloc NULL 检查（H-10）

- **文件**: `src/ines_load.cpp:176`
- **修复方案**: 添加 if(NULL) return LOADER_HANDLED_ERROR
- **PR 标题**: `fix(h-10): check FCEU_malloc return in ines_load`

### P2-15 SDL_PumpEvents 初始化时序（N-L01）

- **文件**: `src/drivers/Qt/main.cpp:419-427`
- **修复方案**: 在 `SDL_Init` 之后再启动 QTimer
- **PR 标题**: `fix(n-l01): delay SDL pump timer until after SDL_Init`

---

## 五、P3 — 代码质量

### P3-1 修复 `__X6502` 保留标识符
- **文件**: `src/x6502.h`
- **PR 标题**: `chore: rename __X6502 reserved identifier`

### P3-2 清理调试残留
- **文件**: `src/x6502.cpp` (`extern int test`)、`src/boards/fk23c.cpp` (printf)
- **PR 标题**: `chore: remove debug printfs and extern test counter`

### P3-3 处理 const_cast on desc pointer
- **PR 标题**: `chore: add const_cast wrapper or fix const-correctness`

### P3-4 修复 NROM 私有继承（N-L03）
- **文件**: `src/fceu.cpp:1384`
- **修复方案**: `class NROM : public FCEUXCart`
- **PR 标题**: `fix(n-l03): make NROM publicly inherit FCEUXCart`

### P3-5 修复 state_file.rs 死代码（N-L02）
- **文件**: `src/rust/crates/fceux11-core/src/state_file.rs:559`
- **PR 标题**: `chore: remove dead _cap assignment in state_file.rs`

---

## 六、修复执行时间表

| 阶段 | 范围 | 预计 PR 数 | 预计周期 |
|------|------|-----------|---------|
| Phase 1（数据完整性） | P0-1 ~ P0-8 | 8 | 1 周 |
| Phase 2（线程安全） | P1-1 ~ P1-2, P1-12 ~ P1-17 | 8 | 2 周 |
| Phase 3（FFI 安全） | P1-3 ~ P1-11 | 9 | 1.5 周 |
| Phase 4（健壮性） | P2-1 ~ P2-15 | 12 | 2 周 |
| Phase 5（代码质量） | P3-1 ~ P3-5 | 5 | 1 周 |
| **总计** | — | **42 PR** | **7.5 周** |

---

## 七、回归测试策略

每完成一个 P0/P1 PR 必须执行：

1. **编译**: `cmake --build build` Windows/Linux/macOS 三平台全通过
2. **静态分析**:
   - `cppcheck --enable=all` 无新增警告
   - `cargo clippy -- -D warnings` 通过
3. **动态分析**:
   - AddressSanitizer 构建测试套件
   - ThreadSanitizer 构建测试套件（线程相关 PR）
   - UndefinedBehaviorSanitizer 构建
4. **功能测试**:
   - `tests/` 目录下的单元测试全部通过
   - 手动测试：加载 SMB1 → 存档 → 加载 → 精灵渲染一致
5. **存档兼容性**:
   - 用 v1.15 LTS 生成的存档能被新代码加载
   - 用 v1.10 之前生成的存档能被新代码加载（标签兼容）

---

## 八、风险评估

### 8.1 高风险 PR（需要更多测试）

| PR | 风险 | 缓解措施 |
|----|------|---------|
| P1-1 SDL_PumpEvents 移除 | 输入事件丢失 | Windows/Linux 多平台手动测试，保留 0ms QTimer 在主线程 |
| P1-2 closeApp 线程退出 | 关闭变慢或卡住 | 增大超时到 5 秒 + 显式 terminate 兜底 |
| P1-12 nes_shm 原子化 | 跨 PR 修改面广 | 分 PR 改造，每 PR 仅修改一个字段 |
| P2-2 FCEUXCart 析构 | 派生类析构 UB | 改为 public 继承 + virtual 析构，分 PR |

### 8.2 中风险 PR

| PR | 风险 | 缓解措施 |
|----|------|---------|
| P0-2 SFORMAT 修复 | 旧存档兼容性 | 维持标签 "SRx0".."SRx7" 兼容 |
| P1-3 AtomicPtr | ABI 变化 | Rust 内部实现，C ABI 不变 |
| P2-9 MMC5 Close | mapper 内存泄漏修复 | 单独 mapper 测试矩阵 |

### 8.3 低风险 PR（直接合入）

所有 P0 中除 P0-2 外的项；P3 全部。

---

## 九、与原始报告的差异

| 项目 | 原始报告建议 | 本 PLAN 调整 | 理由 |
|------|------------|------------|------|
| H-19 严重度 | CRITICAL/HIGH | **MEDIUM** | deepseek 复审发现已有守卫 |
| H-25 严重度 | HIGH | **LOW** | 仅 UI 一致性问题 |
| N-C01 严重度 | HIGH | **CRITICAL** | SDL 文档明确禁止，实测可能崩 |
| N-C02 严重度 | HIGH | **CRITICAL** | `quit()` no-op + `wait(1000)` 超时后 UAF |
| C-02 修复方案 | 改 8 行下标 | **保留 8 行下标**（兼容性） | 避免存档标签变化 |
| P0-1 修复方案 | "移除 memset 或逐字段初始化" | **移除 memset** | `= default` 构造已保证零初始化 |
| 新增 N-L01 P2-15 | 原始报告未提 | **新增 P2-15** | 实际存在 SDL 初始化时序问题 |

---

## 十、验收清单

### 每个 PR 必须包含：
- [ ] 修改的文件清单
- [ ] 单元测试（如适用）
- [ ] 集成测试通过截图
- [ ] AddressSanitizer / ThreadSanitizer 输出无错
- [ ] 三平台编译通过
- [ ] changelog 条目
- [ ] 引用本 PLAN 的对应章节

### 每个 Phase 完成后：
- [ ] 全量回归测试通过
- [ ] 性能无回退（fps 与基线持平）
- [ ] 存档兼容性测试通过
- [ ] Phase 完成报告

---

*PLAN 制定完毕。共 42 个 PR，分 5 个 Phase，预计 7.5 周完成。优先修复 P0 数据完整性问题，所有 CRITICAL 必须在 v1.15.1 LTS 点版本中修复完毕。*