/* FCE Ultra - NES/Famicom Emulator
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
