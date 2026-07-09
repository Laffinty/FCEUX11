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
