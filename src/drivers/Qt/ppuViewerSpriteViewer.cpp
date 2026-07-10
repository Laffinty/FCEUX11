// ppuViewerSpriteViewer.cpp
//
#include <stdio.h>
#include <stdint.h>

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QGridLayout>
#include <QLabel>
#include <QGroupBox>
#include <QMenuBar>
#include <QMenu>
#include <QAction>
#include <QActionGroup>
#include <QSettings>
#include <QFont>
#include <QFontMetrics>

#include "../../fceu.h"
#include "../../palette.h"

#include "Qt/main.h"
#include "Qt/config.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/ColorMenu.h"
#include "Qt/ppuViewer.h"
#include "Qt/ppuViewerPalette.h"
#include "Qt/ppuViewerSpriteViewer.h"

spriteViewerDialog_t::spriteViewerDialog_t(QWidget *parent)
	: QDialog(parent, Qt::Window)
{
	QSettings    settings;
	QMenuBar    *menuBar;
	QVBoxLayout *mainLayout, *vbox, *vbox1, *vbox2, *vbox3;
	QHBoxLayout *hbox, *hbox1, *hbox2;
	QGridLayout *grid;
	QGroupBox   *frame;
	QLabel      *lbl;
	QActionGroup *group;
	QMenu *fileMenu, *viewMenu, *colorMenu, *optMenu, *subMenu;
	QAction *act;
	QFont font;
	int useNativeMenuBar, pxCharWidth, opt;
	ColorMenuItem *selTileColorAct, *gridColorAct, *locColorAct;

	extern spriteViewerDialog_t *spriteViewWindow;
	spriteViewWindow = this;

	oamView  = new oamPatternView_t(this);
	tileView = new oamTileView_t(this);
	palView  = new oamPaletteView_t(this);
	preView  = new oamPreview_t(this);

	menuBar = new QMenuBar(this);

	g_config->getOption("SDL.UseNativeMenuBar", &useNativeMenuBar);

	menuBar->setNativeMenuBar(useNativeMenuBar ? true : false);

	fileMenu = menuBar->addMenu(tr("&File"));

	act = new QAction(tr("&Close"), this);
	act->setShortcut(QKeySequence::Close);
	act->setStatusTip(tr("Close Window"));
	connect(act, SIGNAL(triggered()), this, SLOT(closeWindow(void)));

	fileMenu->addAction(act);

	viewMenu = menuBar->addMenu(tr("&View"));

	act = new QAction(tr("Toggle &Grid"), this);
	act->setStatusTip(tr("Toggle Grid"));
	connect(act, SIGNAL(triggered()), this, SLOT(toggleGridVis(void)));

	viewMenu->addAction(act);

	colorMenu = menuBar->addMenu(tr("&Color"));

	selTileColorAct = new ColorMenuItem(tr("&Selector"), "SDL.OAM_TileSelColor", this);
	selTileColorAct->connectColor(&oamView->selTileColor);

	colorMenu->addAction(selTileColorAct);

	gridColorAct = new ColorMenuItem(tr("&Grid"), "SDL.OAM_TileGridColor", this);
	gridColorAct->connectColor(&oamView->gridColor);

	colorMenu->addAction(gridColorAct);

	locColorAct = new ColorMenuItem(tr("&Locator Box"), "SDL.OAM_LocatorColor", this);
	locColorAct->connectColor(&preView->boxColor);

	colorMenu->addAction(locColorAct);

	optMenu = menuBar->addMenu(tr("&Options"));

	subMenu = optMenu->addMenu(tr("&Focus Policy"));
	group   = new QActionGroup(this);
	group->setExclusive(true);

	act = new QAction(tr("&Click"), this);
	act->setCheckable(true);
	act->setChecked(!oamView->getHoverFocus());
	group->addAction(act);
	subMenu->addAction(act);
	connect(act, SIGNAL(triggered()), this, SLOT(setClickFocus(void)));

	act = new QAction(tr("&Hover"), this);
	act->setCheckable(true);
	act->setChecked(oamView->getHoverFocus());
	group->addAction(act);
	subMenu->addAction(act);
	connect(act, SIGNAL(triggered()), this, SLOT(setHoverFocus(void)));

	font.setFamily("Courier New");
	font.setStyle(QFont::StyleNormal);
	font.setStyleHint(QFont::Monospace);

	QFontMetrics metrics(font);
#if QT_VERSION > QT_VERSION_CHECK(5, 11, 0)
	pxCharWidth = metrics.horizontalAdvance(QLatin1Char('2'));
#else
	pxCharWidth = metrics.width(QLatin1Char('2'));
#endif

	setWindowTitle(tr("Sprite Viewer"));

	mainLayout = new QVBoxLayout();

	mainLayout->setMenuBar(menuBar);

	setLayout(mainLayout);

	useSprRam = new QRadioButton(tr("Sprite RAM"));
	useCpuPag = new QRadioButton(tr("CPU Page #"));
	cpuPagIdx = new QSpinBox(this);

	scanLineEdit = new QSpinBox(this);
	scanLineEdit->setRange(0, 255);
	scanLineEdit->setValue(PPUViewScanline);

	connect(scanLineEdit, SIGNAL(valueChanged(int)), this, SLOT(scanLineChanged(int)));

	useSprRam->setChecked(true);
	useSprRam->setEnabled(false);
	cpuPagIdx->setEnabled(false);
	useCpuPag->setEnabled(false);

	hFlipBox = new QCheckBoxRO(tr("Horizontal Flip"));
	hFlipBox->setFocusPolicy(Qt::NoFocus);

	vFlipBox = new QCheckBoxRO(tr("Vertical Flip"));
	vFlipBox->setFocusPolicy(Qt::NoFocus);

	bgPrioBox = new QCheckBoxRO(tr("Background Priority"));
	bgPrioBox->setFocusPolicy(Qt::NoFocus);

	spriteIndexBox = new QLineEdit();
	spriteIndexBox->setFont(font);
	spriteIndexBox->setReadOnly(true);
	spriteIndexBox->setMinimumWidth(4 * pxCharWidth);

	tileAddrBox = new QLineEdit();
	tileAddrBox->setFont(font);
	tileAddrBox->setReadOnly(true);
	tileAddrBox->setMinimumWidth(6 * pxCharWidth);

	tileIndexBox = new QLineEdit();
	tileIndexBox->setFont(font);
	tileIndexBox->setReadOnly(true);
	tileIndexBox->setMinimumWidth(4 * pxCharWidth);

	palAddrBox = new QLineEdit();
	palAddrBox->setFont(font);
	palAddrBox->setReadOnly(true);
	palAddrBox->setMinimumWidth(6 * pxCharWidth);

	posBox = new QLineEdit();
	posBox->setFont(font);
	posBox->setReadOnly(true);
	posBox->setMinimumWidth(10 * pxCharWidth);

	showPosHex = new QCheckBox(tr("Show Position in Hex"));

	hbox1 = new QHBoxLayout();
	vbox3 = new QVBoxLayout();
	hbox  = new QHBoxLayout();
	hbox1->addLayout(vbox3);
	vbox3->addWidget(oamView);
	vbox3->addLayout(hbox);
	hbox->addWidget(new QLabel(tr("Display on Scanline:")), 1);
	hbox->addWidget(scanLineEdit, 1);
	hbox->addStretch(5);

	mainLayout->addLayout(hbox1);

	vbox1 = new QVBoxLayout();
	hbox1->addLayout(vbox1);

	vbox2 = new QVBoxLayout();

	hbox  = new QHBoxLayout();
	vbox1->addLayout(hbox, 1);

	hbox->addWidget(new QLabel(tr("Data Source:")));
	hbox->addWidget(useSprRam);
	hbox->addWidget(useCpuPag);
	hbox->addWidget(cpuPagIdx);

	frame    = new QGroupBox(tr("Sprite Info"));
	grid     = new QGridLayout();
	vbox1->addWidget(frame, 1);
	frame->setLayout(vbox2);

	hbox2    = new QHBoxLayout();
	frame    = new QGroupBox(tr("Tile:"));
	hbox     = new QHBoxLayout();
	frame->setLayout(hbox);
	hbox->addWidget(tileView);
	vbox2->addLayout(hbox2);
	hbox2->addWidget(frame);
	hbox2->addLayout(grid);

	vbox     = new QVBoxLayout();
	hbox->addLayout(vbox);

	vbox->addWidget(hFlipBox);

	vbox->addWidget(vFlipBox);

	vbox->addWidget(bgPrioBox);

	frame    = new QGroupBox(tr("Palette:"));
	hbox     = new QHBoxLayout();
	hbox->addWidget(palView);
	frame->setLayout(hbox);
	vbox->addWidget(frame);

	frame    = new QGroupBox(tr("Preview:"));
	vbox     = new QVBoxLayout();
	vbox->addWidget(preView);
	frame->setLayout(vbox);
	vbox1->addWidget(frame, 10);


	lbl      = new QLabel(tr("Sprite Index:"));
	grid->addWidget(lbl, 0, 0);
	grid->addWidget(spriteIndexBox, 0, 1);

	lbl      = new QLabel(tr("Tile Address:"));
	grid->addWidget(lbl, 1, 0);
	grid->addWidget(tileAddrBox, 1, 1);

	lbl      = new QLabel(tr("Tile Index:"));
	grid->addWidget(lbl, 2, 0);
	grid->addWidget(tileIndexBox, 2, 1);

	lbl      = new QLabel(tr("Palette Address:"));
	grid->addWidget(lbl, 3, 0);
	grid->addWidget(palAddrBox, 3, 1);

	lbl      = new QLabel(tr("Position (X,Y):"));
	grid->addWidget(lbl, 4, 0);
	grid->addWidget(posBox, 4, 1);

	grid->addWidget(showPosHex, 5, 0, 1, 2);

	updateTimer  = new QTimer(this);

	connect(updateTimer, &QTimer::timeout, this, &spriteViewerDialog_t::periodicUpdate);

	updateTimer->start(33);

	resize(minimumSizeHint());

	restoreGeometry(settings.value("spriteViewer/geometry").toByteArray());

	connect(this, SIGNAL(rejected(void)), this, SLOT(deleteLater(void)));

	g_config->getOption("SDL.OAM_ShowPosHex", &opt);
	showPosHex->setChecked(opt);
}

spriteViewerDialog_t::~spriteViewerDialog_t(void)
{
	extern spriteViewerDialog_t *spriteViewWindow;
	if (this == spriteViewWindow)
	{
		spriteViewWindow = NULL;
	}
	g_config->setOption("SDL.OAM_ShowPosHex", showPosHex->isChecked());
}

void spriteViewerDialog_t::closeEvent(QCloseEvent *event)
{
	QSettings settings;
	settings.setValue("spriteViewer/geometry", saveGeometry());
	done(0);
	deleteLater();
	event->accept();
}

void spriteViewerDialog_t::closeWindow(void)
{
	QSettings settings;
	settings.setValue("spriteViewer/geometry", saveGeometry());
	done(0);
	deleteLater();
}

void spriteViewerDialog_t::setClickFocus(void)
{
	oamView->setHover2Focus(false);
}

void spriteViewerDialog_t::setHoverFocus(void)
{
	oamView->setHover2Focus(true);
}

void spriteViewerDialog_t::toggleGridVis(void)
{
	oamView->setGridVisibility(!oamView->getGridVisibility());
}

void spriteViewerDialog_t::scanLineChanged(int value)
{
	PPUViewScanline = value;
	g_config->setOption("SDL.PPU_ViewScanLine", PPUViewScanline);
}

void spriteViewerDialog_t::periodicUpdate(void)
{
	int idx;
	char stmp[32];

	idx = oamView->getSpriteIndex();

	snprintf(stmp, sizeof(stmp), "$%02X", idx);
	spriteIndexBox->setText(tr(stmp));

	snprintf(stmp, sizeof(stmp), "$%02X", oamPattern.sprite[idx].tNum);
	tileIndexBox->setText(tr(stmp));

	snprintf(stmp, sizeof(stmp), "$%04X", oamPattern.sprite[idx].chrAddr);
	tileAddrBox->setText(tr(stmp));

	snprintf(stmp, sizeof(stmp), "$%04X", 0x3F00 + (oamPattern.sprite[idx].pal * 4));
	palAddrBox->setText(tr(stmp));

	if (showPosHex->isChecked())
	{
		snprintf(stmp, sizeof(stmp), "$%02X, $%02X", oamPattern.sprite[idx].x, oamPattern.sprite[idx].y);
	}
	else
	{
		snprintf(stmp, sizeof(stmp), "%3i, %3i", oamPattern.sprite[idx].x, oamPattern.sprite[idx].y);
	}
	posBox->setText(tr(stmp));

	if (scanLineEdit->value() != PPUViewScanline)
	{
		scanLineEdit->setValue(PPUViewScanline);
	}

	hFlipBox->setChecked(oamPattern.sprite[idx].hFlip);
	vFlipBox->setChecked(oamPattern.sprite[idx].vFlip);
	bgPrioBox->setChecked(oamPattern.sprite[idx].pri);

	tileView->setIndex(idx);
	palView->setIndex(idx);
	preView->setIndex(idx);

	oamView->update();
	tileView->update();
	palView->update();
	preView->update();
}

oamPatternView_t::oamPatternView_t(QWidget *parent)
	: QWidget(parent)
{
	this->setFocusPolicy(Qt::StrongFocus);
	this->setMouseTracking(true);
	setMinimumWidth(256);
	setMinimumHeight(512);
	viewWidth = 256;
	viewHeight = 512;
	hover2Focus = false;
	showGrid = false;

	selTileColor.setRgb(255, 255, 255);
	gridColor.setRgb(128, 128, 128);

	selSprite.setX(0);
	selSprite.setY(0);
	spriteIdx = 0;

	fceuLoadConfigColor("SDL.OAM_TileSelColor", &selTileColor);
	fceuLoadConfigColor("SDL.OAM_TileGridColor", &gridColor);

	g_config->getOption("SDL.OAM_TileShowGrid", &showGrid);
	g_config->getOption("SDL.OAM_TileFocusPolicy", &hover2Focus);
}

oamPatternView_t::~oamPatternView_t(void)
{

}

void oamPatternView_t::setHover2Focus(bool val)
{
	hover2Focus = val;

	g_config->setOption("SDL.OAM_TileFocusPolicy", hover2Focus);
}

void oamPatternView_t::setGridVisibility(bool val)
{
	showGrid = val;
	g_config->setOption("SDL.OAM_TileShowGrid", showGrid);
}

int oamPatternView_t::getSpriteIndex(void){ return spriteIdx; }

int oamPatternView_t::heightForWidth(int w) const
{
	return 2 * w;
}

QSize oamPatternView_t::minimumSizeHint(void) const
{
	return QSize(256, 512);
}

QSize oamPatternView_t::maximumSizeHint(void) const
{
	return QSize(512, 1024);
}

QSize oamPatternView_t::sizeHint(void) const
{
	return QSize(384, 768);
}

void oamPatternView_t::openTilePpuViewer(void)
{
	int pTable, x, y, tileAddr;

	tileAddr = oamPattern.sprite[spriteIdx].chrAddr;

	pTable = tileAddr >= 0x1000;
	y = (tileAddr & 0x0F00) >> 8;
	x = (tileAddr & 0x00F0) >> 4;

	openPPUViewWindow(consoleWindow);

	setPPUSelPatternTile(pTable, x, y);
	setPPUSelPatternTile(!pTable, -1, -1);
}

void oamPatternView_t::resizeEvent(QResizeEvent *event)
{
	viewWidth = event->size().width();
	viewHeight = event->size().height();
}

QPoint oamPatternView_t::convPixToTile(QPoint p)
{
	QPoint t(0, 0);
	int x, y, w, h, i, j;

	x = p.x(); y = p.y();

	w = oamPattern.w;
	h = oamPattern.h;

	i = w == 0 ? 0 : x / (w * 8);
	j = h == 0 ? 0 : y / (h * 16);

	t.setX(i);
	t.setY(j);

	return t;
}

void oamPatternView_t::keyPressEvent(QKeyEvent *event)
{
	int x, y;

	if (event->key() == Qt::Key_Up)
	{
		y = selSprite.y();

		y = (y - 1);

		if (y < 0)
		{
			y = 7;
		}

		selSprite.setY(y);
		spriteIdx = selSprite.y() * 8 + selSprite.x();
	}
	else if (event->key() == Qt::Key_Down)
	{
		y = selSprite.y();

		y = (y + 1);

		if (y > 7)
		{
			y = 0;
		}

		selSprite.setY(y);
		spriteIdx = selSprite.y() * 8 + selSprite.x();
	}
	else if (event->key() == Qt::Key_Left)
	{
		x = selSprite.x();

		x = (x - 1);

		if (x < 0)
		{
			x = 7;
		}

		selSprite.setX(x);
		spriteIdx = selSprite.y() * 8 + selSprite.x();
	}
	else if (event->key() == Qt::Key_Right)
	{
		x = selSprite.x();

		x = (x + 1);

		if (x > 7)
		{
			x = 0;
		}

		selSprite.setX(x);
		spriteIdx = selSprite.y() * 8 + selSprite.x();
	}

}

void oamPatternView_t::mouseMoveEvent(QMouseEvent *event)
{
	QPoint tile = convPixToTile(event->pos());

	if (hover2Focus)
	{
		if ((tile.x() >= 0) && (tile.x() < 8) &&
			(tile.y() >= 0) && (tile.y() < 8))
		{
			selSprite = tile;
			spriteIdx = tile.y() * 8 + tile.x();
		}
	}


}

void oamPatternView_t::mousePressEvent(QMouseEvent *event)
{
	QPoint tile = convPixToTile(event->pos());

	if (event->button() == Qt::LeftButton)
	{
		if ((tile.x() >= 0) && (tile.x() < 8) &&
			(tile.y() >= 0) && (tile.y() < 8))
		{
			selSprite = tile;
			spriteIdx = tile.y() * 8 + tile.x();
		}
	}
}

void oamPatternView_t::contextMenuEvent(QContextMenuEvent *event)
{
	QAction *act;
	QMenu menu(this);

	act = new QAction(tr("Open PPU CHR &Viewer"), &menu);
	connect(act, SIGNAL(triggered(void)), this, SLOT(openTilePpuViewer(void)));
	menu.addAction(act);

	menu.exec(event->globalPos());
}

void oamPatternView_t::paintEvent(QPaintEvent *event)
{
	int i, j, x, y, w, h, xx, yy, ii, jj;
	QPainter painter(this);
	QPen pen;

	pen = painter.pen();

	viewWidth = event->rect().width();
	viewHeight = event->rect().height();

	w = viewWidth / 64;
	h = viewHeight / 128;

	if (w < h)
	{
		h = w;
	}
	else
	{
		w = h;
	}

	oamPattern.w = w;
	oamPattern.h = h;

	for (i = 0; i < 64; i++)
	{
		ii = (i % 8) * (w * 8);
		jj = (i / 8) * (h * 16);

		for (j = 0; j < 2; j++)
		{
			xx = ii;

			for (x = 0; x < 8; x++)
			{
				yy = jj + (j * h * 8);

				for (y = 0; y < 8; y++)
				{
					painter.fillRect(xx, yy, w, h, oamPattern.sprite[i].tile[j].pixel[y][x].color);
					yy += h;
				}
				xx += w;
			}
		}
	}

	if (showGrid)
	{
		int tw, th;
		pen.setWidth(1);
		pen.setColor(gridColor);
		painter.setPen(pen);

		tw = 8 * w;
		th = 16 * h;

		xx = 0;
		y = 8 * th;

		for (x = 0; x <= 8; x++)
		{
			painter.drawLine(xx, 0, xx, y); xx += tw;
		}

		yy = 0;
		x = 8 * tw;

		for (y = 0; y <= 8; y++)
		{
			painter.drawLine(0, yy, x, yy); yy += th;
		}
	}

	if ((spriteIdx >= 0) && (spriteIdx < 64))
	{
		xx = (spriteIdx % 8) * (w * 8);
		yy = (spriteIdx / 8) * (h * 16);

		pen.setWidth(3);
		pen.setColor(QColor(0, 0, 0));
		painter.setPen(pen);

		painter.drawRect(xx, yy, w * 8, h * 16);

		pen.setWidth(1);
		pen.setColor(selTileColor);
		painter.setPen(pen);

		painter.drawRect(xx, yy, w * 8, h * 16);
	}
}

oamTileView_t::oamTileView_t(QWidget *parent)
	: QWidget(parent)
{
	setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
	viewWidth = 80;
	viewHeight = 160;
	setMinimumWidth(viewWidth);
	setMinimumHeight(viewHeight);

	spriteIdx = 0;
}

oamTileView_t::~oamTileView_t(void)
{

}

void oamTileView_t::setIndex(int val)
{
	spriteIdx = val;
}

int oamTileView_t::heightForWidth(int w) const
{
	return 2 * w;
}

QSize oamTileView_t::minimumSizeHint(void) const
{
	return QSize(8, 16);
}

QSize oamTileView_t::maximumSizeHint(void) const
{
	return QSize(128, 256);
}

QSize oamTileView_t::sizeHint(void) const
{
	return QSize(64, 128);
}

void oamTileView_t::resizeEvent(QResizeEvent *event)
{
	viewWidth = event->size().width();
	viewHeight = event->size().height();
}

void oamTileView_t::paintEvent(QPaintEvent *event)
{
	int j, x, y, w, h, xx, yy;
	QPainter painter(this);

	viewWidth = event->rect().width();
	viewHeight = event->rect().height();

	w = viewWidth / 8;
	h = viewHeight / 16;

	if (w < h)
	{
		h = w;
	}
	else
	{
		w = h;
	}

	yy = 0;

	for (j = 0; j < 2; j++)
	{
		for (y = 0; y < 8; y++)
		{
			xx = 0;
			for (x = 0; x < 8; x++)
			{
				painter.fillRect(xx, yy, w, h, oamPattern.sprite[spriteIdx].tile[j].pixel[y][x].color);
				xx += w;
			}
			yy += h;
		}
	}
}

oamPreview_t::oamPreview_t(QWidget *parent)
	: QWidget(parent)
{
	setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
	viewHeight = 240;
	viewWidth = 256;
	setMinimumWidth(viewWidth);
	setMinimumHeight(viewHeight);
	selSprite = 0;
	cx = cy = 0;

	boxColor.setRgb(128, 128, 128);

	fceuLoadConfigColor("SDL.OAM_LocatorColor", &boxColor);
}

oamPreview_t::~oamPreview_t(void)
{

}

void oamPreview_t::setIndex(int val)
{
	selSprite = val;
}

void oamPreview_t::setMinScale(int scale)
{
	if (scale < 1)
	{
		scale = 1;
	}
	setMinimumWidth(scale * 256);
	setMinimumHeight(scale * 240);

	return;
}

int oamPreview_t::heightForWidth(int w) const
{
	return ((w * 256) / 240);
}

QSize oamPreview_t::minimumSizeHint(void) const
{
	return QSize(256, 240);
}

QSize oamPreview_t::maximumSizeHint(void) const
{
	return QSize(512, 480);
}

QSize oamPreview_t::sizeHint(void) const
{
	return QSize(512, 480);
}

void oamPreview_t::resizeEvent(QResizeEvent *event)
{
	viewWidth = event->size().width();
	viewHeight = event->size().height();
}

void oamPreview_t::paintEvent(QPaintEvent *event)
{
	int w, h, i, j, x, y, xx, yy, nt;
	QPainter painter(this);
	QColor bgColor(0, 0, 0);
	QPen pen;
	char spriteRendered[64];
	struct oamSpriteData_t *spr;

	pen = painter.pen();

	viewWidth = event->rect().width();
	viewHeight = event->rect().height();

	w = viewWidth / 256;
	h = viewHeight / 240;

	if (w < h)
	{
		h = w;
	}
	else
	{
		w = h;
	}

	cx = (viewWidth - (256 * w)) / 2;
	cy = (viewHeight - (240 * h)) / 2;

	if (palo != NULL)
	{
		int p = palcache[0];

		bgColor.setRed(palo[p].r);
		bgColor.setGreen(palo[p].g);
		bgColor.setBlue(palo[p].b);
	}
	painter.fillRect(cx, cy, w * 256, h * 240, bgColor);

	nt = (oamPattern.mode8x16) ? 2 : 1;

	for (i = 63; i >= 0; i--)
	{
		spr = &oamPattern.sprite[i];

		spriteRendered[i] = 0;

		if (spr->y >= 0xEF)
		{
			continue;
		}

		yy = (spr->y * h) + cy;

		for (j = 0; j < nt; j++)
		{
			for (y = 0; y < 8; y++)
			{
				xx = (spr->x * w) + cx;

				for (x = 0; x < 8; x++)
				{
					painter.fillRect(xx, yy, w, h, spr->tile[j].pixel[y][x].color);
					xx += w;
				}
				yy += h;
			}
		}
		spriteRendered[i] = 1;
	}

	if (spriteRendered[selSprite])
	{
		spr = &oamPattern.sprite[selSprite];

		pen.setWidth(1);
		pen.setColor(boxColor);
		painter.setPen(pen);

		yy = (spr->y * h) + cy;
		xx = (spr->x * w) + cx;

		painter.drawRect(xx, yy, w * 8, h * nt * 8);
	}
}
