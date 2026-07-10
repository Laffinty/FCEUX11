// ConsoleVideoSetup.cpp
//
#include <QScreen>
#include <QWindow>
#include <QWidget>

#include "../../fceu.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/dface.h"
#include "Qt/nes_shm.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleViewerInterface.h"
#include "Qt/sdl-video.h"

int consoleWin_t::videoInit(void)
{
	int ret = 0;

	if (viewport_Interface)
	{
		ret = viewport_Interface->init();
	}
	return ret;
}

void consoleWin_t::videoReset(void)
{
	if (viewport_Interface)
	{
		viewport_Interface->reset();
	}
	return;
}

void consoleWin_t::initScreenHandler(void)
{
	if ( !scrHandlerConnected )
	{
		QWidget *w;
		
		w = this->window();

		if ( w != NULL)
		{
			QWindow *hdl = w->windowHandle();

			if (hdl != NULL)
			{
				connect( hdl, SIGNAL(screenChanged(QScreen*)), this, SLOT(winScreenChanged(QScreen*)) );
				scrHandlerConnected = true;

				winScreenChanged( hdl->screen() );

				connect( hdl, SIGNAL(activeChanged(void)), this, SLOT(winActiveChanged(void)) );
			}
		}
	}

}

void consoleWin_t::winScreenChanged(QScreen *scr)
{
	if ( scr == NULL )
	{
		return;
	}
	refreshRate = scr->refreshRate();

	if ( viewport_GL != NULL )
	{
		viewport_GL->screenChanged( scr );
	}
}

void consoleWin_t::winActiveChanged(void)
{
	QWidget *w;
	bool muteWindow = false;

	w = this->window();

	if ( w != NULL)
	{
		QWindow *hdl = w->windowHandle();

		if (hdl != NULL)
		{
			if ( !soundUseGlobalFocus )
			{
				if ( hdl->isActive() )
				{
					muteWindow = false;
				}
				else
				{
					muteWindow = true;
				}
			}
		}
	}
	FCEUD_MuteSoundWindow(muteWindow);
}

QSize consoleWin_t::calcRequiredSize(void)
{
	QSize out( GL_NES_WIDTH, GL_NES_HEIGHT );

	QSize w, v;
	double xscale = 1.0, yscale = 1.0, aspectRatio = 1.0;
	int texture_width = GL_NES_WIDTH;
	int texture_height = GL_NES_HEIGHT;
	int l=0, r=texture_width;
	int t=0, b=texture_height;
	int dw=0, dh=0, rw, rh;
	bool forceAspect = true;

	CalcVideoDimensions();

	texture_width  = nes_shm->video.ncol;
	texture_height = nes_shm->video.nrow;

	l=0, r=texture_width;
	t=0, b=texture_height;

	w = size();

	if ( viewport_Interface )
	{
		v = viewport_Interface->size();
		forceAspect = viewport_Interface->getForceAspectOpt();
		aspectRatio = viewport_Interface->getAspectRatio();
		xscale = viewport_Interface->getScaleX();
		yscale = viewport_Interface->getScaleY();
	}

	dw = 0;
	dh = 0;

	if ( forceAspect )
	{
		yscale = xscale * (double)nes_shm->video.xyRatio;
	}
	rw=(int)((r-l)*xscale);
	rh=(int)((b-t)*yscale);

	if ( forceAspect )
	{
		double rr;

		rr = (double)rh / (double)rw;

		if ( rr > aspectRatio )
		{
			rw = (int)( (((double)rh) / aspectRatio) + 0.50);
		}
		else
		{
			rh = (int)( (((double)rw) * aspectRatio) + 0.50);
		}
	}

	out.setWidth( rw + dw );
	out.setHeight( rh + dh );

	return out;
}

void consoleWin_t::setViewportAspect(void)
{
	int aspectSel;
	double x,y;

	g_config->getOption ("SDL.AspectSelect", &aspectSel);

	switch ( aspectSel )
	{
		default:
		case 0:
			x =  1.0; y = 1.0;
		break;
		case 1:
			x =  8.0; y = 7.0;
		break;
		case 2:
			x = 11.0; y = 8.0;
		break;
		case 3:
			x =  4.0; y = 3.0;
		break;
		case 4:
			x = 16.0; y = 9.0;
		break;
		case 5:
		{
			x = 1.0; y = 1.0;
		}
		break;
	}

	if (viewport_Interface)
	{
		viewport_Interface->setAspectXY( x, y );
	}
}
