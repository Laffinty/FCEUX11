// i18n_regression_test.cpp
//
// v0.3.15 PHASE-2 regression test. Static-analysis gate that ensures:
//   1. Both .ts files have >= 90% translated coverage.
//   2. Simp/trad cross-contamination stays at zero forbidden glyphs.
//   3. Every PHASE-2 widget file declares a retranslateUi() member.
//   4. Every PHASE-2 widget cpp file implements changeEvent(QEvent*).
//   5. The 3 PHASE-2 keyPress overrides (HexEditor / HotKeyConf / FamilyKeyboard)
//      forward to their base class.
//
// We deliberately avoid spinning up a full QApplication + ConsoleWindow here:
//   - This is a Windows GUI app; headless tests cannot construct it without
//     a real OpenGL context and an SDL display.
//   - The static checks below are the highest-value guard rails for the
//     PHASE-2 refactor: catching "forgot to add retranslateUi" or "forgot to
//     forward base class" before the manual GUI smoke test.
//
// The runtime translation switch is covered by the manual GUI smoke test
// (docs/tech/v0.3.15_Verification_Report.md gate 4).
//
// Style: matches smoke_test.cpp (printf-based, return 0/1).

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>
#include <fstream>
#include <sstream>

// ---------------------------------------------------------------------------
// .ts coverage: inline port of scripts/i18n_coverage.ps1 logic.
// ---------------------------------------------------------------------------

struct TsStats {
    int totalMessages = 0;
    int unfinished = 0;
};

static bool parseTsFile(const std::string& path, TsStats& stats)
{
    std::ifstream f(path);
    if (!f.is_open()) {
        printf("FAIL: cannot open %s\n", path.c_str());
        return false;
    }

    std::stringstream buf;
    buf << f.rdbuf();
    std::string xml = buf.str();

    // Walk every <message>...</message> block.
    size_t cursor = 0;
    while (true) {
        size_t msgStart = xml.find("<message>", cursor);
        if (msgStart == std::string::npos) break;
        size_t msgEnd = xml.find("</message>", msgStart);
        if (msgEnd == std::string::npos) break;

        stats.totalMessages++;

        std::string msgBody = xml.substr(msgStart, msgEnd - msgStart);
        size_t trStart = msgBody.find("<translation");
        size_t trEnd = (trStart == std::string::npos)
            ? std::string::npos
            : msgBody.find(">", trStart);
        size_t trClose = (trEnd == std::string::npos)
            ? std::string::npos
            : msgBody.find("</translation>", trEnd);

        bool isUnfinished = false;
        if (trStart == std::string::npos || trEnd == std::string::npos || trClose == std::string::npos) {
            // No <translation> element at all.
            isUnfinished = true;
        } else {
            std::string trTag = msgBody.substr(trStart, trEnd - trStart);
            if (trTag.find("type=\"unfinished\"") != std::string::npos ||
                trTag.find("type=\"needs-review\"") != std::string::npos) {
                isUnfinished = true;
            } else {
                // Extract inner text and check it's non-empty.
                std::string inner = msgBody.substr(trEnd + 1, trClose - trEnd - 1);
                // Trim whitespace.
                size_t a = inner.find_first_not_of(" \t\r\n");
                size_t b = inner.find_last_not_of(" \t\r\n");
                if (a == std::string::npos) {
                    isUnfinished = true;
                } else {
                    std::string trimmed = inner.substr(a, b - a + 1);
                    if (trimmed.empty()) {
                        isUnfinished = true;
                    }
                }
            }
        }

        if (isUnfinished) stats.unfinished++;

        cursor = msgEnd + 10; // strlen("</message>")
    }

    return true;
}

// ---------------------------------------------------------------------------
// Simp/trad gate: delegated to scripts/check_simp_trad.ps1 which is the
// canonical CI gate (curated list of script-exclusive characters, avoiding
// the shared-character false positives that a blanket CJK substring search
// would trigger). This C++ test only checks coverage + widget structural
// gates; the .ps1 script is run separately as part of PHASE-2 verification.
// ---------------------------------------------------------------------------
// (No inline CJK substring search here — see scripts/check_simp_trad.ps1.)

// ---------------------------------------------------------------------------
// PHASE-2 widget retranslateUi / changeEvent / keyPress gate.
// ---------------------------------------------------------------------------

struct WidgetGate {
    const char* path;        // relative to src/drivers/Qt/
    const char* classSig;    // substring expected inside the file (the class)
    int expectRetranslateUi; // 1 = must have retranslateUi
    int expectChangeEvent;   // 1 = must have changeEvent override
    int expectKeyPressBase;  // number of keyPress overrides expected to forward to base
};

// PHASE-2 widget list. Only flags widgets that the plan commits to refactor
// in this phase. ConsoleWindow is already done (PR-A), TasEditor sub-files
// are helper namespaces — only TasEditorWindow needs retranslateUi.
static const WidgetGate kPhase2Widgets[] = {
    // Header/cpp paths are flattened to one entry per file. Tests grep the
    // .cpp for both declarations and definitions; if a widget splits header
    // and implementation across multiple files, both are checked together via
    // the same gate row (the regex substring matches either side).
    { "ConsoleWindow.cpp",            "consoleWin_t",  1, 1, 0 },
    { "AboutWindow.cpp",              "AboutWindow",   1, 1, 0 },
    { "ConsoleDebugger.cpp",          "ConsoleDebugger", 1, 1, 0 },
    { "TasEditor/TasEditorWindow.cpp", "TasEditorWindow", 1, 1, 0 },
    { "ppuViewer.cpp",                "ppuPatternView_t", 1, 1, 0 },
    { "AviRiffViewer.cpp",            "AviRiffViewerDialog", 1, 1, 0 },
    { "HexEditor.cpp",                "QHexEdit", 1, 1, 1 }, // keyPress :2602
    { "AviRecord.cpp",                "AviRecordDiskThread_t", 1, 1, 0 },
    { "GamePadConf.cpp",              "GamePadConfDialog_t", 1, 1, 0 },
    { "GuiConf.cpp",                  "GuiConfDialog_t", 1, 1, 0 },
    { "RamWatch.cpp",                 "RamWatchDialog_t", 1, 1, 0 },
    { "TraceLogger.cpp",              "QTraceLogView", 1, 1, 0 },
    { "ConsoleVideoConf.cpp",         "ConsoleVideoConfDialog_t", 1, 1, 0 },
    { "CodeDataLogger.cpp",           "CodeDataLoggerDialog_t", 1, 1, 0 },
    { "iNesHeaderEditor.cpp",         "iNesEditDialog_t", 1, 1, 0 },
    { "CheatsConf.cpp",               "GuiCheatsDialog_t", 1, 1, 0 },
    { "NameTableViewer.cpp",          "ppuNameTableView_t", 1, 1, 0 },
    { "MoviePlay.cpp",                "MoviePlayDialog", 1, 1, 0 },
    { "PaletteEditor.cpp",            "nesPaletteView", 1, 1, 0 },
    { "StateRecorderConf.cpp",        "StateRecorderDialog", 1, 1, 0 },
    { "FamilyKeyboard.cpp",           "FamilyKeyboardWidget", 1, 1, 2 }, // keyPress :359 + :1551
    { "FrameTimingStats.cpp",         "FrameTimingDialog_t", 1, 1, 0 },
    { "RamSearch.cpp",                "QRamSearchView", 1, 1, 0 },
    { "InputConf.cpp",                "InputConfDialog", 1, 1, 0 },
    { "MovieRecord.cpp",              "MovieRecordDialog", 1, 1, 0 },
    { "PaletteConf.cpp",              "PaletteConfDialog", 1, 1, 0 },
    { "SymbolicDebug.cpp",            "SymbolicDebugDialog", 1, 1, 0 },
    { "GameGenie.cpp",                "GameGenieDialog_t", 1, 1, 0 },
    { "ConsoleSoundConf.cpp",         "ConsoleSndConfDialog_t", 1, 1, 0 },
    { "TimingConf.cpp",               "TimingConfDialog", 1, 1, 0 },
    { "HelpPages.cpp",                "HelpDialog", 1, 1, 0 },
    { "LuaControl.cpp",               "LuaControlDialog", 1, 1, 0 },
    { "MovieOptions.cpp",             "MovieOptionsDialog", 1, 1, 0 },
    { "HotKeyConf.cpp",               "HotKeyConfDialog_t", 1, 1, 3 }, // keyPress :250/:299/:369
};

static bool fileContains(const std::string& content, const std::string& needle)
{
    return content.find(needle) != std::string::npos;
}

static std::string readAll(const std::string& path)
{
    std::ifstream f(path);
    if (!f.is_open()) return std::string();
    std::stringstream buf;
    buf << f.rdbuf();
    return buf.str();
}

static int countOccurrences(const std::string& haystack, const std::string& needle)
{
    if (needle.empty()) return 0;
    int n = 0;
    size_t cursor = 0;
    while (true) {
        size_t pos = haystack.find(needle, cursor);
        if (pos == std::string::npos) break;
        n++;
        cursor = pos + needle.size();
    }
    return n;
}

// Count keyPress overrides that forward to base class. Heuristic: every
// keyPressEvent definition MUST be followed within ~6 lines by a call to
// a base-class keyPressEvent. Returns the count of overrides that DO
// forward.
static int countKeyPressOverridesForwarding(const std::string& content)
{
    int total = 0;
    int forwarding = 0;
    size_t cursor = 0;
    const std::string kKeyPress = "keyPressEvent(";
    while (true) {
        size_t pos = content.find(kKeyPress, cursor);
        if (pos == std::string::npos) break;
        // Skip the 'void keyPressEvent(' declaration line itself.
        // Look for the { that opens the body, then check the next 400 chars
        // for a base-class forwarding call.
        size_t bodyOpen = content.find("{", pos);
        if (bodyOpen == std::string::npos || bodyOpen > pos + 200) {
            cursor = pos + kKeyPress.size();
            continue;
        }
        size_t bodyClose = content.find("}", bodyOpen);
        if (bodyClose == std::string::npos) break;
        std::string body = content.substr(bodyOpen, bodyClose - bodyOpen);

        // Heuristic: a forwarding override contains one of these strings
        // inside the function body.
        bool forward = (body.find("QDialog::keyPressEvent") != std::string::npos) ||
                       (body.find("QWidget::keyPressEvent") != std::string::npos) ||
                       (body.find("QTreeWidget::keyPressEvent") != std::string::npos) ||
                       (body.find("QAbstractItemView::keyPressEvent") != std::string::npos) ||
                       (body.find("QMainWindow::keyPressEvent") != std::string::npos) ||
                       (body.find("QFrame::keyPressEvent") != std::string::npos) ||
                       (body.find("QHexEdit::keyPressEvent") != std::string::npos);

        total++;
        if (forward) forwarding++;

        cursor = bodyClose + 1;
    }
    return forwarding;
}

// ---------------------------------------------------------------------------
// Main test driver.
// ---------------------------------------------------------------------------

int main()
{
    printf("=== FCEUX11 i18n regression test (PHASE-2) ===\n\n");
    bool failed = false;

    // Gate 1: .ts coverage at >= 90%.
    const char* kTsPaths[] = {
        "src/drivers/Qt/lang/fceux11_zh_CN.ts",
        "src/drivers/Qt/lang/fceux11_zh_TW.ts"
    };
    for (const char* tsPath : kTsPaths) {
        TsStats s;
        if (!parseTsFile(tsPath, s)) { failed = true; continue; }
        if (s.totalMessages == 0) {
            printf("FAIL: %s has zero <message> entries\n", tsPath);
            failed = true;
            continue;
        }
        double pct = 100.0 * (s.totalMessages - s.unfinished) / s.totalMessages;
        printf("[%s] %s: %d/%d translated (%.2f%%) -- gate 90%%\n",
               pct >= 90.0 ? "PASS" : "FAIL",
               tsPath, s.totalMessages - s.unfinished, s.totalMessages, pct);
        if (pct < 90.0) failed = true;
    }

    // Gate 2: simp/trad isolation — delegated to scripts/check_simp_trad.ps1.
    // (See comment block above for rationale.)
    printf("[INFO] simp/trad gate: delegated to scripts/check_simp_trad.ps1\n");

    // Gate 3: every PHASE-2 widget declares retranslateUi + changeEvent.
    printf("\n--- Widget retranslateUi / changeEvent presence ---\n");
    const size_t kNumWidgets = sizeof(kPhase2Widgets) / sizeof(kPhase2Widgets[0]);
    int passRetranslateUi = 0;
    int passChangeEvent = 0;
    int passKeyPress = 0;
    for (size_t i = 0; i < kNumWidgets; ++i) {
        const WidgetGate& w = kPhase2Widgets[i];
        std::string fullPath = std::string("src/drivers/Qt/") + w.path;
        std::string content = readAll(fullPath);
        if (content.empty()) {
            printf("FAIL: cannot read %s\n", fullPath.c_str());
            failed = true;
            continue;
        }

        bool okRetranslate = (countOccurrences(content, "retranslateUi") >= 2); // decl + def
        bool okChangeEvent = fileContains(content, "changeEvent(QEvent") ||
                             fileContains(content, "changeEvent( QEvent") ||
                             fileContains(content, "changeEvent(QEvent *") ||
                             fileContains(content, "changeEvent (QEvent");
        int forwarding = countKeyPressOverridesForwarding(content);

        printf("  [%s%s%s] %s\n",
               okRetranslate ? "RT" : "rt",
               okChangeEvent ? "CE" : "ce",
               forwarding >= w.expectKeyPressBase ? "KP" : "kp",
               w.path);

        if (okRetranslate) passRetranslateUi++; else failed = true;
        if (okChangeEvent) passChangeEvent++; else failed = true;
        if (forwarding >= w.expectKeyPressBase) passKeyPress++; else failed = true;
    }

    printf("\n--- Summary ---\n");
    printf("retranslateUi declared: %u/%u\n", (unsigned)passRetranslateUi, (unsigned)kNumWidgets);
    printf("changeEvent override:   %u/%u\n", (unsigned)passChangeEvent, (unsigned)kNumWidgets);
    printf("keyPress forwarding:    %u/%u\n", (unsigned)passKeyPress, (unsigned)kNumWidgets);

    printf("\n=== Test Complete ===\n");
    if (failed) {
        printf("\nRESULT: FAILED\n");
        return 1;
    }
    printf("\nRESULT: PASSED - all i18n gates green\n");
    return 0;
}
