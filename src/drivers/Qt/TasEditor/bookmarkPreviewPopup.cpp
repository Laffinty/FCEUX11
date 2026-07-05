// bookmarkPreviewPopup.cpp
//

#include <stdio.h>
#include "utils/safe_string.h"
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <string>
#include <zlib.h>

#include <QSettings>

#include "fceu.h"
#include "core_api.h"

#include "common/vidblit.h"
#include "Qt/config.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/TasEditor/TasColors.h"
#include "Qt/TasEditor/TasEditorWindow.h"
#include "Qt/TasEditor/bookmarkPreviewPopup.h"
#include "Qt/TasEditor/bookmarks.h"
#include "Qt/TasEditor/markers_manager.h"

bookmarkPreviewPopup *bookmarkPreviewPopup::instance = 0;
//----------------------------------------------------------------------------
bookmarkPreviewPopup::bookmarkPreviewPopup( int index, QWidget *parent )
	: QDialog( parent, Qt::ToolTip )
{
	int p;
	QPoint pos;
	QVBoxLayout *vbox;
	uint32_t *pixBuf;
	uint32_t  pixel;
	QPixmap pixmap;

	if ( instance )
	{
		//instance->done(0);
		//instance->deleteLater();
		instance->actv = false;
		instance = 0;
	}
	instance = this;

	imageIndex = index;

	//qApp->installEventFilter(this);

	//FCEU_WRAPPER_LOCK();

	// retrieve info from the pointed bookmark's Markers
	int frame = bookmarks->bookmarksArray[index].snapshot.keyFrame;
	int markerID = markersManager->getMarkerAboveFrame(bookmarks->bookmarksArray[index].snapshot.markers, frame);

	screenShotRaster = (unsigned char *)malloc( SCREENSHOT_SIZE );

	if ( screenShotRaster == NULL )
	{
		printf("Error: Failed to allocate screenshot image memory\n");
	}
	// bookmarks.itemUnderMouse

	pixBuf = (uint32_t *)malloc( SCREENSHOT_SIZE * sizeof(uint32_t) );

	loadImage(index);

	p=0;
	for (int h=0; h<SCREENSHOT_HEIGHT; h++)
	{
		for (int w=0; w<SCREENSHOT_WIDTH; w++)
		{
			pixel = ModernDeemphColorMap( &screenShotRaster[p], screenShotRaster, 1 );
			pixBuf[p]  = 0xFF000000;
			pixBuf[p] |= (pixel & 0x000000FF) << 16;
			pixBuf[p] |= (pixel & 0x00FF0000) >> 16;
			pixBuf[p] |= (pixel & 0x0000FF00);
			p++;
		}
	}
	QImage img( (unsigned char*)pixBuf, SCREENSHOT_WIDTH, SCREENSHOT_HEIGHT, SCREENSHOT_WIDTH*4, QImage::Format_RGBA8888 );
	pixmap.convertFromImage( img );

	vbox = new QVBoxLayout();

	setLayout( vbox );

	imgLbl  = new QLabel();
	descLbl = new QLabel();

	imgLbl->setPixmap( pixmap );

	vbox->addWidget( imgLbl , 100 );
	vbox->addWidget( descLbl, 1 );

	descLbl->setText( tr(markersManager->getNoteCopy(bookmarks->bookmarksArray[index].snapshot.markers, markerID).c_str()) );

	resize( 256, 256 );

	if ( pixBuf )
	{
		free( pixBuf ); pixBuf = NULL;
	}

	pos = tasWin->getPreviewPopupCoordinates();

	pos.setX( pos.x() - 300 );

	move(pos);

	//FCEU_WRAPPER_UNLOCK();

	alpha = 0;
	actv  = true;

	setWindowOpacity(0.0f);

	timer = new QTimer(this);

	connect( timer, &QTimer::timeout, this, &bookmarkPreviewPopup::periodicUpdate );

	timer->start(33);

}
//----------------------------------------------------------------------------
void bookmarkPreviewPopup::retranslateUi(void)
{
	if (descLbl)
	{
		int frame = bookmarks->bookmarksArray[imageIndex].snapshot.keyFrame;
		int markerID = markersManager->getMarkerAboveFrame(bookmarks->bookmarksArray[imageIndex].snapshot.markers, frame);
		descLbl->setText( tr(markersManager->getNoteCopy(bookmarks->bookmarksArray[imageIndex].snapshot.markers, markerID).c_str()) );
	}
}
//----------------------------------------------------------------------------
void bookmarkPreviewPopup::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//----------------------------------------------------------------------------
bookmarkPreviewPopup::~bookmarkPreviewPopup( void )
{
	timer->stop();

	if ( screenShotRaster != NULL )
	{
		free( screenShotRaster ); screenShotRaster = NULL;
	}
	//printf("Popup Deleted\n");
}
//----------------------------------------------------------------------------
void bookmarkPreviewPopup::periodicUpdate(void)
{
	if ( actv )
	{
		if ( alpha < 255 )
		{
			alpha += 25;

			if ( alpha > 255 )
			{
				alpha = 255;
			}
			setWindowOpacity( alpha / 255.0f );

			update();
		}
	}
	else
	{
		if ( alpha > 0 )
		{
			alpha -= 25;

			if ( alpha < 0 )
			{
				alpha = 0;
			}
			setWindowOpacity( alpha / 255.0f );

			update();
		}
		else
		{
			if ( instance == this )
			{
				instance = NULL;
			}
			done(0);
			deleteLater();
		}
	}
}
//----------------------------------------------------------------------------
bookmarkPreviewPopup *bookmarkPreviewPopup::currentInstance(void)
{
	return instance;
}
//----------------------------------------------------------------------------
int bookmarkPreviewPopup::currentIndex(void)
{
	if ( instance )
	{
		return instance->imageIndex;
	}
	return -1;
}
//----------------------------------------------------------------------------
void bookmarkPreviewPopup::imageIndexChanged(int newIndex)
{
	FCEU_CRITICAL_SECTION(emuLock);
	//printf("newIndex:%i\n", newIndex );

	if ( newIndex >= 0 )
	{
		reloadImage(newIndex);
		actv = true;
	}
	else
	{
		actv = false;
	}

	//if ( instance == this )
	//{
	//	instance = NULL;
	//}
}
//----------------------------------------------------------------------------
int bookmarkPreviewPopup::loadImage(int index)
{
	// uncompress
	int ret = 0;
	uLongf destlen = SCREENSHOT_SIZE;
	int e = uncompress(screenShotRaster, &destlen, &bookmarks->bookmarksArray[index].savedScreenshot[0], bookmarks->bookmarksArray[index].savedScreenshot.size());
	if (e != Z_OK && e != Z_BUF_ERROR)
	{
		// error decompressing
		FCEU_printf("Error decompressing screenshot %d\n", index);
		// at least fill bitmap with zeros
		memset(screenShotRaster, 0, SCREENSHOT_SIZE);
		ret = -1;
	}
	return ret;
}
//----------------------------------------------------------------------------
int bookmarkPreviewPopup::reloadImage(int index)
{
	int p, ret = 0;
	uint32_t *pixBuf;
	uint32_t  pixel;
	QPixmap pixmap;

	if ( index == imageIndex )
	{	// no change
		return 0;
	}
	actv = true;
	imageIndex = index;

	// retrieve info from the pointed bookmark's Markers
	int frame = bookmarks->bookmarksArray[index].snapshot.keyFrame;
	int markerID = markersManager->getMarkerAboveFrame(bookmarks->bookmarksArray[index].snapshot.markers, frame);

	pixBuf = (uint32_t *)malloc( SCREENSHOT_SIZE * sizeof(uint32_t) );

	loadImage(index);

	p=0;
	for (int h=0; h<SCREENSHOT_HEIGHT; h++)
	{
		for (int w=0; w<SCREENSHOT_WIDTH; w++)
		{
			pixel = ModernDeemphColorMap( &screenShotRaster[p], screenShotRaster, 1 );
			pixBuf[p]  = 0xFF000000;
			pixBuf[p] |= (pixel & 0x000000FF) << 16;
			pixBuf[p] |= (pixel & 0x00FF0000) >> 16;
			pixBuf[p] |= (pixel & 0x0000FF00);
			p++;
		}
	}
	QImage img( (unsigned char*)pixBuf, SCREENSHOT_WIDTH, SCREENSHOT_HEIGHT, SCREENSHOT_WIDTH*4, QImage::Format_RGBA8888 );
	pixmap.convertFromImage( img );

	if ( pixBuf )
	{
		free( pixBuf ); pixBuf = NULL;
	}

	imgLbl->setPixmap( pixmap );

	descLbl->setText( tr(markersManager->getNoteCopy(bookmarks->bookmarksArray[index].snapshot.markers, markerID).c_str()) );

	update();

	return ret;
}
//----------------------------------------------------------------------------
