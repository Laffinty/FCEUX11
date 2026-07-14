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
#include "Qt/ConsoleMenu.h"
#include "Qt/ConsoleRecording.h"
#include "Qt/ConsoleFile.h"
#include "Qt/ConsoleActions.h"
#include "Qt/ConsoleVideo.h"
#include "Qt/ConsoleTranslation.h"
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
#include "Qt/nes_shm.h"
#include "Qt/TasEditor/TasEditorWindow.h"

consoleWin_t *consoleWindow = NULL;

consoleWin_t::consoleWin_t(QWidget *parent)
	: QMainWindow( parent )
{
	int opt, xWinPos = -1, yWinPos = -1, xWinSize = 256, yWinSize = 240;
	int videoDriver = 0;
	int setFullScreen = false;

	//QString libpath = QLibraryInfo::location(QLibraryInfo::PluginsPath);
	//printf("LibPath: '%s'\n", libpath.toStdString().c_str() );

	printf("Running on Platform: %s\n", QGuiApplication::platformName().toStdString().c_str() );

	QThread *thread = QThread::currentThread();

	if (thread)
	{
		thread->setObjectName( QString("MainThread") );
	}

	QApplication::setStyle( new fceuStyle() );

	initHotKeys();

	firstResize    = true;
	closeRequested = false;
	errorMsgValid  = false;
	viewport_GL      = NULL;
	viewport_SDL     = NULL;
	viewport_QWidget = NULL;
	viewport_Interface = NULL;

	contextMenuEnable      = false;
	soundUseGlobalFocus    = false;
	mainMenuEmuPauseSet    = false;
	mainMenuEmuWasPaused   = false;
	mainMenuPauseWhenActv  = false;
	autoHideMenuFullscreen = false;

#ifdef _WIN32
	// v0.3.15.x PHASE-3: bind the ITaskbarList3 wrapper to the
	// QMainWindow's HWND. The wrap is non-owning and lives for the
	// lifetime of consoleWin_t; the destructor releases the COM
	// reference. Failure to bind is non-fatal (older Windows / non-
	// shell processes), so we just leave taskbarProgress == nullptr.
	taskbarProgress = nullptr;
#endif

	createMainMenu();

	g_config->getOption( "SDL.PauseOnMainMenuAccess", &mainMenuPauseWhenActv );
	g_config->getOption( "SDL.AutoHideMenuFullsreen", &autoHideMenuFullscreen );
	g_config->getOption( "SDL.ContextMenuEnable", &contextMenuEnable );
	g_config->getOption( "SDL.Sound.UseGlobalFocus", &soundUseGlobalFocus );
	g_config->getOption ("SDL.VideoDriver", &videoDriver);

	loadVideoDriver( videoDriver );

	setWindowTitle( tr(FCEU_NAME_AND_VERSION) );
	setWindowIcon(QIcon(":fceux1.png"));
	setAcceptDrops(true);

#ifdef _WIN32
	// v0.3.15.x PHASE-3: Snap Layouts / taskbar progress hookup.
	// The QMainWindow is fully constructed by this point and has a
	// stable HWND, so we can hand it to ITaskbarList3. We allocate
	// the wrapper on the heap so the dtor release() runs at a
	// deterministic point (consoleWin_t dtor below).
	{
		taskbarProgress = new fceu11::platform::win11::TaskbarProgress();
		if (!taskbarProgress->init( (HWND)winId() )) {
			delete taskbarProgress;
			taskbarProgress = nullptr;
		}
	}
#endif

	gameTimer  = new QTimer( this );
mutex      = new QRecursiveMutex();
	emulatorThread = new emulatorThread_t(this);

	connect(emulatorThread, &QThread::finished, emulatorThread, &QObject::deleteLater);
	connect(emulatorThread, SIGNAL(frameFinished(void)), this, SLOT(emuFrameFinish(void)) );
	connect(emulatorThread, SIGNAL(loadRomRequest(QString)), this, SLOT(loadRomRequestCB(QString)) );

	connect( gameTimer, &QTimer::timeout, this, &consoleWin_t::updatePeriodic );

	gameTimer->setTimerType( Qt::PreciseTimer );
	gameTimer->start( 8 ); // 120hz

	emulatorThread->start();

	g_config->getOption( "SDL.SetSchedParam", &opt );

	if ( opt )
	{

	}


	SDL_DisplayMode mode;
	int sdl_err = SDL_GetCurrentDisplayMode(0,&mode);
	g_config->getOption( "SDL.Fullscreen", &setFullScreen );
	if( (sdl_err == 0) && setFullScreen )
	{
	        xWinPos = 0;
	        yWinPos = 0;
	        xWinSize = mode.w;
	        yWinSize = mode.h;
	}
	else
	{
	        g_config->getOption( "SDL.WinPosX" , &xWinPos );
	        g_config->getOption( "SDL.WinPosY" , &yWinPos );
	        g_config->getOption( "SDL.WinSizeX", &xWinSize );
	        g_config->getOption( "SDL.WinSizeY", &yWinSize );
	}

	if ( (xWinSize >= 256) && (yWinSize >= 224) )
	{
		this->resize( xWinSize, yWinSize );

		if ( (xWinPos >= 0) && (yWinPos >= 0) )
		{
			this->move( xWinPos, yWinPos );
		}
	}
	else
	{
		QSize reqSize = calcRequiredSize();

		// Since the height of menu is unknown until Qt has shows the window
		// Set the minimum viewport sizes to exactly what we need so that 
		// the window is resized appropriately. On the first resize event,
		// we will set the minimum viewport size back to 1x values that the
		// window can be shrunk by dragging lower corner.
		if ( viewport_Interface != NULL )
		{
			viewport_Interface->setMinimumSize( reqSize );
		}
		//this->resize( reqSize );
	}

	g_config->getOption( "SDL.Fullscreen", &setFullScreen );
	g_config->setOption( "SDL.Fullscreen", 0 ); // Reset full screen config parameter to false so it is never saved this way

	if ( setFullScreen )
	{
		if ( autoHideMenuFullscreen )
		{
			menubar->setVisible(false);
		}
		this->showFullScreen();
	}

	refreshRate = 0.0;
	updateCounter = 0;
	recentRomMenuReset = false;
	helpWin = 0;

	// Viewport Cursor Type and Visibility
	loadCursor();

	// Create AVI Recording Disk Thread
	aviDiskThread = new AviRecordDiskThread_t(this);

	scrHandlerConnected = false;
}

consoleWin_t::~consoleWin_t(void)
{
	QSize w;
	QClipboard *clipboard;

	// Save window size and image scaling parameters at app exit.
	w = size();

	// Only Save window size if not fullscreen and not maximized
	if ( !isFullScreen() && !isMaximized() )
	{
		// Scaling is only saved when applying video settings
		g_config->setOption( "SDL.WinPosX" , pos().x() );
		g_config->setOption( "SDL.WinPosY" , pos().y() );
		g_config->setOption( "SDL.WinSizeX", w.width() );
		g_config->setOption( "SDL.WinSizeY", w.height() );
	}
	else
	{
		QRect rect = normalGeometry();

		if ( rect.isValid() )
		{
			g_config->setOption( "SDL.WinPosX" , rect.x() );
			g_config->setOption( "SDL.WinPosY" , rect.y() );
			g_config->setOption( "SDL.WinSizeX", rect.width() );
			g_config->setOption( "SDL.WinSizeY", rect.height() );
		}
	}
	g_config->save();

	// Signal Emulator Thread to Stop
	nes_shm->runEmulator.store(0, std::memory_order_release);

#ifdef _WIN32
	// v0.3.15.x PHASE-3: release the ITaskbarList3 wrapper before
	// the QMainWindow HWND is destroyed (the wrapper holds the
	// HWND; if we let the QObject parent destructor free the
	// wrapper first, the HWND is still valid because Qt tears down
	// children after this dtor, so the order is safe).
	if (taskbarProgress) {
		taskbarProgress->release();
		delete taskbarProgress;
		taskbarProgress = nullptr;
	}
#endif

	gameTimer->stop();

	closeGamePadConfWindow();

	// The closeApp function call stops all threads.
	// Calling quit on threads should not happen here. 
	//printf("Thread Finished: %i \n", emulatorThread->isFinished() );
	//emulatorThread->quit();
	//emulatorThread->wait( 1000 );

	//aviDiskThread->requestInterruption();
	//aviDiskThread->quit();
	//aviDiskThread->wait( 10000 );

	//FCEU_WRAPPER_LOCK();
	//fceuWrapperClose();
	//FCEU_WRAPPER_UNLOCK();

	unloadVideoDriver();

	delete mutex;

	// LoadGame() checks for an IP and if it finds one begins a network session
	// clear the NetworkIP field so this doesn't happen unintentionally
	//g_config->setOption ("SDL.NetworkIP", "");
	//g_config->save ();

	// Clear Clipboard Contents on Program Exit
	clipboard = QGuiApplication::clipboard();

	if ( clipboard->ownsClipboard() )
	{
		clipboard->clear( QClipboard::Clipboard );
	}
	if ( clipboard->ownsSelection() )
	{
		clipboard->clear( QClipboard::Selection );
	}

	clearRomList();

	if ( this == consoleWindow )
	{
		consoleWindow = NULL;
	}

}


void consoleWin_t::setMenuAccessPauseEnable( bool enable )
{
	mainMenuPauseWhenActv = enable;
}

void consoleWin_t::setContextMenuEnable( bool enable )
{
	contextMenuEnable = enable;
}

void consoleWin_t::setSoundUseGlobalFocus( bool enable )
{
	soundUseGlobalFocus = enable;

	winActiveChanged();
}


void consoleWin_t::resizeEvent(QResizeEvent *event)
{
	if ( firstResize )
	{
		// We are assuming that window has been exposed and all sizing of menu is finished
		// Restore minimum sizes to 1x values after first resize event so that
		// window is still able to be shrunk by dragging lower corners.
		if (viewport_Interface)
		{
			viewport_Interface->setMinimumSize( QSize( 256, 224 ) );
		}

		firstResize = false;
	}
	//printf("%i x %i \n", event->size().width(), event->size().height() );
}

void consoleWin_t::setCyclePeriodms( int ms )
{
	// If timer is already running, it will be restarted.
	gameTimer->start( ms );
   
	//printf("Period Set to: %i ms \n", ms );
}

void consoleWin_t::showErrorMsgWindow()
{
	QMessageBox msgBox(this);

	FCEU_WRAPPER_LOCK();
	msgBox.resize( this->size() );
	msgBox.setIcon( QMessageBox::Critical );
	msgBox.setText( tr(errorMsg.c_str()) );
	errorMsg.clear();
	FCEU_WRAPPER_UNLOCK();
	//msgBox.show();
	msgBox.exec();
}

void consoleWin_t::QueueErrorMsgWindow( const char *msg )
{
	errorMsg.append( msg );
	errorMsg.append("\n");
	errorMsgValid = true;
}

void consoleWin_t::closeEvent(QCloseEvent *event)
{
	//printf("Main Window Close Event\n");
	closeGamePadConfWindow();

	event->accept();

	closeApp();
}

void consoleWin_t::requestClose(void)
{
	closeRequested = true;
}

void consoleWin_t::keyPressEvent(QKeyEvent *event)
{
	//printf("Key Press: 0x%x \n", event->key() );

	// v0.3.15 PR-B: forward to the base class first so that QInputMethodEvent
	// (Chinese IME composition state) reaches the focused QLineEdit / QInputDialog
	// before we route the key to the emulator game key state.
	QMainWindow::keyPressEvent(event);

	pushKeyEvent( event, 1 );

	event->accept();
}

void consoleWin_t::keyReleaseEvent(QKeyEvent *event)
{
	//printf("Key Release: 0x%x \n", event->key() );

	// v0.3.15 PR-B: same IME forwarding rationale as keyPressEvent above.
	QMainWindow::keyReleaseEvent(event);

	pushKeyEvent( event, 0 );

	event->accept();
}

void consoleWin_t::dragEnterEvent(QDragEnterEvent *event)
{
	if (event->mimeData()->hasUrls() )
	{
		event->acceptProposedAction();
	}
}

void consoleWin_t::dropEvent(QDropEvent *event)
{
	if (event->mimeData()->hasUrls() )
	{
		QList<QUrl> urls = event->mimeData()->urls();

		QString filename = urls[0].toString( QUrl::PreferLocalFile );

		QFileInfo fi( filename );
		QString suffix = fi.suffix();

		bool isStateSaveFile = (suffix.size() == 3) && 
						(suffix[0] == 'f') && (suffix[1] == 'c') &&
							( (suffix[2] == 's') || suffix[2].isDigit() );

		//printf("DragNDrop Suffix: %s\n", suffix.toStdString().c_str() );

		if (isStateSaveFile)
		{
			FCEU_WRAPPER_LOCK();
			fceu11::LoadStateFile( filename.toStdString().c_str() );
			FCEU_WRAPPER_UNLOCK();

			event->accept();
		}
		else if ( suffix.compare("lua", Qt::CaseInsensitive) == 0 )
		{
			int luaLoadSuccess;

			FCEU_WRAPPER_LOCK();
			luaLoadSuccess = FCEU_LoadLuaCode( filename.toStdString().c_str() );
			FCEU_WRAPPER_UNLOCK();

			if (luaLoadSuccess)
			{
				g_config->setOption("SDL.LastLoadLua", filename.toStdString().c_str());
			}
			event->accept();
		}
		else
		{
			int romLoadSuccess;

			FCEU_WRAPPER_LOCK();
			romLoadSuccess = LoadGame( filename.toStdString().c_str() );
			FCEU_WRAPPER_UNLOCK();

			if (!romLoadSuccess)
			{
				printf("DragNDrop ROM Load Failed for %s\n", filename.toStdString().c_str() );
			}
			event->accept();
		}
	}
}

void consoleWin_t::showEvent(QShowEvent *event)
{
	//printf("Main Window Show Event\n");
	initScreenHandler();
}

void consoleWin_t::contextMenuEvent(QContextMenuEvent *event)
{
	QAction *act;
	QMenu menu(this);

	if ( !contextMenuEnable )
	{
		return;
	}

	act = new QAction(tr("Open ROM"), &menu);
	connect( act, SIGNAL(triggered(void)), this, SLOT(openROMFile(void)) );

	menu.addAction( act );

	act = new QAction(tr("Last ROM Used"), &menu);
	act->setEnabled( romList.size() > 0 );
	connect( act, SIGNAL(triggered(void)), this, SLOT(loadMostRecentROM(void)) );

	menu.addAction( act );

	menu.addSeparator();

	act = new QAction(tr("Online Help"), &menu);
	connect( act, SIGNAL(triggered(void)), this, SLOT(openOnlineDocs(void)) );

	menu.addAction( act );

	menu.addSeparator();

	act = new QAction(tr("Disable Context Menu via Options -> GUI Config"), &menu);
	connect( act, SIGNAL(triggered(void)), this, SLOT(openGuiConfWin(void)) );

	menu.addAction( act );

	menu.addSeparator();

	menu.exec(event->globalPos());

	event->accept();
}
//---------------------------------------------------------------------------

void consoleWin_t::transferVideoBuffer(void)
{
	FCEU_PROFILE_FUNC(prof, "VideoXfer");
	if ( nes_shm->blitUpdated.load(std::memory_order_acquire) )
	{
		nes_shm->blitUpdated.store(0, std::memory_order_relaxed);

		if (viewport_Interface)
		{
			viewport_Interface->transfer2LocalBuffer();
			viewport_Interface->queueRedraw();
		}
	}
}

void consoleWin_t::emuFrameFinish(void)
{
	static bool eventProcessingInProg = false;

	if ( eventProcessingInProg )
	{   // Prevent recursion as processEvents function can double back on us
		return;
	}
	eventProcessingInProg = true;
	// Process all events before attempting to render viewport
	QCoreApplication::processEvents();

	eventProcessingInProg = false;

	// Update Input Devices
	FCEUD_UpdateInput();
	
	//printf("EMU Frame Finish\n");

	transferVideoBuffer();
}

void consoleWin_t::updatePeriodic(void)
{
	FCEU_PROFILE_FUNC(prof, "updatePeriodic");
	static bool eventProcessingInProg = false;

	if ( eventProcessingInProg )
	{   // Prevent recursion as processEvents function can double back on us
		return;
	}
	eventProcessingInProg = true;
	// Process all events before attempting to render viewport
	QCoreApplication::processEvents();

	eventProcessingInProg = false;

	// Update Input Devices
	FCEUD_UpdateInput();
	
	// RePaint Game Viewport
	transferVideoBuffer();

	// Low Rate Updates
	if ( (updateCounter % 30) == 0 )
	{
		// Keep region menu selection sync'd to actual state
		int actRegion = fceu11::GetRegion();

		if ( !region[ actRegion ]->isChecked() )
		{
			region[ actRegion ]->setChecked(true);
		}

		powerAct->setEnabled( FCEU_IsValidUI( FCEUI_POWER ) );
		resetAct->setEnabled( FCEU_IsValidUI( FCEUI_RESET ) );
		sresetAct->setEnabled( FCEU_IsValidUI( FCEUI_RESET ) );
		playMovBeginAct->setEnabled( FCEU_IsValidUI( FCEUI_PLAYFROMBEGINNING ) );
		insCoinAct->setEnabled( FCEU_IsValidUI( FCEUI_INSERT_COIN ) );
		fdsSwitchAct->setEnabled( FCEU_IsValidUI( FCEUI_SWITCH_DISK ) );
		fdsEjectAct->setEnabled( FCEU_IsValidUI( FCEUI_EJECT_DISK ) );
		stopMovAct->setEnabled( FCEU_IsValidUI( FCEUI_STOPMOVIE ) );
		recentRomMenu->setEnabled( !recentRomMenu->isEmpty() );
		quickLoadAct->setEnabled( FCEU_IsValidUI( FCEUI_QUICKLOAD ) );
		quickSaveAct->setEnabled( FCEU_IsValidUI( FCEUI_QUICKSAVE ) );
		loadStateAct->setEnabled( FCEU_IsValidUI( FCEUI_LOADSTATE ) );
		saveStateAct->setEnabled( FCEU_IsValidUI( FCEUI_SAVESTATE ) );
		openMovAct->setEnabled( FCEU_IsValidUI( FCEUI_PLAYMOVIE ) );
		recMovAct->setEnabled( FCEU_IsValidUI( FCEUI_RECORDMOVIE ) );
		recAviAct->setEnabled( FCEU_IsValidUI( FCEUI_RECORDMOVIE ) && !FCEU_IsValidUI( FCEUI_STOPAVI ) );
		recAsAviAct->setEnabled( FCEU_IsValidUI( FCEUI_RECORDMOVIE ) && !FCEU_IsValidUI( FCEUI_STOPAVI ) );
		stopAviAct->setEnabled( FCEU_IsValidUI( FCEUI_STOPAVI ) );
		recWavAct->setEnabled( FCEU_IsValidUI( FCEUI_RECORDMOVIE ) && !fceu11::WaveRecordRunning() );
		recAsWavAct->setEnabled( FCEU_IsValidUI( FCEUI_RECORDMOVIE ) && !fceu11::WaveRecordRunning() );
		stopWavAct->setEnabled( fceu11::WaveRecordRunning() );
		tasEditorAct->setEnabled( FCEU_IsValidUI(FCEUI_TASEDITOR) );
	}

	if ( errorMsgValid )
	{
		showErrorMsgWindow();
		errorMsgValid = false;
	}

	if ( recentRomMenuReset )
	{
		FCEU_WRAPPER_LOCK();
		buildRecentRomMenu();
		recentRomMenuReset = false;
		FCEU_WRAPPER_UNLOCK();
	}

	if ( closeRequested )
	{
		closeApp();
		closeRequested = false;
	}

	updateCounter++;

#ifdef __FCEU_PROFILER_ENABLE__
		FCEU_profiler_log_thread_activity();
#endif
   return;
}

//-----------------------------------------------------------------------------
// Custom QMenuBar for Console
//-----------------------------------------------------------------------------
//-----------------------------------------------------------------------------

void consoleWin_t::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QMainWindow::changeEvent(event);
}

void consoleWin_t::retranslateUi(void)
{
	setWindowTitle(tr(FCEU_NAME_AND_VERSION));

	if (fileMenu) fileMenu->setTitle(tr("&File"));
	if (optMenu) optMenu->setTitle(tr("&Options"));
	if (emuMenu) emuMenu->setTitle(tr("&Emulation"));
	if (advMenu) advMenu->setTitle(tr("&Advanced"));
	if (advEmuMenu) advEmuMenu->setTitle(tr("&Emulation"));
	if (advMovieMenu) advMovieMenu->setTitle(tr("&Movie"));
	if (advDebugMenu) advDebugMenu->setTitle(tr("&Debug"));
	if (advMemoryMenu) advMemoryMenu->setTitle(tr("&Memory Tools"));
	if (advMiscMenu) advMiscMenu->setTitle(tr("&Misc Tools"));
	if (advSettingsMenu) advSettingsMenu->setTitle(tr("&Advanced Settings"));
	if (helpMenu) helpMenu->setTitle(tr("&Help"));
	if (languageMenu) languageMenu->setTitle(tr("&Language"));
	if (changeStateMenu) changeStateMenu->setTitle(tr("Change &State Slot"));
	if (windowResizeMenu) windowResizeMenu->setTitle(tr("Window Resi&ze"));
	if (regionMenu) regionMenu->setTitle(tr("&Region"));
	if (speedMenu) speedMenu->setTitle(tr("&Speed"));
	if (autoFireMenu) autoFireMenu->setTitle(tr("&AutoFire Pattern"));
	if (fdsMenu) fdsMenu->setTitle(tr("&FDS"));
	if (ramInitMenu) ramInitMenu->setTitle(tr("&RAM Init"));

	if (openROM) openROM->setText(tr("&Open ROM"));
	if (closeROM) closeROM->setText(tr("&Close ROM"));
	if (recentRomMenu) recentRomMenu->setTitle(tr("&Recent ROMs"));
	if (playNSF) playNSF->setText(tr("Play &NSF"));
	if (loadStateAct) loadStateAct->setText(tr("Load State &From"));
	if (saveStateAct) saveStateAct->setText(tr("Save State &As"));
	if (quickLoadAct) quickLoadAct->setText(tr("Quick &Load"));
	if (quickSaveAct) quickSaveAct->setText(tr("Quick &Save"));
	if (loadLuaAct) loadLuaAct->setText(tr("Load &Lua Script"));
	if (scrShotAct) scrShotAct->setText(tr("Screens&hot"));
	if (quitAct) quitAct->setText(tr("&Quit"));

	if (inputConfig) inputConfig->setText(tr("&Input Config"));
	if (gamePadConfig) gamePadConfig->setText(tr("&GamePad Config"));
	if (gameSoundConfig) gameSoundConfig->setText(tr("&Sound Config"));
	if (gameVideoConfig) gameVideoConfig->setText(tr("&Video Config"));
	if (hotkeyConfig) hotkeyConfig->setText(tr("Hot&Key Config"));
	if (paletteConfig) paletteConfig->setText(tr("&Palette Config"));
	if (guiConfig) guiConfig->setText(tr("G&UI Config"));
	if (timingConfig) timingConfig->setText(tr("&Timing Config"));
	if (stateRecordConfig) stateRecordConfig->setText(tr("&State Recorder Config"));
	if (movieConfig) movieConfig->setText(tr("&Movie Options"));
	if (autoResume) autoResume->setText(tr("Auto-&Resume Play"));
	if (fullscreen) fullscreen->setText(tr("&Fullscreen"));
	if (hideMenuAct) hideMenuAct->setText(tr("&Hide Menu"));
	if (autoHideMenuAct) autoHideMenuAct->setText(tr("&Auto Hide Menu on Fullscreen"));
	if (useBgPaletteAct) useBgPaletteAct->setText(tr("Use BG Palette for Video BG Color"));
	if (bgColorMenuItem) bgColorMenuItem->setText(tr("BG Side Panel Color"));

	if (powerAct) powerAct->setText(tr("&Power"));
	if (resetAct) resetAct->setText(tr("Hard &Reset"));
	if (sresetAct) sresetAct->setText(tr("&Soft Reset"));
	if (pauseAct) pauseAct->setText(tr("&Pause"));
	if (gameGenieAct) gameGenieAct->setText(tr("Enable Game &Genie"));
	if (loadGgROMAct) loadGgROMAct->setText(tr("Load Game Genie ROM"));
	if (insCoinAct) insCoinAct->setText(tr("&Insert Coin"));
	if (fdsSwitchAct) fdsSwitchAct->setText(tr("&Switch Disk"));
	if (fdsEjectAct) fdsEjectAct->setText(tr("&Eject Disk"));
	if (fdsLoadBiosAct) fdsLoadBiosAct->setText(tr("&Load BIOS"));
	if (virtualFkbAct) virtualFkbAct->setText(tr("Virtual Family Keyboard"));

	if (cheatsAct) cheatsAct->setText(tr("&Cheats..."));
	if (ramSearchAct) ramSearchAct->setText(tr("RAM &Search..."));
	if (ramWatchAct) ramWatchAct->setText(tr("RAM &Watch..."));
	if (tasEditorAct) tasEditorAct->setText(tr("&TAS Editor ..."));

	if (debuggerAct) debuggerAct->setText(tr("&Debugger..."));
	if (hexEditAct) hexEditAct->setText(tr("&Hex Editor..."));
	if (ppuViewAct) ppuViewAct->setText(tr("&PPU Viewer..."));
	if (oamViewAct) oamViewAct->setText(tr("&Sprite Viewer..."));
	if (ntViewAct) ntViewAct->setText(tr("&Name Table Viewer..."));
	if (traceLogAct) traceLogAct->setText(tr("&Trace Logger..."));
	if (codeDataLogAct) codeDataLogAct->setText(tr("&Code/Data Logger..."));
	if (ggEncodeAct) ggEncodeAct->setText(tr("&Game Genie Encode/Decode"));
	if (iNesEditAct) iNesEditAct->setText(tr("NES Header Edito&r..."));
	if (frameTimingAct) frameTimingAct->setText(tr("&Frame Timing ..."));
	if (paletteEditorAct) paletteEditorAct->setText(tr("&Palette Editor ..."));
	if (aviRiffViewerAct) aviRiffViewerAct->setText(tr("&AVI RIFF Viewer ..."));

	if (openMovAct) openMovAct->setText(tr("Movie &Play"));
	if (playMovBeginAct) playMovBeginAct->setText(tr("Movie Play From &Beginning"));
	if (stopMovAct) stopMovAct->setText(tr("Movie &Stop"));
	if (recMovAct) recMovAct->setText(tr("Movie &Record"));
	if (recAviAct) recAviAct->setText(tr("AVI &Record"));
	if (recAsAviAct) recAsAviAct->setText(tr("AVI Record &As"));
	if (stopAviAct) stopAviAct->setText(tr("AVI &Stop"));
	if (recWavAct) recWavAct->setText(tr("WAV &Record"));
	if (recAsWavAct) recAsWavAct->setText(tr("WAV Record &As"));
	if (stopWavAct) stopWavAct->setText(tr("WAV &Stop"));

	for (int i = 0; i < 10; i++)
	{
		if (state[i])
		{
			char stmp[8];
			snprintf(stmp, sizeof(stmp), "Slot &%i", i);
			state[i]->setText(tr(stmp));
		}
	}

	for (int i = 0; i < 4; i++)
	{
		if (winSizeAct[i])
		{
			char stmp[8];
			snprintf(stmp, sizeof(stmp), "&%ix", i + 1);
			winSizeAct[i]->setText(tr(stmp));
		}
	}

	if (region[0]) region[0]->setText(tr("&NTSC"));
	if (region[1]) region[1]->setText(tr("&PAL"));
	if (region[2]) region[2]->setText(tr("&Dendy"));

	if (ramInit[0]) ramInit[0]->setText(tr("&Default"));
	if (ramInit[1]) ramInit[1]->setText(tr("Fill $&FF"));
	if (ramInit[2]) ramInit[2]->setText(tr("Fill $&00"));
	if (ramInit[3]) ramInit[3]->setText(tr("&Random"));

	if (speedUpAct) speedUpAct->setText(tr("Speed &Up"));
	if (slowDownAct) slowDownAct->setText(tr("Slow &Down"));
	if (slowestSpdAct) slowestSpdAct->setText(tr("&Slowest"));
	if (normalSpdAct) normalSpdAct->setText(tr("&Normal"));
	if (turboSpdAct) turboSpdAct->setText(tr("&Turbo"));
	if (customSpdAct) customSpdAct->setText(tr("&Custom"));
	if (frameAdvDelayAct) frameAdvDelayAct->setText(tr("Set Frame &Advance Delay"));
	if (setCustomAutoFireAct) setCustomAutoFireAct->setText(tr("Set Custom Pattern"));

	if (languageActionGroup)
	{
		for (auto action : languageActionGroup->actions())
		{
			QString code = action->data().toString();
			if (code == QStringLiteral("en"))
				action->setText(tr("English"));
			else if (code == QStringLiteral("zh_CN"))
				action->setText(tr("Simplified Chinese"));
			else if (code == QStringLiteral("zh_TW"))
				action->setText(tr("Traditional Chinese"));
			else if (code == QStringLiteral("ja"))
				action->setText(tr("Japanese"));
			else if (code == QStringLiteral("ko"))
				action->setText(tr("Korean"));
			else if (code == QStringLiteral("es"))
				action->setText(tr("Spanish"));
			else if (code == QStringLiteral("fr"))
				action->setText(tr("French"));
			else if (code == QStringLiteral("de"))
				action->setText(tr("German"));
			else if (code == QStringLiteral("vi"))
				action->setText(tr("Vietnamese"));
			else if (code == QStringLiteral("th"))
				action->setText(tr("Thai"));
			else if (code == QStringLiteral("hi"))
				action->setText(tr("Hindi (beta)"));
			else if (code == QStringLiteral("ar"))
				action->setText(tr("Arabic (beta)"));
		}
	}

	for (auto afAct : afActList)
	{
		if (afAct)
		{
			char stmp[64];
			snprintf(stmp, sizeof(stmp), "%i On, %i Off", afAct->getOnValue(), afAct->getOffValue());
			afAct->setText(tr(stmp));
		}
	}
	if (afActCustom)
	{
		afActCustom->setText(tr("Custom"));
	}

	if (aboutAct) aboutAct->setText(tr("&About FCEUX11"));
	if (aboutActQt) aboutActQt->setText(tr("About &Qt"));
	if (msgLogAct) msgLogAct->setText(tr("&Message Log"));
}
//-----------------------------------------------------------------------------

