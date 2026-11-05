// trace_common.cpp
//
// B1 (v2.0.2 P1 续): 见 trace_common.h 头注。
// traceRecord_t 三个成员函数与 convToXchar 为 TraceLogger.cpp:936-1224 的
// 纯剪切；fce11_trace_capture 为 TraceLogger.cpp:1373-1547 采集体逐行迁移
// （唯二的结构性改动见头注：pushToLogBuffer 移出采集体改为 sink 注入、
// DisassembleWithDebug 经钩子注入；另有 :1363 治理③ 的 1-entry memo，
// 带 Page/PRG 守卫、可证明等价）。

#include <stdio.h>
#include <string.h>

#include "types.h"
#include "fceu.h"
#include "cpu.h"     // g_cpu
#include "movie.h"   // currFrameCounter
#include "debug.h"
#include "asm.h"
#include "x6502.h"
#include "bus.h"

#include "trace_common.h"

// 原 TraceLogger.cpp:109 的默认值，逐字未动。
int logging_options = LOG_REGISTERS | LOG_PROCESSOR_STATUS | LOG_TO_THE_LEFT | LOG_MESSAGES | LOG_BREAKPOINTS | LOG_CODE_TABBING;

// 原 TraceLogger.cpp:110 的 static 文件级变量，仅采集体使用，随迁。
static int oldcodecount = 0, olddatacount = 0;

// ---------------------------------------------------------------------------
// 反汇编钩子：默认回落到 core 的 Disassemble（asm.cpp，Rust 后端）。
// ---------------------------------------------------------------------------
static int fce11_trace_core_disasm(int addr, uint8 *opcode, int flags, char *str)
{
	char *result;
	(void)flags;

	result = Disassemble(addr, opcode);
	if (result == NULL)
	{
		str[0] = 0;
		return 0;
	}

	{
		size_t i = 0;
		while (result[i] != 0)
		{
			str[i] = result[i];
			i++;
		}
		str[i] = 0;
	}
	return 1;
}

static fce11_trace_disasm_t fce11_trace_disasm = fce11_trace_core_disasm;

void fce11_trace_set_disassembler(fce11_trace_disasm_t fn)
{
	if (fn)
	{
		fce11_trace_disasm = fn;
	}
}

// ---------------------------------------------------------------------------
// traceRecord_t —— 纯剪切自 TraceLogger.cpp:936-990
// ---------------------------------------------------------------------------
traceRecord_t::traceRecord_t(void)
{
	cpu.PC = 0;
	cpu.A = 0;
	cpu.X = 0;
	cpu.Y = 0;
	cpu.S = 0;
	cpu.P = 0;

	opCode[0] = 0;
	opCode[1] = 0;
	opCode[2] = 0;
	opSize = 0;
	asmTxtSize = 0;
	asmTxt[0] = 0;

	cycleCount = 0;
	instrCount = 0;
	flags = 0;

	callAddr = -1;
	romAddr = -1;
	bank = -1;
	skippedLines = 0;
}
//----------------------------------------------------
int traceRecord_t::appendAsmText(const char *txt)
{
	int i = 0;

	while (txt[i] != 0)
	{
		asmTxt[asmTxtSize] = txt[i];
		i++;
		asmTxtSize++;
	}
	asmTxt[asmTxtSize] = 0;

	return 0;
}
//----------------------------------------------------
static int convToXchar(int i)
{
	int c = 0;

	if ((i >= 0) && (i < 10))
	{
		c = i + '0';
	}
	else if (i < 16)
	{
		c = (i - 10) + 'A';
	}
	return c;
}
//----------------------------------------------------
int traceRecord_t::convToText(char *txt, int *len)
{
	int i = 0, j = 0;
	char stmp[128];
	char str_axystate[32], str_procstatus[32];

	str_axystate[0] = 0;
	str_procstatus[0] = 0;

	txt[0] = 0;
	if (opSize == 0)
	{
		j = 0;
		while (asmTxt[j] != 0)
		{
			txt[i] = asmTxt[j];
			i++;
			j++;
		}
		txt[i] = 0;

		return -1;
	}

	if (skippedLines > 0)
	{
		snprintf( stmp, sizeof(stmp), "(%d lines skipped) ", skippedLines);

		j = 0;
		while (stmp[j] != 0)
		{
			txt[i] = stmp[j];
			i++;
			j++;
		}
	}

	// Start filling the str_temp line: Frame count, Cycles count, Instructions count, AXYS state, Processor status, Tabs, Address, Data, Disassembly
	if (logging_options & LOG_FRAMES_COUNT)
	{
		snprintf( stmp, sizeof(stmp), "f%-6llu ", (long long unsigned int)frameCount);

		j = 0;
		while (stmp[j] != 0)
		{
			txt[i] = stmp[j];
			i++;
			j++;
		}
	}

	if (logging_options & LOG_CYCLES_COUNT)
	{
		snprintf( stmp, sizeof(stmp), "c%-11llu ", (long long unsigned int)cycleCount);

		j = 0;
		while (stmp[j] != 0)
		{
			txt[i] = stmp[j];
			i++;
			j++;
		}
	}

	if (logging_options & LOG_INSTRUCTIONS_COUNT)
	{
		snprintf( stmp, sizeof(stmp), "i%-11llu ", (long long unsigned int)instrCount);

		j = 0;
		while (stmp[j] != 0)
		{
			txt[i] = stmp[j];
			i++;
			j++;
		}
	}

	if (logging_options & LOG_REGISTERS)
	{
		snprintf(str_axystate, sizeof(str_axystate), "A:%02X X:%02X Y:%02X S:%02X ", (cpu.A), (cpu.X), (cpu.Y), (cpu.S));
	}

	if (logging_options & LOG_PROCESSOR_STATUS)
	{
		int tmp = cpu.P ^ 0xFF;
		snprintf(str_procstatus, sizeof(str_procstatus), "P:%c%c%c%c%c%c%c%c ",
				'N' | (tmp & 0x80) >> 2,
				'V' | (tmp & 0x40) >> 1,
				'U' | (tmp & 0x20),
				'B' | (tmp & 0x10) << 1,
				'D' | (tmp & 0x08) << 2,
				'I' | (tmp & 0x04) << 3,
				'Z' | (tmp & 0x02) << 4,
				'C' | (tmp & 0x01) << 5);
	}

	if (logging_options & LOG_TO_THE_LEFT)
	{
		if (logging_options & LOG_REGISTERS)
		{
			j = 0;
			while (str_axystate[j] != 0)
			{
				txt[i] = str_axystate[j];
				i++;
				j++;
			}
		}
		if (logging_options & LOG_PROCESSOR_STATUS)
		{
			j = 0;
			while (str_procstatus[j] != 0)
			{
				txt[i] = str_procstatus[j];
				i++;
				j++;
			}
		}
	}

	if (logging_options & LOG_CODE_TABBING)
	{
		// add spaces at the beginning of the line according to stack pointer
		int spaces = (0xFF - cpu.S) & LOG_TABS_MASK;

		while (spaces > 0)
		{
			txt[i] = ' ';
			i++;
			spaces--;
		}
	}
	else if (logging_options & LOG_TO_THE_LEFT)
	{
		txt[i] = ' ';
		i++;
	}

	if (logging_options & LOG_BANK_NUMBER)
	{
		if (cpu.PC >= 0x8000)
		{
			snprintf( stmp, sizeof(stmp), "$%02X:%04X: ", bank, cpu.PC);
		}
		else
		{
			snprintf( stmp, sizeof(stmp), "  $%04X: ", cpu.PC);
		}
	}
	else
	{
		snprintf( stmp, sizeof(stmp), "$%04X: ", cpu.PC);
	}
	j = 0;
	while (stmp[j] != 0)
	{
		txt[i] = stmp[j];
		i++;
		j++;
	}

	for (j = 0; j < opSize; j++)
	{
		txt[i] = convToXchar((opCode[j] >> 4) & 0x0F);
		i++;
		txt[i] = convToXchar(opCode[j] & 0x0F);
		i++;
		txt[i] = ' ';
		i++;
	}
	while (j < 3)
	{
		txt[i] = ' ';
		i++;
		txt[i] = ' ';
		i++;
		txt[i] = ' ';
		i++;
		j++;
	}
	j = 0;
	while (asmTxt[j] != 0)
	{
		txt[i] = asmTxt[j];
		i++;
		j++;
	}
	if (callAddr >= 0)
	{
		snprintf( stmp, sizeof(stmp), " (from $%04X)", callAddr);

		j = 0;
		while (stmp[j] != 0)
		{
			txt[i] = stmp[j];
			i++;
			j++;
		}
	}

	if (!(logging_options & LOG_TO_THE_LEFT))
	{
		if (logging_options & LOG_REGISTERS)
		{
			j = 0;
			while (str_axystate[j] != 0)
			{
				txt[i] = str_axystate[j];
				i++;
				j++;
			}
		}
		if (logging_options & LOG_PROCESSOR_STATUS)
		{
			j = 0;
			while (str_procstatus[j] != 0)
			{
				txt[i] = str_procstatus[j];
				i++;
				j++;
			}
		}
	}

	txt[i] = 0;

	if (len)
	{
		*len = i;
	}

	return 0;
}

// ---------------------------------------------------------------------------
// 采集体 —— 原 fceWrapper_TraceInstruction (TraceLogger.cpp:1373-1547) 迁移
// ---------------------------------------------------------------------------
int fce11_trace_capture(traceRecord_t &rec, uint8 *opcode, int size, uint32_t options)
{
	// B1 :1363 治理③：GetPRGAddress/getBank 1-entry memo。
	// 两个函数的返回值都是 (Page[A>>11], PRGptr[0..1], PRGsize[0..1],
	// GameInfo->type, debuggerPageSize, A) 的纯函数。键带上 Page 项指针与
	// PRG 指针/大小：bank 切换必然改写 Page[]，键随之失配 → 重算。
	// 命中即等价，Qt 侧产出字节不变。
	static int      memo_valid   = 0;
	static unsigned int memo_addr    = 0;
	static uint8_t *memo_page    = NULL;
	static uint8_t *memo_prgptr0 = NULL;
	static uint8_t *memo_prgptr1 = NULL;
	static uint32_t memo_prgsize0 = 0;
	static uint32_t memo_prgsize1 = 0;
	static unsigned int memo_dbgPageSize = 0;
	static int      memo_romAddr = -1;
	static int      memo_bank    = -1;

	// 原 fceWrapper_TraceInstruction 内的 static 局部，逐字未动。
	static int unloggedlines = 0;

	char asmTxt[256];
	unsigned int addr = g_cpu.native_layout().PC;
	int opts = (int)options;
	int asmFlags = ASM_DEBUG_TRACES;

	rec.cpu.PC = g_cpu.native_layout().PC;
	rec.cpu.A = g_cpu.native_layout().A;
	rec.cpu.X = g_cpu.native_layout().X;
	rec.cpu.Y = g_cpu.native_layout().Y;
	rec.cpu.S = g_cpu.native_layout().S;
	rec.cpu.P = g_cpu.native_layout().P;

	for (int i = 0; i < size; i++)
	{
		rec.opCode[i] = opcode[i];
	}
	rec.opSize = size;

	// ---- 原: rec.romAddr = GetPRGAddress(addr); rec.bank = getBank(addr); ----
	{
		uint8_t *pageEntry = Page[addr >> 11];

		if (memo_valid &&
			memo_addr == addr &&
			memo_page == pageEntry &&
			memo_prgptr0 == PRGptr[0] &&
			memo_prgptr1 == PRGptr[1] &&
			memo_prgsize0 == PRGsize[0] &&
			memo_prgsize1 == PRGsize[1] &&
			memo_dbgPageSize == debuggerPageSize)
		{
			rec.romAddr = memo_romAddr;
			rec.bank = memo_bank;
		}
		else
		{
			rec.romAddr = GetPRGAddress(addr);
			rec.bank = getBank(addr);
			memo_valid = 1;
			memo_addr = addr;
			memo_page = pageEntry;
			memo_prgptr0 = PRGptr[0];
			memo_prgptr1 = PRGptr[1];
			memo_prgsize0 = PRGsize[0];
			memo_prgsize1 = PRGsize[1];
			memo_dbgPageSize = debuggerPageSize;
			memo_romAddr = rec.romAddr;
			memo_bank = rec.bank;
		}
	}

	rec.frameCount = currFrameCounter;
	rec.instrCount = total_instructions;

	int64 counter_value = timestampbase + (uint64)g_cpu.timestamp_ref() - total_cycles_base;
	if (counter_value < 0) // sanity check
	{
		ResetDebugStatisticsCounters();
		counter_value = 0;
	}
	rec.cycleCount = counter_value;

	if (opts & LOG_SYMBOLIC)
	{
		asmFlags |= ASM_DEBUG_SYMS | ASM_DEBUG_REGS;
	}

	// if instruction executed from the RAM, skip this, log all instead
	// TODO: loops folding mame-lyke style
	if (rec.romAddr != -1)
	{
		if (((opts & LOG_NEW_INSTRUCTIONS) && (oldcodecount != codecount)) ||
			((opts & LOG_NEW_DATA) && (olddatacount != datacount)))
		{
			//something new was logged
			oldcodecount = codecount;
			olddatacount = datacount;
			if (unloggedlines > 0)
			{
				//snprintf(str_result, sizeof(str_result), "(%d lines skipped)", unloggedlines);
				rec.skippedLines = unloggedlines;
				unloggedlines = 0;
			}
		}
		else
		{
			if ((opts & LOG_NEW_INSTRUCTIONS) ||
				(opts & LOG_NEW_DATA))
			{
				if (FCEUI_GetLoggingCD())
				{
					unloggedlines++;
				}
				return -1; // filtered: 原函数此处直接 return（不 push）
			}
		}
	}

	if ((addr + size) > 0xFFFF)
	{
		//snprintf(str_data, sizeof(str_data), "%02X        ", opcode[0]);
		//snprintf(str_disassembly, sizeof(str_disassembly), "OVERFLOW");
		rec.flags |= 0x01;
	}
	else
	{
		char *a = 0;
		switch (size)
		{
		case 0:
			//snprintf(str_disassembly, sizeof(str_disassembly),"UNDEFINED");
			rec.flags |= 0x02;
			break;
		case 1:
		{
			fce11_trace_disasm(addr + 1, opcode, asmFlags, asmTxt);
			// special case: an RTS opcode
			if (opcode[0] == 0x60)
			{
				// add the beginning address of the subroutine that we exit from
				unsigned int caller_addr = GetMem(((g_cpu.native_layout().S) + 1) | 0x0100) + (GetMem(((g_cpu.native_layout().S) + 2) | 0x0100) << 8) - 0x2;
				if (GetMem(caller_addr) == 0x20)
				{
					// this was a JSR instruction - take the subroutine address from it
					unsigned int call_addr = GetMem(caller_addr + 1) + (GetMem(caller_addr + 2) << 8);
					rec.callAddr = call_addr;
				}
			}
			a = asmTxt;
			break;
		}
		case 2:
			fce11_trace_disasm(addr + 2, opcode, asmFlags, asmTxt);
			a = asmTxt;
			break;
		case 3:
			fce11_trace_disasm(addr + 3, opcode, asmFlags, asmTxt);
			a = asmTxt;
			break;
		}

		if (a)
		{
			rec.appendAsmText(a);
		}
	}

	switch ( opcode[0] )
	{
		// Zero page
		case 0x85: // STA - Store Accumulator
		case 0x86: // STX - Store X Register
		case 0x84: // STY - Store Y Register
		case 0xC6: // DEC - Decrement Memory
		case 0xE6: // INC - Increment Memory
			rec.writeAddr = opcode[1];
		break;
		// Absolute
		case 0x8D: // STA - Store Accumulator
		case 0x8E: // STX - Store X Register
		case 0x8C: // STY - Store Y Register
		case 0xCE: // DEC - Decrement Memory
		case 0xEE: // INC - Increment Memory
			rec.writeAddr = opcode[1] | opcode[2] << 8;
		break;
		// Indirect X
		case 0x81: // STA - Store A Register
			rec.writeAddr = (opcode[1] + g_cpu.native_layout().X) & 0xFF;
			rec.writeAddr = GetMem((rec.writeAddr)) | (GetMem(((rec.writeAddr)+1)&0xff))<<8;
		break;
		// Indirect Y
		case 0x91: // STA - Store A Register
			rec.writeAddr  = GetMem(opcode[1]) | (GetMem((opcode[1]+1)&0xff))<<8;
			rec.writeAddr += g_cpu.native_layout().Y;
		break;
		// Zero Page X
		case 0x95: // STA - Store Accumulator
		case 0x94: // STY - Store Y Register
		case 0xD6: // DEC - Decrement Memory
		case 0xF6: // INC - Increment Memory
			rec.writeAddr = (opcode[1]+ g_cpu.native_layout().X) & 0xFF;
		break;
		// Zero Page Y
		case 0x96: // STX - Store X Register
			rec.writeAddr = (opcode[1]+ g_cpu.native_layout().Y) & 0xFF;
		break;
		default:
			rec.writeAddr = -1;
		break;
	}

	if ( rec.writeAddr >= 0 )
	{
		rec.preWriteVal = GetMem( rec.writeAddr );
	}
	else
	{
		rec.preWriteVal = 0;
	}

	return 0;
}
//----------------------------------------------------
void fce11_trace_instruction(uint8 *opcode, int size, uint32_t options,
                             fce11_trace_sink_t sink, void *user)
{
	traceRecord_t rec;

	if (fce11_trace_capture(rec, opcode, size, options) != 0)
	{
		return;
	}

	if (sink)
	{
		sink(rec, user);
	}
}
