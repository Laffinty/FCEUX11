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
// ppuViewerPalette.h
//
#pragma once

#include <QWidget>
#include <QPainter>
#include <QMouseEvent>
#include <QResizeEvent>
#include <QContextMenuEvent>

#include "Qt/ppuViewerContext.h"

class tilePaletteView_t : public QWidget
{
	Q_OBJECT

	public:
		tilePaletteView_t(QWidget *parent = 0);
		~tilePaletteView_t(void);

		void setIndex(int val);
	protected:
		void paintEvent(QPaintEvent *event);
		void resizeEvent(QResizeEvent *event);
		void mouseMoveEvent(QMouseEvent *event);
		void contextMenuEvent(QContextMenuEvent *event);
		int  heightForWidth(int w) const;
		QSize  minimumSizeHint(void) const;
		QSize  maximumSizeHint(void) const;
		QSize  sizeHint(void) const;
		QPoint convPixToCell(QPoint p);

	private:
		int  viewWidth;
		int  viewHeight;
		int  palIdx;
		int  boxWidth;
		int  boxHeight;
		int  selBox;

	private slots:
		void openColorPicker(void);
		void exportPaletteFileDialog(void);
		void copyColor2ClipBoardHex(void);
		void copyColor2ClipBoardRGB(void);
};

class oamPaletteView_t : public QWidget
{
	Q_OBJECT

	public:
		oamPaletteView_t(QWidget *parent = 0);
		~oamPaletteView_t(void);

		void setIndex(int val);
	protected:
		void paintEvent(QPaintEvent *event);
		void resizeEvent(QResizeEvent *event);
		int  heightForWidth(int w) const;
		QSize  minimumSizeHint(void) const;
		QSize  maximumSizeHint(void) const;
		QSize  sizeHint(void) const;

	private:
		int  viewWidth;
		int  viewHeight;
		int  palIdx;
};
