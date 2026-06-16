# Menu Migration: v0.3.14 → v0.3.15 (5+1 Audience-Tiered Model)

> **Status**: PR-A delivered (commit `[PHASE-A]`)
> **Breaking change**: yes — menu items have been moved. No functional change. All `HK_*` hotkeys unchanged. All `consoleWin_t` public `QAction*` fields preserved.

## What changed

v0.3.14 had **7 flat top-level menus**: File / Movie / Options / Emulation / Tools / Debug / Help.

v0.3.15 ships the **5+1 audience-tiered model**: 4 basic menus for new players + 1 "Advanced" top-level menu that collects the formerly-flat Tools / Debug / Movie / advanced-Options into 5 sub-menus.

```
v0.3.14 (7 menus)              v0.3.15 (5+1)
─────────────                  ──────────────
File                  ───►     File
Options               ───►     Options       (sound / video / GUI / language /
Emulation             ───►     Emulation     (window / fullscreen / hide / BG)
                                  ▼
Tools                 ───►  ┌──────────────────────┐
Debug                 ───►  │   Advanced (&A)      │
Movie                 ───►  │  ├ Emulation         │
Options (advanced)    ───►  │  ├ Movie             │
                             │  ├ Debug             │
                             │  ├ Memory Tools      │
                             │  ├ Misc Tools        │
                             │  └ Advanced Settings │
Help                  ───►  └──────────────────────┘
                             Help
```

## Item-by-item migration table

### File menu (unchanged location)
| v0.3.14 location | v0.3.15 location | Hotkey | Note |
|---|---|---|---|
| File → Open ROM | File → Open ROM | Ctrl+O (default) | unchanged |
| File → Close ROM | File → Close ROM | — | unchanged |
| File → Recent ROMs | File → Recent ROMs | — | unchanged |
| File → Play NSF | File → Play NSF | — | unchanged |
| File → Quick Load | File → Quick Load | F7 | unchanged |
| File → Quick Save | File → Quick Save | F5 | unchanged |
| File → Change State Slot | File → Change State Slot | Shift+0..9 | unchanged |
| File → Load Lua Script | File → Load Lua Script | — | unchanged |
| File → Screenshot | File → Screenshot | F12 | unchanged |
| File → Quit | File → Quit | Alt+F4 | unchanged |

### Emulation menu (slimmed down — basic workflow only)
| v0.3.14 location | v0.3.15 location | Hotkey | Note |
|---|---|---|---|
| Emulation → Power | Emulation → Power | — | unchanged |
| Emulation → Hard Reset | Emulation → Hard Reset | — | unchanged |
| Emulation → Pause | Emulation → Pause | Pause | unchanged |
| Emulation → Region | Emulation → Region | — | unchanged |
| Emulation → Speed | Emulation → Speed | +/- | unchanged |
| Emulation → AutoFire Pattern | Emulation → AutoFire Pattern | — | unchanged |
| ~~Emulation → Soft Reset~~ | **Advanced → Emulation → Soft Reset** | — | **moved** |
| ~~Emulation → Enable Game Genie~~ | **Advanced → Emulation → Enable Game Genie** | — | **moved** |
| ~~Emulation → Load Game Genie ROM~~ | **Advanced → Emulation → Load Game Genie ROM** | — | **moved** |
| ~~Emulation → Virtual Family Keyboard~~ | **Advanced → Emulation → Virtual Family Keyboard** | — | **moved** |
| ~~Emulation → Insert Coin (VS)~~ | **Advanced → Emulation → Insert Coin** | — | **moved** |
| ~~Emulation → FDS sub-menu~~ | **Advanced → Emulation → FDS sub-menu** | — | **moved** |
| ~~Emulation → RAM Init~~ | **Advanced → Emulation → RAM Init** | — | **moved** |

### Options menu (slimmed down — daily-use only)
| v0.3.14 location | v0.3.15 location | Note |
|---|---|---|
| Options → Sound Config | Options → Sound Config | unchanged |
| Options → Video Config | Options → Video Config | unchanged |
| Options → GUI Config | Options → GUI Config | unchanged |
| Options → Language | Options → Language | unchanged |
| Options → Window Resize | Options → Window Resize | unchanged |
| Options → Fullscreen | Options → Fullscreen | unchanged |
| Options → Hide Menu | Options → Hide Menu | unchanged |
| Options → Auto Hide Menu on Fullscreen | Options → Auto Hide Menu on Fullscreen | unchanged |
| Options → BG Side Panel Color | Options → BG Side Panel Color | unchanged |
| ~~Options → Input Config~~ | **Advanced → Advanced Settings → Input Config** | **moved** |
| ~~Options → GamePad Config~~ | **Advanced → Advanced Settings → GamePad Config** | **moved** |
| ~~Options → HotKey Config~~ | **Advanced → Advanced Settings → HotKey Config** | **moved** |
| ~~Options → Palette Config~~ | **Advanced → Advanced Settings → Palette Config** | **moved** |
| ~~Options → Timing Config~~ | **Advanced → Advanced Settings → Timing Config** | **moved** |
| ~~Options → State Recorder Config~~ | **Advanced → Advanced Settings → State Recorder Config** | **moved** |
| ~~Options → Movie Options~~ | **Advanced → Advanced Settings → Movie Options** | **moved** |
| ~~Options → Auto-Resume Play~~ | **Advanced → Advanced Settings → Auto-Resume Play** | **moved** |

### Advanced menu (NEW top-level — 5 sub-menus)

#### Advanced → Emulation
- Soft Reset
- Enable Game Genie
- Load Game Genie ROM
- Virtual Family Keyboard
- Insert Coin (VS System)
- FDS sub-menu:
  - Switch Disk
  - Eject Disk
  - Load BIOS
- RAM Init sub-menu:
  - Default
  - Fill $FF
  - Fill $00
  - Random

#### Advanced → Movie (was: top-level Movie)
- Movie Play
- Movie Play From Beginning
- Movie Stop
- ───
- Movie Record
- ───
- AVI Record
- AVI Record As
- AVI Stop
- ───
- WAV Record
- WAV Record As
- WAV Stop

#### Advanced → Debug (was: top-level Debug)
- Debugger
- Hex Editor
- PPU Viewer
- Sprite Viewer
- Name Table Viewer
- Trace Logger
- Code/Data Logger
- Game Genie Encode/Decode
- NES Header Editor

#### Advanced → Memory Tools (was: Tools partial)
- Cheats
- RAM Search
- RAM Watch

#### Advanced → Misc Tools (was: Tools partial)
- Frame Timing
- Palette Editor
- AVI RIFF Viewer
- TAS Editor

#### Advanced → Advanced Settings (was: Options partial)
- Input Config
- GamePad Config
- HotKey Config
- Palette Config
- Timing Config
- State Recorder Config
- Movie Options
- ───
- Auto-Resume Play

### Help menu (unchanged)
- About FCEUX11
- About Qt
- Message Log

## New `SDL.HideAdvancedMenu` config option

Available in **Options → GUI Config → "Hide Advanced Menu"** (default OFF).

- **OFF (default)**: menu bar shows 5 top-level menus (File / Emulation / Options / Advanced / Help)
- **ON**: menu bar shows only 4 top-level menus (File / Emulation / Options / Help) — the Advanced menu and all its 5 sub-menus are hidden

This is intended for users who only want to play games and never touch the debugger / TAS / movie recording features. The change takes effect on next application start.

## Rationale

1. **Basic player first**: A new user opening FCEUX11 for the first time now sees 4 menus, not 7. They can open a ROM and play immediately without being overwhelmed by Debug / Tools / Movie / Game Genie / TAS.
2. **Power users retain access**: All former tools are still reachable, just one extra click away (Advanced menu). All hotkeys work identically.
3. **Hamburger-menu escape hatch**: The `HideAdvancedMenu` checkbox lets a power user opt to hide the advanced menus entirely (e.g. for a kid's account on a shared PC), without losing the underlying functionality.

## Compatibility

- **All `HK_*` hotkeys** — unchanged. F5 / F7 / F2 / Esc / Shift+F5 / etc. all continue to work identically.
- **All `consoleWin_t` public `QAction*` fields** — preserved by name. External callers in `src/drivers/Qt/` reference the fields only by slot invocation (`consoleWindow->openDebugWindow()`), so the menu regrouping is fully internal.
- **Config keys** — only `SDL.HideAdvancedMenu` is new. All other config keys are unchanged.
- **v0.3.15 tr() freeze**: After PR-A merge, the set of `tr()` source strings in `ConsoleWindow.cpp` is frozen. Any new menu item requires a v0.3.15.x hotfix + lupdate re-scan.

## See also

- `docs/v0.3.x_Construction_Plan_v3.md` §5 v0.3.15 PR-A specification
- `src/drivers/Qt/MenuCatalog.h` — declarative 5+1 model specification
