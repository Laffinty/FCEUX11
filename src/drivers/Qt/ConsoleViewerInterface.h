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
// ConsoleViewerInterface.h
//
#pragma once
#include <stdint.h>

#include <QColor>
#include <QCursor>
#include <QSize>

class ConsoleViewerBase
{
	public:
		enum VideoDriver
		{
			VIDEO_DRIVER_OPENGL = 0,
			VIDEO_DRIVER_SDL,
			VIDEO_DRIVER_QPAINTER
		};
		virtual int  init(void) = 0;
		virtual void reset(void) = 0;
		virtual void queueRedraw(void) = 0;
		virtual int  driver(void) = 0;

		virtual void transfer2LocalBuffer(void) = 0;

		virtual void setVsyncEnable( bool ena ) = 0;
		virtual void setLinearFilterEnable( bool ena ) = 0;

		virtual bool   getForceAspectOpt(void) = 0;
		virtual void   setForceAspectOpt( bool val ) = 0;
		virtual bool   getAutoScaleOpt(void) = 0;
		virtual void   setAutoScaleOpt( bool val ) = 0;
		virtual double getScaleX(void) = 0;
		virtual double getScaleY(void) = 0;
		virtual void   setScaleXY( double xs, double ys ) = 0;
		virtual void   getNormalizedCursorPos( double &x, double &y ) = 0;
		virtual bool   getMouseButtonState( unsigned int btn ) = 0;
		virtual void   setAspectXY( double x, double y ) = 0;
		virtual void   getAspectXY( double &x, double &y ) = 0;
		virtual double getAspectRatio(void) = 0;

		virtual void   setCursor(const QCursor &c) = 0;
		virtual void   setCursor( Qt::CursorShape s ) = 0;
		virtual void   setBgColor( QColor &c ) = 0;

		virtual QSize   size(void) = 0;
		virtual QCursor cursor(void) = 0;
		virtual void    setMinimumSize(const QSize &) = 0;
		virtual void    setMaximumSize(const QSize &) = 0;

		static  void    memset32( void *buf, uint32_t val, size_t size);
		static  void    copyPixels32( void *dest, void *src, size_t size, uint32_t alphaMask);

		static constexpr uint32_t alphaMask = 0xff000000;
	protected:
};
