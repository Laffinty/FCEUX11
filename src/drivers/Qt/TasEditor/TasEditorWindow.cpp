/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2021 mjbudd77
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
// TasEditorWindow.cpp
//
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <string>
#include <zlib.h>

#include <QDir>
#include <QDrag>
#include <QString>
#include <QPainter>
#include <QSettings>
#include <QTextEdit>
#include <QHeaderView>
#include <QMessageBox>
#include <QFontMetrics>
#include <QFileDialog>
#include <QFontDialog>
#include <QInputDialog>
#include <QStandardPaths>
#include <QActionGroup>
#include <QApplication>
#include <QGuiApplication>
#include <QDesktopServices>

#include "fceu.h"
#include "movie.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"

#include "common/vidblit.h"
#include "Qt/config.h"
#include "Qt/keyscan.h"
#include "Qt/throttle.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ColorMenu.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/TasEditor/TasColors.h"
#include "Qt/TasEditor/TasEditorWindow.h"

TasEditorWindow   *tasWin = NULL;
TASEDITOR_PROJECT *project = NULL;
TASEDITOR_CONFIG  *taseditorConfig = NULL;
TASEDITOR_LUA     *taseditor_lua = NULL;
MARKERS_MANAGER   *markersManager = NULL;
SELECTION         *selection = NULL;
GREENZONE         *greenzone = NULL;
BOOKMARKS         *bookmarks = NULL;
BRANCHES          *branches = NULL;
PLAYBACK          *playback = NULL;
RECORDER          *recorder = NULL;
HISTORY           *history = NULL;
SPLICER           *splicer = NULL;

char pianoRollSaveID[PIANO_ROLL_ID_LEN] = "PIANO_ROLL";
char pianoRollSkipSaveID[PIANO_ROLL_ID_LEN] = "PIANO_ROLX";
TasFindNoteWindow *findWin = NULL;
uint64_t tasEditorTimeStamp = 0;

//----------------------------------------------------------------------------
//----  Main TAS Editor Window
//----------------------------------------------------------------------------
bool tasWindowIsOpen(void)
{
	return tasWin != NULL;
}
//----------------------------------------------------------------------------
void tasWindowSetFocus(bool val)
{
	if ( tasWin )
	{
		tasWin->activateWindow();
		tasWin->raise();
	}
}
// this getter contains formula to decide whether to record or replay movie
bool isTaseditorRecording(void)
{
	if ( tasWin == NULL )
	{
		return false;
	}
	if (movie_readonly || playback->getPauseFrame() >= 0 || (taseditorConfig->oldControlSchemeForBranching && !recorder->stateWasLoadedInReadWriteMode))
	{
		return false;		// replay
	}
	return true;			// record
}

uint64_t getTasEditorTime(void)
{
	return tasEditorTimeStamp;
}

void recordInputByTaseditor(void)
{
	if ( recorder )
	{
		recorder->recordInput();
	}
	return;
}

void applyMovieInputConfig(void)
{
	// update FCEUX input config
	FCEUD_SetInput(currMovieData.fourscore, currMovieData.microphone, (ESI)currMovieData.ports[0], (ESI)currMovieData.ports[1], (ESIFC)currMovieData.ports[2]);
	// update PAL flag
	pal_emulation = currMovieData.palFlag;
	if (pal_emulation)
	{
		dendy = 0;
	}
	fceu11::SetVidSystem(pal_emulation);
	RefreshThrottleFPS();
	//PushCurrentVideoSettings();
	// update PPU type
	newppu = currMovieData.PPUflag;
	//SetMainWindowText();
	// return focus to TAS Editor window
	//SetFocus(taseditorWindow.hwndTASEditor);
	RAMInitOption = currMovieData.RAMInitOption;
}
//----------------------------------------------------------------------------
TasEditorWindow::TasEditorWindow(QWidget *parent)
	: QDialog( parent, Qt::Window ), bookmarks(this), branches(this)
{
	QSettings  settings;
	QVBoxLayout *mainLayout;
	//QHBoxLayout *hbox;
	QMenuBar    *menuBar;

	tasWin = this;
	::project         = &this->project;
	::taseditorConfig = &this->taseditorConfig;
	::taseditor_lua   = &this->taseditor_lua;
	::markersManager  = &this->markersManager;
	::selection       = &this->selection;
	::greenzone       = &this->greenzone;
	::bookmarks       = &this->bookmarks;
	::playback        = &this->playback;
	::recorder        = &this->recorder;
	::history         = &this->history;
	::branches        = &this->branches;
	::splicer         = &this->splicer;

	this->taseditorConfig.load();

	clipboard = QGuiApplication::clipboard();

	setWindowTitle(tr("TAS Editor"));
	//setWindowIcon( QIcon(":icons/taseditor-icon32.png") );

	resize(512, 512);

	mainLayout = new QVBoxLayout();
	mainHBox   = new TasEditorSplitter();

	initPatterns();
	buildPianoRollDisplay();
	buildSideControlPanel();

	mainHBox->addWidget( pianoRollContainerWidget );
	mainHBox->addWidget( controlPanelContainerWidget );
	mainLayout->addWidget(mainHBox);

	mainHBox->setStretchFactor( 0, 5 );
	mainHBox->setStretchFactor( 1, 1 );

	menuBar = buildMenuBar();

	setLayout(mainLayout);
	mainLayout->setMenuBar( menuBar );
	pianoRoll->setFocus();

	for (int i=0; i<HK_MAX; i++)
	{
		hotkeyShortcut[i] = nullptr;	
	}
	initHotKeys();
	initModules();

	updateCheckedItems();
	updateToolTips();

	// Restore Window Geometry
	restoreGeometry(settings.value("tasEditor/geometry").toByteArray());

	// Restore Horizontal Panel State
	mainHBox->restoreState( settings.value("tasEditor/hPanelState").toByteArray() );
}
//----------------------------------------------------------------------------
TasEditorWindow::~TasEditorWindow(void)
{
	QSettings  settings;

	printf("Destroy Tas Editor Window\n");

	FCEU_WRAPPER_LOCK();
	//if (!askToSaveProject()) return false;

	// destroy window
	taseditorConfig.save();
	//taseditorWindow.exit();
	//disableGeneralKeyboardInput();
	// release memory
	//editor.free();
	//pianoRoll.free();
	markersManager.free();
	greenzone.free();
	bookmarks.free();
	branches.free();
	//popupDisplay.free();
	history.free();
	playback.stopSeeking();
	selection.free();

	// switch off TAS Editor mode
	movieMode = MOVIEMODE_INACTIVE;
	FCEU_DispMessage("TAS Editor disengaged", 0);
	FCEUMOV_CreateCleanMovie();

	if ( tasWin == this )
	{
		tasWin = NULL;
	}

	::project         = NULL;
	::taseditorConfig = NULL;
	::taseditor_lua   = NULL;
	::markersManager  = NULL;
	::selection       = NULL;
	::greenzone       = NULL;
	::bookmarks       = NULL;
	::playback        = NULL;
	::recorder        = NULL;
	::history         = NULL;
	::branches        = NULL;
	::splicer         = NULL;

	clearProjectList();

	FCEU_WRAPPER_UNLOCK();

	// Save Horizontal Panel State
	settings.setValue("tasEditor/hPanelState", mainHBox->saveState());

	// Save Window Geometry
	settings.setValue("tasEditor/geometry", saveGeometry());
}
//----------------------------------------------------------------------------
void TasEditorWindow::closeEvent(QCloseEvent *event)
{
	printf("Tas Editor Close Window Event\n");

	if (!askToSaveProject())
	{
	        event->ignore();
		return;
	}
	project.reset();

	done(0);
	deleteLater();
	event->accept();
}
//----------------------------------------------------------------------------
void TasEditorWindow::closeWindow(void)
{
	if (!askToSaveProject())
	{
		return;
	}
	project.reset();

	printf("Tas Editor Close Window\n");
	done(0);
	deleteLater();
}
//----------------------------------------------------------------------------
void TasEditorWindow::retranslateUi(void)
{
	setWindowTitle( tr("TAS Editor") );

	// Piano roll display
	if (upperMarkerLabel) upperMarkerLabel->setText( tr("Marker 0") );
	if (lowerMarkerLabel) lowerMarkerLabel->setText( tr("Marker 0") );

	// Side control panel
	if (playbackGBox) playbackGBox->setTitle( tr("Playback") );
	if (recorderGBox) recorderGBox->setTitle( tr("Recorder") );
	if (splicerGBox ) splicerGBox->setTitle( tr("Splicer") );

	if (followCursorCbox) followCursorCbox->setText( tr("Follow Cursor") );
	if (turboSeekCbox   ) turboSeekCbox->setText( tr("Turbo Seek") );
	if (autoRestoreCbox ) autoRestoreCbox->setText( tr("Auto-Restore Last Position") );

	if (recRecordingCbox  ) recRecordingCbox->setText( tr("Recording") );
	if (recSuperImposeCbox) recSuperImposeCbox->setText( tr("Superimpose") );
	if (recUsePatternCbox ) recUsePatternCbox->setText( tr("Use Pattern") );
	if (recAllBtn         ) recAllBtn->setText( tr("All") );
	if (rec1PBtn          ) rec1PBtn->setText( tr("1P") );
	if (rec2PBtn          ) rec2PBtn->setText( tr("2P") );
	if (rec3PBtn          ) rec3PBtn->setText( tr("3P") );
	if (rec4PBtn          ) rec4PBtn->setText( tr("4P") );

	if (selectionLbl) selectionLbl->setText( tr("Empty") );
	if (clipboardLbl) clipboardLbl->setText( tr("Empty") );

	if (similarBtn) similarBtn->setText( tr("Similar") );
	if (moreBtn   ) moreBtn->setText( tr("More") );

	// Tab widget labels
	if (bkmkBrnchStack)
	{
		bkmkBrnchStack->setTabText( 0, tr("Bookmarks") );
		bkmkBrnchStack->setTabText( 1, tr("Branches")  );
		bkmkBrnchStack->setTabText( 2, tr("History")   );
	}

	// Update tool tips for all hotkey-bound actions
	updateToolTips();

	// Refresh menu actions and titles (full retranslate)
	retranslateMenuBar();
}
//----------------------------------------------------------------------------
void TasEditorWindow::retranslateMenuBar(void)
{
	// Partial PHASE-2 menu retranslation.
	//
	// Full retranslate would require storing the original English source
	// string in QAction::data() at buildMenuBar() construction time, then
	// calling setText( tr(source) ) here. That is a 68-site change in
	// buildMenuBar() and is deferred to a follow-up commit; the menu bar
	// will keep its initial-locale text on language switch (acceptable for
	// PHASE-2 per the plan).
	//
	// The recent project menu IS rebuilt so the file paths get re-tr()'d.
	if (recentProjectMenu)
	{
		buildRecentProjectMenu();
	}
	(void)0;
}
//----------------------------------------------------------------------------
void TasEditorWindow::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		setWindowTitle( tr("TAS Editor") );
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//----------------------------------------------------------------------------
int TasEditorWindow::requestWindowClose(void)
{
	askToSaveProject();

	project.reset();

	printf("Tas Editor Close Window\n");
	done(0);
	deleteLater();

	return 0;
}
//----------------------------------------------------------------------------
QMenuBar *TasEditorWindow::buildMenuBar(void)
{
	QMenu       *fileMenu, *editMenu, *viewMenu,
		    *confMenu, *luaMenu,  *helpMenu,
		    *patternMenu;
	QActionGroup *actGroup;
	QAction     *act;
	ColorMenuItem  *colorAct;
	int useNativeMenuBar=0;

	QMenuBar *menuBar = new QMenuBar(this);

	// This is needed for menu bar to show up on MacOS
	g_config->getOption( "SDL.UseNativeMenuBar", &useNativeMenuBar );

	menuBar->setNativeMenuBar( useNativeMenuBar ? true : false );

	//-----------------------------------------------------------------------
	// Menu Start
	//-----------------------------------------------------------------------
	// File
	fileMenu = menuBar->addMenu(tr("&File"));

	// File -> New
	act = new QAction(tr("&New"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Open New Project"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(createNewProject(void)) );

	fileMenu->addAction(act);

	// File -> Open
	act = new QAction(tr("&Open"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+O")));
	act->setStatusTip(tr("Open Project"));
	//act->setIcon( style()->standardIcon( QStyle::SP_BrowserStop ) );
	connect(act, SIGNAL(triggered()), this, SLOT(openProject(void)) );

	fileMenu->addAction(act);

	// File -> Save
	act = new QAction(tr("&Save"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+S")));
	act->setStatusTip(tr("Save Project"));
	//act->setIcon( style()->standardIcon( QStyle::SP_BrowserStop ) );
	connect(act, SIGNAL(triggered()), this, SLOT(saveProjectCb(void)) );

	fileMenu->addAction(act);

	// File -> Save As
	act = new QAction(tr("Save &As"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Shift+S")));
	act->setStatusTip(tr("Save Project As"));
	//act->setIcon( style()->standardIcon( QStyle::SP_BrowserStop ) );
	connect(act, SIGNAL(triggered()), this, SLOT(saveProjectAsCb(void)) );

	fileMenu->addAction(act);

	// File -> Save Compact
	act = new QAction(tr("Save &Compact"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+Shift+S")));
	act->setStatusTip(tr("Save Compact"));
	//act->setIcon( style()->standardIcon( QStyle::SP_BrowserStop ) );
	connect(act, SIGNAL(triggered()), this, SLOT(saveProjectCompactCb(void)) );

	fileMenu->addAction(act);

	// File -> Recent
	recentProjectMenu = fileMenu->addMenu( tr("&Recent") );

	buildRecentProjectMenu();
	recentProjectMenuReset = false;

	fileMenu->addSeparator();

	// File -> Import Input
	act = new QAction(tr("&Import Input"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+Shift+S")));
	act->setStatusTip(tr("Import Input"));
	//act->setIcon( style()->standardIcon( QStyle::SP_BrowserStop ) );
	connect(act, SIGNAL(triggered()), this, SLOT(importMovieFile(void)) );

	fileMenu->addAction(act);

	// File -> Export to fm2
	act = new QAction(tr("&Export to fm2"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+Shift+S")));
	act->setStatusTip(tr("Export to fm2"));
	//act->setIcon( style()->standardIcon( QStyle::SP_BrowserStop ) );
	connect(act, SIGNAL(triggered()), this, SLOT(exportMovieFile(void)) );

	fileMenu->addAction(act);

	fileMenu->addSeparator();

	// File -> Quit
	act = new QAction(tr("&Quit Window"), this);
	act->setShortcut(QKeySequence(tr("Alt+F4")));
	act->setStatusTip(tr("Close Window"));
	act->setIcon(style()->standardIcon(QStyle::SP_DialogCloseButton));
	connect(act, SIGNAL(triggered()), this, SLOT(closeWindow(void)) );

	fileMenu->addAction(act);

	// Edit
	editMenu = menuBar->addMenu(tr("&Edit"));

	// Edit -> Undo
	act = new QAction(tr("&Undo"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Z")));
	act->setStatusTip(tr("Undo Changes"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editUndoCB(void)) );

	editMenu->addAction(act);

	// Edit -> Redo
	act = new QAction(tr("&Redo"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Y")));
	act->setStatusTip(tr("Redo Changes"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editRedoCB(void)) );

	editMenu->addAction(act);

	// Edit -> Selection Undo
	act = new QAction(tr("Selection &Undo"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Q")));
	act->setStatusTip(tr("Undo Selection"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editUndoSelCB(void)) );

	editMenu->addAction(act);

	// Edit -> Selection Redo
	act = new QAction(tr("Selection &Redo"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+W")));
	act->setStatusTip(tr("Redo Selection"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editRedoSelCB(void)) );

	editMenu->addAction(act);

	editMenu->addSeparator();

	// Edit -> Deselect
	act = new QAction(tr("Deselect"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+W")));
	act->setStatusTip(tr("Deselect"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editDeselectAll(void)) );

	editMenu->addAction(act);

	// Edit -> Select All
	act = new QAction(tr("Select All"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+W")));
	act->setStatusTip(tr("Select All"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editSelectAll(void)) );

	editMenu->addAction(act);

	// Edit -> Select Between Markers
	act = new QAction(tr("Select Between Markers"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+A")));
	act->setStatusTip(tr("Select Between Markers"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editSelBtwMkrs(void)) );

	editMenu->addAction(act);

	// Edit -> Reselect Clipboard
	act = new QAction(tr("Reselect Clipboard"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+B")));
	act->setStatusTip(tr("Reselect Clipboard"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editReselectClipboard(void)) );

	editMenu->addAction(act);

	editMenu->addSeparator();

	// Edit -> Copy
	act = new QAction(tr("Copy"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+C")));
	act->setStatusTip(tr("Copy"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editCopyCB(void)) );

	editMenu->addAction(act);

	// Edit -> Paste
	act = new QAction(tr("Paste"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+V")));
	act->setStatusTip(tr("Paste"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editPasteCB(void)) );

	editMenu->addAction(act);

	// Edit -> Paste Insert
	act = new QAction(tr("Paste Insert"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Shift+V")));
	act->setStatusTip(tr("Paste Insert"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editPasteInsertCB(void)) );

	editMenu->addAction(act);

	// Edit -> Cut
	act = new QAction(tr("Cut"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+X")));
	act->setStatusTip(tr("Cut"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editCutCB(void)) );

	editMenu->addAction(act);

	editMenu->addSeparator();

	// Edit -> Clear
	act = new QAction(tr("Clear"), this);
	act->setShortcut(QKeySequence(tr("Del")));
	act->setStatusTip(tr("Clear"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editClearCB(void)) );

	editMenu->addAction(act);

	// Edit -> Delete
	act = new QAction(tr("Delete"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Del")));
	act->setStatusTip(tr("Delete"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editDeleteCB(void)) );

	editMenu->addAction(act);

	// Edit -> Clone
	act = new QAction(tr("Clone"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Ins")));
	act->setStatusTip(tr("Clone"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editCloneCB(void)) );

	editMenu->addAction(act);

	// Edit -> Insert
	act = new QAction(tr("Insert"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+Shift+Ins")));
	act->setStatusTip(tr("Insert"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editInsertCB(void)) );

	editMenu->addAction(act);

	// Edit -> Insert # of Frames
	act = new QAction(tr("Insert # of Frames"), this);
	act->setShortcut(QKeySequence(tr("Ins")));
	act->setStatusTip(tr("Insert # of Frames"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editInsertNumFramesCB(void)) );

	editMenu->addAction(act);

	editMenu->addSeparator();

	// Edit -> Truncate Movie
	act = new QAction(tr("Truncate Movie"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+Ins")));
	act->setStatusTip(tr("Truncate Movie"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(editTruncateMovieCB(void)) );

	editMenu->addAction(act);

	// View
	viewMenu = menuBar->addMenu(tr("&View"));

	// View -> Find Note Window
	act = new QAction(tr("Find Note Window"), this);
	act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Find Note Window"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(openFindNoteWindow(void)) );

	viewMenu->addAction(act);

	viewMenu->addSeparator();
	
	// View -> Display Branch Screenshots
	dpyBrnchScrnAct = act = new QAction(tr("Display Branch Screenshots"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Display Branch Screenshots"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(dpyBrnchScrnChanged(bool)) );

	viewMenu->addAction(act);

	// View -> Display Branch Screenshots
	dpyBrnchDescAct = act = new QAction(tr("Display Branch Descriptions"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Display Branch Descriptions"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(dpyBrnchDescChanged(bool)) );

	viewMenu->addAction(act);

	// View -> Enable Hot Changes
	enaHotChgAct = act = new QAction(tr("Enable Hot Changes"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Enable Hot Changes"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(enaHotChgChanged(bool)) );

	viewMenu->addAction(act);

	viewMenu->addSeparator();

	// View -> Follow Undo Content
	followUndoAct = act = new QAction(tr("Follow Undo Content"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Follow Undo Content"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(followUndoActChanged(bool)) );

	viewMenu->addAction(act);

	// View -> Follow Marker Note Content
	followMkrAct = act = new QAction(tr("Follow Marker Note Content"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Follow Marker Note Content"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(followMkrActChanged(bool)) );

	viewMenu->addAction(act);

	viewMenu->addSeparator();

	// View -> Piano Roll Font
	act = new QAction(tr("Piano Roll Font..."), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Select Piano Roll Font"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(void)), this, SLOT(changePianoRollFontCB(void)) );

	viewMenu->addAction(act);

	// View -> Bookmarks Font
	act = new QAction(tr("Bookmarks View Font..."), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Select Bookmarks View Font"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(void)), this, SLOT(changeBookmarksFontCB(void)) );

	viewMenu->addAction(act);

	// View -> Branches Font
	act = new QAction(tr("Branches View Font..."), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+F")));
	act->setStatusTip(tr("Select Branches View Font"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(void)), this, SLOT(changeBranchesFontCB(void)) );

	viewMenu->addAction(act);

	viewMenu->addSeparator();

	// View -> Piano Roll Grid Color
	colorAct = new ColorMenuItem(tr("Piano Roll Grid Color..."), "SDL.TasPianoRollGridColor", this);
	colorAct->setStatusTip(tr("Select Piano Roll Grid Color"));

	colorAct->connectColor( &pianoRoll->gridColor );

	viewMenu->addAction(colorAct);

	// Config
	confMenu = menuBar->addMenu(tr("&Config"));

	// Config -> Project File Saving Options
	act = new QAction(tr("Project File Saving Options"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Project File Saving Options"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(openProjectSaveOptions(void)) );

	confMenu->addAction(act);

	// Config -> Set Max Undo Levels
	act = new QAction(tr("Set Max Undo Levels"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Set Max Undo History"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(setMaxUndoCapacity(void)) );

	confMenu->addAction(act);

	// Config -> Set Greenzone Capacity
	act = new QAction(tr("Set Greenzone Capacity"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Set Greenzone Capacity"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(setGreenzoneCapacity(void)) );

	confMenu->addAction(act);

	confMenu->addSeparator();

	// Config -> Enable Greenzoneing
	enaGrnznAct = act = new QAction(tr("Enable Greenzoning"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Enable Greenzoning"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(enaGrnznActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> Autofire Pattern skips Lag
	afPtrnSkipLagAct = act = new QAction(tr("Autofire Pattern skips Lag"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Autofire Pattern skips Lag"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(afPtrnSkipLagActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> Auto Adjust Input According to Lag
	adjInputLagAct = act = new QAction(tr("Auto Adjust Input According to Lag"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Auto Adjust Input According to Lag"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(adjInputLagActChanged(bool)) );

	confMenu->addAction(act);

	confMenu->addSeparator();

	// Config -> Draw Input by Dragging
	drawInputDragAct = act = new QAction(tr("Draw Input by Dragging"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Draw Input by Dragging"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(drawInputDragActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> Combine Consecutive Recordings/Draws
	cmbRecDrawAct = act = new QAction(tr("Combine Consecutive Recordings/Draws"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Combine Consecutive Recordings/Draws"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(cmbRecDrawActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> Use 1P Keys for all Single Recordings
	use1PforRecAct = act = new QAction(tr("Use 1P Keys for all Single Recordings"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Use 1P Keys for all Single Recordings"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(use1PforRecActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> Use Input Keys for Column Set
	useInputColSetAct = act = new QAction(tr("Use Input Keys for Column Set"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Use Input Keys for Column Set"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(useInputColSetActChanged(bool)) );

	confMenu->addAction(act);

	confMenu->addSeparator();

	// Config -> Bind Markers to Input
	bindMkrInputAct = act = new QAction(tr("Bind Markers to Input"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Bind Markers to Input"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(bindMkrInputActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> Empty New Marker Notes
	emptyNewMkrNotesAct = act = new QAction(tr("Empty New Marker Notes"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Empty New Marker Notes"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(emptyNewMkrNotesActChanged(bool)) );

	confMenu->addAction(act);

	confMenu->addSeparator();

	// Config -> Old Control Scheme for Branching
	oldCtlBrnhSchemeAct = act = new QAction(tr("Old Control Scheme for Branching"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Old Control Scheme for Branching"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(oldCtlBrnhSchemeActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> Branches Restore Entire Movie
	brnchRestoreMovieAct = act = new QAction(tr("Branches Restore Entire Movie"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Branches Restore Entire Movie"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(brnchRestoreMovieActChanged(bool)) );

	confMenu->addAction(act);

	// Config -> HUD in Branch Screenshots
	hudInScrnBranchAct = act = new QAction(tr("HUD in Branch Screenshots"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("HUD in Branch Screenshots"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(hudInScrnBranchActChanged(bool)) );

	confMenu->addAction(act);

	confMenu->addSeparator();

	// Config -> Autopause at End of Movie
	pauseAtEndAct = act = new QAction(tr("Autopause at End of Movie"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Autopause at End of Movie"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(pauseAtEndActChanged(bool)) );

	confMenu->addAction(act);

	// Lua
	luaMenu = menuBar->addMenu(tr("&Lua"));

	// Lua -> Run Function
	act = new QAction(tr("Run Function"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Run Function"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(manLuaRun(void)) );

	luaMenu->addAction(act);

	luaMenu->addSeparator();

	// Lua -> Auto Function
	autoLuaAct = act = new QAction(tr("Auto Function"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Auto Function"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(autoLuaRunChanged(bool)) );

	luaMenu->addAction(act);

	// Pattern
	patternMenu = menuBar->addMenu(tr("&Pattern"));

	actGroup = new QActionGroup(this);

	for (size_t i=0; i<patternsNames.size(); i++)
	{
		// Pattern -> Names
		act = new QAction(tr(patternsNames[i].c_str()), this);
		act->setCheckable(true);
		//act->setShortcut(QKeySequence(tr("Ctrl+N")));
		act->setStatusTip(tr(patternsNames[i].c_str()));
		//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
		connect(act, &QAction::triggered, [this, i] { setCurrentPattern(i); } );

		actGroup->addAction(act);
		patternMenu->addAction(act);

		act->setChecked( static_cast<size_t>(taseditorConfig.currentPattern) == i );
	}

	// Help
	helpMenu = menuBar->addMenu(tr("&Help"));

	// Help -> Open TAS Editor Manual
	act = new QAction(tr("Open TAS Editor Manual"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Open TAS Editor Manual"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(openOnlineDocs(void)) );

	helpMenu->addAction(act);

	// Help -> Enable Tool Tips
	showToolTipsAct = act = new QAction(tr("Enable Tool Tips"), this);
	act->setCheckable(true);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("Enable Tool Tips"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered(bool)), this, SLOT(showToolTipsActChanged(bool)) );

	helpMenu->addAction(act);

	helpMenu->addSeparator();

	// Help -> About
	act = new QAction(tr("About"), this);
	//act->setShortcut(QKeySequence(tr("Ctrl+N")));
	act->setStatusTip(tr("About"));
	//act->setIcon( style()->standardIcon( QStyle::SP_FileDialogStart ) );
	connect(act, SIGNAL(triggered()), this, SLOT(openAboutWindow(void)) );

	helpMenu->addAction(act);

	return menuBar;
}
//----------------------------------------------------------------------------
void TasEditorWindow::buildPianoRollDisplay(void)
{
	QVBoxLayout *vbox;
	QHBoxLayout *hbox;
	QGridLayout *grid;

	pianoRollFrame   = new QFrame();
	grid             = new QGridLayout();
	pianoRoll        = new QPianoRoll(this);
	pianoRollVBar    = new PianoRollScrollBar( this );
	pianoRollHBar    = new QScrollBar( Qt::Horizontal, this );
	upperMarkerLabel = new QPushButton( this );
	upperMarkerLabel->setText( tr("Marker 0") );
	lowerMarkerLabel = new QPushButton( this );
	lowerMarkerLabel->setText( tr("Marker 0") );
	upperMarkerNote  = new UpperMarkerNoteEdit();
	lowerMarkerNote  = new LowerMarkerNoteEdit();

	//upperMarkerLabel->setFlat(true);
	//lowerMarkerLabel->setFlat(true);

	pianoRollFrame->setLineWidth(2);
	pianoRollFrame->setMidLineWidth(1);
	//pianoRollFrame->setFrameShape( QFrame::StyledPanel );
	pianoRollFrame->setFrameShape( QFrame::Box );

	pianoRollVBar->setInvertedControls(false);
	pianoRollVBar->setInvertedAppearance(false);
	pianoRoll->setScrollBars( pianoRollHBar, pianoRollVBar );
	connect( pianoRollHBar, SIGNAL(valueChanged(int)), pianoRoll, SLOT(hbarChanged(int)) );
	connect( pianoRollVBar, SIGNAL(valueChanged(int)), pianoRoll, SLOT(vbarChanged(int)) );
	//connect( pianoRollVBar, SIGNAL(actionTriggered(int)), pianoRoll, SLOT(vbarActionTriggered(int)) );

	grid->addWidget( pianoRoll    , 0, 0 );
	grid->addWidget( pianoRollVBar, 0, 1 );
	grid->addWidget( pianoRollHBar, 1, 0 );

	vbox = new QVBoxLayout();

	pianoRollHBar->setMinimum(0);
	pianoRollHBar->setMaximum(100);
	pianoRollVBar->setMinimum(0);
	pianoRollVBar->setMaximum(100);

	hbox = new QHBoxLayout();
	hbox->addWidget( upperMarkerLabel, 1 );
	hbox->addWidget( upperMarkerNote, 10 );

	vbox->addLayout( hbox, 1 );
	vbox->addWidget( pianoRollFrame, 100 );
	//vbox->addLayout( grid, 100 );
	pianoRollFrame->setLayout( grid );

	hbox = new QHBoxLayout();
	hbox->addWidget( lowerMarkerLabel, 1 );
	hbox->addWidget( lowerMarkerNote, 10 );

	vbox->addLayout( hbox, 1 );
	
	pianoRollContainerWidget = new QWidget();
	pianoRollContainerWidget->setLayout( vbox );

	connect( upperMarkerLabel, SIGNAL(clicked(void)), this, SLOT(upperMarkerLabelClicked(void)) );
	connect( lowerMarkerLabel, SIGNAL(clicked(void)), this, SLOT(lowerMarkerLabelClicked(void)) );
}
//----------------------------------------------------------------------------
void TasEditorWindow::initPatterns(void)
{
	if (patterns.size() == 0)
	{
		FCEU_printf("Will be using default set of patterns...\n");
		patterns.resize(4);
		patternsNames.resize(4);
		// Default Pattern 0: Alternating (1010...)
		patternsNames[0] = "Alternating (1010...)";
		patterns[0].resize(2);
		patterns[0][0] = 1;
		patterns[0][1] = 0;
		// Default Pattern 1: Alternating at 30FPS (11001100...)
		patternsNames[1] = "Alternating at 30FPS (11001100...)";
		patterns[1].resize(4);
		patterns[1][0] = 1;
		patterns[1][1] = 1;
		patterns[1][2] = 0;
		patterns[1][3] = 0;
		// Default Pattern 2: One Quarter (10001000...)
		patternsNames[2] = "One Quarter (10001000...)";
		patterns[2].resize(4);
		patterns[2][0] = 1;
		patterns[2][1] = 0;
		patterns[2][2] = 0;
		patterns[2][3] = 0;
		// Default Pattern 3: Tap'n'Hold (1011111111111111111111111111111111111...)
		patternsNames[3] = "Tap'n'Hold (101111111...)";
		patterns[3].resize(1000);
		patterns[3][0] = 1;
		patterns[3][1] = 0;
		for (int i = 2; i < 1000; ++i)
		{
			patterns[3][i] = 1;
		}
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::buildSideControlPanel(void)
{
	QShortcut   *shortcut;
	QVBoxLayout *vbox;
	QHBoxLayout *hbox;
	QGridLayout *grid;
	QScrollArea *scrollArea1, *scrollArea2;
	QTreeWidgetItem *item;

	ctlPanelMainVbox = new QVBoxLayout();

	playbackGBox  = new QGroupBox( this );
	playbackGBox->setTitle( tr("Playback") );
	recorderGBox  = new QGroupBox( this );
	recorderGBox->setTitle( tr("Recorder") );
	splicerGBox   = new QGroupBox( this );
	splicerGBox->setTitle( tr("Splicer") );
	//luaGBox       = new QGroupBox( tr("Lua") );
	//historyGBox   = new QGroupBox( tr("History") );
	bbFrame       = new QFrame();

	bbFrame->setFrameShape( QFrame::StyledPanel );

	rewindMkrBtn  = new QPushButton();
	rewindFrmBtn  = new QPushButton();
	playPauseBtn  = new QPushButton();
	advFrmBtn     = new QPushButton();
	advMkrBtn     = new QPushButton();

	rewindMkrBtn->setIcon( style()->standardIcon( QStyle::SP_MediaSkipBackward ) );
	rewindFrmBtn->setIcon( style()->standardIcon( QStyle::SP_MediaSeekBackward ) );
	playPauseBtn->setIcon( style()->standardIcon( QStyle::SP_MediaPause ) );
	   advFrmBtn->setIcon( style()->standardIcon( QStyle::SP_MediaSeekForward ) );
	   advMkrBtn->setIcon( style()->standardIcon( QStyle::SP_MediaSkipForward ) );

	progBar = new QProgressBar();
	progBar->setRange( 0, 1 );

	followCursorCbox = new QCheckBox( this );
	followCursorCbox->setText( tr("Follow Cursor") );
	   turboSeekCbox = new QCheckBox( this );
	   turboSeekCbox->setText( tr("Turbo Seek") );
	 autoRestoreCbox = new QCheckBox( this );
	 autoRestoreCbox->setText( tr("Auto-Restore Last Position") );

	recRecordingCbox   = new QCheckBox( this );
	recRecordingCbox->setText( tr("Recording") );
	recSuperImposeCbox = new QCheckBox( this );
	recSuperImposeCbox->setText( tr("Superimpose") );
	recUsePatternCbox  = new QCheckBox( this );
	recUsePatternCbox->setText( tr("Use Pattern") );
	recAllBtn          = new QRadioButton( this );
	recAllBtn->setText( tr("All") );
	rec1PBtn           = new QRadioButton( this );
	rec1PBtn->setText( tr("1P") );
	rec2PBtn           = new QRadioButton( this );
	rec2PBtn->setText( tr("2P") );
	rec3PBtn           = new QRadioButton( this );
	rec3PBtn->setText( tr("3P") );
	rec4PBtn           = new QRadioButton( this );
	rec4PBtn->setText( tr("4P") );

	selectionLbl = new QLabel( this );
	selectionLbl->setText( tr("Empty") );
	clipboardLbl = new QLabel( this );
	clipboardLbl->setText( tr("Empty") );

	//runLuaBtn   = new QPushButton( tr("Run Function") );
	//autoLuaCBox = new QCheckBox( tr("Auto Function") );
	//runLuaBtn->setEnabled(false);
	//autoLuaCBox->setChecked(true);

	histTree = new QTreeWidget();

	histTree->setColumnCount(1);
	histTree->setSelectionMode( QAbstractItemView::SingleSelection );
	histTree->setAlternatingRowColors(true);

	item = new QTreeWidgetItem();
	item->setText(0, QString::fromStdString("Time / Description"));

	histTree->setHeaderItem(item);

	prevMkrBtn = new QPushButton();
	nextMkrBtn = new QPushButton();
	similarBtn = new QPushButton( this );
	similarBtn->setText( tr("Similar") );
	moreBtn    = new QPushButton( this );
	moreBtn->setText( tr("More") );

	prevMkrBtn->setIcon( style()->standardIcon( QStyle::SP_MediaSkipBackward ) );
	nextMkrBtn->setIcon( style()->standardIcon( QStyle::SP_MediaSkipForward  ) );

	vbox = new QVBoxLayout();
	hbox = new QHBoxLayout();
	vbox->addLayout( hbox );
	hbox->addWidget( rewindMkrBtn );
	hbox->addWidget( rewindFrmBtn );
	hbox->addWidget( playPauseBtn );
	hbox->addWidget( advFrmBtn    );
	hbox->addWidget( advMkrBtn    );
	vbox->addWidget( progBar );

	hbox = new QHBoxLayout();
	vbox->addLayout( hbox );
	hbox->addWidget( followCursorCbox );
	hbox->addWidget( turboSeekCbox    );

	vbox->addWidget( autoRestoreCbox );

	playbackGBox->setLayout( vbox );

	grid = new QGridLayout();
	grid->addWidget( recRecordingCbox, 0, 0, 1, 2 );
	grid->addWidget( recAllBtn       , 0, 3, 1, 1 );
	grid->addWidget( rec1PBtn        , 1, 0, 1, 1 );
	grid->addWidget( rec2PBtn        , 1, 1, 1, 1 );
	grid->addWidget( rec3PBtn        , 1, 2, 1, 1 );
	grid->addWidget( rec4PBtn        , 1, 3, 1, 1 );
	grid->addWidget( recSuperImposeCbox, 2, 0, 1, 2 );
	grid->addWidget( recUsePatternCbox , 2, 2, 1, 2 );
	recorderGBox->setLayout( grid );

	grid = new QGridLayout();
	QLabel *selectionHdrLbl = new QLabel( this );
	selectionHdrLbl->setText( tr("Selection:") );
	QLabel *clipboardHdrLbl = new QLabel( this );
	clipboardHdrLbl->setText( tr("Clipboard:") );
	grid->addWidget( selectionHdrLbl, 0, 0, 1, 1 );
	grid->addWidget( clipboardHdrLbl, 1, 0, 1, 1 );
	grid->addWidget( selectionLbl, 0, 1, 1, 3 );
	grid->addWidget( clipboardLbl, 1, 1, 1, 3 );
	splicerGBox->setLayout( grid );

	//hbox = new QHBoxLayout();
	//hbox->addWidget( runLuaBtn );
	//hbox->addWidget( autoLuaCBox );
	//luaGBox->setLayout( hbox );
	
	scrollArea1 = new QScrollArea();
	scrollArea1->setWidgetResizable(false);
	scrollArea1->setSizeAdjustPolicy( QAbstractScrollArea::AdjustToContents );
	scrollArea1->setHorizontalScrollBarPolicy( Qt::ScrollBarAsNeeded );
	scrollArea1->setVerticalScrollBarPolicy( Qt::ScrollBarAsNeeded );
	scrollArea1->setMinimumSize( QSize( 128, 128 ) );
	scrollArea1->setWidget( &bookmarks );

	scrollArea2 = new QScrollArea();
	scrollArea2->setWidgetResizable(true);
	scrollArea2->setSizeAdjustPolicy( QAbstractScrollArea::AdjustToContents );
	scrollArea2->setHorizontalScrollBarPolicy( Qt::ScrollBarAsNeeded );
	scrollArea2->setVerticalScrollBarPolicy( Qt::ScrollBarAsNeeded );
	scrollArea2->setMinimumSize( QSize( 128, 128 ) );
	scrollArea2->setWidget( &branches );

	bkmkBrnchStack = new QTabWidget();
	bkmkBrnchStack->addTab( scrollArea1, QString() );
	bkmkBrnchStack->setTabText( 0, tr("Bookmarks") );
	bkmkBrnchStack->addTab( scrollArea2, QString() );
	bkmkBrnchStack->setTabText( 1, tr("Branches")  );
	bkmkBrnchStack->addTab( histTree   , QString() );
	bkmkBrnchStack->setTabText( 2, tr("History")   );

	taseditorConfig.displayBranchesTree = 0;

	vbox = new QVBoxLayout();
	vbox->addWidget( bkmkBrnchStack );
	bbFrame->setLayout( vbox );

	//vbox = new QVBoxLayout();
	//vbox->addWidget( histTree );
	//historyGBox->setLayout( vbox );

	ctlPanelMainVbox->addWidget( playbackGBox  );
	ctlPanelMainVbox->addWidget( recorderGBox  );
	ctlPanelMainVbox->addWidget( splicerGBox   );
	//ctlPanelMainVbox->addWidget( luaGBox       );
	ctlPanelMainVbox->addWidget( bbFrame       );
	//ctlPanelMainVbox->addWidget( historyGBox   );

	hbox = new QHBoxLayout();
	hbox->addWidget( prevMkrBtn );
	hbox->addWidget( similarBtn );
	hbox->addWidget( moreBtn    );
	hbox->addWidget( nextMkrBtn );
	ctlPanelMainVbox->addLayout( hbox );

	controlPanelContainerWidget = new QWidget();
	controlPanelContainerWidget->setLayout( ctlPanelMainVbox );

	recRecordingCbox->setChecked( !movie_readonly );
	connect( recRecordingCbox, SIGNAL(stateChanged(int)), this, SLOT(recordingChanged(int)) );

	recUsePatternCbox->setChecked( taseditorConfig.recordingUsePattern );
	connect( recUsePatternCbox, SIGNAL(stateChanged(int)), this, SLOT(usePatternChanged(int)) );

	recSuperImposeCbox->setTristate(true);
	connect( recSuperImposeCbox, SIGNAL(stateChanged(int)), this, SLOT(superImposedChanged(int)) );

	connect( recAllBtn, &QRadioButton::clicked, [ this ] { recordInputChanged( MULTITRACK_RECORDING_ALL ); } );
	connect( rec1PBtn , &QRadioButton::clicked, [ this ] { recordInputChanged( MULTITRACK_RECORDING_1P  ); } );
	connect( rec2PBtn , &QRadioButton::clicked, [ this ] { recordInputChanged( MULTITRACK_RECORDING_2P  ); } );
	connect( rec3PBtn , &QRadioButton::clicked, [ this ] { recordInputChanged( MULTITRACK_RECORDING_3P  ); } );
	connect( rec4PBtn , &QRadioButton::clicked, [ this ] { recordInputChanged( MULTITRACK_RECORDING_4P  ); } );

	connect( rewindMkrBtn, SIGNAL(pressed(void)), this, SLOT(playbackFrameRewindFull(void)) );
	connect( rewindFrmBtn, SIGNAL(pressed(void)), this, SLOT(playbackFrameRewind(void))     );
	connect( playPauseBtn, SIGNAL(pressed(void)), this, SLOT(playbackPauseCB(void))         );
	connect( advFrmBtn   , SIGNAL(pressed(void)), this, SLOT(playbackFrameForward(void))    );
	connect( advMkrBtn   , SIGNAL(pressed(void)), this, SLOT(playbackFrameForwardFull(void)));

	connect( followCursorCbox, SIGNAL(clicked(bool)), this, SLOT(playbackFollowCursorCb(bool)));
	connect( turboSeekCbox   , SIGNAL(clicked(bool)), this, SLOT(playbackTurboSeekCb(bool)));
	connect( autoRestoreCbox , SIGNAL(clicked(bool)), this, SLOT(playbackAutoRestoreCb(bool)));

	connect( prevMkrBtn, SIGNAL(clicked(void)), this, SLOT(jumpToPreviousMarker(void)) );
	connect( nextMkrBtn, SIGNAL(clicked(void)), this, SLOT(jumpToNextMarker(void)) );
	connect( similarBtn, SIGNAL(clicked(void)), this, SLOT(findSimilarNote(void)) );
	connect( moreBtn   , SIGNAL(clicked(void)), this, SLOT(findNextSimilarNote(void)) );

	//shortcut = new QShortcut( QKeySequence("Pause"), this);
	//connect( shortcut, SIGNAL(activated(void)), this, SLOT(playbackPauseCB(void)) );

	shortcut = new QShortcut( QKeySequence("Shift+Up"), this);
	connect( shortcut, SIGNAL(activated(void)), this, SLOT(playbackFrameRewind(void)) );

	shortcut = new QShortcut( QKeySequence("Shift+Down"), this);
	connect( shortcut, SIGNAL(activated(void)), this, SLOT(playbackFrameForward(void)) );

	shortcut = new QShortcut( QKeySequence("Shift+PgUp"), this);
	connect( shortcut, SIGNAL(activated(void)), this, SLOT(playbackFrameRewindFull(void)) );

	shortcut = new QShortcut( QKeySequence("Shift+PgDown"), this);
	connect( shortcut, SIGNAL(activated(void)), this, SLOT(playbackFrameForwardFull(void)) );

	shortcut = new QShortcut( QKeySequence("Ctrl+Up"), this);
	connect( shortcut, SIGNAL(activated(void)), this, SLOT(scrollSelectionUpOne(void)) );

	shortcut = new QShortcut( QKeySequence("Ctrl+Down"), this);
	connect( shortcut, SIGNAL(activated(void)), this, SLOT(scrollSelectionDnOne(void)) );

	connect( histTree, SIGNAL(itemClicked(QTreeWidgetItem*,int)), this, SLOT(histTreeItemActivated(QTreeWidgetItem*,int) ) );
	connect( histTree, SIGNAL(itemActivated(QTreeWidgetItem*,int)), this, SLOT(histTreeItemActivated(QTreeWidgetItem*,int) ) );

	connect( bkmkBrnchStack, SIGNAL(currentChanged(int)), this, SLOT(tabViewChanged(int) ) );
}
//----------------------------------------------------------------------------
void TasEditorWindow::initHotKeys(void)
{
	for (int i=0; i<HK_MAX; i++)
	{
		QKeySequence ks = Hotkeys[i].getKeySeq();
		QShortcut *shortcut = Hotkeys[i].getShortcut();

		//printf("HotKey: %i   %s\n", i, ks.toString().toStdString().c_str() );

		if ( hotkeyShortcut[i] == nullptr )
		{
			hotkeyShortcut[i] = new QShortcut( ks, this );

			if ( shortcut != nullptr )
			{
				connect( hotkeyShortcut[i], &QShortcut::activated, [ this, i, shortcut ] { activateHotkey( i, shortcut ); } );
			}
		}
		else
		{
			hotkeyShortcut[i]->setKey( ks );
		}
	}

	// Frame Advance uses key state directly, disable shortcut events
	hotkeyShortcut[HK_FRAME_ADVANCE]->setEnabled(false);
	hotkeyShortcut[HK_TURBO        ]->setEnabled(false);

	// Disable shortcuts that are not allowed with TAS Editor
	hotkeyShortcut[HK_OPEN_ROM      ]->setEnabled(false);
	hotkeyShortcut[HK_CLOSE_ROM     ]->setEnabled(false);
	hotkeyShortcut[HK_QUIT          ]->setEnabled(false);
	hotkeyShortcut[HK_FULLSCREEN    ]->setEnabled(false);
	hotkeyShortcut[HK_MAIN_MENU_HIDE]->setEnabled(false);
	hotkeyShortcut[HK_LOAD_LUA      ]->setEnabled(false);
	hotkeyShortcut[HK_FA_LAG_SKIP   ]->setEnabled(false);
}
//----------------------------------------------------------------------------
void TasEditorWindow::activateHotkey( int hkIdx, QShortcut *shortcut )
{
	shortcut->activated();
}
//----------------------------------------------------------------------------
void TasEditorWindow::updateRecordStatus(void)
{
	recRecordingCbox->setChecked( !movie_readonly );
}
//----------------------------------------------------------------------------
void TasEditorWindow::updateCheckedItems(void)
{

	followCursorCbox->setChecked( taseditorConfig.followPlaybackCursor );
	autoRestoreCbox->setChecked( taseditorConfig.autoRestoreLastPlaybackPosition );
	turboSeekCbox->setChecked( taseditorConfig.turboSeek );

	if ( taseditorConfig.superimpose == SUPERIMPOSE_CHECKED )
	{
		recSuperImposeCbox->setCheckState( Qt::Checked );
	}
	else if ( taseditorConfig.superimpose == SUPERIMPOSE_INDETERMINATE )
	{
		recSuperImposeCbox->setCheckState( Qt::PartiallyChecked );
	}
	else
	{	//taseditorConfig.superimpose == SUPERIMPOSE_UNCHECKED;
		recSuperImposeCbox->setCheckState( Qt::Unchecked );
	}
	recRecordingCbox->setChecked( !movie_readonly );
	recUsePatternCbox->setChecked( taseditorConfig.recordingUsePattern );
	dpyBrnchScrnAct->setChecked( taseditorConfig.displayBranchScreenshots );
	dpyBrnchDescAct->setChecked( taseditorConfig.displayBranchDescriptions );
	enaHotChgAct->setChecked( taseditorConfig.enableHotChanges );
	followMkrAct->setChecked( taseditorConfig.followMarkerNoteContext );
	followUndoAct->setChecked( taseditorConfig.followUndoContext );
	//autoLuaCBox->setChecked( taseditorConfig.enableLuaAutoFunction );
	autoLuaAct->setChecked( taseditorConfig.enableLuaAutoFunction );
	enaGrnznAct->setChecked( taseditorConfig.enableGreenzoning );
	afPtrnSkipLagAct->setChecked( taseditorConfig.autofirePatternSkipsLag );
	adjInputLagAct->setChecked( taseditorConfig.autoAdjustInputAccordingToLag );
	drawInputDragAct->setChecked( taseditorConfig.drawInputByDragging );
	cmbRecDrawAct->setChecked( taseditorConfig.combineConsecutiveRecordingsAndDraws );
	use1PforRecAct->setChecked( taseditorConfig.use1PKeysForAllSingleRecordings );
	useInputColSetAct->setChecked( taseditorConfig.useInputKeysForColumnSet );
	bindMkrInputAct->setChecked( taseditorConfig.bindMarkersToInput );
	emptyNewMkrNotesAct->setChecked( taseditorConfig.emptyNewMarkerNotes );
	oldCtlBrnhSchemeAct->setChecked( taseditorConfig.oldControlSchemeForBranching );
	brnchRestoreMovieAct->setChecked( taseditorConfig.branchesRestoreEntireMovie );
	hudInScrnBranchAct->setChecked( taseditorConfig.HUDInBranchScreenshots );
	pauseAtEndAct->setChecked( taseditorConfig.autopauseAtTheEndOfMovie );
	showToolTipsAct->setChecked( taseditorConfig.tooltipsEnabled );
}
//----------------------------------------------------------------------------
bool TasEditorWindow::updateHistoryItems(void)
{
	int i, cursorPos;
	QTreeWidgetItem *item;
	const char *txt;
	bool isVisible;

	isVisible = histTree->isVisible();

	if ( !isVisible )
	{
		return false;
	}

	cursorPos = history.getCursorPos();

	for (i=0; i<history.getNumItems(); i++)
	{
		txt = history.getItemDesc(i);

		item = histTree->topLevelItem(i);

		if (item == NULL)
		{
			item = new QTreeWidgetItem();

			histTree->addTopLevelItem(item);

			//histTree->setCurrentItem(item);
		}

		if ( txt )
		{
			if ( item->text(0).compare( tr(txt) ) != 0 )
			{
				item->setText(0, tr(txt));

				//histTree->setCurrentItem(item);
			}
		}
		if ( cursorPos == i )
		{
			histTree->setCurrentItem(item);
		}
	}

	while ( (histTree->topLevelItemCount() > 0) && (history.getNumItems() < histTree->topLevelItemCount()) )
	{
		item = histTree->takeTopLevelItem( histTree->topLevelItemCount()-1 );

		if ( item )
		{
			delete item;
		}
	}
	histTree->viewport()->update();

	return true;
}
//----------------------------------------------------------------------------
QPoint TasEditorWindow::getPreviewPopupCoordinates(void)
{
	return bkmkBrnchStack->mapToGlobal(QPoint(0,0));
}
//----------------------------------------------------------------------------
int TasEditorWindow::initModules(void)
{
#if SDL_VERSION_ATLEAST(2, 0, 18)
	tasEditorTimeStamp = SDL_GetTicks64();
#else
	tasEditorTimeStamp = SDL_GetTicks();
#endif
	// init modules
	//editor.init();
	//pianoRoll.init();
	selection.init();
	splicer.init();
	playback.init();
	greenzone.init();
	recorder.init();
	markersManager.init();
	project.init();
	bookmarks.init();
	branches.init();
	//popupDisplay.init();
	history.init();
	taseditor_lua.init();
	// either start new movie or use current movie
	if (!FCEUMOV_Mode(MOVIEMODE_RECORD|MOVIEMODE_PLAY) || currMovieData.savestate.size() != 0)
	{
		if (currMovieData.savestate.size() != 0)
		{
			FCEUD_PrintError("This version of TAS Editor doesn't work with movies starting from savestate.");
		}
		// create new movie
		fceu11::StopMovie();
		movieMode = MOVIEMODE_TASEDITOR;
		FCEUMOV_CreateCleanMovie();
		playback.restartPlaybackFromZeroGround();
	}
	else
	{
		// use current movie to create a new project
		fceu11::StopMovie();
		movieMode = MOVIEMODE_TASEDITOR;
	}
	// if movie length is less or equal to currFrame, pad it with empty frames
	if (((int)currMovieData.records.size() - 1) < currFrameCounter)
	{
		currMovieData.insertEmpty(-1, currFrameCounter - ((int)currMovieData.records.size() - 1));
	}
	// ensure that movie has correct set of ports/fourscore
	setInputType(currMovieData, getInputType(currMovieData));
	// force the input configuration stored in the movie to apply to FCEUX config
	applyMovieInputConfig();
	// reset some modules that need MovieData info
	pianoRoll->reset();
	recorder.reset();
	// create initial snapshot in history
	history.reset();
	// reset Taseditor variables
	mustCallManualLuaFunction = false;
	
	//SetFocus(history.hwndHistoryList);		// set focus only once, to show blue selection cursor
	//SetFocus(pianoRoll.hwndList);
	FCEU_DispMessage("TAS Editor engaged", 0);
	update();
	return 0;
}
//----------------------------------------------------------------------------
void TasEditorWindow::frameUpdate(void)
{
	FCEU_WRAPPER_LOCK();

#if SDL_VERSION_ATLEAST(2, 0, 18)
	tasEditorTimeStamp = SDL_GetTicks64();
#else
	tasEditorTimeStamp = SDL_GetTicks();
#endif

	//printf("TAS Frame Update: %zi   %u\n", currMovieData.records.size(), tasEditorTimeStamp);

	//taseditorWindow.update();
	greenzone.update();
	recorder.update();
	pianoRoll->periodicUpdate();
	markersManager.update();
	playback.update();
	bookmarks.update();
	branches.update();
	//popupDisplay.update();
	selection.update();
	splicer.update();
	history.update();
	project.update();

#ifdef _S9XLUA_H
	// run Lua functions if needed
	if (taseditorConfig.enableLuaAutoFunction)
	{
		TaseditorAutoFunction();
	}
	if (mustCallManualLuaFunction)
	{
		TaseditorManualFunction();
		mustCallManualLuaFunction = false;
	}
#endif

	pianoRoll->update();

	if ( recentProjectMenuReset )
	{
		buildRecentProjectMenu();
		recentProjectMenuReset = false;
	}

	FCEU_WRAPPER_UNLOCK();
}
//----------------------------------------------------------------------------
bool TasEditorWindow::loadProject(const char* fullname)
{
	bool success = false;

	FCEU_WRAPPER_LOCK();

	// try to load project
	if (project.load(fullname))
	{
		// loaded successfully
		applyMovieInputConfig();
		// add new file to Recent menu
		addRecentProject( fullname );
		updateCaption();
		update();
		success = true;
	}
	else
	{
		// failed to load
		updateCaption();
		update();
	}
	FCEU_WRAPPER_UNLOCK();

	return success;
}
bool TasEditorWindow::saveProject(bool save_compact)
{
	bool ret = true;

	FCEU_WRAPPER_LOCK();

	if (project.getProjectFile().empty())
	{
		ret = saveProjectAs(save_compact);
	}
	else
	{
		if (save_compact)
		{
			project.save(0, taseditorConfig.saveCompact_SaveInBinary, taseditorConfig.saveCompact_SaveMarkers, taseditorConfig.saveCompact_SaveBookmarks, taseditorConfig.saveCompact_GreenzoneSavingMode, taseditorConfig.saveCompact_SaveHistory, taseditorConfig.saveCompact_SavePianoRoll, taseditorConfig.saveCompact_SaveSelection);
		}
		else
		{
			project.save(0, taseditorConfig.projectSavingOptions_SaveInBinary, taseditorConfig.projectSavingOptions_SaveMarkers, taseditorConfig.projectSavingOptions_SaveBookmarks, taseditorConfig.projectSavingOptions_GreenzoneSavingMode, taseditorConfig.projectSavingOptions_SaveHistory, taseditorConfig.projectSavingOptions_SavePianoRoll, taseditorConfig.projectSavingOptions_SaveSelection);
		}
		updateCaption();
	}

	FCEU_WRAPPER_UNLOCK();

	return ret;
}

bool TasEditorWindow::saveProjectAs(bool save_compact)
{
	std::string last;
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string lastPath;
	//char dir[512];
	const char *base, *rom;
	QFileDialog  dialog(this, tr("Save TAS Editor Project As") );
	QList<QUrl> urls;
	QDir d;

	dialog.setFileMode(QFileDialog::AnyFile);

	dialog.setNameFilter(tr("TAS Project Files (*.fm3) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Save") );

	base = fceu11::GetBaseDirectory();

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	if ( base )
	{
		urls << QUrl::fromLocalFile( QDir( base ).absolutePath() );

		d.setPath( QString(base) + "/movies");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		dialog.setDirectory( d.absolutePath() );
	}
	dialog.setDefaultSuffix( tr(".fm3") );

	g_config->getOption ("SDL.TasProjectFilePath", &lastPath);
	if ( lastPath.size() > 0 )
	{
		dialog.setDirectory( QString::fromStdString(lastPath) );
	}

	rom = getRomFile();

	if ( rom )
	{
		char baseName[512];
		getFileBaseName( rom, baseName );

		if ( baseName[0] != 0 )
		{
			safe_strcat(baseName, sizeof(baseName), ".fm3");

			dialog.selectFile(baseName);
		}
	}

	// Check config option to use native file dialog or not
	g_config->getOption ("SDL.UseNativeFileDialog", &useNativeFileDialogVal);

	dialog.setOption(QFileDialog::DontUseNativeDialog, !useNativeFileDialogVal);
	dialog.setSidebarUrls(urls);

	ret = dialog.exec();

	if ( ret )
	{
		QStringList fileList;
		fileList = dialog.selectedFiles();

		if ( fileList.size() > 0 )
		{
			filename = fileList[0];
		}
	}

	if ( filename.isNull() )
	{
	   return false;
	}
	QFileInfo fi( filename );

	if ( fi.exists() )
	{
		int ret;
		std::string msg;

		msg = "Pre-existing TAS project file will be overwritten:\n\n" +
			fi.fileName().toStdString() + "\n\nReplace file?";

		ret = QMessageBox::warning( this, QObject::tr("Overwrite Warning"),
				QString::fromStdString(msg), QMessageBox::Yes | QMessageBox::No, QMessageBox::No );

		if ( ret == QMessageBox::No )
		{
			return false;
		}
	}

	project.renameProject( filename.toStdString().c_str(), true);
	if (save_compact)
	{
		project.save( filename.toStdString().c_str(), taseditorConfig.saveCompact_SaveInBinary, taseditorConfig.saveCompact_SaveMarkers, taseditorConfig.saveCompact_SaveBookmarks, taseditorConfig.saveCompact_GreenzoneSavingMode, taseditorConfig.saveCompact_SaveHistory, taseditorConfig.saveCompact_SavePianoRoll, taseditorConfig.saveCompact_SaveSelection);
	}
	else
	{
		project.save( filename.toStdString().c_str(), taseditorConfig.projectSavingOptions_SaveInBinary, taseditorConfig.projectSavingOptions_SaveMarkers, taseditorConfig.projectSavingOptions_SaveBookmarks, taseditorConfig.projectSavingOptions_GreenzoneSavingMode, taseditorConfig.projectSavingOptions_SaveHistory, taseditorConfig.projectSavingOptions_SavePianoRoll, taseditorConfig.projectSavingOptions_SaveSelection);
	}
	addRecentProject( filename.toStdString().c_str() );
	// saved successfully - remove * mark from caption
	project.reset();
	updateCaption();
	return true;
}

// returns false if user doesn't want to exit
bool TasEditorWindow::askToSaveProject(void)
{
	bool changesFound = false;
	if (project.getProjectChanged())
	{
		changesFound = true;
	}

	// ask saving project
	if (changesFound)
	{
		int ans = QMessageBox::question( this, tr("TAS Editor"), tr("Save project changes?"),
				QMessageBox::Yes | QMessageBox::No | QMessageBox::Cancel, QMessageBox::Yes );

		//int answer = MessageBox(taseditorWindow.hwndTASEditor, "Save Project changes?", "TAS Editor", MB_YESNOCANCEL);
		if (ans == QMessageBox::Yes)
		{
			return saveProject();
		}
		return (ans != QMessageBox::Cancel);
	}
	return true;
}
//----------------------------------------------------------------------------
void TasEditorWindow::openProject(void)
{
	std::string last;
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string lastPath;
	//char dir[512];
	const char *base, *rom;
	QFileDialog  dialog(this, tr("Open TAS Editor Project") );
	QList<QUrl> urls;
	QDir d;

	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilter(tr("TAS Project Files (*.fm3) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Open") );

	base = fceu11::GetBaseDirectory();

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	if ( base )
	{
		urls << QUrl::fromLocalFile( QDir( base ).absolutePath() );

		d.setPath( QString(base) + "/movies");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		dialog.setDirectory( d.absolutePath() );
	}
	dialog.setDefaultSuffix( tr(".fm3") );

	g_config->getOption ("SDL.TasProjectFilePath", &lastPath);
	if ( lastPath.size() > 0 )
	{
		dialog.setDirectory( QString::fromStdString(lastPath) );
	}

	rom = getRomFile();

	if ( rom )
	{
		char baseName[512];
		getFileBaseName( rom, baseName );

		if ( baseName[0] != 0 )
		{
			dialog.selectFile(baseName);
		}
	}

	// Check config option to use native file dialog or not
	g_config->getOption ("SDL.UseNativeFileDialog", &useNativeFileDialogVal);

	dialog.setOption(QFileDialog::DontUseNativeDialog, !useNativeFileDialogVal);
	dialog.setSidebarUrls(urls);

	ret = dialog.exec();

	if ( ret )
	{
		QStringList fileList;
		fileList = dialog.selectedFiles();

		if ( fileList.size() > 0 )
		{
			filename = fileList[0];
		}
	}

	if ( filename.isNull() )
	{
	   return;
	}

	loadProject( filename.toStdString().c_str());

	return;
}
//----------------------------------------------------------------------------
void TasEditorWindow::createNewProject(void)
{
	int ret;
	QDialog dialog(this);
	QGroupBox *gbox;
	QVBoxLayout *mainLayout, *vbox;
	QHBoxLayout *hbox;
	QPushButton *okButton, *cancelButton;
	QRadioButton *p1, *p2, *p4;
	QCheckBox    *copyInput, *copyMarkers;
	QLineEdit    *authorEdit;
	static struct NewProjectParameters params;

	if (!askToSaveProject())
	{
		return;
	}
	
	params.inputType = getInputType(currMovieData);
	params.copyCurrentInput = params.copyCurrentMarkers = false;
	if (strlen(taseditorConfig.lastAuthorName) > 0)
	{
		int i=0;
		// convert UTF8 char* string to Unicode wstring
		wchar_t savedAuthorName[AUTHOR_NAME_MAX_LEN] = {0};

		while ( taseditorConfig.lastAuthorName[i] != 0 )
		{
			savedAuthorName[i] = taseditorConfig.lastAuthorName[i]; i++;
		}
		savedAuthorName[i] = 0;
		params.authorName = savedAuthorName;
	}
	else
	{
		params.authorName = L"";
	}

	mainLayout = new QVBoxLayout();
	hbox       = new QHBoxLayout();
	vbox       = new QVBoxLayout();
	gbox       = new QGroupBox( tr("Input Type") );

	mainLayout->addLayout( hbox );
	hbox->addWidget( gbox );
	gbox->setLayout( vbox );

	p1 = new QRadioButton( tr("1 Player") );
	p2 = new QRadioButton( tr("2 Players") );
	p4 = new QRadioButton( tr("4 Score") );

	p1->setChecked( params.inputType == INPUT_TYPE_1P );
	p2->setChecked( params.inputType == INPUT_TYPE_2P );
	p4->setChecked( params.inputType == INPUT_TYPE_FOURSCORE );

	vbox->addWidget( p1 );
	vbox->addWidget( p2 );
	vbox->addWidget( p4 );

	vbox       = new QVBoxLayout();
	hbox->addLayout( vbox );

	copyInput   = new QCheckBox( tr("Copy Input") );
	copyMarkers = new QCheckBox( tr("Copy Markers") );

	vbox->addWidget( copyInput );
	vbox->addWidget( copyMarkers );

	hbox       = new QHBoxLayout();
	mainLayout->addLayout( hbox );

	authorEdit = new QLineEdit();
	hbox->addWidget( new QLabel( tr("Author") ), 1 );
	hbox->addWidget( authorEdit, 5 );

	hbox       = new QHBoxLayout();
	mainLayout->addLayout( hbox );

	okButton     = new QPushButton( tr("Ok") );
	cancelButton = new QPushButton( tr("Cancel") );

	hbox->addWidget( cancelButton, 1 );
	hbox->addStretch( 5 );
	hbox->addWidget( okButton    , 1 );

	okButton->setIcon( style()->standardIcon( QStyle::SP_DialogApplyButton ) );
	cancelButton->setIcon( style()->standardIcon( QStyle::SP_DialogCancelButton ) );

	connect(     okButton, SIGNAL(clicked(void)), &dialog, SLOT(accept(void)) );
	connect( cancelButton, SIGNAL(clicked(void)), &dialog, SLOT(reject(void)) );

	dialog.setLayout( mainLayout );

	dialog.setWindowTitle( tr("Create New Project") );

	okButton->setDefault(true);

	ret = dialog.exec();
	
	if ( p4->isChecked() )
	{
		params.inputType = INPUT_TYPE_FOURSCORE;
	}
	else if ( p2->isChecked() )
	{
		params.inputType = INPUT_TYPE_2P;
	}
	else
	{
		params.inputType = INPUT_TYPE_1P;
	}
	params.copyCurrentInput   = copyInput->isChecked();
	params.copyCurrentMarkers = copyMarkers->isChecked();
	params.authorName = authorEdit->text().toStdWString();

	FCEU_WRAPPER_LOCK();

	if ( QDialog::Accepted == ret )
	{
		FCEUMOV_CreateCleanMovie();
		// apply selected options
		setInputType(currMovieData, params.inputType);
		applyMovieInputConfig();
		if (params.copyCurrentInput)
		{
			// copy Input from current snapshot (from history)
			history.getCurrentSnapshot().inputlog.toMovie(currMovieData);
		}
		if (!params.copyCurrentMarkers)
		{
			markersManager.reset();
		}
		if (params.authorName != L"") currMovieData.comments.push_back(L"author " + params.authorName);
		
		// reset Taseditor
		project.init();			// new project has blank name
		greenzone.reset();
		if (params.copyCurrentInput)
		{
			// copy LagLog from current snapshot (from history)
			greenzone.lagLog = history.getCurrentSnapshot().laglog;
		}
		playback.reset();
		playback.restartPlaybackFromZeroGround();
		bookmarks.reset();
		branches.reset();
		history.reset();
		pianoRoll->reset();
		selection.reset();
		//editor.reset();
		splicer.reset();
		recorder.reset();
		//popupDisplay.reset();
		//taseditorWindow.redraw();
		updateCaption();
		update();
	}
	FCEU_WRAPPER_UNLOCK();
}
//----------------------------------------------------------------------------
void TasEditorWindow::importMovieFile( const char *path )
{
	EMUFILE_FILE ifs( path, "rb");

	// Load Input to temporary moviedata
	MovieData md;
	if (LoadFM2(md, &ifs, ifs.size(), false))
	{
		QFileInfo fi( path );
		// loaded successfully, now register Input changes
		//char drv[512], dir[512], name[1024], ext[512];
		//splitpath(filename.toStdString().c_str(), drv, dir, name, ext);
		//safe_strcat(name, sizeof(name), ext);
		int result = history.registerImport(md, fi.fileName().toStdString().c_str() );
		if (result >= 0)
		{
			greenzone.invalidateAndUpdatePlayback(result);
			greenzone.lagLog.invalidateFromFrame(result);
			// keep current snapshot laglog in touch
			history.getCurrentSnapshot().laglog.invalidateFromFrame(result);
		}
		else
		{
			//MessageBox(taseditorWindow.hwndTASEditor, "Imported movie has the same Input.\nNo changes were made.", "TAS Editor", MB_OK);
		}
	}
	else
	{
		FCEUD_PrintError("Error loading movie data!");
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::importMovieFile(void)
{
	std::string last;
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string lastPath;
	//char dir[512];
	const char *base, *rom;
	QFileDialog  dialog(this, tr("Import Movie File") );
	QList<QUrl> urls;
	QDir d;

	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilter(tr("FCEUX Movie Files (*.fm2) ;; TAS Project Files (*.fm3) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Import") );

	base = fceu11::GetBaseDirectory();

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	if ( base )
	{
		urls << QUrl::fromLocalFile( QDir( base ).absolutePath() );

		d.setPath( QString(base) + "/movies");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		dialog.setDirectory( d.absolutePath() );
	}
	dialog.setDefaultSuffix( tr(".fm2") );

	g_config->getOption ("SDL.TasProjectFilePath", &lastPath);
	if ( lastPath.size() > 0 )
	{
		dialog.setDirectory( QString::fromStdString(lastPath) );
	}

	rom = getRomFile();

	if ( rom )
	{
		char baseName[512];
		getFileBaseName( rom, baseName );

		if ( baseName[0] != 0 )
		{
			dialog.selectFile(baseName);
		}
	}

	// Check config option to use native file dialog or not
	g_config->getOption ("SDL.UseNativeFileDialog", &useNativeFileDialogVal);

	dialog.setOption(QFileDialog::DontUseNativeDialog, !useNativeFileDialogVal);
	dialog.setSidebarUrls(urls);

	ret = dialog.exec();

	if ( ret )
	{
		QStringList fileList;
		fileList = dialog.selectedFiles();

		if ( fileList.size() > 0 )
		{
			filename = fileList[0];
		}
	}

	if ( filename.isNull() )
	{
	   return;
	}

	importMovieFile( filename.toStdString().c_str() );

	//EMUFILE_FILE ifs( filename.toStdString().c_str(), "rb");

	//// Load Input to temporary moviedata
	//MovieData md;
	//if (LoadFM2(md, &ifs, ifs.size(), false))
	//{
	//	QFileInfo fi( filename );
	//	// loaded successfully, now register Input changes
	//	//char drv[512], dir[512], name[1024], ext[512];
	//	//splitpath(filename.toStdString().c_str(), drv, dir, name, ext);
	//	//safe_strcat(name, sizeof(name), ext);
	//	int result = history.registerImport(md, fi.fileName().toStdString().c_str() );
	//	if (result >= 0)
	//	{
	//		greenzone.invalidateAndUpdatePlayback(result);
	//		greenzone.lagLog.invalidateFromFrame(result);
	//		// keep current snapshot laglog in touch
	//		history.getCurrentSnapshot().laglog.invalidateFromFrame(result);
	//	}
	//	else
	//	{
	//		//MessageBox(taseditorWindow.hwndTASEditor, "Imported movie has the same Input.\nNo changes were made.", "TAS Editor", MB_OK);
	//	}
	//}
	//else
	//{
	//	FCEUD_PrintError("Error loading movie data!");
	//}

	return;
}
//----------------------------------------------------------------------------
void TasEditorWindow::exportMovieFile(void)
{
	std::string last;
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string lastPath;
	//char dir[512];
	const char *base, *rom;
	QFileDialog  dialog(this, tr("Export to FM2 File") );
	QList<QUrl> urls;
	QDir d;

	dialog.setFileMode(QFileDialog::AnyFile);

	dialog.setNameFilter(tr("FCEUX Movie File (*.fm2) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Export") );

	base = fceu11::GetBaseDirectory();

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	if ( base )
	{
		urls << QUrl::fromLocalFile( QDir( base ).absolutePath() );

		d.setPath( QString(base) + "/movies");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		dialog.setDirectory( d.absolutePath() );
	}
	dialog.setDefaultSuffix( tr(".fm2") );

	g_config->getOption ("SDL.TasProjectFilePath", &lastPath);
	if ( lastPath.size() > 0 )
	{
		dialog.setDirectory( QString::fromStdString(lastPath) );
	}

	rom = getRomFile();

	if ( rom )
	{
		char baseName[512];
		getFileBaseName( rom, baseName );

		if ( baseName[0] != 0 )
		{
			dialog.selectFile(baseName);
		}
	}

	// Check config option to use native file dialog or not
	g_config->getOption ("SDL.UseNativeFileDialog", &useNativeFileDialogVal);

	dialog.setOption(QFileDialog::DontUseNativeDialog, !useNativeFileDialogVal);
	dialog.setSidebarUrls(urls);

	ret = dialog.exec();

	if ( ret )
	{
		QStringList fileList;
		fileList = dialog.selectedFiles();

		if ( fileList.size() > 0 )
		{
			filename = fileList[0];
		}
	}

	if ( filename.isNull() )
	{
	   return;
	}

	EMUFILE* osRecordingMovie = FCEUD_UTF8_fstream( filename.toStdString().c_str(), "wb");
	// create copy of current movie data
	MovieData temp_md = currMovieData;
	// modify the copy according to selected type of export
	setInputType(temp_md, taseditorConfig.lastExportedInputType);
	temp_md.loadFrameCount = -1;
	// also add subtitles if needed
	if (taseditorConfig.lastExportedSubtitlesStatus)
	{
		// convert Marker Notes to Movie Subtitles
		char framenum[16];
		std::string subtitle;
		int markerID;
		for (int i = 0; i < markersManager.getMarkersArraySize(); ++i)
		{
			markerID = markersManager.getMarkerAtFrame(i);
			if (markerID)
			{
				snprintf(framenum, sizeof(framenum), "%i ", i );
				//_itoa(i, framenum, 10);
				//safe_strcat(framenum, sizeof(framenum), " ");
				subtitle = framenum;
				subtitle.append(markersManager.getNoteCopy(markerID));
				temp_md.subtitles.push_back(subtitle);
			}
		}
	}
	// dump to disk
	temp_md.dump(osRecordingMovie, false);
	delete osRecordingMovie;
	osRecordingMovie = 0;

}
//----------------------------------------------------------------------------
void TasEditorWindow::updateCaption(void)
{
	char newCaption[300];
	FCEU_strlcpy(newCaption, sizeof(newCaption), "TAS Editor");
	if (!movie_readonly)
	{
		safe_strcat(newCaption, sizeof(newCaption), recorder.getRecordingCaption());
	}
	// add project name
	std::string projectname = project.getProjectName();
	if (!projectname.empty())
	{
		safe_strcat(newCaption, sizeof(newCaption), " - ");
		safe_strcat(newCaption, sizeof(newCaption), projectname.c_str());
	}
	// and * if project has unsaved changes
	if (project.getProjectChanged())
	{
		safe_strcat(newCaption, sizeof(newCaption), "*");
	}
	setWindowTitle( tr(newCaption) );
	//SetWindowText(hwndTASEditor, newCaption);
}
//----------------------------------------------------------------------------
void TasEditorWindow::clearProjectList(void)
{
	std::list <std::string*>::iterator it;

	for (it=projList.begin(); it != projList.end(); it++)
	{
		delete *it;
	}
	projList.clear();
}
//----------------------------------------------------------------------------
void TasEditorWindow::buildRecentProjectMenu(void)
{
	QAction *act;
	std::string s;
	std::string *sptr;
	char buf[128];

	clearProjectList();
	recentProjectMenu->clear();

	for (int i=0; i<10; i++)
	{
		snprintf(buf, sizeof(buf), "SDL.RecentTasProject%02i", i);

		g_config->getOption( buf, &s);

		//printf("Recent Rom:%i  '%s'\n", i, s.c_str() );

		if ( s.size() > 0 )
		{
			act = new TasRecentProjectAction( tr(s.c_str()), recentProjectMenu);

			recentProjectMenu->addAction( act );

			connect(act, SIGNAL(triggered()), act, SLOT(activateCB(void)) );

			sptr = new std::string();

			sptr->assign( s.c_str() );

			projList.push_front( sptr );
		}
	}
	recentProjectMenu->setEnabled( !recentProjectMenu->isEmpty() );
}
//---------------------------------------------------------------------------
void TasEditorWindow::saveRecentProjectMenu(void)
{
	int i;
	std::string *s;
	std::list <std::string*>::iterator it;
	char buf[128];

	i = projList.size() - 1;

	for (it=projList.begin(); it != projList.end(); it++)
	{
		s = *it;
		snprintf(buf, sizeof(buf), "SDL.RecentTasProject%02i", i);

		g_config->setOption( buf, s->c_str() );

		//printf("Recent Rom:%u  '%s'\n", i, s->c_str() );
		i--;
	}
}
//---------------------------------------------------------------------------
void TasEditorWindow::addRecentProject( const char *proj )
{
	std::string *s;
	std::list <std::string*>::iterator match_it;

	for (match_it=projList.begin(); match_it != projList.end(); match_it++)
	{
		s = *match_it;

		if ( s->compare( proj ) == 0 )
		{
			//printf("Found Match: %s\n", proj );
			break;
		}
	}

	if ( match_it != projList.end() )
	{
		s = *match_it;

		projList.erase(match_it);

		projList.push_back(s);
	}
	else
	{
		s = new std::string();

		s->assign( proj );
		
		projList.push_back(s);

		if ( projList.size() > 10 )
		{
			s = projList.front();

			projList.pop_front();

			delete s;
		}
	}

	saveRecentProjectMenu();

	recentProjectMenuReset = true;
}
//----------------------------------------------------------------------------
void TasEditorWindow::saveProjectCb(void)
{
	saveProject();
}
//----------------------------------------------------------------------------
void TasEditorWindow::saveProjectAsCb(void)
{
	saveProjectAs();
}
//----------------------------------------------------------------------------
bool TasEditorWindow::saveCompactGetFilename( QString &outputFilePath )
{
	std::string last;
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string lastPath;
	//char dir[512];
	const char *base, *rom;
	QFileDialog  dialog(this, tr("Save Compact TAS Editor Project As") );
	QList<QUrl> urls;
	QDir d;

	dialog.setFileMode(QFileDialog::AnyFile);

	dialog.setNameFilter(tr("TAS Project Files (*.fm3) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Save") );

	base = fceu11::GetBaseDirectory();

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	if ( base )
	{
		urls << QUrl::fromLocalFile( QDir( base ).absolutePath() );

		d.setPath( QString(base) + "/movies");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		dialog.setDirectory( d.absolutePath() );
	}
	dialog.setDefaultSuffix( tr(".fm3") );

	g_config->getOption ("SDL.TasProjectFilePath", &lastPath);
	if ( lastPath.size() > 0 )
	{
		dialog.setDirectory( QString::fromStdString(lastPath) );
	}

	rom = getRomFile();

	if (!project.getProjectName().empty())
	{
		char baseName[512];

		FCEU_strlcpy(baseName, sizeof(baseName), project.getProjectName().c_str());

		if (strstr(baseName, "-compact") == NULL)
		{
			safe_strcat(baseName, sizeof(baseName), "-compact");
		}
		safe_strcat(baseName, sizeof(baseName), ".fm3");

		dialog.selectFile(baseName);
	}
	else if ( rom )
	{
		char baseName[512];
		getFileBaseName( rom, baseName );

		if ( baseName[0] != 0 )
		{
			if (strstr(baseName, "-compact") == NULL)
			{
				safe_strcat(baseName, sizeof(baseName), "-compact");
			}
			safe_strcat(baseName, sizeof(baseName), ".fm3");

			dialog.selectFile(baseName);
		}
	}

	// Check config option to use native file dialog or not
	g_config->getOption ("SDL.UseNativeFileDialog", &useNativeFileDialogVal);

	dialog.setOption(QFileDialog::DontUseNativeDialog, !useNativeFileDialogVal);
	dialog.setSidebarUrls(urls);

	ret = dialog.exec();

	if ( ret )
	{
		QStringList fileList;
		fileList = dialog.selectedFiles();

		if ( fileList.size() > 0 )
		{
			filename = fileList[0];
		}
	}

	if ( filename.isNull() )
	{
	   return false;
	}
	QFileInfo fi( filename );

	if ( fi.exists() )
	{
		int ret;
		std::string msg;

		msg = "Pre-existing TAS project file will be overwritten:\n\n" +
			fi.fileName().toStdString() + "\n\nReplace file?";

		ret = QMessageBox::warning( this, QObject::tr("Overwrite Warning"),
				QString::fromStdString(msg), QMessageBox::Yes | QMessageBox::No, QMessageBox::No );

		if ( ret == QMessageBox::No )
		{
			return false;
		}
	}
	outputFilePath = filename;


	return true;
}
//----------------------------------------------------------------------------
void TasEditorWindow::saveProjectCompactCb(void)
{
	int ret;
	QDialog dialog(this);
	FCEU_CRITICAL_SECTION(emuLock);
	QGroupBox *fileContentsBox, *greenZoneSaveBox;
	QVBoxLayout *mainLayout, *vbox1, *vbox;
	QHBoxLayout *hbox;
	QCheckBox *binaryInput, *saveMarkers, *saveBookmarks;
	QCheckBox *saveHistory, *savePianoRoll, *saveSelection;
	QRadioButton *allFrames, *every16thFrame, *markedFrames, *dontSave;
	QPushButton  *okButton, *cancelButton;

	dialog.setWindowTitle( tr("Save Compact") );

	mainLayout       = new QVBoxLayout();
	fileContentsBox  = new QGroupBox( tr("File Contents") );
	greenZoneSaveBox = new QGroupBox( tr("Greenzone Saving Options") );

	binaryInput    = new QCheckBox( tr("Binary Input") );
	saveMarkers    = new QCheckBox( tr("Markers") );
	saveBookmarks  = new QCheckBox( tr("Bookmarks") );
	saveHistory    = new QCheckBox( tr("History") );
	savePianoRoll  = new QCheckBox( tr("Piano Roll") );
	saveSelection  = new QCheckBox( tr("Selection") );

	allFrames      = new QRadioButton( tr("All Frames") );
	every16thFrame = new QRadioButton( tr("Every 16th Frame") );
	markedFrames   = new QRadioButton( tr("Marked Frame") );
	dontSave       = new QRadioButton( tr("Don't Save") );

	okButton       = new QPushButton( tr("Ok") );
	cancelButton   = new QPushButton( tr("Cancel") );

	okButton->setIcon( style()->standardIcon( QStyle::SP_DialogApplyButton ) );
	cancelButton->setIcon( style()->standardIcon( QStyle::SP_DialogCancelButton ) );

	connect(     okButton, SIGNAL(clicked(void)), &dialog, SLOT(accept(void)) );
	connect( cancelButton, SIGNAL(clicked(void)), &dialog, SLOT(reject(void)) );

	vbox1 = new QVBoxLayout();

	dialog.setLayout( mainLayout );
	mainLayout->addWidget( fileContentsBox );

	fileContentsBox->setLayout( vbox1 );

	vbox1->addWidget( binaryInput    );
	vbox1->addWidget( saveMarkers    );
	vbox1->addWidget( saveBookmarks  );
	vbox1->addWidget( saveHistory    );
	vbox1->addWidget( savePianoRoll  );
	vbox1->addWidget( saveSelection  );
	vbox1->addWidget( greenZoneSaveBox );

	vbox  = new QVBoxLayout();
	greenZoneSaveBox->setLayout( vbox );

	vbox->addWidget( allFrames      );
	vbox->addWidget( every16thFrame );
	vbox->addWidget( markedFrames   );
	vbox->addWidget( dontSave       );

	hbox = new QHBoxLayout();
	mainLayout->addLayout( hbox );
	hbox->addStretch(5);
	hbox->addWidget( okButton );
	hbox->addWidget( cancelButton );

	binaryInput->setChecked( taseditorConfig.saveCompact_SaveInBinary );
	saveMarkers->setChecked( taseditorConfig.saveCompact_SaveMarkers );
	saveBookmarks->setChecked( taseditorConfig.saveCompact_SaveBookmarks );
	saveHistory->setChecked( taseditorConfig.saveCompact_SaveHistory );
	savePianoRoll->setChecked( taseditorConfig.saveCompact_SavePianoRoll );
	saveSelection->setChecked( taseditorConfig.saveCompact_SaveSelection );

	     allFrames->setChecked( taseditorConfig.saveCompact_GreenzoneSavingMode == GREENZONE_SAVING_MODE_ALL );
	every16thFrame->setChecked( taseditorConfig.saveCompact_GreenzoneSavingMode == GREENZONE_SAVING_MODE_16TH );
	  markedFrames->setChecked( taseditorConfig.saveCompact_GreenzoneSavingMode == GREENZONE_SAVING_MODE_MARKED );
	      dontSave->setChecked( taseditorConfig.saveCompact_GreenzoneSavingMode == GREENZONE_SAVING_MODE_NO );

	okButton->setDefault(true);

	ret = dialog.exec();

	if ( ret == QDialog::Accepted )
	{
		QString filename;

		taseditorConfig.saveCompact_SaveInBinary  = binaryInput->isChecked();
		taseditorConfig.saveCompact_SaveMarkers   = saveMarkers->isChecked();
		taseditorConfig.saveCompact_SaveBookmarks = saveBookmarks->isChecked();
		taseditorConfig.saveCompact_SaveHistory   = saveHistory->isChecked();
		taseditorConfig.saveCompact_SavePianoRoll = savePianoRoll->isChecked();
		taseditorConfig.saveCompact_SaveSelection = saveSelection->isChecked();

		if ( allFrames->isChecked() )
		{
			taseditorConfig.saveCompact_GreenzoneSavingMode = GREENZONE_SAVING_MODE_ALL;
		}
		else if ( every16thFrame->isChecked() )
		{
			taseditorConfig.saveCompact_GreenzoneSavingMode = GREENZONE_SAVING_MODE_16TH;
		}
		else if ( markedFrames->isChecked() )
		{
			taseditorConfig.saveCompact_GreenzoneSavingMode = GREENZONE_SAVING_MODE_MARKED;
		}
		else
		{
			taseditorConfig.saveCompact_GreenzoneSavingMode = GREENZONE_SAVING_MODE_NO;
		}

		if ( saveCompactGetFilename( filename ) )
		{
			project.save(filename.toStdString().c_str(), taseditorConfig.saveCompact_SaveInBinary, taseditorConfig.saveCompact_SaveMarkers, taseditorConfig.saveCompact_SaveBookmarks, taseditorConfig.saveCompact_GreenzoneSavingMode, taseditorConfig.saveCompact_SaveHistory, taseditorConfig.saveCompact_SavePianoRoll, taseditorConfig.saveCompact_SaveSelection);
		}
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::openOnlineDocs(void)
{
	if ( QDesktopServices::openUrl( QUrl("https://fceux.com/web/help/taseditor/Title.html") ) == false )
	{
		QMessageBox::critical( this, tr("Error"), 
		                        tr("Error: Failed to open link to: https://fceux.com/web/help/taseditor/Title.html") );
	}
	return;
}
//----------------------------------------------------------------------------
void TasEditorWindow::setCurrentPattern(int idx)
{
	if ( idx < 0 )
	{
		return;
	}
	if ( (size_t)idx >= patternsNames.size() )
	{
		return;
	}
	//printf("Set Pattern: %i\n", idx);
	taseditorConfig.currentPattern = idx;
}
//----------------------------------------------------------------------------
void TasEditorWindow::recordingChanged(int newState)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int oldState = !movie_readonly ? Qt::Checked : Qt::Unchecked;

	if ( newState != oldState )
	{
		fceu11::MovieToggleReadOnly();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::editUndoCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	history.undo();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editRedoCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	history.redo();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editUndoSelCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	int dragMode = pianoRoll->getDragMode();

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.undo();
		pianoRoll->followSelection();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::editRedoSelCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	int dragMode = pianoRoll->getDragMode();

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.redo();
		pianoRoll->followSelection();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::editDeselectAll(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	int dragMode = pianoRoll->getDragMode();

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.clearAllRowsSelection();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::editSelectAll(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	int dragMode = pianoRoll->getDragMode();

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.selectAllRows();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::editSelBtwMkrs(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	int dragMode = pianoRoll->getDragMode();

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.selectAllRowsBetweenMarkers();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::editReselectClipboard(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	int dragMode = pianoRoll->getDragMode();

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.reselectClipboard();
		pianoRoll->followSelection();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::editCutCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.cutSelectedInputToClipboard();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editCopyCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.copySelectedInputToClipboard();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editPasteCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.pasteInputFromClipboard();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editPasteInsertCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.pasteInsertInputFromClipboard();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editClearCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.clearSelectedFrames();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editDeleteCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.deleteSelectedFrames();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editCloneCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.cloneSelectedFrames();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editInsertCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.insertSelectedFrames();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editInsertNumFramesCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.insertNumberOfFrames();
}
//----------------------------------------------------------------------------
void TasEditorWindow::editTruncateMovieCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	splicer.truncateMovie();
}
//----------------------------------------------------------------------------
void TasEditorWindow::superImposedChanged(int state)
{
	if ( state == Qt::Checked )
	{
		taseditorConfig.superimpose = SUPERIMPOSE_CHECKED;
	}
	else if ( state == Qt::PartiallyChecked )
	{
		taseditorConfig.superimpose = SUPERIMPOSE_INDETERMINATE;
	}
	else
	{
		taseditorConfig.superimpose = SUPERIMPOSE_UNCHECKED;
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::usePatternChanged(int state)
{
	taseditorConfig.recordingUsePattern ^= 1;
	recorder.patternOffset = 0;
}
//----------------------------------------------------------------------------
void TasEditorWindow::recordInputChanged(int input)
{
	//printf("Input Change: %i\n", input);
	recorder.multitrackRecordingJoypadNumber = input;
}
//----------------------------------------------------------------------------
void TasEditorWindow::openFindNoteWindow(void)
{
	if ( findWin )
	{
		findWin->activateWindow();
		findWin->raise();
	}
	else
	{
		findWin = new TasFindNoteWindow(this);
		findWin->show();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::dpyBrnchScrnChanged(bool val)
{
	taseditorConfig.displayBranchScreenshots = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::dpyBrnchDescChanged(bool val)
{
	taseditorConfig.displayBranchDescriptions = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::enaHotChgChanged(bool val)
{
	taseditorConfig.enableHotChanges = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::followUndoActChanged(bool val)
{
	taseditorConfig.followUndoContext = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::followMkrActChanged(bool val)
{
	taseditorConfig.followMarkerNoteContext = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::enaGrnznActChanged(bool val)
{
	taseditorConfig.enableGreenzoning = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::afPtrnSkipLagActChanged(bool val)
{
	taseditorConfig.autofirePatternSkipsLag = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::adjInputLagActChanged(bool val)
{
	taseditorConfig.autoAdjustInputAccordingToLag = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::drawInputDragActChanged(bool val)
{
	taseditorConfig.drawInputByDragging = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::cmbRecDrawActChanged(bool val)
{
	taseditorConfig.combineConsecutiveRecordingsAndDraws = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::use1PforRecActChanged(bool val)
{
	taseditorConfig.use1PKeysForAllSingleRecordings = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::useInputColSetActChanged(bool val)
{
	taseditorConfig.useInputKeysForColumnSet = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::bindMkrInputActChanged(bool val)
{
	taseditorConfig.bindMarkersToInput = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::emptyNewMkrNotesActChanged(bool val)
{
	taseditorConfig.emptyNewMarkerNotes = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::oldCtlBrnhSchemeActChanged(bool val)
{
	taseditorConfig.oldControlSchemeForBranching = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::brnchRestoreMovieActChanged(bool val)
{
	taseditorConfig.branchesRestoreEntireMovie = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::hudInScrnBranchActChanged(bool val)
{
	taseditorConfig.HUDInBranchScreenshots = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::pauseAtEndActChanged(bool val)
{
	taseditorConfig.autopauseAtTheEndOfMovie = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::manLuaRun(void)
{
	mustCallManualLuaFunction = true;
}
//----------------------------------------------------------------------------
void TasEditorWindow::autoLuaRunChanged(bool val)
{
	taseditorConfig.enableLuaAutoFunction = val;
}
//----------------------------------------------------------------------------
void TasEditorWindow::showToolTipsActChanged(bool val)
{
	taseditorConfig.tooltipsEnabled = val;

	updateToolTips();
}
//----------------------------------------------------------------------------
void TasEditorWindow::updateToolTips(void)
{
	if ( taseditorConfig.tooltipsEnabled )
	{
		upperMarkerLabel->setToolTip( tr("Click here to scroll Piano Roll to Playback cursor") );
		lowerMarkerLabel->setToolTip( tr("Click here to scroll Piano Roll to Selection") );
		upperMarkerNote->setToolTip( tr("Click to edit text") );
		lowerMarkerNote->setToolTip( tr("Click to edit text") );

		recRecordingCbox->setToolTip( tr("Switch Input Recording on/off") );
		recSuperImposeCbox->setToolTip( tr("Allows to superimpose old Input with new buttons, instead of overwriting") );
		recUsePatternCbox->setToolTip( tr("Applies current Autofire Pattern to Input recording") );
		recAllBtn->setToolTip( tr("Switch off Multitracking") );
		rec1PBtn->setToolTip( tr("Select Joypad 1 as Current") );
		rec2PBtn->setToolTip( tr("Select Joypad 2 as Current") );
		rec3PBtn->setToolTip( tr("Select Joypad 3 as Current") );
		rec4PBtn->setToolTip( tr("Select Joypad 4 as Current") );

		rewindMkrBtn->setToolTip( tr("Send Playback to previous Marker (mouse: Shift+Wheel up) (hotkey: Shift+PageUp)") );
		rewindFrmBtn->setToolTip( tr("Rewind 1 frame (mouse: Right button+Wheel up) (hotkey: Shift+Up)") );
		playPauseBtn->setToolTip( tr("Pause/Unpause Emulation (mouse: Middle button)") );
		   advFrmBtn->setToolTip( tr("Advance 1 frame (mouse: Right button+Wheel down) (hotkey: Shift+Down)") );
		   advMkrBtn->setToolTip( tr("Send Playback to next Marker (mouse: Shift+Wheel down) (hotkey: Shift+PageDown)") );

		followCursorCbox->setToolTip( tr("The Piano Roll will follow Playback cursor movements") );
		   turboSeekCbox->setToolTip( tr("Uncheck when you need to watch seeking in slow motion") );
		 autoRestoreCbox->setToolTip( tr("Whenever you change Input above Playback cursor, the cursor returns to where it was before the change") );

		 selectionLbl->setToolTip( tr("Current size of Selection") );
		 clipboardLbl->setToolTip( tr("Current size of Input in the Clipboard") );

		 prevMkrBtn->setToolTip( tr("Send Selection to previous Marker (mouse: Ctrl+Wheel up) (hotkey: Ctrl+PageUp)") );
		 nextMkrBtn->setToolTip( tr("Send Selection to next Marker (mouse: Ctrl+Wheel up) (hotkey: Ctrl+PageDown)") );
		 similarBtn->setToolTip( tr("Auto-search for Marker Note") );
		    moreBtn->setToolTip( tr("Continue Auto-search") );
	}
	else
	{
		upperMarkerLabel->setToolTip( tr("") );
		lowerMarkerLabel->setToolTip( tr("") );
		upperMarkerNote->setToolTip( tr("") );
		lowerMarkerNote->setToolTip( tr("") );

		recRecordingCbox->setToolTip( tr("") );
		recSuperImposeCbox->setToolTip( tr("") );
		recUsePatternCbox->setToolTip( tr("") );
		recAllBtn->setToolTip( tr("") );
		rec1PBtn->setToolTip( tr("") );
		rec2PBtn->setToolTip( tr("") );
		rec3PBtn->setToolTip( tr("") );
		rec4PBtn->setToolTip( tr("") );

		rewindMkrBtn->setToolTip( tr("") );
		rewindFrmBtn->setToolTip( tr("") );
		playPauseBtn->setToolTip( tr("") );
		   advFrmBtn->setToolTip( tr("") );
		   advMkrBtn->setToolTip( tr("") );

		followCursorCbox->setToolTip( tr("") );
		   turboSeekCbox->setToolTip( tr("") );
		 autoRestoreCbox->setToolTip( tr("") );

		 selectionLbl->setToolTip( tr("") );
		 clipboardLbl->setToolTip( tr("") );

		 prevMkrBtn->setToolTip( tr("") );
		 nextMkrBtn->setToolTip( tr("") );
		 similarBtn->setToolTip( tr("") );
		    moreBtn->setToolTip( tr("") );
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::changePianoRollFontCB(void)
{
	bool ok = false;

	QFont selFont = QFontDialog::getFont( &ok, pianoRoll->QWidget::font(), this, tr("Select Font"), QFontDialog::MonospacedFonts );

	if ( ok )
	{
		pianoRoll->setFont( selFont );

		//printf("Font Changed to: '%s'\n", selFont.toString().toStdString().c_str() );

		g_config->setOption("SDL.TasPianoRollFont", selFont.toString().toStdString().c_str() );
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::changeBookmarksFontCB(void)
{
	bool ok = false;

	QFont selFont = QFontDialog::getFont( &ok, bookmarks.QWidget::font(), this, tr("Select Font"), QFontDialog::MonospacedFonts );

	if ( ok )
	{
		bookmarks.setFont( selFont );

		//printf("Font Changed to: '%s'\n", selFont.toString().toStdString().c_str() );

		g_config->setOption("SDL.TasBookmarksFont", selFont.toString().toStdString().c_str() );
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::changeBranchesFontCB(void)
{
	bool ok = false;

	QFont selFont = QFontDialog::getFont( &ok, branches.QWidget::font(), this, tr("Select Font"), QFontDialog::MonospacedFonts );

	if ( ok )
	{
		branches.setFont( selFont );

		//printf("Font Changed to: '%s'\n", selFont.toString().toStdString().c_str() );

		g_config->setOption("SDL.TasBranchesFont", selFont.toString().toStdString().c_str() );
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::playbackPauseCB(void)
{
	FCEU_CRITICAL_SECTION( emuLock );
	playback.toggleEmulationPause();
	pianoRoll->update();
}
//----------------------------------------------------------------------------
void TasEditorWindow::playbackFrameRewind(void)
{
	FCEU_CRITICAL_SECTION( emuLock );
	playback.handleRewindFrame();
	pianoRoll->update();
}
//----------------------------------------------------------------------------
void TasEditorWindow::playbackFrameForward(void)
{
	FCEU_CRITICAL_SECTION( emuLock );
	playback.handleForwardFrame();
	pianoRoll->update();
}
//----------------------------------------------------------------------------
void TasEditorWindow::playbackFrameRewindFull(void)
{
	FCEU_CRITICAL_SECTION( emuLock );
	playback.handleRewindFull();
	pianoRoll->update();
}
//----------------------------------------------------------------------------
void TasEditorWindow::playbackFrameForwardFull(void)
{
	FCEU_CRITICAL_SECTION( emuLock );
	playback.handleForwardFull();
	pianoRoll->update();
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::playbackFollowCursorCb(bool val)
{
	taseditorConfig.followPlaybackCursor = val;

	if ( val )
	{
		pianoRoll->ensureTheLineIsVisible( currFrameCounter );
	}
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::playbackTurboSeekCb(bool val)
{
	FCEU_CRITICAL_SECTION( emuLock );

	taseditorConfig.turboSeek = val;

	// if currently seeking, apply this option immediately
	if (playback.getPauseFrame() >= 0)
	{
		turbo = taseditorConfig.turboSeek;
	}
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::playbackAutoRestoreCb(bool val)
{
	taseditorConfig.autoRestoreLastPlaybackPosition = val;
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::scrollSelectionUpOne(void)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int dragMode = pianoRoll->getDragMode();

	//printf("DragMode: %i\n", dragMode);

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.transposeVertically(-1);
		int selectionBeginning = selection.getCurrentRowsSelectionBeginning();
		if (selectionBeginning >= 0)
		{
			pianoRoll->ensureTheLineIsVisible(selectionBeginning);
		}
		pianoRoll->update();
	}
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::scrollSelectionDnOne(void)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int dragMode = pianoRoll->getDragMode();

	//printf("DragMode: %i\n", dragMode);

	if ( (dragMode != DRAG_MODE_SELECTION) && (dragMode != DRAG_MODE_DESELECTION) )
	{
		selection.transposeVertically(1);
		int selectionEnd = selection.getCurrentRowsSelectionEnd();
		if (selectionEnd >= 0)
		{
			pianoRoll->ensureTheLineIsVisible(selectionEnd);
		}
		pianoRoll->update();
	}
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::histTreeItemActivated(QTreeWidgetItem *item, int col)
{
	int row = histTree->indexOfTopLevelItem(item);

	if ( row < 0 )
	{
		return;
	}
	FCEU_CRITICAL_SECTION( emuLock );
	history.handleSingleClick(row);
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::tabViewChanged(int idx)
{
	FCEU_CRITICAL_SECTION( emuLock );
	taseditorConfig.displayBranchesTree = (idx == 1);
	bookmarks.redrawBookmarksSectionCaption();
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::openProjectSaveOptions(void)
{
	int ret;
	QDialog dialog(this);
	FCEU_CRITICAL_SECTION( emuLock );
	QGroupBox *settingsBox, *fileContentsBox, *greenZoneSaveBox;
	QVBoxLayout *mainLayout, *vbox1, *vbox;
	QHBoxLayout *hbox1, *hbox;
	QCheckBox *autoSaveOpt, *saveSilentOpt;
	QSpinBox  *autoSavePeriod;
	QCheckBox *binaryInput, *saveMarkers, *saveBookmarks;
	QCheckBox *saveHistory, *savePianoRoll, *saveSelection;
	QRadioButton *allFrames, *every16thFrame, *markedFrames, *dontSave;
	QPushButton  *okButton, *cancelButton;

	dialog.setWindowTitle( tr("Project File Saving Options") );

	mainLayout       = new QVBoxLayout();
	settingsBox      = new QGroupBox( tr("Settings") );
	fileContentsBox  = new QGroupBox( tr("File Contents") );
	greenZoneSaveBox = new QGroupBox( tr("Greenzone Saving Options") );
	hbox1            = new QHBoxLayout();

	autoSaveOpt    = new QCheckBox( tr("Autosave project") );
	saveSilentOpt  = new QCheckBox( tr("silently") );
	autoSavePeriod = new QSpinBox();

	binaryInput    = new QCheckBox( tr("Binary Input") );
	saveMarkers    = new QCheckBox( tr("Markers") );
	saveBookmarks  = new QCheckBox( tr("Bookmarks") );
	saveHistory    = new QCheckBox( tr("History") );
	savePianoRoll  = new QCheckBox( tr("Piano Roll") );
	saveSelection  = new QCheckBox( tr("Selection") );

	allFrames      = new QRadioButton( tr("All Frames") );
	every16thFrame = new QRadioButton( tr("Every 16th Frame") );
	markedFrames   = new QRadioButton( tr("Marked Frame") );
	dontSave       = new QRadioButton( tr("Don't Save") );

	okButton       = new QPushButton( tr("Ok") );
	cancelButton   = new QPushButton( tr("Cancel") );

	okButton->setIcon( style()->standardIcon( QStyle::SP_DialogApplyButton ) );
	cancelButton->setIcon( style()->standardIcon( QStyle::SP_DialogCancelButton ) );

	connect(     okButton, SIGNAL(clicked(void)), &dialog, SLOT(accept(void)) );
	connect( cancelButton, SIGNAL(clicked(void)), &dialog, SLOT(reject(void)) );

	hbox1->addWidget( settingsBox );
	hbox1->addWidget( fileContentsBox );

	dialog.setLayout( mainLayout );
	mainLayout->addLayout( hbox1 );

	vbox = new QVBoxLayout();
	hbox = new QHBoxLayout();
	settingsBox->setLayout( vbox );

	hbox->addWidget( new QLabel( tr("every") ) );
	hbox->addWidget( autoSavePeriod );
	hbox->addWidget( new QLabel( tr("minutes") ) );

	vbox->addWidget( autoSaveOpt );
	vbox->addLayout( hbox );
	vbox->addWidget( saveSilentOpt );
	vbox->addStretch( 10 );

	vbox1 = new QVBoxLayout();
	fileContentsBox->setLayout( vbox1 );

	vbox1->addWidget( binaryInput    );
	vbox1->addWidget( saveMarkers    );
	vbox1->addWidget( saveBookmarks  );
	vbox1->addWidget( saveHistory    );
	vbox1->addWidget( savePianoRoll  );
	vbox1->addWidget( saveSelection  );
	vbox1->addWidget( greenZoneSaveBox );

	vbox  = new QVBoxLayout();
	greenZoneSaveBox->setLayout( vbox );

	vbox->addWidget( allFrames      );
	vbox->addWidget( every16thFrame );
	vbox->addWidget( markedFrames   );
	vbox->addWidget( dontSave       );

	hbox1 = new QHBoxLayout();
	mainLayout->addLayout( hbox1 );
	hbox1->addStretch(5);
	hbox1->addWidget( okButton );
	hbox1->addWidget( cancelButton );

	autoSavePeriod->setRange( AUTOSAVE_PERIOD_MIN, AUTOSAVE_PERIOD_MAX );

	autoSaveOpt->setChecked( taseditorConfig.autosaveEnabled );
	autoSavePeriod->setValue( taseditorConfig.autosavePeriod );
	saveSilentOpt->setChecked( taseditorConfig.autosaveSilent );

	autoSavePeriod->setEnabled( taseditorConfig.autosaveEnabled );
	saveSilentOpt->setEnabled( taseditorConfig.autosaveEnabled );

	binaryInput->setChecked( taseditorConfig.projectSavingOptions_SaveInBinary );
	saveMarkers->setChecked( taseditorConfig.projectSavingOptions_SaveMarkers );
	saveBookmarks->setChecked( taseditorConfig.projectSavingOptions_SaveBookmarks );
	saveHistory->setChecked( taseditorConfig.projectSavingOptions_SaveHistory );
	savePianoRoll->setChecked( taseditorConfig.projectSavingOptions_SavePianoRoll );
	saveSelection->setChecked( taseditorConfig.projectSavingOptions_SaveSelection );

	     allFrames->setChecked( taseditorConfig.projectSavingOptions_GreenzoneSavingMode == GREENZONE_SAVING_MODE_ALL );
	every16thFrame->setChecked( taseditorConfig.projectSavingOptions_GreenzoneSavingMode == GREENZONE_SAVING_MODE_16TH );
	  markedFrames->setChecked( taseditorConfig.projectSavingOptions_GreenzoneSavingMode == GREENZONE_SAVING_MODE_MARKED );
	      dontSave->setChecked( taseditorConfig.projectSavingOptions_GreenzoneSavingMode == GREENZONE_SAVING_MODE_NO );

	connect( autoSaveOpt, SIGNAL(clicked(bool)), autoSavePeriod, SLOT(setEnabled(bool)) );
	connect( autoSaveOpt, SIGNAL(clicked(bool)), saveSilentOpt , SLOT(setEnabled(bool)) );

	okButton->setDefault(true);

	ret = dialog.exec();

	if ( ret == QDialog::Accepted )
	{
		taseditorConfig.autosaveEnabled = autoSaveOpt->isChecked();
		taseditorConfig.autosavePeriod  = autoSavePeriod->value();
		taseditorConfig.autosaveSilent  = saveSilentOpt->isChecked();

		taseditorConfig.projectSavingOptions_SaveInBinary  = binaryInput->isChecked();
		taseditorConfig.projectSavingOptions_SaveMarkers   = saveMarkers->isChecked();
		taseditorConfig.projectSavingOptions_SaveBookmarks = saveBookmarks->isChecked();
		taseditorConfig.projectSavingOptions_SaveHistory   = saveHistory->isChecked();
		taseditorConfig.projectSavingOptions_SavePianoRoll = savePianoRoll->isChecked();
		taseditorConfig.projectSavingOptions_SaveSelection = saveSelection->isChecked();

		if ( allFrames->isChecked() )
		{
			taseditorConfig.projectSavingOptions_GreenzoneSavingMode = GREENZONE_SAVING_MODE_ALL;
		}
		else if ( every16thFrame->isChecked() )
		{
			taseditorConfig.projectSavingOptions_GreenzoneSavingMode = GREENZONE_SAVING_MODE_16TH;
		}
		else if ( markedFrames->isChecked() )
		{
			taseditorConfig.projectSavingOptions_GreenzoneSavingMode = GREENZONE_SAVING_MODE_MARKED;
		}
		else
		{
			taseditorConfig.projectSavingOptions_GreenzoneSavingMode = GREENZONE_SAVING_MODE_NO;
		}
	}
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::setGreenzoneCapacity(void)
{
	int ret;
	int newValue = taseditorConfig.greenzoneCapacity;
	QInputDialog dialog(this);
	FCEU_CRITICAL_SECTION( emuLock );

	dialog.setWindowTitle( tr("Greenzone Capacity") );
	dialog.setInputMode( QInputDialog::IntInput );
	dialog.setIntRange( GREENZONE_CAPACITY_MIN, GREENZONE_CAPACITY_MAX );
	dialog.setLabelText( tr("Keep savestates for how many frames?\n(actual limit of savestates can be 5 times more than the number provided)") );
	dialog.setIntValue( newValue );

	ret = dialog.exec();

	if ( ret == QDialog::Accepted )
	{
		newValue = dialog.intValue();

		if (newValue < GREENZONE_CAPACITY_MIN)
		{
			newValue = GREENZONE_CAPACITY_MIN;
		}
		else if (newValue > GREENZONE_CAPACITY_MAX)
		{
			newValue = GREENZONE_CAPACITY_MAX;
		}
		if (newValue < taseditorConfig.greenzoneCapacity)
		{
			taseditorConfig.greenzoneCapacity = newValue;
			greenzone.runGreenzoneCleaning();
		}
		else
		{
			taseditorConfig.greenzoneCapacity = newValue;
		}
	}
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::setMaxUndoCapacity(void)
{
	int ret;
	int newValue = taseditorConfig.maxUndoLevels;
	QInputDialog dialog(this);
	FCEU_CRITICAL_SECTION( emuLock );

	dialog.setWindowTitle( tr("Max undo levels") );
	dialog.setInputMode( QInputDialog::IntInput );
	dialog.setIntRange( UNDO_LEVELS_MIN, UNDO_LEVELS_MAX );
	dialog.setLabelText( tr("Keep history of how many changes?") );
	dialog.setIntValue( newValue );

	ret = dialog.exec();

	if ( ret == QDialog::Accepted )
	{
		newValue = dialog.intValue();

		if (newValue < UNDO_LEVELS_MIN)
		{
			newValue = UNDO_LEVELS_MIN;
		}
		else if (newValue > UNDO_LEVELS_MAX)
		{
			newValue = UNDO_LEVELS_MAX;
		}
		if (newValue != taseditorConfig.maxUndoLevels)
		{
			taseditorConfig.maxUndoLevels = newValue;
			history.updateHistoryLogSize();
			selection.updateHistoryLogSize();
		}
	}
}
// ----------------------------------------------------------------------------------------------
void TasEditorWindow::loadClipboard(const char *txt)
{
	clipboard->setText( tr(txt), QClipboard::Clipboard );

	if ( clipboard->supportsSelection() )
	{
		clipboard->setText( tr(txt), QClipboard::Selection );
	}
}
// ----------------------------------------------------------------------------------------------
// following functions use function parameters to determine range of frames
void TasEditorWindow::toggleInput(int start, int end, int joy, int button, int consecutivenessTag)
{
	if (joy < 0 || joy >= joysticksPerFrame[getInputType(currMovieData)]) return;

	int check_frame = end;
	if (start > end)
	{
		// swap
		int temp_start = start;
		start = end;
		end = temp_start;
	}
	if (start < 0) start = end;
	if (end >= currMovieData.getNumRecords())
		return;

	if (currMovieData.records[check_frame].checkBit(joy, button))
	{
		// clear range
		for (int i = start; i <= end; ++i)
			currMovieData.records[i].clearBit(joy, button);
		greenzone.invalidateAndUpdatePlayback(history.registerChanges(MODTYPE_UNSET, start, end, 0, NULL, consecutivenessTag));
	} else
	{
		// set range
		for (int i = start; i <= end; ++i)
			currMovieData.records[i].setBit(joy, button);
		greenzone.invalidateAndUpdatePlayback(history.registerChanges(MODTYPE_SET, start, end, 0, NULL, consecutivenessTag));
	}
}
void TasEditorWindow::setInputUsingPattern(int start, int end, int joy, int button, int consecutivenessTag)
{
	if (joy < 0 || joy >= joysticksPerFrame[getInputType(currMovieData)]) return;

	if (start > end)
	{
		// swap
		int temp_start = start;
		start = end;
		end = temp_start;
	}
	if (start < 0) start = end;
	if (end >= currMovieData.getNumRecords())
	{
		return;
	}

	int pattern_offset = 0, current_pattern = taseditorConfig.currentPattern;
	bool changes_made = false;
	bool value;

	for (int i = start; i <= end; ++i)
	{
		// skip lag frames
		if (taseditorConfig.autofirePatternSkipsLag && greenzone.lagLog.getLagInfoAtFrame(i) == LAGGED_YES)
		{
			continue;
		}
		value = (patterns[current_pattern][pattern_offset] != 0);
		if (currMovieData.records[i].checkBit(joy, button) != value)
		{
			changes_made = true;
			currMovieData.records[i].setBitValue(joy, button, value);
		}
		pattern_offset++;
		if (pattern_offset >= (int)patterns[current_pattern].size())
		{
			pattern_offset -= patterns[current_pattern].size();
		}
	}
	if (changes_made)
	{
		greenzone.invalidateAndUpdatePlayback(history.registerChanges(MODTYPE_PATTERN, start, end, 0, patternsNames[current_pattern].c_str(), consecutivenessTag));
	}
}

// following functions use current Selection to determine range of frames
bool TasEditorWindow::handleColumnSet(void)
{
	RowsSelection* current_selection = selection.getCopyOfCurrentRowsSelection();
	if (current_selection->size() == 0) return false;
	RowsSelection::iterator current_selection_begin(current_selection->begin());
	RowsSelection::iterator current_selection_end(current_selection->end());

	// inspect the selected frames, if they are all set, then unset all, else set all
	bool unset_found = false, changes_made = false;
	for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
	{
		if (!markersManager.getMarkerAtFrame(*it))
		{
			unset_found = true;
			break;
		}
	}
	if (unset_found)
	{
		// set all
		for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
		{
			if (!markersManager.getMarkerAtFrame(*it))
			{
				if (markersManager.setMarkerAtFrame(*it))
				{
					changes_made = true;
					//pianoRoll.redrawRow(*it);
					//pianoRoll->update(); // Piano roll will update at next periodic cycle
					lowerMarkerNote->setFocus();
				}
			}
		}
		if (changes_made)
		{
			history.registerMarkersChange(MODTYPE_MARKER_SET, *current_selection_begin, *current_selection->rbegin());
		}
	}
	else
	{
		// unset all
		for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
		{
			if (markersManager.getMarkerAtFrame(*it))
			{
				markersManager.removeMarkerFromFrame(*it);
				changes_made = true;
				//pianoRoll.redrawRow(*it);
				//pianoRoll->update(); // Piano roll will update at next periodic cycle
			}
		}
		if (changes_made)
		{
			history.registerMarkersChange(MODTYPE_MARKER_REMOVE, *current_selection_begin, *current_selection->rbegin());
		}
	}
	if (changes_made)
	{
		selection.mustFindCurrentMarker = playback.mustFindCurrentMarker = true;
	}
	return changes_made;
}

bool TasEditorWindow::handleColumnSetUsingPattern(void)
{
	RowsSelection* current_selection = selection.getCopyOfCurrentRowsSelection();
	if (current_selection->size() == 0) return false;
	RowsSelection::iterator current_selection_begin(current_selection->begin());
	RowsSelection::iterator current_selection_end(current_selection->end());
	int pattern_offset = 0, current_pattern = taseditorConfig.currentPattern;
	bool changes_made = false;

	for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
	{
		// skip lag frames
		if (taseditorConfig.autofirePatternSkipsLag && greenzone.lagLog.getLagInfoAtFrame(*it) == LAGGED_YES)
			continue;
		if (patterns[current_pattern][pattern_offset])
		{
			if (!markersManager.getMarkerAtFrame(*it))
			{
				if (markersManager.setMarkerAtFrame(*it))
				{
					changes_made = true;
					pianoRoll->update();
				}
			}
		}
		else
		{
			if (markersManager.getMarkerAtFrame(*it))
			{
				markersManager.removeMarkerFromFrame(*it);
				changes_made = true;
				pianoRoll->update();
			}
		}
		pattern_offset++;
		if (pattern_offset >= (int)patterns[current_pattern].size())
			pattern_offset -= patterns[current_pattern].size();
	}
	if (changes_made)
	{
		history.registerMarkersChange(MODTYPE_MARKER_PATTERN, *current_selection_begin, *current_selection->rbegin(), patternsNames[current_pattern].c_str());
		selection.mustFindCurrentMarker = playback.mustFindCurrentMarker = true;
		return true;
	}
	return false;
}
bool TasEditorWindow::handleInputColumnSetUsingPattern(int joy, int button)
{
	if (joy < 0 || joy >= joysticksPerFrame[getInputType(currMovieData)]) return false;

	RowsSelection* current_selection = selection.getCopyOfCurrentRowsSelection();
	if (current_selection->size() == 0) return false;
	RowsSelection::iterator current_selection_begin(current_selection->begin());
	RowsSelection::iterator current_selection_end(current_selection->end());
	int pattern_offset = 0, current_pattern = taseditorConfig.currentPattern;

	for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
	{
		// skip lag frames
		if (taseditorConfig.autofirePatternSkipsLag && greenzone.lagLog.getLagInfoAtFrame(*it) == LAGGED_YES)
			continue;
		currMovieData.records[*it].setBitValue(joy, button, patterns[current_pattern][pattern_offset] != 0);
		pattern_offset++;
		if (pattern_offset >= (int)patterns[current_pattern].size())
			pattern_offset -= patterns[current_pattern].size();
	}
	int first_changes = history.registerChanges(MODTYPE_PATTERN, *current_selection_begin, *current_selection->rbegin(), 0, patternsNames[current_pattern].c_str());
	if (first_changes >= 0)
	{
		greenzone.invalidateAndUpdatePlayback(first_changes);
		return true;
	} else
		return false;
}

bool TasEditorWindow::handleInputColumnSet(int joy, int button)
{
	if (joy < 0 || joy >= joysticksPerFrame[getInputType(currMovieData)]) return false;

	RowsSelection* current_selection = selection.getCopyOfCurrentRowsSelection();
	if (current_selection->size() == 0) return false;
	RowsSelection::iterator current_selection_begin(current_selection->begin());
	RowsSelection::iterator current_selection_end(current_selection->end());

	//inspect the selected frames, if they are all set, then unset all, else set all
	bool newValue = false;
	for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
	{
		if (!(currMovieData.records[*it].checkBit(joy,button)))
		{
			newValue = true;
			break;
		}
	}
	// apply newValue
	for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
		currMovieData.records[*it].setBitValue(joy,button,newValue);

	int first_changes;
	if (newValue)
	{
		first_changes = history.registerChanges(MODTYPE_SET, *current_selection_begin, *current_selection->rbegin());
	} else
	{
		first_changes = history.registerChanges(MODTYPE_UNSET, *current_selection_begin, *current_selection->rbegin());
	}
	if (first_changes >= 0)
	{
		greenzone.invalidateAndUpdatePlayback(first_changes);
		return true;
	}
	return false;
}

void TasEditorWindow::setMarkers(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	RowsSelection* current_selection = selection.getCopyOfCurrentRowsSelection();
	if (current_selection->size())
	{
		RowsSelection::iterator current_selection_begin(current_selection->begin());
		RowsSelection::iterator current_selection_end(current_selection->end());
		bool changes_made = false;
		for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
		{
			if (!markersManager.getMarkerAtFrame(*it))
			{
				if (markersManager.setMarkerAtFrame(*it))
				{
					changes_made = true;
					//pianoRoll->update();
				}
			}
		}
		if (changes_made)
		{
			selection.mustFindCurrentMarker = playback.mustFindCurrentMarker = true;
			history.registerMarkersChange(MODTYPE_MARKER_SET, *current_selection_begin, *current_selection->rbegin());
		}
	}
}
void TasEditorWindow::removeMarkers(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	RowsSelection* current_selection = selection.getCopyOfCurrentRowsSelection();
	if (current_selection->size())
	{
		RowsSelection::iterator current_selection_begin(current_selection->begin());
		RowsSelection::iterator current_selection_end(current_selection->end());
		bool changes_made = false;
		for(RowsSelection::iterator it(current_selection_begin); it != current_selection_end; it++)
		{
			if (markersManager.getMarkerAtFrame(*it))
			{
				markersManager.removeMarkerFromFrame(*it);
				changes_made = true;
				//pianoRoll->update();
			}
		}
		if (changes_made)
		{
			selection.mustFindCurrentMarker = playback.mustFindCurrentMarker = true;
			history.registerMarkersChange(MODTYPE_MARKER_REMOVE, *current_selection_begin, *current_selection->rbegin());
		}
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::ungreenzoneSelectedFrames(void)
{
	FCEU_CRITICAL_SECTION( emuLock );

	greenzone.ungreenzoneSelectedFrames();
}
//----------------------------------------------------------------------------
void TasEditorWindow::upperMarkerLabelClicked(void)
{
	pianoRoll->followPlaybackCursor();
}
//----------------------------------------------------------------------------
void TasEditorWindow::lowerMarkerLabelClicked(void)
{
	int dragMode = pianoRoll->getDragMode();

	if (dragMode != DRAG_MODE_SELECTION && dragMode != DRAG_MODE_DESELECTION)
	{
		pianoRoll->followSelection();
	}
}
//----------------------------------------------------------------------------
void TasEditorWindow::jumpToPreviousMarker(void)
{
	selection.jumpToPreviousMarker();
}
//----------------------------------------------------------------------------
void TasEditorWindow::jumpToNextMarker(void)
{
	selection.jumpToNextMarker();
}
//----------------------------------------------------------------------------
void TasEditorWindow::findSimilarNote(void)
{
	markersManager.findSimilarNote();
}
//----------------------------------------------------------------------------
void TasEditorWindow::findNextSimilarNote(void)
{
	markersManager.findNextSimilarNote();
}
//----------------------------------------------------------------------------
void TasEditorWindow::openAboutWindow(void)
{
	QDialog about(this);
	QVBoxLayout *mainLayout, *vbox;
	QHBoxLayout *hbox;
	QPixmap pm(":icons/taseditor-icon32.png");
	QPixmap pm2;
	QLabel *imgLbl;
	QTextEdit *txtEdit;
	QPushButton *okButton;
	const char *txt = "\
Created by AnS\n\n\
Originated from TASEdit\n\
made by zeromus & adelikat\n\n\
Ported to Qt by mjbudd77\n\
";
	
	pm2 = pm.scaled( 64, 64 );

	mainLayout = new QVBoxLayout();
	vbox       = new QVBoxLayout();
	hbox       = new QHBoxLayout();
	txtEdit    = new QTextEdit();
	okButton   = new QPushButton( tr("OK") );

	about.setWindowTitle( tr("About") );
	about.setLayout( mainLayout );

	imgLbl = new QLabel();
	imgLbl->setPixmap(pm2);

	mainLayout->addLayout( hbox );
	hbox->addWidget( imgLbl, 2, Qt::AlignCenter );
	hbox->addLayout( vbox, 2 );
	vbox->addWidget( new QLabel( tr("TAS Editor") ), 1, Qt::AlignCenter );
	vbox->addWidget( new QLabel( tr("Version 1.01") ), 1, Qt::AlignCenter );
	mainLayout->addWidget( txtEdit );

	hbox = new QHBoxLayout();
	hbox->addStretch(5);
	hbox->addWidget( okButton, 1 );
	mainLayout->addLayout( hbox );

	txtEdit->setText( tr(txt) );
	txtEdit->setReadOnly(true);

	okButton->setDefault(true);
	okButton->setIcon(style()->standardIcon(QStyle::SP_DialogOkButton));
	connect( okButton, SIGNAL(clicked(void)), &about, SLOT(accept(void)) );

	about.exec();
}
//----------------------------------------------------------------------------
//----------------------------------------------------------------------------
//---- TAS Window Main Horizontal Splitter
//----------------------------------------------------------------------------
TasEditorSplitter::TasEditorSplitter( QWidget *parent )
	: QSplitter( Qt::Horizontal, parent )
{
	panelInitDone = false;
}
//----------------------------------------------------------------------------
TasEditorSplitter::~TasEditorSplitter(void)
{

}
//----------------------------------------------------------------------------
void TasEditorSplitter::resizeEvent(QResizeEvent *event)
{
       	int minWidth;
	//int widthDelta;
	QList<int> panelWidth;

	//printf("Panel Resize\n");
	if ( !panelInitDone )
	{
		QSplitter::resizeEvent(event);
		panelInitDone = true;
		return;
	}
	//widthDelta = event->size().width() - event->oldSize().width();

	panelWidth = sizes();


	//for (int i=0; i<panelWidth.count(); i++)
	//{
	//	printf("Panel %i: %i\n", i, panelWidth[i] );
	//}
	panelWidth[0] = event->size().width() - panelWidth[1] - handleWidth();
	//panelWidth[0] += widthDelta;

	minWidth = widget(0)->minimumWidth();

	if ( panelWidth[0] < minWidth )
	{
		panelWidth[0] = minWidth;
	}
	setSizes( panelWidth );
}
//----------------------------------------------------------------------------
