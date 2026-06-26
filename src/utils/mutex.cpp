// mutex.cpp
#include <cstdio>

#include "mutex.h"

namespace FCEU
{

//-----------------------------------------------------
// Cross platform mutex
// __QT_DRIVER__  multi-threaded application that uses Qt mutex implementation for synchronization
// __WIN_DRIVER__ is single thread application so sync methods are unimplemented.
//-----------------------------------------------------

// R5a R6.1: mtx is now std::unique_ptr<QRecursiveMutex> / std::unique_ptr<QMutex>
// (see mutex.h). The ctor body uses std::make_unique so the deleter is set
// up safely; the dtor is `= default` so the unique_ptr's dtor handles the
// underlying delete without any chance of a leak from a forgotten branch.
mutex::mutex(void)
{
#ifdef __QT_DRIVER__
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
	mtx = std::make_unique<QRecursiveMutex>();
#else
	mtx = std::make_unique<QMutex>();
#endif
#endif
}

mutex::~mutex(void) = default;

void mutex::lock(void)
{
#ifdef __QT_DRIVER__
	mtx->lock();
#endif
}

void mutex::unlock(void)
{
#ifdef __QT_DRIVER__
	mtx->unlock();
#endif
}

//-----------------------------------------------------
// Scoped AutoLock
// R5a R6.2: the two non-template overloads were collapsed into a single
// forwarding-reference template defined in mutex.h. The two constructor
// bodies that used to live here are gone; the dtor stayed (mirroring the
// header's definition) so the out-of-line body matches the in-header
// declaration exactly.
//-----------------------------------------------------
// autoScopedLock::autoScopedLock(mutex*)  -- moved to mutex.h (template)
// autoScopedLock::autoScopedLock(mutex&)  -- moved to mutex.h (template)

};
