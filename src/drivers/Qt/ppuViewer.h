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
// ppuViewer.h
//

#pragma once

#include <QWidget>
#include <QDialog>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QComboBox>
#include <QCheckBox>
#include <QPushButton>
#include <QRadioButton>
#include <QLabel>
#include <QFrame>
#include <QTimer>
#include <QSlider>
#include <QSpinBox>
#include <QLineEdit>
#include <QGroupBox>
#include <QCloseEvent>
#include <QPropertyAnimation>

#include "Qt/main.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/ppuViewerContext.h"
#include "Qt/ppuViewerPalette.h"
#include "Qt/ppuViewerTileEditor.h"
#include "Qt/ppuViewerSpriteViewer.h"
#include "Qt/ppuViewerPatternTables.h"

class ppuViewerDialog_t : public QDialog
{
   Q_OBJECT

	public:
		ppuViewerDialog_t(QWidget *parent = 0);
		~ppuViewerDialog_t(void);

		ppuPatternView_t  *patternView[2];
		tilePaletteView_t *tilePalView[8];

		void retranslateUi(void);
	protected:

		void closeEvent(QCloseEvent *bar);
		void changeEvent(QEvent *event) override;
	private:

		QGroupBox  *patternFrame[2];
		QGroupBox  *paletteFrame;
		QLabel     *tileLabel[2];
		QCheckBox  *sprite8x16Cbox[2];
		QCheckBox  *maskUnusedCbox;
		QCheckBox  *invertMaskCbox;
		QSlider    *refreshSlider;
		QSpinBox   *scanLineEdit;
		QLabel     *refreshMoreLbl;
		QLabel     *refreshLessLbl;
		QLabel     *scanLineLbl;
		QTimer     *updateTimer;

		int         cycleCount;

	public slots:
		void closeWindow(void);
	private slots:
		void periodicUpdate(void);
		void sprite8x16Changed0(int state);
		void sprite8x16Changed1(int state);
		void invertMaskChanged(int state);
		void maskUnusedGraphicsChanged(int state);
		void refreshSliderChanged(int value);
		void scanLineChanged(int value);
		void setClickFocus(void);
		void setHoverFocus(void);
};

extern spriteViewerDialog_t *spriteViewWindow;

int openPPUViewWindow( QWidget *parent );
int openOAMViewWindow( QWidget *parent );
void setPPUSelPatternTile( int table, int x, int y );
