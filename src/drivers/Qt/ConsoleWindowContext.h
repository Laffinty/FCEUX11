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
// ConsoleWindowContext.h
//

#pragma once

#include <QObject>
#include <list>
#include <string>

class ConsoleWindowContext : public QObject
{
	Q_OBJECT

public:
	ConsoleWindowContext(QObject *parent = nullptr)
		: QObject(parent)
		, romLoaded(false)
		, emulationPaused(false)
	{
	}

	bool romLoaded;
	bool emulationPaused;

	std::list<std::string*> romList;

signals:
	void romLoadedSignal();
	void romClosedSignal();
	void pauseStateChanged(bool paused);
	void recentRomListUpdated();
	void regionChanged(int region);
};
