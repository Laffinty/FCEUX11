// mutex.h — v1.11 Phase H: Qt types isolated behind void* pImpl.
#pragma once

#include <utility>  // std::forward

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
			// Opaque pointer; actual type is QRecursiveMutex* (Qt6) / QMutex* (Qt5),
			// cast in mutex.cpp. nullptr when __QT_DRIVER__ is not defined.
			void* impl_;
	};

	class autoScopedLock
	{
		public:
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
			static mutex* getPtr(mutex* p) noexcept { return p; }
			static mutex* getPtr(mutex& r) noexcept { return &r; }

			mutex *m;
	};

};
