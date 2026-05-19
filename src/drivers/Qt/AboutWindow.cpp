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
// AboutWindow.cpp
//
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <QDesktopServices>
#include <QUrl>
#include "Qt/sdl.h"
#include "Qt/AboutWindow.h"
#include "../../version.h"
#include "../../fceu.h"
//----------------------------------------------------------------------------
AboutWindow::AboutWindow(QWidget *parent)
	: QDialog( parent )
{
	QVBoxLayout *mainLayout;
	QHBoxLayout *hbox;
	QPixmap pm(":fceux1.png");
	QPixmap pm2;
	QLabel *lbl;

	pm2 = pm.scaled( 128, 128 );

	setWindowTitle( tr("About FCEUX11") );

	resize( 400, 320 );

	mainLayout = new QVBoxLayout();

	hbox = new QHBoxLayout();
	lbl = new QLabel();
	lbl->setPixmap(pm2);
	hbox->addWidget( lbl );
	hbox->setAlignment( Qt::AlignCenter );
	mainLayout->addLayout( hbox );

	hbox = new QHBoxLayout();
	versionLabel = new QLabel( tr("FCEUX11 v0.2.0") );
	hbox->addWidget( versionLabel );
	hbox->setAlignment( Qt::AlignCenter );
	mainLayout->addLayout( hbox );

	hbox = new QHBoxLayout();
	licenseLabel = new QLabel( tr("Based on FCEUX | License: GPLv2") );
	hbox->addWidget( licenseLabel );
	hbox->setAlignment( Qt::AlignCenter );
	mainLayout->addLayout( hbox );

	hbox = new QHBoxLayout();
	copyrightLabel = new QLabel( tr("\u00A9 2026 FCEUX11 Contributors") );
	hbox->addWidget( copyrightLabel );
	hbox->setAlignment( Qt::AlignCenter );
	mainLayout->addLayout( hbox );

	hbox = new QHBoxLayout();
	urlLabel = new QLabel();
	urlLabel->setText("<a href=\"https://github.com/Laffinty/FCEUX11\">https://github.com/Laffinty/FCEUX11</a>");
	urlLabel->setTextInteractionFlags(Qt::TextBrowserInteraction);
	urlLabel->setOpenExternalLinks(true);
	hbox->addWidget( urlLabel );
	hbox->setAlignment( Qt::AlignCenter );
	mainLayout->addLayout( hbox );

	hbox = new QHBoxLayout();
	viewLicenseButton = new QPushButton( tr("View License") );
	viewLicenseButton->setIcon(style()->standardIcon(QStyle::SP_DialogOpenButton));
	connect(viewLicenseButton, SIGNAL(clicked(void)), this, SLOT(openLicense(void)));
	hbox->addWidget( viewLicenseButton );
	hbox->setAlignment( Qt::AlignCenter );
	mainLayout->addLayout( hbox );

	closeButton = new QPushButton( tr("OK") );
	closeButton->setIcon(style()->standardIcon(QStyle::SP_DialogOkButton));
	connect(closeButton, SIGNAL(clicked(void)), this, SLOT(closeWindow(void)));

	hbox = new QHBoxLayout();
	hbox->addStretch(5);
	hbox->addWidget( closeButton, 1 );
	mainLayout->addLayout( hbox );

	setLayout( mainLayout );

	closeButton->setFocus();
	closeButton->setDefault(true);
}
//----------------------------------------------------------------------------
AboutWindow::~AboutWindow(void)
{

}
//----------------------------------------------------------------------------
void AboutWindow::retranslateUi(void)
{
	if (versionLabel) versionLabel->setText(tr("FCEUX11 v0.2.0"));
	if (licenseLabel) licenseLabel->setText(tr("Based on FCEUX | License: GPLv2"));
	if (copyrightLabel) copyrightLabel->setText(tr("\u00A9 2026 FCEUX11 Contributors"));
	if (viewLicenseButton) viewLicenseButton->setText(tr("View License"));
	if (closeButton) closeButton->setText(tr("OK"));
}
//----------------------------------------------------------------------------
void AboutWindow::openLicense(void)
{
	QDesktopServices::openUrl( QUrl::fromLocalFile("COPYING") );
}
//----------------------------------------------------------------------------
void AboutWindow::closeEvent(QCloseEvent *event)
{
	done(0);
	deleteLater();
	event->accept();
}
//----------------------------------------------------------------------------
void AboutWindow::closeWindow(void)
{
	done(0);
	deleteLater();
}
//----------------------------------------------------------------------------
void AboutWindow::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		setWindowTitle(tr("About FCEUX11"));
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//----------------------------------------------------------------------------