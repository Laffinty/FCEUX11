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
