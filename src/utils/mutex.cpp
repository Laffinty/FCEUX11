// mutex.cpp — v1.11 Phase H: pImpl backend for FCEU::mutex.
// Qt mutex types are confined to this single translation unit; the header
// exposes only a void* opaque pointer so no Qt headers leak into core TUs.

#include "mutex.h"

#ifdef __QT_DRIVER__
#include <QMutex>
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
#include <QRecursiveMutex>
#endif
#endif

namespace FCEU
{

mutex::mutex(void)
	: impl_(nullptr)
{
#ifdef __QT_DRIVER__
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
	impl_ = new QRecursiveMutex();
#else
	impl_ = new QMutex();
#endif
#endif
}

mutex::~mutex(void)
{
#ifdef __QT_DRIVER__
	if (impl_)
	{
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
		delete static_cast<QRecursiveMutex*>(impl_);
#else
		delete static_cast<QMutex*>(impl_);
#endif
	}
#endif
}

void mutex::lock(void)
{
#ifdef __QT_DRIVER__
	if (impl_)
	{
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
		static_cast<QRecursiveMutex*>(impl_)->lock();
#else
		static_cast<QMutex*>(impl_)->lock();
#endif
	}
#endif
}

void mutex::unlock(void)
{
#ifdef __QT_DRIVER__
	if (impl_)
	{
#if QT_VERSION >= QT_VERSION_CHECK(6,0,0)
		static_cast<QRecursiveMutex*>(impl_)->unlock();
#else
		static_cast<QMutex*>(impl_)->unlock();
#endif
	}
#endif
}

};
