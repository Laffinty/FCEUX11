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
// MsgLogViewer.h
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
#include <QTimer>
#include <QFrame>
#include <QGroupBox>
#include <QPlainTextEdit>

#include "Qt/main.h"

class MsgLogViewDialog_t : public QDialog
{
	Q_OBJECT

public:
	MsgLogViewDialog_t(QWidget *parent = 0);
	~MsgLogViewDialog_t(void);

	// hotfix4 D-15: handle LanguageChange so tr() in the constructor
	// re-fires when the user switches language.
	void changeEvent(QEvent *event) override;

protected:
	void closeEvent(QCloseEvent *event);

	QTimer *updateTimer;
	QPlainTextEdit *txtView;
	// hotfix4 D-15: promoted from constructor locals so changeEvent can
	// re-apply tr() on language switch.
	QPushButton *clearBtn;
	QPushButton *closeBtn;

	size_t totalLines;

private:
public slots:
	void closeWindow(void);
private slots:
	void updatePeriodic(void);
	void clearLog(void);
};
