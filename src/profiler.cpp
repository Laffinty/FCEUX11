/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
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
// profiler.cpp
//
// Phase 8 (v0.2.9): profilerFuncMap & profilerManager backend moved to Rust.
// C++ side retains the class shells, macros, and RAII scoping objects;
// only the map/stack storage and thread-list management are delegated.
//
#ifdef __FCEU_PROFILER_ENABLE__

#include <stdio.h>
#include <algorithm>
#include <mutex>
#include <vector>

#include "utils/safe_string.h"

#include "fceu.h"
#include "driver_callbacks.h"
#include "profiler.h"
#include "rust/fceux11_rust.h"

namespace FCEU
{
static thread_local profileExecVector execList;
static thread_local profilerFuncMap threadProfileMap;

// B2③: C++ 侧线程表副本 —— Rust mgr 只按 cpp 指针管理生命周期，没有
// 遍历 ABI；报告需要跨线程（emu 线程的 Emulate/DoLine/RefreshLine 探点
// 记录在各自 thread_local 的 map 里），所以在 add/removeThreadProfiler
// 的同时维护这份 C++ 侧列表。
static std::mutex s_threadMapsMutex;
static std::vector<profilerFuncMap *> s_threadMaps;

FILE *profilerManager::pLog = nullptr;

static profilerManager  pMgr;

//-------------------------------------------------------------------------
//---- Function Profile Record
//-------------------------------------------------------------------------
funcProfileRecord::funcProfileRecord(const char *fileNameStringLiteral,
				     const int   fileLineNumber,
				     const char *funcNameStringLiteral,
				     const char *commentStringLiteral)

	: fileLineNum(fileLineNumber), fileName(fileNameStringLiteral),
	  funcName(funcNameStringLiteral), comment(commentStringLiteral)
{
	// B2③ ABI 桥接（修复 ON 分支预存编译错误）：Rust registry 以 u64 id
	// 定址，构造时 take 一次；静态记录与进程同生命周期，永不 drop，
	// registry 条目数 = 探点数（有界）。fromSeconds(9) 的 int 字面量在
	// double/unsigned int 两个重载间有二义性，改 9.0（语义不变）。
	rustId = fceux11_rust_profiler_record_take(this);

	min.fromSeconds(9.0);
	max.zero();
	sum.zero();
	numCalls = 0;
	recursionCount = 0;

	threadProfileMap.addRecord( fileNameStringLiteral, fileLineNumber,
					funcNameStringLiteral, commentStringLiteral, rustId);
}
//-------------------------------------------------------------------------
void funcProfileRecord::reset(void)
{
	min.fromSeconds(9.0);
	max.zero();
	sum.zero();
	numCalls = 0;
}
//-------------------------------------------------------------------------
double funcProfileRecord::average(void)
{
	double avg = 0.0;

	if (numCalls)
	{
		avg = sum.toSeconds() / static_cast<double>(numCalls);
	}
	return avg;
}
//-------------------------------------------------------------------------
//---- Profile Scoped Function Class
//-------------------------------------------------------------------------
profileFuncScoped::profileFuncScoped( funcProfileRecord *recordIn )
{
	rec = recordIn;

	if (rec)
	{
		threadProfileMap.pushStack(rec);
		start.readNew();
		rec->numCalls++;
		rec->recursionCount++;
	}
}
//-------------------------------------------------------------------------
profileFuncScoped::~profileFuncScoped(void)
{
	if (rec)
	{
		timeStampRecord ts, dt;
		ts.readNew();
		dt = ts - start;

		rec->last = dt;
		rec->sum += dt;
		if (dt < rec->min) rec->min = dt;
		if (dt > rec->max) rec->max = dt;

		rec->recursionCount--;

		execList._vec.push_back(*rec);

		threadProfileMap.popStack(rec);
	}
}
//-------------------------------------------------------------------------
//---- Profile Execution Vector
//-------------------------------------------------------------------------
profileExecVector::profileExecVector(void)
{
	_vec.reserve( 10000 );

	char threadName[128];
	char fileName[256];

	FCEU_strlcpy(threadName, sizeof(threadName), "MainThread");

	if (auto* fn = fceu11::g_driver().get_thread_name) {
		FCEU_strlcpy(threadName, sizeof(threadName), fn());
	}
	snprintf( fileName, sizeof(fileName), "fceux-profile-%s.log", threadName);

	logFp = ::fopen(fileName, "w");

	if (logFp == nullptr)
	{
		printf("Error: Failed to create profiler logfile: %s\n", fileName);
	}
}
//-------------------------------------------------------------------------
profileExecVector::~profileExecVector(void)
{
	if (logFp)
	{
		::fclose(logFp);
	}
}
//-------------------------------------------------------------------------
void profileExecVector::update(void)
{
	size_t n = _vec.size();

	for (size_t i=0; i<n; i++)
	{
		funcProfileRecord &rec = _vec[i];

		fprintf( logFp, "%s: %u  %f  %f  %f  %f\n", rec.funcName, rec.numCalls, rec.last.toSeconds(), rec.average(), rec.min.toSeconds(), rec.max.toSeconds());
	}
	_vec.clear();
}
//-------------------------------------------------------------------------
//---- Profile Function Record Map
//-------------------------------------------------------------------------
profilerFuncMap::profilerFuncMap(void)
{
	//printf("profilerFuncMap Constructor: %p\n", this);
	_rust_handle = fceux11_rust_profiler_map_create();
	pMgr.addThreadProfiler(this);
}
//-------------------------------------------------------------------------
profilerFuncMap::~profilerFuncMap(void)
{
	//printf("profilerFuncMap Destructor: %p\n", this);
	pMgr.removeThreadProfiler(this);

	fceux11_rust_profiler_map_destroy(_rust_handle);
}
//-------------------------------------------------------------------------
void profilerFuncMap::pushStack(funcProfileRecord *rec)
{
	// B2③ ABI 桥接：push/pop 传记录的常驻 id（构造时 record_take 取得）。
	// 每次调用不再 take/drop（push/pop 发生在 Emulate/DoLine 频度），
	// registry 零增长。
	fceux11_rust_profiler_map_push_stack(_rust_handle, rec->rustId);
}
//-------------------------------------------------------------------------
void profilerFuncMap::popStack(funcProfileRecord *rec)
{
	fceux11_rust_profiler_map_pop_stack(_rust_handle, rec->rustId);
}
//-------------------------------------------------------------------------
int profilerFuncMap::addRecord(const char *fileNameStringLiteral,
			      const int   fileLineNumber,
			      const char *funcNameStringLiteral,
			      const char *commentStringLiteral,
			      uint64_t record_id )
{
	return fceux11_rust_profiler_map_add_record(_rust_handle,
						    fileNameStringLiteral,
						    fileLineNumber,
						    funcNameStringLiteral,
						    commentStringLiteral,
						    record_id);
}
//-------------------------------------------------------------------------
funcProfileRecord *profilerFuncMap::findRecord(const char *fileNameStringLiteral,
				       const int   fileLineNumber,
				       const char *funcNameStringLiteral,
				       const char *commentStringLiteral,
				       bool create)
{
	// Not implemented in Rust path; this function is unused in the
	// current codebase and exists only for API completeness.
	(void)fileNameStringLiteral;
	(void)fileLineNumber;
	(void)funcNameStringLiteral;
	(void)commentStringLiteral;
	(void)create;
	return nullptr;
}
//-------------------------------------------------------------------------
funcProfileRecord *profilerFuncMap::iterateBegin(void)
{
	return static_cast<funcProfileRecord*>(fceux11_rust_profiler_map_iterate_begin(_rust_handle));
}
//-------------------------------------------------------------------------
funcProfileRecord *profilerFuncMap::iterateNext(void)
{
	return static_cast<funcProfileRecord*>(fceux11_rust_profiler_map_iterate_next(_rust_handle));
}
//-------------------------------------------------------------------------
//-----  profilerManager class
//-------------------------------------------------------------------------
profilerManager* profilerManager::instance = nullptr;

profilerManager* profilerManager::getInstance(void)
{
	return instance;
}
//-------------------------------------------------------------------------
profilerManager::profilerManager(void)
{
	//printf("profilerManager Constructor\n");
	if (pLog == nullptr)
	{
		pLog = stdout;
	}

	if (instance == nullptr)
	{
		instance = this;
	}
}

profilerManager::~profilerManager(void)
{
	//printf("profilerManager Destructor\n");
	fceux11_rust_profiler_mgr_clear();

	if (pLog && (pLog != stdout))
	{
		fclose(pLog); pLog = nullptr;
	}
	if (instance == this)
	{
		instance = nullptr;
	}
}

int profilerManager::addThreadProfiler( profilerFuncMap *m )
{
	{
		std::lock_guard<std::mutex> lock(s_threadMapsMutex);
		s_threadMaps.push_back(m);
	}
	return fceux11_rust_profiler_mgr_add(m);
}

int profilerManager::removeThreadProfiler( profilerFuncMap *m, bool shouldDestroy )
{
	{
		std::lock_guard<std::mutex> lock(s_threadMapsMutex);
		for (size_t i=0; i<s_threadMaps.size(); i++)
		{
			if (s_threadMaps[i] == m)
			{
				s_threadMaps.erase(s_threadMaps.begin() + i);
				break;
			}
		}
	}
	return fceux11_rust_profiler_mgr_remove(m, shouldDestroy ? 1 : 0);
}
//-------------------------------------------------------------------------
} // namespace FCEU

//-------------------------------------------------------------------------
int FCEU_profiler_log_thread_activity(void)
{
	FCEU::execList.update();
	return 0;
}

//-------------------------------------------------------------------------
// B2③: 汇总报告。遍历所有已注册线程的 profilerFuncMap（Rust 后端
// iterateBegin/Next），按 sum 降序输出汇总表。
//
// 前提：调用点在 fceu11::Kill() 末尾 —— 模拟线程已停，跨线程读记录
// 安全；运行中调用读到的只是瞬态值，不做同步。
//-------------------------------------------------------------------------
int FCEU_profiler_report(FILE *out)
{
	std::vector<FCEU::funcProfileRecord *> recs;
	double total = 0.0;
	unsigned long long totalCalls = 0;

	if (out == nullptr)
	{
		return -1;
	}

	{
		std::lock_guard<std::mutex> lock(FCEU::s_threadMapsMutex);

		for (size_t i=0; i<FCEU::s_threadMaps.size(); i++)
		{
			FCEU::profilerFuncMap *m = FCEU::s_threadMaps[i];

			for (FCEU::funcProfileRecord *r = m->iterateBegin(); r; r = m->iterateNext())
			{
				if (r->numCalls > 0)
				{
					recs.push_back(r);
				}
			}
		}
	}

	for (size_t i=0; i<recs.size(); i++)
	{
		total += recs[i]->sum.toSeconds();
		totalCalls += recs[i]->numCalls;
	}

	std::sort(recs.begin(), recs.end(),
		[](const FCEU::funcProfileRecord *a, const FCEU::funcProfileRecord *b)
		{
			return a->sum.toSeconds() > b->sum.toSeconds();
		});

	fprintf(out, "==== FCEU Profiler Report: %u probe sites, %llu calls, total %.6f s ====\n",
		(unsigned)recs.size(), totalCalls, total);
	fprintf(out, "%12s %8s %12s %12s %12s %10s  %s\n",
		"sum(s)", "%total", "avg(us)", "min(us)", "max(us)", "calls", "func [file:line]");

	for (size_t i=0; i<recs.size(); i++)
	{
		FCEU::funcProfileRecord *r = recs[i];
		double sum = r->sum.toSeconds();
		double pct = (total > 0.0) ? (100.0 * sum / total) : 0.0;

		fprintf(out, "%12.6f %7.2f%% %12.3f %12.3f %12.3f %10u  %s [%s:%d]\n",
			sum, pct,
			r->average() * 1e6,
			r->min.toSeconds() * 1e6,
			r->max.toSeconds() * 1e6,
			r->numCalls,
			r->funcName, r->fileName, r->fileLineNum);
	}

	fflush(out);
	return (int)recs.size();
}
#endif //  __FCEU_PROFILER_ENABLE__
