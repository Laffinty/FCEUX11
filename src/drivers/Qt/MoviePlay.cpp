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
// MoviePlay.cpp
//
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdlib.h>
#include <string.h>
#include <string>

#include <QHeaderView>
#include <QCloseEvent>
#include <QFileDialog>
#include <QMessageBox>
#include <QGridLayout>
#include <QSettings>

#include "../../fceu.h"
#include "../../movie.h"

#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/input.h"
#include "Qt/config.h"
#include "Qt/keyscan.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/MoviePlay.h"

//----------------------------------------------------------------------------
MoviePlayDialog_t::MoviePlayDialog_t(QWidget *parent)
	: QDialog(parent)
{
	QVBoxLayout *mainLayout, *vbox;
	QHBoxLayout *hbox;
	QGridLayout *grid;
	bool replayReadOnlySetting;
	QSettings settings;

	setWindowTitle(tr("Movie Play"));

	mainLayout = new QVBoxLayout();
	hbox = new QHBoxLayout();

	fileLabel = new QLabel(this);
	fileLabel->setText(tr("File:"));
	movSelBox = new QComboBox();
	movBrowseBtn = new QPushButton(this);
	movBrowseBtn->setText(tr("Browse"));

	movSelBox->setMaximumWidth(512);

	hbox->addWidget(fileLabel, 1);
	hbox->addWidget(movSelBox, 100);
	hbox->addWidget(movBrowseBtn, 1);

	mainLayout->addLayout(hbox);

	paramFrame = new QGroupBox(this);
	paramFrame->setTitle(tr("Parameters:"));
	vbox = new QVBoxLayout();
	hbox = new QHBoxLayout();

	paramFrame->setLayout(vbox);

	openReadOnly = new QCheckBox(this);
	openReadOnly->setText(tr("Open Read-Only"));
	pauseAtFrame = new QCheckBox(this);
	pauseAtFrame->setText(tr("Pause Movie At Frame"));

	validator = new fceuDecIntValidtor(0, 100000000, this);

	pauseAtFrameEntry = new QLineEdit();

	pauseAtFrameEntry->setValidator(validator);

	vbox->addWidget(openReadOnly);
	vbox->addLayout(hbox);
	hbox->addWidget(pauseAtFrame);
	hbox->addWidget(pauseAtFrameEntry);

	mainLayout->addWidget(paramFrame);

	grid = new QGridLayout();
	grid->setColumnStretch(0, 1);
	grid->setColumnStretch(1, 10);

	mainLayout->addLayout(grid);

	movLenLbl = new QLabel();
	movFramesLbl = new QLabel();
	recCountLbl = new QLabel();
	recFromLbl = new QLabel();
	romUsedLbl = new QLabel();
	romCsumLbl = new QLabel();
	curCsumLbl = new QLabel();
	emuUsedLbl = new QLabel();
	palUsedLbl = new QLabel();
	newppuUsedLbl = new QLabel();

	movLenHeaderLbl = new QLabel(this);
	movLenHeaderLbl->setText(tr("Length:"));
	movFramesHeaderLbl = new QLabel(this);
	movFramesHeaderLbl->setText(tr("Frames:"));
	recCountHeaderLbl = new QLabel(this);
	recCountHeaderLbl->setText(tr("Record Count:"));
	recFromHeaderLbl = new QLabel(this);
	recFromHeaderLbl->setText(tr("Recorded From:"));
	romUsedHeaderLbl = new QLabel(this);
	romUsedHeaderLbl->setText(tr("ROM Used:"));
	romCsumHeaderLbl = new QLabel(this);
	romCsumHeaderLbl->setText(tr("ROM Checksum:"));
	curCsumHeaderLbl = new QLabel(this);
	curCsumHeaderLbl->setText(tr("Current ROM Sum:"));
	emuUsedHeaderLbl = new QLabel(this);
	emuUsedHeaderLbl->setText(tr("Emulator Used:"));
	palUsedHeaderLbl = new QLabel(this);
	palUsedHeaderLbl->setText(tr("PAL:"));
	newppuUsedHeaderLbl = new QLabel(this);
	newppuUsedHeaderLbl->setText(tr("New PPU:"));

	grid->addWidget(movLenHeaderLbl, 0, 0, Qt::AlignRight);
	grid->addWidget(movFramesHeaderLbl, 1, 0, Qt::AlignRight);
	grid->addWidget(recCountHeaderLbl, 2, 0, Qt::AlignRight);
	grid->addWidget(recFromHeaderLbl, 3, 0, Qt::AlignRight);
	grid->addWidget(romUsedHeaderLbl, 4, 0, Qt::AlignRight);
	grid->addWidget(romCsumHeaderLbl, 5, 0, Qt::AlignRight);
	grid->addWidget(curCsumHeaderLbl, 6, 0, Qt::AlignRight);
	grid->addWidget(emuUsedHeaderLbl, 7, 0, Qt::AlignRight);
	grid->addWidget(palUsedHeaderLbl, 8, 0, Qt::AlignRight);
	grid->addWidget(newppuUsedHeaderLbl, 9, 0, Qt::AlignRight);

	grid->addWidget(movLenLbl, 0, 1, Qt::AlignLeft);
	grid->addWidget(movFramesLbl, 1, 1, Qt::AlignLeft);
	grid->addWidget(recCountLbl, 2, 1, Qt::AlignLeft);
	grid->addWidget(recFromLbl, 3, 1, Qt::AlignLeft);
	grid->addWidget(romUsedLbl, 4, 1, Qt::AlignLeft);
	grid->addWidget(romCsumLbl, 5, 1, Qt::AlignLeft);
	grid->addWidget(curCsumLbl, 6, 1, Qt::AlignLeft);
	grid->addWidget(emuUsedLbl, 7, 1, Qt::AlignLeft);
	grid->addWidget(palUsedLbl, 8, 1, Qt::AlignLeft);
	grid->addWidget(newppuUsedLbl, 9, 1, Qt::AlignLeft);

	hbox = new QHBoxLayout();
	okButton = new QPushButton(this);
	okButton->setText(tr("Play"));
	cancelButton = new QPushButton(this);
	cancelButton->setText(tr("Cancel"));
	hbox->addWidget(cancelButton);
	hbox->addWidget(okButton);
	okButton->setDefault(true);
	mainLayout->addLayout(hbox);

	setLayout(mainLayout);

	okButton->setIcon( style()->standardIcon( QStyle::SP_MediaPlay ) );
	cancelButton->setIcon( style()->standardIcon( QStyle::SP_DialogCancelButton ) );

	connect(cancelButton, SIGNAL(clicked(void)), this, SLOT(closeWindow(void)));
	connect(okButton, SIGNAL(clicked(void)), this, SLOT(playMovie(void)));

	connect(movBrowseBtn, SIGNAL(clicked(void)), this, SLOT(openMovie(void)));
	connect(movSelBox, SIGNAL(activated(int)), this, SLOT(movieSelect(int)));

	connect(pauseAtFrame, SIGNAL(stateChanged(int)), this, SLOT(pauseAtFrameChange(int)));

	if (suggestReadOnlyReplay)
	{
		replayReadOnlySetting = true;
	}
	else
	{
		replayReadOnlySetting = fceu11::GetMovieToggleReadOnly();
	}
	openReadOnly->setChecked(replayReadOnlySetting);

	pauseAtFrameEntry->setEnabled(pauseAtFrame->isChecked());

	doScan();

	updateMovieText();

	restoreGeometry(settings.value("moviePlayWindow/geometry").toByteArray());
}
//----------------------------------------------------------------------------
MoviePlayDialog_t::~MoviePlayDialog_t(void)
{
	//printf("Destroy Movie Play Window\n");
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::closeEvent(QCloseEvent *event)
{
	QSettings settings;
	settings.setValue("moviePlayWindow/geometry", saveGeometry());
	done(0);
	deleteLater();
	event->accept();
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::retranslateUi(void)
{
	setWindowTitle(tr("Movie Play"));

	if (fileLabel) fileLabel->setText(tr("File:"));
	if (movBrowseBtn) movBrowseBtn->setText(tr("Browse"));
	if (paramFrame) paramFrame->setTitle(tr("Parameters:"));
	if (openReadOnly) openReadOnly->setText(tr("Open Read-Only"));
	if (pauseAtFrame) pauseAtFrame->setText(tr("Pause Movie At Frame"));
	if (movLenHeaderLbl) movLenHeaderLbl->setText(tr("Length:"));
	if (movFramesHeaderLbl) movFramesHeaderLbl->setText(tr("Frames:"));
	if (recCountHeaderLbl) recCountHeaderLbl->setText(tr("Record Count:"));
	if (recFromHeaderLbl) recFromHeaderLbl->setText(tr("Recorded From:"));
	if (romUsedHeaderLbl) romUsedHeaderLbl->setText(tr("ROM Used:"));
	if (romCsumHeaderLbl) romCsumHeaderLbl->setText(tr("ROM Checksum:"));
	if (curCsumHeaderLbl) curCsumHeaderLbl->setText(tr("Current ROM Sum:"));
	if (emuUsedHeaderLbl) emuUsedHeaderLbl->setText(tr("Emulator Used:"));
	if (palUsedHeaderLbl) palUsedHeaderLbl->setText(tr("PAL:"));
	if (newppuUsedHeaderLbl) newppuUsedHeaderLbl->setText(tr("New PPU:"));
	if (okButton) okButton->setText(tr("Play"));
	if (cancelButton) cancelButton->setText(tr("Cancel"));
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::closeWindow(void)
{
	QSettings settings;
	settings.setValue("moviePlayWindow/geometry", saveGeometry());
	done(0);
	deleteLater();
}
//----------------------------------------------------------------------------
//void  MoviePlayDialog_t::readOnlyReplayChanged( int state )
//{
//	suggestReadOnlyReplay = (state != Qt::Unchecked);
//}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::movieSelect(int index)
{
	updateMovieText();
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::pauseAtFrameChange(int state)
{
	pauseAtFrameEntry->setEnabled(state != Qt::Unchecked);
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::clearMovieText(void)
{
	movLenLbl->clear();
	movFramesLbl->clear();
	recCountLbl->clear();
	recFromLbl->clear();
	romUsedLbl->clear();
	romCsumLbl->clear();
	curCsumLbl->clear();
	emuUsedLbl->clear();
	palUsedLbl->clear();
	newppuUsedLbl->clear();
	pauseAtFrameEntry->clear();
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::updateMovieText(void)
{
	int idx;
	std::string path;
	FCEUFILE *fp;
	MOVIE_INFO info;
	bool scanok;
	char stmp[256];

	if (movSelBox->count() == 0)
	{
		return;
	}
	idx = movSelBox->currentIndex();

	path = movSelBox->itemText(idx).toStdString();

	fp = FCEU_fopen(path.c_str(), 0, "rb", 0);

	if (fp == NULL)
	{
		snprintf( stmp, sizeof(stmp), "Error: Failed to open file '%s'", path.c_str());
		showErrorMsgWindow(stmp);
		clearMovieText();
		return;
	}
	scanok = fceu11::MovieGetInfo(fp, info, false);

	if (scanok)
	{
		double div;

		validator->setMinMax(0, info.num_frames);

		snprintf( stmp, sizeof(stmp), "%u", (unsigned)info.num_frames);

		movFramesLbl->setText(tr(stmp));
		pauseAtFrameEntry->setText(tr(stmp));

		div = (fceu11::GetCurrentVidSystem(0, 0)) ? 50.006977968268290849 : 60.098813897440515532; // PAL timing
		double tempCount = (info.num_frames / div) + 0.005;										 // +0.005s for rounding
		int num_seconds = (int)tempCount;
		int fraction = (int)((tempCount - num_seconds) * 100);
		int seconds = num_seconds % 60;
		int minutes = (num_seconds / 60) % 60;
		int hours = (num_seconds / 60 / 60) % 60;
		snprintf( stmp, sizeof(stmp), "%02d:%02d:%02d.%02d", hours, minutes, seconds, fraction);

		movLenLbl->setText(tr(stmp));

		snprintf( stmp, sizeof(stmp), "%u", (unsigned)info.rerecord_count);

		recCountLbl->setText(tr(stmp));

		recFromLbl->setText(tr(info.poweron ? "Power-On" : (info.reset ? "Soft-Reset" : "Savestate")));

		romUsedLbl->setText(tr(info.name_of_rom_used.c_str()));

		romCsumLbl->setText(tr(md5_asciistr(info.md5_of_rom_used)));

		if (GameInfo)
		{
			curCsumLbl->setText(tr(md5_asciistr(GameInfo->MD5)));
		}
		else
		{
			curCsumLbl->clear();
		}

		if (info.emu_version_used < 20000)
		{
			snprintf( stmp, sizeof(stmp), "FCEU %u.%02u.%02u%s", info.emu_version_used / 10000, (info.emu_version_used / 100) % 100, (info.emu_version_used) % 100, info.emu_version_used < 9813 ? " (blip)" : "");
		}
		else
		{
			snprintf( stmp, sizeof(stmp), "FCEUX %u.%02u.%02u", info.emu_version_used / 10000, (info.emu_version_used / 100) % 100, (info.emu_version_used) % 100);
		}
		emuUsedLbl->setText(tr(stmp));

		palUsedLbl->setText(tr(info.pal ? "On" : "Off"));

		newppuUsedLbl->setText(tr(info.ppuflag ? "On" : "Off"));

		if (GameInfo)
		{
			FCEU_strlcpy( stmp, sizeof(stmp), md5_asciistr(GameInfo->MD5));

			if (strcmp(stmp, md5_asciistr(info.md5_of_rom_used)) != 0)
			{
				snprintf( stmp, sizeof(stmp), "Warning: Selected movie file '%s' may not have been created using the currently loaded ROM.", path.c_str());
				showWarningMsgWindow(stmp);
			}
		}
	}
	else
	{
		snprintf( stmp, sizeof(stmp), "Error: Selected file '%s' does not have a recognized movie format.", path.c_str());
		showErrorMsgWindow(stmp);
		clearMovieText();
	}
	delete fp;

	return;
}
//----------------------------------------------------------------------------
int MoviePlayDialog_t::addFileToList(const char *file, bool setActive)
{
	int n = 0;

	for (int i = 0; i < movSelBox->count(); i++)
	{
		if (strcmp(file, movSelBox->itemText(i).toStdString().c_str()) == 0)
		{
			if (setActive)
			{
				movSelBox->setCurrentIndex(i);
			}
			return -1;
		}
	}

	n = movSelBox->count();

	movSelBox->addItem(tr(file), n);

	if (setActive)
	{
		movSelBox->setCurrentIndex(n);
	}

	return 0;
}
//----------------------------------------------------------------------------
bool MoviePlayDialog_t::checkMD5Sum(const char *path, const char *md5)
{
	FCEUFILE *fp;
	MOVIE_INFO info;
	bool scanok, md5Match = false;

	fp = FCEU_fopen(path, 0, "rb", 0);

	if (fp == NULL)
	{
		return md5Match;
	}
	scanok = fceu11::MovieGetInfo(fp, info, true);

	if (scanok)
	{
		if (strcmp(md5, md5_asciistr(info.md5_of_rom_used)) == 0)
		{
			md5Match = true;
		}
	}
	delete fp;

	return md5Match;
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::scanDirectory(const char *dirPath, const char *md5)
{
	QDir dir;
	QFileInfoList list;
	std::string path;
	const QStringList filters({"*.fm2"});

	path.assign(dirPath);

	dir.setPath(QString::fromStdString(path));

	list = dir.entryInfoList(filters, QDir::Files);

	for (int i = 0; i < list.size(); ++i)
	{
		QFileInfo fileInfo = list.at(i);

		path = std::string(dirPath) + fileInfo.fileName().toStdString();

		//printf("File: '%s'\n", path.c_str() );

		if (checkMD5Sum(path.c_str(), md5))
		{
			addFileToList(path.c_str());
		}
	}
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::doScan(void)
{
	std::string path, last;
	const char *romFile;
	const char *baseDir = fceu11::GetBaseDirectory();
	std::string lastDir;
	char md5[256];

	md5[0] = 0;

	if (GameInfo)
	{
		FCEU_strlcpy( md5, sizeof(md5), md5_asciistr(GameInfo->MD5));
	}

	path = std::string(baseDir) + "/movies/";

	scanDirectory(path.c_str(), md5);

	romFile = getRomFile();

	if (romFile != NULL)
	{
		std::string dir;

		parseFilepath(romFile, &dir);

		path = dir;

		scanDirectory(path.c_str(), md5);
	}

	g_config->getOption("SDL.LastOpenMovie", &last);

	getDirFromFile(last.c_str(), lastDir);

	scanDirectory(lastDir.c_str(), md5);
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::playMovie(void)
{
	int idx, pauseframe = 0;
	bool replayReadOnlySetting, movieLoadError = false;

	if (movSelBox->count() == 0)
	{
		return;
	}
	std::string path;

	idx = movSelBox->currentIndex();

	path = movSelBox->itemText(idx).toStdString();

	replayReadOnlySetting = openReadOnly->isChecked();

	if (pauseAtFrame->isChecked())
	{
		pauseframe = strtol(pauseAtFrameEntry->text().toStdString().c_str(), NULL, 0);
	}

	FCEU_WRAPPER_LOCK();
	if (fceu11::LoadMovie(path.c_str(),
						replayReadOnlySetting, pauseframe ? pauseframe : false) == false)
	{
		movieLoadError = true;
	}
	FCEU_WRAPPER_UNLOCK();

	if (movieLoadError)
	{
		char stmp[256];
		snprintf( stmp, sizeof(stmp), "Error: Could not load movie file: %s \n", path.c_str());
		showErrorMsgWindow(stmp);
	}
	else
	{
		closeWindow();
	}
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::openMovie(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	std::string last;
	std::string dir;
	char md5Match = 0;
	QFileDialog dialog(this, tr("Open FM2 Movie"));

	dialog.setFileMode(QFileDialog::ExistingFile);

	dialog.setNameFilter(tr("FM2 Movies (*.fm2) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter(QDir::AllEntries | QDir::AllDirs | QDir::Hidden);
	dialog.setLabelText(QFileDialog::Accept, tr("Open"));

	g_config->getOption("SDL.LastOpenMovie", &last);

	getDirFromFile(last.c_str(), dir);

	dialog.setDirectory(tr(dir.c_str()));

	// Check config option to use native file dialog or not
	g_config->getOption("SDL.UseNativeFileDialog", &useNativeFileDialogVal);

	dialog.setOption(QFileDialog::DontUseNativeDialog, !useNativeFileDialogVal);

	ret = dialog.exec();

	if (ret)
	{
		QStringList fileList;
		fileList = dialog.selectedFiles();

		if (fileList.size() > 0)
		{
			filename = fileList[0];
		}
	}

	if (filename.isNull())
	{
		return;
	}
	qDebug() << "selected file path : " << filename.toUtf8();

	if (GameInfo)
	{
		char md5[256];

		FCEU_strlcpy( md5, sizeof(md5), md5_asciistr(GameInfo->MD5));

		if (checkMD5Sum(filename.toStdString().c_str(), md5))
		{
			md5Match = 1;
		}

		if (!md5Match)
		{
			printf("Warning MD5 Mismatch\n");
		}
	}

	addFileToList(filename.toStdString().c_str(), true);

	updateMovieText();

	g_config->setOption("SDL.LastOpenMovie", filename.toStdString().c_str());

	return;
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::showErrorMsgWindow(const char *str)
{
	QMessageBox msgBox(this);

	msgBox.setIcon(QMessageBox::Critical);
	msgBox.setText(tr(str));
	msgBox.show();
	msgBox.exec();
}
//----------------------------------------------------------------------------
void MoviePlayDialog_t::showWarningMsgWindow(const char *str)
{
	QMessageBox msgBox(this);

	msgBox.setIcon(QMessageBox::Warning);
	msgBox.setText(tr(str));
	msgBox.show();
	msgBox.exec();
}
//----------------------------------------------------------------------------
