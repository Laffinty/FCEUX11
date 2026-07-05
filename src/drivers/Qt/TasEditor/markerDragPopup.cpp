// markerDragPopup.cpp
//

#include <QApplication>
#include <QMouseEvent>
#include <QPainter>

#include "Qt/TasEditor/markerDragPopup.h"

markerDragPopup::markerDragPopup(QWidget *parent)
	: QDialog( parent, Qt::ToolTip )
{
	rowIndex = 0;
	alpha = 255;
	bgColor = QColor( 255,255,255 );
	liveCount = 30;

	qApp->installEventFilter(this);

	timer = new QTimer(this);

	connect( timer, &QTimer::timeout, this, &markerDragPopup::fadeAway );

	timer->start(33);

	released = false;
	thrownAway = false;
	dropAborted = false;
	dropAccepted = false;
}
//----------------------------------------------------------------------------
markerDragPopup::~markerDragPopup(void)
{

}
//----------------------------------------------------------------------------
void markerDragPopup::setRowIndex( int row )
{
	rowIndex = row;
}
//----------------------------------------------------------------------------
void markerDragPopup::setBgColor( QColor c )
{
	bgColor = c;
}
//----------------------------------------------------------------------------
void markerDragPopup::setInitialPosition( QPoint p )
{
	initialPos = p;

	move( initialPos );
}
//----------------------------------------------------------------------------
void markerDragPopup::throwAway(void)
{
	thrownAway = true;
}
//----------------------------------------------------------------------------
void markerDragPopup::dropAccept(void)
{
	dropAccepted = true;
}
//----------------------------------------------------------------------------
void markerDragPopup::dropAbort(void)
{
	dropAborted = true;
}
//----------------------------------------------------------------------------
void markerDragPopup::fadeAway(void)
{

	if ( released )
	{
		if ( thrownAway )
		{
			QPoint p = pos();
			//printf("Fade:%i\n", alpha);

			p.setY( p.y() + 2 );

			move(p);

			if ( alpha > 0 )
			{
				alpha -= 10;

				if ( alpha < 0 )
				{
					alpha = 0;
				}
			}
			else
			{
				done(0);
				deleteLater();
			}
			setWindowOpacity( alpha / 255.0f );

			update();
		}
		else if ( dropAborted )
		{
			QPoint p = pos();
			int vx, vy, vm = 10;

			vx = initialPos.x() - p.x();

			if ( vx < -vm )
			{
				vx = -vm;
			}
			else if ( vx > vm )
			{
				vx = vm;
			}

			vy = initialPos.y() - p.y();

			if ( vy < -vm )
			{
				vy = -vm;
			}
			else if ( vy > vm )
			{
				vy = vm;
			}

			p.setX( p.x() + vx );
			p.setY( p.y() + vy );

			if ( (vx == 0) && (vy == 0) )
			{
				done(0);
				deleteLater();
			}
			else
			{
				move(p);
			}
		}
		else if ( dropAccepted )
		{
			done(0);
			deleteLater();
		}
		else
		{
			if ( liveCount > 0 )
			{
				liveCount--;
			}
			if ( liveCount == 0 )
			{
				done(0);
				deleteLater();
			}
		}
	}
}
//----------------------------------------------------------------------------
void markerDragPopup::paintEvent(QPaintEvent *event)
{
	int w,h;
	QPainter painter(this);
	char txt[32];

	w = event->rect().width();
	h = event->rect().height();

	snprintf(txt, sizeof(txt), "%07i", rowIndex );

	//painter.setFont(font);
	//I want to make the title bar pasted on the content
	//But you can't get the image of the default title bar, just draw a rectangular box
	//If the external theme color is set, you need to change it
	QRect title_rect{0,0,w,h};
	painter.fillRect(title_rect,bgColor);
	painter.drawText(title_rect,Qt::AlignCenter, txt);
	//painter.drawText(title_rect,Qt::AlignHCenter | Qt::AlignBottom, txt);
	//painter.drawRect(pixmap.rect().adjusted(0,0,-1,-1));
}
//----------------------------------------------------------------------------
bool markerDragPopup::eventFilter( QObject *obj, QEvent *event)
{
	//printf("Event:%i   %p\n", event->type(), obj);
	switch (event->type() )
	{
		case QEvent::MouseMove:
		{
			if ( !released )
			{
				move( QCursor::pos() );
			}
			break;
		}
		case QEvent::MouseButtonRelease:
		{
			released = true;
			break;
		}
		default:
			// Ignore
		break;
	}
	return false;
}
