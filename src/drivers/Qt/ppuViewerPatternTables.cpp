// ppuViewerPatternTables.cpp
//
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdint.h>

#include <QMenu>
#include <QAction>
#include <QActionGroup>
#include <QPainter>

#include "../../fceu.h"

#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/config.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/ppuViewerPatternTables.h"
#include "Qt/ppuViewerTileEditor.h"

ppuPatternView_t::ppuPatternView_t( int patternIndexID, QWidget *parent)
	: QWidget(parent)
{
	this->setFocusPolicy(Qt::StrongFocus);
	this->setMouseTracking(true);
	patternIndex = patternIndexID;
	setMinimumWidth( 256 );
	setMinimumHeight( 256 );
	viewWidth = 256;
	viewHeight = 256;
	tileLabel = NULL;
	mode = 0;
	cycleCount   = 0;
	drawTileGrid = true;
	hover2Focus  = false;

	selTileColor.setRgb(255,255,255);
	gridColor.setRgb(128,128,128);
	selTile.setX(-1);
	selTile.setY(-1);

	if ( patternIndexID )
	{
		fceuLoadConfigColor("SDL.PPU_TileSelColor1"  , &selTileColor  );
		fceuLoadConfigColor("SDL.PPU_TileGridColor1" , &gridColor     );
		g_config->getOption("SDL.PPU_TileShowGrid1"  , &drawTileGrid );
	}
	else
	{
		fceuLoadConfigColor("SDL.PPU_TileSelColor0"  , &selTileColor  );
		fceuLoadConfigColor("SDL.PPU_TileGridColor0" , &gridColor     );
		g_config->getOption("SDL.PPU_TileShowGrid0"  , &drawTileGrid );
	}

	g_config->getOption("SDL.PPU_TileFocusPolicy", &hover2Focus );
}

void ppuPatternView_t::setPattern( ppuPatternTable_t *p )
{
	pattern = p;
}

void ppuPatternView_t::setTileLabel( QLabel *l )
{
	tileLabel = l;
}

void ppuPatternView_t::setHoverFocus( bool h )
{
	hover2Focus = h;
	g_config->setOption("SDL.PPU_TileFocusPolicy", hover2Focus );
}

void ppuPatternView_t::setTileCoord( int x, int y )
{
	selTile.setX(x);
	selTile.setY(y);
	cycleCount = 0;
}

ppuPatternView_t::~ppuPatternView_t(void)
{

}

QPoint ppuPatternView_t::convPixToTile( QPoint p )
{
	QPoint t(0,0);
	int x,y,w,h,i,j,ii,jj,rr;

	x = p.x(); y = p.y();

	w = pattern->w;
	h = pattern->h;

	i = w == 0 ? 0 : x / (w*8);
	j = h == 0 ? 0 : y / (h*8);

	if ( PPUView_sprite16Mode[ patternIndex ] )
	{
		rr = (j%2);
		jj =  j;

		if ( rr )
		{
			jj--;
		}

		ii = (i*2)+rr;

		if ( ii >= 16 )
		{
			ii = ii % 16;
			jj++;
		}
	}
	else
	{
		ii = i; jj = j;
	}

	t.setX(ii);
	t.setY(jj);

	return t;
}

void ppuPatternView_t::resizeEvent(QResizeEvent *event)
{
	viewWidth  = event->size().width();
	viewHeight = event->size().height();

	pattern->w = viewWidth / 128;
	pattern->h = viewHeight / 128;
}

void ppuPatternView_t::keyPressEvent(QKeyEvent *event)
{
	if ( event->key() == Qt::Key_Z )
	{
		mode = !mode;

		event->accept();
	}
	else if ( event->key() == Qt::Key_G )
	{
		drawTileGrid = !drawTileGrid;

		event->accept();
	}
	else if ( event->key() == Qt::Key_E )
	{
		openTileEditor();

		event->accept();
	}
	else if ( event->key() == Qt::Key_P )
	{
		pindex[ patternIndex ] = (pindex[ patternIndex ] + 1) % 9;
	
		PPUViewSkip = 100;
	
		FCEUD_UpdatePPUView( -1, 0 );

		event->accept();
	}
	else if ( event->key() == Qt::Key_F5 )
	{
		PPUViewSkip = 100;

		FCEUD_UpdatePPUView( -1, 1 );

		event->accept();
	}
	else if ( event->key() == Qt::Key_Up )
	{
		int x, y;

		y = selTile.y();
		x = selTile.x();

		if ( PPUView_sprite16Mode[ patternIndex ] )
		{
			if ( (x % 2) == 0 )
			{
				y -= 2;
				x++;
			}
			else
			{
				x--;
			}
		}
		else
		{
			y--;
		}
		if ( y < 0 )
		{
			y += 16;
		}
		selTile.setX(x);
		selTile.setY(y);

		cycleCount = 0;

		updateSelTileLabel();

		event->accept();
	}
	else if ( event->key() == Qt::Key_Down )
	{
		int x,y;

		y = selTile.y();
		x = selTile.x();

		if ( PPUView_sprite16Mode[ patternIndex ] )
		{
			if ( (x % 2) )
			{
				y += 2;
				x--;
			}
			else
			{
				x++;
			}
		}
		else
		{
			y++;
		}
		if ( y >= 16 )
		{
			y = 0;
		}
		selTile.setX(x);
		selTile.setY(y);

		cycleCount = 0;

		updateSelTileLabel();

		event->accept();
	}
	else if ( event->key() == Qt::Key_Left )
	{
		int x,y;

		x = selTile.x();
		y = selTile.y();

		if ( PPUView_sprite16Mode[ patternIndex ] )
		{
			x -= 2;

			if ( x < 0 )
			{
				if ( y % 2 )
				{
					y--;
				}
				else
				{
					y++;
				}
				x += 16;
			}
		}
		else
		{
			x--;
		}
		if ( x < 0 )
		{
			x = 15;
		}
		selTile.setX(x);
		selTile.setY(y);

		cycleCount = 0;

		updateSelTileLabel();

		event->accept();
	}
	else if ( event->key() == Qt::Key_Right )
	{
		int x,y;

		x = selTile.x();
		y = selTile.y();

		if ( PPUView_sprite16Mode[ patternIndex ] )
		{
			x += 2;

			if ( x >= 16 )
			{
				if ( y % 2 )
				{
					y--;
				}
				else
				{
					y++;
				}
				if ( x % 2 )
				{
					x = 1;
				}
				else
				{
					x = 0;
				}
			}
		}
		else
		{
			x++;
		}
		if ( x >= 16 )
		{
			x = 0;
		}
		selTile.setX(x);
		selTile.setY(y);

		cycleCount = 0;

		updateSelTileLabel();

		event->accept();
	}

}

void ppuPatternView_t::updateSelTileLabel(void)
{
	char stmp[32];
	if ( (selTile.y() >= 0) && (selTile.x() >= 0) )
	{
		snprintf( stmp, sizeof(stmp), "Tile: $%X%X", selTile.y(), selTile.x() );
	}
	else
	{
		FCEU_strlcpy( stmp, sizeof(stmp), "Tile:");
	}
	tileLabel->setText( tr(stmp) );
}

void ppuPatternView_t::mouseMoveEvent(QMouseEvent *event)
{
	if ( mode == 0 )
	{
		QPoint tile = convPixToTile( event->pos() );

		if ( (tile.x() < 16) && (tile.y() < 16) )
		{
			if ( hover2Focus )
			{
				selTile = tile;

				cycleCount = 0;

				updateSelTileLabel();
			}
		}
	}
}

void ppuPatternView_t::mousePressEvent(QMouseEvent * event)
{
	QPoint tile = convPixToTile( event->pos() );

	if ( event->button() == Qt::LeftButton )
	{
		if ( (tile.x() < 16) && (tile.y() < 16) )
		{
			selTile = tile;

			cycleCount = 0;

			updateSelTileLabel();
		}
	}
}

void ppuPatternView_t::contextMenuEvent(QContextMenuEvent *event)
{
	QAction *act;
	QMenu menu(this);
	QMenu *subMenu;
	QActionGroup *group;
	QAction *paletteAct[9];
	char stmp[64];

	act = new QAction(tr("Open Tile &Editor"), &menu);
	act->setShortcut( QKeySequence(tr("E")));
	connect( act, SIGNAL(triggered(void)), this, SLOT(openTileEditor(void)) );
	menu.addAction( act );

	if ( mode )
	{
		snprintf( stmp, sizeof(stmp), "Exit Tile &View: %X%X", selTile.y(), selTile.x() );
		
		act = new QAction(tr(stmp), &menu);
		act->setShortcut( QKeySequence(tr("Z")));
		connect( act, SIGNAL(triggered(void)), this, SLOT(exitTileMode(void)) );
		menu.addAction( act );
	}
	else
	{
		snprintf( stmp, sizeof(stmp), "&View Tile: %X%X", selTile.y(), selTile.x() );
		
		act = new QAction(tr(stmp), &menu);
		act->setShortcut( QKeySequence(tr("Z")));
		connect( act, SIGNAL(triggered(void)), this, SLOT(showTileMode(void)) );
		menu.addAction( act );
	}

	act = new QAction(tr("Draw Tile &Grid Lines"), &menu);
	act->setCheckable(true);
	act->setChecked(drawTileGrid);
	act->setShortcut( QKeySequence(tr("G")));
	connect( act, SIGNAL(triggered(void)), this, SLOT(toggleTileGridLines(void)) );
	menu.addAction( act );

	act = new QAction(tr("Next &Palette"), &menu);
	act->setShortcut( QKeySequence(tr("P")));
	connect( act, SIGNAL(triggered(void)), this, SLOT(cycleNextPalette(void)) );
	menu.addAction( act );

	subMenu = menu.addMenu(tr("Palette &Select"));
	group   = new QActionGroup(&menu);

	group->setExclusive(true);

	for (int i=0; i<9; i++)
	{
	   char stmp[8];

	   snprintf( stmp, sizeof(stmp), "&%i", i+1 );

	   paletteAct[i] = new QAction(tr(stmp), &menu);
	   paletteAct[i]->setCheckable(true);

	   group->addAction(paletteAct[i]);
	   subMenu->addAction(paletteAct[i]);
      
	   paletteAct[i]->setChecked( pindex[ patternIndex ] == i );
	}

	connect( paletteAct[0], SIGNAL(triggered(void)), this, SLOT(selPalette0(void)) );
	connect( paletteAct[1], SIGNAL(triggered(void)), this, SLOT(selPalette1(void)) );
	connect( paletteAct[2], SIGNAL(triggered(void)), this, SLOT(selPalette2(void)) );
	connect( paletteAct[3], SIGNAL(triggered(void)), this, SLOT(selPalette3(void)) );
	connect( paletteAct[4], SIGNAL(triggered(void)), this, SLOT(selPalette4(void)) );
	connect( paletteAct[5], SIGNAL(triggered(void)), this, SLOT(selPalette5(void)) );
	connect( paletteAct[6], SIGNAL(triggered(void)), this, SLOT(selPalette6(void)) );
	connect( paletteAct[7], SIGNAL(triggered(void)), this, SLOT(selPalette7(void)) );
	connect( paletteAct[8], SIGNAL(triggered(void)), this, SLOT(selPalette8(void)) );
	
	menu.exec(event->globalPos());
}

void ppuPatternView_t::toggleTileGridLines(void)
{
	drawTileGrid = !drawTileGrid;
	
	if ( patternIndex )
	{
	     g_config->setOption( "SDL.PPU_TileShowGrid1", drawTileGrid );
	}
	else
	{
	     g_config->setOption( "SDL.PPU_TileShowGrid0", drawTileGrid );
	}
}

void ppuPatternView_t::showTileMode(void)
{
   mode = 1;
}

void ppuPatternView_t::exitTileMode(void)
{
   mode = 0;
}

void ppuPatternView_t::openTileEditor(void)
{
	ppuTileEditor_t *tileEditor;

	tileEditor = new ppuTileEditor_t( patternIndex, this );

	tileEditor->setTile( &selTile );

	tileEditor->show();
}

void ppuPatternView_t::cycleNextPalette(void)
{
	pindex[ patternIndex ] = (pindex[ patternIndex ] + 1) % 9;

	PPUViewSkip = 100;

	FCEUD_UpdatePPUView( -1, 0 );
}

void ppuPatternView_t::selPalette0(void)
{
   pindex[ patternIndex ] = 0;
}

void ppuPatternView_t::selPalette1(void)
{
   pindex[ patternIndex ] = 1;
}

void ppuPatternView_t::selPalette2(void)
{
   pindex[ patternIndex ] = 2;
}

void ppuPatternView_t::selPalette3(void)
{
   pindex[ patternIndex ] = 3;
}

void ppuPatternView_t::selPalette4(void)
{
   pindex[ patternIndex ] = 4;
}

void ppuPatternView_t::selPalette5(void)
{
   pindex[ patternIndex ] = 5;
}

void ppuPatternView_t::selPalette6(void)
{
   pindex[ patternIndex ] = 6;
}

void ppuPatternView_t::selPalette7(void)
{
   pindex[ patternIndex ] = 7;
}

void ppuPatternView_t::selPalette8(void)
{
   pindex[ patternIndex ] = 8;
}

void ppuPatternView_t::updateCycleCounter(void)
{
	cycleCount = (cycleCount + 1) % 30;
}

void ppuPatternView_t::paintEvent(QPaintEvent *event)
{
	int i,j,x,y,w,h,xx,yy,ii,jj,rr;
	QPainter painter(this);
	QPen pen;
	char showSelector;

	viewWidth  = event->rect().width();
	viewHeight = event->rect().height();

	pen = painter.pen();

	pen.setWidth( 1 );
	pen.setColor( gridColor );
	painter.setPen( pen );

	w = viewWidth / 128;
	h = viewHeight / 128;

	pattern->w = w;
	pattern->h = h;

	xx = 0; yy = 0;

	showSelector = (cycleCount < 20) && (selTile.x() >= 0) && (selTile.y() >= 0);

	if ( mode == 1 )
	{
		w = viewWidth / 8;
		h = viewHeight / 8;
	
		if ( w < h )
		{
		   h = w;
		}
		else
		{
		   w = h;
		}
		
		ii = selTile.x();
		jj = selTile.y();
		
		for (x=0; x < 8; x++)
		{
			yy = 0;

			for (y=0; y < 8; y++)
			{
				painter.fillRect( xx, yy, w, h, pattern->tile[jj][ii].pixel[y][x].color );
				yy += h;
			}
			xx += w;
		}

		if ( drawTileGrid )
		{
			xx = 0; y = 8*h;
			
			for (x=0; x<9; x++)
			{
				painter.drawLine( xx, 0 , xx, y ); xx += w;
			}
			yy = 0; x = 8*w;
			
			for (y=0; y<9; y++)
			{
				painter.drawLine( 0, yy , x, yy ); yy += h;
			}
		}
	}
	else if ( PPUView_sprite16Mode[ patternIndex ] )
	{
		for (i=0; i<16; i++)
		{
			for (j=0; j<16; j++)
			{
				rr = (j%2);
				jj =  j;

				if ( rr )
				{
					jj--;
				}

				ii = (i*2)+rr;

				if ( ii >= 16 )
				{
					ii = ii % 16;
					jj++;
				}

				xx = (i*8)*w;
				yy = (j*8)*h;

				pattern->tile[jj][ii].x = xx;
				pattern->tile[jj][ii].y = yy;

				for (x=0; x < 8; x++)
				{
					yy = (j*8)*h;

					for (y=0; y < 8; y++)
					{
						painter.fillRect( xx, yy, w, h, pattern->tile[jj][ii].pixel[y][x].color );
						yy += h;
					}
					xx += w;
				}
			}
		}

		if ( drawTileGrid )
		{
			xx = 0; y = 128*h;

			for (i=0; i<16; i++)
			{
				painter.drawLine( xx, 0 , xx, y ); xx += (8*w);
			}

			yy = 0; x = 128*w;

			for (j=0; j<16; j++)
			{
				painter.drawLine( 0, yy , x, yy ); yy += (8*h);
			}
		}

		if ( showSelector )
		{
			xx = pattern->tile[ selTile.y() ][ selTile.x() ].x;
			yy = pattern->tile[ selTile.y() ][ selTile.x() ].y;

			pen.setWidth( 3 );
			pen.setColor( QColor(  0,  0,  0) );
			painter.setPen( pen );

			painter.drawRect( xx, yy, w*8, h*8 );

			pen.setWidth( 1 );
			pen.setColor( selTileColor );
			painter.setPen( pen );

			painter.drawRect( xx, yy, w*8, h*8 );
		}
	}
	else
	{
		for (i=0; i<16; i++)
		{
			for (j=0; j<16; j++)
			{
				xx = (i*8)*w;
				yy = (j*8)*h;

				pattern->tile[j][i].x = xx;
				pattern->tile[j][i].y = yy;

				for (x=0; x < 8; x++)
				{
					yy = (j*8)*h;

					for (y=0; y < 8; y++)
					{
						painter.fillRect( xx, yy, w, h, pattern->tile[j][i].pixel[y][x].color );
						yy += h;
					}
					xx += w;
				}
			}
		}

		if ( drawTileGrid )
		{
			xx = 0; y = 128*h;

			for (i=0; i<16; i++)
			{
				painter.drawLine( xx, 0 , xx, y ); xx += (8*w);
			}

			yy = 0; x = 128*w;

			for (j=0; j<16; j++)
			{
				painter.drawLine( 0, yy , x, yy ); yy += (8*h);
			}
		}

		if ( showSelector )
		{
			xx = pattern->tile[ selTile.y() ][ selTile.x() ].x;
			yy = pattern->tile[ selTile.y() ][ selTile.x() ].y;

			pen.setWidth( 3 );
			pen.setColor( QColor(  0,  0,  0) );
			painter.setPen( pen );

			painter.drawRect( xx, yy, w*8, h*8 );

			pen.setWidth( 1 );
			pen.setColor( selTileColor );
			painter.setPen( pen );

			painter.drawRect( xx, yy, w*8, h*8 );
		}
	}
}
