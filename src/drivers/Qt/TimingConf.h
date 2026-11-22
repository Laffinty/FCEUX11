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
// TimingConf.h
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
#include <QSlider>
#include <QFrame>
#include <QGroupBox>
#include <QSpinBox>
#include <QTreeView>
#include <QTreeWidget>

#include "Qt/main.h"

class TimingConfDialog_t : public QDialog
{
	Q_OBJECT

public:
	TimingConfDialog_t(QWidget *parent = 0);
	~TimingConfDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QCheckBox *emuPrioCtlEna;
#ifdef WIN32
	QComboBox *emuSchedPrioBox;
	QComboBox *guiSchedPrioBox;
#else
	QComboBox *emuSchedPolicyBox;
	QSlider *emuSchedPrioSlider;
	QSlider *emuSchedNiceSlider;
	QLabel *emuSchedPrioLabel;
	QLabel *emuSchedNiceLabel;
	QComboBox *guiSchedPolicyBox;
	QSlider *guiSchedPrioSlider;
	QSlider *guiSchedNiceSlider;
	QLabel *guiSchedPrioLabel;
	QLabel *guiSchedNiceLabel;
#endif
	QComboBox *timingDevSelBox;

	QGroupBox *ppuOverClockBox;
	QSpinBox  *postRenderBox;
	QSpinBox  *vblankScanlinesBox;
	QCheckBox *no7bitSamples;

	QTimer    *updateTimer;

private:
	void updatePolicyBox(void);
	void updateSliderLimits(void);
	void updateSliderValues(void);
	void updateTimingMech(void);
	void updateOverclocking(void);
	void saveValues(void);

public slots:
	void closeWindow(void);
private slots:
	void periodicUpdate(void);
	void emuSchedCtlChange(int state);
	void emuSchedNiceChange(int val);
	void emuSchedPrioChange(int val);
	void emuSchedPolicyChange(int index);
	void guiSchedNiceChange(int val);
	void guiSchedPrioChange(int val);
	void guiSchedPolicyChange(int index);
	void emuTimingMechChange(int index);
	void overclockingToggled(bool on);
	void postRenderChanged(int value);
	void vblankScanlinesChanged(int value);
	void no7bitChanged(int value);
};
