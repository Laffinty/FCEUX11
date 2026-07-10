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
