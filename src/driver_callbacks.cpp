// FCEUX11 v1.11 Bridge — DriverCallbacks implementation
//
// Phase B scaffolding only. Qt driver is NOT yet calling register_driver();
// all FCEUD_* free functions continue to use their existing implementations
// in src/drivers/Qt/fceuWrapper.cpp. This file is purely additive.

#include "driver_callbacks.h"

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
