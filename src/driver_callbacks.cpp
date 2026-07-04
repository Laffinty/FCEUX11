// FCEUX11 v1.11 Bridge — DriverCallbacks implementation
//
// Phase C: FCEUD_* 自由函数迁移为 g_driver()->fn 转发（Batch 1 + 7）。
// Qt 驱动在 fceuWrapperInit 中注册实现后，core 调用 FCEUD_* 即转发到
// Qt 驱动的回调表字段。

#include "driver_callbacks.h"
#include "core_api.h"    // FCEUD_PrintError / FCEUD_Message / FCEUD_SetEmulationSpeed / ...
#include "diag_api.h"    // FCEUD_GetCompilerString
#include "io_api.h"      // FCEUD_UTF8fopen / FCEUD_UTF8_fstream / FCEUD_ScanArchive / ...
#include "net_api.h"     // FCEUD_SendData / FCEUD_RecvData / FCEUD_NetplayText / FCEUD_NetworkClose
#include "file.h"        // ArchiveScanRecord / FCEUFILE (full type needed for return-by-value)

namespace fceu11 {
namespace detail {
    // 定义头文件中声明的进程级单例存储。默认零初始化 → 所有函数指针
    // 字段为 nullptr。register_driver() 通过 POD 赋值覆盖字段。
    DriverCallbacks g_driver_storage{};
} // namespace detail

// POD struct 拷贝赋值。无需 atomic：设计为单线程初始化阶段调用。
// 重复调用覆盖前次注册；调用方负责保证不与并发 driver 调用重叠。
void register_driver(const DriverCallbacks& cb) noexcept {
    detail::g_driver_storage = cb;
}

} // namespace fceu11

// ===========================================================================
// Phase C: Batch 1 — Messages & Lifecycle 回调转发
// ===========================================================================
void FCEUD_PrintError(const char *s) {
    if (auto* fn = fceu11::g_driver().print_error) fn(s);
}
void FCEUD_Message(const char *s) {
    if (auto* fn = fceu11::g_driver().message) fn(s);
}
const char* FCEUD_GetCompilerString() {
    if (auto* fn = fceu11::g_driver().get_compiler_string) return fn();
    return "";
}
int FCEUD_ShowStatusIcon() {
    if (auto* fn = fceu11::g_driver().show_status_icon) return fn();
    return 0;
}
void FCEUD_ToggleStatusIcon() {
    if (auto* fn = fceu11::g_driver().toggle_status_icon) fn();
}
void FCEUD_HideMenuToggle() {
    if (auto* fn = fceu11::g_driver().hide_menu_toggle) fn();
}
bool FCEUD_PauseAfterPlayback() {
    if (auto* fn = fceu11::g_driver().pause_after_playback) return fn();
    return false;
}

// ===========================================================================
// Phase C: Batch 7 — Turbo / Speed 回调转发
// ===========================================================================
void FCEUD_SetEmulationSpeed(int cmd) {
    if (auto* fn = fceu11::g_driver().set_emulation_speed) fn(cmd);
}
void FCEUD_TurboOn() {
    if (auto* fn = fceu11::g_driver().turbo_on) fn();
}
void FCEUD_TurboOff() {
    if (auto* fn = fceu11::g_driver().turbo_off) fn();
}
void FCEUD_TurboToggle() {
    if (auto* fn = fceu11::g_driver().turbo_toggle) fn();
}

// ===========================================================================
// Phase D: Batch 2 — File I/O 回调转发 (Archive)
// ===========================================================================
ArchiveScanRecord FCEUD_ScanArchive(std::string fname) {
    if (auto* fn = fceu11::g_driver().scan_archive) return fn(fname);
    return ArchiveScanRecord{};
}
FCEUFILE* FCEUD_OpenArchive(ArchiveScanRecord& asr, std::string& fname,
                             std::string* innerFilename, int* userCancel) {
    if (auto* fn = fceu11::g_driver().open_archive)
        return fn(asr, fname, innerFilename, userCancel);
    return nullptr;
}
FCEUFILE* FCEUD_OpenArchiveIndex(ArchiveScanRecord& asr, std::string& fname,
                                  int innerIndex, int* userCancel) {
    if (auto* fn = fceu11::g_driver().open_archive_index)
        return fn(asr, fname, innerIndex, userCancel);
    return nullptr;
}

// ===========================================================================
// Phase D: Batch 2 — File I/O 回调转发 (UTF8 fopen/fstream)
// ===========================================================================
FILE* FCEUD_UTF8fopen(const char *fn, const char *mode) {
    if (auto* cb = fceu11::g_driver().utf8_fopen) return cb(fn, mode);
    return nullptr;
}
EMUFILE_FILE* FCEUD_UTF8_fstream(const char *fn, const char *m) {
    if (auto* cb = fceu11::g_driver().utf8_fstream) return cb(fn, m);
    return nullptr;
}

// ===========================================================================
// Phase D: Batch 8 — Driver-command file dialogs 回调转发
// ===========================================================================
void FCEUD_AviRecordTo() {
    if (auto* fn = fceu11::g_driver().avi_record_to) fn();
}
void FCEUD_AviStop() {
    if (auto* fn = fceu11::g_driver().avi_stop) fn();
}
void FCEUD_SaveStateAs() {
    if (auto* fn = fceu11::g_driver().save_state_as) fn();
}
void FCEUD_LoadStateFrom() {
    if (auto* fn = fceu11::g_driver().load_state_from) fn();
}
void FCEUD_MovieRecordTo() {
    if (auto* fn = fceu11::g_driver().movie_record_to) fn();
}
void FCEUD_MovieReplayFrom() {
    if (auto* fn = fceu11::g_driver().movie_replay_from) fn();
}

// ===========================================================================
// Phase E: Batch 3 — Video / Palette 回调转发
// ===========================================================================
void FCEUD_SetPalette(uint8 index, uint8 r, uint8 g, uint8 b) {
    if (auto* fn = fceu11::g_driver().set_palette) fn(index, r, g, b);
}
void FCEUD_GetPalette(uint8 i, uint8* r, uint8* g, uint8* b) {
    if (auto* fn = fceu11::g_driver().get_palette) fn(i, r, g, b);
}
void FCEUD_VideoChanged(void) {
    if (auto* fn = fceu11::g_driver().video_changed) fn();
}
bool FCEUD_ShouldDrawInputAids(void) {
    if (auto* fn = fceu11::g_driver().should_draw_input_aids) return fn();
    return true;
}
uint64 FCEUD_GetTime(void) {
    if (auto* fn = fceu11::g_driver().get_time) return fn();
    return 0;
}
uint64 FCEUD_GetTimeFreq(void) {
    if (auto* fn = fceu11::g_driver().get_time_freq) return fn();
    return 1000;
}

// ===========================================================================
// Phase E: Batch 4 — Sound 回调转发
// ===========================================================================
void FCEUD_SoundToggle(void) {
    if (auto* fn = fceu11::g_driver().sound_toggle) fn();
}
void FCEUD_SoundVolumeAdjust(int n) {
    if (auto* fn = fceu11::g_driver().sound_volume_adjust) fn(n);
}

// ===========================================================================
// Phase E: Batch 5 — Input 回调转发
// ===========================================================================
void FCEUD_SetInput(bool fourscore, bool microphone, ESI port0, ESI port1, ESIFC fcexp) {
    if (auto* fn = fceu11::g_driver().set_input)
        fn(fourscore, microphone, port0, port1, fcexp);
}

// ===========================================================================
// Phase E: Batch 6 — Debug 回调转发
// ===========================================================================
void FCEUD_DebugBreakpoint(int bp_num) {
    if (auto* fn = fceu11::g_driver().debug_breakpoint) fn(bp_num);
}
void FCEUD_TraceInstruction(uint8* opcode, int size) {
    if (auto* fn = fceu11::g_driver().trace_instruction) fn(opcode, size);
}
void FCEUD_FlushTrace(void) {
    if (auto* fn = fceu11::g_driver().flush_trace) fn();
}
void FCEUD_UpdateNTView(int scanline, bool drawall) {
    if (auto* fn = fceu11::g_driver().update_nt_view) fn(scanline, drawall);
}
void FCEUD_UpdatePPUView(int scanline, int drawall) {
    if (auto* fn = fceu11::g_driver().update_ppu_view) fn(scanline, drawall);
}

// ===========================================================================
// Phase F: Batch 9 — Netplay 回调转发
// ===========================================================================
int FCEUD_SendData(void *data, uint32 len) {
    if (auto* fn = fceu11::g_driver().send_data) return fn(data, len);
    return 0;
}
int FCEUD_RecvData(void *data, uint32 len) {
    if (auto* fn = fceu11::g_driver().recv_data) return fn(data, len);
    return 0;
}
void FCEUD_NetplayText(uint8 *text) {
    if (auto* fn = fceu11::g_driver().netplay_text) fn(text);
}
void FCEUD_NetworkClose(void) {
    if (auto* fn = fceu11::g_driver().network_close) fn();
}
