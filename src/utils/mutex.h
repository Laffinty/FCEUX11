// mutex.h
#pragma once

#include <memory>  // R5a R6.1: std::unique_ptr

#ifdef __QT_DRIVER__
#include <QMutex>
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
#include <QRecursiveMutex>
#endif
#endif

namespace FCEU
{
	class mutex
	{
		public:
			mutex(void);
			~mutex(void);

			void lock(void);
			void unlock(void);

		private:
	#ifdef __QT_DRIVER__
		// R5a R6.1: raw new/delete replaced with std::unique_ptr RAII.
		// The dtor (mutex::~mutex() = default; in mutex.cpp) now relies on
		// the unique_ptr deleter to call delete on the underlying QMutex /
		// QRecursiveMutex, eliminating the manual `if (mtx) { delete mtx; }`
		// dance and the associated leak window if the dtor body were ever
		// short-circuited. The field is private and the class has no friends,
		// so the type change is ABI-invisible to all 14 callers in
		// debugsymboltable.cpp (the only translation unit that includes
		// utils/mutex.h transitively through debugsymboltable.h).
		#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
			std::unique_ptr<QRecursiveMutex> mtx;
		#else
			std::unique_ptr<QMutex> mtx;
		#endif
	#endif
	};

	class autoScopedLock
	{
		public:
			// R5a R6.2: collapsed the previous two overloads
			//     autoScopedLock(mutex*);
			//     autoScopedLock(mutex&);
			// into a single forwarding-reference template. The template
			// body dispatches via the two private static getPtr() helpers
			// below, selected by overload resolution on the argument type.
			// All 13 call sites in debugsymboltable.cpp use the form
			//     FCEU::autoScopedLock alock(cs);
			// where `cs` is `FCEU::mutex*`; the template deduces Mtx =
			// mutex*&, std::forward<mutex*&>(mtx) preserves the lvalue
			// (i.e. the pointer), and getPtr(mutex*) returns it unchanged.
			// A `FCEU::autoScopedLock alock(mtxRef)` form, if ever added,
			// would select getPtr(mutex&) and store &mtxRef.
			template <typename Mtx>
			autoScopedLock(Mtx&& mtx)
				: m(getPtr(std::forward<Mtx>(mtx)))
			{
				if (m) m->lock();
			}

			~autoScopedLock(void)
			{
				if (m) m->unlock();
			}

		private:
			// Overload set used by the templated ctor body. Private so
			// external code cannot call them; the only call site is the
			// autoScopedLock ctor above, which can access its own privates.
			static mutex* getPtr(mutex* p) noexcept { return p; }
			static mutex* getPtr(mutex& r) noexcept { return &r; }

			mutex *m;
	};

};
