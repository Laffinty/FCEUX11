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
#include <stdio.h>
#include <stdlib.h>
#include <exception>
#include <mutex>
#include <string>
#include <typeinfo>
#include <SDL.h>
#include <QApplication>
#include <QSplashScreen>
#include <QSettings>
#include <QSurfaceFormat>
#include <QTimer>
#include <QFile>
#include <QTranslator>
//#include <QProxyStyle>

#include "Qt/ConsoleWindow.h"
#include "Qt/fceuWrapper.h"
#include "Qt/SplashScreen.h"

#ifdef WIN32
#include <windows.h>
// Types only, and only after <windows.h>: dbghelp.h declares SYMBOL_INFOW in
// terms of HANDLE/BOOL/DWORD64, so including it earlier is a wall of errors.
// Every dbghelp *function* is reached through GetProcAddress in
// `fceux_diag::symbols`, so this must not become a link-time dependency.
#include <dbghelp.h>
#include <io.h>
#include <fcntl.h>
#if QT_VERSION < QT_VERSION_CHECK(6, 0, 0)
#include <QtPlatformHeaders/QWindowsWindowFunctions>
#endif
#endif

static void MessageOutput(QtMsgType type, const QMessageLogContext &context, const QString &msg)
{
    QByteArray localMsg = msg.toLocal8Bit();
    const char *file = context.file ? context.file : "";
    const char *function = context.function ? context.function : "";
    char cmsg[2048];
    switch (type) 
    {
       case QtDebugMsg:
           snprintf( cmsg, sizeof(cmsg), "Qt Debug: %s (%s:%u, %s)\n", localMsg.constData(), file, context.line, function);
	   FCEUD_Message(cmsg);
           break;
       case QtInfoMsg:
           snprintf( cmsg, sizeof(cmsg), "Qt Info: %s (%s:%u, %s)\n", localMsg.constData(), file, context.line, function);
	   FCEUD_Message(cmsg);
           break;
       case QtWarningMsg:
           snprintf( cmsg, sizeof(cmsg), "Qt Warning: %s (%s:%u, %s)\n", localMsg.constData(), file, context.line, function);
	   FCEUD_Message(cmsg);
           break;
       case QtCriticalMsg:
           snprintf( cmsg, sizeof(cmsg), "Qt Critical: %s (%s:%u, %s)\n", localMsg.constData(), file, context.line, function);
	   FCEUD_PrintError(cmsg);
           break;
       case QtFatalMsg:
           snprintf( cmsg, sizeof(cmsg), "Qt Fatal: %s (%s:%u, %s)\n", localMsg.constData(), file, context.line, function);
	   FCEUD_PrintError(cmsg);
           break;
    }
    cmsg[sizeof(cmsg)-1] = 0;
    fprintf(stderr, "%s", cmsg );
}


// This custom menu style wrapper used to prevent the menu bar from permanently stealing window focus when the ALT key is pressed.
//class MenuStyle : public QProxyStyle
//{
//public:
//    int styleHint(StyleHint stylehint, const QStyleOption *opt, const QWidget *widget, QStyleHintReturn *returnData) const
//    {
//        if (stylehint == QStyle::SH_MenuBar_AltKeyNavigation)
//            return 0;
//
//        return QProxyStyle::styleHint(stylehint, opt, widget, returnData);
//    }
//};


#undef main   // undef main in case SDL_Main

#include "Qt/ConfigStore.h"

static bool showSplashScreen(void)
{
	// v0.3.15.x PHASE-4: TypedConfig replaces the bare QSettings
	// call. The static const caches the key + default so subsequent
	// showSplashScreen() calls (called once per startup) don't pay
	// the string-literal / QSettings constructor cost twice.
	static const fceu11::qt::TypedConfig<bool> kShowSplash(
		"mainWindow/showSplashScreen", false);
	return kShowSplash.get();
}

// ---------------------------------------------------------------------------
// v2.0.1 S4 crash diagnostics -- TEMPORARY
//
// Two reproductions of "loading a .gba aborts the process" ended in a
// fast-fail 7 (`_FAST_FAIL_FATAL_APP_EXIT`, i.e. `abort()`) with **nothing on
// stdout or stderr**. A silent abort rules out the two explanations that would
// have been easy to fix: a Rust panic prints its `file:line` through the panic
// hook before aborting, and an uncaught C++ exception prints
// "terminate called after throwing..." through `std::terminate`. Neither
// appeared, so the abort is coming from a path that says nothing about
// itself, and reading the source did not converge on it.
//
// So the process is asked to say it itself. This block installs a terminate
// handler that records the exception (when there is one), a symbolised stack,
// and the module each frame belongs to, then aborts as before. The Rust side
// (`gba::install_diagnostic_hooks`) appends to the same file, named through
// `FCEUX11_DIAG_LOG` so both halves land in one place.
//
// The log is only written when something actually goes wrong, and this is
// meant to come back out once the fault is fixed.
// ---------------------------------------------------------------------------
namespace fceux_diag
{
	// Beside the executable, so it is found next to the thing that died
	// rather than wherever the working directory happened to be.
	std::string log_path()
	{
		static std::string cached = []
		{
			char buf[MAX_PATH] = { 0 };
			DWORD n = GetModuleFileNameA(nullptr, buf, MAX_PATH);
			std::string dir = (n > 0) ? std::string(buf, buf + n) : std::string();
			const size_t slash = dir.find_last_of("\\/");
			dir = (slash == std::string::npos) ? std::string(".") : dir.substr(0, slash);
			return dir + "\\fceux_diag.log";
		}();
		return cached;
	}

	// Append-only, opened per call. A crash handler cannot assume a stream is
	// healthy, and reopening is what makes this survive the case it exists for.
	void append(const std::string& text)
	{
		static std::mutex io_lock;
		std::lock_guard<std::mutex> guard(io_lock);
		if (FILE* f = std::fopen(log_path().c_str(), "ab"))
		{
			std::fwrite(text.data(), 1, text.size(), f);
			std::fflush(f);
			std::fclose(f);
		}
	}

	// dbghelp is loaded by hand rather than linked: this file must not grow a
	// link-time dependency on a library that exists only to explain a crash.
	struct Symbols
	{
		HMODULE dll = nullptr;
		BOOL (WINAPI* init)(HANDLE, PCSTR, BOOL) = nullptr;
		DWORD (WINAPI* options)(DWORD) = nullptr;
		BOOL (WINAPI* from_addr)(HANDLE, DWORD64, DWORD64*, PSYMBOL_INFOW) = nullptr;
		BOOL (WINAPI* line_from_addr)(HANDLE, DWORD64, PDWORD64, PIMAGEHLP_LINE64) = nullptr;
		bool ready = false;
	};

	Symbols& symbols()
	{
		static Symbols s;
		if (!s.ready)
		{
			s.ready = true;  // set first: no re-entry from the loader
			s.dll = LoadLibraryA("dbghelp.dll");
			if (s.dll)
			{
				s.init = reinterpret_cast<decltype(s.init)>(GetProcAddress(s.dll, "SymInitialize"));
				s.options = reinterpret_cast<decltype(s.options)>(GetProcAddress(s.dll, "SymSetOptions"));
				s.from_addr = reinterpret_cast<decltype(s.from_addr)>(GetProcAddress(s.dll, "SymFromAddrW"));
				s.line_from_addr = reinterpret_cast<decltype(s.line_from_addr)>(GetProcAddress(s.dll, "SymGetLineFromAddr64"));
				if (s.init && s.options && s.from_addr && s.line_from_addr)
				{
					s.options(SYMOPT_UNDNAME | SYMOPT_LOAD_LINES | SYMOPT_FAIL_CRITICAL_ERRORS);
					char exe[MAX_PATH] = { 0 };
					GetModuleFileNameA(nullptr, exe, MAX_PATH);
					s.init(GetCurrentProcess(), exe, TRUE);
				}
			}
		}
		return s;
	}

	void write_backtrace()
	{
		Symbols& sym = symbols();
		void* frames[62] = { nullptr };
		const USHORT n = CaptureStackBackTrace(1, 62, frames, nullptr);

		char line[1024];
		std::snprintf(line, sizeof(line), "  backtrace: %u frames (dbghelp %s)\r\n",
			(unsigned)n, sym.from_addr ? "loaded" : "UNAVAILABLE");
		append(line);

		for (USHORT i = 0; i < n; ++i)
		{
			const DWORD64 addr = reinterpret_cast<DWORD64>(frames[i]);
			wchar_t symbol[512] = { 0 };
			char file[MAX_PATH] = "?";
			unsigned file_line = 0;
			DWORD64 mod_base = 0;

			// The symbol name and the file:line come from two different calls,
			// because SYMBOL_INFOW has no FileName/LineNumber at all -- those
			// live in IMAGEHLP_LINE64. The module is identified by its base
			// address rather than by name: neither struct exposes a module name
			// that is available here, and base+offset is enough to place a frame.
			if (sym.from_addr)
			{
				// SYMBOL_INFOW is 88 bytes on x64 and its `Name[1]` has to sit
				// in the same allocation, hence the trailing scratch space.
				alignas(8) char buf[sizeof(SYMBOL_INFOW) + 512] = { 0 };
				SYMBOL_INFOW* info = reinterpret_cast<SYMBOL_INFOW*>(buf);
				info->SizeOfStruct = sizeof(SYMBOL_INFOW);
				info->MaxNameLen = 511;
				DWORD64 disp = 0;
				if (sym.from_addr(GetCurrentProcess(), addr, &disp, info))
				{
					if (info->Name) wcscpy_s(symbol, 512, info->Name);
					mod_base = info->ModBase;
				}
			}
			if (sym.line_from_addr)
			{
				// This SDK's IMAGEHLP_LINE64 carries only SizeOfStruct, Key,
				// LineNumber, FileName and Address -- the module base is only
				// available from SYMBOL_INFOW, so it is not read here.
				IMAGEHLP_LINE64 info = { 0 };
				info.SizeOfStruct = sizeof(IMAGEHLP_LINE64);
				DWORD64 disp = 0;
				if (sym.line_from_addr(GetCurrentProcess(), addr, &disp, &info))
				{
					if (info.FileName) strcpy_s(file, MAX_PATH, info.FileName);
					file_line = (unsigned)info.LineNumber;
				}
			}

			if (symbol[0] || file_line)
			{
				char module[64];
				if (mod_base)
				{
					std::snprintf(module, sizeof(module), "mod+%llX",
						(unsigned long long)(addr - mod_base));
				}
				else
				{
					std::strcpy(module, "?");
				}
				std::snprintf(line, sizeof(line), "    #%02u %016llX  [%s]  %s  %s:%u\r\n",
					(unsigned)i, (unsigned long long)addr, module,
					symbol[0] ? "fn" : "  ", file, file_line);
			}
			else
			{
				// Without symbols the raw address still bounds the search: it can
				// be compared against the module list in a fresh crash dump.
				std::snprintf(line, sizeof(line), "    #%02u %016llX  (no symbols)\r\n",
					(unsigned)i, (unsigned long long)addr);
			}
			append(line);
		}
	}

	void terminate_handler()
	{
		char head[128];
		std::snprintf(head, sizeof(head), "\r\n=== C++ std::terminate  thread %lu ===\r\n",
			(unsigned long)GetCurrentThreadId());
		append(head);

		// Re-throwing inside the handler is legal and is the only way to learn
		// *what* was in flight: the handler is called with the exception
		// already active, and rethrow_exception is the documented way to reach
		// it without a nested catch being required.
		try
		{
			if (std::current_exception())
			{
				std::rethrow_exception(std::current_exception());
			}
		}
		catch (const std::exception& e)
		{
			char buf[1024];
			std::snprintf(buf, sizeof(buf), "  exception: std::exception: %s\r\n", e.what());
			append(buf);
		}
		catch (...)
		{
			append("  exception: (not derived from std::exception)\r\n");
		}

		write_backtrace();
		append("  (no message on stderr => not an uncaught C++ exception; see notes)\r\n");
		std::abort();
	}

	void install()
	{
		// The Rust half reads this to find the same file.
		SetEnvironmentVariableA("FCEUX11_DIAG_LOG", log_path().c_str());
		std::set_terminate(terminate_handler);
	}
}  // namespace fceux_diag

int main( int argc, char *argv[] )
{
	int retval = 0;

	fceux_diag::install();

	// Set the default QSurfaceFormat BEFORE creating QApplication. The v0.3.14
	// OpenGL backend uses QOpenGLWindow + #version 330 core shaders, which
	// require a Core Profile OpenGL 3.3 context. Without this default format,
	// Qt falls back to a Compatibility Profile 2.x context and the
	// initializeOpenGLFunctions() call for QOpenGLFunctions_3_3_Core fails.
	{
		QSurfaceFormat defaultFmt;
		defaultFmt.setRenderableType(QSurfaceFormat::OpenGL);
		defaultFmt.setProfile(QSurfaceFormat::CoreProfile);
		defaultFmt.setVersion(3, 3);
		defaultFmt.setSwapBehavior(QSurfaceFormat::DoubleBuffer);
		defaultFmt.setSwapInterval(1);
		QSurfaceFormat::setDefaultFormat(defaultFmt);
	}

	fceuWrapperPreInit(argc, argv);

	qInstallMessageHandler(MessageOutput);

#if QT_VERSION < QT_VERSION_CHECK(6, 0, 0)
	QApplication::setAttribute(Qt::AA_EnableHighDpiScaling);
	QApplication::setAttribute(Qt::AA_UseHighDpiPixmaps);
#endif
	// v0.3.15.x PHASE-4: rounding policy under review for Win11 24H2
	// multi-monitor scenarios. PassThrough gives the most accurate
	// per-monitor scaling (no Qt-side rounding) at the cost of fuzzy
	// pixels when the per-monitor scale factor is not an integer
	// (e.g. 125% / 150% / 175%). RoundPreferFloor would avoid the
	// fuzziness on those fractional scales but produces slightly
	// mismatched geometry on mixed-DPI multi-monitor setups.
	//
	// Decision (2026-06-17): keep PassThrough. The @2x.png resource
	// set generated by PHASE-4 task 4.2 makes 200% scaling sharp
	// (where it matters most for QA / debugging). The fuzzy-pixel
	// concern is bounded to 125/150/175% scales, where 32x32 icons
	// scaled to 40/48/56 pixels are visually acceptable; the NES
	// emulator viewport is the user-visible high-fidelity surface
	// and is rendered at native framebuffer resolution regardless
	// of the policy.
	QApplication::setHighDpiScaleFactorRoundingPolicy(Qt::HighDpiScaleFactorRoundingPolicy::PassThrough);

	QApplication app(argc, argv);

	// v0.3.15 PR-B: CJK fallback chain so that traditional/simplified
	// CJK glyphs render in the UI even when Segoe UI Variable lacks
	// the specific code point. Microsoft YaHei UI is the Windows 11
	// default CJK UI font; Noto Sans CJK SC is a free alternative.
	QFont font("Segoe UI Variable", 9);
	font.setStyleHint(QFont::SansSerif);
	font.setFamilies(QStringList{"Segoe UI Variable",
	                             "Microsoft YaHei UI",
	                             "Microsoft YaHei",
	                             "Noto Sans CJK SC"});
	QApplication::setFont(font);

	QApplication::setAttribute(Qt::AA_DontShowShortcutsInContextMenus, false);

	#ifdef WIN32
	// v0.3.15.x PHASE-3: detect system dark mode via the Win10 1809+
	// uxtheme.dll!ShouldAppsUseDarkMode (ordinal 132) instead of the
	// QSettings registry path. The native API is faster (no registry
	// round-trip) and survives if the Personalize key is ever renamed
	// in a future Windows release. Falls back to the registry path on
	// Windows builds that do not export the ordinal.
	bool isDarkMode = false;
	bool usedNativeApi = false;
	{
		HMODULE hUxtheme = LoadLibraryW(L"uxtheme.dll");
		if (hUxtheme != nullptr) {
			using ShouldAppsUseDarkModeFn = bool (WINAPI*)();
			auto pfn = reinterpret_cast<ShouldAppsUseDarkModeFn>(
				GetProcAddress(hUxtheme, MAKEINTRESOURCEA(132)));
			if (pfn != nullptr) {
				isDarkMode = pfn();
				usedNativeApi = true;
			}
			FreeLibrary(hUxtheme);
		}
		if (!usedNativeApi) {
			// Fallback: registry detection for Win10 1809- / shells
			// that do not export the ordinal. This branch runs at
			// most once per process (on startup).
			QSettings themeSettings(
				"HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
				QSettings::NativeFormat);
			isDarkMode = themeSettings.value("AppsUseLightTheme", 1).toInt() == 0;
		}
	}
	if (isDarkMode)
	{
		QFile styleSheetFile(":/styles/dark.qss");
		if (styleSheetFile.open(QFile::ReadOnly))
		{
			QString styleSheet = QLatin1String(styleSheetFile.readAll());
			app.setStyleSheet(styleSheet);
		}
	}
	#endif

	QCoreApplication::setOrganizationName("TasEmulators");
	QCoreApplication::setOrganizationDomain("TasEmulators.org");
	QCoreApplication::setApplicationName("fceux");

	// Auto-detect system language preference for all 12 supported languages.
	// v1.11 §11.5: extended from zh_CN/zh_TW only to all 12 UI languages so the
	// early translator (splash screen) loads the correct .qm on first launch.
	auto detectSystemLang = []() -> QString {
		QLocale sys = QLocale::system();
		QLocale::Language lang = sys.language();

		if (lang == QLocale::Chinese) {
			QLocale::Script script = sys.script();
			if (script == QLocale::SimplifiedHanScript) {
				return QStringLiteral("zh_CN");
			}
			if (script == QLocale::TraditionalHanScript) {
				return QStringLiteral("zh_TW");
			}
			const QStringList uiLangs = sys.uiLanguages();
			for (const QString &l : uiLangs) {
				QString low = l.toLower();
				if (low.startsWith("zh-cn") || low.startsWith("zh-hans") ||
					low.startsWith("zh-sg") || low.startsWith("zh-my")) {
					return QStringLiteral("zh_CN");
				}
				if (low.startsWith("zh-tw") || low.startsWith("zh-hk") ||
					low.startsWith("zh-mo") || low.startsWith("zh-hant")) {
					return QStringLiteral("zh_TW");
				}
			}
			QString name = sys.name().toLower();
			if (name.startsWith("zh_cn") || name.startsWith("zh_hans") ||
				name.startsWith("zh_sg") || name.startsWith("zh_my")) {
				return QStringLiteral("zh_CN");
			}
			if (name.startsWith("zh_tw") || name.startsWith("zh_hk") ||
				name.startsWith("zh_mo") || name.startsWith("zh_hant")) {
				return QStringLiteral("zh_TW");
			}
			return QStringLiteral("zh_CN");
		}

		if (lang == QLocale::Japanese)  return QStringLiteral("ja");
		if (lang == QLocale::Korean)    return QStringLiteral("ko");
		if (lang == QLocale::Spanish)   return QStringLiteral("es");
		if (lang == QLocale::French)    return QStringLiteral("fr");
		if (lang == QLocale::German)    return QStringLiteral("de");
		if (lang == QLocale::Vietnamese) return QStringLiteral("vi");
		if (lang == QLocale::Thai)      return QStringLiteral("th");
		if (lang == QLocale::Hindi)     return QStringLiteral("hi");
		if (lang == QLocale::Arabic)    return QStringLiteral("ar");

		return QStringLiteral("en");
	};
	QString defaultLang = detectSystemLang();
	qDebug("i18n: auto-detected system language = %s", qUtf8Printable(defaultLang));

	// Load saved language preference, or use auto-detected default
	// v0.3.15.x PHASE-4: TypedConfig<QString> replaces bare QSettings
	// value() call. The default value is the auto-detected language;
	// the static const caches the key, the default-override is
	// applied at call time so the auto-detect still wins on first
	// run.
	//
	// PHASE-5: A saved value of "" or "auto" re-runs auto-detection
	// every startup. This lets users who flip Windows regional
	// settings recover Chinese UI without manually re-picking the
	// language in the Options menu.
	QString savedLang;
	{
		static const fceu11::qt::TypedConfig<QString> kLanguage(
			"General/Language", defaultLang);
		savedLang = kLanguage.get();
	}
	if (savedLang.isEmpty() || savedLang.compare("auto", Qt::CaseInsensitive) == 0) {
		savedLang = defaultLang;
	}
	extern QTranslator *g_earlyTranslator;
	g_earlyTranslator = nullptr;
	if (savedLang != "en")
	{
		g_earlyTranslator = new QTranslator(&app);
		QStringList candidates;
		candidates << QString(":/i18n/fceux11_%1.qm").arg(savedLang);
		candidates << QString(":/i18n/fceux11_%1").arg(savedLang);
		QString exeDir = QCoreApplication::applicationDirPath();
		candidates << exeDir + "/lang/fceux11_" + savedLang + ".qm";
		candidates << exeDir + "/i18n/fceux11_" + savedLang + ".qm";
		candidates << exeDir + "/../share/fceux11/i18n/fceux11_" + savedLang + ".qm";
		candidates << exeDir + "/../lang/fceux11_" + savedLang + ".qm";
		bool loaded = false;
		for (const QString &path : candidates) {
			if (g_earlyTranslator->load(path)) {
				loaded = true;
				qDebug("i18n: early translator loaded %s", qUtf8Printable(path));
				break;
			}
		}
		if (loaded) {
			app.installTranslator(g_earlyTranslator);
		} else {
			qWarning("i18n: early translator failed to load fceux11_%s.qm; UI will start in English.",
			         qUtf8Printable(savedLang));
			delete g_earlyTranslator;
			g_earlyTranslator = nullptr;
		}
	}

	fceuSplashScreen *splash = NULL;

	if ( showSplashScreen() )
	{
		splash = new fceuSplashScreen();
		splash->show();
		app.processEvents();
	}

	#ifdef WIN32
	// Attach to parent console if available, so that printf() / qDebug()
	// output is visible when launched from a console (cmd.exe, PowerShell).
	//
	// Defensive: when the parent process is a pseudo-tty (e.g. MSYS2 mintty,
	// WSL, or any non-Windows console), AttachConsole may claim success
	// (returning non-zero) but the underlying CONOUT$ handle is invalid or
	// shares a file descriptor with stderr. Calling freopen() on the shared
	// stream in that situation crashes the process.
	//
	// Guard: only redirect when GetConsoleWindow() is non-NULL after attach,
	// which indicates a real Windows console (not a pseudo-tty).
	//
	// v0.3.15.x PHASE-3: --no-console command-line argument skips the
	// entire AttachConsole + freopen block. The flag is parsed in
	// fceuWrapperPreInit (which runs before main reaches this point).
	extern bool g_noConsole;
	if (!g_noConsole && AttachConsole(ATTACH_PARENT_PROCESS) && GetConsoleWindow() != NULL)
	{
		HANDLE hConOut = CreateFileA("CONOUT$", GENERIC_WRITE, FILE_SHARE_WRITE,
		                            NULL, OPEN_EXISTING, 0, NULL);
		if (hConOut != INVALID_HANDLE_VALUE)
		{
			// hotfix1 P1-9 (C-10): the previous version reused the same
			// `hConOut` handle for both stdout and stderr via two
			// `_open_osfhandle` calls. When stdout's FILE* was later
			// fclose()d, the CRT pulled the OS handle out from under
			// stderr — subsequent writes to stderr would then write to
			// an already-freed handle (use-after-free). Duplicate into
			// two independent kernel objects so each FILE* owns its own
			// handle and can be closed without affecting the other.
			HANDLE hStdout = INVALID_HANDLE_VALUE;
			HANDLE hStderr = INVALID_HANDLE_VALUE;
			if (!DuplicateHandle(GetCurrentProcess(), hConOut,
			                     GetCurrentProcess(), &hStdout,
			                     0, TRUE, DUPLICATE_SAME_ACCESS) ||
			    !DuplicateHandle(GetCurrentProcess(), hConOut,
			                     GetCurrentProcess(), &hStderr,
			                     0, TRUE, DUPLICATE_SAME_ACCESS)) {
				CloseHandle(hConOut);
				if (hStdout != INVALID_HANDLE_VALUE) CloseHandle(hStdout);
				if (hStderr != INVALID_HANDLE_VALUE) CloseHandle(hStderr);
			} else {
				CloseHandle(hConOut);

				int conFd = _open_osfhandle((intptr_t)hStdout, _O_TEXT);
				if (conFd != -1)
				{
					FILE *conFile = _fdopen(conFd, "w");
					if (conFile != NULL)
					{
						*stdout = *conFile;
						setvbuf(stdout, NULL, _IONBF, 0);
					}
				}
				int conFd2 = _open_osfhandle((intptr_t)hStderr, _O_TEXT);
				if (conFd2 != -1)
				{
					FILE *conFile2 = _fdopen(conFd2, "w");
					if (conFile2 != NULL)
					{
						*stderr = *conFile2;
						setvbuf(stderr, NULL, _IONBF, 0);
					}
				}
			}
		}
	}
	#endif
	//app.setStyle( new MenuStyle() );

	//styleSheetEnv = ::getenv("FCEUX_QT_STYLESHEET");
	//
	//if ( styleSheetEnv != NULL )
	//{
	//   QFile File(styleSheetEnv);
	//
	//   if ( File.open(QFile::ReadOnly) )
	//   {
	//      QString StyleSheet = QLatin1String(File.readAll());
	//
	//      app.setStyleSheet(StyleSheet);
	//
	//      printf("Using Qt Stylesheet file '%s'\n", styleSheetEnv );
	//   }
	//   else
	//   {
	//      printf("Warning: Could not open Qt Stylesheet file '%s'\n", styleSheetEnv );
	//   }
	//}

	fceuWrapperInit( argc, argv );

	// CRITICAL: Pump SDL events from the MAIN (Qt) thread.
	//
	// On Windows, keyboard (WM_INPUT) and other raw-input messages arrive in
	// the foreground window's THREAD message queue. Qt is running the main
	// event loop on the main thread, so Qt pumps those messages — but Qt
	// does not know about SDL and never translates WM_INPUT to SDL events.
	// The emulator thread's SDL_PumpEvents() can only drain its OWN thread
	// queue, so SDL's keyboard (and similar) event queue stays empty, and
	// no key press is ever delivered to fceu11's UpdatePhysicalInput().
	//
	// This regressed in v0.3.14: the OpenGL backend migrated from
	// QOpenGLWidget to QOpenGLWindow + QWidget::createWindowContainer. With
	// QOpenGLWidget, the widget was embedded inside the QMainWindow so Qt
	// routed key events through the QMainWindow's QWidget::keyPressEvent,
	// which fed g_keyState. With QOpenGLWindow, the GL surface is a separate
	// top-level QWindow (with its own HWND), Qt no longer routes keys to
	// fceu11's keyboard handling, and the only path is via SDL — which
	// requires the main thread to pump events.
	//
	// Fix: a 0-ms QTimer fires every event-loop iteration, calling
	// SDL_PumpEvents() on the main thread. This drains Windows messages
	// from the main thread's queue and converts WM_INPUT into SDL events,
	// which are then readable via SDL_PollEvent() from the emulator thread
	// (where UpdatePhysicalInput runs).
	QTimer *sdlPumpTimer = new QTimer(&app);
	// hotfix3 B-6: explicit Qt::QueuedConnection. The timer and the receiver
	// lambda both live on the main thread today, so AutoConnection would pick
	// DirectConnection. Pinning QueuedConnection is defensive: if the SDL pump
	// is ever relocated off the main thread (the hotfix1 P2-15 comment in this
	// file already contemplates this risk), the call still marshals correctly.
	// The 4-arg `connect(sender, signal, context, slot, type)` form is required:
	// Qt's overload set does not accept a bare lambda + ConnectionType without
	// a context object to anchor the receiver's thread affinity.
	QObject::connect(sdlPumpTimer, &QTimer::timeout, &app, []() {
		SDL_PumpEvents();
	}, Qt::QueuedConnection);
	// hotfix1 P2-15 (N-L01): SDL_PumpEvents() must not be called before
	// SDL_Init() succeeds — calling it on an uninitialised subsystem is
	// undefined per SDL2 docs (some platforms deref a null internal
	// subsystem pointer). fceuWrapperInit above runs SDL_Init(SDL_INIT_VIDEO);
	// confirm that subsystem came up before arming the timer. If it
	// didn't, skip starting the timer so we don't crash; the fceuWrapper
	// path will already have exit()ed in that case but the runtime
	// check is cheap insurance against any future refactor that moves
	// the init call.
	if (SDL_WasInit(SDL_INIT_VIDEO) != 0) {
		sdlPumpTimer->start(0);
	} else {
		printf("Warning: SDL_INIT_VIDEO not active; skipping SDL pump timer.\n");
	}

	consoleWindow = new consoleWin_t();

	consoleWindow->show();

	// Need to wait for window to initialize before video init can be called.
	//consoleWindow->videoInit();

#ifdef WIN32
	// This function is needed to fix the issue referenced below. It adds a 1-pixel border
	// around the fullscreen window due to some limitation in windows.
	// https://doc.qt.io/qt-5/windows-issues.html#fullscreen-opengl-based-windows
#if QT_VERSION < QT_VERSION_CHECK(6, 0, 0)
	QWindowsWindowFunctions::setHasBorderInFullScreen( consoleWindow->windowHandle(), true);
#endif
#endif

	if ( splash )
	{
		splash->finish( consoleWindow );
		//delete splash; this is handled by Qt event loop
	}

	retval = app.exec();

	//printf("App Return: %i \n", retval );

	delete consoleWindow;

	fceuWrapperMemoryCleanup();

	return retval;
}

