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
// MovieOptions.h
//

#pragma once

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QLabel>
#include <QFrame>
#include <QGroupBox>
#include <QTreeView>
#include <QTreeWidget>
#include <QStackedWidget>

#include "Qt/main.h"

class MovieOptionsDialog_t : public QDialog
{
	Q_OBJECT

public:
	MovieOptionsDialog_t(QWidget *parent = 0);
	~MovieOptionsDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QCheckBox *readOnlyReplay;
	QCheckBox *pauseAfterPlay;
	QCheckBox *closeAfterPlay;
	QCheckBox *bindSaveStates;
	QCheckBox *dpySubTitles;
	QCheckBox *putSubTitlesAvi;
	QCheckBox *autoBackUp;
	QCheckBox *loadFullStates;
	QCheckBox *aviEnableHUD;
	QCheckBox *aviEnableMsg;
	QCheckBox *aviEnableAudio;
	QComboBox *aviBackend;
	QStackedWidget *aviPageStack;

	QPushButton *closeButton;
	QLabel *aviDriverLabel;
	QLabel *infoLabel;
	QGroupBox *aviGroupBox;

private:
public slots:
	void closeWindow(void);
private slots:
	void readOnlyReplayChanged(int state);
	void pauseAfterPlayChanged(int state);
	void closeAfterPlayChanged(int state);
	void bindSaveStatesChanged(int state);
	void dpySubTitlesChanged(int state);
	void putSubTitlesAviChanged(int state);
	void setAviHudEnable(int state);
	void setAviMsgEnable(int state);
	void setAviAudioEnable(int state);
	void autoBackUpChanged(int state);
	void loadFullStatesChanged(int state);
	void aviBackendChanged(int idx);
};
