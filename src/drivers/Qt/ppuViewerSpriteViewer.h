// ppuViewerSpriteViewer.h
//
#pragma once

#include <QWidget>
#include <QDialog>
#include <QTimer>
#include <QRadioButton>
#include <QSpinBox>
#include <QLineEdit>
#include <QCheckBox>
#include <QGroupBox>
#include <QKeyEvent>
#include <QCloseEvent>
#include <QMouseEvent>
#include <QResizeEvent>
#include <QContextMenuEvent>
#include <QPainter>

#include "Qt/ConsoleUtilities.h"
#include "Qt/ppuViewerContext.h"

class oamPatternView_t : public QWidget
{
	Q_OBJECT

	public:
		oamPatternView_t(QWidget *parent = 0);
		~oamPatternView_t(void);

		QPoint convPixToTile(QPoint p);

		int  getSpriteIndex(void);
		void setHover2Focus(bool val);
		bool getHoverFocus(void){ return hover2Focus; };
		void setGridVisibility(bool val);
		bool getGridVisibility(void){ return showGrid; };

		QColor gridColor;
		QColor selTileColor;
	protected:
		void paintEvent(QPaintEvent *event);
		void resizeEvent(QResizeEvent *event);
		void keyPressEvent(QKeyEvent *event);
		void mouseMoveEvent(QMouseEvent *event);
		void mousePressEvent(QMouseEvent * event);
		void contextMenuEvent(QContextMenuEvent *event);
		int  heightForWidth(int w) const;
		QSize  minimumSizeHint(void) const;
		QSize  maximumSizeHint(void) const;
		QSize  sizeHint(void) const;

		int  viewWidth;
		int  viewHeight;

		bool hover2Focus;
		bool showGrid;

		QPoint selSprite;
		int    spriteIdx;
	private:

	private slots:
		void openTilePpuViewer(void);
};

class oamTileView_t : public QWidget
{
	Q_OBJECT

	public:
		oamTileView_t(QWidget *parent = 0);
		~oamTileView_t(void);

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
		int  spriteIdx;
};

class oamPreview_t : public QWidget
{
	Q_OBJECT

	public:
		oamPreview_t(QWidget *parent = 0);
		~oamPreview_t(void);

		void setIndex(int val);
		void setMinScale(int val);

		QColor  boxColor;
	protected:
		void paintEvent(QPaintEvent *event);
		void resizeEvent(QResizeEvent *event);
		int  heightForWidth(int w) const;
		QSize  minimumSizeHint(void) const;
		QSize  maximumSizeHint(void) const;
		QSize sizeHint(void) const;

	private:
		int  viewWidth;
		int  viewHeight;
		int  selSprite;
		int  cx;
		int  cy;
};

class spriteViewerDialog_t : public QDialog
{
   Q_OBJECT

	public:
		spriteViewerDialog_t(QWidget *parent = 0);
		~spriteViewerDialog_t(void);

		oamPatternView_t  *oamView;
		oamPaletteView_t  *palView;
		oamTileView_t     *tileView;
		oamPreview_t      *preView;

	protected:
		void closeEvent(QCloseEvent *bar);
	private:
		QTimer *updateTimer;
		QRadioButton *useSprRam;
		QRadioButton *useCpuPag;
		QSpinBox     *cpuPagIdx;
		QSpinBox     *scanLineEdit;
		QLineEdit    *spriteIndexBox;
		QLineEdit    *tileIndexBox;
		QLineEdit    *tileAddrBox;
		QLineEdit    *palAddrBox;
		QLineEdit    *posBox;
		QCheckBoxRO  *hFlipBox;
		QCheckBoxRO  *vFlipBox;
		QCheckBoxRO  *bgPrioBox;
		QCheckBox    *showPosHex;
		QGroupBox    *previewFrame;

	public slots:
		void closeWindow(void);
	private slots:
		void periodicUpdate(void);
		void setClickFocus(void);
		void setHoverFocus(void);
		void toggleGridVis(void);
		void scanLineChanged(int value);
};
