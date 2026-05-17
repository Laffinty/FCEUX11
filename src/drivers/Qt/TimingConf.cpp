/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2020 mjbudd77
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
//
// TimingConf.cpp
//
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <string>

#include <SDL.h>
#include <QHeaderView>
#include <QCloseEvent>

#include "fceu.h"
#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/input.h"
#include "Qt/config.h"
#include "Qt/keyscan.h"
#include "Qt/throttle.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/TimingConf.h"

//----------------------------------------------------------------------------
static bool hasNicePermissions(int val)
{
	return false;
}
//----------------------------------------------------------------------------
TimingConfDialog_t::TimingConfDialog_t(QWidget *parent)
	: QDialog(parent)
{
	int opt;
	QVBoxLayout *mainLayout, *vbox;
	QHBoxLayout *hbox;
	QGridLayout *grid;
	QPushButton *closeButton;
	QGroupBox *emuPrioBox, *guiPrioBox;

	setWindowTitle("Timing Configuration");

	mainLayout = new QVBoxLayout();

	emuPrioCtlEna = new QCheckBox(tr("Set Scheduling Parameters at Startup"));

	emuPrioBox = new QGroupBox(tr("EMU Thread Scheduling Parameters"));
	guiPrioBox = new QGroupBox(tr("GUI Thread Scheduling Parameters"));
	grid = new QGridLayout();
	emuPrioBox->setLayout(grid);

	mainLayout->addWidget(emuPrioCtlEna);
	mainLayout->addWidget(emuPrioBox);
	mainLayout->addWidget(guiPrioBox);

#ifdef WIN32
	emuSchedPrioBox = new QComboBox();
	guiSchedPrioBox = new QComboBox();

	grid->addWidget(emuSchedPrioBox, 0, 0);

	grid = new QGridLayout();
	guiPrioBox->setLayout(grid);

	grid->addWidget(guiSchedPrioBox, 0, 0);

	emuSchedPrioBox->addItem(tr("Idle"), QThread::IdlePriority);
	emuSchedPrioBox->addItem(tr("Lowest"), QThread::LowestPriority);
	emuSchedPrioBox->addItem(tr("Low"), QThread::LowPriority);
	emuSchedPrioBox->addItem(tr("Normal"), QThread::NormalPriority);
	emuSchedPrioBox->addItem(tr("High"), QThread::HighPriority);
	emuSchedPrioBox->addItem(tr("Highest"), QThread::HighestPriority);
	emuSchedPrioBox->addItem(tr("Time Critical"), QThread::TimeCriticalPriority);
	emuSchedPrioBox->addItem(tr("Inherit"), QThread::InheritPriority);

	guiSchedPrioBox->addItem(tr("Idle"), QThread::IdlePriority);
	guiSchedPrioBox->addItem(tr("Lowest"), QThread::LowestPriority);
	guiSchedPrioBox->addItem(tr("Low"), QThread::LowPriority);
	guiSchedPrioBox->addItem(tr("Normal"), QThread::NormalPriority);
	guiSchedPrioBox->addItem(tr("High"), QThread::HighPriority);
	guiSchedPrioBox->addItem(tr("Highest"), QThread::HighestPriority);
	guiSchedPrioBox->addItem(tr("Time Critical"), QThread::TimeCriticalPriority);
	guiSchedPrioBox->addItem(tr("Inherit"), QThread::InheritPriority);

	hbox = new QHBoxLayout();
	timingDevSelBox = new QComboBox();
	timingDevSelBox->addItem(tr("SDL_Delay"), 0);
	hbox->addWidget(new QLabel(tr("Timing Mechanism:")));
	hbox->addWidget(timingDevSelBox);
	mainLayout->addLayout(hbox);

	vbox = new QVBoxLayout();
	grid = new QGridLayout();
	ppuOverClockBox = new QGroupBox( tr("Overclocking (Old PPU Only)") );
	ppuOverClockBox->setCheckable(true);
	ppuOverClockBox->setChecked(overclock_enabled);
	ppuOverClockBox->setEnabled(!newppu);
	ppuOverClockBox->setLayout(vbox);
	mainLayout->addWidget( ppuOverClockBox );

	postRenderBox      = new QSpinBox();
	vblankScanlinesBox = new QSpinBox();
	no7bitSamples      = new QCheckBox( tr("Don't Overclock 7-bit Samples") );

	postRenderBox->setRange(0, 999);
	vblankScanlinesBox->setRange(0, 999);

	postRenderBox->setValue( postrenderscanlines );
	vblankScanlinesBox->setValue( vblankscanlines );
	no7bitSamples->setChecked( skip_7bit_overclocking );

	vbox->addLayout( grid );
	grid->addWidget( new QLabel( tr("Post-render") ), 0, 0 );
	grid->addWidget( new QLabel( tr("Vblank Scanlines") ), 1, 0 );
	grid->addWidget( postRenderBox, 0, 1 );
	grid->addWidget( vblankScanlinesBox, 1, 1 );
	vbox->addWidget( no7bitSamples );

	closeButton = new QPushButton( tr("Close") );
	closeButton->setIcon(style()->standardIcon(QStyle::SP_DialogCloseButton));
	connect(closeButton, SIGNAL(clicked(void)), this, SLOT(closeWindow(void)));

	hbox = new QHBoxLayout();
	hbox->addStretch(5);
	hbox->addWidget( closeButton, 1 );
	mainLayout->addLayout( hbox );

	setLayout(mainLayout);

	g_config->getOption("SDL.SetSchedParam", &opt);

	emuPrioCtlEna->setChecked(opt);

	updatePolicyBox();
	updateSliderLimits();
	updateSliderValues();
	updateTimingMech();
	updateOverclocking();
#endif

	connect(emuSchedPrioBox, SIGNAL(activated(int)), this, SLOT(emuSchedPrioChange(int)));
	connect(guiSchedPrioBox, SIGNAL(activated(int)), this, SLOT(guiSchedPrioChange(int)));
	connect(emuPrioCtlEna, SIGNAL(stateChanged(int)), this, SLOT(emuSchedCtlChange(int)));
	connect(timingDevSelBox, SIGNAL(activated(int)), this, SLOT(emuTimingMechChange(int)));

	connect( ppuOverClockBox   , SIGNAL(toggled(bool))    , this, SLOT(overclockingToggled(bool)));
	connect( postRenderBox     , SIGNAL(valueChanged(int)), this, SLOT(postRenderChanged(int)));
	connect( vblankScanlinesBox, SIGNAL(valueChanged(int)), this, SLOT(vblankScanlinesChanged(int)));
	connect( no7bitSamples     , SIGNAL(stateChanged(int)), this, SLOT(no7bitChanged(int)));

	updateTimer  = new QTimer( this );

	connect( updateTimer, &QTimer::timeout, this, &TimingConfDialog_t::periodicUpdate );

	updateTimer->start( 500 ); // 2Hz
}
//----------------------------------------------------------------------------
TimingConfDialog_t::~TimingConfDialog_t(void)
{
	//printf("Destroy Timing Config Window\n");
	updateTimer->stop();
	saveValues();
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::closeEvent(QCloseEvent *event)
{
	//printf("Timing Close Window Event\n");
	done(0);
	deleteLater();
	event->accept();
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::closeWindow(void)
{
	//printf("Close Window\n");
	done(0);
	deleteLater();
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::periodicUpdate(void)
{
	updateOverclocking();

}
//----------------------------------------------------------------------------
void TimingConfDialog_t::emuSchedCtlChange(int state)
{
	g_config->setOption("SDL.SetSchedParam", (state != Qt::Unchecked));
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::saveValues(void)
{
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::emuSchedNiceChange(int val)
{
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::emuSchedPrioChange(int val)
{
	if (consoleWindow == NULL)
	{
		return;
	}
#ifdef WIN32
	printf("Setting EMU Thread to %i\n", val);
	FCEU_WRAPPER_LOCK();
	consoleWindow->emulatorThread->setPriority((QThread::Priority)val);
	FCEU_WRAPPER_UNLOCK();
#else
#error "Platform not supported"
#endif
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::emuSchedPolicyChange(int index)
{
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::guiSchedNiceChange(int val)
{
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::guiSchedPrioChange(int val)
{
#ifdef WIN32
	printf("Setting GUI Thread to %i\n", val);
	QThread::currentThread()->setPriority((QThread::Priority)val);
#else
#error "Platform not supported"
#endif
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::guiSchedPolicyChange(int index)
{
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::updatePolicyBox(void)
{
	if (consoleWindow == NULL)
	{
		return;
	}
#ifdef WIN32
	int prio;

	prio = consoleWindow->emulatorThread->priority();

	printf("EMU Priority %i\n", prio);
	for (int j = 0; j < emuSchedPrioBox->count(); j++)
	{
		if (emuSchedPrioBox->itemData(j).toInt() == prio)
		{
			printf("EMU Found Priority %i  %i\n", j, prio);
			emuSchedPrioBox->setCurrentIndex(j);
		}
	}

	prio = QThread::currentThread()->priority();

	for (int j = 0; j < guiSchedPrioBox->count(); j++)
	{
		if (guiSchedPrioBox->itemData(j).toInt() == prio)
		{
			printf("GUI Found Priority %i  %i\n", j, prio);
			guiSchedPrioBox->setCurrentIndex(j);
		}
	}
#else
#error "Platform not supported"
#endif
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::updateSliderValues(void)
{
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::updateSliderLimits(void)
{
	if (consoleWindow == NULL)
	{
		return;
	}
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::emuTimingMechChange(int index)
{
	int mode;

	if (consoleWindow == NULL)
	{
		return;
	}
	FCEU_WRAPPER_LOCK();

	mode = timingDevSelBox->itemData(index).toInt();

	setTimingMode(mode);

	RefreshThrottleFPS();

	g_config->setOption("SDL.EmuTimingMech", mode);

	FCEU_WRAPPER_UNLOCK();
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::updateTimingMech(void)
{
	int mode = getTimingMode();

	for (int j = 0; j < timingDevSelBox->count(); j++)
	{
		if (timingDevSelBox->itemData(j).toInt() == mode)
		{
			timingDevSelBox->setCurrentIndex(j);
		}
	}
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::overclockingToggled(bool on)
{
	FCEU_WRAPPER_LOCK();
	if ( !newppu )
	{
		overclock_enabled = on;
		g_config->setOption("SDL.OverClockEnable", overclock_enabled );
	}
	FCEU_WRAPPER_UNLOCK();
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::postRenderChanged(int value)
{
	FCEU_WRAPPER_LOCK();
	postrenderscanlines = value;
	g_config->setOption("SDL.PostRenderScanlines", postrenderscanlines );
	FCEU_WRAPPER_UNLOCK();
	//printf("Post Render %i\n", postrenderscanlines );
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::vblankScanlinesChanged(int value)
{
	FCEU_WRAPPER_LOCK();
	vblankscanlines = value;
	g_config->setOption("SDL.VBlankScanlines", vblankscanlines );
	FCEU_WRAPPER_UNLOCK();
	//printf("Vblank Scanlines %i\n", vblankscanlines );
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::no7bitChanged(int value)
{
	FCEU_WRAPPER_LOCK();
	skip_7bit_overclocking = (value != Qt::Unchecked);
	g_config->setOption("SDL.Skip7bitOverClocking", skip_7bit_overclocking );
	FCEU_WRAPPER_UNLOCK();
	//printf("Skip 7-bit: %i\n", skip_7bit_overclocking );
}
//----------------------------------------------------------------------------
void TimingConfDialog_t::updateOverclocking(void)
{
	ppuOverClockBox->setEnabled( !newppu );
	ppuOverClockBox->setChecked( overclock_enabled );
}
//----------------------------------------------------------------------------
