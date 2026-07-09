// ppuViewerTileEditor.cpp
//
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#include <QMenuBar>
#include <QMenu>
#include <QAction>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QTreeWidget>
#include <QSettings>

#include "../../fceu.h"
#include "../../ppu.h"
#include "../../palette.h"

#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/config.h"
#include "Qt/fceuWrapper.h"
#include "Qt/HexEditor.h"
#include "Qt/ppuViewerTileEditor.h"

ppuTileEditor_t::ppuTileEditor_t(int patternIndex, QWidget *parent)
	: QDialog(parent, Qt::Window)
{
	QVBoxLayout *mainLayout;
	QHBoxLayout *hbox;
	QMenuBar *menuBar;
	QMenu *fileMenu, *helpMenu;
	QAction *act;
	QSettings settings;
	int useNativeMenuBar;

	this->setFocusPolicy(Qt::StrongFocus);

	menuBar = new QMenuBar(this);

	g_config->getOption("SDL.UseNativeMenuBar", &useNativeMenuBar);

	menuBar->setNativeMenuBar(useNativeMenuBar ? true : false);

	fileMenu = menuBar->addMenu(tr("&File"));

	act = new QAction(tr("&Close"), this);
	act->setShortcut(QKeySequence::Close);
	act->setStatusTip(tr("Close Window"));
	connect(act, SIGNAL(triggered()), this, SLOT(closeWindow(void)));

	fileMenu->addAction(act);

	helpMenu = menuBar->addMenu(tr("&Help"));

	act = new QAction(tr("Keys"), this);
	act->setStatusTip(tr("View Key Descriptions"));
	connect(act, SIGNAL(triggered()), this, SLOT(showKeyAssignments(void)));

	helpMenu->addAction(act);

	tileAddr = 0;
	palIdx = pindex[patternIndex];

	setWindowTitle(tr("PPU Tile Editor"));

	mainLayout = new QVBoxLayout();

	mainLayout->setMenuBar(menuBar);

	setLayout(mainLayout);

	tileView = new ppuTileView_t(patternIndex, this);

	if (patternIndex)
	{
		tileView->setPattern(&pattern1);
	}
	else
	{
		tileView->setPattern(&pattern0);
	}
	tileView->setPaletteNES(palIdx);

	colorPicker = new ppuTileEditColorPicker_t();

	colorPicker->setPaletteNES(palIdx);

	tileIdxLbl = new QLabel();

	palSelBox = new QComboBox();
	palSelBox->addItem(tr("Tile 0"), 0);
	palSelBox->addItem(tr("Tile 1"), 1);
	palSelBox->addItem(tr("Tile 2"), 2);
	palSelBox->addItem(tr("Tile 3"), 3);
	palSelBox->addItem(tr("Sprite 0"), 4);
	palSelBox->addItem(tr("Sprite 1"), 5);
	palSelBox->addItem(tr("Sprite 2"), 6);
	palSelBox->addItem(tr("Sprite 3"), 7);
	palSelBox->addItem(tr("GrayScale"), 8);

	palSelBox->setCurrentIndex(palIdx);

	connect(palSelBox, SIGNAL(currentIndexChanged(int)), this, SLOT(paletteChanged(int)));

	mainLayout->addWidget(tileIdxLbl, 1);
	mainLayout->addWidget(tileView, 100);
	mainLayout->addWidget(colorPicker, 10);

	hbox = new QHBoxLayout();
	hbox->addWidget(new QLabel(tr("Palette:")), 1);
	hbox->addWidget(palSelBox, 10);

	mainLayout->addLayout(hbox, 1);

	updateTimer = new QTimer(this);

	connect(updateTimer, &QTimer::timeout, this, &ppuTileEditor_t::periodicUpdate);

	updateTimer->start(100);

	restoreGeometry(settings.value("ppuTileEditorWindow/geometry").toByteArray());
}

ppuTileEditor_t::~ppuTileEditor_t(void)
{
	QSettings settings;
	updateTimer->stop();

	settings.setValue("ppuTileEditorWindow/geometry", saveGeometry());
}

void ppuTileEditor_t::closeEvent(QCloseEvent *event)
{
	done(0);
	deleteLater();
	event->accept();
}

void ppuTileEditor_t::closeWindow(void)
{
	done(0);
	deleteLater();
}

void ppuTileEditor_t::periodicUpdate(void)
{
	tileView->update();
	colorPicker->update();
}

void ppuTileEditor_t::paletteChanged(int index)
{
	palIdx = index;

	tileView->setPaletteNES(palIdx);
	colorPicker->setPaletteNES(palIdx);
}

void ppuTileEditor_t::setTile(QPoint *t)
{
	if ((t->x() < 16) && (t->y() < 16))
	{
		int addr;
		char stmp[64];

		addr = tileView->getPatternIndex() ? 0x1000 : 0x0000;
		addr = addr + (t->y() * 0x0100);
		addr = addr + (t->x() * 0x0010);

		snprintf(stmp, sizeof(stmp), "Tile Index: $%X%X   Address: $%04X", t->y(), t->x(), addr);
		tileIdxLbl->setText(tr(stmp));

		tileView->setTile(t);
		tileAddr = addr;
	}
}

void ppuTileEditor_t::setCellValue(int y, int x, int colorIndex)
{
	int a;
	unsigned char chr0, chr1, mask, val;

	chr0 = (colorIndex & 0x01) ? 1 : 0;
	chr1 = (colorIndex & 0x02) ? 1 : 0;

	a = tileAddr + y;

	mask = (0x01 << (7 - x));

	val = getPPU(a);

	if (chr0)
	{
		val = val | mask;
	}
	else
	{
		val = val & ~mask;
	}
	writeMemPPU(a, val);

	val = getPPU(a + 8);

	if (chr1)
	{
		val = val | mask;
	}
	else
	{
		val = val & ~mask;
	}
	writeMemPPU(a + 8, val);

	hexEditorRequestUpdateAll();
}

void ppuTileEditor_t::showKeyAssignments(void)
{
	int i;
	QDialog *dialog;
	QVBoxLayout *mainLayout;
	QTreeWidget *tree;
	QTreeWidgetItem *item;
	const char *txt[] =
	{
		"Up", "Move Selected Cell Up",
		"Down", "Move Selected Cell Down",
		"Left", "Move Selected Cell Left",
		"Right", "Move Selected Cell Right",
		"1", "Set Selected Cell to Color #1",
		"2", "Set Selected Cell to Color #2",
		"3", "Set Selected Cell to Color #3",
		"4", "Set Selected Cell to Color #4",
		"P", "Cycle to Next Tile Palette",
		"ESC", "Close Window",
		NULL
	};

	dialog = new QDialog(this);
	dialog->setWindowTitle("Tile Editor Key Descriptions");
	dialog->resize(512, 512);

	tree = new QTreeWidget();

	tree->setColumnCount(2);

	item = new QTreeWidgetItem();
	item->setText(0, QString::fromStdString("Key"));
	item->setText(1, QString::fromStdString("Description"));
	item->setTextAlignment(0, Qt::AlignLeft);
	item->setTextAlignment(1, Qt::AlignLeft);

	tree->setHeaderItem(item);

	tree->header()->setSectionResizeMode(QHeaderView::ResizeToContents);

	i = 0;
	while (txt[i] != NULL)
	{

		item = new QTreeWidgetItem();

		item->setText(0, tr(txt[i])); i++;
		item->setText(1, tr(txt[i])); i++;

		item->setTextAlignment(0, Qt::AlignLeft);
		item->setTextAlignment(1, Qt::AlignLeft);

		tree->addTopLevelItem(item);
	}
	mainLayout = new QVBoxLayout();

	mainLayout->addWidget(tree);

	dialog->setLayout(mainLayout);

	dialog->show();
}

void ppuTileEditor_t::keyPressEvent(QKeyEvent *event)
{
	if ((event->key() >= Qt::Key_1) && (event->key() <= Qt::Key_4))
	{
		QPoint cell;
		int selColor = event->key() - Qt::Key_1;

		colorPicker->setColor(selColor);

		cell = tileView->getSelPix();

		setCellValue(cell.y(), cell.x(), selColor);

		PPUViewSkip = 100;
		FCEUD_UpdatePPUView(-1, 1);

		event->accept();
	}
	else if (event->key() == Qt::Key_P)
	{
		palIdx = (palIdx + 1) % 9;

		tileView->setPaletteNES(palIdx);
		colorPicker->setPaletteNES(palIdx);
		palSelBox->setCurrentIndex(palIdx);

		event->accept();
	}
	else if (event->key() == Qt::Key_Up)
	{
		tileView->moveSelBoxUpDown(-1);

		event->accept();
	}
	else if (event->key() == Qt::Key_Down)
	{
		tileView->moveSelBoxUpDown(1);

		event->accept();
	}
	else if (event->key() == Qt::Key_Left)
	{
		tileView->moveSelBoxLeftRight(-1);

		event->accept();
	}
	else if (event->key() == Qt::Key_Right)
	{
		tileView->moveSelBoxLeftRight(1);

		event->accept();
	}
	else if (event->key() == Qt::Key_Escape)
	{
		closeWindow();

		event->accept();
	}

}

ppuTileView_t::ppuTileView_t(int patternIndexID, QWidget *parent)
	: QWidget(parent)
{
	this->setFocusPolicy(Qt::StrongFocus);
	this->setMouseTracking(true);
	patternIndex = patternIndexID;
	paletteIndex = 0;
	setMinimumWidth(256);
	setMinimumHeight(256);
	viewWidth = 256;
	viewHeight = 256;
	tileLabel = NULL;
	drawTileGrid = true;
}

void ppuTileView_t::setPattern(ppuPatternTable_t *p)
{
	pattern = p;
}

void ppuTileView_t::setPaletteNES(int palIndex)
{
	paletteIndex = palIndex << 2;
}

void ppuTileView_t::setTile(QPoint *t)
{
	selTile = *t;
}

void ppuTileView_t::setTileLabel(QLabel *l)
{
	tileLabel = l;
}

void ppuTileView_t::moveSelBoxUpDown(int i)
{
	int y;

	y = selPix.y();

	y = y + (i % 8);

	if (y < 0)
	{
		y = y + 8;
	}
	else if (y >= 8)
	{
		y = y - 8;
	}

	selPix.setY(y);

}

void ppuTileView_t::moveSelBoxLeftRight(int i)
{
	int x;

	x = selPix.x();

	x = x + (i % 8);

	if (x < 0)
	{
		x = x + 8;
	}
	else if (x >= 8)
	{
		x = x - 8;
	}

	selPix.setX(x);

}

void ppuTileView_t::setSelCell(QPoint &p)
{
	selPix = p;
}

ppuTileView_t::~ppuTileView_t(void)
{

}

QPoint ppuTileView_t::convPixToCell(QPoint p)
{
	QPoint t(0, 0);
	int x, y, w, h, i, j;

	x = p.x(); y = p.y();

	w = boxWidth;
	h = boxHeight;

	i = x / w;
	j = y / h;

	t.setX(i);
	t.setY(j);

	return t;
}

void ppuTileView_t::resizeEvent(QResizeEvent *event)
{
	viewWidth = event->size().width();
	viewHeight = event->size().height();

	boxWidth = viewWidth / 8;
	boxHeight = viewHeight / 8;
}

void ppuTileView_t::keyPressEvent(QKeyEvent *event)
{
	event->ignore();
}

void ppuTileView_t::mouseMoveEvent(QMouseEvent *event)
{
}

void ppuTileView_t::mousePressEvent(QMouseEvent * event)
{
	QPoint cell = convPixToCell(event->pos());

	if (event->button() == Qt::LeftButton)
	{
		setSelCell(cell);
	}
}

void ppuTileView_t::contextMenuEvent(QContextMenuEvent *event)
{
}

void ppuTileView_t::paintEvent(QPaintEvent *event)
{
	int x, y, w, h, xx, yy, ii, jj;
	QPainter painter(this);
	QColor   color[4];
	QPen     pen;

	pen = painter.pen();

	viewWidth = event->rect().width();
	viewHeight = event->rect().height();

	for (x = 0; x < 4; x++)
	{
		color[x].setBlue(palo[palcache[paletteIndex | x]].b);
		color[x].setGreen(palo[palcache[paletteIndex | x]].g);
		color[x].setRed(palo[palcache[paletteIndex | x]].r);
	}

	w = viewWidth / 8;
	h = viewHeight / 8;

	boxWidth = w;
	boxHeight = h;

	xx = 0; yy = 0;

	if (w < h)
	{
		h = w;
	}
	else
	{
		w = h;
	}

	ii = selTile.x();
	jj = selTile.y();

	for (x = 0; x < 8; x++)
	{
		yy = 0;

		for (y = 0; y < 8; y++)
		{
			painter.fillRect(xx, yy, w, h, color[pattern->tile[jj][ii].pixel[y][x].val & 0x03]);
			yy += h;
		}
		xx += w;
	}

	if (drawTileGrid)
	{
		pen.setWidth(1);
		pen.setColor(QColor(128, 128, 128));
		painter.setPen(pen);

		xx = 0; y = 8 * h;

		for (x = 0; x < 9; x++)
		{
			painter.drawLine(xx, 0, xx, y); xx += w;
		}
		yy = 0; x = 8 * w;

		for (y = 0; y < 9; y++)
		{
			painter.drawLine(0, yy, x, yy); yy += h;
		}
	}

	x = selPix.x() * w;
	y = selPix.y() * h;

	pen.setWidth(6);
	pen.setColor(QColor(0, 0, 0));
	painter.setPen(pen);

	painter.drawRect(x, y, w, h);

	pen.setWidth(2);
	pen.setColor(QColor(255, 0, 0));
	painter.setPen(pen);

	painter.drawRect(x, y, w, h);
}

ppuTileEditColorPicker_t::ppuTileEditColorPicker_t(QWidget *parent)
	: QWidget(parent)
{
	int boxPixSize = 64;
	this->setFocusPolicy(Qt::StrongFocus);
	this->setMouseTracking(true);

	font.setFamily("Courier New");
	font.setStyle(QFont::StyleNormal);
	font.setStyleHint(QFont::Monospace);
	font.setPixelSize(boxPixSize / 3);
	QFontMetrics fm(font);

	setMinimumWidth(boxPixSize * NUM_COLORS);
	setMinimumHeight(boxPixSize);

	viewWidth = boxPixSize * NUM_COLORS;
	viewHeight = boxPixSize;

	boxWidth = viewWidth / NUM_COLORS;
	boxHeight = viewHeight;

	selColor = 0;
	paletteIndex = 0;

#if QT_VERSION > QT_VERSION_CHECK(5, 11, 0)
	pxCharWidth = fm.horizontalAdvance(QLatin1Char('2'));
#else
	pxCharWidth = fm.width(QLatin1Char('2'));
#endif
	pxCharHeight = fm.height();
}

ppuTileEditColorPicker_t::~ppuTileEditColorPicker_t(void)
{

}

QPoint ppuTileEditColorPicker_t::convPixToTile(QPoint p)
{
	QPoint t(0, 0);

	t.setX(p.x() / boxWidth);
	t.setY(p.y() / boxHeight);

	return t;
}

void ppuTileEditColorPicker_t::setColor(int colorIndex)
{
	selColor = colorIndex;
}

void ppuTileEditColorPicker_t::setPaletteNES(int palIndex)
{
	paletteIndex = palIndex << 2;
}

void ppuTileEditColorPicker_t::resizeEvent(QResizeEvent *event)
{
	viewWidth = event->size().width();
	viewHeight = event->size().height();

	boxWidth = viewWidth / NUM_COLORS;
	boxHeight = viewHeight;
}

void ppuTileEditColorPicker_t::mouseMoveEvent(QMouseEvent *event)
{
}

void ppuTileEditColorPicker_t::mousePressEvent(QMouseEvent * event)
{
}

void ppuTileEditColorPicker_t::paintEvent(QPaintEvent *event)
{
	int x, y, w, h, xx, yy;
	QPainter painter(this);
	viewWidth = event->rect().width();
	viewHeight = event->rect().height();
	QColor color[NUM_COLORS];
	QPen pen;

	painter.setFont(font);

	pen = painter.pen();

	w = boxWidth;
	h = boxHeight;

	yy = 0;
	xx = 0;

	y = 0;
	for (x = 0; x < NUM_COLORS; x++)
	{
		color[x].setBlue(palo[palcache[paletteIndex | x]].b);
		color[x].setGreen(palo[palcache[paletteIndex | x]].g);
		color[x].setRed(palo[palcache[paletteIndex | x]].r);

		painter.fillRect(xx, yy, w, h, color[x]);
		xx += w;
	}

	y = h;
	for (int i = 0; i <= NUM_COLORS; i++)
	{
		x = i * w;
		painter.drawLine(x, 0, x, y);
	}

	x = NUM_COLORS * w;
	for (int i = 0; i <= 1; i++)
	{
		y = i * h;
		painter.drawLine(0, y, x, y);
	}

	pen.setWidth(6);
	painter.setPen(pen);

	x = selColor * w;
	painter.drawRect(x + 3, 3, w - 6, h - 6);

	pen.setWidth(2);
	pen.setColor(QColor(255, 255, 255));
	painter.setPen(pen);
	painter.drawRect(x + 3, 3, w - 6, h - 6);

	y = (pxCharHeight) + (h - pxCharHeight) / 2;

	for (int i = 0; i < NUM_COLORS; i++)
	{
		char c[2];

		x = (i * w) + (w - pxCharWidth) / 2;

		c[0] = '1' + i;
		c[1] = 0;

		if (qGray(color[i].red(), color[i].green(), color[i].blue()) > 128)
		{
			painter.setPen(QColor(0, 0, 0));
		}
		else
		{
			painter.setPen(QColor(255, 255, 255));
		}
		painter.drawText(x, y, tr(c));
	}
}
