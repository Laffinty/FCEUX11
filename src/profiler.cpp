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
#include "utils/safe_string.h"

#include "fceu.h"
#include "driver_callbacks.h"
#include "profiler.h"
#include "rust/fceux11_rust.h"

namespace FCEU
{
static thread_local profileExecVector execList;
static thread_local profilerFuncMap threadProfileMap;

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
	min.fromSeconds(9);
	max.zero();
	sum.zero();
	numCalls = 0;
	recursionCount = 0;

	threadProfileMap.addRecord( fileNameStringLiteral, fileLineNumber,
					funcNameStringLiteral, commentStringLiteral, this);
}
//-------------------------------------------------------------------------
void funcProfileRecord::reset(void)
{
	min.fromSeconds(9);
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
	fceux11_rust_profiler_map_push_stack(_rust_handle, rec);
}
//-------------------------------------------------------------------------
void profilerFuncMap::popStack(funcProfileRecord *rec)
{
	fceux11_rust_profiler_map_pop_stack(_rust_handle, rec);
}
//-------------------------------------------------------------------------
int profilerFuncMap::addRecord(const char *fileNameStringLiteral,
			      const int   fileLineNumber,
			      const char *funcNameStringLiteral,
			      const char *commentStringLiteral,
			      funcProfileRecord *rec )
{
	return fceux11_rust_profiler_map_add_record(_rust_handle,
						    fileNameStringLiteral,
						    fileLineNumber,
						    funcNameStringLiteral,
						    commentStringLiteral,
						    rec);
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
	return fceux11_rust_profiler_mgr_add(m);
}

int profilerManager::removeThreadProfiler( profilerFuncMap *m, bool shouldDestroy )
{
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
#endif //  __FCEU_PROFILER_ENABLE__
