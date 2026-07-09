// ppuViewerPalette.cpp
//
#include <stdio.h>
#include <stdint.h>

#include <QDir>
#include <QMenu>
#include <QAction>
#include <QPainter>
#include <QFileDialog>
#include <QClipboard>
#include <QGuiApplication>

#include "../../fceu.h"
#include "../../palette.h"

#include "Qt/main.h"
#include "Qt/config.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/PaletteEditor.h"
#include "Qt/ppuViewerPalette.h"

tilePaletteView_t::tilePaletteView_t(QWidget *parent)
	: QWidget(parent)
{
	setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
	viewHeight = 32;
	viewWidth = viewHeight * 4;
	setMinimumWidth(viewWidth);
	setMinimumHeight(viewHeight);

	boxWidth = viewWidth / 4;
	boxHeight = viewHeight;
	palIdx = 0;
	selBox = 0;
}

tilePaletteView_t::~tilePaletteView_t(void)
{

}

void tilePaletteView_t::setIndex(int val)
{
	palIdx = val;
}

int tilePaletteView_t::heightForWidth(int w) const
{
	return w / 4;
}

QSize tilePaletteView_t::minimumSizeHint(void) const
{
	return QSize(48, 12);
}

QSize tilePaletteView_t::maximumSizeHint(void) const
{
	return QSize(256, 64);
}

QSize tilePaletteView_t::sizeHint(void) const
{
	return QSize(128, 32);
}

QPoint tilePaletteView_t::convPixToCell(QPoint p)
{
	QPoint o;

	o.setX(p.x() / boxWidth);
	o.setY(0);

	return o;
}

void tilePaletteView_t::mouseMoveEvent(QMouseEvent *event)
{
	QPoint cell = convPixToCell(event->pos());

	selBox = cell.x();
}

void tilePaletteView_t::contextMenuEvent(QContextMenuEvent *event)
{
	QAction *act;
	QMenu menu(this);
	QMenu *subMenu;
	char stmp[128];
	QPoint p;

	p = convPixToCell(event->pos());

	selBox = p.x();

	act = new QAction(tr("Change Color"), &menu);
	connect(act, SIGNAL(triggered(void)), this, SLOT(openColorPicker(void)));
	menu.addAction(act);

	act = new QAction(tr("Export ACT"), &menu);
	connect(act, SIGNAL(triggered(void)), this, SLOT(exportPaletteFileDialog(void)));
	menu.addAction(act);

	if (palo)
	{
		int i;

		i = palcache[(palIdx << 2) | selBox];

		subMenu = menu.addMenu(tr("Copy Color to Clipboard"));

		snprintf(stmp, sizeof(stmp), "Hex #%02X%02X%02X", palo[i].r, palo[i].g, palo[i].b);
		act = new QAction(tr(stmp), &menu);
		connect(act, SIGNAL(triggered(void)), this, SLOT(copyColor2ClipBoardHex(void)));
		subMenu->addAction(act);

		snprintf(stmp, sizeof(stmp), "rgb(%3i,%3i,%3i)", palo[i].r, palo[i].g, palo[i].b);
		act = new QAction(tr(stmp), &menu);
		connect(act, SIGNAL(triggered(void)), this, SLOT(copyColor2ClipBoardRGB(void)));
		subMenu->addAction(act);
	}

	menu.exec(event->globalPos());
}

void tilePaletteView_t::copyColor2ClipBoardHex(void)
{
	int p;
	char txt[64];
	QClipboard *clipboard = QGuiApplication::clipboard();

	if (palo == NULL)
	{
		return;
	}
	p = palcache[(palIdx << 2) | selBox];

	snprintf(txt, sizeof(txt), "#%02X%02X%02X", palo[p].r, palo[p].g, palo[p].b);

	clipboard->setText(tr(txt), QClipboard::Clipboard);

	if (clipboard->supportsSelection())
	{
		clipboard->setText(tr(txt), QClipboard::Selection);
	}
}

void tilePaletteView_t::copyColor2ClipBoardRGB(void)
{
	int p;
	char txt[64];
	QClipboard *clipboard = QGuiApplication::clipboard();

	if (palo == NULL)
	{
		return;
	}
	p = palcache[(palIdx << 2) | selBox];

	snprintf(txt, sizeof(txt), "rgb(%3i,%3i,%3i)", palo[p].r, palo[p].g, palo[p].b);

	clipboard->setText(tr(txt), QClipboard::Clipboard);

	if (clipboard->supportsSelection())
	{
		clipboard->setText(tr(txt), QClipboard::Selection);
	}
}

void tilePaletteView_t::exportPaletteFileDialog(void)
{
	int ret, useNativeFileDialogVal;
	QString filename;
	QFileDialog dialog(this, tr("Export Palette To File"));
	const char *home;

	dialog.setFileMode(QFileDialog::AnyFile);

	dialog.setNameFilter(tr("Adobe Color Table Files (*.act *.ACT) ;; All files (*)"));

	dialog.setViewMode(QFileDialog::List);
	dialog.setFilter(QDir::AllEntries | QDir::AllDirs | QDir::Hidden);
	dialog.setLabelText(QFileDialog::Accept, tr("Export"));
	dialog.setDefaultSuffix(tr(".act"));

	home = ::getenv("HOME");

	if (home)
	{
		dialog.setDirectory(tr(home));
	}

	g_config->getOption("SDL.UseNativeFileDialog", &useNativeFileDialogVal);

	dialog.setOption(QFileDialog::DontUseNativeDialog, !useNativeFileDialogVal);

	ret = dialog.exec();

	if (ret)
	{
		QStringList fileList;
		fileList = dialog.selectedFiles();

		if (fileList.size() > 0)
		{
			filename = fileList[0];
		}
	}

	if (filename.isNull())
	{
		return;
	}

	exportActivePaletteACT(filename.toStdString().c_str());
}

void tilePaletteView_t::openColorPicker(void)
{
	nesPalettePickerDialog *dialog;

	dialog = new nesPalettePickerDialog((palIdx << 2) + selBox, this);

	dialog->show();
}

void tilePaletteView_t::resizeEvent(QResizeEvent *event)
{
	viewWidth = event->size().width();
	viewHeight = event->size().height();
}

void tilePaletteView_t::paintEvent(QPaintEvent *event)
{
	int x, w, h, xx, yy, p, p2, i, j;
	QPainter painter(this);
	QColor color(0, 0, 0);
	QColor white(255, 255, 255), black(0, 0, 0);
	char c[4];

	viewWidth = event->rect().width();
	viewHeight = event->rect().height();

	w = viewWidth / 4;
	h = viewHeight;

	boxWidth = w;
	boxHeight = h;

	i = w / 4;
	j = h / 4;

	p2 = palIdx << 2;
	yy = 0;
	xx = 0;
	for (x = 0; x < 4; x++)
	{
		if (palo != NULL)
		{
			p = palcache[p2 | x];
			color.setBlue(palo[p].b);
			color.setGreen(palo[p].g);
			color.setRed(palo[p].r);

			c[0] = conv2hex((p & 0xF0) >> 4);
			c[1] = conv2hex(p & 0x0F);
			c[2] = 0;
		}
		painter.fillRect(xx, yy, w, h, color);

		if (qGray(color.red(), color.green(), color.blue()) > 128)
		{
			painter.setPen(black);
		}
		else
		{
			painter.setPen(white);
		}
		painter.drawText(xx + i, yy + h - j, tr(c));

		painter.setPen(black);
		painter.drawRect(xx, yy, w - 1, h - 1);
		painter.setPen(white);
		painter.drawRect(xx + 1, yy + 1, w - 3, h - 3);
		xx += w;
	}
}

oamPaletteView_t::oamPaletteView_t(QWidget *parent)
	: QWidget(parent)
{
	setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
	viewHeight = 32;
	viewWidth = viewHeight * 4;
	setMinimumWidth(viewWidth);
	setMinimumHeight(viewHeight);

	palIdx = 0;
}

oamPaletteView_t::~oamPaletteView_t(void)
{

}

void oamPaletteView_t::setIndex(int val)
{
	palIdx = oamPattern.sprite[val].pal;
}

int oamPaletteView_t::heightForWidth(int w) const
{
	return w / 4;
}

QSize oamPaletteView_t::minimumSizeHint(void) const
{
	return QSize(48, 12);
}

QSize oamPaletteView_t::maximumSizeHint(void) const
{
	return QSize(256, 64);
}

QSize oamPaletteView_t::sizeHint(void) const
{
	return QSize(128, 32);
}

void oamPaletteView_t::resizeEvent(QResizeEvent *event)
{
	viewWidth = event->size().width();
	viewHeight = event->size().height();
}

void oamPaletteView_t::paintEvent(QPaintEvent *event)
{
	int x, w, h, xx, yy, p, p2, i, j;
	QPainter painter(this);
	QColor color(0, 0, 0);
	QColor white(255, 255, 255), black(0, 0, 0);
	char c[4];

	viewWidth = event->rect().width();
	viewHeight = event->rect().height();

	w = viewWidth / 4;
	h = viewHeight;

	if (w < h)
	{
		h = w;
	}
	else
	{
		w = h;
	}

	i = w / 4;
	j = h / 4;

	p2 = palIdx * 4;
	yy = 0;
	xx = 0;
	for (x = 0; x < 4; x++)
	{
		if (palo != NULL)
		{
			p = palcache[p2 | x];
			color.setBlue(palo[p].b);
			color.setGreen(palo[p].g);
			color.setRed(palo[p].r);

			c[0] = conv2hex((p & 0xF0) >> 4);
			c[1] = conv2hex(p & 0x0F);
			c[2] = 0;
		}
		painter.fillRect(xx, yy, w, h, color);

		if (qGray(color.red(), color.green(), color.blue()) > 128)
		{
			painter.setPen(black);
		}
		else
		{
			painter.setPen(white);
		}
		painter.drawText(xx + i, yy + h - j, tr(c));

		painter.setPen(black);
		painter.drawRect(xx, yy, w - 1, h - 1);
		painter.setPen(white);
		painter.drawRect(xx + 1, yy + 1, w - 3, h - 3);
		xx += w;
	}
}
