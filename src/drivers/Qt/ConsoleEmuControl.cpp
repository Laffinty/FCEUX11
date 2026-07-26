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


#include "Qt/ConsoleEmuControl.h"
#include "Qt/ConsoleWindow.h"
void consoleWin_t::loadStateFrom(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string last;
	std::string dir;
	const char *base;
	QFileDialog  dialog(this, tr("Load State From File") );
	QList<QUrl> urls;
	QDir d;

	base = fceu11::GetBaseDirectory();

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	if ( base )
	{
		urls << QUrl::fromLocalFile( QDir( base ).absolutePath() );

		d.setPath( QString(base) + "/fcs");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		d.setPath( QString(base) + "/sav");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}
	}


	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilter(tr("FCS & SAV Files (*.sav *.SAV *.fc? *.FC?) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Load") );

	g_config->getOption ("SDL.LastLoadStateFrom", &last );

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

	g_config->setOption ("SDL.LastLoadStateFrom", filename.toStdString().c_str() );

	FCEU_WRAPPER_LOCK();
	fceu11::LoadStateFile( filename.toStdString().c_str() );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::saveStateAs(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string last;
	std::string dir;
	const char *base;
	QFileDialog  dialog(this, tr("Save State To File") );
	QList<QUrl> urls;
	QDir d;

	base = fceu11::GetBaseDirectory();

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	if ( base )
	{
		urls << QUrl::fromLocalFile( QDir( base ).absolutePath() );

		d.setPath( QString(base) + "/fcs");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		d.setPath( QString(base) + "/sav");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}
	}

	dialog.setFileMode(QFileDialog::AnyFile);

	dialog.setNameFilter(tr("SAV Files (*.sav *.SAV) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Save") );
	dialog.setDefaultSuffix( tr(".sav") );

	g_config->getOption ("SDL.LastSaveStateAs", &last );

	if ( last.size() == 0 )
	{
		if ( base )
		{
			last = std::string(base) + "/sav";
		}
	}
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

	g_config->setOption ("SDL.LastSaveStateAs", filename.toStdString().c_str() );

	FCEU_WRAPPER_LOCK();
	fceu11::SaveStateFile( filename.toStdString().c_str() );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::loadState(int slot)
{
	int prevState;
	FCEU_WRAPPER_LOCK();
	prevState = fceu11::SelectStateSlot( slot, false );
	fceu11::LoadStateFile( NULL, true );
	fceu11::SelectStateSlot( prevState, false );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::loadState0(void){ loadState(0); }

void consoleWin_t::loadState1(void){ loadState(1); }

void consoleWin_t::loadState2(void){ loadState(2); }

void consoleWin_t::loadState3(void){ loadState(3); }

void consoleWin_t::loadState4(void){ loadState(4); }

void consoleWin_t::loadState5(void){ loadState(5); }

void consoleWin_t::loadState6(void){ loadState(6); }

void consoleWin_t::loadState7(void){ loadState(7); }

void consoleWin_t::loadState8(void){ loadState(8); }

void consoleWin_t::loadState9(void){ loadState(9); }

void consoleWin_t::loadPrevState(void)
{
	FCEU_WRAPPER_LOCK();
	FCEU_StateRecorderLoadPrevState();
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::loadNextState(void)
{
	FCEU_WRAPPER_LOCK();
	FCEU_StateRecorderLoadNextState();
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::saveState(int slot)
{
	int prevState;
	FCEU_WRAPPER_LOCK();
	prevState = fceu11::SelectStateSlot( slot, false );
	fceu11::SaveStateFile( NULL, true );
	fceu11::SelectStateSlot( prevState, false );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::saveState0(void){ saveState(0); }

void consoleWin_t::saveState1(void){ saveState(1); }

void consoleWin_t::saveState2(void){ saveState(2); }

void consoleWin_t::saveState3(void){ saveState(3); }

void consoleWin_t::saveState4(void){ saveState(4); }

void consoleWin_t::saveState5(void){ saveState(5); }

void consoleWin_t::saveState6(void){ saveState(6); }

void consoleWin_t::saveState7(void){ saveState(7); }

void consoleWin_t::saveState8(void){ saveState(8); }

void consoleWin_t::saveState9(void){ saveState(9); }

void consoleWin_t::changeState(int slot)
{
	FCEU_WRAPPER_LOCK();
	fceu11::SelectStateSlot( slot, true );
	FCEU_WRAPPER_UNLOCK();
	state[slot]->setChecked(true);
}

void consoleWin_t::changeState0(void){ changeState(0); }

void consoleWin_t::changeState1(void){ changeState(1); }

void consoleWin_t::changeState2(void){ changeState(2); }

void consoleWin_t::changeState3(void){ changeState(3); }

void consoleWin_t::changeState4(void){ changeState(4); }

void consoleWin_t::changeState5(void){ changeState(5); }

void consoleWin_t::changeState6(void){ changeState(6); }

void consoleWin_t::changeState7(void){ changeState(7); }

void consoleWin_t::changeState8(void){ changeState(8); }

void consoleWin_t::changeState9(void){ changeState(9); }

void consoleWin_t::incrementState(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::SelectStateNext(1);
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::decrementState(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::SelectStateNext(-1);
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::powerConsoleCB(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::PowerNES();
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::consoleHardReset(void)
{
	FCEU_WRAPPER_LOCK();
	fceuWrapperHardReset();
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::consoleSoftReset(void)
{
	FCEU_WRAPPER_LOCK();
	fceuWrapperSoftReset();
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::consolePause(void)
{
	FCEU_WRAPPER_LOCK();
	fceuWrapperTogglePause();
	FCEU_WRAPPER_UNLOCK();

	mainMenuEmuPauseSet = false;

#ifdef _WIN32
	// v0.3.15.x PHASE-3: update the taskbar overlay icon and
	// progress state to reflect the new pause state. We deliberately
	// do NOT add a custom HICON resource here; the existing icon
	// is cleared (nullptr) and the progress state is switched to
	// TBPF_PAUSED so the bar shows the standard Windows paused
	// (yellow) accent.
	if (taskbarProgress) {
		const bool nowPaused = FCEUI_EmulationPaused() != 0;
		taskbarProgress->setOverlayIcon(nullptr,
			nowPaused ? L"Paused" : L"");
		if (nowPaused) {
			taskbarProgress->setState(TBPF_PAUSED);
		} else {
			taskbarProgress->setState(TBPF_NOPROGRESS);
		}
	}
#endif
   return;
}

void consoleWin_t::setRegionNTSC(void)
{
	setRegion(0);
	return;
}

void consoleWin_t::setRegionPAL(void)
{
	setRegion(1);
	return;
}

void consoleWin_t::setRegionDendy(void)
{
	setRegion(2);
	return;
}

void consoleWin_t::setRamInit0(void)
{
	RAMInitOption = 0;

	g_config->setOption ("SDL.RamInitMethod", RAMInitOption);
	return;
}

void consoleWin_t::setRamInit1(void)
{
	RAMInitOption = 1;

	g_config->setOption ("SDL.RamInitMethod", RAMInitOption);
	return;
}

void consoleWin_t::setRamInit2(void)
{
	RAMInitOption = 2;

	g_config->setOption ("SDL.RamInitMethod", RAMInitOption);
	return;
}

void consoleWin_t::setRamInit3(void)
{
	RAMInitOption = 3;

	g_config->setOption ("SDL.RamInitMethod", RAMInitOption);
	return;
}

void consoleWin_t::insertCoin(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::VSUniCoin();
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::fdsSwitchDisk(void)
{
	FCEU_WRAPPER_LOCK();
	FCEU_FDSSelect();
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::fdsEjectDisk(void)
{
	FCEU_WRAPPER_LOCK();
	FCEU_FDSInsert();
	FCEU_WRAPPER_UNLOCK();
   return;
}

void consoleWin_t::fdsLoadBiosFile(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string last;
	std::string dir;
	QFileDialog  dialog(this, tr("Load FDS BIOS (disksys.rom)") );
	QList<QUrl> urls;

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());

	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilter(tr("ROM files (*.rom *.ROM) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Load") );

	g_config->getOption ("SDL.LastOpenFile", &last );

	getDirFromFile( last.c_str(), dir);

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

	// copy BIOS file to proper place (~/.fceux/disksys.rom)
	std::ifstream fdsBios (filename.toStdString().c_str(), std::fstream::binary);
	std::string output_filename =
		FCEU_MakeFName (FCEUMKF_FDSROM, 0, "");
	std::ofstream outFile (output_filename.c_str (),
			       std::fstream::trunc | std::fstream::
			       binary);
	outFile << fdsBios.rdbuf ();
	if (outFile.fail ())
	{
		FCEUD_PrintError ("Error copying the FDS BIOS file.");
	}
	else
	{
		printf("Famicom Disk System BIOS loaded.  If you are you having issues, make sure your BIOS file is 8KB in size.\n");
	}

   return;
}

void consoleWin_t::emuSpeedUp(void)
{
   IncreaseEmulationSpeed();
}

void consoleWin_t::emuSlowDown(void)
{
   DecreaseEmulationSpeed();
}

void consoleWin_t::emuSlowestSpd(void)
{
   FCEUD_SetEmulationSpeed( EMUSPEED_SLOWEST );
}

void consoleWin_t::emuNormalSpd(void)
{
   FCEUD_SetEmulationSpeed( EMUSPEED_NORMAL );
}

void consoleWin_t::emuFastestSpd(void)
{
   FCEUD_SetEmulationSpeed( EMUSPEED_FASTEST );
}

void consoleWin_t::emuCustomSpd(void)
{
	int ret;
	QInputDialog dialog(this);

	dialog.setWindowTitle( tr("Emulation Speed") );
	dialog.setLabelText( tr("Enter a percentage from 1 to 1000.") );
	dialog.setOkButtonText( tr("Ok") );
	dialog.setInputMode( QInputDialog::IntInput );
	dialog.setIntRange( 1, 1000 );
	dialog.setIntValue( 100 );
	
	ret = dialog.exec();
	
	if ( QDialog::Accepted == ret )
	{
	   int spdPercent;
	
	   spdPercent = dialog.intValue();
	
	   CustomEmulationSpeed( spdPercent );
	}
}

void consoleWin_t::emuSetFrameAdvDelay(void)
{
	int ret;
	QInputDialog dialog(this);

	dialog.setWindowTitle( tr("Frame Advance Delay") );
	dialog.setLabelText( tr("How much time should elapse before holding the frame advance unpauses the simulation?") );
	dialog.setOkButtonText( tr("Ok") );
	dialog.setInputMode( QInputDialog::IntInput );
	dialog.setIntRange( 0, 1000 );
	dialog.setIntValue( frameAdvance_Delay );
	
	ret = dialog.exec();
	
	if ( QDialog::Accepted == ret )
	{
	   frameAdvance_Delay = dialog.intValue();

	   g_config->setOption("SDL.FrameAdvanceDelay", frameAdvance_Delay );
	   g_config->save();
	}
}

void consoleWin_t::syncAutoFirePatternMenu(void)
{
	int on, off;

	GetAutoFirePattern( &on, &off );

	for (size_t i=0; i<afActList.size(); i++)
	{
		if ( afActList[i]->isMatch( on, off ) )
		{
			afActList[i]->setChecked(true);
			return;
		}
	}

	// If we get here, then the custom option is selected.
	afActCustom->setChecked(true);

}

void consoleWin_t::setCustomAutoFire(void)
{
	int ret, autoFireOnFrames, autoFireOffFrames;
	QDialog dialog(this);
	QLabel *lbl;
	QGridLayout *grid;
	QVBoxLayout *vbox;
	QSpinBox *onBox, *offBox;
	QPushButton *okButton, *cancelButton;

	autoFireOnFrames  = afActCustom->getOnValue();
	autoFireOffFrames = afActCustom->getOffValue();

	dialog.setWindowTitle( tr("Custom AutoFire Pattern") );

	 onBox = new QSpinBox();
	offBox = new QSpinBox();

	 onBox->setMinimum( 1);
	offBox->setMinimum( 1);
	 onBox->setMaximum(30);
	offBox->setMaximum(30);

	 onBox->setValue( autoFireOnFrames  );
	offBox->setValue( autoFireOffFrames );

	vbox = new QVBoxLayout();
	grid = new QGridLayout();

	lbl = new QLabel( tr("# ON Frames") );

	grid->addWidget( lbl, 0, 0 );

	lbl = new QLabel( tr("# OFF Frames") );

	grid->addWidget( lbl, 1, 0 );

	grid->addWidget( onBox , 0, 1 );
	grid->addWidget( offBox, 1, 1 );

	    okButton = new QPushButton( tr("Ok") );
	cancelButton = new QPushButton( tr("Cancel") );

	    okButton->setIcon( style()->standardIcon( QStyle::SP_DialogApplyButton  ) );
	cancelButton->setIcon( style()->standardIcon( QStyle::SP_DialogCancelButton ) );

	grid->addWidget( cancelButton , 2, 0 );
	grid->addWidget(     okButton , 2, 1 );

	vbox->addLayout( grid );

	dialog.setLayout( vbox );
	
	connect( cancelButton, SIGNAL(clicked(void)), &dialog, SLOT(reject(void)) );
	connect(     okButton, SIGNAL(clicked(void)), &dialog, SLOT(accept(void)) );

	okButton->setDefault(true);

	ret = dialog.exec();
	
	if ( QDialog::Accepted == ret )
	{
		autoFireOnFrames  =  onBox->value();
		autoFireOffFrames = offBox->value();

		afActCustom->setPattern( autoFireOnFrames, autoFireOffFrames );

		if ( afActCustom->isChecked() )
		{
			afActCustom->activateCB();
		}
		g_config->setOption("SDL.AutofireCustomOnFrames"  , autoFireOnFrames );
		g_config->setOption("SDL.AutofireCustomOffFrames" , autoFireOffFrames);
		g_config->save();
	}
}
