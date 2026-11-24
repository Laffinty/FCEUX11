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

#ifndef __FCEUX11_DIAGNOSTICS_DIALOG_H_
#define __FCEUX11_DIAGNOSTICS_DIALOG_H_

#include <QDialog>
#include <QPlainTextEdit>
#include <QPushButton>

class DiagnosticsDialog_t : public QDialog
{
	Q_OBJECT

public:
	DiagnosticsDialog_t(QWidget *parent = 0);
	~DiagnosticsDialog_t();

	/// Rebuilds the report and refreshes the view.
	void refresh();

	/// hotfix4 D-15 pattern: retranslate on a language switch.
	void changeEvent(QEvent *event) override;

protected:
	void closeEvent(QCloseEvent *event) override;

public slots:
	void closeWindow(void);
	void copyReport(void);
	void openLogDirectory(void);
	void openCrashDirectory(void);

private:
	QPlainTextEdit* txtView = nullptr;
	QPushButton* copyBtn = nullptr;
	QPushButton* logDirBtn = nullptr;
	QPushButton* crashDirBtn = nullptr;
	QPushButton* closeBtn = nullptr;

	std::string mLogDir;
	std::string mCrashDir;
};

#endif // __FCEUX11_DIAGNOSTICS_DIALOG_H_
