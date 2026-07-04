// FCEUX11 v1.11 Bridge — DriverCallbacks implementation
//
// Phase C: FCEUD_* 自由函数迁移为 g_driver()->fn 转发（Batch 1 + 7）。
// Qt 驱动在 fceuWrapperInit 中注册实现后，core 调用 FCEUD_* 即转发到
// Qt 驱动的回调表字段。

#include "driver_callbacks.h"
#include "core_api.h"    // FCEUD_PrintError / FCEUD_Message / FCEUD_SetEmulationSpeed / ...
#include "diag_api.h"    // FCEUD_GetCompilerString

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
