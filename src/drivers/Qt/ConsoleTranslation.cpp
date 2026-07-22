// ConsoleEmuControl.cpp
//

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
// ConsoleWindow.cpp
//
#include <fstream>
#include <iostream>
#include <cstdlib>

#include <QPixmap>
#include <QWindow>
#include <QScreen>
#include <QSettings>

// v0.3.15.x PHASE-4: TypedConfig<T> wrapper for QSettings.
#include "ConfigStore.h"
#include <QHeaderView>
#include <QFileInfo>
#include <QFileDialog>
#include <QMessageBox>
#include <QInputDialog>
#include <QTranslator>
#include <QActionGroup>
#include <QSignalBlocker>
#include <QDesktopServices>
#include <QStyleFactory>
#include <QApplication>
#include <QShortcut>
#include <QUrl>

#include "../../fceu.h"
#include "../../fds.h"
#include "../../file.h"
#include "../../input.h"
#include "../../movie.h"
#include "../../wave.h"
#include "../../state.h"
#include "../../profiler.h"
#include "../../version.h"
#include "../../core_api.h"
#include "common/os_utils.h"
#include "utils/timeStamp.h"

#ifdef _S9XLUA_H
#include "../../fceulua.h"
#endif

#include "Qt/main.h"
#include "Qt/dface.h"
#include "Qt/input.h"
#include "Qt/ColorMenu.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/InputConf.h"
#include "Qt/GamePadConf.h"
#include "Qt/FamilyKeyboard.h"
#include "Qt/HotKeyConf.h"
#include "Qt/PaletteConf.h"
#include "Qt/PaletteEditor.h"
#include "Qt/HelpPages.h"
#include "Qt/GuiConf.h"
#include "Qt/AviRecord.h"
#include "Qt/AviRiffViewer.h"
#include "Qt/MoviePlay.h"
#include "Qt/MovieRecord.h"
#include "Qt/MovieOptions.h"
#include "Qt/StateRecorderConf.h"
#include "Qt/TimingConf.h"
#include "Qt/FrameTimingStats.h"
#include "Qt/LuaControl.h"
#include "Qt/CheatsConf.h"
#include "Qt/GameGenie.h"
#include "Qt/HexEditor.h"
#include "Qt/TraceLogger.h"
#include "Qt/CodeDataLogger.h"
#include "Qt/ConsoleDebugger.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/ConsoleSoundConf.h"
#include "Qt/ConsoleVideoConf.h"
#include "Qt/MsgLogViewer.h"
#include "Qt/AboutWindow.h"
#include "Qt/fceuWrapper.h"
#include "Qt/ppuViewer.h"
#include "Qt/NameTableViewer.h"
#include "Qt/iNesHeaderEditor.h"
#include "Qt/RamWatch.h"
#include "Qt/RamSearch.h"
#include "Qt/keyscan.h"
#include "Qt/nes_shm.h"
#include "Qt/TasEditor/TasEditorWindow.h"


#include "Qt/ConsoleTranslation.h"
#include "Qt/ConsoleWindow.h"

static QTranslator *appTranslator = nullptr;
QTranslator *g_earlyTranslator = nullptr;

// v0.3.15.x PHASE-5 fix: previously, if the embedded :/i18n/...
// resource was missing (e.g. the qt_add_resources() target was
// not linked into the executable, or the .qm was deleted from
// the build tree), QTranslator::load() silently returned false
// and the UI stayed in English. We now try the resource first,
// then fall back to a sibling-of-exe lookup
// ("<exe-dir>/lang/fceux11_<lang>.qm" and a few common dev/build
// locations) so a half-installed build still translates when run
// from the build output tree. We also emit a qWarning() so the
// failure is visible in --debug logs.
static bool loadQmWithFallback(QTranslator *translator, const QString &langCode)
{
	QStringList candidates;
	candidates << QString(":/i18n/fceux11_%1.qm").arg(langCode);
	candidates << QString(":/i18n/fceux11_%1").arg(langCode);

	const QApplication *app = qobject_cast<QApplication *>(QCoreApplication::instance());
	if (app) {
		QString exeDir = QCoreApplication::applicationDirPath();
		candidates << exeDir + "/lang/fceux11_" + langCode + ".qm";
		candidates << exeDir + "/i18n/fceux11_" + langCode + ".qm";
		candidates << exeDir + "/../share/fceux11/i18n/fceux11_" + langCode + ".qm";
		candidates << exeDir + "/../lang/fceux11_" + langCode + ".qm";
	}

	for (const QString &path : candidates) {
		if (translator->load(path)) {
			qDebug("i18n: loaded %s", qUtf8Printable(path));
			return true;
		}
	}
	qWarning("i18n: failed to load any fceux11_%s.qm candidate; UI will stay in English.",
	         qUtf8Printable(langCode));
	return false;
}

void consoleWin_t::loadTranslation(const QString &langCode)
{
	if (!appTranslator)
	{
		appTranslator = new QTranslator(qApp);
	}
	qApp->removeTranslator(appTranslator);

	if (g_earlyTranslator)
	{
		qApp->removeTranslator(g_earlyTranslator);
		delete g_earlyTranslator;
		g_earlyTranslator = nullptr;
	}

	if (loadQmWithFallback(appTranslator, langCode))
	{
		qApp->installTranslator(appTranslator);
	}

	// v1.11 §11.5.4: RTL layout for Arabic
	if (langCode == QStringLiteral("ar")) {
		qApp->setLayoutDirection(Qt::RightToLeft);
	} else {
		qApp->setLayoutDirection(Qt::LeftToRight);
	}

	// Save preference (skip the QSettings write for the implicit
	// "en" sentinel so the auto-detect path keeps winning on
	// future startups when the user has not explicitly chosen
	// a language).
	// v0.3.15.x PHASE-4: TypedConfig<QString>::set replaces bare
	// QSettings::setValue. Same key path, same value, same
	// QSettings backend, no behavioural change.
	if (!langCode.isEmpty()) {
		static const fceu11::qt::TypedConfig<QString> kLanguage(
			"General/Language", QStringLiteral("en"));
		kLanguage.set(langCode);
	}

	// Update checkmark on language actions (block signals to prevent setChecked from triggering loadTranslation)
	if (languageActionGroup) {
		QSignalBlocker blocker(languageActionGroup);
		for (auto action : languageActionGroup->actions())
		{
			action->setChecked(action->data().toString() == langCode);
		}
	}

	retranslateUi();
}
