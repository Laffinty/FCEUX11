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

// v2.0.4（2026-11-15）— 残留精度与调试待办（构建计划 r16 收口：**20/22 项落地**，
// V12/V5 业主裁决 defer 转长尾；V8 A7 实验已完成——结论「不值得，收益上限 ≈ 0」封口）：
// 批 4 崩溃与内存安全全清（V9 mapper 218 越界 /
// V10 CHR-RAM 垫层与掩码钳制 / V11 VRC7 读档 / V13 AVI / V14 TraceLogger+CDL /
// V15 JSreturn / V24 GUID）、批 5 防御性（V19 GUID 置零 / V22 鼠标 / V23 CDL FDS-only）、
// 批 6 纵深（V16/V20）、批 1 补完（V1 Lua abort / V6 麦克风协议）、批 3 首项（V7 profiler 菜单）。
// ⚠️ **行为变更四处**（随包 Known limitations 汇总）：V23 非 FDS 卡带 .cdl 输出改变、
// V6 按住麦克风每读 bit2=1、V10 <8K 卡画面会变（movie 会漂移）、V22 input.get() 恒含
// 数值 click 字段。详见 CHANGELOG 2.0.4 与计划 §七 r1–r14。GBA 仍是 Alpha，不阻塞发版。
//
// ⚠️ **整版数值随惯例递增**（v2.0.3 → v2.0.4，即公式值 20003 → 20004；旧注释里写的
// "20202/20203" 是沿袭的位数笔误，以第 91 行公式为准）：该数值进 FCSX 存档头
// （`state.cpp:397`）与 FM2 录像头（`movie_record.cpp:591`），但两侧读取器都只把它当
// **信息性字段**回读（movie.rs `parse().unwrap_or(0)`、state_file.rs `load_v1` 原样返回、
// 均不与当前版本比对）⇒ v2.0.2/2.0.3 产出的存档/录像在本版**仍可正常读取**。
// 只有 hotfix 才冻结数值（见 hotfix1 注）。
//
// v2.0.3（2026-11-12）— GBA 卡带兼容性线（三段式收口）+ FC 侧 S4/S5：
// BIOS SWI 0x16–0x18/0x19/0x1F/0x25/0x27 真实现 + 未实现 SWI 兜底（返回 + 记录）、
// OAM 禁止值容错、EEPROM 动态容量、mode 5 页面翻转、apply_state 读档修复、
// issue #12 两缺陷 + 修正表/hash 深层修复。详见 CHANGELOG 2.0.3 与
// v2.0.3 计划 §九 r1–r19。GBA 仍是 Alpha，不阻塞本体发版。
//
// ⚠️ **整版数值随惯例递增**（20202 → 20203，与 v2.0.0/2.0.1/2.0.2 同口径）：
// 该数值进 FCSX 存档头（`state.cpp:397`）与 FM2 录像头（`movie_record.cpp:591`），
// 但两侧读取器都只把它当**信息性字段**回读（movie.rs `parse().unwrap_or(0)`、
// state_file.rs `load_v1` 原样返回、均不与当前版本比对）⇒ v2.0.2 产出的
// 存档/录像在本版**仍可正常读取**。只有 hotfix 才冻结数值（见 hotfix1 注）。
//
// v2.0.2 — 以 DEBUG（调试能力/排查效率）与性能优化为主的一次迭代：
// A 线（调试门控/批量步进/逐字节对拍/HQ 渲染器 SIMD 化）、B 线（无头 trace、
// profiler 开关、lua memory 导出）、C 线（GBA 帧导出与 savestate 编码加速），
// 外加 D1 专线的中断模型基础设施与两轮 S2.5。NES 侧基线 F11QA 矩阵
// 122 项 / grade B 不变。成果逐条见 CHANGELOG 2.0.2 与计划 §十 r1–r26。
//
// v2.0.2_hotfix1（2026-11-10）— v2.0.2 的 hotfix 发布：
// 内容 = v2.0.2 的全部成果 + v2.0.3 线的 **S4 · FC 侧修复批次**
// （GitHub issue #12：iNES loader 硬拒非 2 幂 PRG、mapper 191/199 未注册灰屏）
// + GBA 侧 S1-a/S1-b/S2-a 的在途成果（GBA 仍是 Alpha，长期不阻塞本体发版）。
//
// ⚠️ **数值口径不动**：`FCEU_VERSION_NUMERIC` 仍为 20202 —— 它进 FCSX 电影模式
// 存档头（`state.cpp:397`）与录像头（`movie_record.cpp:591`），改了会让
// v2.0.2 产出的存档/录像与本版**互读失败**。hotfix 编号只由 `FCEU_HOTFIX_TAG` 承载。
// （v2.0.3 起整版恢复递增口径，并经读取侧核实：版本字段信息性、从不拒绝比对。）
// v2.0.4_hotfix1（2026-11-16）— v2.0.4 的 hotfix 发布：
// 内容 = v2.0.4 的全部成果 + **BUG-V205-01 修复**（loader 误拒「CHR 数量非 2 幂」
// 的合法 iNES 1.0 ROM ⇒ 打开时弹 "Unknown ROM file format." 错误框）。
// 根因：本仓 Rust loader 的 CHR 边界检查用了**舍入后**的 CHR 数量，而上游
// FCEUX 2.6.6 / 2.8.0 不做该检查（memset 舍入缓冲为 0xFF 后只读文件实有的字节）
// ⇒ 上游能装、本仓不能。修复 = 边界检查与拷贝改用**头里的原始 CHR 数量**，
// 分配/映射/哈希仍用舍入值（上游同款；尾部按上游语义补 0xFF）。
// v2.0.5 线已并行开工（A1 恢复 75 个未注册 mapper 的板级 Power），但 hotfix
// 只取 BUG-V205-01 这一项。
//
// ⚠️ **数值口径不动**：`FCEU_VERSION_NUMERIC` 仍为 20004 —— 它进 FCSX 电影模式
// 存档头（`state.cpp:397`）与录像头（`movie_record.cpp:591`），hotfix 编号只由
// `FCEU_HOTFIX_TAG` 承载（与 v2.0.2_hotfix1 同一口径）。
// v2.0.5（2026-11-18）— **整版发布**（不再是 hotfix）：
// 内容 = 上游 mapper 能力对齐收尾（A1 75 个 mapper 注册 / A2 18 号板级 Power 接线 /
// A3 矫正表补 22 条 / A4 mapper 45 写窗口 / A6 mapper 210 镜像语义 /
// A7 MMC5 上电默认值 + bandai x24c02 + mapper 72 / A8-471 Impact Soft IM1）
// + 线 B 长尾四条（LT-08/09/10/16）+ 线 C 加载卡顿与中文名路径 + CI 诊断增强。
// ⚠️ **A5（mapper 4 的 NES 2.0 PRG-RAM）已回退**：上游 2.8.0 把头声明的 PRG-RAM
// 大小原样转成 MMC3 的 WRAM 尺寸，会让声明 0 的 mapper 4 卡带（ tepples
// mmc3bigchrram 与本仓 6 个 fixture 全是这类）失去 $6000-$7FFF ⇒ 本仓 mapper 4
// 的 WRAM 恒 8 KiB（完整根因见 docs/history/plans/FCEUX11-v2.0.5_构建计划.md §9.25）。
//
// ⚠️ **整版数值随惯例递增**（v2.0.4 → v2.0.5，即公式值 20004 → 20005）；该字段进
// FCSX 电影模式存档头（state.cpp:397）与录像头（movie_record.cpp:591），但读取侧
// 从不拒绝比对（信息性字段）⇒ 旧存档/录像仍可读。
// v2.0.6（2026-11-20）— **整版发布**（稳定性维护版）：
// 内容 = v2.0.5「音视频异常缓慢」稳定性审计线的三阶段修复（A1+A2 音频探测关闭
// 路径把探测时长搬回 GUI 线程 / 阶段 2 设备停排空时每帧自旋约 1 秒 /
// A3 SDL_AUDIODRIVER 回退永久污染进程环境 / GBA 会话结束时限速基准不复位）
// + 遗留问题盘点批 1/2/3（MMC3 家族 7 板板级 Reset 接线 / f11qa direct 门禁
// 空洞护栏 / mapper198 锁改写 / 生成器机械字段对齐 / STA 垫片扩展 6 处 /
// PGO 立项材料）+ A5 实测裁决登记不修。
//
// ⚠️ **整版数值随惯例递增**（v2.0.5 → v2.0.6，即公式值 20005 → 20006）；该字段进
// FCSX 电影模式存档头（state.cpp:397）与录像头（movie_record.cpp:591），但读取侧
// 从不拒绝比对（信息性字段）⇒ 旧存档/录像仍可读。
inline constexpr int FCEU_VERSION_MAJOR = 2;
inline constexpr int FCEU_VERSION_MINOR = 0;
inline constexpr int FCEU_VERSION_PATCH = 6;
inline constexpr int FCEU_VERSION_TWEAK = 0;	// hotfix 计数；**不参与** NUMERIC

inline constexpr int FCEU_VERSION_NUMERIC = (FCEU_VERSION_MAJOR * 10000) + (FCEU_VERSION_MINOR * 100) + FCEU_VERSION_PATCH;
inline constexpr int FCEU_VERSION_MAJOR_DECODE(int x) { return x / 10000; }
inline constexpr int FCEU_VERSION_MINOR_DECODE(int x) { return (x / 100) % 100; }
inline constexpr int FCEU_VERSION_PATCH_DECODE(int x) { return x % 100; }

// v2.0.6 是整版发布 ⇒ 无 hotfix 后缀（hotfix 编号口径不变：只由
// FCEU_HOTFIX_TAG 承载，不进 FCEU_VERSION_NUMERIC）。
#define FCEU_HOTFIX_TAG ""

#define FCEU_VERSION_STRING "2.0.6" FCEU_HOTFIX_TAG " " FCEU_SUBVERSION_STRING FCEU_FEATURE_STRING FCEU_COMPILER
#define FCEU_DISPLAY_VERSION "v2.0.6" FCEU_HOTFIX_TAG
#define FCEU_NAME_AND_VERSION FCEU_NAME " " FCEU_DISPLAY_VERSION

// FCEUX11 Contributors — Derivative work based on FCEUX
// Copyright (C) 2026 FCEUX11 Contributors
// This file is part of FCEUX11, a derivative work of FCEUX.

#endif
