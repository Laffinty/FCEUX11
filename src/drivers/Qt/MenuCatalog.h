// MenuCatalog.h - v0.3.15 PR-A
// Declarative specification of the 5+1 audience-tiered menu structure.
// This file is the authoritative source for menu item paths; the actual
// QAction* construction remains in consoleWin_t::createMainMenu() for
// compatibility with v0.3.14 hotkey/shortcut wiring, but the catalog
// documents the target layout for migration and translation.
//
// Iron rule: tr() source strings in this file are FROZEN at PR-A merge.
// Adding a new menu item requires a hotfix + lupdate re-scan.
//
// Iron rule: 63 consoleWin_t QAction* fields are preserved unchanged.
// The catalog only describes WHERE each action is routed.

#ifndef __FCEUX_MENU_CATALOG_H__
#define __FCEUX_MENU_CATALOG_H__

#include <QString>
#include <QStringList>

namespace fceu11
{

/// 5+1 menu model (plan §5 v0.3.15 PR-A):
///   File / Emulation / Options / Help        (basic player workflow)
///   Advanced (5 sub-menus)                   (TAS / Debug / etc.)
///
/// Hidden by GUI.HideAdvancedMenu = ON, leaving 4 top-level menus.
enum class MenuRoot : int {
    File      = 0,  // basic
    Emulation = 1,  // basic
    Options   = 2,  // basic
    Advanced  = 3,  // new top-level collector (advanced)
    Help      = 4,  // basic
    COUNT     = 5,
};

enum class AdvancedSub : int {
    Emulation  = 0,  // soft reset / GG / FKB / FDS / VS / RAM init / AutoFire
    Movie      = 1,  // full movie + AVI/WAV record sub-tree
    Debug      = 2,  // debugger / hex / ppu / sprite / nt / trace / cdl / gg / iNes
    Memory     = 3,  // cheats / RAM search / RAM watch
    Misc       = 4,  // frame timing / palette editor / avi riff / tas editor
    Settings   = 5,  // input / gamepad / hotkey / palette / timing / stateRec / movieOpt / autoResume
    COUNT      = 6,
};

/// Returns the localized title for a top-level menu (used by retranslateUi
/// and by MenuCatalog as a single source of truth for tr() strings).
inline QString menuRootTitle(MenuRoot r)
{
    switch (r) {
        case MenuRoot::File:      return QObject::tr("&File");
        case MenuRoot::Emulation: return QObject::tr("&Emulation");
        case MenuRoot::Options:   return QObject::tr("&Options");
        case MenuRoot::Advanced:  return QObject::tr("&Advanced");
        case MenuRoot::Help:      return QObject::tr("&Help");
        default:                  return QString();
    }
}

inline QString advancedSubTitle(AdvancedSub s)
{
    switch (s) {
        case AdvancedSub::Emulation: return QObject::tr("&Emulation");
        case AdvancedSub::Movie:     return QObject::tr("&Movie");
        case AdvancedSub::Debug:     return QObject::tr("&Debug");
        case AdvancedSub::Memory:    return QObject::tr("&Memory Tools");
        case AdvancedSub::Misc:      return QObject::tr("&Misc Tools");
        case AdvancedSub::Settings:  return QObject::tr("&Advanced Settings");
        default:                     return QString();
    }
}

/// Stable identifier for each menu path. Used to detect when a future PR
/// changes a menu path (CI guard).
struct MenuPathId {
    MenuRoot    root;
    AdvancedSub sub;  // only meaningful when root == Advanced
    bool        isAdvanced() const { return root == MenuRoot::Advanced; }
    QString     toString() const
    {
        if (!isAdvanced()) return menuRootTitle(root);
        return menuRootTitle(MenuRoot::Advanced) + QStringLiteral("/")
             + advancedSubTitle(sub);
    }
};

} // namespace fceu11

#endif // __FCEUX_MENU_CATALOG_H__
