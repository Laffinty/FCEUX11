// TasEditorTimeline.cpp
//

/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2021 mjbudd77
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
// TasEditorTimeline.cpp
//
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <string>
#include <zlib.h>

#include <QDir>
#include <QDrag>
#include <QString>
#include <QPainter>
#include <QSettings>
#include <QTextEdit>
#include <QHeaderView>
#include <QMessageBox>
#include <QFontMetrics>
#include <QFileDialog>
#include <QFontDialog>
#include <QInputDialog>
#include <QStandardPaths>
#include <QActionGroup>
#include <QApplication>
#include <QGuiApplication>
#include <QDesktopServices>

#include "fceu.h"
#include "movie.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"

#include "common/vidblit.h"
#include "Qt/config.h"
#include "Qt/keyscan.h"
#include "Qt/throttle.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ColorMenu.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/TasEditor/TasColors.h"
#include "Qt/TasEditor/TasEditorWindow.h"

extern TasEditorWindow   *tasWin;
extern TASEDITOR_PROJECT *project;
extern TASEDITOR_CONFIG  *taseditorConfig;
extern TASEDITOR_LUA     *taseditor_lua;
extern MARKERS_MANAGER   *markersManager;
extern SELECTION         *selection;
extern GREENZONE         *greenzone;
extern BOOKMARKS         *bookmarks;
extern BRANCHES          *branches;
extern PLAYBACK          *playback;
extern RECORDER          *recorder;
extern HISTORY           *history;
extern SPLICER           *splicer;


#include "Qt/TasEditor/TasEditorTimeline.h"
#include "Qt/TasEditor/bookmarks.h"
#include "Qt/TasEditor/selection.h"
#include "Qt/TasEditor/markers_manager.h"
#include "Qt/TasEditor/taseditor_config.h"
#include "Qt/TasEditor/taseditor_project.h"
#include "Qt/TasEditor/playback.h"
#include "Qt/TasEditor/history.h"
#include "Qt/TasEditor/branches.h"
#include "Qt/TasEditor/splicer.h"

extern char pianoRollSaveID[PIANO_ROLL_ID_LEN];
extern char pianoRollSkipSaveID[PIANO_ROLL_ID_LEN];
extern TasFindNoteWindow *findWin;
extern uint64_t tasEditorTimeStamp;


//------ Custom Vertical Scroll For Piano Roll
PianoRollScrollBar::PianoRollScrollBar( QWidget *parent )
	: QScrollBar( Qt::Vertical, parent )
{
	pxLineSpacing = 12;
	wheelPixelCounter = 0;
	wheelAngleCounter = 0;
}
//----------------------------------------------------------------------------
PianoRollScrollBar::~PianoRollScrollBar(void)
{
}
//----------------------------------------------------------------------------
void PianoRollScrollBar::wheelEvent(QWheelEvent *event)
{
	int ofs, zDelta = 0;

	//QScrollBar::wheelEvent(event);
	QPoint numPixels = event->pixelDelta();
	QPoint numDegrees = event->angleDelta();

	ofs = value();

	if (!numPixels.isNull())
	{
		wheelPixelCounter -= numPixels.y();
		//printf("numPixels: (%i,%i) \n", numPixels.x(), numPixels.y() );

		if ( wheelPixelCounter >= pxLineSpacing )
		{
			zDelta = wheelPixelCounter / pxLineSpacing;

			wheelPixelCounter = wheelPixelCounter % pxLineSpacing;
		}
		else if ( wheelPixelCounter <= -pxLineSpacing )
		{
			zDelta = wheelPixelCounter / pxLineSpacing;

			wheelPixelCounter = wheelPixelCounter % pxLineSpacing;
		}
	}
	else if (!numDegrees.isNull())
	{
		int stepDeg = 120;
		//QPoint numSteps = numDegrees / 15;
		//printf("numSteps: (%i,%i) \n", numSteps.x(), numSteps.y() );
		//printf("numDegrees: (%i,%i)  %i\n", numDegrees.x(), numDegrees.y(), pxLineSpacing );
		wheelAngleCounter -= numDegrees.y();

		if ( wheelAngleCounter <= stepDeg )
		{
			zDelta = wheelAngleCounter / stepDeg;

			wheelAngleCounter = wheelAngleCounter % stepDeg;
		}
		else if ( wheelAngleCounter >= stepDeg )
		{
			zDelta = wheelAngleCounter / stepDeg;

			wheelAngleCounter = wheelAngleCounter % stepDeg;
		}
	}

	if ( zDelta != 0 )
	{
		ofs = ofs + (6 * zDelta);

		if ( ofs < 0 )
		{
			ofs = 0;
		}
		else if ( ofs > maximum() )
		{
			ofs = maximum();
		}
		setValue( ofs );
	}
	event->accept();
}
//----------------------------------------------------------------------------

//---- TAS Piano Roll Widget
QPianoRoll::QPianoRoll(QWidget *parent)
	: QWidget( parent )
{
	QPalette pal;
	std::string fontString;
	QColor fg("black"), bg("white"), c;

	useDarkTheme = false;

	viewWidth  = 256;
	viewHeight = 512;
	setMinimumWidth( viewWidth );
	setMinimumHeight( viewHeight );
	setAcceptDrops(true);

	g_config->getOption("SDL.TasPianoRollFont", &fontString);

	if ( fontString.size() > 0 )
	{
		//printf("Font String: '%s'\n", fontString.c_str() );
		font.fromString( QString::fromStdString( fontString ) );
	}
	else
	{
		font.setFamily("Courier New");
		font.setStyle( QFont::StyleNormal );
		font.setStyleHint( QFont::Monospace );
	}
	font.setBold(true);

	pal = this->palette();

	windowColor = pal.color(QPalette::Window);

	// Figure out if we are using a light or dark theme by checking the 
	// default window text grayscale color. If more white, then we will
	// use white text on black background, else we do the opposite.
	c = pal.color(QPalette::WindowText);

	if ( qGray( c.red(), c.green(), c.blue() ) > 128 )
	{
		useDarkTheme = true;
	}
	//printf("WindowText: R:%i  G:%i  B:%i \n", c.red(), c.green(), c.blue() );

	if ( useDarkTheme )
	{
		pal.setColor(QPalette::Base      , fg );
		pal.setColor(QPalette::Window    , fg );
		pal.setColor(QPalette::WindowText, bg );
	}
	else 
	{
		pal.setColor(QPalette::Base      , bg );
		pal.setColor(QPalette::Window    , bg );
		pal.setColor(QPalette::WindowText, fg );
	}
	this->parent = qobject_cast <TasEditorWindow*>( parent );
	this->setFocusPolicy(Qt::StrongFocus);
	this->setMouseTracking(true);
	this->setPalette(pal);

	numCtlr = 2;
	numColumns = 2 + (NUM_JOYPAD_BUTTONS * numCtlr);

	vbar = NULL;
	hbar = NULL;

	mkrDrag = NULL;
	lineOffset = 0;
	maxLineOffset = 0;
	playbackCursorPos = 0;
	dragMode = DRAG_MODE_NONE;
	dragSelectionStartingFrame = 0;
	dragSelectionEndingFrame = 0;
	realRowUnderMouse = -1;
	rowUnderMouse = -1;
	columnUnderMouse = 0;
	rowUnderMouseAtPress = -1;
	columnUnderMouseAtPress = 0;
	markerDragFrameNumber = 0;
	markerDragCountdown = 0;
	drawingStartTimestamp = 0;
	wheelPixelCounter = 0;
	wheelAngleCounter = 0;
	headerItemUnderMouse = 0;
	nextHeaderUpdateTime = 0;
	rightButtonDragMode = false;
	mouse_x = mouse_y = -1;
	scroll_x = scroll_y = 0;
	memset( headerColors, 0, sizeof(headerColors) );

	headerLightsColors[ 0] = QColor( 0x00, 0x00, 0x00 );
	headerLightsColors[ 1] = QColor( 0x13, 0x73, 0x00 );
	headerLightsColors[ 2] = QColor( 0x00, 0x91, 0x00 );
	headerLightsColors[ 3] = QColor( 0x00, 0xAF, 0x1D );
	headerLightsColors[ 4] = QColor( 0x00, 0xC7, 0x42 );
	headerLightsColors[ 5] = QColor( 0x00, 0xD9, 0x65 );
	headerLightsColors[ 6] = QColor( 0x00, 0xE5, 0x91 );
	headerLightsColors[ 7] = QColor( 0x00, 0xF0, 0xB0 );
	headerLightsColors[ 8] = QColor( 0x00, 0xF7, 0xDA );
	headerLightsColors[ 9] = QColor( 0x7C, 0xFC, 0xF0 );
	headerLightsColors[10] = QColor( 0xBA, 0xFF, 0xFC );

	hotChangesColors[ 0] = QColor( 0x00, 0x00, 0x00 );
	hotChangesColors[ 1] = QColor( 0x35, 0x40, 0x00 );
	hotChangesColors[ 2] = QColor( 0x18, 0x52, 0x18 );
	hotChangesColors[ 3] = QColor( 0x34, 0x5C, 0x5E );
	hotChangesColors[ 4] = QColor( 0x00, 0x4C, 0x80 );
	hotChangesColors[ 5] = QColor( 0x00, 0x03, 0xBA );
	hotChangesColors[ 6] = QColor( 0x38, 0x00, 0xD1 );
	hotChangesColors[ 7] = QColor( 0x72, 0x12, 0xB2 );
	hotChangesColors[ 8] = QColor( 0xAB, 0x00, 0xBA );
	hotChangesColors[ 9] = QColor( 0xB0, 0x00, 0x6F );
	hotChangesColors[10] = QColor( 0xC2, 0x00, 0x37 );
	hotChangesColors[11] = QColor( 0xBA, 0x0C, 0x00 );
	hotChangesColors[12] = QColor( 0xC9, 0x2C, 0x00 );
	hotChangesColors[13] = QColor( 0xBF, 0x53, 0x00 );
	hotChangesColors[14] = QColor( 0xCF, 0x72, 0x00 );
	hotChangesColors[15] = QColor( 0xC7, 0x8B, 0x3C );

	gridPixelWidth = 1;
	gridColor = QColor( 0x00, 0x00, 0x00 );

	fceuLoadConfigColor("SDL.TasPianoRollGridColor"   , &gridColor );

	calcFontData();
}
//----------------------------------------------------------------------------
QPianoRoll::~QPianoRoll(void)
{

}
//----------------------------------------------------------------------------
void QPianoRoll::reset(void)
{
	int num_joysticks = joysticksPerFrame[getInputType(currMovieData)];

	numCtlr = num_joysticks;

	numColumns = 2 + (NUM_JOYPAD_BUTTONS * num_joysticks);

	calcFontData();
}
//----------------------------------------------------------------------------
void QPianoRoll::save(EMUFILE *os, bool really_save)
{
	if (really_save)
	{
		updateLinesCount();
		// write "PIANO_ROLL" string
		os->fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(pianoRollSaveID), PIANO_ROLL_ID_LEN));
		// write current top item
		int top_item = lineOffset;
		write32le(top_item, os);
	}
	else
	{
		// write "PIANO_ROLX" string
		os->fwrite(pianoRollSkipSaveID, PIANO_ROLL_ID_LEN);
	}
}
//----------------------------------------------------------------------------
// returns true if couldn't load
bool QPianoRoll::load(EMUFILE *is, unsigned int offset)
{
	reset();
	updateLinesCount();
	if (offset)
	{
		if (is->fseek(offset, SEEK_SET)) goto error;
	}
	else
	{
		// scroll to the beginning
		//ListView_EnsureVisible(hwndList, 0, FALSE);
		lineOffset = 0;
		return false;
	}
	// read "PIANO_ROLL" string
	char save_id[PIANO_ROLL_ID_LEN];
	if ((int)is->fread(std::span<std::byte>(reinterpret_cast<std::byte*>(save_id), PIANO_ROLL_ID_LEN)) < PIANO_ROLL_ID_LEN) goto error;
	if (!strcmp(pianoRollSkipSaveID, save_id))
	{
		// string says to skip loading Piano Roll
		FCEU_printf("No Piano Roll data in the file\n");
		// scroll to the beginning
		//ListView_EnsureVisible(hwndList, 0, FALSE);
		lineOffset = 0;
		return false;
	}
	if (strcmp(pianoRollSaveID, save_id)) goto error;		// string is not valid
	// read current top item and scroll Piano Roll there
	int top_item;
	if (!read32le(&top_item, is)) goto error;
	//ListView_EnsureVisible(hwndList, currMovieData.getNumRecords() - 1, FALSE);
	//ListView_EnsureVisible(hwndList, top_item, FALSE);
	ensureTheLineIsVisible( currMovieData.getNumRecords() - 1 );
	ensureTheLineIsVisible( top_item );
	return false;
error:
	FCEU_printf("Error loading Piano Roll data\n");
	// scroll to the beginning
	//ListView_EnsureVisible(hwndList, 0, FALSE);
	lineOffset = 0;
	return true;
}
//----------------------------------------------------------------------------
void QPianoRoll::setScrollBars( QScrollBar *h, QScrollBar *v )
{
	hbar = h; vbar = v;
}
//----------------------------------------------------------------------------
void QPianoRoll::hbarChanged(int val)
{
	if ( viewWidth >= pxLineWidth )
	{
		pxLineXScroll = 0;
	}
	else
	{
		pxLineXScroll = val;
	}
	update();
}
//----------------------------------------------------------------------------
//void QPianoRoll::vbarActionTriggered(int act)
//{
//	int val = vbar->value();
//
//	if ( act == QAbstractSlider::SliderSingleStepAdd )
//	{
//		val = val - vbar->singleStep();
//
//		if ( val < 0 )
//		{
//			val = 0;
//		}
//		vbar->setSliderPosition(val);
//	}
//	else if ( act == QAbstractSlider::SliderSingleStepSub )
//	{
//		val = val + vbar->singleStep();
//
//		if ( val >= maxLineOffset )
//		{
//			val = maxLineOffset;
//		}
//		vbar->setSliderPosition(val);
//	}
//        else if ( act == QAbstractSlider::SliderPageStepAdd )
//        {
//               	val = val - vbar->pageStep();
//
//		if ( val < 0 )
//		{
//			val = 0;
//		}
//		vbar->setSliderPosition(val);
//        }
//        else if ( act == QAbstractSlider::SliderPageStepSub )
//        {
//                val = val + vbar->pageStep();
//
//		if ( val >= maxLineOffset )
//		{
//			val = maxLineOffset;
//		}
//		vbar->setSliderPosition(val);
//        }
//	//printf("ACT:%i\n", act);
//}
//----------------------------------------------------------------------------
void QPianoRoll::vbarChanged(int val)
{
	lineOffset = val;

	if ( lineOffset < 0 )
	{
		lineOffset = 0;
	}
	else if ( lineOffset > maxLineOffset )
	{
		lineOffset = maxLineOffset;
	}
	update();
}
//----------------------------------------------------------------------------
void QPianoRoll::setFont( QFont &newFont )
{
	font = newFont;
	font.setBold(true);
	QWidget::setFont( font );
	calcFontData();
}
//----------------------------------------------------------------------------
void QPianoRoll::calcFontData(void)
{
	QRect rect;
	QWidget::setFont(font);
	QFontMetrics metrics(font);
#if QT_VERSION > QT_VERSION_CHECK(5, 11, 0)
	pxCharWidth = metrics.horizontalAdvance(QLatin1Char('2'));
#else
	pxCharWidth = metrics.width(QLatin1Char('2'));
#endif
	pxCharHeight   = metrics.capHeight();
	pxLineSpacing  = metrics.lineSpacing() * 1.25;
	pxLineLead     = pxLineSpacing - metrics.height();
	pxCursorHeight = metrics.height();
	pxLineTextOfs  = pxLineSpacing - ((pxLineSpacing - pxCharHeight) / 2) + (pxLineSpacing - pxCharHeight + 1) % 2;

	//printf("W:%i  H:%i  LS:%i  \n", pxCharWidth, pxCharHeight, pxLineSpacing );

	viewLines   = (viewHeight / pxLineSpacing) + 1;

	pxWidthCol1     =  3 * pxCharWidth;
	pxWidthFrameCol =  9 * pxCharWidth;
	pxWidthBtnCol   =  3 * pxCharWidth;
	pxWidthCtlCol   =  8 * pxWidthBtnCol;

	rect = metrics.boundingRect( tr("000000000") );

	//printf("FrameWidth:  %i   %i\n", pxWidthFrameCol, rect.width() );
	if ( pxWidthFrameCol < rect.width() )
	{
		pxWidthFrameCol = rect.width();
	}

	pxFrameColX     = pxWidthCol1;

	for (int i=0; i<4; i++)
	{
		pxFrameCtlX[i] = pxFrameColX + pxWidthFrameCol + (i*pxWidthCtlCol);
	}
	pxLineWidth = pxFrameCtlX[ numCtlr-1 ] + pxWidthCtlCol;

	if ( vbar )
	{
		if ( maxLineOffset < 0 )
		{
			vbar->hide();
			maxLineOffset = 0;
		}
		else
		{
			vbar->show();
		}
		vbar->setMinimum(0);
		vbar->setMaximum(maxLineOffset);
		vbar->setPageStep( (7*viewLines)/8 );
	}

	if ( hbar )
	{
		if ( viewWidth >= pxLineWidth )
		{
			pxLineXScroll = 0;
			hbar->hide();
		}
		else
		{
			hbar->setPageStep( viewWidth );
			hbar->setMaximum( pxLineWidth - viewWidth );
			hbar->show();
			pxLineXScroll = hbar->value();
		}
	}
}
//----------------------------------------------------------------------------
QPoint QPianoRoll::convPixToCursor( QPoint p )
{
	QPoint c(0,0);

	if ( p.x() < 0 )
	{
		c.setX(0);
	}
	else
	{
		float x = (float)(p.x() + pxLineXScroll) / pxCharWidth;

		c.setX( (int)x );
	}

	if ( p.y() < 0 )
	{
		c.setY( -1 );
	}
	else 
	{
		float py = ( (float)p.y() ) /  (float)pxLineSpacing;

		c.setY( (int)py - 1 );
	}
	return c;
}
//----------------------------------------------------------------------------
int  QPianoRoll::calcColumn( int px )
{
	int col = -1;

	px = px + pxLineXScroll;

	if ( px < pxFrameColX )
	{
		col = COLUMN_ICONS;
	}
	else if ( px < pxFrameCtlX[0] )
	{
		col = COLUMN_FRAMENUM;
	}
	else
	{
		int i=0;

		while ( px < pxFrameCtlX[i] )
		{
			if ( i >= 3 )
			{
				break;
			}
			i++;
		}
		col = COLUMN_JOYPAD1_A + (i*8) + ( (px - pxFrameCtlX[i]) / pxWidthBtnCol);
	}
	return col;
}
//----------------------------------------------------------------------------
void QPianoRoll::drawArrow( QPainter *painter, int xl, int yl, int value )
{
	int x, y, w, h;
	QPoint p[3];
	bool hasBookmark = false;
	bool draw2ndArrow = false;
	bool draw1stArrow = true;
	QColor green( 0, 0xC0, 0x40 ), blue( 0x60, 0xC0, 0xC0 );
	QColor arrowColor1 = green;
	QColor arrowColor2 = blue;

	x = xl+(pxCharWidth/3);
	y = yl+1;
	w = pxCharWidth;
	h = pxLineSpacing-2;

	if ( (value & BOOKMARKS_WITH_GREEN_ARROW) || (value & BOOKMARKS_WITH_BLUE_ARROW) || (value & BOOKMARKS_WITH_NO_ARROW) )
	{
		char txt[4];
		int bookmarkNum;

		bookmarkNum = (value & 0x0000FFFF);

		txt[0] = (bookmarkNum % TOTAL_BOOKMARKS) + '0';
		txt[1] = 0;
		
		painter->drawText( x, y+pxLineTextOfs, tr(txt) );

		hasBookmark  = true;
		draw1stArrow = false;
		draw2ndArrow = (value & BOOKMARKS_WITH_NO_ARROW) ? false : true;

		x += pxCharWidth;

	}

	p[0] = QPoint( x, y );
	p[1] = QPoint( x, y+h );
	p[2] = QPoint( x+w, y+(h/2) );

	if ( hasBookmark )
	{
		if ( value & BOOKMARKS_WITH_GREEN_ARROW )
		{
			arrowColor1 = green;
		}
		else if ( value & BOOKMARKS_WITH_BLUE_ARROW )
		{
			arrowColor1 = blue;
		}
	}
	else
	{
		if ( value & GREEN_ARROW_IMAGE_ID )
		{
			arrowColor1 = green;

			if ( value & BLUE_ARROW_IMAGE_ID )
			{
				draw2ndArrow = true;
				arrowColor2 = blue;
			}
		}
		else if ( value & BLUE_ARROW_IMAGE_ID )
		{
			arrowColor1 = blue;
		}
	}
	if ( draw1stArrow )
	{
		painter->setBrush( arrowColor1 );
		painter->drawPolygon( p, 3 );
		x += pxCharWidth;
	}

	if ( draw2ndArrow )
	{
		x += (pxCharWidth / 4);

		p[0] = QPoint( x, y+1 );
		p[1] = QPoint( x, y+h-1 );
		p[2] = QPoint( x+w-1, y+(h/2) );

		painter->setBrush( arrowColor2 );

		painter->drawPolygon( p, 3 );
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::updateLinesCount(void)
{
	// update the number of items in the list
	int movie_size = currMovieData.getNumRecords();

	maxLineOffset = movie_size - viewLines + 2;

	if ( maxLineOffset < 0 )
	{
		maxLineOffset = 0;
	}
}
//----------------------------------------------------------------------------
bool QPianoRoll::lineIsVisible( int lineNum )
{
	int lineEnd = lineOffset + viewLines - 2;

	return ( (lineNum >= lineOffset) && (lineNum < lineEnd) );
}
//----------------------------------------------------------------------------
void QPianoRoll::ensureTheLineIsVisible( int lineNum )
{
	if ( !lineIsVisible( lineNum ) )
	{
		//int lineEnd = lineOffset + viewLines - 2;
		//printf("Seeking Frame %i\n", lineNum );

		if ( lineNum < lineOffset )
		{
			lineOffset = lineNum;
		}
		else
		{
			//printf("Seeking View Frame %i\n", lineNum );
			lineOffset = lineOffset - viewLines + 2;
		}

		if ( lineOffset < 0 )
		{
			lineOffset = 0;
		}
		else if ( lineOffset > maxLineOffset )
		{
			lineOffset = maxLineOffset;
		}
		vbar->setValue( lineOffset );

		update();
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::resizeEvent(QResizeEvent *event)
{
	viewWidth  = event->size().width();
	viewHeight = event->size().height();

	//printf("QPianoRoll Resize: %ix%i  $%04X\n", viewWidth, viewHeight );

	viewLines = (viewHeight / pxLineSpacing) + 1;

	maxLineOffset = currMovieData.records.size() - viewLines + 2;

	if ( maxLineOffset < 0 )
	{
		vbar->hide();
		maxLineOffset = 0;
	}
	else
	{
		vbar->show();
	}
	vbar->setMinimum(0);
	vbar->setMaximum(maxLineOffset);
	vbar->setPageStep( (7*viewLines)/8 );

	if ( viewWidth >= pxLineWidth )
	{
		pxLineXScroll = 0;
		hbar->hide();
	}
	else
	{
		hbar->setPageStep( viewWidth );
		hbar->setMaximum( pxLineWidth - viewWidth );
		hbar->show();
		pxLineXScroll = hbar->value();
	}

}
//----------------------------------------------------------------------------
void QPianoRoll::mouseDoubleClickEvent(QMouseEvent * event)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int col, line, row_index, column_index, kbModifiers, alt_pressed;
	bool headerClicked, row_valid;
	QPoint c = convPixToCursor( event->pos() );

	//printf("Mouse Double Click Pressed: 0x%x (%i,%i)\n", event->button(), c.x(), c.y() );

	mouse_x = event->pos().x();
	mouse_y = event->pos().y();

	if ( c.y() >= 0 )
	{
		line = lineOffset + c.y();
		headerClicked = false;
	}
	else
	{
		line = -1;
		headerClicked = true;
	}
	col  = calcColumn( event->pos().x() );

	rowUnderMouseAtPress = rowUnderMouse = realRowUnderMouse = row_index = line;
	columnUnderMouseAtPress = columnUnderMouse = column_index = col;

	row_valid = (row_index >= 0) && ( (size_t)row_index < currMovieData.records.size() );

	kbModifiers = QApplication::keyboardModifiers();
	alt_pressed = (kbModifiers & Qt::AltModifier) ? 1 : 0;

	if ( event->button() == Qt::LeftButton )
	{
		if (col == COLUMN_ICONS)
		{
			// clicked on the "icons" column
			startDraggingPlaybackCursor();
		}
		else if ( (col == COLUMN_FRAMENUM) || (col == COLUMN_FRAMENUM2) )
		{
			//handleColumnSet( col, alt_pressed );

			// doubleclick - set Marker and start dragging it
			if (!markersManager->getMarkerAtFrame(row_index))
			{
				if (markersManager->setMarkerAtFrame(row_index))
				{
					selection->mustFindCurrentMarker = playback->mustFindCurrentMarker = true;
					history->registerMarkersChange(MODTYPE_MARKER_SET, row_index);
					update();
				}
			}
			// Delay drag event by 100ms incase the button is quickly released
			QTimer::singleShot( 100, this, SLOT(setupMarkerDrag(void)) );

			//startDraggingMarker( mouse_x, mouse_y, row_index, column_index);
		}
		else if (column_index >= COLUMN_JOYPAD1_A && column_index <= COLUMN_JOYPAD4_R)
		{
			// clicked on Input
			if (headerClicked)
			{
				drawingStartTimestamp = getTasEditorTime();
				int joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
				int button = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
				int selection_beginning = selection->getCurrentRowsSelectionBeginning();
				int selection_end       = selection->getCurrentRowsSelectionEnd();

				if ( (selection_beginning >= 0) && (selection_end >= 0) )
				{
					tasWin->toggleInput(selection_beginning, selection_end, joy, button, drawingStartTimestamp);
				}
			}
			else if (row_index >= 0)
			{
				if (!alt_pressed && !(kbModifiers & Qt::ShiftModifier))
				{
					// clicked without Shift/Alt - bring Selection cursor to this row
					selection->clearAllRowsSelection();
					selection->setRowSelection(row_index);
				}
				// toggle Input
				drawingStartTimestamp = getTasEditorTime();
				int joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
				int button = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
				int selection_beginning = selection->getCurrentRowsSelectionBeginning();
				if (alt_pressed && selection_beginning >= 0)
				{
					tasWin->setInputUsingPattern(selection_beginning, row_index, joy, button, drawingStartTimestamp);
				}
				else if ((kbModifiers & Qt::ShiftModifier) && selection_beginning >= 0)
				{
					tasWin->toggleInput(selection_beginning, row_index, joy, button, drawingStartTimestamp);
				}
				else
				{
					tasWin->toggleInput(row_index, row_index, joy, button, drawingStartTimestamp);
				}
				// and start dragging/drawing
				if (dragMode == DRAG_MODE_NONE)
				{
					if (taseditorConfig->drawInputByDragging)
					{
						// if clicked this click created buttonpress, then start painting, else start erasing
						if ( row_valid && currMovieData.records[row_index].checkBit(joy, button))
						{
							dragMode = DRAG_MODE_SET;
						}
						else
						{
							dragMode = DRAG_MODE_UNSET;
						}
					}
					else
					{
						dragMode = DRAG_MODE_OBSERVE;
					}
				}
			}
		}
	}
	else if ( event->button() == Qt::MiddleButton )
	{
		playback->handleMiddleButtonClick();
	}
	event->accept();
}
//----------------------------------------------------------------------------
void QPianoRoll::contextMenuEvent(QContextMenuEvent *event)
{
	bool drawContext, rowIsSel;

	rowIsSel = selection->isRowSelected( rowUnderMouse );

	drawContext = rowIsSel && 
		( (columnUnderMouse == COLUMN_ICONS) || (columnUnderMouse == COLUMN_FRAMENUM) || (columnUnderMouse == COLUMN_FRAMENUM2) );

	if ( !drawContext )
	{
		return;
	}
	int mkr;
	QAction *act;
	QMenu menu(this);
	FCEU_CRITICAL_SECTION( emuLock );

	mkr = markersManager->getMarkerAtFrame( rowUnderMouse );

	act = new QAction(tr("Set Markers\tDbl-Clk"), &menu);
	menu.addAction(act);
	act->setEnabled( mkr == 0 );
	//act->setShortcut(QKeySequence(tr("Double Click")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(setMarkers(void)));

	act = new QAction(tr("Remove Markers"), &menu);
	menu.addAction(act);
	act->setEnabled( mkr > 0 );
	//act->setShortcut(QKeySequence(tr("Dbl-clk")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(removeMarkers(void)));

	menu.addSeparator();

	act = new QAction(tr("Deselect"), &menu);
	menu.addAction(act);
	//act->setShortcut(QKeySequence(tr("D")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editDeselectAll(void)));

	act = new QAction(tr("Select between markers"), &menu);
	menu.addAction(act);
	act->setShortcut(QKeySequence(tr("Ctrl-A")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editSelBtwMkrs(void)));

	menu.addSeparator();

	act = new QAction(tr("Ungreenzone"), &menu);
	menu.addAction(act);
	//act->setShortcut(QKeySequence(tr("Ctrl-A")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(ungreenzoneSelectedFrames(void)));

	menu.addSeparator();

	act = new QAction(tr("Clear"), &menu);
	menu.addAction(act);
	act->setShortcut(QKeySequence(tr("Del")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editClearCB(void)));

	act = new QAction(tr("Delete"), &menu);
	menu.addAction(act);
	act->setShortcut(QKeySequence(tr("Ctrl+Del")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editDeleteCB(void)));

	act = new QAction(tr("Clone"), &menu);
	menu.addAction(act);
	act->setShortcut(QKeySequence(tr("Ctrl+Ins")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editCloneCB(void)));

	act = new QAction(tr("Insert"), &menu);
	menu.addAction(act);
	act->setShortcut(QKeySequence(tr("Ctrl+Shift+Ins")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editInsertCB(void)));

	act = new QAction(tr("Insert # of Frames"), &menu);
	menu.addAction(act);
	act->setShortcut(QKeySequence(tr("Ins")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editInsertNumFramesCB(void)));

	menu.addSeparator();

	act = new QAction(tr("Truncate Movie"), &menu);
	menu.addAction(act);
	//act->setShortcut(QKeySequence(tr("Ins")));
	connect(act, SIGNAL(triggered(void)), tasWin, SLOT(editTruncateMovieCB(void)));

	menu.exec(event->globalPos());

	event->accept();
}
//----------------------------------------------------------------------------
void QPianoRoll::mousePressEvent(QMouseEvent * event)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int col, line, row_index, column_index, kbModifiers, alt_pressed;
	bool row_valid, headerClicked;
	QPoint c = convPixToCursor( event->pos() );

	mouse_x = event->pos().x();
	mouse_y = event->pos().y();

	if ( c.y() >= 0 )
	{
		line = lineOffset + c.y();
		headerClicked = false;
	}
	else
	{
		line = -1;
		headerClicked = true;
	}
	col  = calcColumn( event->pos().x() );

	row_index = line;
	rowUnderMouseAtPress = rowUnderMouse = realRowUnderMouse = line;
	columnUnderMouseAtPress = columnUnderMouse = column_index = col;

	row_valid = (row_index >= 0) && ( (size_t)row_index < currMovieData.records.size() );

	kbModifiers = QApplication::keyboardModifiers();
	alt_pressed = (kbModifiers & Qt::AltModifier) ? 1 : 0;

	//printf("Mouse Button Pressed: 0x%x (%i,%i)\n", event->button(), c.x(), c.y() );
	
	if ( event->button() == Qt::LeftButton )
	{
		if (col == COLUMN_ICONS)
		{
			// clicked on the "icons" column
			startDraggingPlaybackCursor();
		}
		else if ( (col == COLUMN_FRAMENUM) || (col == COLUMN_FRAMENUM2) )
		{
			// clicked on the "Frame#" column
			if (row_index >= 0)
			{
				if (kbModifiers & Qt::ShiftModifier)
				{
					// select region from selection_beginning to row_index
					int selection_beginning = selection->getCurrentRowsSelectionBeginning();
					if (selection_beginning >= 0)
					{
						if (selection_beginning < row_index)
						{
							selection->setRegionOfRowsSelection(selection_beginning, row_index + 1);
						}
						else
						{
							selection->setRegionOfRowsSelection(row_index, selection_beginning + 1);
						}
					}
					startSelectingDrag(row_index);
				}
				else if (kbModifiers & Qt::AltModifier)
				{
					// make Selection by Pattern
					int selection_beginning = selection->getCurrentRowsSelectionBeginning();
					if (selection_beginning >= 0)
					{
						selection->clearAllRowsSelection();
						if (selection_beginning < row_index)
						{
							selection->setRegionOfRowsSelectionUsingPattern(selection_beginning, row_index);
						}
						else
						{
							selection->setRegionOfRowsSelectionUsingPattern(row_index, selection_beginning);
						}
					}
					if (selection->isRowSelected(row_index))
					{
						startDeselectingDrag(row_index);
					}
					else
					{
						startSelectingDrag(row_index);
					}
				}
				else if (kbModifiers & Qt::ControlModifier)
				{
					// clone current selection, so that user will be able to revert
					if (selection->getCurrentRowsSelectionSize() > 0)
					{
						selection->addCurrentSelectionToHistory();
					}
					if (selection->isRowSelected(row_index))
					{
						selection->clearSingleRowSelection(row_index);
						startDeselectingDrag(row_index);
					}
					else
					{
						selection->setRowSelection(row_index);
						startSelectingDrag(row_index);
					}
				}
				else	// just click
				{
					selection->clearAllRowsSelection();
					selection->setRowSelection(row_index);
					startSelectingDrag(row_index);
				}
			}
		}
		else if (column_index >= COLUMN_JOYPAD1_A && column_index <= COLUMN_JOYPAD4_R)
		{
			// clicked on Input
			if (headerClicked)
			{
				drawingStartTimestamp = getTasEditorTime();
				int joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
				int button = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
				int selection_beginning = selection->getCurrentRowsSelectionBeginning();
				int selection_end       = selection->getCurrentRowsSelectionEnd();

				if ( (selection_beginning >= 0) && (selection_end >= 0) )
				{
					tasWin->toggleInput(selection_beginning, selection_end, joy, button, drawingStartTimestamp);
				}
			}
			else if (row_index >= 0)
			{
				if (!alt_pressed && !(kbModifiers & Qt::ShiftModifier))
				{
					// clicked without Shift/Alt - bring Selection cursor to this row
					selection->clearAllRowsSelection();
					selection->setRowSelection(row_index);
				}
				// toggle Input
				drawingStartTimestamp = getTasEditorTime();
				int joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
				int button = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
				int selection_beginning = selection->getCurrentRowsSelectionBeginning();
				if (alt_pressed && selection_beginning >= 0)
				{
					tasWin->setInputUsingPattern(selection_beginning, row_index, joy, button, drawingStartTimestamp);
				}
				else if ((kbModifiers & Qt::ShiftModifier) && selection_beginning >= 0)
				{
					tasWin->toggleInput(selection_beginning, row_index, joy, button, drawingStartTimestamp);
				}
				else
				{
					tasWin->toggleInput(row_index, row_index, joy, button, drawingStartTimestamp);
				}
				// and start dragging/drawing
				if (dragMode == DRAG_MODE_NONE)
				{
					if (taseditorConfig->drawInputByDragging)
					{
						// if clicked this click created buttonpress, then start painting, else start erasing
						if ( row_valid && currMovieData.records[row_index].checkBit(joy, button))
						{
							dragMode = DRAG_MODE_SET;
						}
						else
						{
							dragMode = DRAG_MODE_UNSET;
						}
					}
					else
					{
						dragMode = DRAG_MODE_OBSERVE;
					}
				}
			}
		}
	}
	else if ( event->button() == Qt::MiddleButton )
	{
		playback->handleMiddleButtonClick();
	}
	else if ( event->button() == Qt::RightButton )
	{
		//rightButtonDragMode = true;
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::mouseReleaseEvent(QMouseEvent * event)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int col, line;
	QPoint c = convPixToCursor( event->pos() );

	mouse_x = event->pos().x();
	mouse_y = event->pos().y();

	if ( c.y() >= 0 )
	{
		line = lineOffset + c.y();
	}
	else
	{
		line = lineOffset;
	}
	col  = calcColumn( event->pos().x() );

	rowUnderMouse = realRowUnderMouse = line;
	columnUnderMouse = col;

	//printf("Mouse Button Released: 0x%x (%i,%i)\n", event->button(), c.x(), c.y() );
	
	if ( event->button() == Qt::LeftButton )
	{
		if (dragMode != DRAG_MODE_NONE)
		{
			// check if user released left button
			finishDrag();
		}
	}
	else if ( event->button() == Qt::RightButton )
	{
		//rightButtonDragMode = false;
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::mouseMoveEvent(QMouseEvent * event)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int col, line;
	QPoint c = convPixToCursor( event->pos() );

	mouse_x = event->pos().x();
	mouse_y = event->pos().y();

	if ( c.y() >= 0 )
	{
		line = lineOffset + c.y();
	}
	else
	{
		line = lineOffset;
	}
	col =  calcColumn( event->pos().x() );

	rowUnderMouse = realRowUnderMouse = line;
	columnUnderMouse = col;

	//printf("Mouse Move Event: 0x%x (%i,%i)  Col:%i\n", event->button(), c.x(), c.y(), col );
	
	if ( event->button() == Qt::LeftButton )
	{

	}
	updateDrag();
}
//----------------------------------------------------------------------------
void QPianoRoll::wheelEvent(QWheelEvent *event)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int ofs, kbModifiers, msButtons, zDelta = 0;

	QPoint numPixels = event->pixelDelta();
	QPoint numDegrees = event->angleDelta();

	msButtons   = QApplication::mouseButtons();
	kbModifiers = QApplication::keyboardModifiers();

	ofs = vbar->value();

	if (!numPixels.isNull())
	{
		wheelPixelCounter += numPixels.y();
		//printf("numPixels: (%i,%i) \n", numPixels.x(), numPixels.y() );

		if (wheelPixelCounter <= -pxLineSpacing)
		{
			zDelta = (wheelPixelCounter / pxLineSpacing);

			wheelPixelCounter = wheelPixelCounter % pxLineSpacing;
		}
		else if (wheelPixelCounter >= pxLineSpacing)
		{
			zDelta = (wheelPixelCounter / pxLineSpacing);

			wheelPixelCounter = wheelPixelCounter % pxLineSpacing;
		}
	}
	else if (!numDegrees.isNull())
	{
		int stepDeg = 120;
		//QPoint numSteps = numDegrees / 15;
		//printf("numSteps: (%i,%i) \n", numSteps.x(), numSteps.y() );
		//printf("numDegrees: (%i,%i)  %i\n", numDegrees.x(), numDegrees.y(), pxLineSpacing );
		wheelAngleCounter += numDegrees.y();

		if ( wheelAngleCounter <= stepDeg )
		{
			zDelta = wheelAngleCounter / stepDeg;

			wheelAngleCounter = wheelAngleCounter % stepDeg;
		}
		else if ( wheelAngleCounter >= stepDeg )
		{
			zDelta = wheelAngleCounter / stepDeg;

			wheelAngleCounter = wheelAngleCounter % stepDeg;
		}
	}
	//printf("Wheel Event: %i\n", wheelPixelCounter);

	if ( kbModifiers & Qt::ShiftModifier )
	{
		// Shift + wheel = Playback rewind full(speed)/forward full(speed)
		if (zDelta < 0)
		{
			playback->handleForwardFull( -zDelta );
		}
		else if (zDelta > 0)
		{
			playback->handleRewindFull( zDelta );
		}
	}
	else if ( kbModifiers & Qt::ControlModifier )
	{
		// Ctrl + wheel = Selection rewind full(speed)/forward full(speed)
		if (zDelta < 0)
		{
			selection->jumpToNextMarker( -zDelta );
		}
		else if (zDelta > 0)
		{
			selection->jumpToPreviousMarker( zDelta );
		}
	}
	else if ( msButtons & Qt::RightButton )
	{
		// Right button + wheel = rewind/forward Playback
		int delta = zDelta;
		if (delta < -1 || delta > 1)
		{
			delta *= PLAYBACK_WHEEL_BOOST;
		}
		int destination_frame;
		if (fceu11::IsEmulationPaused() || playback->getPauseFrame() < 0)
		{
			destination_frame = currFrameCounter - delta;
		}
		else
		{
			destination_frame = playback->getPauseFrame() - delta;
		}
		if (destination_frame < 0)
		{
			destination_frame = 0;
		}
		playback->jump(destination_frame);
	}
	else if (kbModifiers & Qt::AltModifier)
	{
		// cross gaps in Input/Markers
		if ( zDelta != 0 )
		{
			crossGaps(zDelta);
		}
	}
	else
	{
		if (zDelta > 0)
		{
			ofs -= (zDelta*6);

			if (ofs > maxLineOffset)
			{
				ofs = maxLineOffset;
			}
			vbar->setValue(ofs);
		}
		else if (zDelta < 0)
		{
			ofs -= (zDelta*6);

			if (ofs < 0)
			{
				ofs = 0;
			}
			vbar->setValue(ofs);
		}
	}

	event->accept();
}
//----------------------------------------------------------------------------
void QPianoRoll::keyPressEvent(QKeyEvent *event)
{
	//printf("Key Press: 0x%x \n", event->key() );
	// PHASE-2 搂2.3: Forward to base class first so base handlers (e.g. focus
	// traversal, shortcut activation) can act on the event before we consume
	// it for the custom piano-roll binding pipeline.
	QWidget::keyPressEvent(event);

	pushKeyEvent( event, 1 );

	event->accept();
}

void QPianoRoll::keyReleaseEvent(QKeyEvent *event)
{
	//printf("Key Release: 0x%x \n", event->key() );
	pushKeyEvent( event, 0 );

	event->accept();
}
//----------------------------------------------------------------------------
void QPianoRoll::focusInEvent(QFocusEvent *event)
{
	QWidget::focusInEvent(event);

	//printf("PianoRoll Focus In\n");

	parent->pianoRollFrame->setStyleSheet("QFrame { border: 2px solid rgb(48,140,198); }");
}
//----------------------------------------------------------------------------
void QPianoRoll::focusOutEvent(QFocusEvent *event)
{
	QWidget::focusOutEvent(event);

	//printf("PianoRoll Focus Out\n");

	parent->pianoRollFrame->setStyleSheet(NULL);
}
//----------------------------------------------------------------------------
void QPianoRoll::dragEnterEvent(QDragEnterEvent *event)
{
	if (event->mimeData()->hasUrls() )
	{
		QList<QUrl> urls = event->mimeData()->urls();
		QFileInfo fi( urls[0].toString( QUrl::PreferLocalFile ) );

		//printf("Suffix: '%s'\n", fi.suffix().toStdString().c_str() );

		if ( fi.suffix().compare("fm3") == 0)
		{
			event->acceptProposedAction();
		}
		else if ( fi.suffix().compare("fm2") == 0 )
		{
			event->acceptProposedAction();
		}
	}
	else
	{
		if ( event->source() == this )
		{
			event->acceptProposedAction();
		}
	}
}

//----------------------------------------------------------------------------
void QPianoRoll::dropEvent(QDropEvent *event)
{
	if (event->mimeData()->hasUrls() )
	{
		QList<QUrl> urls = event->mimeData()->urls();
		QFileInfo fi( urls[0].toString( QUrl::PreferLocalFile ) );

		if ( fi.suffix().compare("fm3") == 0 )
		{
			FCEU_WRAPPER_LOCK();
			tasWin->loadProject( fi.filePath().toStdString().c_str() );
			FCEU_WRAPPER_UNLOCK();
			event->accept();
		}
		else if ( fi.suffix().compare("fm2") == 0 )
		{
			FCEU_WRAPPER_LOCK();
			tasWin->importMovieFile( fi.filePath().toStdString().c_str() );
			FCEU_WRAPPER_UNLOCK();
			event->accept();
		}
	}
}
//----------------------------------------------------------------------------
bool QPianoRoll::checkIfTheresAnIconAtFrame(int frame)
{
	if (frame == currFrameCounter)
		return true;
	if (frame == playback->getLastPosition())
		return true;
	if (frame == playback->getPauseFrame())
		return true;
	if (bookmarks->findBookmarkAtFrame(frame) >= 0)
		return true;
	return false;
}
//----------------------------------------------------------------------------
void QPianoRoll::crossGaps(int zDelta)
{
	int row_index = rowUnderMouse;
	int column_index = columnUnderMouse;

	if (row_index >= 0 && column_index >= COLUMN_ICONS && column_index <= COLUMN_FRAMENUM2)
	{
		if (column_index == COLUMN_ICONS)
		{
			// cross gaps in Icons
			if (zDelta < 0)
			{
				// search down
				int last_frame = currMovieData.getNumRecords() - 1;
				if (row_index < last_frame)
				{
					int frame = row_index + 1;
					bool result_of_closest_frame = checkIfTheresAnIconAtFrame(frame);
					while ((++frame) <= last_frame)
					{
						if (checkIfTheresAnIconAtFrame(frame) != result_of_closest_frame)
						{
							// found different result, so we crossed the gap
							//ListView_Scroll(hwndList, 0, listRowHeight * (frame - row_index));
							centerListAroundLine(frame);
							break;
						}
					}
				}
			}
			else
			{
				// search up
				int first_frame = 0;
				if (row_index > first_frame)
				{
					int frame = row_index - 1;
					bool result_of_closest_frame = checkIfTheresAnIconAtFrame(frame);
					while ((--frame) >= first_frame)
					{
						if (checkIfTheresAnIconAtFrame(frame) != result_of_closest_frame)
						{
							// found different result, so we crossed the gap
							//ListView_Scroll(hwndList, 0, listRowHeight * (frame - row_index));
							centerListAroundLine(frame);
							break;
						}
					}
				}
			}
		}
		else if (column_index == COLUMN_FRAMENUM || column_index == COLUMN_FRAMENUM2)
		{
			// cross gaps in Markers
			if (zDelta < 0)
			{
				// search down
				int last_frame = currMovieData.getNumRecords() - 1;
				if (row_index < last_frame)
				{
					int frame = row_index + 1;
					bool result_of_closest_frame = (markersManager->getMarkerAtFrame(frame) != 0);
					while ((++frame) <= last_frame)
					{
						if ((markersManager->getMarkerAtFrame(frame) != 0) != result_of_closest_frame)
						{
							// found different result, so we crossed the gap
							//ListView_Scroll(hwndList, 0, listRowHeight * (frame - row_index));
							centerListAroundLine(frame);
							break;
						}
					}
				}
			}
			else
			{
				// search up
				int first_frame = 0;
				if (row_index > first_frame)
				{
					int frame = row_index - 1;
					bool result_of_closest_frame = (markersManager->getMarkerAtFrame(frame) != 0);
					while ((--frame) >= first_frame)
					{
						if ((markersManager->getMarkerAtFrame(frame) != 0) != result_of_closest_frame)
						{
							// found different result, so we crossed the gap
							//ListView_Scroll(hwndList, 0, listRowHeight * (frame - row_index));
							centerListAroundLine(frame);
							break;
						}
					}
				}
			}
		}
		else
		{
			// cross gaps in Input
			int joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
			int button = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
			if (zDelta < 0)
			{
				// search down
				int last_frame = currMovieData.getNumRecords() - 1;
				if (row_index < last_frame)
				{
					int frame = row_index + 1;
					bool result_of_closest_frame = currMovieData.records[frame].checkBit(joy, button);
					while ((++frame) <= last_frame)
					{
						if (currMovieData.records[frame].checkBit(joy, button) != result_of_closest_frame)
						{
							// found different result, so we crossed the gap
							//ListView_Scroll(hwndList, 0, listRowHeight * (frame - row_index));
							centerListAroundLine(frame);
							break;
						}
					}
				}
			}
			else
			{
				// search up
				int first_frame = 0;
				if (row_index > first_frame)
				{
					int frame = row_index - 1;
					bool result_of_closest_frame = currMovieData.records[frame].checkBit(joy, button);
					while ((--frame) >= first_frame)
					{
						if (currMovieData.records[frame].checkBit(joy, button) != result_of_closest_frame)
						{
							// found different result, so we crossed the gap
							//ListView_Scroll(hwndList, 0, listRowHeight * (frame - row_index));
							centerListAroundLine(frame);
							break;
						}
					}
				}
			}
		}
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::updateDrag(void)
{
	int kbModifiers, altHeld;

	if ( dragMode == DRAG_MODE_NONE )
	{
		return;
	}
	kbModifiers = QApplication::keyboardModifiers();

	altHeld = (kbModifiers & Qt::AltModifier) ? 1 : 0;

	// perform drag
	switch (dragMode)
	{
		case DRAG_MODE_PLAYBACK:
		{
			handlePlaybackCursorDragging();
			break;
		}
		case DRAG_MODE_MARKER:
		{
			// if suddenly source frame lost its Marker, abort drag
			if (!markersManager->getMarkerAtFrame(markerDragFrameNumber))
			{
				//if (hwndMarkerDragBox)
				//{
				//	DestroyWindow(hwndMarkerDragBox);
				//	hwndMarkerDragBox = 0;
				//}
				setCursor( Qt::ArrowCursor );
				dragMode = DRAG_MODE_NONE;
				break;
			}
			// when dragging, always show semi-transparent yellow rectangle under mouse
			//POINT p = {0, 0};
			//GetCursorPos(&p);
			//markerDragBoxX = p.x - markerDragBoxDX;
			//markerDragBoxY = p.y - markerDragBoxDY;
			//if (!hwndMarkerDragBox)
			//{
			//	hwndMarkerDragBox = CreateWindowEx(WS_EX_LAYERED | WS_EX_TRANSPARENT, markerDragBoxClassName, markerDragBoxClassName, WS_POPUP, markerDragBoxX, markerDragBoxY, COLUMN_FRAMENUM_WIDTH, listRowHeight, taseditorWindow.hwndTASEditor, NULL, fceu_hInstance, NULL);
			//	ShowWindow(hwndMarkerDragBox, SW_SHOWNA);
			//} else
			//{
			//	SetWindowPos(hwndMarkerDragBox, 0, markerDragBoxX, markerDragBoxY, 0, 0, SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE);
			//}
			//SetLayeredWindowAttributes(hwndMarkerDragBox, 0, MARKER_DRAG_BOX_ALPHA, LWA_ALPHA);
			//UpdateLayeredWindow(hwndMarkerDragBox, 0, 0, 0, 0, 0, 0, &blend, ULW_ALPHA);
			break;
		}
		case DRAG_MODE_SET:
		case DRAG_MODE_UNSET:
		{
			//ScreenToClient(hwndList, &p);
			//int drawing_current_x = p.x + GetScrollPos(hwndList, SB_HORZ);
			//int drawing_current_y = p.y + GetScrollPos(hwndList, SB_VERT) * listRowHeight;
			//// draw (or erase) line from [drawing_current_x, drawing_current_y] to (drawing_last_x, drawing_last_y)
			//int total_dx = drawingLastX - drawing_current_x, total_dy = drawingLastY - drawing_current_y;
			//if (!shiftHeld)
			//{
			//	// when user is not holding Shift, draw only vertical lines
			//	total_dx = 0;
			//	drawing_current_x = drawingLastX;
			//	p.x = drawing_current_x - GetScrollPos(hwndList, SB_HORZ);
			//}
			//LVHITTESTINFO info;
			int row_index, column_index, joy, bit;
			int min_row_index = currMovieData.getNumRecords(), max_row_index = -1;
			bool changes_made = false;
			if (altHeld)
			{
				// special mode: draw pattern
				int selection_beginning = selection->getCurrentRowsSelectionBeginning();
				if (selection_beginning >= 0)
				{
					// perform hit test
					row_index = rowUnderMouse;
					// pad movie size if user tries to draw pattern below Piano Roll limit
					if (row_index >= currMovieData.getNumRecords())
					{
						currMovieData.insertEmpty(-1, row_index + 1 - currMovieData.getNumRecords());
					}
					column_index = columnUnderMouseAtPress;

					if (row_index >= 0 && column_index >= COLUMN_JOYPAD1_A && column_index <= COLUMN_JOYPAD4_R)
					{
						joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
						bit = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
						tasWin->setInputUsingPattern(selection_beginning, row_index, joy, bit, drawingStartTimestamp);
					}
				}
			}
			else
			{
				row_index = rowUnderMouseAtPress;

				while (row_index != rowUnderMouse)
				{
					// perform hit test
					//row_index = rowUnderMouse;
					if ( row_index < 0 )
					{
						break;
					}
					// pad movie size if user tries to draw below Piano Roll limit
					if (row_index >= currMovieData.getNumRecords())
					{
						currMovieData.insertEmpty(-1, row_index + 1 - currMovieData.getNumRecords());
					}
					column_index = columnUnderMouseAtPress;

					if (row_index >= 0 && column_index >= COLUMN_JOYPAD1_A && column_index <= COLUMN_JOYPAD4_R)
					{
						joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
						bit = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
						if (dragMode == DRAG_MODE_SET && !currMovieData.records[row_index].checkBit(joy, bit))
						{
							currMovieData.records[row_index].setBit(joy, bit);
							changes_made = true;
							if (min_row_index > row_index) min_row_index = row_index;
							if (max_row_index < row_index) max_row_index = row_index;
						}
						else if (dragMode == DRAG_MODE_UNSET && currMovieData.records[row_index].checkBit(joy, bit))
						{
							currMovieData.records[row_index].clearBit(joy, bit);
							changes_made = true;
							if (min_row_index > row_index) min_row_index = row_index;
							if (max_row_index < row_index) max_row_index = row_index;
						}
					}
					if ( row_index < rowUnderMouse )
					{
						row_index++;
					}
					else if ( row_index > rowUnderMouse )
					{
						row_index--;
					}
				}
				// pad movie size if user tries to draw below Piano Roll limit
				if (row_index >= currMovieData.getNumRecords())
				{
					currMovieData.insertEmpty(-1, row_index + 1 - currMovieData.getNumRecords());
				}
				column_index = columnUnderMouseAtPress;

				if (row_index >= 0 && column_index >= COLUMN_JOYPAD1_A && column_index <= COLUMN_JOYPAD4_R)
				{
					joy = (column_index - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
					bit = (column_index - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
					if (dragMode == DRAG_MODE_SET && !currMovieData.records[row_index].checkBit(joy, bit))
					{
						currMovieData.records[row_index].setBit(joy, bit);
						changes_made = true;
						if (min_row_index > row_index) min_row_index = row_index;
						if (max_row_index < row_index) max_row_index = row_index;
					}
					else if (dragMode == DRAG_MODE_UNSET && currMovieData.records[row_index].checkBit(joy, bit))
					{
						currMovieData.records[row_index].clearBit(joy, bit);
						changes_made = true;
						if (min_row_index > row_index) min_row_index = row_index;
						if (max_row_index < row_index) max_row_index = row_index;
					}
				}
				if (changes_made)
				{
					if (dragMode == DRAG_MODE_SET)
					{
						greenzone->invalidateAndUpdatePlayback(history->registerChanges(MODTYPE_SET, min_row_index, max_row_index, 0, NULL, drawingStartTimestamp));
					}
					else
					{
						greenzone->invalidateAndUpdatePlayback(history->registerChanges(MODTYPE_UNSET, min_row_index, max_row_index, 0, NULL, drawingStartTimestamp));
					}
				}
			}
			break;
		}
		case DRAG_MODE_SELECTION:
		{
			int new_drag_selection_ending_frame = realRowUnderMouse;
			// if trying to select above Piano Roll, select from frame 0
			if (new_drag_selection_ending_frame < 0)
				new_drag_selection_ending_frame = 0;
			else if (new_drag_selection_ending_frame >= currMovieData.getNumRecords())
				new_drag_selection_ending_frame = currMovieData.getNumRecords() - 1;
			if (new_drag_selection_ending_frame >= 0 && new_drag_selection_ending_frame != dragSelectionEndingFrame)
			{
				// change Selection shape
				if (new_drag_selection_ending_frame >= dragSelectionStartingFrame)
				{
					// selecting from upper to lower
					if (dragSelectionEndingFrame < dragSelectionStartingFrame)
					{
						selection->clearRegionOfRowsSelection(dragSelectionEndingFrame, dragSelectionStartingFrame);
						selection->setRegionOfRowsSelection(dragSelectionStartingFrame, new_drag_selection_ending_frame + 1);
					} else	// both ending_frame and new_ending_frame are >= starting_frame
					{
						if (dragSelectionEndingFrame > new_drag_selection_ending_frame)
							selection->clearRegionOfRowsSelection(new_drag_selection_ending_frame + 1, dragSelectionEndingFrame + 1);
						else
							selection->setRegionOfRowsSelection(dragSelectionEndingFrame + 1, new_drag_selection_ending_frame + 1);
					}
				} else
				{
					// selecting from lower to upper
					if (dragSelectionEndingFrame > dragSelectionStartingFrame)
					{
						selection->clearRegionOfRowsSelection(dragSelectionStartingFrame + 1, dragSelectionEndingFrame + 1);
						selection->setRegionOfRowsSelection(new_drag_selection_ending_frame, dragSelectionStartingFrame);
					} else	// both ending_frame and new_ending_frame are <= starting_frame
					{
						if (dragSelectionEndingFrame < new_drag_selection_ending_frame)
							selection->clearRegionOfRowsSelection(dragSelectionEndingFrame, new_drag_selection_ending_frame);
						else
							selection->setRegionOfRowsSelection(new_drag_selection_ending_frame, dragSelectionEndingFrame);
					}
				}
				dragSelectionEndingFrame = new_drag_selection_ending_frame;
			}
			break;
		}
		case DRAG_MODE_DESELECTION:
		{
			int new_drag_selection_ending_frame = realRowUnderMouse;
			// if trying to deselect above Piano Roll, deselect from frame 0
			if (new_drag_selection_ending_frame < 0)
				new_drag_selection_ending_frame = 0;
			else if (new_drag_selection_ending_frame >= currMovieData.getNumRecords())
				new_drag_selection_ending_frame = currMovieData.getNumRecords() - 1;
			if (new_drag_selection_ending_frame >= 0 && new_drag_selection_ending_frame != dragSelectionEndingFrame)
			{
				// change Deselection shape
				if (new_drag_selection_ending_frame >= dragSelectionStartingFrame)
				{
					// deselecting from upper to lower
					selection->clearRegionOfRowsSelection(dragSelectionStartingFrame, new_drag_selection_ending_frame + 1);
				}
				else
				{
					// deselecting from lower to upper
					selection->clearRegionOfRowsSelection(new_drag_selection_ending_frame, dragSelectionStartingFrame + 1);
				}
				dragSelectionEndingFrame = new_drag_selection_ending_frame;
			}
			break;
		}
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::handleColumnSet(int column, bool altPressed)
{
	if (column == COLUMN_FRAMENUM || column == COLUMN_FRAMENUM2)
	{
		// user clicked on "Frame#" - apply ColumnSet to Markers
		if (altPressed)
		{
			if (parent->handleColumnSetUsingPattern())
			{
				setLightInHeaderColumn(COLUMN_FRAMENUM, HEADER_LIGHT_MAX);
			}
		}
		else
		{
			if (parent->handleColumnSet())
			{
				setLightInHeaderColumn(COLUMN_FRAMENUM, HEADER_LIGHT_MAX);
			}
		}
	}
	else
	{
		// user clicked on Input column - apply ColumnSet to Input
		int joy = (column - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS;
		int button = (column - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS;
		if (altPressed)
		{
			if (parent->handleInputColumnSetUsingPattern(joy, button))
			{
				setLightInHeaderColumn(column, HEADER_LIGHT_MAX);
			}
		}
		else
		{
			if (parent->handleInputColumnSet(joy, button))
			{
				setLightInHeaderColumn(column, HEADER_LIGHT_MAX);
			}
		}
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::periodicUpdate(void)
{
	// scroll Piano Roll if user is dragging cursor outside
	if ( (dragMode != DRAG_MODE_NONE) || rightButtonDragMode)
	{
		int x, y, line, col, scroll_up_threshold, scroll_down_threshold;
		QPoint p, c;

		x = mouse_x;
		y = mouse_y;

		p.setX(x);
		p.setY(y);

		c = convPixToCursor(p);

		if ( c.y() >= 0 )
		{
			line = lineOffset + c.y();
		}
		else
		{
			line = lineOffset;
		}
		col =  calcColumn( c.x() );

		rowUnderMouse = realRowUnderMouse = line;
		columnUnderMouse = col;

		scroll_up_threshold = pxLineSpacing;
		scroll_down_threshold = (viewHeight - pxLineSpacing/2);

		//if (dragMode != DRAG_MODE_MARKER)		// in DRAG_MODE_MARKER user can't scroll Piano Roll horizontally
		//{
		//	if (p.x < DRAG_SCROLLING_BORDER_SIZE)
		//		scroll_dx = p.x - DRAG_SCROLLING_BORDER_SIZE;
		//	else if (p.x > (wrect.right - wrect.left - DRAG_SCROLLING_BORDER_SIZE))
		//		scroll_dx = p.x - (wrect.right - wrect.left - DRAG_SCROLLING_BORDER_SIZE);
		//}
		if (y < scroll_up_threshold )
		{
			scroll_y += (scroll_up_threshold - y);
			
			if ( scroll_y > pxLineSpacing )
			{
				int d, v = vbar->value();
				
				d = scroll_y / pxLineSpacing;

				v -= d; scroll_y = 0;

				if ( v < 0 )
				{
					v = 0;
				}
				else if ( v > maxLineOffset )
				{
					v = maxLineOffset;
				}
				vbar->setValue(v);
			}
		}
		else if (y > scroll_down_threshold)
		{
			scroll_y += (scroll_down_threshold - y);

			if ( scroll_y < -pxLineSpacing )
			{
				int d, v = vbar->value();
				
				d = scroll_y / pxLineSpacing;

				v -= d; scroll_y = 0;

				if ( v < 0 )
				{
					v = 0;
				}
				else if ( v > maxLineOffset )
				{
					v = maxLineOffset;
				}
				vbar->setValue(v);
			}
		}
	}
	else
	{
		scroll_x = scroll_y = 0;
	}

	updateDrag();

	// once per 40 milliseconds update colors alpha in the Header
	if (getTasEditorTime() > nextHeaderUpdateTime)
	{
		nextHeaderUpdateTime = getTasEditorTime() + HEADER_LIGHT_UPDATE_TICK;
		bool changes_made = false;
		int light_value = 0;
		// 1 - update Frame# columns' heads
		//if (GetAsyncKeyState(VK_MENU) & 0x8000) light_value = HEADER_LIGHT_HOLD; else
		if (dragMode == DRAG_MODE_NONE && (headerItemUnderMouse == COLUMN_FRAMENUM || headerItemUnderMouse == COLUMN_FRAMENUM2))
		{
			light_value = (selection->getCurrentRowsSelectionSize() > 0) ? HEADER_LIGHT_MOUSEOVER_SEL : HEADER_LIGHT_MOUSEOVER;
		}
		if (headerColors[COLUMN_FRAMENUM] < light_value)
		{
			headerColors[COLUMN_FRAMENUM]++;
			changes_made = true;
		}
		else if (headerColors[COLUMN_FRAMENUM] > light_value)
		{
			headerColors[COLUMN_FRAMENUM]--;
			changes_made = true;
		}
		headerColors[COLUMN_FRAMENUM2] = headerColors[COLUMN_FRAMENUM];
		// 2 - update Input columns' heads
		int i = numColumns-1;
		if (i == COLUMN_FRAMENUM2) i--;
		for (; i >= COLUMN_JOYPAD1_A; i--)
		{
			light_value = 0;
			if (recorder->currentJoypadData[(i - COLUMN_JOYPAD1_A) / NUM_JOYPAD_BUTTONS] & (1 << ((i - COLUMN_JOYPAD1_A) % NUM_JOYPAD_BUTTONS)))
			{
				light_value = HEADER_LIGHT_HOLD;
			}
			else if (dragMode == DRAG_MODE_NONE && headerItemUnderMouse == i)
			{
				light_value = (selection->getCurrentRowsSelectionSize() > 0) ? HEADER_LIGHT_MOUSEOVER_SEL : HEADER_LIGHT_MOUSEOVER;
			}

			if (headerColors[i] < light_value)
			{
				headerColors[i]++;
				changes_made = true;
			}
			else if (headerColors[i] > light_value)
			{
				headerColors[i]--;
				changes_made = true;
			}
		}
		// 3 - redraw
		if (changes_made)
		{
			update();
		}
	}

}
//----------------------------------------------------------------------------
void QPianoRoll::setLightInHeaderColumn(int column, int level)
{
	if (column < COLUMN_FRAMENUM || column >= numColumns || level < 0 || level > HEADER_LIGHT_MAX)
	{
		return;
	}

	if (headerColors[column] != level)
	{
		headerColors[column] = level;
		//redrawHeader();
		nextHeaderUpdateTime = getTasEditorTime() + HEADER_LIGHT_UPDATE_TICK;
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::followSelection(void)
{
	RowsSelection* current_selection = selection->getCopyOfCurrentRowsSelection();
	if (current_selection->size() == 0) return;

	int list_items = viewLines - 1;
	int selection_start = *current_selection->begin();
	int selection_end = *current_selection->rbegin();
	int selection_items = 1 + selection_end - selection_start;
	
	if (selection_items <= list_items)
	{
		// selected region can fit in screen
		int lower_border = (list_items - selection_items) / 2;
		int upper_border = (list_items - selection_items) - lower_border;
		int index = selection_end + lower_border;
		if (index >= currMovieData.getNumRecords())
		{
			index = currMovieData.getNumRecords()-1;
		}
		ensureTheLineIsVisible(index);

		index = selection_start - upper_border;
		if (index < 0)
		{
			index = 0;
		}
		ensureTheLineIsVisible(index);
	}
	else
	{
		// selected region is too big to fit in screen
		// oh well, just center at selection_start
		centerListAroundLine(selection_start);
	}
}

//----------------------------------------------------------------------------
void QPianoRoll::followMarker(int markerID)
{
	if (markerID > 0)
	{
		int frame = markersManager->getMarkerFrameNumber(markerID);
		if (frame >= 0)
		{
			centerListAroundLine(frame);
		}
	}
	else
	{
		ensureTheLineIsVisible(0);
	}
}

//----------------------------------------------------------------------------
void QPianoRoll::followPlaybackCursor(void)
{
	centerListAroundLine(currFrameCounter);
}
//----------------------------------------------------------------------------
void QPianoRoll::followPlaybackCursorIfNeeded(bool followPauseframe)
{
	if (taseditorConfig->followPlaybackCursor)
	{
		if (playback->getPauseFrame() < 0)
		{
			ensureTheLineIsVisible( currFrameCounter );
		}
		else if (followPauseframe)
		{
			ensureTheLineIsVisible( playback->getPauseFrame() );
		}
	}
}

//----------------------------------------------------------------------------
void QPianoRoll::followPauseframe(void)
{
	if (playback->getPauseFrame() >= 0)
	{
		centerListAroundLine(playback->getPauseFrame());
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::followUndoHint(void)
{
	int keyframe = history->getUndoHint();
	if (taseditorConfig->followUndoContext && keyframe >= 0)
	{
		if (!lineIsVisible(keyframe))
		{
			centerListAroundLine(keyframe);
		}
	}
}
//----------------------------------------------------------------------------
void QPianoRoll::centerListAroundLine(int rowIndex)
{
	int numItemsPerPage = viewLines - 1;
	int lowerBorder = (numItemsPerPage - 1) / 2;
	int upperBorder = (numItemsPerPage - 1) - lowerBorder;
	int index = rowIndex + lowerBorder;
	if (index >= currMovieData.getNumRecords())
	{
		index = currMovieData.getNumRecords()-1;
	}
	ensureTheLineIsVisible(index);

	index = rowIndex - upperBorder;
	if (index < 0)
	{
		index = 0;
	}
	ensureTheLineIsVisible(index);
}
//----------------------------------------------------------------------------

void QPianoRoll::startDraggingPlaybackCursor(void)
{
	if (dragMode == DRAG_MODE_NONE)
	{
		dragMode = DRAG_MODE_PLAYBACK;
		// call it once
		handlePlaybackCursorDragging();
	}
}
void QPianoRoll::setupMarkerDrag(void)
{
	if ( QApplication::mouseButtons() & Qt::LeftButton )
	{
		startDraggingMarker( mouse_x, mouse_y, rowUnderMouseAtPress, columnUnderMouseAtPress);
	}
	else
	{
		tasWin->lowerMarkerNote->setFocus();
	}
}

void QPianoRoll::startDraggingMarker(int mouseX, int mouseY, int rowIndex, int columnIndex)
{
	if (dragMode == DRAG_MODE_NONE)
	{
		QColor bgColor = (taseditorConfig->bindMarkersToInput) ? QColor( BINDMARKED_FRAMENUM_COLOR ) : QColor( MARKED_FRAMENUM_COLOR );

		QSize iconSize(pxWidthFrameCol, pxLineSpacing);

		mkrDrag = new markerDragPopup(this);
		mkrDrag->resize( iconSize );
		mkrDrag->setInitialPosition( QCursor::pos() );

		mkrDrag->setBgColor( bgColor );
		mkrDrag->setRowIndex( rowIndex );

		font.setItalic(true);
		font.setBold(false);
		mkrDrag->setFont(font);
		font.setItalic(false);
		font.setBold(true);

		// start dragging the Marker
		dragMode = DRAG_MODE_MARKER;
		markerDragFrameNumber = rowIndex;
		markerDragCountdown = MARKER_DRAG_COUNTDOWN_MAX;
		setCursor( Qt::ClosedHandCursor );

		connect(mkrDrag, &QDialog::destroyed, this, [=, this]()
		{
			if ( mkrDrag == sender() )
			{
				//printf("Drag Destroyed\n");
				mkrDrag = NULL;
			}
		});
		
		mkrDrag->show();

		update();
	}
}
void QPianoRoll::startSelectingDrag(int start_frame)
{
	if (dragMode == DRAG_MODE_NONE)
	{
		dragMode = DRAG_MODE_SELECTION;
		dragSelectionStartingFrame = start_frame;
		dragSelectionEndingFrame = dragSelectionStartingFrame;	// assuming that start_frame is already selected
	}
}
void QPianoRoll::startDeselectingDrag(int start_frame)
{
	if (dragMode == DRAG_MODE_NONE)
	{
		dragMode = DRAG_MODE_DESELECTION;
		dragSelectionStartingFrame = start_frame;
		dragSelectionEndingFrame = dragSelectionStartingFrame;	// assuming that start_frame is already deselected
	}
}

void QPianoRoll::handlePlaybackCursorDragging(void)
{
	int target_frame = realRowUnderMouse;
	if (target_frame < 0)
	{
		target_frame = 0;
	}
	if (currFrameCounter != target_frame)
	{
		playback->jump(target_frame);
	}
}

void QPianoRoll::finishDrag(void)
{
	switch (dragMode)
	{
		case DRAG_MODE_MARKER:
		{
			// place Marker here
			if (markersManager->getMarkerAtFrame(markerDragFrameNumber))
			{
				//POINT p = {0, 0};
				//GetCursorPos(&p);
				//int mouse_x = p.x, mouse_y = p.y;
				//ScreenToClient(hwndList, &p);
				//RECT wrect;
				//GetClientRect(hwndList, &wrect);
				if (mouse_x < 0 || mouse_x > viewWidth || mouse_y < 0 || mouse_y > viewHeight)
				{
					// user threw the Marker away
					markersManager->removeMarkerFromFrame(markerDragFrameNumber);
					//redrawRow(markerDragFrameNumber);
					history->registerMarkersChange(MODTYPE_MARKER_REMOVE, markerDragFrameNumber);
					selection->mustFindCurrentMarker = playback->mustFindCurrentMarker = true;
					// calculate vector of movement
					if ( mkrDrag )
					{
						mkrDrag->throwAway();
					}
				}
				else
				{
					if (rowUnderMouse >= 0 && (columnUnderMouse <= COLUMN_FRAMENUM || columnUnderMouse >= COLUMN_FRAMENUM2))
					{
						if (rowUnderMouse == markerDragFrameNumber)
						{
							// it was just double-click and release
							// if Selection points at dragged Marker, set focus to lower Note edit field
							int dragged_marker_id = markersManager->getMarkerAtFrame(markerDragFrameNumber);
							int selection_marker_id = markersManager->getMarkerAboveFrame(selection->getCurrentRowsSelectionBeginning());
							if (dragged_marker_id == selection_marker_id)
							{
								//SetFocus(selection.hwndSelectionMarkerEditField);
								// select all text in case user wants to overwrite it
								//SendMessage(selection.hwndSelectionMarkerEditField, EM_SETSEL, 0, -1); 
								tasWin->lowerMarkerNote->setFocus();
							}
						}
						else if (markersManager->getMarkerAtFrame(rowUnderMouse))
						{
							int dragged_marker_id = markersManager->getMarkerAtFrame(markerDragFrameNumber);
							int destination_marker_id = markersManager->getMarkerAtFrame(rowUnderMouse);
							// swap Notes of these Markers
							char dragged_marker_note[MAX_NOTE_LEN];
							FCEU_strlcpy(dragged_marker_note, sizeof(dragged_marker_note), markersManager->getNoteCopy(dragged_marker_id).c_str());
							if (strcmp(markersManager->getNoteCopy(destination_marker_id).c_str(), dragged_marker_note))
							{
								// notes are different, swap them
								markersManager->setNote(dragged_marker_id, markersManager->getNoteCopy(destination_marker_id).c_str());
								markersManager->setNote(destination_marker_id, dragged_marker_note);
								history->registerMarkersChange(MODTYPE_MARKER_SWAP, markerDragFrameNumber, rowUnderMouse);
								selection->mustFindCurrentMarker = playback->mustFindCurrentMarker = true;
								setLightInHeaderColumn(COLUMN_FRAMENUM, HEADER_LIGHT_MAX);
							}
						}
						else
						{
							// move Marker
							int new_marker_id = markersManager->setMarkerAtFrame(rowUnderMouse);
							if (new_marker_id)
							{
								markersManager->setNote(new_marker_id, markersManager->getNoteCopy(markersManager->getMarkerAtFrame(markerDragFrameNumber)).c_str());
								// and delete it from old frame
								markersManager->removeMarkerFromFrame(markerDragFrameNumber);
								history->registerMarkersChange(MODTYPE_MARKER_DRAG, markerDragFrameNumber, rowUnderMouse, markersManager->getNoteCopy(markersManager->getMarkerAtFrame(rowUnderMouse)).c_str());
								selection->mustFindCurrentMarker = playback->mustFindCurrentMarker = true;
								setLightInHeaderColumn(COLUMN_FRAMENUM, HEADER_LIGHT_MAX);
								//redrawRow(rowUnderMouse);
							}
						}
						if ( mkrDrag )
						{
							mkrDrag->dropAccept();
						}
					}
					else
					{
						if ( mkrDrag )
						{
							mkrDrag->dropAbort();
						}
					}

					//redrawRow(markerDragFrameNumber);
					//if (hwndMarkerDragBox)
					//{
					//	DestroyWindow(hwndMarkerDragBox);
					//	hwndMarkerDragBox = 0;
					//}
				}
			}
			else
			{
				// abort drag
				if ( mkrDrag )
				{
					mkrDrag->dropAbort();
				}
				//if (hwndMarkerDragBox)
				//{
				//	DestroyWindow(hwndMarkerDragBox);
				//	hwndMarkerDragBox = 0;
				//}
			}
			//if ( mkrDrag )
			//{
			//	mkrDrag->done(0);
			//	mkrDrag->deleteLater();
			//}
			setCursor( Qt::ArrowCursor );
		}
		break;
	}
	dragMode = DRAG_MODE_NONE;
	//mustCheckItemUnderMouse = true;
}
//----------------------------------------------------------------------------
void QPianoRoll::paintEvent(QPaintEvent *event)
{
	FCEU_CRITICAL_SECTION( emuLock );
	int x, y, row, nrow, lineNum;
	QPainter painter(this);
	QColor /*white(255,255,255),*/ black(0,0,0), blkColor, rowTextColor, hdrGridColor;
	static const char *buttonNames[] = { "A", "B", "S", "T", "U", "D", "L", "R", NULL };
	char stmp[32];
	char rowIsSel=0;
	char rowSelArray[256];
	int numSelRows=0;
	QRect rect;

	font.setBold(true);
	painter.setFont(font);
	viewWidth  = event->rect().width();
	viewHeight = event->rect().height();

	nrow = (viewHeight / pxLineSpacing) + 1;

	if ( nrow < 1 ) nrow = 1;

	memset( rowSelArray, 0, nrow+1 );

	viewLines = nrow;

	maxLineOffset = currMovieData.records.size() - nrow + 2;

	vbar->setMinimum(0);
	vbar->setMaximum(maxLineOffset);

	if ( maxLineOffset < 0 )
	{
		vbar->hide();
		maxLineOffset = 0;
	}
	else
	{
		vbar->show();
	}

	if ( taseditorConfig->followPlaybackCursor )
	{
		lineOffset = vbar->value();

		if ( playbackCursorPos != currFrameCounter )
		{
			int lineOffsetLowerLim, lineOffsetUpperLim;

			playbackCursorPos = currFrameCounter;

			lineOffsetLowerLim = lineOffset;
			lineOffsetUpperLim = lineOffset + nrow - 2;

			if ( playbackCursorPos < lineOffsetLowerLim )
			{
				lineOffset = playbackCursorPos;
				vbar->setValue( lineOffset );
			}
			else if ( playbackCursorPos >= lineOffsetUpperLim )
			{
				lineOffset = playbackCursorPos - nrow + 3;
				if ( lineOffset < 0 )
				{
					lineOffset = 0;
				}
				vbar->setValue( lineOffset );
			}
		}
	}
	else
	{
		vbar->setValue( lineOffset );
	}

	if ( lineOffset < 0 )
	{
		lineOffset = 0;
	}
	if ( lineOffset > maxLineOffset )
	{
		lineOffset = maxLineOffset;
	}

	painter.fillRect( 0, 0, viewWidth, viewHeight, this->palette().color(QPalette::Window) );

	// Draw Title Bar
	x = -pxLineXScroll; y = 0;
	painter.fillRect( 0, 0, viewWidth, pxLineSpacing, windowColor );
	painter.setPen( black );

	//font.setBold(true);
	//painter.setFont(font);

	rect = painter.fontMetrics().boundingRect( tr("Frame#") );

	//x = -pxLineXScroll + pxFrameColX + (pxWidthFrameCol - 6*pxCharWidth) / 2;
	x = -pxLineXScroll + pxFrameColX + (pxWidthFrameCol - rect.width()) / 2;

	painter.drawText( x, pxLineTextOfs, tr("Frame#") );
	
	//rect = QRect( -pxLineXScroll + pxFrameColX, 0, pxWidthFrameCol, pxLineSpacing );
	//painter.drawText( rect, Qt::AlignCenter, tr("Frame#") );
	//painter.drawText( rect, Qt::AlignHCenter | Qt::AlignBottom, tr("Frame#") );

	//font.setBold(false);
	//painter.setFont(font);

	// Draw Grid
	painter.drawLine( -pxLineXScroll, 0, -pxLineXScroll, viewHeight );

	//x = pxFrameColX - pxLineXScroll;
	//painter.drawLine( x, 0, x, viewHeight );

	for (int i=0; i<numCtlr; i++)
	{
		x = pxFrameCtlX[i] - pxLineXScroll;

		if ( i % 2 )
		{
			painter.fillRect( x, pxLineSpacing, pxWidthCtlCol, viewHeight, this->palette().color(QPalette::AlternateBase) );
		}
	}

	y = pxLineSpacing;

	for (row=0; row<nrow; row++)
	{
		uint8_t data;

		lineNum = lineOffset + row;

		if ( static_cast<size_t>(lineNum) >= currMovieData.records.size() )
		{
			break;
		}
		int frame_lag = greenzone->lagLog.getLagInfoAtFrame(lineNum);

		rowSelArray[row] = rowIsSel = selection->isRowSelected( lineNum );

		for (int i=0; i<numCtlr; i++)
		{
			x = pxFrameCtlX[i] - pxLineXScroll;

			if ( lineNum == history->getUndoHint())
			{
				// undo hint here
				blkColor = (i%2) ? QColor(UNDOHINT_INPUT_COLOR2) : QColor(UNDOHINT_INPUT_COLOR1);
			}
			else if ( lineNum == currFrameCounter ||  lineNum == (playback->getFlashingPauseFrame() - 1))
			{
				// this is current frame
				blkColor = (i%2) ? QColor(CUR_INPUT_COLOR2) : QColor(CUR_INPUT_COLOR1);
			}
			else if ( lineNum < greenzone->getSize() )
			{
				if (!greenzone->isSavestateEmpty(lineNum))
				{
					// the frame is normal Greenzone frame
					if (frame_lag == LAGGED_YES)
					{
						blkColor = (i%2) ? QColor(LAG_INPUT_COLOR2) : QColor(LAG_INPUT_COLOR1);
					}
					else
					{
						blkColor = (i%2) ? QColor(GREENZONE_INPUT_COLOR2) : QColor(GREENZONE_INPUT_COLOR1);
					}
				}
				else if (  !greenzone->isSavestateEmpty(lineNum & EVERY16TH)
					|| !greenzone->isSavestateEmpty(lineNum & EVERY8TH)
					|| !greenzone->isSavestateEmpty(lineNum & EVERY4TH)
					|| !greenzone->isSavestateEmpty(lineNum & EVERY2ND))
				{
					// the frame is in a gap (in Greenzone tail)
					if (frame_lag == LAGGED_YES)
					{
						blkColor = (i%2) ? QColor(PALE_LAG_INPUT_COLOR2) : QColor(PALE_LAG_INPUT_COLOR1);
					}
					else
					{
						blkColor = (i%2) ? QColor(PALE_GREENZONE_INPUT_COLOR2) : QColor(PALE_GREENZONE_INPUT_COLOR1);
					}
				}
				else 
				{
					// the frame is above Greenzone tail
					if (frame_lag == LAGGED_YES)
					{
						blkColor = (i%2) ? QColor(VERY_PALE_LAG_INPUT_COLOR2) : QColor(VERY_PALE_LAG_INPUT_COLOR1);
					}
					else if (frame_lag == LAGGED_NO)
					{
						blkColor = (i%2) ? QColor(VERY_PALE_GREENZONE_INPUT_COLOR2) : QColor(VERY_PALE_GREENZONE_INPUT_COLOR1);
					}
					else
					{
						blkColor = (i%2) ? QColor(NORMAL_INPUT_COLOR2) : QColor(NORMAL_INPUT_COLOR1);
					}
				}
			}
			else
			{
				// the frame is below Greenzone head
				if (frame_lag == LAGGED_YES)
				{
					blkColor = (i%2) ? QColor(VERY_PALE_LAG_INPUT_COLOR2) : QColor(VERY_PALE_LAG_INPUT_COLOR1);
				}
				else if (frame_lag == LAGGED_NO)
				{
					blkColor = (i%2) ? QColor(VERY_PALE_GREENZONE_INPUT_COLOR2) : QColor(VERY_PALE_GREENZONE_INPUT_COLOR1);
				}
				else
				{
					blkColor = (i%2) ? QColor(NORMAL_INPUT_COLOR2) : QColor(NORMAL_INPUT_COLOR1);
				}
			}
			painter.fillRect( x, y, pxWidthCtlCol, pxLineSpacing, blkColor );
		}

		// Frame number column
		// font
		//if (markersManager.getMarkerAtFrame(lineNum))
		//	SelectObject(msg->nmcd.hdc, hMainListSelectFont);
		//else
		//	SelectObject(msg->nmcd.hdc, hMainListFont);
		// bg
		// frame number
		if (lineNum == history->getUndoHint())
		{
			// undo hint here
			if (markersManager->getMarkerAtFrame(lineNum) && (dragMode != DRAG_MODE_MARKER || markerDragFrameNumber != lineNum))
			{
				blkColor = (taseditorConfig->bindMarkersToInput) ? QColor( BINDMARKED_UNDOHINT_FRAMENUM_COLOR ) : QColor( MARKED_UNDOHINT_FRAMENUM_COLOR );
			}
			else
			{
				blkColor = QColor( UNDOHINT_FRAMENUM_COLOR );
			}
		}
		else if (lineNum == currFrameCounter || lineNum == (playback->getFlashingPauseFrame() - 1))
		{
			// this is current frame
			if (markersManager->getMarkerAtFrame(lineNum) && (dragMode != DRAG_MODE_MARKER || markerDragFrameNumber != lineNum))
			{
				blkColor = (taseditorConfig->bindMarkersToInput) ? QColor( CUR_BINDMARKED_FRAMENUM_COLOR ) : QColor( CUR_MARKED_FRAMENUM_COLOR );
			}
			else
			{
				blkColor = QColor( CUR_FRAMENUM_COLOR );
			}
		}
		else if (markersManager->getMarkerAtFrame(lineNum) && (dragMode != DRAG_MODE_MARKER || markerDragFrameNumber != lineNum))
		{
			// this is marked frame
			blkColor = (taseditorConfig->bindMarkersToInput) ? QColor( BINDMARKED_FRAMENUM_COLOR ) : QColor( MARKED_FRAMENUM_COLOR );
		}
		else if (lineNum < greenzone->getSize())
		{
			if (!greenzone->isSavestateEmpty(lineNum))
			{
				// the frame is normal Greenzone frame
				if (frame_lag == LAGGED_YES)
				{
					blkColor = QColor( LAG_FRAMENUM_COLOR );
				}
				else
				{
					blkColor = QColor( GREENZONE_FRAMENUM_COLOR );
				}
			}
			else if (!greenzone->isSavestateEmpty(lineNum & EVERY16TH)
				|| !greenzone->isSavestateEmpty(lineNum & EVERY8TH)
				|| !greenzone->isSavestateEmpty(lineNum & EVERY4TH)
				|| !greenzone->isSavestateEmpty(lineNum & EVERY2ND))
			{
				// the frame is in a gap (in Greenzone tail)
				if (frame_lag == LAGGED_YES)
				{
					blkColor = QColor( PALE_LAG_FRAMENUM_COLOR );
				}
				else
				{
					blkColor = QColor( PALE_GREENZONE_FRAMENUM_COLOR );
				}
			}
			else 
			{
				// the frame is above Greenzone tail
				if (frame_lag == LAGGED_YES)
				{
					blkColor = QColor( VERY_PALE_LAG_FRAMENUM_COLOR );
				}
				else if (frame_lag == LAGGED_NO)
				{
					blkColor = QColor( VERY_PALE_GREENZONE_FRAMENUM_COLOR );
				}
				else
				{
					blkColor = QColor( NORMAL_FRAMENUM_COLOR );
				}
			}
		}
		else
		{
			// the frame is below Greenzone head
			if (frame_lag == LAGGED_YES)
			{
				blkColor = QColor( VERY_PALE_LAG_FRAMENUM_COLOR );
			}
			else if (frame_lag == LAGGED_NO)
			{
				blkColor = QColor( VERY_PALE_GREENZONE_FRAMENUM_COLOR );
			}
			else
			{
				blkColor = QColor( NORMAL_FRAMENUM_COLOR );
			}
		}
		x = -pxLineXScroll + pxFrameColX;

		painter.fillRect( x, y, pxWidthFrameCol, pxLineSpacing, blkColor );

		// Selected Line
		if ( rowIsSel )
		{
			painter.fillRect( 0, y, viewWidth, pxLineSpacing, QColor( 10, 36, 106 ) );

			rowTextColor = QColor( 255, 255, 255 );

			numSelRows++;
		}
		else
		{
			rowTextColor = QColor( 0, 0, 0 );
		}
		painter.setPen( rowTextColor );

		for (int i=0; i<numCtlr; i++)
		{
			int ctlrOfs, btnOfs, hotChangeVal;
			data = currMovieData.records[ lineNum ].joysticks[i];

			x = pxFrameCtlX[i] - pxLineXScroll;

			ctlrOfs = i*8;

			for (int j=0; j<8; j++)
			{
				btnOfs = ctlrOfs+j;

				if (taseditorConfig->enableHotChanges)
				{
					hotChangeVal = history->getCurrentSnapshot().inputlog.getHotChangesInfo( lineNum, btnOfs );

					if ( !rowIsSel && (hotChangeVal >= 0) && (hotChangeVal < 16) )
					{
						painter.setPen( hotChangesColors[hotChangeVal] );
					}
					else
					{
						painter.setPen( rowTextColor );
					}
				}
				else
				{
					hotChangeVal = -1;
				}
				rect = QRect( x, y, pxWidthBtnCol, pxLineSpacing );

				if ( data & (0x01 << j) )
				{
					painter.drawText( x + pxCharWidth, y+pxLineTextOfs, tr(buttonNames[j]) );
					//painter.drawText( rect, Qt::AlignCenter, tr(buttonNames[j]) );
					//painter.drawText( rect, Qt::AlignHCenter | Qt::AlignBottom, tr(buttonNames[j]) );
				}
				else if ( hotChangeVal > 0 )
				{
					painter.drawText( x + pxCharWidth, y+pxLineTextOfs, tr("-") );
					//painter.drawText( rect, Qt::AlignCenter, tr("-") );
					//painter.drawText( rect, Qt::AlignHCenter | Qt::AlignBottom, tr("-") );
				}
				x += pxWidthBtnCol;
			}
			//painter.drawLine( x, y, x, pxLineSpacing );
		}

		// Frame number column
		// font
		//if (markersManager.getMarkerAtFrame(lineNum))
		//	SelectObject(msg->nmcd.hdc, hMainListSelectFont);
		//else
		//	SelectObject(msg->nmcd.hdc, hMainListFont);
		// bg
		painter.setPen( rowTextColor );

		//rect = QRect( -pxLineXScroll + pxFrameColX, y, pxWidthFrameCol, pxLineSpacing );

		snprintf(stmp, sizeof(stmp), "%07i", lineNum );

		rect = painter.fontMetrics().boundingRect( tr(stmp) );

		x = -pxLineXScroll + pxFrameColX + (pxWidthFrameCol - rect.width()) / 2;

		if (markersManager->getMarkerAtFrame(lineNum))
		{
			font.setItalic(true);
			font.setBold(false);
		}
		else
		{
			font.setBold(true);
			font.setItalic(false);
		}
		painter.setFont(font);
		painter.drawText( x, y+pxLineTextOfs, tr(stmp) );
		//painter.drawText( rect, Qt::AlignCenter, tr(stmp) );
		//painter.drawText( rect, Qt::AlignHCenter | Qt::AlignBottom, tr(stmp) );

		if ( font.italic() )
		{
			font.setBold(true);
			font.setItalic(false);
			painter.setFont(font);
		}

		x = -pxLineXScroll;

		int iImage = bookmarks->findBookmarkAtFrame(lineNum);
		if (iImage < 0)
		{
			// no bookmark at this frame
			if (lineNum == playback->getLastPosition())
			{
				if (lineNum == currFrameCounter)
				{
					iImage = GREEN_BLUE_ARROW_IMAGE_ID;
				}
				else
				{
					iImage = GREEN_ARROW_IMAGE_ID;
				}
			}
			else if (lineNum == currFrameCounter)
			{
				iImage = BLUE_ARROW_IMAGE_ID;
			}
		}
		else
		{
			// bookmark at this frame
			if (lineNum == playback->getLastPosition())
			{
				iImage |= BOOKMARKS_WITH_GREEN_ARROW;
			}
			else if (lineNum == currFrameCounter)
			{
				iImage |= BOOKMARKS_WITH_BLUE_ARROW;
			}
			else
			{
				iImage |= BOOKMARKS_WITH_NO_ARROW;
			}
		}

		if ( iImage >= 0 )
		{
			drawArrow( &painter, x, y, iImage );
		}

		y += pxLineSpacing;
	}

	int gridBlack = gridColor.black();
	hdrGridColor = gridColor;

	if ( gridBlack < 128 )
	{
		hdrGridColor = QColor(128,128,128);
	}

	// Draw Grid lines
	painter.setPen( QPen(gridColor,gridPixelWidth) );
	x = pxFrameColX - pxLineXScroll;
	painter.drawLine( x, 0, x, viewHeight );
	
	painter.setPen( QPen(hdrGridColor,gridPixelWidth) );
	painter.drawLine( x, 0, x, pxLineSpacing );

	font.setBold(true);
	font.setItalic(false);
	painter.setFont(font);


	for (int i=0; i<numCtlr; i++)
	{
		x = pxFrameCtlX[i] - pxLineXScroll;

		for (int j=0; j<8; j++)
		{
			//painter.setPen( QColor( 128, 128, 128 ) );
			//painter.drawLine( x, 0, x, viewHeight ); x++;
			painter.setPen( QPen(gridColor,gridPixelWidth) );
			painter.drawLine( x, 0, x, viewHeight ); //x--;

			painter.setPen( QPen(hdrGridColor,gridPixelWidth) );
			painter.drawLine( x, 0, x, pxLineSpacing );

			rect = QRect( x, 0, pxWidthBtnCol, pxLineSpacing );
			painter.setPen( QPen(headerLightsColors[ headerColors[COLUMN_JOYPAD1_A + (i*8) + j] ],1) );
			painter.drawText( x + pxCharWidth, pxLineTextOfs, tr(buttonNames[j]) );
			//painter.drawText( rect, Qt::AlignCenter, tr(buttonNames[j]) );
			//painter.drawText( rect, Qt::AlignHCenter | Qt::AlignBottom, tr(buttonNames[j]) );

			x += pxWidthBtnCol;
		}
		//painter.setPen( QColor( 128, 128, 128 ) );
		//painter.drawLine( x, 0, x, viewHeight ); x++;
		painter.setPen( QPen(gridColor,gridPixelWidth) );
		painter.drawLine( x, 0, x, viewHeight );

		painter.setPen( QPen(hdrGridColor,gridPixelWidth) );
		painter.drawLine( x, 0, x, pxLineSpacing );

	}
	painter.setPen( QPen(gridColor,gridPixelWidth) );

	y = 0;
	for (int i=0; i<nrow; i++)
	{
		painter.drawLine( 0, y, viewWidth, y );
		
		y += pxLineSpacing;
	}

	painter.setPen( QPen(hdrGridColor,gridPixelWidth) );
	painter.drawLine( 0, 0, viewWidth, 0 );
	painter.drawLine( 0, pxLineSpacing, viewWidth, pxLineSpacing );

	// Draw grid lines for selections
	if ( numSelRows > 0 )
	{
		int inv;
		QColor invGrid;

		inv = gridColor.black();

		if ( inv < 128 )
		{
			inv = 255 - inv;
		}

		invGrid.setRed( inv );
		invGrid.setGreen( inv );
		invGrid.setBlue( inv );

		painter.setPen( QPen(invGrid,gridPixelWidth) );

		y = pxLineSpacing;

		for (row=0; row<nrow; row++)
		{
			if ( rowSelArray[row] )
			{
				int yl = y + pxLineSpacing;

				x = pxFrameColX - pxLineXScroll;
				painter.drawLine( x, y, x, yl );

				for (int i=0; i<numCtlr; i++)
				{
					x = pxFrameCtlX[i] - pxLineXScroll;

					for (int j=0; j<8; j++)
					{
						painter.drawLine( x, y, x, yl );

						x += pxWidthBtnCol;
					}
				}
				painter.drawLine( x, y, x, yl );
				painter.drawLine( 0, y , viewWidth, y  );
				painter.drawLine( 0, yl, viewWidth, yl );
			}
			y += pxLineSpacing;
		}
		painter.setPen( QPen(gridColor,gridPixelWidth) );
	}

	font.setBold(false);
	painter.setFont(font);
}
//----------------------------------------------------------------------------
