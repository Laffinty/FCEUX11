// headless_trace.cpp
//
// B1 (v2.0.2 P1 续): 见 headless_trace.h 头注。
//
// 采集体复用 trace_common.cpp 的 fce11_trace_capture（与 Qt TraceLogger 同一
// 份代码、同一种 convToText 行格式），sink 走本文件的行缓冲批量 fwrite。
// :1363 治理② 的无头分支在这里落地：缓冲写不下时丢弃本条并计数，每累计
// 2^20 条丢弃向 stdout 报一次 overrun（Qt 路径的 logBuf 自旋在
// TraceLogger.cpp pushToLogBuffer 原样保留，两边分叉）。

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "driver_callbacks.h"
#include "trace_common.h"

#include "headless_trace.h"

namespace fceu11
{
	namespace
	{
		// 与 Qt TraceLogDiskThread_t::run 同族的批处理参数：8 KiB 行缓冲、
		// 攒到 4 KiB 就写一次（Qt 侧为 WriteFile 同节奏）。
		const size_t TRACE_BATCH_SIZE  = 8192;
		const size_t TRACE_FLUSH_AT    = 4096;
		const unsigned long long TRACE_OVERRUN_REPORT_INTERVAL = 1ull << 20;

		FILE *s_fp = nullptr;
		bool s_active = false;
		size_t s_batchLen = 0;
		char s_batch[TRACE_BATCH_SIZE];
		unsigned long long s_dropCount = 0;
		unsigned long long s_dropAtLastReport = 0;

		void sinkWriteLine(const char *line)
		{
			size_t n = strlen(line);

			if ((n + 1) > TRACE_BATCH_SIZE)
			{
				// 单行超过缓冲（正常行 <= ~160 字节，不会发生）。
				// :1363 治理②（无头分支）：丢弃本条 + 计数。
				s_dropCount++;
				return;
			}

			if (s_batchLen + n + 1 > TRACE_BATCH_SIZE)
			{
				fwrite(s_batch, 1, s_batchLen, s_fp);
				s_batchLen = 0;
			}

			memcpy(s_batch + s_batchLen, line, n);
			s_batchLen += n;
			s_batch[s_batchLen] = '\n';
			s_batchLen++;

			if (s_batchLen >= TRACE_FLUSH_AT)
			{
				fwrite(s_batch, 1, s_batchLen, s_fp);
				s_batchLen = 0;
			}
		}

		void headlessTraceSink(traceRecord_t &rec, void *user)
		{
			char line[256];

			(void)user;

			// opSize==0 是 Qt 侧 pushMsgToLogBuffer 的消息记录；无头路径
			// 不产生，防御性跳过（convToText 对它的输出不带换行前缀字段）。
			if (rec.opSize == 0)
			{
				return;
			}

			rec.convToText(line);

			sinkWriteLine(line);

			// :1363 治理②（无头分支）：每累计 2^20 条丢弃报一次 overrun。
			if (s_dropCount - s_dropAtLastReport >= TRACE_OVERRUN_REPORT_INTERVAL)
			{
				printf("headless trace: %llu lines dropped (buffer overrun)\n",
				       (unsigned long long)(s_dropCount - s_dropAtLastReport));
				s_dropAtLastReport = s_dropCount;
			}
		}

		void headlessTraceInstruction(uint8 *opcode, int size)
		{
			// 与 Qt 同一份采集体、同一份默认 logging_options（trace_common.cpp），
			// 落盘行格式与 Qt TraceLogger 完全一致。
			fce11_trace_instruction(opcode, size, (uint32_t)logging_options,
			                        headlessTraceSink, nullptr);
		}
	}

	void headless_trace_init(void)
	{
		const char *env = getenv("FCEUX11_TRACE");
		const char *path;

		if (env == nullptr || env[0] == 0)
		{
			return; // 未设：零开销、零产物
		}

		if (s_active)
		{
			// 本进程内已安装（fceu11::Initialize 可能被多次调用，
			// 如测试循环装载多个 ROM）：保持现状即可。
			return;
		}

		if (g_driver().trace_instruction != nullptr)
		{
			// 槽被占（Qt TraceLogger 已注册）：让位，只说明一句。
			printf("FCEUX11_TRACE is set but a trace instruction handler is already installed; headless trace not started.\n");
			return;
		}

		path = (strcmp(env, "1") == 0) ? "fceux11-trace.log" : env;

		s_fp = fopen(path, "w");
		if (s_fp == nullptr)
		{
			printf("FCEUX11_TRACE: failed to open trace log file for writing: %s\n", path);
			return;
		}

		g_driver().trace_instruction = headlessTraceInstruction;
		// A1 状态位：与 DebugCycle 快速门同窗，不置位则采集器永远到不了。
		g_driver().trace_logging = 1;
		s_active = true;
	}

	void headless_trace_flush(void)
	{
		if (!s_active)
		{
			return;
		}

		if (s_batchLen > 0)
		{
			fwrite(s_batch, 1, s_batchLen, s_fp);
			s_batchLen = 0;
		}
		fflush(s_fp);
	}
}
