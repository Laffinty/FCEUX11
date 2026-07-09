/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2020 mjbudd77
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
//
// ppuViewer.cpp
//
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdint.h>
#include <string.h>

#include <QMenu>
#include <QAction>
#include <QMenuBar>
#include <QPainter>
#include <QSettings>
#include <QActionGroup>

#include "../../types.h"
#include "../../fceu.h"
#include "../../cart.h"
#include "../../ppu.h"
#include "../../debug.h"
#include "../../palette.h"

#include "Qt/ppuViewer.h"
#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/input.h"
#include "Qt/config.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/ColorMenu.h"
#include "Qt/ConfigStore.h"

static ppuViewerDialog_t *ppuViewWindow = NULL;
spriteViewerDialog_t *spriteViewWindow = NULL;

int openPPUViewWindow( QWidget *parent )
{
	if ( ppuViewWindow != NULL )
	{
		ppuViewWindow->activateWindow();
		ppuViewWindow->raise();
		return -1;
	}
	initPPUViewer();

	ppuViewWindow = new ppuViewerDialog_t(parent);

	ppuViewWindow->show();

	return 0;
}

int openOAMViewWindow( QWidget *parent )
{
	if ( spriteViewWindow != NULL )
	{
		spriteViewWindow->activateWindow();
		spriteViewWindow->raise();
		return -1;
	}
	initPPUViewer();

	spriteViewWindow = new spriteViewerDialog_t(parent);

	spriteViewWindow->show();

	return 0;
}

void setPPUSelPatternTile( int table, int x, int y )
{
	if ( ppuViewWindow == NULL )
	{
		return;
	}
	if ( table )
	{
		table = 1;
	}
	else
	{
		table = 0;
	}
	ppuViewWindow->patternView[ table ]->setTileCoord( x, y );
	ppuViewWindow->patternView[ table ]->updateSelTileLabel();
}

ppuViewerDialog_t::ppuViewerDialog_t(QWidget *parent)
	: QDialog( parent, Qt::Window )
{
	QSettings    settings;
	QMenuBar    *menuBar;
	QVBoxLayout *mainLayout, *vbox;
	QVBoxLayout *patternVbox[2];
	QHBoxLayout *hbox, *hbox1, *hbox2;
	QGridLayout *grid;
	QActionGroup *group;
	QMenu *fileMenu, *viewMenu, *colorMenu, *optMenu, *subMenu;
	QAction *act;
	int useNativeMenuBar;
	ColorMenuItem *tileSelColorAct[2], *tileGridColorAct[2];

	ppuViewWindow = this;

	menuBar = new QMenuBar(this);

	g_config->getOption( "SDL.UseNativeMenuBar", &useNativeMenuBar );

	menuBar->setNativeMenuBar( useNativeMenuBar ? true : false );

	setWindowTitle( tr("PPU Viewer") );

	mainLayout = new QVBoxLayout();

	mainLayout->setMenuBar( menuBar );

	setLayout( mainLayout );

	hbox              = new QHBoxLayout();
	grid              = new QGridLayout;
	patternVbox[0]    = new QVBoxLayout();
	patternVbox[1]    = new QVBoxLayout();
	patternFrame[0]   = new QGroupBox( this );
	patternFrame[0]->setTitle( tr("Pattern Table 0") );
	patternFrame[1]   = new QGroupBox( this );
	patternFrame[1]->setTitle( tr("Pattern Table 1") );
	patternView[0]    = new ppuPatternView_t( 0, this);
	patternView[1]    = new ppuPatternView_t( 1, this);
	sprite8x16Cbox[0] = new QCheckBox( this );
	sprite8x16Cbox[0]->setText( tr("Sprites 8x16 Mode") );
	sprite8x16Cbox[1] = new QCheckBox( this );
	sprite8x16Cbox[1]->setText( tr("Sprites 8x16 Mode") );
	tileLabel[0]      = new QLabel( this );
	tileLabel[0]->setText( tr("Tile:") );
	tileLabel[1]      = new QLabel( this );
	tileLabel[1]->setText( tr("Tile:") );

	g_config->getOption("SDL.PPU_View1_8x16", &PPUView_sprite16Mode[0]);
	g_config->getOption("SDL.PPU_View2_8x16", &PPUView_sprite16Mode[1]);

	sprite8x16Cbox[0]->setChecked( PPUView_sprite16Mode[0] );
	sprite8x16Cbox[1]->setChecked( PPUView_sprite16Mode[1] );

	patternVbox[0]->addWidget( patternView[0], 100 );
	patternVbox[0]->addWidget( tileLabel[0], 1 );
	patternVbox[0]->addWidget( sprite8x16Cbox[0], 1 );
	patternVbox[1]->addWidget( patternView[1], 100 );
	patternVbox[1]->addWidget( tileLabel[1], 1 );
	patternVbox[1]->addWidget( sprite8x16Cbox[1], 1 );

	patternFrame[0]->setLayout( patternVbox[0] );
	patternFrame[1]->setLayout( patternVbox[1] );

	hbox->addWidget( patternFrame[0] );
	hbox->addWidget( patternFrame[1] );

	mainLayout->addLayout( hbox, 10 );
	mainLayout->addLayout( grid,  1 );

	maskUnusedCbox = new QCheckBox( this );
	maskUnusedCbox->setText( tr("Mask unused Graphics (Code/Data Logger)") );
	invertMaskCbox = new QCheckBox( this );
	invertMaskCbox->setText( tr("Invert the Mask (Code/Data Logger)") );

	g_config->getOption("SDL.PPU_MaskUnused", &PPUView_maskUnusedGraphics);
	g_config->getOption("SDL.PPU_InvertMask", &PPUView_invertTheMask);

	maskUnusedCbox->setChecked( PPUView_maskUnusedGraphics );
	invertMaskCbox->setChecked( PPUView_invertTheMask );

	connect( maskUnusedCbox   , SIGNAL(stateChanged(int)), this, SLOT(maskUnusedGraphicsChanged(int)));
	connect( invertMaskCbox   , SIGNAL(stateChanged(int)), this, SLOT(invertMaskChanged(int)));
	connect( sprite8x16Cbox[0], SIGNAL(stateChanged(int)), this, SLOT(sprite8x16Changed0(int)));
	connect( sprite8x16Cbox[1], SIGNAL(stateChanged(int)), this, SLOT(sprite8x16Changed1(int)));

	hbox           = new QHBoxLayout();
	refreshSlider  = new QSlider( Qt::Horizontal );
	refreshMoreLbl = new QLabel( this );
	refreshMoreLbl->setText( tr("Refresh: More") );
	refreshLessLbl = new QLabel( this );
	refreshLessLbl->setText( tr("Less") );
	hbox->addWidget( refreshMoreLbl );
	hbox->addWidget( refreshSlider );
	hbox->addWidget( refreshLessLbl );

	grid->addWidget( maskUnusedCbox, 0, 0, Qt::AlignLeft );
	grid->addWidget( invertMaskCbox, 1, 0, Qt::AlignLeft );
	grid->addLayout( hbox, 0, 1, Qt::AlignRight );

	hbox         = new QHBoxLayout();
	scanLineEdit = new QSpinBox();
	scanLineLbl  = new QLabel( this );
	scanLineLbl->setText( tr("Display on Scanline:") );
	hbox->addWidget( scanLineLbl );
	hbox->addWidget( scanLineEdit );
	grid->addLayout( hbox, 1, 1, Qt::AlignRight );

	vbox         = new QVBoxLayout();
	paletteFrame = new QGroupBox( this );
	paletteFrame->setTitle( tr("Palettes:") );

	hbox1        = new QHBoxLayout();
	hbox2        = new QHBoxLayout();

	for (int i=0; i<8; i++)
	{
		tilePalView[i] = new tilePaletteView_t(this);

		if ( i < 4 )
		{
			hbox1->addWidget( tilePalView[i] );
		}
		else
		{
			hbox2->addWidget( tilePalView[i] );
		}
		tilePalView[i]->setIndex(i);
	}

	vbox->addLayout( hbox1, 1 );
	vbox->addLayout( hbox2, 1 );
	paletteFrame->setLayout( vbox );

	mainLayout->addWidget( paletteFrame,  1 );

	patternView[0]->setPattern( &pattern0 );
	patternView[1]->setPattern( &pattern1 );
	patternView[0]->setTileLabel( tileLabel[0] );
	patternView[1]->setTileLabel( tileLabel[1] );

	g_config->getOption("SDL.PPU_ViewScanLine", &PPUViewScanline);

	scanLineEdit->setRange( 0, 255 );
	scanLineEdit->setValue( PPUViewScanline );

	connect( scanLineEdit, SIGNAL(valueChanged(int)), this, SLOT(scanLineChanged(int)));

	g_config->getOption("SDL.PPU_ViewRefreshFrames", &PPUViewRefresh);

	refreshSlider->setMinimum( 0);
	refreshSlider->setMaximum(25);
	refreshSlider->setValue(PPUViewRefresh);

	connect( refreshSlider, SIGNAL(valueChanged(int)), this, SLOT(refreshSliderChanged(int)));

	cycleCount  = 0;
	PPUViewSkip = 100;
	
	FCEUD_UpdatePPUView( -1, 1 );

	fileMenu = menuBar->addMenu(tr("&File"));

	act = new QAction(tr("&Close"), this);
	act->setShortcut(QKeySequence::Close);
	act->setStatusTip(tr("Close Window"));
	connect(act, SIGNAL(triggered()), this, SLOT(closeWindow(void)) );
	
	fileMenu->addAction(act);

	viewMenu = menuBar->addMenu(tr("View&1"));

	act = new QAction(tr("Toggle &Grid"), this);
	act->setStatusTip(tr("Toggle Grid"));
	connect( act, SIGNAL(triggered()), patternView[0], SLOT(toggleTileGridLines()) );
	
	viewMenu->addAction(act);

	colorMenu = viewMenu->addMenu(tr("&Colors"));

	tileSelColorAct[0] = new ColorMenuItem(tr("Tile &Selector"), "SDL.PPU_TileSelColor0", this);
	tileSelColorAct[0]->connectColor( &patternView[0]->selTileColor );
	
	colorMenu->addAction(tileSelColorAct[0]);

	tileGridColorAct[0] = new ColorMenuItem(tr("Tile &Grid"), "SDL.PPU_TileGridColor0", this);
	tileGridColorAct[0]->connectColor( &patternView[0]->gridColor );
	
	colorMenu->addAction(tileGridColorAct[0]);

	viewMenu = menuBar->addMenu(tr("View&2"));

	act = new QAction(tr("Toggle &Grid"), this);
	act->setStatusTip(tr("Toggle Grid"));
	connect( act, SIGNAL(triggered()), patternView[1], SLOT(toggleTileGridLines()) );
	
	viewMenu->addAction(act);

	colorMenu = viewMenu->addMenu(tr("&Colors"));

	tileSelColorAct[1] = new ColorMenuItem(tr("Tile &Selector"), "SDL.PPU_TileSelColor1", this);
	tileSelColorAct[1]->connectColor( &patternView[1]->selTileColor );
	
	colorMenu->addAction(tileSelColorAct[1]);

	tileGridColorAct[1] = new ColorMenuItem(tr("Tile &Grid"), "SDL.PPU_TileGridColor1", this);
	tileGridColorAct[1]->connectColor( &patternView[1]->gridColor );
	
	colorMenu->addAction(tileGridColorAct[1]);

	optMenu = menuBar->addMenu(tr("&Options"));

	subMenu = optMenu->addMenu(tr("&Focus Policy"));
	group   = new QActionGroup(this);
	group->setExclusive(true);

	act = new QAction(tr("&Click"), this);
	act->setCheckable(true);
	act->setChecked( !patternView[0]->getHoverFocus() );
	group->addAction(act);
	subMenu->addAction(act);
	connect(act, SIGNAL(triggered()), this, SLOT(setClickFocus(void)) );

	act = new QAction(tr("&Hover"), this);
	act->setCheckable(true);
	act->setChecked( patternView[0]->getHoverFocus() );
	group->addAction(act);
	subMenu->addAction(act);
	connect(act, SIGNAL(triggered()), this, SLOT(setHoverFocus(void)) );

	updateTimer  = new QTimer( this );

	connect( updateTimer, &QTimer::timeout, this, &ppuViewerDialog_t::periodicUpdate );

	updateTimer->start( 33 ); // 30hz

	{
		static const fceu11::qt::TypedConfig<QByteArray> kGeometry(
			"ppuViewer/geometry", QByteArray());
		restoreGeometry(kGeometry.get());
	}

	connect( this, SIGNAL(rejected(void)), this, SLOT(deleteLater(void)));
}

ppuViewerDialog_t::~ppuViewerDialog_t(void)
{
	updateTimer->stop();
	ppuViewWindow = NULL;

	static const fceu11::qt::TypedConfig<QByteArray> kGeometry(
		"ppuViewer/geometry", QByteArray());
	kGeometry.set(saveGeometry());
}

void ppuViewerDialog_t::closeEvent(QCloseEvent *event)
{
	static const fceu11::qt::TypedConfig<QByteArray> kGeometry(
		"ppuViewer/geometry", QByteArray());
	kGeometry.set(saveGeometry());
	done(0);
	deleteLater();
	event->accept();
}

void ppuViewerDialog_t::retranslateUi(void)
{
	setWindowTitle( tr("PPU Viewer") );

	if (patternFrame[0])   patternFrame[0]->setTitle( tr("Pattern Table 0") );
	if (patternFrame[1])   patternFrame[1]->setTitle( tr("Pattern Table 1") );
	if (sprite8x16Cbox[0]) sprite8x16Cbox[0]->setText( tr("Sprites 8x16 Mode") );
	if (sprite8x16Cbox[1]) sprite8x16Cbox[1]->setText( tr("Sprites 8x16 Mode") );
	if (tileLabel[0])      tileLabel[0]->setText( tr("Tile:") );
	if (tileLabel[1])      tileLabel[1]->setText( tr("Tile:") );
	if (maskUnusedCbox)    maskUnusedCbox->setText( tr("Mask unused Graphics (Code/Data Logger)") );
	if (invertMaskCbox)    invertMaskCbox->setText( tr("Invert the Mask (Code/Data Logger)") );
	if (refreshMoreLbl)    refreshMoreLbl->setText( tr("Refresh: More") );
	if (refreshLessLbl)    refreshLessLbl->setText( tr("Less") );
	if (scanLineLbl)       scanLineLbl->setText( tr("Display on Scanline:") );
	if (paletteFrame)      paletteFrame->setTitle( tr("Palettes:") );
}

void ppuViewerDialog_t::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QDialog::changeEvent(event);
}

void ppuViewerDialog_t::closeWindow(void)
{
	QSettings settings;
	settings.setValue("ppuViewer/geometry", saveGeometry());
	done(0);
	deleteLater();
}

void ppuViewerDialog_t::periodicUpdate(void)
{
	cycleCount = (cycleCount + 1) % 4;

	if ( redrawWindow || (cycleCount == 0) )
	{
		this->update();
		redrawWindow = false;
	}
	patternView[0]->updateCycleCounter();
	patternView[1]->updateCycleCounter();

	if ( scanLineEdit->value() != PPUViewScanline )
	{
		scanLineEdit->setValue( PPUViewScanline );
	}
}

void ppuViewerDialog_t::scanLineChanged(int value)
{
	PPUViewScanline = value;
	g_config->setOption("SDL.PPU_ViewScanLine", PPUViewScanline);
}

void ppuViewerDialog_t::invertMaskChanged(int state)
{
	PPUView_invertTheMask = (state == Qt::Unchecked) ? 0 : 1;
	g_config->setOption("SDL.PPU_InvertMask", PPUView_invertTheMask);
}

void ppuViewerDialog_t::maskUnusedGraphicsChanged(int state)
{
	PPUView_maskUnusedGraphics = (state == Qt::Unchecked) ? 0 : 1;
	g_config->setOption("SDL.PPU_MaskUnused", PPUView_maskUnusedGraphics);
}

void ppuViewerDialog_t::sprite8x16Changed0(int state)
{
	PPUView_sprite16Mode[0] = (state == Qt::Unchecked) ? 0 : 1;
	g_config->setOption("SDL.PPU_View1_8x16", PPUView_sprite16Mode[0]);
}

void ppuViewerDialog_t::sprite8x16Changed1(int state)
{
	PPUView_sprite16Mode[1] = (state == Qt::Unchecked) ? 0 : 1;
	g_config->setOption("SDL.PPU_View2_8x16", PPUView_sprite16Mode[1]);
}

void ppuViewerDialog_t::refreshSliderChanged(int value)
{
	PPUViewRefresh = value;
	g_config->setOption("SDL.PPU_ViewRefreshFrames", PPUViewRefresh);
}

void ppuViewerDialog_t::setClickFocus(void)
{
	patternView[0]->setHoverFocus(false);
	patternView[1]->setHoverFocus(false);
}

void ppuViewerDialog_t::setHoverFocus(void)
{
	patternView[0]->setHoverFocus(true);
	patternView[1]->setHoverFocus(true);
}

void fceWrapper_UpdatePPUView(int scanline, int refreshchr)
{
	if ( (ppuViewWindow == NULL) && (spriteViewWindow == NULL) )
	{
		return;
	}
	if ( (scanline != -1) && (scanline != PPUViewScanline) )
	{
		return;
	}
	int x,i;

	if (refreshchr)
	{
		int i10, x10;
		for (i = 0, x=0x1000; i < 0x1000; i++, x++)
		{
			i10 = i>>10;
			x10 = x>>10;

			if ( VPage[i10] == NULL )
			{
				continue;
			}
			chrcache0[i] = VPage[i10][i];
			chrcache1[i] = VPage[x10][x];

			if (debug_loggingCD) 
			{
				if (cdloggerVideoDataSize)
				{
					int addr;
					addr = &VPage[i10][i] - CHRptr[0];
					if ((addr >= 0) && (addr < (int)cdloggerVideoDataSize))
						logcache0[i] = cdloggervdata[addr];
					addr = &VPage[x10][x] - CHRptr[0];
					if ((addr >= 0) && (addr < (int)cdloggerVideoDataSize))
						logcache1[i] = cdloggervdata[addr];
				}
				else
				{
					logcache0[i] = cdloggervdata[i];
					logcache1[i] = cdloggervdata[x];
				}
			}
		}
	}

	if (PPUViewSkip < PPUViewRefresh) 
	{
		PPUViewSkip++;
		return;
	}
	PPUViewSkip = 0;
	
	if ( (palo != NULL) && ( (memcmp(pallast, PALRAM.data(), 32) != 0) || (memcmp(pallast+32, UPALRAM.data(), 3) != 0) ))
	{
		memcpy(pallast, PALRAM.data(), 32);
		memcpy(pallast+32, UPALRAM.data(), 3);

		memcpy(palcache,PALRAM.data(),32);
		palcache[0x10] = palcache[0x00];
		palcache[0x04] = palcache[0x14] = UPALRAM[0];
		palcache[0x08] = palcache[0x18] = UPALRAM[1];
		palcache[0x0C] = palcache[0x1C] = UPALRAM[2];
	}

	DrawPatternTable( &pattern0,chrcache0,logcache0,pindex[0]);
	DrawPatternTable( &pattern1,chrcache1,logcache1,pindex[1]);

	if ( spriteViewWindow != NULL )
	{
		memcpy( oam, SPRAM, 256 );

		drawSpriteTable();
	}
	redrawWindow = true;
}
