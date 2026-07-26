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


#include "Qt/ConsoleRecording.h"
#include "Qt/ConsoleWindow.h"


void consoleWin_t::aviRecordStart(void)
{
	if ( !aviRecordRunning() )
	{
		FCEU_WRAPPER_LOCK();
		if ( aviRecordOpenFile(NULL) == 0 )
		{
			aviDiskThread->start();
		}
		FCEU_WRAPPER_UNLOCK();
	}
}

void consoleWin_t::aviRecordAsStart(void)
{
	if ( aviRecordRunning() )
	{
		return;
	}
	std::string last;
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string lastPath;
	//char dir[512];
	const char *base, *rom;
	QFileDialog  dialog(this, tr("Save AVI Movie for Recording") );
	QList<QUrl> urls;
	QDir d;

	dialog.setFileMode(QFileDialog::AnyFile);

	dialog.setNameFilter(tr("AVI Movies (*.avi) ;; All files (*)"));

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

		d.setPath( QString(base) + "/avi");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		dialog.setDirectory( d.absolutePath() );
	}
	dialog.setDefaultSuffix( tr(".avi") );

	g_config->getOption ("SDL.AviFilePath", &lastPath);
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

	FCEUI_printf ("AVI Recording movie to %s\n", filename.toStdString().c_str() );

	lastPath = QFileInfo(filename).absolutePath().toStdString();

	if ( lastPath.size() > 0 )
	{
		g_config->setOption ("SDL.AviFilePath", lastPath);
	}

	FCEU_WRAPPER_LOCK();
	if ( aviRecordOpenFile( filename.toStdString().c_str() ) == 0 )
	{
		aviDiskThread->start();
	}
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::aviRecordStop(void)
{
	if ( aviRecordRunning() )
	{
		QGuiApplication::setOverrideCursor( QCursor(Qt::BusyCursor) );
		FCEU_WRAPPER_LOCK();
		aviDiskThread->requestInterruption();
		aviDiskThread->quit();
		aviDiskThread->wait(10000);
		FCEU_WRAPPER_UNLOCK();
		QGuiApplication::restoreOverrideCursor();
	}
}

void consoleWin_t::aviAudioEnableChange(bool checked)
{
	aviSetAudioEnable( checked );

	return;
}

void consoleWin_t::setAviHudEnable(bool checked)
{
	fceu11::SetAviEnableHUDrecording( checked );

	g_config->setOption("SDL.RecordHUD", checked );
}

void consoleWin_t::setAviMsgEnable(bool checked)
{
	fceu11::SetAviDisableMovieMessages( !checked );

	g_config->setOption("SDL.MovieMsg", checked );
}

void consoleWin_t::aviVideoFormatChanged(int idx)
{
	aviSetSelVideoFormat(idx);
}

void consoleWin_t::wavRecordStart(void)
{
	if ( !fceu11::WaveRecordRunning() )
	{
		const char *romFile;
		std::string fileName;

		romFile = getRomFile();

		if ( romFile )
		{
			char base[512];
			const char *baseDir = fceu11::GetBaseDirectory();
			std::string lastPath;

			getFileBaseName( romFile, base );

			g_config->getOption ("SDL.WavFilePath", &lastPath);

			if ( lastPath.size() > 0 )
			{
				fileName.assign( lastPath );
				fileName.append( "/" );
			}
			else if ( baseDir )
			{
				fileName.assign( baseDir );
				fileName.append( "/wav/" );
			}
			else
			{
				fileName.clear();
			}
			fileName.append( base );
			fileName.append(".wav");
			//printf("WAV Filepath:'%s'\n", fileName );
		}
		else
		{
			return;
		}
		FCEU_WRAPPER_LOCK();
		fceu11::BeginWaveRecord( fileName.c_str() );
		FCEU_WRAPPER_UNLOCK();
	}
}

void consoleWin_t::wavRecordAsStart(void)
{
	if ( fceu11::WaveRecordRunning() )
	{
		return;
	}
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string lastPath;
	const char *base, *rom;
	QFileDialog  dialog(this, tr("Save WAV Movie for Recording") );
	QList<QUrl> urls;
	QDir d;

	dialog.setFileMode(QFileDialog::AnyFile);

	dialog.setNameFilter(tr("WAV Movies (*.wav) ;; All files (*)"));

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

		d.setPath( QString(base) + "/wav");

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}

		dialog.setDirectory( d.absolutePath() );
	}
	dialog.setDefaultSuffix( tr(".wav") );

	g_config->getOption ("SDL.WavFilePath", &lastPath);
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

	FCEUI_printf ("WAV Recording movie to %s\n", filename.toStdString().c_str() );

	lastPath = QFileInfo(filename).absolutePath().toStdString();

	if ( lastPath.size() > 0 )
	{
		g_config->setOption ("SDL.WavFilePath", lastPath);
	}

	FCEU_WRAPPER_LOCK();
	fceu11::BeginWaveRecord( filename.toStdString().c_str() );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::wavRecordStop(void)
{
	if ( fceu11::WaveRecordRunning() )
	{
		FCEU_WRAPPER_LOCK();
		fceu11::EndWaveRecord();
		FCEU_WRAPPER_UNLOCK();
	}
}
