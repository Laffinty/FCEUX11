// ConsoleRecentRom.cpp
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


#include "Qt/ConsoleRecentRom.h"
#include "Qt/ConsoleWindow.h"
void consoleWin_t::clearRomList(void)
{
	std::list <std::string*>::iterator it;

	for (it=romList.begin(); it != romList.end(); it++)
	{
		delete *it;
	}
	romList.clear();
}

void consoleWin_t::buildRecentRomMenu(void)
{
	QAction *act;
	std::string s;
	std::string *sptr;
	char buf[128];

	clearRomList();
	recentRomMenu->clear();

	for (int i=0; i<10; i++)
	{
		snprintf( buf, sizeof(buf), "SDL.RecentRom%02i", i);

		g_config->getOption( buf, &s);

		//printf("Recent Rom:%i  '%s'\n", i, s.c_str() );

		if ( s.size() > 0 )
		{
			act = new consoleRecentRomAction( tr(s.c_str()), recentRomMenu);

			recentRomMenu->addAction( act );

			connect(act, SIGNAL(triggered()), act, SLOT(activateCB(void)) );

			sptr = new std::string();

			sptr->assign( s.c_str() );

			romList.push_front( sptr );
		}
	}

	// hotfix4 D-11: separator + Clear entry at the end of the menu
	// (matches upstream master ConsoleWindow.cpp:2344-2346 layout).
	if (recentRomMenu->actions().size() > 0)
	{
		recentRomMenu->addSeparator();
	}
	clearRecentRomAct = new QAction(tr("&Clear Recent ROM List"), recentRomMenu);
	connect(clearRecentRomAct, SIGNAL(triggered()), this, SLOT(clearRecentRomMenu(void)) );
	recentRomMenu->addAction(clearRecentRomAct);
}

// hotfix4 D-11: implementation of the slot wired above. Drops the in-memory
// romList, persists empty values for the 10 RecentRomNN config slots, and
// rebuilds the menu so the change shows immediately without a restart.
void consoleWin_t::clearRecentRomMenu(void)
{
	for (int i=0; i<10; i++)
	{
		char buf[128];
		snprintf( buf, sizeof(buf), "SDL.RecentRom%02i", i);
		g_config->setOption( buf, "");
	}
	g_config->save();
	buildRecentRomMenu();
}

void consoleWin_t::saveRecentRomMenu(void)
{
	int i;
	std::string *s;
	std::list <std::string*>::iterator it;
	char buf[128];

	i = romList.size() - 1;

	for (it=romList.begin(); it != romList.end(); it++)
	{
		s = *it;
		snprintf( buf, sizeof(buf), "SDL.RecentRom%02i", i);

		g_config->setOption( buf, s->c_str() );

		//printf("Recent Rom:%u  '%s'\n", i, s->c_str() );
		i--;
	}
}

void consoleWin_t::addRecentRom( const char *rom )
{
	std::string *s;
	std::list <std::string*>::iterator match_it;

	for (match_it=romList.begin(); match_it != romList.end(); match_it++)
	{
		s = *match_it;

		if ( s->compare( rom ) == 0 )
		{
			//printf("Found Match: %s\n", rom );
			break;
		}
	}

	if ( match_it != romList.end() )
	{
		s = *match_it;

		romList.erase(match_it);

		romList.push_back(s);
	}
	else
	{
		s = new std::string();

		s->assign( rom );
		
		romList.push_back(s);

		if ( romList.size() > 10 )
		{
			s = romList.front();

			romList.pop_front();

			delete s;
		}
	}

	saveRecentRomMenu();

	recentRomMenuReset = true;
}

void consoleWin_t::loadMostRecentROM(void)
{
	if ( romList.size() <= 0 )
	{
		return;
	}
	FCEU_WRAPPER_LOCK();
	CloseGame ();
	LoadGame ( (romList.back())->c_str() );
	FCEU_WRAPPER_UNLOCK();
}

consoleRecentRomAction::consoleRecentRomAction(QString desc, QWidget *parent)
	: QAction( desc, parent )
{
	QString txt;
	QFileInfo fi(desc);

	path = desc.toStdString();

	txt  = fi.fileName();
	txt += QString("\t");
	txt += desc;

	setText( txt );
}

consoleRecentRomAction::~consoleRecentRomAction(void)
{
	//printf("Recent ROM Menu Action Deleted\n");
}

void consoleRecentRomAction::activateCB(void)
{
	printf("Activate Recent ROM: %s \n", path.c_str() );

	FCEU_WRAPPER_LOCK();
	CloseGame ();
	LoadGame ( path.c_str() );
	FCEU_WRAPPER_UNLOCK();
}
