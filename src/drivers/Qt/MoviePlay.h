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
// MoviePlay.h
//

#pragma once

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QLineEdit>
#include <QLabel>
#include <QFrame>
#include <QGroupBox>

#include "Qt/main.h"
#include "Qt/ConsoleUtilities.h"

class MoviePlayDialog_t : public QDialog
{
	Q_OBJECT

public:
	MoviePlayDialog_t(QWidget *parent = 0);
	~MoviePlayDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QComboBox *movSelBox;
	QPushButton *movBrowseBtn;
	QCheckBox *openReadOnly;
	QCheckBox *pauseAtFrame;
	QLineEdit *pauseAtFrameEntry;

	QLabel *movLenLbl;
	QLabel *movFramesLbl;
	QLabel *recCountLbl;
	QLabel *recFromLbl;
	QLabel *romUsedLbl;
	QLabel *romCsumLbl;
	QLabel *curCsumLbl;
	QLabel *emuUsedLbl;
	QLabel *palUsedLbl;
	QLabel *newppuUsedLbl;

	QLabel *fileLabel;
	QLabel *lengthLabel;
	QLabel *framesLabel;
	QLabel *recordCountLabel;
	QLabel *recordedFromLabel;
	QLabel *romUsedLabel;
	QLabel *romChecksumLabel;
	QLabel *currentRomSumLabel;
	QLabel *emulatorUsedLabel;
	QLabel *palLabel;
	QLabel *newPpuLabel;
	QLabel *movLenHeaderLbl;
	QLabel *movFramesHeaderLbl;
	QLabel *recCountHeaderLbl;
	QLabel *recFromHeaderLbl;
	QLabel *romUsedHeaderLbl;
	QLabel *romCsumHeaderLbl;
	QLabel *curCsumHeaderLbl;
	QLabel *emuUsedHeaderLbl;
	QLabel *palUsedHeaderLbl;
	QLabel *newppuUsedHeaderLbl;
	QGroupBox *paramFrame;
	QPushButton *okButton;
	QPushButton *cancelButton;

	fceuDecIntValidtor *validator;

private:
	void doScan(void);
	void clearMovieText(void);
	void updateMovieText(void);
	int addFileToList(const char *file, bool setActive = false);
	bool checkMD5Sum(const char *path, const char *md5);
	void scanDirectory(const char *dirPath, const char *md5);
	void showErrorMsgWindow(const char *str);
	void showWarningMsgWindow(const char *str);

public slots:
	void closeWindow(void);
private slots:
	void openMovie(void);
	void playMovie(void);
	void movieSelect(int index);
	void pauseAtFrameChange(int state);
};
