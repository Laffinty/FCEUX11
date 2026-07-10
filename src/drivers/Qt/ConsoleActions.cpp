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
#include "Qt/nes_shm.h"
#include "Qt/TasEditor/TasEditorWindow.h"


#include "Qt/ConsoleActions.h"`n#include "Qt/ConsoleWindow.h"


void consoleWin_t::openInputConfWin(void)
{
	//printf("Open Input Config Window\n");
	
	openInputConfWindow(this);
}

void consoleWin_t::openGamePadConfWin(void)
{
	//printf("Open GamePad Config Window\n");
	
	openGamePadConfWindow(this);
}

void consoleWin_t::openGameSndConfWin(void)
{
	ConsoleSndConfDialog_t *sndConfWin;

	//printf("Open Sound Config Window\n");
	
   sndConfWin = new ConsoleSndConfDialog_t(this);
	
   sndConfWin->show();
}

void consoleWin_t::openGameVideoConfWin(void)
{
	ConsoleVideoConfDialog_t *vidConfWin;

	//printf("Open Video Config Window\n");
	
   vidConfWin = new ConsoleVideoConfDialog_t(this);
	
   vidConfWin->show();
}

void consoleWin_t::openHotkeyConfWin(void)
{
	HotKeyConfDialog_t *hkConfWin;

	//printf("Open Hot Key Config Window\n");
	
   hkConfWin = new HotKeyConfDialog_t(this);
	
   hkConfWin->show();
}

void consoleWin_t::openPaletteConfWin(void)
{
	PaletteConfDialog_t *paletteConfWin;

	//printf("Open Palette Config Window\n");
	
   paletteConfWin = new PaletteConfDialog_t(this);
	
   paletteConfWin->show();
}

void consoleWin_t::openGuiConfWin(void)
{
	GuiConfDialog_t *guiConfWin;

	//printf("Open GUI Config Window\n");
	
   guiConfWin = new GuiConfDialog_t(this);
	
   guiConfWin->show();
}

void consoleWin_t::openTimingConfWin(void)
{
	TimingConfDialog_t *tmConfWin;

	//printf("Open Timing Config Window\n");
	
   tmConfWin = new TimingConfDialog_t(this);
	
   tmConfWin->show();
}




void consoleWin_t::openTasEditor(void)
{
	FCEU_WRAPPER_LOCK();

	if ( tasWindowIsOpen() )
	{
		tasWindowSetFocus(true);
	}
	else if (FCEU_IsValidUI(FCEUI_TASEDITOR))
	{
		TasEditorWindow *win;

		win = new TasEditorWindow(this);
		
		win->show();

		connect(emulatorThread, SIGNAL(frameFinished(void)), win, SLOT(frameUpdate(void)) );
	}
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::openMovieOptWin(void)
{
	MovieOptionsDialog_t *win;

	//printf("Open Movie Options Window\n");
	
   win = new MovieOptionsDialog_t(this);
	
   win->show();
}













void consoleWin_t::toggleAutoResume(void)
{
   //printf("Auto Resume: %i\n", autoResume->isChecked() );

	g_config->setOption ("SDL.AutoResume", (int) autoResume->isChecked() );

	AutoResumePlay = autoResume->isChecked();
}

void consoleWin_t::winResizeIx(int iscale)
{
	QSize w, v;
	double xscale = 1.0, yscale = 1.0, aspectRatio = 1.0;
	int texture_width  = nes_shm->video.ncol;
	int texture_height = nes_shm->video.nrow;
	int l=0, r=texture_width;
	int t=0, b=texture_height;
	int dw=0, dh=0, rw, rh;
	bool forceAspect = false;

	xscale = (double)iscale;
	yscale = (double)iscale;

	w = size();

	if ( viewport_Interface )
	{
		v = viewport_Interface->size();
		aspectRatio = viewport_Interface->getAspectRatio();
		forceAspect = viewport_Interface->getForceAspectOpt();
	}

	dw = w.width()  - v.width();
	dh = w.height() - v.height();

	if ( forceAspect )
	{
		xscale = xscale / nes_shm->video.xscale;
		yscale = xscale * (double)nes_shm->video.xyRatio;
	}
	else
	{
		xscale = xscale / nes_shm->video.xscale;
		yscale = yscale / nes_shm->video.yscale;
	}
	rw=(int)((r-l)*xscale);
	rh=(int)((b-t)*yscale);

	if ( forceAspect )
	{
		double rr;

		rr = (double)rh / (double)rw;

		if ( rr > aspectRatio )
		{
			rw = (int)( (((double)rh) / aspectRatio) + 0.50);
		}
		else
		{
			rh = (int)( (((double)rw) * aspectRatio) + 0.50);
		}
	}

	resize( rw + dw, rh + dh );
}

void consoleWin_t::toggleFullscreen(void)
{
	if ( isFullScreen() )
	{
		showNormal();

		if ( autoHideMenuFullscreen )
		{
			menubar->setVisible(true);
		}
	}
	else
	{
		if ( autoHideMenuFullscreen )
		{
			menubar->setVisible(false);
		}
		showFullScreen();
	}
}

void consoleWin_t::toggleFamKeyBrdEnable(void)
{
	toggleFamilyKeyboardFunc();
}

extern int globalCheatDisabled;

void consoleWin_t::toggleGlobalCheatEnable(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::GlobalToggleCheat(globalCheatDisabled);
	FCEU_WRAPPER_UNLOCK();

	g_config->setOption("SDL.CheatsDisabled", globalCheatDisabled);
	g_config->save();

	updateCheatDialog();
}

void consoleWin_t::warnAmbiguousShortcut( QShortcut *shortcut)
{
	char stmp[256];
	std::string msg;
	int c = 0;

	snprintf( stmp, sizeof(stmp), "Error: Ambiguous Shortcut Activation for Key Sequence: '%s'\n", shortcut->key().toString().toStdString().c_str() );

	msg.assign( stmp );

	for (int i = 0; i < HK_MAX; i++)
	{
		QShortcut *sc = Hotkeys[i].getShortcut();

		if ( sc == NULL )
		{
			continue;
		}

		if ( (sc == shortcut) || (shortcut->key().matches( sc->key() ) == QKeySequence::ExactMatch) )
		{
			if ( c == 0 )
			{
				msg.append("Hot Key Conflict: "); c++;
			}
			else
			{
				msg.append(" and "); c++;
			}
			msg.append( Hotkeys[i].getConfigName() );
		}
	}
	QueueErrorMsgWindow( msg.c_str() );
}





#ifdef _WIN32
void consoleWin_t::setTaskbarProgress(double pct)
{
	if (taskbarProgress) {
		taskbarProgress->setProgress(pct);
	}
}

void consoleWin_t::setTaskbarState(int tbpfState)
{
	if (taskbarProgress) {
		taskbarProgress->setState(tbpfState);
	}
}
#endif

void consoleWin_t::setRegion(int region)
{
	int currentRegion;

	g_config->setOption ("SDL.PAL", region);
	g_config->save ();

	currentRegion = fceu11::GetRegion();

	if ( currentRegion != region )
	{
		FCEU_WRAPPER_LOCK();
		FCEUI_SetRegion (region, true);
		FCEU_WRAPPER_UNLOCK();
	}
	return;
}










void consoleWin_t::openFamilyKeyboard(void)
{
	openFamilyKeyboardDialog(this);
	return;
}













void consoleWin_t::muteSoundVolume(void)
{
	FCEU_WRAPPER_LOCK();
	FCEUD_SoundToggle();
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::incrSoundVolume(void)
{
	FCEU_WRAPPER_LOCK();
	FCEUD_SoundVolumeAdjust( 1);
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::decrSoundVolume(void)
{
	FCEU_WRAPPER_LOCK();
	FCEUD_SoundVolumeAdjust(-1);
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleLagCounterDisplay(void)
{
	FCEU_WRAPPER_LOCK();
	lagCounterDisplay = !lagCounterDisplay;
	g_config->setOption("SDL.ShowLagCount", lagCounterDisplay);
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleFrameAdvLagSkip(void)
{
	FCEU_WRAPPER_LOCK();
	frameAdvanceLagSkip = !frameAdvanceLagSkip;
	FCEUI_DispMessage ("Skipping lag in Frame Advance %sabled.", 0, frameAdvanceLagSkip ? "en" : "dis");
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleMovieBindSaveState(void)
{
	FCEU_WRAPPER_LOCK();
	bindSavestate = !bindSavestate;
	g_config->setOption("SDL.MovieBindSavestate", bindSavestate);
	FCEUI_DispMessage ("Savestate binding to movie %sabled.", 0, bindSavestate ? "en" : "dis");
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleMovieFrameDisplay(void)
{
	extern int frame_display;
	FCEU_WRAPPER_LOCK();
	fceu11::MovieToggleFrameDisplay();
	g_config->setOption("SDL.ShowFrameCount", frame_display );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleMovieReadWrite(void)
{
	FCEU_WRAPPER_LOCK();
	//FCEUI_SetMovieToggleReadOnly (!FCEUI_GetMovieToggleReadOnly ());
	fceu11::MovieToggleReadOnly();

	if ( tasWin != NULL )
	{
		tasWin->updateRecordStatus();
	}
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleInputDisplay(void)
{
	FCEU_WRAPPER_LOCK();
	FCEUI_ToggleInputDisplay();
	g_config->setOption ("SDL.InputDisplay", input_display);
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleBackground(void)
{
	bool fgOn, bgOn;
	FCEU_WRAPPER_LOCK();
	fceu11::GetRenderPlanes( fgOn,  bgOn );
	fceu11::SetRenderPlanes( fgOn, !bgOn );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleForeground(void)
{
	bool fgOn, bgOn;
	FCEU_WRAPPER_LOCK();
	fceu11::GetRenderPlanes(  fgOn, bgOn );
	fceu11::SetRenderPlanes( !fgOn, bgOn );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::toggleTurboMode(void)
{
	NoWaiting ^= 1;
}

void consoleWin_t::openStateRecorderConfWin(void)
{
	StateRecorderDialog_t *win;

	win = new StateRecorderDialog_t(this);

	win->show();
}

void consoleWin_t::openMovie(void)
{
	MoviePlayDialog_t *win;

	win = new MoviePlayDialog_t(this);

	win->show();
}

void consoleWin_t::playMovieFromBeginning(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::MoviePlayFromBeginning();
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::stopMovie(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::StopMovie();
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::recordMovie(void)
{
	FCEU_WRAPPER_LOCK();
	if (fceuWrapperGameLoaded())
	{
		MovieRecordDialog_t dialog(this);
		dialog.exec();
	}
	FCEU_WRAPPER_UNLOCK();
	return;
}
