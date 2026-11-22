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
// ConsoleMenuBar.cpp
//
#include <QKeyEvent>
#include <QMenuBar>

#include "../../fceu.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/keyscan.h"
#include "Qt/fceuWrapper.h"

consoleMenuBar::consoleMenuBar(QWidget *parent)
	: QMenuBar(parent)
{

}
consoleMenuBar::~consoleMenuBar(void)
{

}

void consoleMenuBar::keyPressEvent(QKeyEvent *event)
{
	QMenuBar::keyPressEvent(event);

	pushKeyEvent( event, 1 );

	if ( event->key() == Qt::Key_Escape )
	{
		((QWidget*)parent())->setFocus();
	}
	event->accept();
}

void consoleMenuBar::keyReleaseEvent(QKeyEvent *event)
{
	QMenuBar::keyReleaseEvent(event);

	pushKeyEvent( event, 0 );

	event->accept();
}

autoFireMenuAction::autoFireMenuAction(int on, int off, QString name, QWidget *parent)
	: QAction( name, parent)
{
	onFrames = on;  offFrames = off;
}

autoFireMenuAction::~autoFireMenuAction(void)
{

}

void autoFireMenuAction::activateCB(void)
{
	g_config->setOption("SDL.AutofireOnFrames"  , onFrames );
	g_config->setOption("SDL.AutofireOffFrames" , offFrames);
	g_config->save();

	SetAutoFirePattern( onFrames, offFrames );
}

bool autoFireMenuAction::isMatch( int on, int off )
{
	return ( (on == onFrames) && (off == offFrames) );
}

void autoFireMenuAction::setPattern(int on, int off)
{
	onFrames = on;  offFrames = off;
}
