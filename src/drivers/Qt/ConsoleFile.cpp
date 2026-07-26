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


#include "Qt/ConsoleFile.h"
#include "Qt/ConsoleWindow.h"

//---------------------------------------------------------------------------
int  consoleWin_t::showListSelectDialog( const char *title, std::vector <std::string> &l )
{
	if ( QThread::currentThread() == emulatorThread )
	{
		printf("Cannot display list selection dialog from within emulation thread...\n");
		return 0;
	}
	int ret, idx = 0;
	QDialog dialog(this);
	QVBoxLayout *mainLayout;
	QHBoxLayout *hbox;
	QPushButton *okButton, *cancelButton;
	QTreeWidget *tree;
	QTreeWidgetItem *item;
	QSettings  settings;

	dialog.setWindowTitle( tr(title) );

	tree = new QTreeWidget();

	tree->setColumnCount(1);

	item = new QTreeWidgetItem();
	item->setText( 0, QString::fromStdString( "File" ) );
	item->setTextAlignment( 0, Qt::AlignLeft);

	tree->setHeaderItem( item );

	tree->header()->setSectionResizeMode( QHeaderView::ResizeToContents );

	for (size_t i=0; i<l.size(); i++)
	{
		item = new QTreeWidgetItem();

		item->setText( 0, QString::fromStdString( l[i] ) );

		item->setTextAlignment( 0, Qt::AlignLeft);

		tree->addTopLevelItem( item );
	}

	mainLayout = new QVBoxLayout();

	hbox         = new QHBoxLayout();
	okButton     = new QPushButton( tr("OK") );
	cancelButton = new QPushButton( tr("Cancel") );

	mainLayout->addWidget( tree );
	mainLayout->addLayout( hbox );
	hbox->addWidget( cancelButton );
	hbox->addWidget(     okButton );

	connect(     okButton, SIGNAL(clicked(void)), &dialog, SLOT(accept(void)) );
	connect( cancelButton, SIGNAL(clicked(void)), &dialog, SLOT(reject(void)) );

	    okButton->setIcon( style()->standardIcon( QStyle::SP_DialogOkButton ) );
	cancelButton->setIcon( style()->standardIcon( QStyle::SP_DialogCancelButton ) );

	okButton->setDefault(true);

	dialog.setLayout( mainLayout );

	// Restore Window Geometry
	dialog.restoreGeometry(settings.value("ArchiveViewer/geometry").toByteArray());

	// Run Dialog Execution Loop
	ret = dialog.exec();

	// Save Window Geometry
	settings.setValue("ArchiveViewer/geometry", dialog.saveGeometry());

	if ( ret == QDialog::Accepted )
	{
		idx = 0;

		item = tree->currentItem();

		if ( item != NULL )
		{
			idx = tree->indexOfTopLevelItem(item);
		}
	}
	else
	{
		idx = -1;
	}
	return idx;
}
//---------------------------------------------------------------------------

void consoleWin_t::openROMFile(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string last;
	std::string dir;
	const char *romDir;
	QFileDialog  dialog(this, tr("Open ROM File") );
	QList<QUrl> urls;
	QDir d;

	const QStringList filters(
			{ "All Useable files (*.nes *.NES *.nsf *.NSF *.fds *.FDS *.unf *.UNF *.unif *.UNIF *.zip *.ZIP, *.7z *.7zip)",
           "NES files (*.nes *.NES)",
           "NSF files (*.nsf *.NSF)",
           "UNF files (*.unf *.UNF *.unif *.UNIF)",
           "FDS files (*.fds *.FDS)",
           "Any files (*)"
         });

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());
	urls << QUrl::fromLocalFile( QDir( fceu11::GetBaseDirectory() ).absolutePath() );

	romDir = getenv("FCEUX_ROM_PATH");

	if ( romDir != NULL )
	{
		d.setPath(romDir);

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}
	}

	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilters( filters );

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Open") );

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

	g_config->setOption ("SDL.LastOpenFile", filename.toStdString().c_str() );

	FCEU_WRAPPER_LOCK();
	CloseGame ();
	LoadGame ( filename.toStdString().c_str() );
	FCEU_WRAPPER_UNLOCK();

   return;
}

void consoleWin_t::loadRomRequestCB( QString s )
{
	printf("Load ROM Req: '%s'\n", s.toStdString().c_str() );
	FCEU_WRAPPER_LOCK();
	CloseGame ();
	LoadGame ( s.toStdString().c_str() );
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::closeROMCB(void)
{
	FCEU_WRAPPER_LOCK();
	CloseGame();
	FCEU_WRAPPER_UNLOCK();
}

void consoleWin_t::loadNSF(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string last;
	std::string dir;
	const char *romDir;
	QFileDialog  dialog(this, tr("Load NSF File") );
	QList<QUrl> urls;
	QDir d;

	urls << QUrl::fromLocalFile( QDir::rootPath() );
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::HomeLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DesktopLocation).first());
	urls << QUrl::fromLocalFile(QStandardPaths::standardLocations(QStandardPaths::DownloadLocation).first());
	urls << QUrl::fromLocalFile( QDir( fceu11::GetBaseDirectory() ).absolutePath() );

	romDir = getenv("FCEUX_ROM_PATH");

	if ( romDir != NULL )
	{
		d.setPath(romDir);

		if ( d.exists() )
		{
			urls << QUrl::fromLocalFile( d.absolutePath() );
		}
	}
	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilter(tr("NSF Sound Files (*.nsf *.NSF) ;; Zip Files (*.zip *.ZIP) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter( QDir::AllEntries | QDir::AllDirs | QDir::Hidden );
	dialog.setLabelText( QFileDialog::Accept, tr("Load") );

	g_config->getOption ("SDL.LastOpenNSF", &last );

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

	g_config->setOption ("SDL.LastOpenNSF", filename.toStdString().c_str() );

	FCEU_WRAPPER_LOCK();
	LoadGame( filename.toStdString().c_str() );
	FCEU_WRAPPER_UNLOCK();
}



void consoleWin_t::quickLoad(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::LoadStateFile( NULL );
	FCEU_WRAPPER_UNLOCK();
}




void consoleWin_t::quickSave(void)
{
	FCEU_WRAPPER_LOCK();
	fceu11::SaveStateFile( NULL );
	FCEU_WRAPPER_UNLOCK();
}





void consoleWin_t::mainMenuOpen(void)
{
	//printf("Main Menu Open\n");

	mainMenuEmuWasPaused = fceu11::IsEmulationPaused() ? true : false;

	if ( mainMenuPauseWhenActv && !mainMenuEmuPauseSet && !mainMenuEmuWasPaused )
	{
		fceu11::ToggleEmulationPause();
		mainMenuEmuPauseSet  = true;
	}
}

void consoleWin_t::mainMenuClose(void)
{
	//printf("Main Menu Close\n");

	if ( mainMenuEmuPauseSet )
	{
		bool isPaused = fceu11::IsEmulationPaused() ? true : false;

		if ( isPaused != mainMenuEmuWasPaused )
		{
			fceu11::ToggleEmulationPause();
		}
		mainMenuEmuPauseSet = false;
	}
}

void consoleWin_t::prepareScreenShot(void)
{
	// Set a timer single shot to take the screen shot. This gives time
	// for the GUI to remove the menu from view before taking the image.
	QTimer::singleShot( 100, Qt::CoarseTimer, this, SLOT(takeScreenShot(void)) );
}

//void consoleWin_t::takeScreenShot(void)
//{
//	FCEU_WRAPPER_LOCK();
//	fceu11::SaveSnapshot();
//	FCEU_WRAPPER_UNLOCK();
//}

void consoleWin_t::takeScreenShot(void)
{
	int u=0;
	QPixmap  image;
	QScreen *screen = QGuiApplication::primaryScreen();

	if (const QWindow *window = windowHandle())
	{
		screen = window->screen();
	}

	if (screen == NULL)
	{
		FCEU_DispMessage("Error saving screen snapshot.",0);
		return;
	}

	FCEU_WRAPPER_LOCK();

	if ( viewport_GL )
	{
		image = QPixmap::fromImage( viewport_GL->grabFramebuffer() );
	}
	else if ( viewport_SDL )
	{
		image = screen->grabWindow( viewport_SDL->winId() );
	}
	else if ( viewport_QWidget )
	{
		image = screen->grabWindow( viewport_QWidget->winId() );
	}

	for (u = 0; u < 99999; ++u)
	{
		FILE *pp = FCEUD_UTF8fopen( FCEU_MakeFName(FCEUMKF_SNAP,u,"png").c_str(), "rb");

		if (pp == NULL)
		{
			break;
		}
		fclose(pp);
	}

	image.save( tr( FCEU_MakeFName(FCEUMKF_SNAP,u,"png").c_str() ), "png" );

	FCEU_WRAPPER_UNLOCK();

	FCEU_DispMessage("Screen snapshot %d saved.",0,u);
}

void consoleWin_t::loadLua(void)
{
#ifdef _S9XLUA_H
	LuaControlDialog_t *luaCtrlWin;

	//printf("Open Lua Control Window\n");
	
   luaCtrlWin = new LuaControlDialog_t(this);
	
   luaCtrlWin->show();
#endif
}
