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
// InputConf.h
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
#include <QTreeView>
#include <QTreeWidget>
#include <QTimer>

#include "Qt/main.h"

class InputConfDialog_t : public QDialog
{
	Q_OBJECT

public:
	InputConfDialog_t(QWidget *parent = 0);
	~InputConfDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QTimer *inputTimer;
	QCheckBox *fourScoreEna;
	QCheckBox *port2Mic;
	QCheckBox *autoPreset;
	QLabel *nesPortLabel[2];
	QPushButton *nesPortConfButton[2];
	QComboBox *nesPortComboxBox[2];
	QLabel *expPortLabel;
	QPushButton *expPortConfButton;
	QComboBox *expPortComboxBox;
	QPushButton *loadConfigButton;
	QPushButton *saveConfigButton;
	QPushButton *closeButton;

	int curNesInput[3];
	int usrNesInput[3];

private:
	void setInputs(void);
	void updatePortLabels(void);
	void updatePortComboBoxes(void);
	void openPortConfig(int portNum);

public slots:
	void closeWindow(void);
private slots:
	void port1Configure(void);
	void port2Configure(void);
	void expPortConfigure(void);
	void port1Select(int index);
	void port2Select(int index);
	void expSelect(int index);
	void fourScoreChanged(int state);
	void port2MicChanged(int state);
	void autoPresetChanged(int state);
	void openLoadPresetFile(void);
	void openSavePresetFile(void);
	void updatePeriodic(void);
};

void openInputConfWindow(QWidget *parent);
