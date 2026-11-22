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
// MovieRecord.h
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

class MovieRecordDialog_t : public QDialog
{
	Q_OBJECT

public:
	MovieRecordDialog_t(QWidget *parent = 0);
	~MovieRecordDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QLineEdit   *dirEdit;
	QLineEdit   *fileEdit;
	QLineEdit   *authorEdit;
	QPushButton *fileBrowse;
	QPushButton *okButton;
	QPushButton *cancelButton;
	QComboBox   *stateSel;
	QLabel      *pathLabel;
	QLabel      *fileLabel;
	QLabel      *recordLabel;
	QLabel      *authorLabel;
	std::string  filepath;
	std::string  ic_file;

private:
	void setFilePath( QString s );
	void setFilePath( std::string s );
	void setFilePath( const char *s );
	void setLoadState(void);

public slots:
	void closeWindow(void);
private slots:
	void browseFiles(void);
	void recordMovie(void);
	void stateSelChanged(int idx);
};
