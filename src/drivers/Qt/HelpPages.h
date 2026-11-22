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
// HelpPages.h
#pragma once

#include <QDialog>
#include <QAction>
#include <QSplitter>
#include <QTabWidget>
#include <QCloseEvent>

#ifdef _USE_QHELP
#include <QTextBrowser>
#include <QHelpEngine>

class HelpBrowser : public QTextBrowser
{
	public:
    		HelpBrowser(QHelpEngine* helpEngine, QWidget* parent = 0);
    		QVariant loadResource (int type, const QUrl& name);
	private:
		QHelpEngine* helpEngine;
};

class HelpDialog : public QDialog
{
	Q_OBJECT

	public:
		HelpDialog( const char *helpFileName, QWidget *parent = 0);
		~HelpDialog(void);

		void retranslateUi(void);
	protected:
		void closeEvent(QCloseEvent *event);
		void changeEvent(QEvent *event) override;

		QHelpEngine  *helpEngine;
		QSplitter    *hsplitter;
		QTabWidget   *tabWgt;
		HelpBrowser  *textViewer;
		QAction      *backButton;
		QAction      *forwardButton;
		QAction      *closeAction;

	public slots:
		void closeWindow(void);
		void navBackwardAvailable(bool);
		void navForwardAvailable(bool);
};

#endif
