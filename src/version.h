/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2001 Aaron Oneal
 *  Copyright (C) 2002 Xodnizel
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

#define PUBLIC_RELEASE		// uncomment this when making a public release, but comment back before committing

#ifndef __FCEU_VERSION
#define __FCEU_VERSION

//todo - everyone will want to support this eventually, i suppose
#ifdef SVN_REV
#define SCM_REV_STR SCM_REV
#else
#define SCM_REV_STR ""
#endif

#define FCEU_NAME "FCEUX11"

#define FCEU_FEATURE_STRING ""

#ifdef _DEBUG
#define FCEU_SUBVERSION_STRING " debug"
#elif defined(PUBLIC_RELEASE)
#define FCEU_SUBVERSION_STRING ""
#else
#define FCEU_SUBVERSION_STRING "-interim git" SCM_REV_STR
#endif

#if defined(_MSC_VER)
#define FCEU_COMPILER ""
#define FCEU_COMPILER_DETAIL " msvc " _Py_STRINGIZE(_MSC_VER)
#define _Py_STRINGIZE(X) _Py_STRINGIZE1((X))
#define _Py_STRINGIZE1(X) _Py_STRINGIZE2 ## X
#define _Py_STRINGIZE2(X) #X
//re: http://72.14.203.104/search?q=cache:HG-okth5NGkJ:mail.python.org/pipermail/python-checkins/2002-November/030704.html+_msc_ver+compiler+version+string&hl=en&gl=us&ct=clnk&cd=5
#else
// TODO: make for others compilers
#define FCEU_COMPILER ""
#define FCEU_COMPILER_DETAIL ""
#endif

// v2.0.2 — 以 DEBUG（调试能力/排查效率）与性能优化为主的一次迭代：
// A 线（调试门控/批量步进/逐字节对拍/HQ 渲染器 SIMD 化）、B 线（无头 trace、
// profiler 开关、lua memory 导出）、C 线（GBA 帧导出与 savestate 编码加速），
// 外加 D1 专线的中断模型基础设施与两轮 S2.5。NES 侧基线 F11QA 矩阵
// 122 项 / grade B 不变。成果逐条见 CHANGELOG 2.0.2 与计划 §十 r1–r26。
inline constexpr int FCEU_VERSION_MAJOR = 2;
inline constexpr int FCEU_VERSION_MINOR = 0;
inline constexpr int FCEU_VERSION_PATCH = 2;
inline constexpr int FCEU_VERSION_TWEAK = 0;

inline constexpr int FCEU_VERSION_NUMERIC = (FCEU_VERSION_MAJOR * 10000) + (FCEU_VERSION_MINOR * 100) + FCEU_VERSION_PATCH;
inline constexpr int FCEU_VERSION_MAJOR_DECODE(int x) { return x / 10000; }
inline constexpr int FCEU_VERSION_MINOR_DECODE(int x) { return (x / 100) % 100; }
inline constexpr int FCEU_VERSION_PATCH_DECODE(int x) { return x % 100; }

// v2.0.2 release — no hotfix tag.
#define FCEU_HOTFIX_TAG ""

#define FCEU_VERSION_STRING "2.0.2 " FCEU_SUBVERSION_STRING FCEU_FEATURE_STRING FCEU_COMPILER
#define FCEU_DISPLAY_VERSION "v2.0.2"
#define FCEU_NAME_AND_VERSION FCEU_NAME " " FCEU_DISPLAY_VERSION

// FCEUX11 Contributors — Derivative work based on FCEUX
// Copyright (C) 2026 FCEUX11 Contributors
// This file is part of FCEUX11, a derivative work of FCEUX.

#endif
