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
// ppuViewerTileEditor.h
//
#pragma once

#include <QWidget>
#include <QDialog>
#include <QComboBox>
#include <QLabel>
#include <QTimer>
#include <QKeyEvent>
#include <QCloseEvent>
#include <QMouseEvent>
#include <QResizeEvent>
#include <QContextMenuEvent>
#include <QPainter>

#include "Qt/ppuViewerContext.h"

class ppuTileView_t : public QWidget
{
	Q_OBJECT

	public:
		ppuTileView_t(int patternIndex, QWidget *parent = 0);
		~ppuTileView_t(void);

		int  getPatternIndex(void){ return patternIndex; };
		void setPattern(ppuPatternTable_t *p);
		void setPaletteNES(int palIndex);
		void setTileLabel(QLabel *l);
		void setTile(QPoint *t);
		QPoint convPixToCell(QPoint p);
		QPoint getSelPix(void){ return selPix; };
		void setSelCell(QPoint &p);
		void moveSelBoxUpDown(int i);
		void moveSelBoxLeftRight(int i);
	protected:
		void paintEvent(QPaintEvent *event);
		void resizeEvent(QResizeEvent *event);
		void keyPressEvent(QKeyEvent *event);
		void mouseMoveEvent(QMouseEvent *event);
		void mousePressEvent(QMouseEvent * event);
		void contextMenuEvent(QContextMenuEvent *event);

		int patternIndex;
		int paletteIndex;
		int viewWidth;
		int viewHeight;
		int boxWidth;
		int boxHeight;
		bool drawTileGrid;
		QLabel *tileLabel;
		QPoint  selTile;
		QPoint  selPix;
		ppuPatternTable_t *pattern;
};

class ppuTileEditColorPicker_t : public QWidget
{
	Q_OBJECT

	public:
		ppuTileEditColorPicker_t(QWidget *parent = 0);
		~ppuTileEditColorPicker_t(void);

		void  setColor(int colorIndex);
		void  setPaletteNES(int palIndex);

		QPoint convPixToTile(QPoint p);

		static const int NUM_COLORS = 4;

	protected:
		void paintEvent(QPaintEvent *event);
		void resizeEvent(QResizeEvent *event);
		void mouseMoveEvent(QMouseEvent *event);
		void mousePressEvent(QMouseEvent * event);
		int viewWidth;
		int viewHeight;
		int boxWidth;
		int boxHeight;
		int selColor;
		int pxCharWidth;
		int pxCharHeight;
		int paletteIndex;

		QFont   font;
};

class ppuTileEditor_t : public QDialog
{
   Q_OBJECT

	public:
		ppuTileEditor_t(int patternIndex, QWidget *parent = 0);
		~ppuTileEditor_t(void);

		void setTile(QPoint *t);
		void setCellValue(int y, int x, int colorIndex);

		ppuTileView_t  *tileView;
		ppuTileEditColorPicker_t *colorPicker;

	protected:
		void keyPressEvent(QKeyEvent *event);
		void closeEvent(QCloseEvent *bar);

		QTimer     *updateTimer;
	private:
		QLabel    *tileIdxLbl;
		QComboBox *palSelBox;
		int     palIdx;
		int     tileAddr;

	public slots:
		void closeWindow(void);
	private slots:
		void periodicUpdate(void);
		void paletteChanged(int index);
		void showKeyAssignments(void);
};
