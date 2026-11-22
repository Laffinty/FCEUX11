/* FCE Ultra - NES/Famicom Emulator
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
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
