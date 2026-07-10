// ConsoleEmulatorThread.cpp
//
#include <stdio.h>
#include <QString>

#include "Qt/ConsoleWindow.h"
#include "Qt/fceuWrapper.h"
#include "Qt/nes_shm.h"

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
	nes_shm->runEmulator = 1;

	init();

	while ( nes_shm->runEmulator )
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
