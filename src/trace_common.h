// trace_common.h
//
// B1 (v2.0.2 P1 续): TraceLogger 采集层与记录格式的 Qt-free 共享层。
//
// 内容为 src/drivers/Qt/TraceLogger.{h,cpp} 的**纯剪切迁移**（traceRecord_t、
// appendAsmText、convToText、LOG_* 选项位、logging_options），零语义改动；
// 外加两处结构性抽出（不改变任何产出字节）：
//   1. 采集体 fce11_trace_capture —— 原 fceWrapper_TraceInstruction
//      (TraceLogger.cpp:1373-1547) 的主体。原函数把 pushToLogBuffer 硬编码在
//      采集体末尾；本层去掉它，下发由调用方注入的 sink 完成（Qt 侧 sink =
//      pushToLogBuffer，无头侧 sink = fwrite 批量落盘）。
//   2. 反汇编钩子 —— DisassembleWithDebug 定义在 Qt 驱动层
//      (drivers/Qt/SymbolicDebug.cpp)，fceux11_core 不能链接它（分层纪律 8）。
//      采集体经 fce11_trace_disasm_t 钩子取反汇编文本：Qt 侧启动时注入
//      DisassembleWithDebug（输出与迁移前逐字节一致）；无头/未注入时回落到
//      core 自带的 Disassemble（asm.cpp，Rust 后端）。
//
// 本头文件不得包含任何 Qt 头。

#pragma once

#include <stdint.h>

#include "types.h"

// ---------------------------------------------------------------------------
// 反汇编旗标（与 Qt SymbolicDebug.h 同名宏同值。这里用 #ifndef 包住，两个
// 头被同一 TU 包含时靠「相同重定义」规则共存，谁先谁后都无告警。）
// ---------------------------------------------------------------------------
#ifndef ASM_DEBUG_SYMS
#define ASM_DEBUG_SYMS 0x0001
#endif
#ifndef ASM_DEBUG_REGS
#define ASM_DEBUG_REGS 0x0002
#endif
#ifndef ASM_DEBUG_TRACES
#define ASM_DEBUG_TRACES 0x0010
#endif

// ---------------------------------------------------------------------------
// LOG_* 选项位 —— 纯剪切自 TraceLogger.cpp:73-85
// ---------------------------------------------------------------------------
enum traceLogOptionBits
{
	LOG_REGISTERS         = 0x00000001,
	LOG_PROCESSOR_STATUS  = 0x00000002,
	LOG_NEW_INSTRUCTIONS  = 0x00000004,
	LOG_NEW_DATA          = 0x00000008,
	LOG_TO_THE_LEFT       = 0x00000010,
	LOG_FRAMES_COUNT      = 0x00000020,
	LOG_MESSAGES          = 0x00000040,
	LOG_BREAKPOINTS       = 0x00000080,
	LOG_SYMBOLIC          = 0x00000100,
	LOG_CODE_TABBING      = 0x00000200,
	LOG_CYCLES_COUNT      = 0x00000400,
	LOG_INSTRUCTIONS_COUNT = 0x00000800,
	LOG_BANK_NUMBER       = 0x00001000
};

// 纯剪切自 TraceLogger.cpp:102（TraceLogger.cpp 保留同名同值宏，相同重定义合法）。
#ifndef LOG_TABS_MASK
#define LOG_TABS_MASK 31
#endif

// ---------------------------------------------------------------------------
// traceRecord_t —— 纯剪切自 TraceLogger.h:27-62
// ---------------------------------------------------------------------------
struct traceRecord_t
{
	struct
	{
		uint16_t PC;
		uint8_t A;
		uint8_t X;
		uint8_t Y;
		uint8_t S;
		uint8_t P;
	} cpu;

	uint8_t opCode[3];
	uint8_t opSize;
	uint8_t asmTxtSize;
	uint8_t preWriteVal;
	char asmTxt[64];

	uint64_t frameCount;
	uint64_t cycleCount;
	uint64_t instrCount;
	uint64_t flags;

	int32_t callAddr;
	int32_t romAddr;
	int32_t bank;
	int32_t skippedLines;

	int32_t writeAddr;

	traceRecord_t(void);

	int appendAsmText(const char *txt);

	int convToText(char *line, int *len = 0);
};

// ---------------------------------------------------------------------------
// logging_options —— 定义迁到 trace_common.cpp（原 TraceLogger.cpp:109 的
// static 文件级变量，本仓库内唯一使用方即 TraceLogger.cpp 与本层采集体，
// 改为共享全局不改变任何行为）。
// ---------------------------------------------------------------------------
extern int logging_options;

// ---------------------------------------------------------------------------
// 采集体
// ---------------------------------------------------------------------------

// sink：收到一条已采集完成的记录。Qt 注入 pushToLogBuffer 包装，无头注入
// fwrite 批量落盘。user 为调用方上下文（可空）。
typedef void (*fce11_trace_sink_t)(traceRecord_t &rec, void *user);

// 反汇编钩子：语义与 DisassembleWithDebug(addr, opcode, flags, str) 相同。
typedef int (*fce11_trace_disasm_t)(int addr, uint8 *opcode, int flags, char *str);

// 宿主注入反汇编实现。未注入时使用 core 的 Disassemble（asm.cpp）。
void fce11_trace_set_disassembler(fce11_trace_disasm_t fn);

// 采集体 —— 原 fceWrapper_TraceInstruction (TraceLogger.cpp:1373-1547) 主体
// 的逐行迁移（logging_options 改从参数传入，其余不动）。
// 返回 0 = 记录有效、应交给 sink 下发；
// 返回非 0 = 记录被 LOG_NEW_INSTRUCTIONS / LOG_NEW_DATA 过滤丢弃
//（对应原函数中的中途 return —— 不 push、不下发）。
int fce11_trace_capture(traceRecord_t &rec, uint8 *opcode, int size, uint32_t options);

// 每指令入口：采集 + sink 下发一步完成。sink 可为空（仅采集，供探针）。
void fce11_trace_instruction(uint8 *opcode, int size, uint32_t options,
                             fce11_trace_sink_t sink, void *user);
