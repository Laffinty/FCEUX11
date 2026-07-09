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
// GameViewerGL.cpp
//
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#ifdef WIN32
#include <windows.h>
#endif

#include <QApplication>
#include <QImage>
#include <QMatrix4x4>
#include <QMouseEvent>
#include <QOpenGLBuffer>
#include <QOpenGLShaderProgram>
#include <QOpenGLVertexArrayObject>
#include <QScreen>
#include <QWindow>

#include "Qt/nes_shm.h"
#include "Qt/throttle.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleViewerGL.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/keyscan.h"

extern unsigned int gui_draw_area_width;
extern unsigned int gui_draw_area_height;

static const char *vertexShaderSource =
    "#version 330 core\n"
    "layout(location = 0) in vec2 aPos;\n"
    "layout(location = 1) in vec2 aTexCoord;\n"
    "out vec2 vTexCoord;\n"
    "uniform mat4 uProjection;\n"
    "void main()\n"
    "{\n"
    "    gl_Position = uProjection * vec4(aPos, 0.0, 1.0);\n"
    "    vTexCoord = aTexCoord;\n"
    "}\n";

static const char *fragmentShaderSource =
    "#version 330 core\n"
    "in vec2 vTexCoord;\n"
    "out vec4 FragColor;\n"
    "uniform sampler2D uTexture;\n"
    "void main()\n"
    "{\n"
    "    FragColor = texture(uTexture, vTexCoord);\n"
    "}\n";

ConsoleViewGL_t::ConsoleViewGL_t(QWindow *parent)
	: QOpenGLWindow( QOpenGLWindow::NoPartialUpdate, parent )
{
	view_width  = 256;
	view_height = 224;
	gltexture   = 0;
	bgTexture   = 0;
	devPixRatio = 1.0f;
	aspectRatio = 1.0f;
	aspectX     = 1.0f;
	aspectY     = 1.0f;
	linearFilter = false;
	forceAspect  = true;
	autoScaleEna = true;
	xscale = 2.0;
	yscale = 2.0;
	sx = 0; sy = 0;
	rw = 256;
	rh = 240;
	txtWidth  = 0;
	txtHeight = 0;
	mouseButtonMask = 0;

	bgColor = NULL;

	if ( consoleWindow )
	{
		bgColor = consoleWindow->getVideoBgColorPtr();
		bgColor->setRgb( 30, 69, 40 );
	}
	setMinimumSize( QSize(256, 224) );
	setMaximumSize( QSize(16777215, 16777215) );

	localBufSize = (4 * GL_NES_WIDTH) * (4 * GL_NES_HEIGHT) * sizeof(uint32_t);

	localBuf = std::make_unique<uint32_t[]>(localBufSize / sizeof(uint32_t));

	if ( localBuf )
	{
		memset32( localBuf.get(), alphaMask, localBufSize );
	}

	vsyncEnabled = true;
	linearFilter = false;
	glFunctionsInitialized = false;

	if ( g_config )
	{
		int opt;
		g_config->getOption("SDL.OpenGLip", &opt );
		
		linearFilter = (opt) ? true : false;

		g_config->getOption ("SDL.AutoScale", &opt);

		autoScaleEna = (opt) ? true : false;

		g_config->getOption("SDL.XScale", &xscale);
		g_config->getOption("SDL.YScale", &yscale);

		g_config->getOption ("SDL.ForceAspect", &forceAspect);

		if ( bgColor )
		{
			fceuLoadConfigColor( "SDL.VideoBgColor", bgColor );
		}
		g_config->getOption ("SDL.VideoVsync", &vsyncEnabled);
	}

	QSurfaceFormat fmt;
	fmt.setRenderableType(QSurfaceFormat::OpenGL);
	fmt.setProfile(QSurfaceFormat::CoreProfile);
	fmt.setVersion(3, 3);
	fmt.setSwapBehavior(QSurfaceFormat::DoubleBuffer);
	fmt.setSwapInterval( vsyncEnabled ? 1 : 0 );

	setFormat(fmt);

	connect( this, SIGNAL(frameSwapped(void)), this, SLOT(renderFinished(void)) );
}

ConsoleViewGL_t::~ConsoleViewGL_t(void)
{
}

void ConsoleViewGL_t::screenChanged( QScreen *screen )
{
	int w,h;

	devPixRatio = screen->devicePixelRatio();

	w = static_cast<int>(devPixRatio * width()  );
	h = static_cast<int>(devPixRatio * height() );

	view_width  = w;
	view_height = h;

	gui_draw_area_width = w;
	gui_draw_area_height = h;

	// CRITICAL: buildTextures() invokes OpenGL functions (glGenTextures,
	// glBindTexture, glTexImage2D, ...). On Windows, calling these without a
	// current rendering context dereferences a NULL function-pointer table
	// and segfaults the process.
	//
	// In the v0.3.14 QOpenGLWindow flow, the QMainWindow's showEvent fires
	// BEFORE Qt has had a chance to make the QOpenGLWindow's GL context
	// current (initializeGL has not run yet, so glFunctionsInitialized is
	// false). The showEvent handler (consoleWin_t::initScreenHandler ->
	// winScreenChanged -> this slot) used to call buildTextures
	// unconditionally, crashing on the first glGenTextures call.
	//
	// Guard: only call buildTextures if the GL functions are initialized
	// (i.e., initializeGL has run). The screen dimensions are stored in
	// view_width/view_height above regardless, and initializeGL itself calls
	// buildTextures(), so the first-frame texture will be correctly sized.
	// Subsequent screen changes (e.g., window moved to a different monitor)
	// arrive after initializeGL has run, so this guard does not affect them.
	if (glFunctionsInitialized)
	{
		buildTextures();
	}
	else
	{
		// GL context not yet initialized. view_width/view_height are stored
		// above; initializeGL() will call buildTextures() once the context
		// is current, picking up the latest dimensions.
	}

	//printf("GL Ratio: %f  %ix%i\n", screen->devicePixelRatio(), w, h );
}

int ConsoleViewGL_t::init( void )
{
	QScreen *screen = this->screen();

	if ( screen != NULL )
	{
		devPixRatio = screen->devicePixelRatio();
		printf("GL Ratio: %f \n", screen->devicePixelRatio() );
	}
	return 0;
}

void ConsoleViewGL_t::reset(void)
{
	buildTextures();

	return;
}

void ConsoleViewGL_t::buildTextures(void)
{
	int w, h;

	if ( gltexture )
	{
		glDeleteTextures(1, &gltexture);
		gltexture=0;
	}

	glGenTextures(1, &gltexture);

	glBindTexture( GL_TEXTURE_2D, gltexture);

	glTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, linearFilter ? GL_LINEAR : GL_NEAREST );
	glTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, linearFilter ? GL_LINEAR : GL_NEAREST );
	glTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE );
	glTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE );

	txtWidth  = w = nes_shm->video.ncol;
	txtHeight = h = nes_shm->video.nrow;

	glTexImage2D( GL_TEXTURE_2D, 0, 
			GL_RGBA8, w, h, 0,
					GL_BGRA, GL_UNSIGNED_BYTE, 0 );

	//printf("Texture Built: %ix%i\n", w, h);
}

void ConsoleViewGL_t::buildBgTexture(void)
{
	if ( bgTexture )
	{
		glDeleteTextures(1, &bgTexture);
		bgTexture = 0;
	}

	if ( bgPix.isNull() )
	{
		bgPix.load(":/icons/pic.png");
	}
	if ( bgPix.isNull() )
	{
		return;
	}

	QImage img = bgPix.toImage().convertToFormat(QImage::Format_ARGB32);

	glGenTextures(1, &bgTexture);
	glBindTexture(GL_TEXTURE_2D, bgTexture);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
	glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, img.width(), img.height(), 0,
	             GL_BGRA, GL_UNSIGNED_BYTE, img.bits());
}

void ConsoleViewGL_t::initializeGL(void)
{
	printf("initializeGL start\n");

	if ( !initializeOpenGLFunctions() )
	{
		printf("Error: Failed to initialize OpenGL 3.3 Core functions\n");
		glFunctionsInitialized = false;
		return;
	}
	glFunctionsInitialized = true;

	// Set up the rendering context, load shaders and other resources, etc.:
	glClearColor(0.0f, 0.0f, 0.0f, 1.0f);

	const GLubyte *c = glGetString( GL_VERSION );
	if ( c != NULL )
	{
		printf("GL Version: %s \n", c );
	}

	shaderProgram = new QOpenGLShaderProgram(this);
	shaderProgram->addShaderFromSourceCode(QOpenGLShader::Vertex, vertexShaderSource);
	shaderProgram->addShaderFromSourceCode(QOpenGLShader::Fragment, fragmentShaderSource);
	shaderProgram->link();

	vao = new QOpenGLVertexArrayObject(this);
	vao->create();
	vao->bind();

	vbo = new QOpenGLBuffer(QOpenGLBuffer::VertexBuffer);
	vbo->create();
	vbo->bind();
	vbo->setUsagePattern(QOpenGLBuffer::DynamicDraw);

	ebo = new QOpenGLBuffer(QOpenGLBuffer::IndexBuffer);
	ebo->create();
	ebo->bind();
	GLushort indices[] = { 0, 1, 2, 0, 2, 3 };
	ebo->allocate(indices, sizeof(indices));

	shaderProgram->bind();
	shaderProgram->enableAttributeArray(0);
	shaderProgram->setAttributeBuffer(0, GL_FLOAT, 0, 2, 4 * sizeof(float));
	shaderProgram->enableAttributeArray(1);
	shaderProgram->setAttributeBuffer(1, GL_FLOAT, 2 * sizeof(float), 2, 4 * sizeof(float));
	shaderProgram->release();

	vao->release();

	buildTextures();
	buildBgTexture();

	connect(context(), &QOpenGLContext::aboutToBeDestroyed, this, &ConsoleViewGL_t::cleanupGL);
}

void ConsoleViewGL_t::cleanupGL(void)
{
	//printf("cleanupGL\n");
	// Make sure the context is current and then explicitly
	// destroy all underlying OpenGL resources.
	makeCurrent();

	 // Free GL texture
	 if (gltexture) 
	 {
	 	//printf("Destroying GL Texture\n");
	 	glDeleteTextures(1, &gltexture);
	 	gltexture=0;
	 }
	 if (bgTexture) 
	 {
	 	glDeleteTextures(1, &bgTexture);
	 	bgTexture=0;
	 }

	delete vao; vao = nullptr;
	delete vbo; vbo = nullptr;
	delete ebo; ebo = nullptr;
	delete shaderProgram; shaderProgram = nullptr;

	 doneCurrent();
}

void ConsoleViewGL_t::resizeGL(int w, int h)
{
	w = static_cast<int>( devPixRatio * w );
	h = static_cast<int>( devPixRatio * h );
	//printf("GL Resize: %i x %i \n", w, h );
	glViewport(0, 0, w, h);

	view_width  = w;
	view_height = h;

	gui_draw_area_width = w;
	gui_draw_area_height = h;

	buildTextures();
}

void ConsoleViewGL_t::setBgColor( QColor &c )
{
	if ( bgColor )
	{
		*bgColor = c;
	}
}

void ConsoleViewGL_t::setVsyncEnable( bool ena )
{
	if ( vsyncEnabled != ena )
	{
		QSurfaceFormat fmt = format();

		vsyncEnabled = ena;

		fmt.setSwapInterval( vsyncEnabled ? 1 : 0 );

		setFormat(fmt);
	}
}

void ConsoleViewGL_t::setLinearFilterEnable( bool ena )
{
   if ( linearFilter != ena )
   {
      linearFilter = ena;

	   buildTextures();
   }
}

void ConsoleViewGL_t::setScaleXY( double xs, double ys )
{
	xscale = xs;
	yscale = ys;

	if ( forceAspect )
	{
		if ( xscale < yscale )
		{
			yscale = xscale;
		}
		else 
		{
			xscale = yscale;
		}
	}
}

void ConsoleViewGL_t::setAspectXY( double x, double y )
{
	aspectX = x;
	aspectY = y;

	aspectRatio = aspectY / aspectX;
}

void ConsoleViewGL_t::getAspectXY( double &x, double &y )
{
	x = aspectX;
	y = aspectY;
}

double ConsoleViewGL_t::getAspectRatio(void)
{
	return aspectRatio;
}

void ConsoleViewGL_t::transfer2LocalBuffer(void)
{
	int i=0, hq = 0, bufIdx;
	int numPixels = nes_shm->video.ncol * nes_shm->video.nrow;
	unsigned int cpSize = numPixels * 4;
 	uint8_t *src, *dest;

	bufIdx = nes_shm->pixBufIdx-1;

	if ( bufIdx < 0 )
	{
		bufIdx = NES_VIDEO_BUFLEN-1;
	}
	if ( cpSize > localBufSize )
	{
		cpSize = localBufSize;
	}
	src  = reinterpret_cast<uint8_t*>(nes_shm->pixbuf[bufIdx]);
	dest = reinterpret_cast<uint8_t*>(localBuf.get());

	hq = (nes_shm->video.preScaler == 1) || (nes_shm->video.preScaler == 4); // hq2x and hq3x

	if ( hq )
	{
		for (i=0; i<numPixels; i++)
		{
			dest[3] = 0xFF;
			dest[1] = src[1];
			dest[2] = src[2];
			dest[0] = src[0];

			src += 4; dest += 4;
		}
	}
	else
	{
		copyPixels32( dest, src, cpSize, alphaMask);
	}
}

void ConsoleViewGL_t::mousePressEvent(QMouseEvent * event)
{
	//printf("Mouse Button Press: (%i,%i) %x  %x\n", 
	//		event->pos().x(), event->pos().y(), event->button(), event->buttons() );

	mouseButtonMask = event->buttons();
}

void ConsoleViewGL_t::mouseReleaseEvent(QMouseEvent * event)
{
	//printf("Mouse Button Release: (%i,%i) %x  %x\n", 
	//		event->pos().x(), event->pos().y(), event->button(), event->buttons() );

	mouseButtonMask = event->buttons();
}

void ConsoleViewGL_t::keyPressEvent(QKeyEvent *event)
{
	// v0.3.15 PR-B: forward to base class so QInputMethodEvent reaches
	// the focused QLineEdit (Chinese IME composition) before routing
	// the key to the game key state.
	QOpenGLWindow::keyPressEvent(event);
	pushKeyEvent(event, 1);
	event->accept();
}

void ConsoleViewGL_t::keyReleaseEvent(QKeyEvent *event)
{
	// v0.3.15 PR-B: same IME forwarding rationale as keyPressEvent.
	QOpenGLWindow::keyReleaseEvent(event);
	pushKeyEvent(event, 0);
	event->accept();
}

bool ConsoleViewGL_t::getMouseButtonState( unsigned int btn )
{
	return (mouseButtonMask & btn) ? true : false;
}

void  ConsoleViewGL_t::getNormalizedCursorPos( double &x, double &y )
{
	QPoint cursor;

	cursor = QCursor::pos();

	//printf("Global Cursor (%i,%i) \n", cursor.x(), cursor.y() );

	cursor = mapFromGlobal( cursor );

	//printf("Window Cursor (%i,%i) \n", cursor.x(), cursor.y() );

	x = static_cast<double>(cursor.x() - sx) / static_cast<double>(rw);
	y = static_cast<double>(cursor.y() - sy) / static_cast<double>(rh);

	if ( x < 0.0 )
	{
		x = 0.0;
	}
	else if ( x > 1.0 )
	{
		x = 1.0;
	}
	if ( y < 0.0 )
	{
		y = 0.0;
	}
	else if ( y > 1.0 )
	{
		y = 1.0;
	}
	//printf("Normalized Cursor (%f,%f) \n", x, y );
}

void ConsoleViewGL_t::renderFinished(void)
{
	videoBufferSwapMark();
}

void ConsoleViewGL_t::renderBg(void)
{
	if ( !bgTexture )
	{
		return;
	}

	int bgW = bgPix.width();
	int bgH = bgPix.height();
	int x = (view_width  - bgW) / 2;
	int y = (view_height - bgH) / 2;

	glViewport(0, 0, view_width, view_height);

	projectionMatrix.setToIdentity();
	projectionMatrix.ortho(0.0f, static_cast<float>(view_width), 0.0f, static_cast<float>(view_height), -1.0f, 1.0f);

	float vertices[] = {
		// pos                  // tex
		static_cast<float>(x),         static_cast<float>(y),          0.0f, 1.0f,
		static_cast<float>(x + bgW), static_cast<float>(y),          1.0f, 1.0f,
		static_cast<float>(x + bgW), static_cast<float>(y + bgH), 1.0f, 0.0f,
		static_cast<float>(x),         static_cast<float>(y + bgH), 0.0f, 0.0f
	};

	shaderProgram->bind();
	vao->bind();
	vbo->bind();
	vbo->allocate(vertices, sizeof(vertices));
	ebo->bind();

	glActiveTexture(GL_TEXTURE0);
	glBindTexture(GL_TEXTURE_2D, bgTexture);

	shaderProgram->setUniformValue("uProjection", projectionMatrix);
	shaderProgram->setUniformValue("uTexture", 0);

	glDrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_SHORT, nullptr);

	vao->release();
	shaderProgram->release();
}

void ConsoleViewGL_t::renderFrame(void)
{
	int texture_width  = nes_shm->video.ncol;
	int texture_height = nes_shm->video.nrow;
	int l=0, r=texture_width;
	int t=0, b=texture_height;

	float ixScale   = static_cast<float>(nes_shm->video.xscale);
	float iyScale   = static_cast<float>(nes_shm->video.yscale);
	float xscaleTmp = static_cast<float>(view_width)  / static_cast<float>(texture_width);
	float yscaleTmp = static_cast<float>(view_height) / static_cast<float>(texture_height);

	xscaleTmp *= ixScale;
	yscaleTmp *= iyScale;

	if ( forceAspect )
	{
		if ( xscaleTmp < yscaleTmp )
		{
			yscaleTmp = xscaleTmp;
		}
		else 
		{
			xscaleTmp = yscaleTmp;
		}
	}

	if ( autoScaleEna )
	{
		xscale = xscaleTmp;
		yscale = yscaleTmp;
	}
	else
	{
		if ( xscaleTmp > xscale )
		{
			xscaleTmp = xscale;
		}
		if ( yscaleTmp > yscale )
		{
			yscaleTmp = yscale;
		}
	}

	rw=static_cast<int>((r-l)*xscaleTmp/ixScale);
	rh=static_cast<int>((b-t)*yscaleTmp/iyScale);

	if ( forceAspect )
	{
		int iw, ih, ax, ay;

		ax = static_cast<int>(aspectX+0.50);
		ay = static_cast<int>(aspectY+0.50);

		iw = rw * ay;
		ih = rh * ax;
		
		if ( iw > ih )
		{
			rh = (rw * ay) / ax;
		}
		else
		{
			rw = (rh * ax) / ay;
		}

		if ( rw > view_width )
		{
			rw = view_width;
			rh = (rw * ay) / ax;
		}

		if ( rh > view_height )
		{
			rh = view_height;
			rw = (rh * ax) / ay;
		}
	}

	if ( rw > view_width ) rw = view_width;
	if ( rh > view_height) rh = view_height;

	sx=(view_width-rw)/2;   
	sy=(view_height-rh)/2;

	glViewport(sx, sy, rw, rh);

	glActiveTexture(GL_TEXTURE0);
	glBindTexture(GL_TEXTURE_2D, gltexture);
	glTexSubImage2D(GL_TEXTURE_2D, 0,
		  	0, 0, texture_width, texture_height,
				GL_BGRA, GL_UNSIGNED_BYTE, localBuf.get() );

	projectionMatrix.setToIdentity();
	projectionMatrix.ortho( 0.0,  rw,  0.0,  rh,  -1.0,  1.0);

	float u = static_cast<float>(texture_width)  / static_cast<float>(txtWidth);
	float v = static_cast<float>(texture_height) / static_cast<float>(txtHeight);

	float vertices[] = {
		// pos       // tex
		0.0f, 0.0f,  0.0f, v,
		static_cast<float>(rw), 0.0f,  u, v,
		static_cast<float>(rw), static_cast<float>(rh), u, 0.0f,
		0.0f, static_cast<float>(rh), 0.0f, 0.0f
	};

	shaderProgram->bind();
	vao->bind();
	vbo->bind();
	vbo->allocate(vertices, sizeof(vertices));
	ebo->bind();

	shaderProgram->setUniformValue("uProjection", projectionMatrix);
	shaderProgram->setUniformValue("uTexture", 0);

	glDrawElements(GL_TRIANGLES, 6, GL_UNSIGNED_SHORT, nullptr);

	vao->release();
	shaderProgram->release();
}

void ConsoleViewGL_t::paintGL(void)
{
	if ( !glFunctionsInitialized )
	{
		printf("paintGL skipped: OpenGL functions not initialized\n");
		return;
	}

	if ( bgColor )
	{
		glClearColor( bgColor->redF(), bgColor->greenF(), bgColor->blueF(), 1.0f);
	}
	else
	{
		glClearColor( 30.0/255.0, 69.0/255.0, 40.0/255.0, 1.0f);
	}
	glClear(GL_COLOR_BUFFER_BIT);

	extern FCEUGI *GameInfo;
	if ( GameInfo == nullptr )
	{
		renderBg();
		nes_shm->render_count++;
		return;
	}

	renderFrame();

	nes_shm->render_count++;
	 //printf("Paint GL!\n");
}

void ConsoleViewGL_t::calcPixRemap(void)
{
}

void ConsoleViewGL_t::doRemap(void)
{
}
