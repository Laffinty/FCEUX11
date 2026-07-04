// FCEUX11 v1.11 Bridge — DriverCallbacks scaffolding
//
// Core→Driver 回调表 (v1.11_bridge_build_plan.md §2.1).
// 所有 FCEUD_* 函数指针式回调的现代化归宿：POD 结构体 + 进程级单例 +
// 显式 register_driver() 注册。Phase B 仅引入脚手架，不切换调用点；
// Phase C~F 逐步将 FCEUD_* 自由函数改为转发到 g_driver()->fn(...)。
//
// 设计约束（plan §2.1）：
//   1. 字段全部为函数指针（非 std::function），避免热路径堆分配与类型擦除。
//   2. nullptr 表示"未注册"，core 调用前必须判空（debug 断言）。
//   3. DriverCallbacks 是 POD，可 std::memset 清零；不引入虚函数表。
//   4. g_driver() 访问器 __forceinline，使 MSVC 在 LTO 前即可将 g_driver()->fn()
//      优化为直接 call（plan §6.4 验证手段）。
//
// 字段全集：43 个 = 40 live (plan §2.1) + 3 audit §5 增补
// （get_keyboard_state / taseditor_disable_run_function / get_thread_name）。

#pragma once

#include <cstdio>      // FILE (FCEUD_UTF8fopen)
#include <cstdint>     // uint8_t / uint32_t / uint64_t
#include <string>      // std::string (FCEUD_ScanArchive / OpenArchive 系列)

#include "types.h"     // uint8 / uint64 (项目内别名)
#include "git.h"       // ESI / ESIFC (FCEUD_SetInput)

// 前置声明 —— 保持本头文件轻量，避免传递 include 拖入 file.h / emufile.h
class  EMUFILE_FILE;            // src/emufile.h:279
struct FCEUFILE;                // src/file.h:14
struct ArchiveScanRecord;       // src/file.h:103

namespace fceu11 {

// ===========================================================================
// DriverCallbacks — Core→Driver 回调表（POD，43 函数指针字段）
// ===========================================================================
struct DriverCallbacks {
    // ---- Batch 1: Messages & lifecycle (Phase C) ----
    void  (*print_error)(const char* s) = nullptr;                 // FCEUD_PrintError
    void  (*message)(const char* s) = nullptr;                     // FCEUD_Message
    const char* (*get_compiler_string)() = nullptr;               // FCEUD_GetCompilerString
    int   (*show_status_icon)() = nullptr;                         // FCEUD_ShowStatusIcon
    void  (*toggle_status_icon)() = nullptr;                       // FCEUD_ToggleStatusIcon
    void  (*hide_menu_toggle)() = nullptr;                         // FCEUD_HideMenuToggle
    bool  (*pause_after_playback)() = nullptr;                     // FCEUD_PauseAfterPlayback

    // ---- Batch 7: Turbo / speed (Phase C) ----
    void  (*set_emulation_speed)(int cmd) = nullptr;               // FCEUD_SetEmulationSpeed
    void  (*turbo_on)() = nullptr;                                 // FCEUD_TurboOn
    void  (*turbo_off)() = nullptr;                                // FCEUD_TurboOff
    void  (*turbo_toggle)() = nullptr;                             // FCEUD_TurboToggle

    // ---- Batch 2: File I/O (Phase D) ----
    FILE* (*utf8_fopen)(const char* fn, const char* mode) = nullptr;            // FCEUD_UTF8fopen
    EMUFILE_FILE* (*utf8_fstream)(const char* n, const char* m) = nullptr;      // FCEUD_UTF8_fstream
    ArchiveScanRecord (*scan_archive)(std::string fname) = nullptr;             // FCEUD_ScanArchive
    FCEUFILE* (*open_archive)(ArchiveScanRecord& asr, std::string& fname,
                              std::string* innerFilename, int* userCancel) = nullptr;     // FCEUD_OpenArchive
    FCEUFILE* (*open_archive_index)(ArchiveScanRecord& asr, std::string& fname,
                                    int innerIndex, int* userCancel) = nullptr;           // FCEUD_OpenArchiveIndex

    // ---- Batch 8: Driver-command file dialogs (Phase D) ----
    void  (*avi_record_to)() = nullptr;                            // FCEUD_AviRecordTo
    void  (*avi_stop)() = nullptr;                                 // FCEUD_AviStop
    void  (*save_state_as)() = nullptr;                            // FCEUD_SaveStateAs
    void  (*load_state_from)() = nullptr;                          // FCEUD_LoadStateFrom
    void  (*movie_record_to)() = nullptr;                          // FCEUD_MovieRecordTo
    void  (*movie_replay_from)() = nullptr;                        // FCEUD_MovieReplayFrom

    // ---- Batch 3: Video / palette (Phase E) ----
    void  (*set_palette)(uint8 index, uint8 r, uint8 g, uint8 b) = nullptr;    // FCEUD_SetPalette
    void  (*get_palette)(uint8 i, uint8* r, uint8* g, uint8* b) = nullptr;     // FCEUD_GetPalette
    void  (*video_changed)() = nullptr;                            // FCEUD_VideoChanged
    bool  (*should_draw_input_aids)() = nullptr;                   // FCEUD_ShouldDrawInputAids
    uint64 (*get_time)() = nullptr;                                // FCEUD_GetTime
    uint64 (*get_time_freq)() = nullptr;                           // FCEUD_GetTimeFreq

    // ---- Batch 4: Sound (Phase E) ----
    void  (*sound_toggle)() = nullptr;                             // FCEUD_SoundToggle
    void  (*sound_volume_adjust)(int n) = nullptr;                 // FCEUD_SoundVolumeAdjust

    // ---- Batch 5: Input (Phase E) ----
    void  (*set_input)(bool fourscore, bool microphone,
                       ESI port0, ESI port1, ESIFC fcexp) = nullptr;            // FCEUD_SetInput

    // ---- Batch 6: Debug (Phase E) ----
    void  (*debug_breakpoint)(int bp_num) = nullptr;               // FCEUD_DebugBreakpoint
    void  (*trace_instruction)(uint8* opcode, int size) = nullptr; // FCEUD_TraceInstruction
    void  (*flush_trace)() = nullptr;                              // FCEUD_FlushTrace
    void  (*update_nt_view)(int scanline, bool drawall) = nullptr; // FCEUD_UpdateNTView
    void  (*update_ppu_view)(int scanline, int drawall) = nullptr; // FCEUD_UpdatePPUView

    // ---- Batch 9: Netplay (Phase F) ----
    int   (*send_data)(void* data, uint32 len) = nullptr;          // FCEUD_SendData
    int   (*recv_data)(void* data, uint32 len) = nullptr;          // FCEUD_RecvData
    void  (*netplay_text)(uint8* text) = nullptr;                  // FCEUD_NetplayText
    void  (*network_close)() = nullptr;                            // FCEUD_NetworkClose

    // ---- UI refresh / dialogs (Phase C/D, plan §1.2 类别 B/C) ----
    void  (*set_main_window_text)(const char* s) = nullptr;        // SetMainWindowText 抽象
    void  (*update_ram_search)() = nullptr;                        // Update_RAM_Search 抽象
    void  (*update_cheat_list)() = nullptr;                        // UpdateCheatList 抽象
    int   (*message_box)(const char* title, const char* msg, int type) = nullptr; // MessageBox 抽象

    // ---- Emu-command handler (Phase F, input.cpp 27 块) ----
    void  (*emu_command)(int cmd) = nullptr;                       // handleEmuCmdByTaseditor 等

    // ---- audit §5 增补 (Phase E, lua-engine.cpp 类型统一 / profiler QThread) ----
    void  (*get_keyboard_state)(void* out) = nullptr;              // fceux11_lua_GetKeyboardState
    void  (*taseditor_disable_run_function)() = nullptr;           // 取代 core 持有 taseditor_lua
    const char* (*get_thread_name)() = nullptr;                    // QThread::currentThread()->objectName()
};

// POD 不变量（plan §2.4 验收）。static_assert 在编译期触发：
//   - is_trivially_copyable：保证 POD 语义，register_driver() 可 struct 拷贝
//   - sizeof > 0：防止空 struct 退化为 1 字节导致后续 offsetof 错位
static_assert(std::is_trivially_copyable_v<DriverCallbacks>,
              "DriverCallbacks must be trivially copyable (POD)");
static_assert(sizeof(DriverCallbacks) > 0,
              "DriverCallbacks must not degenerate to empty struct");

// ===========================================================================
// g_driver() 单例访问器
// ===========================================================================
namespace detail {
    // 进程级单例存储。在 driver_callbacks.cpp 定义。
    // 默认零初始化（每个函数指针 = nullptr）。register_driver() 通过
    // POD 赋值覆盖字段。g_driver() 永远返回此同一引用，保证 __forceinline
    // 路径上地址加载在编译期已知 → MSVC 可优化为直接 call。
    extern DriverCallbacks g_driver_storage;
}

// 永远返回同一引用。未注册时字段全 nullptr（安全 no-op 默认值）。
// __forceinline 确保 g_driver()->fn() 在调用点折叠为单一 load + indirect call。
__forceinline DriverCallbacks& g_driver() noexcept {
    return detail::g_driver_storage;
}

// ===========================================================================
// register_driver() —— Qt 驱动在 fceuWrapperInit 中调用一次以安装实现
// ===========================================================================
//
// 语义：POD struct 拷贝赋值。无堆分配、无虚调用、无线程安全保证（设计为
// 单线程初始化阶段调用）。重复调用覆盖前次注册（最新调用生效）。
//
// Phase B 状态下 Qt 不调用此函数（所有 FCEUD_* 自由函数仍走原 Qt 实现）；
// Phase C 起 Qt 在 fceuWrapperInit 末尾调用一次，install Batch 1+7 等回调。
void register_driver(const DriverCallbacks& cb) noexcept;

} // namespace fceu11
