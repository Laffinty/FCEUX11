/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2026 FCEUX11 Contributors
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

// v2.0.8 C2/C3 O2 -- the diagnostics dialog.

#include "Qt/DiagnosticsDialog.h"

#include <QApplication>
#include <QClipboard>
#include <QDesktopServices>
#include <QFileInfo>
#include <QGuiApplication>
#include <QLabel>
#include <QUrl>
#include <QVBoxLayout>
#include <QHBoxLayout>

#include <filesystem>

#include "Qt/diagnostics_report.h"

namespace {

/// The directory that holds a file (empty when the path has no parent).
std::string parent_dir_of(const std::string& path)
{
	if (path.empty())
		return std::string();
	std::filesystem::path p(path);
	const auto parent = p.parent_path();
	return parent.empty() ? std::string() : parent.string();
}

bool dir_exists(const std::string& dir)
{
	if (dir.empty())
		return false;
	std::error_code ec;
	return std::filesystem::is_directory(dir, ec) && !ec;
}

} // anonymous namespace

DiagnosticsDialog_t::DiagnosticsDialog_t(QWidget *parent)
	: QDialog(parent)
{
	QVBoxLayout* mainLayout = new QVBoxLayout();

	setWindowTitle(tr("Diagnostics"));
	resize(640, 560);

	txtView = new QPlainTextEdit();
	txtView->setReadOnly(true);
	txtView->setLineWrapMode(QPlainTextEdit::NoWrap);
	mainLayout->addWidget(txtView);

	QHBoxLayout* hbox = new QHBoxLayout();
	copyBtn = new QPushButton(tr("&Copy Report"));
	logDirBtn = new QPushButton(tr("Open &Log Directory"));
	crashDirBtn = new QPushButton(tr("Open &Crash Directory"));
	closeBtn = new QPushButton(tr("Close"));

	hbox->addWidget(copyBtn);
	hbox->addWidget(logDirBtn);
	hbox->addWidget(crashDirBtn);
	hbox->addStretch(5);
	hbox->addWidget(closeBtn);
	mainLayout->addLayout(hbox);

	setLayout(mainLayout);

	connect(copyBtn, SIGNAL(clicked(void)), this, SLOT(copyReport(void)));
	connect(logDirBtn, SIGNAL(clicked(void)), this, SLOT(openLogDirectory(void)));
	connect(crashDirBtn, SIGNAL(clicked(void)), this, SLOT(openCrashDirectory(void)));
	connect(closeBtn, SIGNAL(clicked(void)), this, SLOT(closeWindow(void)));

	refresh();
}

DiagnosticsDialog_t::~DiagnosticsDialog_t()
{
}

void DiagnosticsDialog_t::refresh()
{
	const fceux11::DiagnosticsReport report = fceux11::gather_diagnostics();

	mLogDir = parent_dir_of(report.logFilePath);
	mCrashDir = report.crashDir;

	txtView->setPlainText(QString::fromStdString(report.to_text()));

	// A directory that does not exist cannot be opened -- grey the button
	// out instead of failing when it is clicked.
	logDirBtn->setEnabled(dir_exists(mLogDir));
	crashDirBtn->setEnabled(dir_exists(mCrashDir));

	txtView->moveCursor(QTextCursor::End);
}

void DiagnosticsDialog_t::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		setWindowTitle(tr("Diagnostics"));
		if (copyBtn) copyBtn->setText(tr("&Copy Report"));
		if (logDirBtn) logDirBtn->setText(tr("Open &Log Directory"));
		if (crashDirBtn) crashDirBtn->setText(tr("Open &Crash Directory"));
		if (closeBtn) closeBtn->setText(tr("Close"));
	}
	QDialog::changeEvent(event);
}

void DiagnosticsDialog_t::closeEvent(QCloseEvent* event)
{
	done(0);
	deleteLater();
	event->accept();
}

void DiagnosticsDialog_t::closeWindow(void)
{
	done(0);
	deleteLater();
}

void DiagnosticsDialog_t::copyReport(void)
{
	const fceux11::DiagnosticsReport report = fceux11::gather_diagnostics();
	QGuiApplication::clipboard()->setText(QString::fromStdString(report.to_text()));
}

void DiagnosticsDialog_t::openLogDirectory(void)
{
	if (mLogDir.empty())
		return;
	QDesktopServices::openUrl(QUrl::fromLocalFile(QString::fromStdString(mLogDir)));
}

void DiagnosticsDialog_t::openCrashDirectory(void)
{
	if (mCrashDir.empty())
		return;
	QDesktopServices::openUrl(QUrl::fromLocalFile(QString::fromStdString(mCrashDir)));
}
