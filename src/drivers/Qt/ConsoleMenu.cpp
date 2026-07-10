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
#include "Qt/nes_shm.h"
#include "Qt/TasEditor/TasEditorWindow.h"


#include "Qt/ConsoleMenu.h"
#include "Qt/ConsoleWindow.h"

void consoleWin_t::createMainMenu(void)
{
	QAction *act;
	QMenu *subMenu;
	QActionGroup *group;
	int useNativeMenuBar;
	int customAutofireOnFrames, customAutofireOffFrames;
	//QShortcut *shortcut;

	menubar = new consoleMenuBar(this);

	this->setMenuBar(menubar);

	// This is needed for menu bar to show up on MacOS
	g_config->getOption( "SDL.UseNativeMenuBar", &useNativeMenuBar );

	menubar->setNativeMenuBar( useNativeMenuBar ? true : false );

	// Top Level Menu Iterms
	// v0.3.15 PR-A: 5+1 audience-tiered model.
	// The "Advanced" menu is a new top-level collector that gathers the
	// formerly-flat Tools / Debug / Movie menus. Hidden by SDL.HideAdvancedMenu.
	fileMenu  = menubar->addMenu(tr("&File"));
	optMenu   = menubar->addMenu(tr("&Options"));
	emuMenu   = menubar->addMenu(tr("&Emulation"));
	advMenu   = menubar->addMenu(tr("&Advanced"));
	helpMenu  = menubar->addMenu(tr("&Help"));

	// Five sub-menus under Advanced.
	advEmuMenu     = advMenu->addMenu(tr("&Emulation"));
	advMovieMenu   = advMenu->addMenu(tr("&Movie"));
	advDebugMenu   = advMenu->addMenu(tr("&Debug"));
	advMemoryMenu  = advMenu->addMenu(tr("&Memory Tools"));
	advMiscMenu    = advMenu->addMenu(tr("&Misc Tools"));
	advSettingsMenu = advMenu->addMenu(tr("&Advanced Settings"));

	// v0.3.15 PR-A: read SDL.HideAdvancedMenu (default off) and hide the
	// Advanced top-level menu entirely if requested.
	g_config->getOption("SDL.HideAdvancedMenu", &hideAdvancedMenu);
	if (hideAdvancedMenu)
	{
		advMenu->menuAction()->setVisible(false);
	}

	//-----------------------------------------------------------------------
	// File
	
	connect( fileMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( fileMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// File -> Open ROM
	openROM = new QAction(tr("&Open ROM"), this);
	//openROM->setShortcuts(QKeySequence::Open);
	openROM->setStatusTip(tr("Open ROM File"));
	//openROM->setIcon( QIcon(":icons/rom.png") );
	//openROM->setIcon( style->standardIcon( QStyle::SP_FileIcon ) );
	openROM->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(openROM, SIGNAL(triggered()), this, SLOT(openROMFile(void)) );

	Hotkeys[ HK_OPEN_ROM ].setAction( openROM );
	connect( Hotkeys[ HK_OPEN_ROM ].getShortcut(), SIGNAL(activated()), this, SLOT(openROMFile(void)) );
	
	fileMenu->addAction(openROM);

	// File -> Close ROM
	closeROM = new QAction(tr("&Close ROM"), this);
	//closeROM->setShortcut( QKeySequence(tr("Ctrl+C")));
	closeROM->setStatusTip(tr("Close Loaded ROM"));
	closeROM->setIcon( style()->standardIcon( QStyle::SP_BrowserStop ) );
	connect(closeROM, SIGNAL(triggered()), this, SLOT(closeROMCB(void)) );
	
	Hotkeys[ HK_CLOSE_ROM ].setAction( closeROM );
	connect( Hotkeys[ HK_CLOSE_ROM ].getShortcut(), SIGNAL(activated()), this, SLOT(closeROMCB(void)) );
	
	fileMenu->addAction(closeROM);
	
	// File -> Recent ROMs
	recentRomMenu = fileMenu->addMenu( tr("&Recent ROMs") );

	buildRecentRomMenu();

	fileMenu->addSeparator();

	// File -> Play NSF
	playNSF = new QAction(tr("Play &NSF"), this);
	//playNSF->setShortcut( QKeySequence(tr("Ctrl+N")));
	playNSF->setStatusTip(tr("Play NSF"));
	connect(playNSF, SIGNAL(triggered()), this, SLOT(loadNSF(void)) );
	
	fileMenu->addAction(playNSF);
	
	fileMenu->addSeparator();

	// File -> Load State From
	loadStateAct = new QAction(tr("Load State &From"), this);
	//loadStateAct->setShortcut( QKeySequence(tr("Ctrl+N")));
	loadStateAct->setStatusTip(tr("Load State From"));
	loadStateAct->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(loadStateAct, SIGNAL(triggered()), this, SLOT(loadStateFrom(void)) );
	
	fileMenu->addAction(loadStateAct);

	// File -> Save State As
	saveStateAct = new QAction(tr("Save State &As"), this);
	//loadStateAct->setShortcut( QKeySequence(tr("Ctrl+N")));
	saveStateAct->setStatusTip(tr("Save State As"));
	saveStateAct->setIcon( style()->standardIcon( QStyle::SP_DialogSaveButton ) );
	connect(saveStateAct, SIGNAL(triggered()), this, SLOT(saveStateAs(void)) );
	
	fileMenu->addAction(saveStateAct);

	// File -> Quick Load
	quickLoadAct = new QAction(tr("Quick &Load"), this);
	//quickLoadAct->setShortcut( QKeySequence(tr("Shift+I")));
	quickLoadAct->setStatusTip(tr("Quick Load"));
	connect(quickLoadAct, SIGNAL(triggered()), this, SLOT(quickLoad(void)) );
	
	fileMenu->addAction(quickLoadAct);

	Hotkeys[ HK_LOAD_STATE ].setAction( quickLoadAct );
	connect( Hotkeys[ HK_LOAD_STATE ].getShortcut(), SIGNAL(activated()), this, SLOT(quickLoad(void)) );
	
	// File -> Quick Save
	quickSaveAct = new QAction(tr("Quick &Save"), this);
	//quickSaveAct->setShortcut( QKeySequence(tr("F5")));
	quickSaveAct->setStatusTip(tr("Quick Save"));
	connect(quickSaveAct, SIGNAL(triggered()), this, SLOT(quickSave(void)) );
	
	fileMenu->addAction(quickSaveAct);

	Hotkeys[ HK_SAVE_STATE ].setAction( quickSaveAct );
	connect( Hotkeys[ HK_SAVE_STATE ].getShortcut(), SIGNAL(activated()), this, SLOT(quickSave(void)) );
	
	// File -> Change State Slot
	changeStateMenu = fileMenu->addMenu(tr("Change &State Slot"));
	group   = new QActionGroup(this);

	group->setExclusive(true);

	for (int i=0; i<10; i++)
	{
	        char stmp[8];

	        snprintf( stmp, sizeof(stmp), "Slot &%i", i );

	        state[i] = new QAction(tr(stmp), this);
	        state[i]->setCheckable(true);

	        group->addAction(state[i]);
		changeStateMenu->addAction(state[i]);
	}
	state[0]->setChecked(true);

	connect(state[0], SIGNAL(triggered()), this, SLOT(changeState0(void)) );
	connect(state[1], SIGNAL(triggered()), this, SLOT(changeState1(void)) );
	connect(state[2], SIGNAL(triggered()), this, SLOT(changeState2(void)) );
	connect(state[3], SIGNAL(triggered()), this, SLOT(changeState3(void)) );
	connect(state[4], SIGNAL(triggered()), this, SLOT(changeState4(void)) );
	connect(state[5], SIGNAL(triggered()), this, SLOT(changeState5(void)) );
	connect(state[6], SIGNAL(triggered()), this, SLOT(changeState6(void)) );
	connect(state[7], SIGNAL(triggered()), this, SLOT(changeState7(void)) );
	connect(state[8], SIGNAL(triggered()), this, SLOT(changeState8(void)) );
	connect(state[9], SIGNAL(triggered()), this, SLOT(changeState9(void)) );
	
	fileMenu->addSeparator();

	Hotkeys[ HK_SELECT_STATE_0 ].setAction( state[0] );
	Hotkeys[ HK_SELECT_STATE_1 ].setAction( state[1] );
	Hotkeys[ HK_SELECT_STATE_2 ].setAction( state[2] );
	Hotkeys[ HK_SELECT_STATE_3 ].setAction( state[3] );
	Hotkeys[ HK_SELECT_STATE_4 ].setAction( state[4] );
	Hotkeys[ HK_SELECT_STATE_5 ].setAction( state[5] );
	Hotkeys[ HK_SELECT_STATE_6 ].setAction( state[6] );
	Hotkeys[ HK_SELECT_STATE_7 ].setAction( state[7] );
	Hotkeys[ HK_SELECT_STATE_8 ].setAction( state[8] );
	Hotkeys[ HK_SELECT_STATE_9 ].setAction( state[9] );

	connect( Hotkeys[ HK_SELECT_STATE_0 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState0(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_1 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState1(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_2 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState2(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_3 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState3(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_4 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState4(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_5 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState5(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_6 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState6(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_7 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState7(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_8 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState8(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_9 ].getShortcut(), SIGNAL(activated()), this, SLOT(changeState9(void)) );
	
	connect( Hotkeys[ HK_SELECT_STATE_PREV ].getShortcut(), SIGNAL(activated()), this, SLOT(decrementState(void)) );
	connect( Hotkeys[ HK_SELECT_STATE_NEXT ].getShortcut(), SIGNAL(activated()), this, SLOT(incrementState(void)) );

#ifdef _S9XLUA_H
	// File -> Quick Save
	loadLuaAct = new QAction(tr("Load &Lua Script"), this);
	//loadLuaAct->setShortcut( QKeySequence(tr("F5")));
	loadLuaAct->setStatusTip(tr("Load Lua Script"));
	//loadLuaAct->setIcon( QIcon(":icons/lua-logo.png") );
	connect(loadLuaAct, SIGNAL(triggered()), this, SLOT(loadLua(void)) );
	
	fileMenu->addAction(loadLuaAct);
	
	fileMenu->addSeparator();
#else
	loadLuaAct = NULL;
#endif

	// File -> Screenshot
	scrShotAct = new QAction(tr("Screens&hot"), this);
	//scrShotAct->setShortcut( QKeySequence(tr("F12")));
	scrShotAct->setStatusTip(tr("Screenshot"));
	scrShotAct->setIcon( QIcon(":icons/camera.png") );
	connect(scrShotAct, SIGNAL(triggered()), this, SLOT(prepareScreenShot(void)));
	
	fileMenu->addAction(scrShotAct);

	Hotkeys[ HK_SCREENSHOT ].setAction( scrShotAct );
	connect( Hotkeys[ HK_SCREENSHOT ].getShortcut(), SIGNAL(activated()), this, SLOT(takeScreenShot(void)) );

	// File -> Quit
	quitAct = new QAction(tr("&Quit"), this);
	//quitAct->setShortcut( QKeySequence(tr("Ctrl+Q")));
	quitAct->setStatusTip(tr("Quit the Application"));
	//quitAct->setIcon( style()->standardIcon( QStyle::SP_DialogCloseButton ) );
	quitAct->setIcon( QIcon(":icons/application-exit.png") );
	connect(quitAct, SIGNAL(triggered()), this, SLOT(closeApp()));
	
	fileMenu->addAction(quitAct);

	Hotkeys[ HK_QUIT ].setAction( quitAct );
	connect( Hotkeys[ HK_QUIT ].getShortcut(), SIGNAL(activated()), this, SLOT(closeApp(void)) );

	//-----------------------------------------------------------------------
	// Options

	connect( optMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( optMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// v0.3.15 PR-A: Sound / Video / GUI / Language / Window / Fullscreen
	// / Hide Menu / Auto-Hide-Menu / BG Color stay in the top-level
	// "Options" menu because they are the most common daily-use settings.
	// The advanced config dialogs (Input / GamePad / HotKey / Palette /
	// Timing / State Recorder / Movie / Auto-Resume) are collected in
	// the "Advanced -> Advanced Settings" sub-menu (built later).
	//
	// v0.3.15.x PHASE-5: Input Config is also promoted to the top of
	// the Options menu (as its first action) so it is one click away
	// from the menu bar without having to drill through Advanced.
	// The same QAction is reused — the same QAction can live in
	// multiple menus at once — so the entry in
	// Advanced -> Advanced Settings remains accessible.

	// Options -> Input Config (first row)
	inputConfig = new QAction(tr("&Input Config"), this);
	inputConfig->setStatusTip(tr("Input Configure"));
	inputConfig->setIcon( QIcon(":icons/input-gaming.png") );
	connect(inputConfig, SIGNAL(triggered()), this, SLOT(openInputConfWin(void)) );

	optMenu->addAction(inputConfig);

	// Options -> Sound Config
	gameSoundConfig = new QAction(tr("&Sound Config"), this);
	//gameSoundConfig->setShortcut( QKeySequence(tr("Ctrl+C")));
	gameSoundConfig->setStatusTip(tr("Sound Configure"));
	gameSoundConfig->setIcon( style()->standardIcon( QStyle::SP_MediaVolume ) );
	connect(gameSoundConfig, SIGNAL(triggered()), this, SLOT(openGameSndConfWin(void)) );

	optMenu->addAction(gameSoundConfig);

	// Options -> Video Config
	gameVideoConfig = new QAction(tr("&Video Config"), this);
	//gameVideoConfig->setShortcut( QKeySequence(tr("Ctrl+C")));
	gameVideoConfig->setStatusTip(tr("Video Preferences"));
	gameVideoConfig->setIcon( style()->standardIcon( QStyle::SP_ComputerIcon ) );
	connect(gameVideoConfig, SIGNAL(triggered()), this, SLOT(openGameVideoConfWin(void)) );

	optMenu->addAction(gameVideoConfig);

	// Options -> GUI Config
	guiConfig = new QAction(tr("G&UI Config"), this);
	//guiConfig->setShortcut( QKeySequence(tr("Ctrl+C")));
	guiConfig->setStatusTip(tr("GUI Configure"));
	guiConfig->setIcon( style()->standardIcon( QStyle::SP_TitleBarNormalButton ) );
	connect(guiConfig, SIGNAL(triggered()), this, SLOT(openGuiConfWin(void)) );

	optMenu->addAction(guiConfig);

	// Options -> Language
	languageMenu = new QMenu(tr("&Language"), this);
	languageActionGroup = new QActionGroup(this);

	QAction *langEn = new QAction(tr("English"), languageActionGroup);
	langEn->setCheckable(true);
	langEn->setData("en");
	languageMenu->addAction(langEn);

	QAction *langZhCN = new QAction(tr("Simplified Chinese"), languageActionGroup);
	langZhCN->setCheckable(true);
	langZhCN->setData("zh_CN");
	languageMenu->addAction(langZhCN);

	QAction *langZhTW = new QAction(tr("Traditional Chinese"), languageActionGroup);
	langZhTW->setCheckable(true);
	langZhTW->setData("zh_TW");
	languageMenu->addAction(langZhTW);

	QAction *langJa = new QAction(tr("Japanese"), languageActionGroup);
	langJa->setCheckable(true);
	langJa->setData("ja");
	languageMenu->addAction(langJa);

	QAction *langKo = new QAction(tr("Korean"), languageActionGroup);
	langKo->setCheckable(true);
	langKo->setData("ko");
	languageMenu->addAction(langKo);

	QAction *langEs = new QAction(tr("Spanish"), languageActionGroup);
	langEs->setCheckable(true);
	langEs->setData("es");
	languageMenu->addAction(langEs);

	QAction *langFr = new QAction(tr("French"), languageActionGroup);
	langFr->setCheckable(true);
	langFr->setData("fr");
	languageMenu->addAction(langFr);

	QAction *langDe = new QAction(tr("German"), languageActionGroup);
	langDe->setCheckable(true);
	langDe->setData("de");
	languageMenu->addAction(langDe);

	QAction *langVi = new QAction(tr("Vietnamese"), languageActionGroup);
	langVi->setCheckable(true);
	langVi->setData("vi");
	languageMenu->addAction(langVi);

	QAction *langTh = new QAction(tr("Thai"), languageActionGroup);
	langTh->setCheckable(true);
	langTh->setData("th");
	languageMenu->addAction(langTh);

	QAction *langHi = new QAction(tr("Hindi (beta)"), languageActionGroup);
	langHi->setCheckable(true);
	langHi->setData("hi");
	languageMenu->addAction(langHi);

	QAction *langAr = new QAction(tr("Arabic (beta)"), languageActionGroup);
	langAr->setCheckable(true);
	langAr->setData("ar");
	languageMenu->addAction(langAr);

	connect(languageActionGroup, &QActionGroup::triggered, this, [this](QAction *action) {
		loadTranslation(action->data().toString());
	});
	optMenu->addMenu(languageMenu);

	optMenu->addSeparator();

	// Options -> Window Resize
	windowResizeMenu = optMenu->addMenu( tr("Window Resi&ze") );

	for (int i=0; i<4; i++)
	{
	        char stmp[8];

	        snprintf( stmp, sizeof(stmp), "&%ix", i+1 );

	        winSizeAct[i] = new QAction(tr(stmp), this);

		windowResizeMenu->addAction(winSizeAct[i]);

		connect( winSizeAct[i], &QAction::triggered, [ this, i ]{ consoleWin_t::winResizeIx(i+1); } );
	}

	// Options -> Full Screen
	fullscreen = new QAction(tr("&Fullscreen"), this);
	//fullscreen->setShortcut( QKeySequence(tr("Alt+Return")));
	fullscreen->setStatusTip(tr("Fullscreen"));
	fullscreen->setIcon( QIcon(":icons/view-fullscreen.png") );
	connect(fullscreen, SIGNAL(triggered()), this, SLOT(toggleFullscreen(void)) );
	
	optMenu->addAction(fullscreen);

	Hotkeys[ HK_FULLSCREEN ].setAction( fullscreen );
	connect( Hotkeys[ HK_FULLSCREEN ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleFullscreen(void)) );

	// Options -> Hide Menu Screen
	hideMenuAct = new QAction(tr("&Hide Menu"), this);
	//hideMenuAct->setShortcut( QKeySequence(tr("Alt+/")));
	hideMenuAct->setStatusTip(tr("Hide Menu"));
	hideMenuAct->setIcon( style()->standardIcon( QStyle::SP_TitleBarMaxButton ) );
	connect(hideMenuAct, SIGNAL(triggered()), this, SLOT(toggleMenuVis(void)) );
	
	optMenu->addAction(hideMenuAct);

	Hotkeys[ HK_MAIN_MENU_HIDE ].setAction( hideMenuAct );
	connect( Hotkeys[ HK_MAIN_MENU_HIDE ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleMenuVis(void)) );

	// Options -> Auto Hide Menu on Fullscreen
	g_config->getOption( "SDL.AutoHideMenuFullsreen", &autoHideMenuFullscreen );

	autoHideMenuAct = new QAction(tr("&Auto Hide Menu on Fullscreen"), this);
	//autoHideMenuAct->setShortcut( QKeySequence(tr("Alt+/")));
	autoHideMenuAct->setCheckable(true);
	autoHideMenuAct->setChecked( autoHideMenuFullscreen );
	autoHideMenuAct->setStatusTip(tr("Auto Hide Menu on Fullscreen"));
	//autoHideMenuAct->setIcon( style()->standardIcon( QStyle::SP_TitleBarMaxButton ) );
	connect(autoHideMenuAct, SIGNAL(triggered(bool)), this, SLOT(toggleMenuAutoHide(bool)) );

	optMenu->addAction(autoHideMenuAct);

	optMenu->addSeparator();

	// Options -> Video BG Color
	fceuLoadConfigColor( "SDL.VideoBgColor", &videoBgColor );

	bgColorMenuItem = new ColorMenuItem( tr("BG Side Panel Color"), "SDL.VideoBgColor", this );
	bgColorMenuItem->connectColor( &videoBgColor );

	optMenu->addAction(bgColorMenuItem);

	connect( bgColorMenuItem, SIGNAL(colorChanged(QColor&)), this, SLOT(videoBgColorChanged(QColor&)) );

	// Options -> Use BG Palette for Video BG Color
	g_config->getOption( "SDL.UseBgPaletteForVideo", &usePaletteForVideoBg );

	act = new QAction(tr("Use BG Palette for Video BG Color"), this);
	//act->setShortcut( QKeySequence(tr("Alt+/")));
	act->setCheckable(true);
	act->setChecked( usePaletteForVideoBg );
	act->setStatusTip(tr("Use BG Palette for Video BG Color"));
	useBgPaletteAct = act;
	//act->setIcon( style()->standardIcon( QStyle::SP_TitleBarMaxButton ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(toggleUseBgPaletteForVideo(bool)) );

	optMenu->addAction(act);

	bgColorMenuItem->setEnabled( !usePaletteForVideoBg );
	//-----------------------------------------------------------------------
	// Emulation

	connect( emuMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( emuMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Emulation -> Power
	powerAct = new QAction(tr("&Power"), this);
	//powerAct->setShortcut( QKeySequence(tr("Ctrl+P")));
	powerAct->setStatusTip(tr("Power On Console"));
	powerAct->setIcon( QIcon(":icons/power.png") );
	connect(powerAct, SIGNAL(triggered()), this, SLOT(powerConsoleCB(void)) );
	
	emuMenu->addAction(powerAct);

	Hotkeys[ HK_POWER ].setAction( powerAct );
	connect( Hotkeys[ HK_POWER ].getShortcut(), SIGNAL(activated()), this, SLOT(powerConsoleCB(void)) );

	// Emulation -> Reset
	resetAct = new QAction(tr("Hard &Reset"), this);
	//resetAct->setShortcut( QKeySequence(tr("Ctrl+R")));
	resetAct->setStatusTip(tr("Hard Reset of Console"));
	resetAct->setIcon( style()->standardIcon( QStyle::SP_DialogResetButton ) );
	connect(resetAct, SIGNAL(triggered()), this, SLOT(consoleHardReset(void)) );
	
	emuMenu->addAction(resetAct);

	Hotkeys[ HK_HARD_RESET ].setAction( resetAct );
	connect( Hotkeys[ HK_HARD_RESET ].getShortcut(), SIGNAL(activated()), this, SLOT(consoleHardReset(void)) );

	// v0.3.15 PR-A: Soft Reset / Game Genie / Virtual FKB / Insert Coin /
	// FDS / RAM Init moved to Advanced -> Emulation (see advEmuMenu below).
	// The sresetAct QAction* is created there.

	// Emulation -> Pause
	pauseAct = new QAction(tr("&Pause"), this);
	//pauseAct->setShortcut( QKeySequence(tr("Pause")));
	pauseAct->setStatusTip(tr("Pause Console"));
	pauseAct->setIcon( style()->standardIcon( QStyle::SP_MediaPause ) );
	connect(pauseAct, SIGNAL(triggered()), this, SLOT(consolePause(void)) );
	
	emuMenu->addAction(pauseAct);
	
	Hotkeys[ HK_PAUSE ].setAction( pauseAct );
	connect( Hotkeys[ HK_PAUSE ].getShortcut(), SIGNAL(activated()), this, SLOT(consolePause(void)) );

	emuMenu->addSeparator();

	// Emulation -> Region
	regionMenu = emuMenu->addMenu(tr("&Region"));
	group   = new QActionGroup(this);

	group->setExclusive(true);

	for (int i=0; i<3; i++)
	{
		const char *txt;

		if ( i == 1 )
		{
			txt = "&PAL";
		}
		else if ( i == 2 )
		{
			txt = "&Dendy";
		}
		else
		{
			txt = "&NTSC";
		}

	        region[i] = new QAction(tr(txt), this);
	        region[i]->setCheckable(true);

	        group->addAction(region[i]);
		regionMenu->addAction(region[i]);
	}
	region[ fceu11::GetRegion() ]->setChecked(true);

	connect( region[0], SIGNAL(triggered(void)), this, SLOT(setRegionNTSC(void)) );
	connect( region[1], SIGNAL(triggered(void)), this, SLOT(setRegionPAL(void)) );
	connect( region[2], SIGNAL(triggered(void)), this, SLOT(setRegionDendy(void)) );

	// v0.3.15 PR-A: RAM Init / Game Genie / Virtual FKB / Insert Coin /
	// FDS sub-menu moved to Advanced -> Emulation (see advEmuMenu below).
	// Top-level Emulation now keeps only basic user workflow: Power /
	// Hard Reset / Pause / Region / Speed / AutoFire.

	// Emulation -> Speed
	speedMenu = emuMenu->addMenu(tr("&Speed"));

	// Emulation -> Speed -> Speed Up
	speedUpAct = new QAction(tr("Speed &Up"), this);
	//speedUpAct->setShortcut( QKeySequence(tr("=")));
	speedUpAct->setStatusTip(tr("Speed Up"));
	speedUpAct->setIcon( style()->standardIcon( QStyle::SP_MediaSeekForward ) );
	connect(speedUpAct, SIGNAL(triggered()), this, SLOT(emuSpeedUp(void)) );
	
	Hotkeys[ HK_INCREASE_SPEED ].setAction( speedUpAct );
	connect( Hotkeys[ HK_INCREASE_SPEED ].getShortcut(), SIGNAL(activated()), this, SLOT(emuSpeedUp(void)) );

	speedMenu->addAction(speedUpAct);

	// Emulation -> Speed -> Slow Down
	slowDownAct = new QAction(tr("Slow &Down"), this);
	//slowDownAct->setShortcut( QKeySequence(tr("-")));
	slowDownAct->setStatusTip(tr("Slow Down"));
	slowDownAct->setIcon( style()->standardIcon( QStyle::SP_MediaSeekBackward ) );
	connect(slowDownAct, SIGNAL(triggered()), this, SLOT(emuSlowDown(void)) );
	
	Hotkeys[ HK_DECREASE_SPEED ].setAction( slowDownAct );
	connect( Hotkeys[ HK_DECREASE_SPEED ].getShortcut(), SIGNAL(activated()), this, SLOT(emuSlowDown(void)) );

	speedMenu->addAction(slowDownAct);
	
	speedMenu->addSeparator();

	// Emulation -> Speed -> Slowest Speed
	slowestSpdAct = new QAction(tr("&Slowest"), this);
	//slowestSpdAct->setShortcut( QKeySequence(tr("-")));
	slowestSpdAct->setStatusTip(tr("Slowest"));
	slowestSpdAct->setIcon( style()->standardIcon( QStyle::SP_MediaSkipBackward ) );
	connect(slowestSpdAct, SIGNAL(triggered()), this, SLOT(emuSlowestSpd(void)) );
	
	speedMenu->addAction(slowestSpdAct);

	// Emulation -> Speed -> Normal Speed
	normalSpdAct = new QAction(tr("&Normal"), this);
	//normalSpdAct->setShortcut( QKeySequence(tr("-")));
	normalSpdAct->setStatusTip(tr("Normal"));
	normalSpdAct->setIcon( style()->standardIcon( QStyle::SP_MediaPlay ) );
	connect(normalSpdAct, SIGNAL(triggered()), this, SLOT(emuNormalSpd(void)) );
	
	speedMenu->addAction(normalSpdAct);
	
	// Emulation -> Speed -> Fastest Speed
	turboSpdAct = new QAction(tr("&Turbo"), this);
	//turboSpdAct->setShortcut( QKeySequence(tr("-")));
	turboSpdAct->setStatusTip(tr("Turbo (Fastest)"));
	turboSpdAct->setIcon( style()->standardIcon( QStyle::SP_MediaSkipForward ) );
	connect(turboSpdAct, SIGNAL(triggered()), this, SLOT(emuFastestSpd(void)) );
	
	speedMenu->addAction(turboSpdAct);
	
	// Emulation -> Speed -> Custom Speed
	customSpdAct = new QAction(tr("&Custom"), this);
	//customSpdAct->setShortcut( QKeySequence(tr("-")));
	customSpdAct->setStatusTip(tr("Custom"));
	connect(customSpdAct, SIGNAL(triggered()), this, SLOT(emuCustomSpd(void)) );
	
	speedMenu->addAction(customSpdAct);
	
	speedMenu->addSeparator();
	
	// Emulation -> Speed -> Set Frame Advance Delay
	frameAdvDelayAct = new QAction(tr("Set Frame &Advance Delay"), this);
	//frameAdvDelayAct->setShortcut( QKeySequence(tr("-")));
	frameAdvDelayAct->setStatusTip(tr("Set Frame Advance Delay"));
	connect(frameAdvDelayAct, SIGNAL(triggered()), this, SLOT(emuSetFrameAdvDelay(void)) );
	
	speedMenu->addAction(frameAdvDelayAct);

	emuMenu->addSeparator();

	// Emulation -> AutoFire Pattern
	autoFireMenu = emuMenu->addMenu(tr("&AutoFire Pattern"));
	
	group   = new QActionGroup(this);
	group->setExclusive(true);

	for (int i=1; i<6; i++)
	{
		char stmp[64];

		for (int j=1; j<=(6-i); j++)
		{
			snprintf( stmp, sizeof(stmp), "%i On, %i Off", i, j );
			autoFireMenuAction *afAct = new autoFireMenuAction( i, j, tr(stmp), this);
			afAct->setCheckable(true);
			group->addAction(afAct);
			autoFireMenu->addAction(afAct);
			afActList.push_back(afAct);

			connect( afAct, SIGNAL(triggered(void)), afAct, SLOT(activateCB(void)) );
		}
	}

	g_config->getOption("SDL.AutofireCustomOnFrames"  , &customAutofireOnFrames );
	g_config->getOption("SDL.AutofireCustomOffFrames" , &customAutofireOffFrames);

	afActCustom = new autoFireMenuAction( customAutofireOnFrames, customAutofireOffFrames, tr("Custom"), this);
	afActCustom->setCheckable(true);
	group->addAction(afActCustom);
	autoFireMenu->addAction(afActCustom);
	//afActList.push_back(afAct);

	connect( afActCustom, SIGNAL(triggered(void)), afActCustom, SLOT(activateCB(void)) );

	autoFireMenu->addSeparator();

	syncAutoFirePatternMenu();

	// Emulation -> AutoFire Pattern -> Set Custom Pattern
	setCustomAutoFireAct = new QAction(tr("Set Custom Pattern"), this);
	setCustomAutoFireAct->setStatusTip(tr("Set Custom Pattern"));
	connect(setCustomAutoFireAct, SIGNAL(triggered()), this, SLOT(setCustomAutoFire(void)) );

	autoFireMenu->addAction(setCustomAutoFireAct);

	//-----------------------------------------------------------------------
	// Advanced -> Emulation  (was: Emulation -> soft reset / GG / FKB /
	//                                   VS coin / FDS / RAM Init / AutoFire)
	// v0.3.15 PR-A: most of the deep-emulation knobs move to Advanced.
	// The top-level Emulation menu keeps Power / Hard Reset / Pause /
	// Region / Speed (basic user workflow).
	//-----------------------------------------------------------------------

	connect( advEmuMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( advEmuMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Advanced -> Emulation -> Soft Reset
	sresetAct = new QAction(tr("&Soft Reset"), this);
	//sresetAct->setShortcut( QKeySequence(tr("Ctrl+R")));
	sresetAct->setStatusTip(tr("Soft Reset of Console"));
	sresetAct->setIcon( style()->standardIcon( QStyle::SP_BrowserReload ) );
	connect(sresetAct, SIGNAL(triggered()), this, SLOT(consoleSoftReset(void)) );

	advEmuMenu->addAction(sresetAct);

	Hotkeys[ HK_SOFT_RESET ].setAction( sresetAct );
	connect( Hotkeys[ HK_SOFT_RESET ].getShortcut(), SIGNAL(activated()), this, SLOT(consoleSoftReset(void)) );

	advEmuMenu->addSeparator();

	// Advanced -> Emulation -> Enable Game Genie
	gameGenieAct = new QAction(tr("Enable Game &Genie"), this);
	//gameGenieAct->setShortcut( QKeySequence(tr("Ctrl+G")));
	gameGenieAct->setCheckable(true);
	gameGenieAct->setStatusTip(tr("Enable Game Genie"));
	connect(gameGenieAct, SIGNAL(triggered(bool)), this, SLOT(toggleGameGenie(bool)) );

	syncActionConfig( gameGenieAct, "SDL.GameGenie" );

	advEmuMenu->addAction(gameGenieAct);

	// Advanced -> Emulation -> Load Game Genie ROM
	loadGgROMAct = new QAction(tr("Load Game Genie ROM"), this);
	//loadGgROMAct->setShortcut( QKeySequence(tr("Ctrl+G")));
	loadGgROMAct->setStatusTip(tr("Load Game Genie ROM"));
	connect(loadGgROMAct, SIGNAL(triggered()), this, SLOT(loadGameGenieROM(void)) );

	advEmuMenu->addAction(loadGgROMAct);

	advEmuMenu->addSeparator();

	// Advanced -> Emulation -> Virtual Family Keyboard
	virtualFkbAct = new QAction(tr("Virtual Family Keyboard"), this);
	//virtualFkbAct->setShortcut( QKeySequence(tr("Ctrl+G")));
	virtualFkbAct->setStatusTip(tr("Virtual Family Keyboard"));
	connect(virtualFkbAct, SIGNAL(triggered()), this, SLOT(openFamilyKeyboard(void)) );

	advEmuMenu->addAction(virtualFkbAct);

	advEmuMenu->addSeparator();

	// Advanced -> Emulation -> Insert Coin (VS System)
	insCoinAct = new QAction(tr("&Insert Coin"), this);
	//insCoinAct->setShortcut( QKeySequence(tr("Ctrl+G")));
	insCoinAct->setStatusTip(tr("Insert Coin"));
	connect(insCoinAct, SIGNAL(triggered()), this, SLOT(insertCoin(void)) );

	advEmuMenu->addAction(insCoinAct);

	Hotkeys[ HK_VS_INSERT_COIN ].setAction( insCoinAct );
	connect( Hotkeys[ HK_VS_INSERT_COIN ].getShortcut(), SIGNAL(activated()), this, SLOT(insertCoin(void)) );

	advEmuMenu->addSeparator();

	// Advanced -> Emulation -> FDS sub-menu
	fdsMenu = advEmuMenu->addMenu(tr("&FDS"));

	fdsSwitchAct = new QAction(tr("&Switch Disk"), this);
	fdsSwitchAct->setStatusTip(tr("Switch Disk"));
	connect(fdsSwitchAct, SIGNAL(triggered()), this, SLOT(fdsSwitchDisk(void)) );

	Hotkeys[ HK_FDS_SELECT ].setAction( fdsSwitchAct );
	connect( Hotkeys[ HK_FDS_SELECT ].getShortcut(), SIGNAL(activated()), this, SLOT(fdsSwitchDisk(void)) );

	fdsMenu->addAction(fdsSwitchAct);

	fdsEjectAct = new QAction(tr("&Eject Disk"), this);
	fdsEjectAct->setStatusTip(tr("Eject Disk"));
	connect(fdsEjectAct, SIGNAL(triggered()), this, SLOT(fdsEjectDisk(void)) );

	fdsMenu->addAction(fdsEjectAct);

	fdsLoadBiosAct = new QAction(tr("&Load BIOS"), this);
	fdsLoadBiosAct->setStatusTip(tr("Load FDS BIOS"));
	connect(fdsLoadBiosAct, SIGNAL(triggered()), this, SLOT(fdsLoadBIOS(void)) );

	fdsMenu->addAction(fdsLoadBiosAct);

	advEmuMenu->addSeparator();

	// Advanced -> Emulation -> RAM Init
	ramInitMenu = advEmuMenu->addMenu(tr("&RAM Init"));
	group   = new QActionGroup(this);
	group->setExclusive(true);

	for (int i=0; i<4; i++)
	{
		const char *txt;
		switch (i)
		{
			default:
			case 0: txt = "&Default";  break;
			case 1: txt = "Fill $&FF"; break;
			case 2: txt = "Fill $&00"; break;
			case 3: txt = "&Random";   break;
		}
		ramInit[i] = new QAction(tr(txt), this);
		ramInit[i]->setCheckable(true);
		group->addAction(ramInit[i]);
		ramInitMenu->addAction(ramInit[i]);
	}
	g_config->getOption ("SDL.RamInitMethod", &RAMInitOption);
	ramInit[ RAMInitOption ]->setChecked(true);

	connect( ramInit[0], SIGNAL(triggered(void)), this, SLOT(setRamInit0(void)) );
	connect( ramInit[1], SIGNAL(triggered(void)), this, SLOT(setRamInit1(void)) );
	connect( ramInit[2], SIGNAL(triggered(void)), this, SLOT(setRamInit2(void)) );
	connect( ramInit[3], SIGNAL(triggered(void)), this, SLOT(setRamInit3(void)) );

	//-----------------------------------------------------------------------
	// Advanced -> Advanced Settings  (was: Options -> Input / GamePad /
	//                                              HotKey / Palette /
	//                                              Timing / State Recorder /
	//                                              Movie / Auto-Resume)
	//-----------------------------------------------------------------------

	connect( advSettingsMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( advSettingsMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Advanced -> Advanced Settings -> Input Config
	// v0.3.15.x PHASE-5: inputConfig is now created in the top-level
	// Options menu (its first row) — re-use the same QAction here so
	// the dialog is also reachable from Advanced without duplicating
	// the trigger wiring.
	if (inputConfig) {
		advSettingsMenu->addAction(inputConfig);
	}

	// Advanced -> Advanced Settings -> GamePad Config
	gamePadConfig = new QAction(tr("&GamePad Config"), this);
	gamePadConfig->setStatusTip(tr("GamePad Configure"));
	gamePadConfig->setIcon( QIcon(":icons/input-gaming-symbolic.png") );
	connect(gamePadConfig, SIGNAL(triggered()), this, SLOT(openGamePadConfWin(void)) );

	advSettingsMenu->addAction(gamePadConfig);

	// Advanced -> Advanced Settings -> HotKey Config
	hotkeyConfig = new QAction(tr("Hot&Key Config"), this);
	hotkeyConfig->setStatusTip(tr("Hotkey Configure"));
	hotkeyConfig->setIcon( QIcon(":icons/input-keyboard.png") );
	connect(hotkeyConfig, SIGNAL(triggered()), this, SLOT(openHotkeyConfWin(void)) );

	advSettingsMenu->addAction(hotkeyConfig);

	// Advanced -> Advanced Settings -> Palette Config
	paletteConfig = new QAction(tr("&Palette Config"), this);
	paletteConfig->setStatusTip(tr("Palette Configure"));
	paletteConfig->setIcon( QIcon(":icons/graphics-palette.png") );
	connect(paletteConfig, SIGNAL(triggered()), this, SLOT(openPaletteConfWin(void)) );

	advSettingsMenu->addAction(paletteConfig);

	// Advanced -> Advanced Settings -> Timing Config
	timingConfig = new QAction(tr("&Timing Config"), this);
	timingConfig->setStatusTip(tr("Timing Configure"));
	timingConfig->setIcon( QIcon(":icons/timer.png") );
	connect(timingConfig, SIGNAL(triggered()), this, SLOT(openTimingConfWin(void)) );

	advSettingsMenu->addAction(timingConfig);

	// Advanced -> Advanced Settings -> State Recorder Config
	stateRecordConfig = new QAction(tr("&State Recorder Config"), this);
	stateRecordConfig->setStatusTip(tr("State Recorder Configure"));
	stateRecordConfig->setIcon( QIcon(":icons/media-record.png") );
	connect(stateRecordConfig, SIGNAL(triggered()), this, SLOT(openStateRecorderConfWin(void)) );

	advSettingsMenu->addAction(stateRecordConfig);

	// Advanced -> Advanced Settings -> Movie Options
	movieConfig = new QAction(tr("&Movie Options"), this);
	movieConfig->setStatusTip(tr("Movie Options"));
	movieConfig->setIcon( QIcon(":icons/movie.png") );
	connect(movieConfig, SIGNAL(triggered()), this, SLOT(openMovieOptWin(void)) );

	advSettingsMenu->addAction(movieConfig);

	advSettingsMenu->addSeparator();

	// Advanced -> Advanced Settings -> Auto-Resume
	autoResume = new QAction(tr("Auto-&Resume Play"), this);
	autoResume->setCheckable(true);
	autoResume->setStatusTip(tr("Auto-Resume Play"));
	syncActionConfig( autoResume, "SDL.AutoResume" );
	connect(autoResume, SIGNAL(triggered()), this, SLOT(toggleAutoResume(void)) );

	advSettingsMenu->addAction(autoResume);

	//-----------------------------------------------------------------------
	// Advanced -> Memory Tools  (was: Tools -> Cheats / RAM Search / RAM Watch)
	//-----------------------------------------------------------------------

	connect( advMemoryMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( advMemoryMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Advanced -> Memory Tools -> Cheats
	cheatsAct = new QAction(tr("&Cheats..."), this);
	//cheatsAct->setShortcut( QKeySequence(tr("Shift+F7")));
	cheatsAct->setStatusTip(tr("Open Cheat Window"));
	connect(cheatsAct, SIGNAL(triggered()), this, SLOT(openCheats(void)) );

	Hotkeys[ HK_CHEAT_MENU ].setAction( cheatsAct );
	connect( Hotkeys[ HK_CHEAT_MENU ].getShortcut(), SIGNAL(activated()), this, SLOT(openCheats(void)) );

	advMemoryMenu->addAction(cheatsAct);

	// Advanced -> Memory Tools -> RAM Search
	ramSearchAct = new QAction(tr("RAM &Search..."), this);
	//ramSearchAct->setShortcut( QKeySequence(tr("Shift+F7")));
	ramSearchAct->setStatusTip(tr("Open RAM Search Window"));
	connect(ramSearchAct, SIGNAL(triggered()), this, SLOT(openRamSearch(void)) );

	advMemoryMenu->addAction(ramSearchAct);

	// Advanced -> Memory Tools -> RAM Watch
	ramWatchAct = new QAction(tr("RAM &Watch..."), this);
	//ramWatchAct->setShortcut( QKeySequence(tr("Shift+F7")));
	ramWatchAct->setStatusTip(tr("Open RAM Watch Window"));
	connect(ramWatchAct, SIGNAL(triggered()), this, SLOT(openRamWatch(void)) );

	advMemoryMenu->addAction(ramWatchAct);

	//-----------------------------------------------------------------------
	// Advanced -> Misc Tools  (was: Tools -> Frame Timing / Palette Editor /
	//                                   AVI RIFF Viewer / TAS Editor)
	//-----------------------------------------------------------------------

	connect( advMiscMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( advMiscMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Advanced -> Misc Tools -> Frame Timing
	frameTimingAct = new QAction(tr("&Frame Timing ..."), this);
	//frameTimingAct->setShortcut( QKeySequence(tr("Shift+F7")));
	frameTimingAct->setStatusTip(tr("Open Frame Timing Window"));
	connect(frameTimingAct, SIGNAL(triggered()), this, SLOT(openTimingStatWin(void)) );

	advMiscMenu->addAction(frameTimingAct);

	// Advanced -> Misc Tools -> Palette Editor
	paletteEditorAct = new QAction(tr("&Palette Editor ..."), this);
	//paletteEditorAct->setShortcut( QKeySequence(tr("Shift+F7")));
	paletteEditorAct->setStatusTip(tr("Open Palette Editor Window"));
	connect(paletteEditorAct, SIGNAL(triggered()), this, SLOT(openPaletteEditorWin(void)) );

	advMiscMenu->addAction(paletteEditorAct);

	// Advanced -> Misc Tools -> AVI RIFF Viewer
	aviRiffViewerAct = new QAction(tr("&AVI RIFF Viewer ..."), this);
	//aviRiffViewerAct->setShortcut( QKeySequence(tr("Shift+F7")));
	aviRiffViewerAct->setStatusTip(tr("Open AVI RIFF Viewer Window"));
	connect(aviRiffViewerAct, SIGNAL(triggered()), this, SLOT(openAviRiffViewer(void)) );

	advMiscMenu->addAction(aviRiffViewerAct);

	// Advanced -> Misc Tools -> TAS Editor
	tasEditorAct = act = new QAction(tr("&TAS Editor ..."), this);
	//act->setShortcut( QKeySequence(tr("Shift+F7")));
	act->setStatusTip(tr("Open TAS Editor Window"));
	connect(act, SIGNAL(triggered()), this, SLOT(openTasEditor(void)) );

	advMiscMenu->addAction(act);

	 //-----------------------------------------------------------------------
	 // Advanced -> Debug (was: Debug)
	 //-----------------------------------------------------------------------

	connect( advDebugMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( advDebugMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Advanced -> Debug -> Debugger
	debuggerAct = new QAction(tr("&Debugger..."), this);
	//debuggerAct->setShortcut( QKeySequence(tr("Shift+F7")));
	debuggerAct->setStatusTip(tr("Open 6502 Debugger"));
	connect(debuggerAct, SIGNAL(triggered()), this, SLOT(openDebugWindow(void)) );

	advDebugMenu->addAction(debuggerAct);

	// Advanced -> Debug -> Hex Editor
	hexEditAct = new QAction(tr("&Hex Editor..."), this);
	//hexEditAct->setShortcut( QKeySequence(tr("Shift+F7")));
	hexEditAct->setStatusTip(tr("Open Memory Hex Editor"));
	connect(hexEditAct, SIGNAL(triggered()), this, SLOT(openHexEditor(void)) );

	advDebugMenu->addAction(hexEditAct);

	// Advanced -> Debug -> PPU Viewer
	ppuViewAct = new QAction(tr("&PPU Viewer..."), this);
	//ppuViewAct->setShortcut( QKeySequence(tr("Shift+F7")));
	ppuViewAct->setStatusTip(tr("Open PPU Viewer"));
	connect(ppuViewAct, SIGNAL(triggered()), this, SLOT(openPPUViewer(void)) );

	advDebugMenu->addAction(ppuViewAct);

	// Advanced -> Debug -> Sprite Viewer
	oamViewAct = new QAction(tr("&Sprite Viewer..."), this);
	//oamViewAct->setShortcut( QKeySequence(tr("Shift+F7")));
	oamViewAct->setStatusTip(tr("Open Sprite Viewer"));
	connect(oamViewAct, SIGNAL(triggered()), this, SLOT(openOAMViewer(void)) );

	advDebugMenu->addAction(oamViewAct);

	// Advanced -> Debug -> Name Table Viewer
	ntViewAct = new QAction(tr("&Name Table Viewer..."), this);
	//ntViewAct->setShortcut( QKeySequence(tr("Shift+F7")));
	ntViewAct->setStatusTip(tr("Open Name Table Viewer"));
	connect(ntViewAct, SIGNAL(triggered()), this, SLOT(openNTViewer(void)) );

	advDebugMenu->addAction(ntViewAct);

	// Advanced -> Debug -> Trace Logger
	traceLogAct = new QAction(tr("&Trace Logger..."), this);
	//traceLogAct->setShortcut( QKeySequence(tr("Shift+F7")));
	traceLogAct->setStatusTip(tr("Open Trace Logger"));
	connect(traceLogAct, SIGNAL(triggered()), this, SLOT(openTraceLogger(void)) );

	advDebugMenu->addAction(traceLogAct);

	// Advanced -> Debug -> Code/Data Logger
	codeDataLogAct = new QAction(tr("&Code/Data Logger..."), this);
	//codeDataLogAct->setShortcut( QKeySequence(tr("Shift+F7")));
	codeDataLogAct->setStatusTip(tr("Open Code Data Logger"));
	connect(codeDataLogAct, SIGNAL(triggered()), this, SLOT(openCodeDataLogger(void)) );

	advDebugMenu->addAction(codeDataLogAct);

	// Advanced -> Debug -> Game Genie Encode/Decode Viewer
	ggEncodeAct = new QAction(tr("&Game Genie Encode/Decode"), this);
	//ggEncodeAct->setShortcut( QKeySequence(tr("Shift+F7")));
	ggEncodeAct->setStatusTip(tr("Open Game Genie Encode/Decode"));
	connect(ggEncodeAct, SIGNAL(triggered()), this, SLOT(openGGEncoder(void)) );

	advDebugMenu->addAction(ggEncodeAct);

	// Advanced -> Debug -> NES Header Editor
	iNesEditAct = new QAction(tr("NES Header Edito&r..."), this);
	//iNesEditAct->setShortcut( QKeySequence(tr("Shift+F7")));
	iNesEditAct->setStatusTip(tr("Open NES Header Editor"));
	connect(iNesEditAct, SIGNAL(triggered()), this, SLOT(openNesHeaderEditor(void)) );

	advDebugMenu->addAction(iNesEditAct);

	//-----------------------------------------------------------------------
	// Advanced -> Movie  (was: top-level Movie menu)
	//-----------------------------------------------------------------------

	connect( advMovieMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( advMovieMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Advanced -> Movie -> Play
	openMovAct = new QAction(tr("Movie &Play"), this);
	//openMovAct->setShortcut( QKeySequence(tr("Shift+F7")));
	openMovAct->setStatusTip(tr("Play Movie File"));
	openMovAct->setIcon( style()->standardIcon( QStyle::SP_MediaPlay ) );
	connect(openMovAct, SIGNAL(triggered()), this, SLOT(openMovie(void)) );

	Hotkeys[ HK_PLAY_MOVIE_FROM ].setAction( openMovAct );
	connect( Hotkeys[ HK_PLAY_MOVIE_FROM ].getShortcut(), SIGNAL(activated()), this, SLOT(openMovie(void)) );

	advMovieMenu->addAction(openMovAct);

	// Advanced -> Movie -> Play From Beginning
	playMovBeginAct = new QAction(tr("Movie Play From &Beginning"), this);
	//playMovBeginAct->setShortcut( QKeySequence(tr("Shift+F7")));
	playMovBeginAct->setStatusTip(tr("Play Movie From Beginning"));
	//playMovBeginAct->setIcon( style()->standardIcon( QStyle::SP_MediaPlay ) );
	connect(playMovBeginAct, SIGNAL(triggered()), this, SLOT(playMovieFromBeginning(void)) );

	Hotkeys[ HK_MOVIE_PLAY_RESTART ].setAction( playMovBeginAct );
	connect( Hotkeys[ HK_MOVIE_PLAY_RESTART ].getShortcut(), SIGNAL(activated()), this, SLOT(playMovieFromBeginning(void)) );

	advMovieMenu->addAction(playMovBeginAct);

	// Advanced -> Movie -> Stop
	stopMovAct = new QAction(tr("Movie &Stop"), this);
	//stopMovAct->setShortcut( QKeySequence(tr("Shift+F7")));
	stopMovAct->setStatusTip(tr("Stop Movie Recording"));
	stopMovAct->setIcon( style()->standardIcon( QStyle::SP_MediaStop ) );
	connect(stopMovAct, SIGNAL(triggered()), this, SLOT(stopMovie(void)) );

	Hotkeys[ HK_STOP_MOVIE ].setAction( stopMovAct );
	connect( Hotkeys[ HK_STOP_MOVIE ].getShortcut(), SIGNAL(activated()), this, SLOT(stopMovie(void)) );

	advMovieMenu->addAction(stopMovAct);

	advMovieMenu->addSeparator();

	// Advanced -> Movie -> Record
	recMovAct = new QAction(tr("Movie &Record"), this);
	//recMovAct->setShortcut( QKeySequence(tr("Shift+F5")));
	recMovAct->setStatusTip(tr("Record Movie"));
	recMovAct->setIcon( QIcon(":icons/media-record.png") );
	connect(recMovAct, SIGNAL(triggered()), this, SLOT(recordMovie(void)) );

	Hotkeys[ HK_RECORD_MOVIE_TO ].setAction( recMovAct );
	connect( Hotkeys[ HK_RECORD_MOVIE_TO ].getShortcut(), SIGNAL(activated()), this, SLOT(recordMovie(void)) );

	advMovieMenu->addAction(recMovAct);

	advMovieMenu->addSeparator();

	// Advanced -> Movie -> Avi Recording

	// Advanced -> Movie -> Avi Recording -> Record
	recAviAct = new QAction(tr("AVI &Record"), this);
	//recAviAct->setShortcut( QKeySequence(tr("Shift+F5")));
	recAviAct->setStatusTip(tr("AVI Record Start"));
	recAviAct->setIcon( QIcon(":icons/media-record.png") );
	connect(recAviAct, SIGNAL(triggered()), this, SLOT(aviRecordStart(void)) );

	Hotkeys[ HK_RECORD_AVI ].setAction( recAviAct );
	connect( Hotkeys[ HK_RECORD_AVI ].getShortcut(), SIGNAL(activated()), this, SLOT(aviRecordStart(void)) );

	advMovieMenu->addAction(recAviAct);

	// Advanced -> Movie -> Avi Recording -> Record As
	recAsAviAct = new QAction(tr("AVI Record &As"), this);
	//recAsAviAct->setShortcut( QKeySequence(tr("Shift+F5")));
	recAsAviAct->setStatusTip(tr("AVI Record As Start"));
	//recAsAviAct->setIcon( QIcon(":icons/media-record.png") );
	connect(recAsAviAct, SIGNAL(triggered()), this, SLOT(aviRecordAsStart(void)) );

	Hotkeys[ HK_RECORD_AVI_TO ].setAction( recAsAviAct );
	connect( Hotkeys[ HK_RECORD_AVI_TO ].getShortcut(), SIGNAL(activated()), this, SLOT(aviRecordAsStart(void)) );

	advMovieMenu->addAction(recAsAviAct);

	// Advanced -> Movie -> Avi Recording -> Stop
	stopAviAct = new QAction(tr("AVI &Stop"), this);
	//stopAviAct->setShortcut( QKeySequence(tr("Shift+F5")));
	stopAviAct->setStatusTip(tr("AVI Record Stop"));
	stopAviAct->setIcon( style()->standardIcon( QStyle::SP_MediaStop ) );
	connect(stopAviAct, SIGNAL(triggered()), this, SLOT(aviRecordStop(void)) );

	Hotkeys[ HK_STOP_AVI ].setAction( stopAviAct );
	connect( Hotkeys[ HK_STOP_AVI ].getShortcut(), SIGNAL(activated()), this, SLOT(aviRecordStop(void)) );

	advMovieMenu->addAction(stopAviAct);

	advMovieMenu->addSeparator();

	// Advanced -> Movie -> WAV Recording

	// Advanced -> Movie -> WAV Recording -> Record
	recWavAct = new QAction(tr("WAV &Record"), this);
	//recWavAct->setShortcut( QKeySequence(tr("Shift+F5")));
	recWavAct->setStatusTip(tr("WAV Record Start"));
	recWavAct->setIcon( QIcon(":icons/media-record.png") );
	connect(recWavAct, SIGNAL(triggered()), this, SLOT(wavRecordStart(void)) );

	Hotkeys[ HK_RECORD_WAV ].setAction( recWavAct );
	connect( Hotkeys[ HK_RECORD_WAV ].getShortcut(), SIGNAL(activated()), this, SLOT(wavRecordStart(void)) );

	advMovieMenu->addAction(recWavAct);

	// Advanced -> Movie -> WAV Recording -> Record As
	recAsWavAct = new QAction(tr("WAV Record &As"), this);
	//recAsWavAct->setShortcut( QKeySequence(tr("Shift+F5")));
	recAsWavAct->setStatusTip(tr("WAV Record As Start"));
	//recAsWavAct->setIcon( QIcon(":icons/media-record.png") );
	connect(recAsWavAct, SIGNAL(triggered()), this, SLOT(wavRecordAsStart(void)) );

	Hotkeys[ HK_RECORD_WAV_TO ].setAction( recAsWavAct );
	connect( Hotkeys[ HK_RECORD_WAV_TO ].getShortcut(), SIGNAL(activated()), this, SLOT(wavRecordAsStart(void)) );

	advMovieMenu->addAction(recAsWavAct);

	// Advanced -> Movie -> WAV Recording -> Stop
	stopWavAct = new QAction(tr("WAV &Stop"), this);
	//stopWavAct->setShortcut( QKeySequence(tr("Shift+F5")));
	stopWavAct->setStatusTip(tr("WAV Record Stop"));
	stopWavAct->setIcon( style()->standardIcon( QStyle::SP_MediaStop ) );
	connect(stopWavAct, SIGNAL(triggered()), this, SLOT(wavRecordStop(void)) );

	Hotkeys[ HK_STOP_WAV ].setAction( stopWavAct );
	connect( Hotkeys[ HK_STOP_WAV ].getShortcut(), SIGNAL(activated()), this, SLOT(wavRecordStop(void)) );

	advMovieMenu->addAction(stopWavAct);

	//-----------------------------------------------------------------------
	// Help
 
	connect( helpMenu, SIGNAL(aboutToShow(void)), this, SLOT(mainMenuOpen(void)) );
	connect( helpMenu, SIGNAL(aboutToHide(void)), this, SLOT(mainMenuClose(void)) );

	// Help -> About FCEUX11
	aboutAct = new QAction(tr("&About FCEUX11"), this);
	aboutAct->setStatusTip(tr("About FCEUX11"));
	aboutAct->setIcon( style()->standardIcon( QStyle::SP_MessageBoxInformation ) );
	connect(aboutAct, SIGNAL(triggered()), this, SLOT(aboutFCEUX(void)) );
	
	helpMenu->addAction(aboutAct);

	// Help -> About Qt
	aboutActQt = new QAction(tr("About &Qt"), this);
	aboutActQt->setStatusTip(tr("About Qt"));
	aboutActQt->setIcon( style()->standardIcon( QStyle::SP_TitleBarMenuButton ) );
	connect(aboutActQt, SIGNAL(triggered()), this, SLOT(aboutQt(void)) );
	
	helpMenu->addAction(aboutActQt);

	// Help -> Message Log
	msgLogAct = new QAction(tr("&Message Log"), this);
	msgLogAct->setStatusTip(tr("Message Log"));
	msgLogAct->setIcon( style()->standardIcon( QStyle::SP_MessageBoxWarning ) );
	connect(msgLogAct, SIGNAL(triggered()), this, SLOT(openMsgLogWin(void)) );
	
	helpMenu->addAction(msgLogAct);

	// Load saved language preference (must be after all actions are created)
	// v0.3.15.x PHASE-4: TypedConfig<QString> replaces bare QSettings.
	QString savedLang;
	{
		static const fceu11::qt::TypedConfig<QString> kLanguage(
			"General/Language", QStringLiteral("en"));
		savedLang = kLanguage.get();
	}
	// v1.11 §11.5: auto-detect system language on first launch (no saved preference)
	if (savedLang.isEmpty() || savedLang == QStringLiteral("en")) {
		QString sysLocale = QLocale::system().name();
		if (sysLocale.startsWith(QLatin1String("zh_CN")) || sysLocale.startsWith(QLatin1String("zh_Hans")))
			savedLang = QStringLiteral("zh_CN");
		else if (sysLocale.startsWith(QLatin1String("zh_TW")) || sysLocale.startsWith(QLatin1String("zh_Hant")))
			savedLang = QStringLiteral("zh_TW");
		else if (sysLocale.startsWith(QLatin1String("ja")))
			savedLang = QStringLiteral("ja");
		else if (sysLocale.startsWith(QLatin1String("ko")))
			savedLang = QStringLiteral("ko");
		else if (sysLocale.startsWith(QLatin1String("es")))
			savedLang = QStringLiteral("es");
		else if (sysLocale.startsWith(QLatin1String("fr")))
			savedLang = QStringLiteral("fr");
		else if (sysLocale.startsWith(QLatin1String("de")))
			savedLang = QStringLiteral("de");
		else if (sysLocale.startsWith(QLatin1String("vi")))
			savedLang = QStringLiteral("vi");
		else if (sysLocale.startsWith(QLatin1String("th")))
			savedLang = QStringLiteral("th");
		else if (sysLocale.startsWith(QLatin1String("hi")))
			savedLang = QStringLiteral("hi");
		else if (sysLocale.startsWith(QLatin1String("ar")))
			savedLang = QStringLiteral("ar");
	}
	loadTranslation(savedLang);
};
