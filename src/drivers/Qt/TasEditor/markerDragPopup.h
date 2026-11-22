/* FCE Ultra - NES/Famicom Emulator
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
