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
// ppuViewerPatternTables.h
//
#pragma once

#include <QWidget>
#include <QPainter>
#include <QLabel>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QResizeEvent>
#include <QContextMenuEvent>

#include "Qt/ppuViewerContext.h"

class ppuPatternView_t : public QWidget
{
	Q_OBJECT

	public:
		ppuPatternView_t( int patternIndex, QWidget *parent = 0);
		~ppuPatternView_t(void);

		void updateCycleCounter(void);
		void updateSelTileLabel(void);
		void setPattern( ppuPatternTable_t *p );
		void setTileLabel( QLabel *l );
		void setTileCoord( int x, int y );
		void setHoverFocus( bool h );
		QPoint convPixToTile( QPoint p );

		bool getHoverFocus(void){ return hover2Focus; };
		bool getDrawTileGrid(void){ return drawTileGrid; };

		QColor  selTileColor;
		QColor  gridColor;
	protected:
		void paintEvent(QPaintEvent *event);
		void resizeEvent(QResizeEvent *event);
		void keyPressEvent(QKeyEvent *event);
		void mouseMoveEvent(QMouseEvent *event);
		void mousePressEvent(QMouseEvent * event);
		void contextMenuEvent(QContextMenuEvent *event);

		int patternIndex;
		int viewWidth;
		int viewHeight;
		int cycleCount;
		int mode;
		bool drawTileGrid;
		bool hover2Focus;
		QLabel *tileLabel;
		QPoint  selTile;
		ppuPatternTable_t *pattern;
   public slots:
	void toggleTileGridLines(void);
   private slots:
	void showTileMode(void);
	void exitTileMode(void);
	void selPalette0(void);
	void selPalette1(void);
	void selPalette2(void);
	void selPalette3(void);
	void selPalette4(void);
	void selPalette5(void);
	void selPalette6(void);
	void selPalette7(void);
	void selPalette8(void);
	void openTileEditor(void);
	void cycleNextPalette(void);
};
