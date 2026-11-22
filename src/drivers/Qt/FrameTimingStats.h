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
// FrameTimingStats.h
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
#include <QTreeWidget>
#include <QTreeWidgetItem>

#include "Qt/main.h"

class FrameTimingDialog_t : public QDialog
{
	Q_OBJECT

public:
	FrameTimingDialog_t(QWidget *parent = 0);
	~FrameTimingDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QTimer *updateTimer;
	QCheckBox *timingEnable;
	QTreeWidgetItem *frameTimeAbs;
	QTreeWidgetItem *frameTimeDel;
	QTreeWidgetItem *frameTimeWork;
	QTreeWidgetItem *frameTimeWorkPct;
	QTreeWidgetItem *frameTimeIdle;
	QTreeWidgetItem *frameTimeIdlePct;
	QTreeWidgetItem *frameLateCount;
	QTreeWidgetItem *videoTimeAbs;
	QGroupBox *statFrame;

	QTreeWidget *tree;
	QPushButton *resetButton;
	QPushButton *closeButton;
	QTreeWidgetItem *headerItem;

private:
	void updateTimingStats(void);

public slots:
	void closeWindow(void);
private slots:
	void updatePeriodic(void);
	void resetTimingClicked(void);
	void timingEnableChanged(int state);
};
