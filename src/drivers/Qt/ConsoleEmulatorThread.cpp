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
// ConsoleEmulatorThread.cpp
//
#include <stdio.h>
#include <QString>

#include "Qt/ConsoleWindow.h"
#include "Qt/fceuWrapper.h"
#include "common/nes_shm.h"

emulatorThread_t::emulatorThread_t( QObject *parent )
	: QThread(parent)
{
	setObjectName( QString("EmulationThread") );
}

void emulatorThread_t::init(void)
{
	int opt;

	g_config->getOption( "SDL.SetSchedParam", &opt );

	if ( opt )
	{
	}
}

void emulatorThread_t::setPriority( QThread::Priority priority_req )
{
	QThread::setPriority( priority_req );
}

void emulatorThread_t::run(void)
{
	printf("Emulator Start\n");
	nes_shm->runEmulator.store(1, std::memory_order_relaxed);

	init();

	while ( nes_shm->runEmulator.load(std::memory_order_acquire) )
	{
		fceuWrapperUpdate();
	}
	printf("Emulator Exit\n");
	emit finished();
}

void emulatorThread_t::signalFrameFinished(void)
{
	emit frameFinished();
}

void emulatorThread_t::signalRomLoad( const char *path )
{
	emit loadRomRequest( QString(path) );
}
