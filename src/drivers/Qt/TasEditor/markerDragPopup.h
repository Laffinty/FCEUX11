// markerDragPopup.h
//

#pragma once

#include <QDialog>
#include <QTimer>
#include <QColor>
#include <QPoint>
#include <QEvent>

class markerDragPopup : public QDialog
{
	Q_OBJECT

	public:
		markerDragPopup( QWidget *parent = nullptr );
		~markerDragPopup( void );

		void setInitialPosition( QPoint p );
		void setRowIndex( int row );
		void setBgColor( QColor c );
		void throwAway(void);
		void dropAccept(void);
		void dropAbort(void);
	protected:
		bool eventFilter(QObject *obj, QEvent *event) override;
		void paintEvent(QPaintEvent *event) override;

		int alpha;
		int rowIndex;
		int liveCount;
		QColor bgColor;
		QPoint initialPos;
		QTimer *timer;

		bool released;
		bool dropAccepted;
		bool dropAborted;
		bool thrownAway;

	private slots:
		void fadeAway(void);
};
