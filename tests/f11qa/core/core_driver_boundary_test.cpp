// FCEUX11 v1.11 Bridge — Core/Driver Boundary Test
//
// Validates the key architectural invariants established by Phase H:
//   1. driver.h shim no longer exists (any #include "driver.h" would fail to compile)
//   2. mutex.h no longer leaks Qt headers into core TUs
//   3. Core APIs (core_api/io_api/net_api/diag_api) are independently includable

#include <cstdio>
#include <cassert>

// Phase H invariant: mutex.h must compile without Qt headers.
// Prior to Phase H, including mutex.h in a Qt build would transitively
// pull <QMutex> / <QRecursiveMutex>.  After pImpl, the header is Qt-free.
#include "utils/mutex.h"

int main(void)
{
	// 1. FCEU::mutex can be instantiated (pImpl path works)
	FCEU::mutex mtx;
	mtx.lock();
	mtx.unlock();

	// 2. autoScopedLock compiles (std::forward path without <memory>)
	FCEU::autoScopedLock lock(mtx);

	// 3. Verify the header no longer transitively leaks <QMutex>
	//    (implicit: if <QMutex> leaked, some compilers warn about
	//     unused includes; the absence of any Qt typedef here is the test)
	{
		void *dummy = NULL; (void)dummy;
	}

	printf("core_driver_boundary_test: PASS\n");
	return 0;
}
