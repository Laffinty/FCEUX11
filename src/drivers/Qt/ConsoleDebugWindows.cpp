// ConsoleDebugWindows.cpp
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
#include "Qt/nes_shm.h"
#include "Qt/TasEditor/TasEditorWindow.h"


#include "Qt/ConsoleDebugWindows.h"
#include "Qt/ConsoleWindow.h"
void consoleWin_t::openTimingStatWin(void)
{
	FrameTimingDialog_t *tmStatWin;

	//printf("Open Timing Statistics Window\n");
	
   tmStatWin = new FrameTimingDialog_t(this);
	
   tmStatWin->show();
}

void consoleWin_t::openPaletteEditorWin(void)
{
	PaletteEditorDialog_t *win;

	//printf("Open Palette Editor Window\n");
	
   win = new PaletteEditorDialog_t(this);
	
   win->show();
}

void consoleWin_t::openAviRiffViewer(void)
{
	AviRiffViewerDialog *win;

	//printf("Open AVI RIFF Viewer Window\n");
	
	win = new AviRiffViewerDialog(this);
	
	win->show();
}

void consoleWin_t::openCheats(void)
{
	//printf("Open GUI Cheat Window\n");
	
   openCheatDialog(this);
}

void consoleWin_t::openRamWatch(void)
{
	RamWatchDialog_t *ramWatchWin;

	//printf("Open GUI RAM Watch Window\n");
	
   ramWatchWin = new RamWatchDialog_t(this);
	
   ramWatchWin->show();
}

void consoleWin_t::openRamSearch(void)
{
	//printf("Open GUI RAM Search Window\n");
	openRamSearchWindow(this);
}

void consoleWin_t::openDebugWindow(void)
{
	//printf("Open GUI 6502 Debugger Window\n");
	
	if ( debuggerWindowIsOpen() )
	{
		debuggerWindowSetFocus();
	}
	else
	{
		ConsoleDebugger *debugWin;

		debugWin = new ConsoleDebugger(this);
	
		debugWin->show();
	}
}

void consoleWin_t::openHexEditor(void)
{
	HexEditorDialog_t *hexEditWin;

	//printf("Open GUI Hex Editor Window\n");
	
   hexEditWin = new HexEditorDialog_t(this);
	
   hexEditWin->show();
}

void consoleWin_t::openPPUViewer(void)
{
	//printf("Open GUI PPU Viewer Window\n");
	
	openPPUViewWindow(this);
}

void consoleWin_t::openOAMViewer(void)
{
	//printf("Open GUI OAM Viewer Window\n");
	
	openOAMViewWindow(this);
}

void consoleWin_t::openNTViewer(void)
{
	//printf("Open GUI Name Table Viewer Window\n");
	
	openNameTableViewWindow(this);
}

void consoleWin_t::openCodeDataLogger(void)
{
	openCDLWindow(this);
}

void consoleWin_t::openGGEncoder(void)
{
	GameGenieDialog_t *win;

	//printf("Open Game Genie Window\n");
	
	win = new GameGenieDialog_t(this);
	
	win->show();
}

void consoleWin_t::openNesHeaderEditor(void)
{
	iNesHeaderEditor_t *win;

	//printf("Open NES Header Editor Window\n");
	
	win = new iNesHeaderEditor_t(this);
	
	if ( win->isInitialized() )
	{
		win->show();
	}
	else
	{
		delete win;
	}
}

void consoleWin_t::openTraceLogger(void)
{
	openTraceLoggerWindow(this);
}

void consoleWin_t::toggleGameGenie(bool checked)
{
	// hotfix4 D-12: drive both config and core from the slot's `checked`
	// parameter so the in-session state matches the menu checkbox. The
	// old implementation toggled gg_enabled for the config write but
	// passed the pre-toggle value to FCEUI_SetGameGenie, leaving the
	// core state opposite to the menu. Upstream has the same defect.
	FCEU_WRAPPER_LOCK();
	g_config->setOption ("SDL.GameGenie", checked);
	g_config->save ();
	FCEUI_SetGameGenie (checked);
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::loadGameGenieROM(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string last;
	std::string dir;
	QFileDialog  dialog(this, tr("Open Game Genie ROM") );
	QList<QUrl> urls;

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilter(tr("GG ROM File (gg.rom  *Genie*.nes) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Load") );

	g_config->getOption ("SDL.LastOpenFile", &last );

	getDirFromFile( last.c_str(), dir );

	dialog.setDirectory( tr(dir.c_str()) );

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

	g_config->setOption ("SDL.LastOpenFile", filename.toStdString().c_str() );

	// copy file to proper place (~/.fceux/gg.rom)
	std::ifstream f1 ( filename.toStdString().c_str(), std::fstream::binary);
	std::string fn_out = FCEU_MakeFName (FCEUMKF_GGROM, 0, "");
	std::ofstream f2 (fn_out.c_str (),
	std::fstream::trunc | std::fstream::binary);
	f2 << f1.rdbuf ();

   return;
}
