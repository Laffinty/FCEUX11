// TasEditorTimeline.h
//

#pragma once

#include <stdint.h>
#include <string>

#include <QWidget>
#include <QDialog>
#include <QScrollBar>
#include <QScrollArea>
#include <QPainter>
#include <QFont>
#include <QFrame>
#include <QMenu>
#include <QCloseEvent>
#include <QColor>

#include "Qt/config.h"

class TasEditorWindow;

enum PIANO_ROLL_COLUMNS
{
	COLUMN_ICONS,
	COLUMN_FRAMENUM,
	COLUMN_JOYPAD1_A,
	COLUMN_JOYPAD1_B,
	COLUMN_JOYPAD1_S,
	COLUMN_JOYPAD1_T,
	COLUMN_JOYPAD1_U,
	COLUMN_JOYPAD1_D,
	COLUMN_JOYPAD1_L,
	COLUMN_JOYPAD1_R,
	COLUMN_JOYPAD2_A,
	COLUMN_JOYPAD2_B,
	COLUMN_JOYPAD2_S,
	COLUMN_JOYPAD2_T,
	COLUMN_JOYPAD2_U,
	COLUMN_JOYPAD2_D,
	COLUMN_JOYPAD2_L,
	COLUMN_JOYPAD2_R,
	COLUMN_JOYPAD3_A,
	COLUMN_JOYPAD3_B,
	COLUMN_JOYPAD3_S,
	COLUMN_JOYPAD3_T,
	COLUMN_JOYPAD3_U,
	COLUMN_JOYPAD3_D,
	COLUMN_JOYPAD3_L,
	COLUMN_JOYPAD3_R,
	COLUMN_JOYPAD4_A,
	COLUMN_JOYPAD4_B,
	COLUMN_JOYPAD4_S,
	COLUMN_JOYPAD4_T,
	COLUMN_JOYPAD4_U,
	COLUMN_JOYPAD4_D,
	COLUMN_JOYPAD4_L,
	COLUMN_JOYPAD4_R,
	COLUMN_FRAMENUM2,

	TOTAL_COLUMNS
};

#define HEADER_LIGHT_MAX 10
#define HEADER_LIGHT_HOLD 5
#define HEADER_LIGHT_MOUSEOVER_SEL 3
#define HEADER_LIGHT_MOUSEOVER 0
#define HEADER_LIGHT_UPDATE_TICK  (40)

#define BOOKMARKS_WITH_NO_ARROW      0x00010000
#define BOOKMARKS_WITH_BLUE_ARROW    0x00020000
#define BOOKMARKS_WITH_GREEN_ARROW   0x00040000
#define BLUE_ARROW_IMAGE_ID          0x00080000
#define GREEN_ARROW_IMAGE_ID         0x00100000
#define GREEN_BLUE_ARROW_IMAGE_ID   (BLUE_ARROW_IMAGE_ID | GREEN_ARROW_IMAGE_ID)

#define MARKER_DRAG_COUNTDOWN_MAX 14
#define PIANO_ROLL_ID_LEN 11
#define PLAYBACK_WHEEL_BOOST 2

enum DRAG_MODES
{
	DRAG_MODE_NONE,
	DRAG_MODE_OBSERVE,
	DRAG_MODE_PLAYBACK,
	DRAG_MODE_MARKER,
	DRAG_MODE_SET,
	DRAG_MODE_UNSET,
	DRAG_MODE_SELECTION,
	DRAG_MODE_DESELECTION,
};

class markerDragPopup;

class QPianoRoll : public QWidget
{
	Q_OBJECT

	public:
		QPianoRoll(QWidget *parent = 0);
		~QPianoRoll(void);

		void reset(void);
		void save(EMUFILE *os, bool really_save);
		bool load(EMUFILE *is, unsigned int offset);
		void setScrollBars( QScrollBar *h, QScrollBar *v );

		QFont getFont(void){ return font; };

		int   getDragMode(void){ return dragMode; };

		bool  lineIsVisible( int lineNum );
		bool  checkIfTheresAnIconAtFrame(int frame);

		void  updateLinesCount(void);
		void  handleColumnSet(int column, bool altPressed);
		void  centerListAroundLine(int rowIndex);
		void  ensureTheLineIsVisible( int lineNum );
		void  followPlaybackCursorIfNeeded(bool followPauseframe);
		void  followMarker(int markerID);
		void  followSelection(void);
		void  followPlaybackCursor(void);
		void  followPauseframe(void);
		void  followUndoHint(void);
		void  setLightInHeaderColumn(int column, int level);
		void  periodicUpdate(void);

		void  setFont( QFont &font );

		QColor      gridColor;
	protected:
		void calcFontData(void);
		void resizeEvent(QResizeEvent *event) override;
		void paintEvent(QPaintEvent *event) override;
		void mousePressEvent(QMouseEvent * event) override;
		void mouseReleaseEvent(QMouseEvent * event) override;
		void mouseMoveEvent(QMouseEvent * event) override;
		void mouseDoubleClickEvent(QMouseEvent * event) override;
		void wheelEvent(QWheelEvent *event) override;
		void keyPressEvent(QKeyEvent *event) override;
		void keyReleaseEvent(QKeyEvent *event) override;
		void focusInEvent(QFocusEvent *event) override;
		void focusOutEvent(QFocusEvent *event) override;
		void contextMenuEvent(QContextMenuEvent *event) override;
		void dragEnterEvent(QDragEnterEvent *event) override;
		void dropEvent(QDropEvent *event) override;

		void crossGaps(int zDelta);
		void startDraggingPlaybackCursor(void);
		void startDraggingMarker(int mouseX, int mouseY, int rowIndex, int columnIndex);
		void startSelectingDrag(int start_frame);
		void startDeselectingDrag(int start_frame);
		void handlePlaybackCursorDragging(void);
		void finishDrag(void);
		void updateDrag(void);

		void drawArrow( QPainter *painter, int xl, int yl, int value );

		int    calcColumn( int px );
		QPoint convPixToCursor( QPoint p );

	private:
		TasEditorWindow *parent;
		QFont       font;
		QScrollBar *hbar;
		QScrollBar *vbar;
		QColor      windowColor;
		QColor      headerLightsColors[11];
		QColor      hotChangesColors[16];

		markerDragPopup *mkrDrag;

		int8_t headerColors[TOTAL_COLUMNS];

		int numCtlr;
		int numColumns;
		int pxCharWidth;
		int pxCharHeight;
		int pxCursorHeight;
		int pxLineXScroll;
		int pxLineWidth;
		int pxLineSpacing;
		int pxLineLead;
		int pxLineTextOfs;
		int pxWidthCol1;
		int pxWidthFrameCol;
		int pxWidthCtlCol;
		int pxWidthBtnCol;
		int pxFrameColX;
		int pxFrameCtlX[4];
		int viewLines;
		int viewWidth;
		int viewHeight;
		int lineOffset;
		int maxLineOffset;
		int numInputDevs;
		int dragMode;
		int dragSelectionStartingFrame;
		int dragSelectionEndingFrame;
		int realRowUnderMouse;
		int rowUnderMouse;
		int columnUnderMouse;
		int rowUnderMouseAtPress;
		int columnUnderMouseAtPress;
		int markerDragFrameNumber;
		int markerDragCountdown;
		int wheelPixelCounter;
		int wheelAngleCounter;
		int headerItemUnderMouse;
		int scroll_x;
		int scroll_y;
		int mouse_x;
		int mouse_y;
		int gridPixelWidth;
		uint64_t drawingStartTimestamp;
		uint64_t nextHeaderUpdateTime;

		int playbackCursorPos;

		bool useDarkTheme;
		bool rightButtonDragMode;

	public slots:
		void hbarChanged(int val);
		void vbarChanged(int val);
		void setupMarkerDrag(void);
};

class PianoRollScrollBar : public QScrollBar
{
	Q_OBJECT

	public:
		PianoRollScrollBar( QWidget *parent );
		~PianoRollScrollBar(void);

	protected:
		void wheelEvent(QWheelEvent *event) override;

		int wheelPixelCounter;
		int wheelAngleCounter;
		int pxLineSpacing;
};
