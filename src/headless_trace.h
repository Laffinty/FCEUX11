// headless_trace.h
//
// B1 (v2.0.2 P1 续): TraceLogger 无头入口。
//
// 读环境变量 FCEUX11_TRACE：未设或空 → 什么都不做（零开销，零产物）；
// "1" → 采集到默认文件 fceux11-trace.log；其他非空值 → 该值即日志路径。
// 仅当 g_driver().trace_instruction == nullptr（无 Qt TraceLogger）时才安装
// 无头采集器，并置 g_driver().trace_logging = 1 —— 与 A1 的 DebugCycle 快速
// 门同一状态位，这是必须的接线，否则采集器永远不会被调到。
//
// 调用点唯一：fceu11::Initialize()（fceu.cpp）入口。

#pragma once

namespace fceu11 {

void headless_trace_init(void);
void headless_trace_flush(void);

} // namespace fceu11
