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
// LuaControl.h
//

#pragma once

#include <stdio.h>
#include <stdarg.h>

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
#include <QLineEdit>
#include <QTextEdit>

#include "Qt/main.h"

class LuaControlDialog_t : public QDialog
{
	Q_OBJECT

public:
	LuaControlDialog_t(QWidget *parent = 0);
	~LuaControlDialog_t(void);

	void refreshState(void);
	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *bar);
	void changeEvent(QEvent *event) override;
	void openLuaKillMessageBox(void);

	QTimer *periodicTimer;
	QLineEdit *scriptPath;
	QLineEdit *scriptArgs;
	QPushButton *browseButton;
	QPushButton *stopButton;
	QPushButton *startButton;
	QTextEdit *luaOutput;

private:
public slots:
	void closeWindow(void);
private slots:
	void updatePeriodic(void);
	void openLuaScriptFile(void);
	void startLuaScript(void);
	void stopLuaScript(void);
};

// Formatted print
int LuaPrintfToWindowConsole( __FCEU_PRINTF_FORMAT const char *format, ...) __FCEU_PRINTF_ATTRIBUTE( 1, 2 );

void PrintToWindowConsole(intptr_t hDlgAsInt, const char *str);

int LuaKillMessageBox(void);
