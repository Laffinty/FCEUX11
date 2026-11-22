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
// ConsoleCursor.cpp
//
#include <QPixmap>

#include "Qt/ConsoleWindow.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleViewerInterface.h"

void consoleWin_t::loadCursor(void)
{
	int cursorVis;

	g_config->getOption("SDL.CursorVis", &cursorVis );

	if ( cursorVis )
	{
		int cursorType;

		g_config->getOption("SDL.CursorType", &cursorType );

		switch ( cursorType )
		{
			case 4:
			{
				QPixmap reticle(":/icons/reticle.png");

				setViewerCursor( QCursor(reticle.scaled(64,64)) );
			}
			break;
			case 3:
			{
				QPixmap reticle(":/icons/reticle.png");

				setViewerCursor( QCursor(reticle.scaled(32,32)) );
			}
			break;
			case 2:
				setViewerCursor( Qt::BlankCursor );
			break;
			case 1:
				setViewerCursor( Qt::CrossCursor );
			break;
			default:
			case 0:
				setViewerCursor( Qt::ArrowCursor );
			break;
		}
	}
	else
	{
		setViewerCursor( Qt::BlankCursor );
	}
}

void consoleWin_t::setViewerCursor( QCursor s )
{
	if (viewport_Interface)
	{
		viewport_Interface->setCursor(s);
	}
}

void consoleWin_t::setViewerCursor( Qt::CursorShape s )
{
	if (viewport_Interface)
	{
		viewport_Interface->setCursor(s);
	}
}

Qt::CursorShape consoleWin_t::getViewerCursor(void)
{
	Qt::CursorShape s = Qt::ArrowCursor;

	if (viewport_Interface)
	{
		s = viewport_Interface->cursor().shape();
	}
	return s;
}
