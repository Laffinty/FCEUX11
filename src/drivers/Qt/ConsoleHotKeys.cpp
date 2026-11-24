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
// ConsoleHotKeys.cpp
//
#include <QShortcut>
#include <QUrl>
#include <QDesktopServices>
#include <QMessageBox>

#include <filesystem>

#include "Qt/ConsoleWindow.h"
#include "Qt/input.h"
#include "Qt/fceuWrapper.h"
#include "Qt/AboutWindow.h"
#include "Qt/MsgLogViewer.h"
#include "Qt/HelpPages.h"
// v2.0.8 C2/C3 O3: the diagnostics dialog, the log sink and the crash dump
// directory (all Qt-free headers).
#include "Qt/DiagnosticsDialog.h"
#include "Qt/log_sink.h"
#include "crash_handler.h"

void consoleWin_t::initHotKeys(void)
{
	for (int i = 0; i < HK_MAX; i++)
	{
		Hotkeys[i].init( this );
	}

	for (int i = 0; i < HK_MAX; i++)
	{
		QShortcut *shortcut = Hotkeys[i].getShortcut();

		connect( shortcut, &QShortcut::activatedAmbiguously, [ this, shortcut ] { warnAmbiguousShortcut( shortcut ); } );
	}

	Hotkeys[HK_FRAME_ADVANCE].getShortcut()->setEnabled(false);
	Hotkeys[HK_TURBO        ].getShortcut()->setEnabled(false);

	connect( Hotkeys[ HK_VOLUME_MUTE ].getShortcut(), SIGNAL(activated()), this, SLOT(muteSoundVolume(void)) );
	connect( Hotkeys[ HK_VOLUME_DOWN ].getShortcut(), SIGNAL(activated()), this, SLOT(decrSoundVolume(void)) );
	connect( Hotkeys[ HK_VOLUME_UP   ].getShortcut(), SIGNAL(activated()), this, SLOT(incrSoundVolume(void)) );

	connect( Hotkeys[ HK_LAG_COUNTER_DISPLAY  ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleLagCounterDisplay(void)) );
	connect( Hotkeys[ HK_FA_LAG_SKIP          ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleFrameAdvLagSkip(void))   );
	connect( Hotkeys[ HK_BIND_STATE           ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleMovieBindSaveState(void)));
	connect( Hotkeys[ HK_TOGGLE_FRAME_DISPLAY ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleMovieFrameDisplay(void)) );
	connect( Hotkeys[ HK_MOVIE_TOGGLE_RW      ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleMovieReadWrite(void))    );
	connect( Hotkeys[ HK_TOGGLE_INPUT_DISPLAY ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleInputDisplay(void))      );
	connect( Hotkeys[ HK_TOGGLE_BG            ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleBackground(void))        );
	connect( Hotkeys[ HK_TOGGLE_FG            ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleForeground(void))        );
	connect( Hotkeys[ HK_FKB_ENABLE           ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleFamKeyBrdEnable(void))   );
	connect( Hotkeys[ HK_TOGGLE_ALL_CHEATS    ].getShortcut(), SIGNAL(activated()), this, SLOT(toggleGlobalCheatEnable(void)) );

	// hotfix4 D-9: preset speed hotkeys (restored from upstream master)
	// Qt5+ functor connect with lambda, matching upstream style (file line 26
	// already uses &QShortcut::activatedAmbiguously + lambda).
	connect( Hotkeys[ HK_SPEED_QUARTER ].getShortcut(), &QShortcut::activated, [] { CustomEmulationSpeed(  25); } );
	connect( Hotkeys[ HK_SPEED_HALF    ].getShortcut(), &QShortcut::activated, [] { CustomEmulationSpeed(  50); } );
	connect( Hotkeys[ HK_SPEED_NORMAL  ].getShortcut(), &QShortcut::activated, [] { CustomEmulationSpeed( 100); } );
	connect( Hotkeys[ HK_SPEED_2X      ].getShortcut(), &QShortcut::activated, [] { CustomEmulationSpeed( 200); } );
	connect( Hotkeys[ HK_SPEED_4X      ].getShortcut(), &QShortcut::activated, [] { CustomEmulationSpeed( 400); } );
	connect( Hotkeys[ HK_SPEED_8X      ].getShortcut(), &QShortcut::activated, [] { CustomEmulationSpeed( 800); } );
	connect( Hotkeys[ HK_SPEED_16X     ].getShortcut(), &QShortcut::activated, [] { CustomEmulationSpeed(1600); } );

	connect( Hotkeys[ HK_SAVE_STATE_0         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState0(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_1         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState1(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_2         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState2(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_3         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState3(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_4         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState4(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_5         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState5(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_6         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState6(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_7         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState7(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_8         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState8(void))        );
	connect( Hotkeys[ HK_SAVE_STATE_9         ].getShortcut(), SIGNAL(activated()), this, SLOT(saveState9(void))        );

	connect( Hotkeys[ HK_LOAD_STATE_0         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState0(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_1         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState1(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_2         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState2(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_3         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState3(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_4         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState4(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_5         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState5(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_6         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState6(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_7         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState7(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_8         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState8(void))        );
	connect( Hotkeys[ HK_LOAD_STATE_9         ].getShortcut(), SIGNAL(activated()), this, SLOT(loadState9(void))        );

	connect( Hotkeys[ HK_LOAD_PREV_STATE      ].getShortcut(), SIGNAL(activated()), this, SLOT(loadPrevState(void))     );
	connect( Hotkeys[ HK_LOAD_NEXT_STATE      ].getShortcut(), SIGNAL(activated()), this, SLOT(loadNextState(void))     );
}

void consoleWin_t::aboutFCEUX(void)
{
	AboutWindow *aboutWin;

	aboutWin = new AboutWindow(this);
	
	aboutWin->show();
	return;
}

void consoleWin_t::aboutQt(void)
{
	QMessageBox::aboutQt(this);
	return;
}

void consoleWin_t::openMsgLogWin(void)
{
	MsgLogViewDialog_t *msgLogWin;

	msgLogWin = new MsgLogViewDialog_t(this);

	msgLogWin->show();

	return;
}

// v2.0.8 C2/C3 O3 -- the Advanced -> Diagnostics entries.
void consoleWin_t::openDiagnosticsDialog(void)
{
	DiagnosticsDialog_t* diagWin = new DiagnosticsDialog_t(this);
	diagWin->show();
}

void consoleWin_t::openLogDirectory(void)
{
	const std::string path = fceux11::LogSink::instance().current_path();
	if (path.empty())
		return;
	const std::filesystem::path parent = std::filesystem::path(path).parent_path();
	if (parent.empty())
		return;
	QDesktopServices::openUrl(QUrl::fromLocalFile(QString::fromStdString(parent.string())));
}

void consoleWin_t::openCrashDirectory(void)
{
	const std::string dir = fceux11::crash_dump_dir();
	if (dir.empty())
		return;
	QDesktopServices::openUrl(QUrl::fromLocalFile(QString::fromStdString(dir)));
}

void consoleWin_t::setLogToFile(bool on)
{
	fceux11::LogSink::instance().set_enabled(on);
	g_config->setOption("SDL.LogToFile", on ? 1 : 0);
}

void consoleWin_t::openOnlineDocs(void)
{
	if ( QDesktopServices::openUrl( QUrl("https://fceux.com/web/help/fceux.html") ) == false )
	{
		QueueErrorMsgWindow("Error: Failed to open link to: https://fceux.com/web/help/fceux.html");
	}
	return;
}

void consoleWin_t::openOfflineDocs(void)
{
	OpenHelpWindow();
	return;
}

void consoleWin_t::syncActionConfig( QAction *act, const char *property )
{
	if ( act->isCheckable() )
	{
		int enable;
		g_config->getOption ( property, &enable);

		act->setChecked( enable ? true : false );
	}
}

int consoleWin_t::getPeriodicInterval(void)
{
	int val = (1000.0 / 60.0) + 0.5;
	return val;
}
