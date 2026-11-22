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
// ColorMenu.h
//

#pragma once

#include <QWidget>
#include <QWidgetAction>
#include <QAction>
#include <QColor>
#include <QDialog>
#include <QColorDialog>
#include <QCloseEvent>

class ColorMenuPickerDialog_t : public QDialog
{
	Q_OBJECT

	public:
		ColorMenuPickerDialog_t( QColor *c, const char *titleText, QWidget *parent = 0);
		~ColorMenuPickerDialog_t(void);

	protected:
		void closeEvent(QCloseEvent *event);

	private:
		QColorDialog *colorDialog;
		QColor *colorPtr;
		QColor  origColor;

	public slots:
		void closeWindow(void);
	private slots:
		void colorChanged( const QColor &color );
		void colorAccepted(void);
		void colorRejected(void);
		void resetColor(void);
};

class ColorMenuItem : public QAction
{
	Q_OBJECT

	public:
		ColorMenuItem( QString txt, const char *confName, QWidget *parent = 0);
		~ColorMenuItem(void);

		void connectColor( QColor *c );
	protected:
		QString  title;
		QColor  *colorPtr;
		QColor   lastColor;
		ColorMenuPickerDialog_t *picker;
		const char *confName;

		void setImageColor( QColor c );
	public slots:
		void openColorPicker(void);
		void pickerClosed(int ret);

	signals:
		void colorChanged( QColor &c );
};
