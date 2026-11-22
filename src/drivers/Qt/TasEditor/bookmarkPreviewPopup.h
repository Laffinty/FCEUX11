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
// bookmarkPreviewPopup.h
//

#pragma once

#include <memory>
#include <QDialog>
#include <QLabel>
#include <QTimer>
#include <QEvent>

class bookmarkPreviewPopup : public QDialog
{
   Q_OBJECT
	public:
	   bookmarkPreviewPopup( int index, QWidget *parent = nullptr );
	   ~bookmarkPreviewPopup(void);

	   int reloadImage(int index);

	   static int currentIndex(void);

	   static bookmarkPreviewPopup *currentInstance(void);

	protected:
		void changeEvent(QEvent *event) override;

	private:
		int loadImage(int index);

		void retranslateUi(void);

		int alpha;
		int imageIndex;
		bool actv;
		std::unique_ptr<unsigned char[]> screenShotRaster;
		QTimer *timer;
		QLabel *imgLbl, *descLbl;

		static bookmarkPreviewPopup *instance;

	public slots:
		void periodicUpdate(void);
		void imageIndexChanged(int);
};
