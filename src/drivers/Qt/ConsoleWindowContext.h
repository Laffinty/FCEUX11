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
