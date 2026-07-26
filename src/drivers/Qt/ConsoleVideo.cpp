// ConsoleEmuControl.cpp
//

/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2020 mjbudd77
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
// ConsoleWindow.cpp
//
#include <fstream>
#include <iostream>
#include <cstdlib>

#include <QPixmap>
#include <QWindow>
#include <QScreen>
#include <QSettings>

// v0.3.15.x PHASE-4: TypedConfig<T> wrapper for QSettings.
#include "ConfigStore.h"
#include <QHeaderView>
#include <QFileInfo>
#include <QFileDialog>
#include <QMessageBox>
#include <QInputDialog>
#include <QTranslator>
#include <QActionGroup>
#include <QSignalBlocker>
#include <QDesktopServices>
#include <QStyleFactory>
#include <QApplication>
#include <QShortcut>
#include <QUrl>

#include "../../fceu.h"
#include "../../fds.h"
#include "../../file.h"
#include "../../input.h"
#include "../../movie.h"
#include "../../wave.h"
#include "../../state.h"
#include "../../profiler.h"
#include "../../version.h"
#include "../../core_api.h"
#include "common/os_utils.h"
#include "utils/timeStamp.h"

#ifdef _S9XLUA_H
#include "../../fceulua.h"
#endif

#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/input.h"
#include "Qt/ColorMenu.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/InputConf.h"
#include "Qt/GamePadConf.h"
#include "Qt/FamilyKeyboard.h"
#include "Qt/HotKeyConf.h"
#include "Qt/PaletteConf.h"
#include "Qt/PaletteEditor.h"
#include "Qt/HelpPages.h"
#include "Qt/GuiConf.h"
#include "Qt/AviRecord.h"
#include "Qt/AviRiffViewer.h"
#include "Qt/MoviePlay.h"
#include "Qt/MovieRecord.h"
#include "Qt/MovieOptions.h"
#include "Qt/StateRecorderConf.h"
#include "Qt/TimingConf.h"
#include "Qt/FrameTimingStats.h"
#include "Qt/LuaControl.h"
#include "Qt/CheatsConf.h"
#include "Qt/GameGenie.h"
#include "Qt/HexEditor.h"
#include "Qt/TraceLogger.h"
#include "Qt/CodeDataLogger.h"
#include "Qt/ConsoleDebugger.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/ConsoleSoundConf.h"
#include "Qt/ConsoleVideoConf.h"
#include "Qt/MsgLogViewer.h"
#include "Qt/AboutWindow.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ppuViewer.h"
#include "Qt/NameTableViewer.h"
#include "Qt/iNesHeaderEditor.h"
#include "Qt/RamWatch.h"
#include "Qt/RamSearch.h"
#include "Qt/keyscan.h"
#include "common/nes_shm.h"
#include "Qt/TasEditor/TasEditorWindow.h"


// hotfix1 P0-4 (C-12): a prior encoding/format conversion corrupted the
// newline between these two includes to a literal backtick-n, breaking the
// build. Restored the real line break.
#include "Qt/ConsoleVideo.h"
#include "Qt/ConsoleWindow.h"

int consoleWin_t::unloadVideoDriver(void)
{
	viewport_Interface = NULL;

	if (viewport_GL != NULL)
	{
		QWidget *container = centralWidget();
		if ( container )
		{
			takeCentralWidget();
			container->deleteLater();
		}
		else
		{
			printf("Error: Central Widget Failed!\n");
		}
		viewport_GL->deleteLater();

		viewport_GL = NULL;
	}

	if (viewport_SDL != NULL)
	{
		if ( viewport_SDL == centralWidget() )
		{
			takeCentralWidget();
		}
		else
		{
			printf("Error: Central Widget Failed!\n");
		}
		viewport_SDL->deleteLater();

		viewport_SDL = NULL;
	}

	if (viewport_QWidget != NULL)
	{
		if ( viewport_QWidget == centralWidget() )
		{
			takeCentralWidget();
		}
		else
		{
			printf("Error: Central Widget Failed!\n");
		}
		viewport_QWidget->deleteLater();

		viewport_QWidget = NULL;
	}
	return 0;
}
//---------------------------------------------------------------------------
void consoleWin_t::videoDriverDestroyed(QObject* obj)
{
	if (viewport_GL == obj)
	{
		//printf("GL Video Driver Destroyed\n");

		if (viewport_Interface == static_cast<ConsoleViewerBase*>(viewport_GL))
		{
			viewport_Interface = NULL;
		}
		viewport_GL = NULL;
	}

	if (viewport_SDL == obj)
	{
		//printf("SDL Video Driver Destroyedi\n");

		if (viewport_Interface == static_cast<ConsoleViewerBase*>(viewport_SDL))
		{
			viewport_Interface = NULL;
		}
		viewport_SDL = NULL;
	}

	if (viewport_QWidget == obj)
	{
		//printf("QPainter Video Driver Destroyed\n");

		if (viewport_Interface == static_cast<ConsoleViewerBase*>(viewport_QWidget))
		{
			viewport_Interface = NULL;
		}
		viewport_QWidget = NULL;
	}
	printf("Video Driver Destroyed: %p\n", obj);
	//printf("viewport_GL: %p\n", viewport_GL);
	//printf("viewport_SDL: %p\n", viewport_SDL);
	//printf("viewport_Qt: %p\n", viewport_QWidget);
	//printf("viewport_Interface: %p\n", viewport_Interface);
}
//---------------------------------------------------------------------------
int consoleWin_t::loadVideoDriver( int driverId, bool force )
{
	if (viewport_Interface)
	{
		if (viewport_Interface->driver() == driverId)
		{  // Already Loaded
			if (force)
			{
				unloadVideoDriver();
			}
			else
			{
				return 0;
			}
		}
	}

	switch ( driverId )
	{  
		case ConsoleViewerBase::VIDEO_DRIVER_SDL:
		{
			viewport_SDL = new ConsoleViewSDL_t(this);

			viewport_Interface = static_cast<ConsoleViewerBase*>(viewport_SDL);

			setCentralWidget(viewport_SDL);

			setViewportAspect();

			viewport_SDL->init();

			connect( viewport_SDL, SIGNAL(destroyed(QObject*)), this, SLOT(videoDriverDestroyed(QObject*)), Qt::QueuedConnection );
		}
		break;
		case ConsoleViewerBase::VIDEO_DRIVER_OPENGL:
		{
			viewport_GL = new ConsoleViewGL_t();

			viewport_Interface = static_cast<ConsoleViewerBase*>(viewport_GL);

			QWidget *container = QWidget::createWindowContainer(viewport_GL, this);
			container->setFocusPolicy(Qt::StrongFocus);
			container->setMinimumSize( QSize(256, 224) );

			setCentralWidget(container);

			setViewportAspect();

			viewport_GL->init();

			connect( viewport_GL, SIGNAL(destroyed(QObject*)), this, SLOT(videoDriverDestroyed(QObject*)), Qt::QueuedConnection );
		}
		break;
		default:
		case ConsoleViewerBase::VIDEO_DRIVER_QPAINTER:
		{
			viewport_QWidget = new ConsoleViewQWidget_t(this);

			viewport_Interface = static_cast<ConsoleViewerBase*>(viewport_QWidget);

			setCentralWidget(viewport_QWidget);

			setViewportAspect();

			viewport_QWidget->init();

			connect( viewport_QWidget, SIGNAL(destroyed(QObject*)), this, SLOT(videoDriverDestroyed(QObject*)), Qt::QueuedConnection );
		}
		break;
	}

	// Reload Viewport Cursor Type and Visibility
	loadCursor();

	return 0;
}
//---------------------------------------------------------------------------
//---------------------------------------------------------------------------
//---------------------------------------------------------------------------
//---------------------------------------------------------------------------
//---------------------------------------------------------------------------
void consoleWin_t::toggleMenuVis(void)
{
	if ( menubar->isVisible() )
	{
		menubar->setVisible( false );
	}
	else
	{
		menubar->setVisible( true );
	}
}
//---------------------------------------------------------------------------
void consoleWin_t::toggleMenuAutoHide(bool checked)
{
	autoHideMenuFullscreen = checked;

	g_config->setOption( "SDL.AutoHideMenuFullsreen", autoHideMenuFullscreen );
	g_config->save();
}
//---------------------------------------------------------------------------
void consoleWin_t::toggleUseBgPaletteForVideo(bool checked)
{
	usePaletteForVideoBg = checked;

	g_config->setOption( "SDL.UseBgPaletteForVideo", usePaletteForVideoBg );
	g_config->save();

	if ( !usePaletteForVideoBg )
	{
		fceuLoadConfigColor( "SDL.VideoBgColor", &videoBgColor );
	}
	bgColorMenuItem->setEnabled( !usePaletteForVideoBg );
}
//---------------------------------------------------------------------------
void consoleWin_t::closeApp(void)
{
	nes_shm->runEmulator.store(0, std::memory_order_release);

	gameTimer->stop();

	closeGamePadConfWindow();

	// hotfix1 P1-2 (N-C02, upgraded to CRITICAL):
	//   emulatorThread_t inherits QThread but overrides run() directly
	//   instead of calling exec(). For a QThread with no event loop,
	//   `quit()` is a documented no-op — `quit` posts a quit event to a
	//   loop that never starts. The old wait(1000) therefore offered no
	//   guarantee that the thread had actually stopped, and once it timed
	//   out we still called fceuWrapperClose() concurrently with a thread
	//   that was still inside fceuWrapperUpdate() / fceuWrapper -> close
	//   path: textbook use-after-free.
	//
	//   Order matters and must be:
	//     1. Tell the thread to stop (runEmulator=0 + requestInterruption)
	//     2. Wait up to 5 s for it to finish its current frame and exit
	//     3. As a last resort terminate() it (UNSAFE: leaves mutexes held,
	//        transitively cleaned up by process exit) and wait() again
	//     4. Only THEN tear down fceuWrapper state.
	//
	// hotfix3 A-4 (QT-CRASH-03): mark `closed_` so the dtor knows the
	// wait has already been performed (avoids a redundant wait block
	// on the graceful-quit path that goes closeApp -> delete).
	closed_ = true;
	emulatorThread->requestInterruption();
	if (!emulatorThread->wait(5000)) {
		qWarning("Emulator thread did not exit cleanly within 5s; terminating");
		emulatorThread->terminate();
		emulatorThread->wait();
	}

	aviDiskThread->requestInterruption();
	aviDiskThread->quit();
	aviDiskThread->wait( 10000 );

	if ( tasWin != NULL )
	{
		tasWin->requestWindowClose();
	}

	FCEU_WRAPPER_LOCK();
	fceuWrapperClose();
	FCEU_WRAPPER_UNLOCK();

	// LoadGame() checks for an IP and if it finds one begins a network session
	// clear the NetworkIP field so this doesn't happen unintentionally
	g_config->setOption ("SDL.NetworkIP", "");
	g_config->save ();

	QApplication::closeAllWindows();

	// Delay Application Quit to allow event processing to complete
	QTimer::singleShot( 250, qApp, SLOT(quit(void)) );
}
//---------------------------------------------------------------------------
void consoleWin_t::videoBgColorChanged( QColor &c )
{
	//printf("Color Changed\n");

	if ( viewport_Interface )
	{
		viewport_Interface->setBgColor(c);
		viewport_Interface->queueRedraw();
	}
}
