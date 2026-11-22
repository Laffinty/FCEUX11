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
// PaletteConf.h
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
#include <QLineEdit>

#include "Qt/main.h"

class PaletteConfDialog_t : public QDialog
{
	Q_OBJECT

public:
	PaletteConfDialog_t(QWidget *parent = 0);
	~PaletteConfDialog_t(void);

	void retranslateUi(void);

protected:
	void closeEvent(QCloseEvent *event);
	void changeEvent(QEvent *event) override;

	QLineEdit *custom_palette_path;
	QCheckBox *useCustom;
	QCheckBox *GrayScale;
	QCheckBox *deemphSwap;
	QPushButton *ntscReset;
	QPushButton *palReset;
	QSlider *tintSlider;
	QSlider *hueSlider;
	QSlider *notchSlider;
	QSlider *saturationSlider;
	QSlider *sharpnessSlider;
	QSlider *contrastSlider;
	QSlider *brightnessSlider;
	QGroupBox *tintFrame;
	QGroupBox *hueFrame;
	QGroupBox *ntscFrame;
	QGroupBox *palFrame;
	QGroupBox *notchFrame;
	QGroupBox *saturationFrame;
	QGroupBox *sharpnessFrame;
	QGroupBox *contrastFrame;
	QGroupBox *brightnessFrame;
	QTimer    *updateTimer;
	QPushButton *closeButton;

private:
public slots:
	void closeWindow(void);
private slots:
	void updatePeriodic(void);
	void hueChanged(int value);
	void tintChanged(int value);
	void openPaletteFile(void);
	void clearPalette(void);
	void ntscResetClicked(void);
	void use_NTSC_Changed(bool v);
	void use_Custom_Changed(int v);
	void force_GrayScale_Changed(int v);
	void deemphswap_Changed(int v);
	void palResetClicked(void);
	void palNotchChanged(int value);
	void palSaturationChanged(int value);
	void palSharpnessChanged(int value);
	void palContrastChanged(int value);
	void palBrightnessChanged(int value);
};
