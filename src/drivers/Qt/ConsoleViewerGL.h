// GameViewerGL.h
//

#pragma  once

#include <stdint.h>

#include <QColor>
#include <QCursor>
#include <QMatrix4x4>
#include <QOpenGLBuffer>
#include <QOpenGLFunctions_3_3_Core>
#include <QOpenGLShaderProgram>
#include <QOpenGLVertexArrayObject>
#include <QOpenGLWindow>
#include <QPixmap>
#include <QScreen>
#include <QSize>

#include "Qt/ConsoleViewerInterface.h"

class ConsoleViewGL_t : public QOpenGLWindow,
                        protected QOpenGLFunctions_3_3_Core,
                        public ConsoleViewerBase
{
    Q_OBJECT

	public:
		ConsoleViewGL_t(QWindow *parent = nullptr);
		~ConsoleViewGL_t(void) override;

		int  init(void) override;
		void reset(void) override;
		void queueRedraw(void) override { update(); }
		int  driver(void) override { return VIDEO_DRIVER_OPENGL; }

		void transfer2LocalBuffer(void) override;

		void setVsyncEnable( bool ena ) override;
		void setLinearFilterEnable( bool ena ) override;

		bool   getForceAspectOpt(void) override { return forceAspect; }
		void   setForceAspectOpt( bool val ) override { forceAspect = val; return; }
		bool   getAutoScaleOpt(void) override { return autoScaleEna; }
		void   setAutoScaleOpt( bool val ) override { autoScaleEna = val; return; }
		double getScaleX(void) override { return xscale; }
		double getScaleY(void) override { return yscale; }
		void   setScaleXY( double xs, double ys ) override;
		void   getNormalizedCursorPos( double &x, double &y ) override;
		bool   getMouseButtonState( unsigned int btn ) override;
		void   setAspectXY( double x, double y ) override;
		void   getAspectXY( double &x, double &y ) override;
		double getAspectRatio(void) override;

		void   screenChanged(QScreen *scr);
		void   setBgColor( QColor &c ) override;
		void   setCursor(const QCursor &c) override { QOpenGLWindow::setCursor(c); }
		void   setCursor( Qt::CursorShape s ) override { QOpenGLWindow::setCursor(s); }

		QSize   size(void) override { return QOpenGLWindow::size(); }
		QCursor cursor(void) override { return QOpenGLWindow::cursor(); }
		void    setMinimumSize(const QSize &s) override { QOpenGLWindow::setMinimumSize(s); }
		void    setMaximumSize(const QSize &s) override { QOpenGLWindow::setMaximumSize(s); }

	protected:
	void initializeGL(void) override;
	void resizeGL(int w, int h) override;
	void paintGL(void) override;
	void mousePressEvent(QMouseEvent * event) override;
	void mouseReleaseEvent(QMouseEvent * event) override;
	void keyPressEvent(QKeyEvent *event) override;
	void keyReleaseEvent(QKeyEvent *event) override;

	void buildTextures(void);
	void buildBgTexture(void);
	void calcPixRemap(void);
	void doRemap(void);
	void renderBg(void);
	void renderFrame(void);

	double devPixRatio;
	double aspectRatio;
	double aspectX;
	double aspectY;
	double xscale;
	double yscale;
	int  view_width;
	int  view_height;
	int  sx;
	int  sy;
	int  rw;
	int  rh;
	int  txtWidth;
	int  txtHeight;
	GLuint gltexture;
	GLuint bgTexture;
	bool   linearFilter;
	bool   forceAspect;
	bool   autoScaleEna;
	bool   vsyncEnabled;
	bool   glFunctionsInitialized;

	unsigned int  mouseButtonMask;
	QColor *bgColor;
	QPixmap bgPix;

	uint32_t  *localBuf;
	uint32_t   localBufSize;

	QOpenGLShaderProgram *shaderProgram;
	QOpenGLVertexArrayObject *vao;
	QOpenGLBuffer *vbo;
	QOpenGLBuffer *ebo;
	QMatrix4x4 projectionMatrix;

	private slots:
		void cleanupGL(void);
		void renderFinished(void);
};
