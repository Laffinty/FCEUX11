# FCEUX11 Source Code Copyright Audit

> **Audit Date**: 2026-08-09
> **Scope**: src/ directory (all .cpp, .h, .c, .hpp, .rs files; rust/target/ excluded)
> **Excluded from scope**: tests/fixtures/ (third-party test ROMs 鈥?see DERIVATIVE_WORK_NOTICE.txt)
> **Total Files Listed**: 1095  (OK=467, New=279, Renamed=57, Stale=349)

## 1. License Distribution

| License | File Count |
|---------|-----------:|
| GPLv2+ | 460 |
| GPLv2 | 0 |
| LGPL | 2 |
| Apache-2.0 | 0 |
| MIT | 19 |
| BSD | 4 |
| zlib | 0 |
| Unknown/None | 610 |

## 2. Files by Bucket

| Bucket | File Count |
|--------|-----------:|
| archived | 13 |
| attic | 50 |
| boards | 198 |
| drivers | 472 |
| fir | 7 |
| input | 24 |
| lua | 55 |
| palettes | 6 |
| platform | 4 |
| root | 127 |
| rust | 96 |
| tests | 5 |
| utils | 38 |

## 3. Unique Copyright Holders (live files)

- * Copyright (C) 1998 Bero
- * Copyright (C) 1998 BERO
- * Copyright (C) 2001 Aaron Oneal
- * Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni
- * Copyright (C) 2002 CaH4e3
- * Copyright (C) 2002 Paul Kuliniewicz
- * Copyright (C) 2002 Xodnizel
- * Copyright (C) 2002 Xodnizel 2006 CaH4e3
- * Copyright (C) 2002,2003 Xodnizel
- * Copyright (C) 2003 Andrea Mazzoleni
- * Copyright (C) 2003 Xodnizel
- * Copyright (C) 2004 Jason Oster (Parasyte)
- * Copyright (C) 2004 Xodnizel
- * Copyright (C) 2005 CaH4e3
- * Copyright (C) 2005 Sebastian Porst
- * Copyright (C) 2005-2019 CaH4e3
- * Copyright (C) 2006 CaH4e3
- * Copyright (C) 2007 CaH4e3
- * Copyright (C) 2007-2008 Mad Dumper, CaH4e3
- * Copyright (C) 2007-2010 CaH4e3
- * Copyright (C) 2008 CaH4e3
- * Copyright (c) 2008-2011, Michael Kohn
- * Copyright (C) 2009 CaH4e3
- * Copyright (C) 2009 qeed
- * Copyright (C) 2011 CaH4e3
- * Copyright (C) 2011 FCEUX team
- * Copyright (C) 2012 CaH4e3
- * Copyright (C) 2012 FCEUX team
- * Copyright (C) 2013 CaH4e3
- * Copyright (c) 2013, Robin Hahling
- * Copyright (C) 2014 CaH4e3
- * Copyright (C) 2014 CaitSith2, 2022 Cluster
- * Copyright (C) 2015 CaH4e3
- * Copyright (C) 2015 Cluster
- * Copyright (C) 2016 CaH4e3
- * Copyright (C) 2016 Cluster
- * Copyright (C) 2017 CaH4e3
- * Copyright (C) 2017 FCEUX Team
- * Copyright (C) 2018 CaH4e3, Cluster
- * Copyright (C) 2019 CaH4e3
- * Copyright (C) 2019 Libretro Team
- * Copyright (C) 2020
- * Copyright (C) 2020 CaH4e3
- * Copyright (C) 2020 mjbudd77
- * Copyright (C) 2021 mjbudd77
- * Copyright (C) 2022
- * Copyright (C) 2022 Cluster
- * Copyright (C) 2022 thor2016
- * Copyright 2001-2004 Unicode, Inc.
- * Copyright 2013 Google Inc. All Rights Reserved.
- * Mapper 12 code Copyright (C) 2003 CaH4e3
- Copyright (c) 1990-2000 Info-ZIP. All rights reserved.
- Copyright (C) 1998 - 2010 Gilles Vollant, Even Rouault, Mathias Svensson
- Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http:
- Copyright (C) 2003 MaxSt ( maxst@hiend3d.com )
- Copyright (C) 2007-2008 Even Rouault
- Copyright (C) 2009-2010 DeSmuME team
- Copyright (C) 2009-2010 Mathias Svensson ( http:
- Copyright (c) 2011-2013 AnS
- Copyright (C) 2012-2017 FCEUX team
- Copyright (C) 2026 FCEUX11 Contributors

## 4. Files Without Explicit License Header (Require Review)

- `src/archived/fir/c44100ntsc.h`
- `src/archived/fir/c44100pal.h`
- `src/archived/fir/c48000ntsc.h`
- `src/archived/fir/c48000pal.h`
- `src/archived/fir/c96000ntsc.h`
- `src/archived/fir/c96000pal.h`
- `src/archived/fir/toh.c`
- `src/boards/emu2413.c`
- `src/boards/emu2413.h`
- `src/boards/mapinc_audio.h`
- `src/boards/mapinc_base.h`
- `src/boards/mapinc_bus.h`
- `src/boards/mapinc_mmc3.h`
- `src/boards/mapinc_state.h`
- `src/boards/registry.cpp`
- `src/boards/registry.h`
- `src/boards/simple_carts.h`
- `src/drivers/common/args.h`
- `src/drivers/common/config.h`
- `src/drivers/common/configSys.cpp`
- `src/drivers/common/configSys.h`
- `src/drivers/common/hq2x.cpp`
- `src/drivers/common/hq2x.h`
- `src/drivers/common/hq3x.cpp`
- `src/drivers/common/hq3x.h`
- `src/drivers/common/nes_ntsc.h`
- `src/drivers/common/nes_ntsc_config.h`
- `src/drivers/common/nes_shm.h`
- `src/drivers/common/os_utils.h`
- `src/drivers/null/null_driver.h`
- `src/drivers/Qt/AboutWindow.h`
- `src/drivers/Qt/AviAudioCodec.cpp`
- `src/drivers/Qt/AviAudioCodec.h`
- `src/drivers/Qt/AviOptionsDialog.h`
- `src/drivers/Qt/AviRecord.h`
- `src/drivers/Qt/AviRecordContext.h`
- `src/drivers/Qt/AviRecordDiskThread.cpp`
- `src/drivers/Qt/AviRiffViewer.h`
- `src/drivers/Qt/AviVideoCodec.cpp`
- `src/drivers/Qt/AviVideoCodec.h`
- `src/drivers/Qt/CheatsConf.h`
- `src/drivers/Qt/CodeDataLogger.h`
- `src/drivers/Qt/ColorMenu.h`
- `src/drivers/Qt/config.h`
- `src/drivers/Qt/ConfigStore.h`
- `src/drivers/Qt/ConsoleActions.h`
- `src/drivers/Qt/ConsoleCursor.cpp`
- `src/drivers/Qt/ConsoleDebugger.h`
- `src/drivers/Qt/ConsoleDebugWindows.h`
- `src/drivers/Qt/ConsoleEmuControl.h`
- `src/drivers/Qt/ConsoleEmulatorThread.cpp`
- `src/drivers/Qt/ConsoleFile.h`
- `src/drivers/Qt/ConsoleHotKeys.cpp`
- `src/drivers/Qt/ConsoleMenu.h`
- `src/drivers/Qt/ConsoleMenuBar.cpp`
- `src/drivers/Qt/ConsoleRecentRom.h`
- `src/drivers/Qt/ConsoleRecording.h`
- `src/drivers/Qt/ConsoleSoundConf.h`
- `src/drivers/Qt/ConsoleTranslation.h`
- `src/drivers/Qt/ConsoleUtilities.h`
- `src/drivers/Qt/ConsoleVideo.h`
- `src/drivers/Qt/ConsoleVideoConf.h`
- `src/drivers/Qt/ConsoleVideoSetup.cpp`
- `src/drivers/Qt/ConsoleViewerGL.h`
- `src/drivers/Qt/ConsoleViewerInterface.cpp`
- `src/drivers/Qt/ConsoleViewerInterface.h`
- `src/drivers/Qt/ConsoleViewerQWidget.h`
- `src/drivers/Qt/ConsoleViewerSDL.h`
- `src/drivers/Qt/ConsoleWindow.h`
- `src/drivers/Qt/ConsoleWindowContext.h`
- `src/drivers/Qt/dface.h`
- `src/drivers/Qt/FamilyKeyboard.h`
- `src/drivers/Qt/fceuWrapper.h`
- `src/drivers/Qt/fceux_git_info.h`
- `src/drivers/Qt/FrameTimingStats.h`
- `src/drivers/Qt/GameGenie.h`
- `src/drivers/Qt/GamePadConf.h`
- `src/drivers/Qt/GuiConf.h`
- `src/drivers/Qt/HelpPages.h`
- `src/drivers/Qt/HexEditor.h`
- `src/drivers/Qt/HotKeyConf.h`
- `src/drivers/Qt/iNesHeaderEditor.h`
- `src/drivers/Qt/input.h`
- `src/drivers/Qt/InputConf.h`
- `src/drivers/Qt/keyscan.h`
- `src/drivers/Qt/LuaControl.h`
- `src/drivers/Qt/MenuCatalog.h`
- `src/drivers/Qt/MovieOptions.h`
- `src/drivers/Qt/MoviePlay.h`
- `src/drivers/Qt/MovieRecord.h`
- `src/drivers/Qt/MsgLogViewer.h`
- `src/drivers/Qt/NameTableViewer.h`
- `src/drivers/Qt/PaletteConf.h`
- `src/drivers/Qt/PaletteEditor.h`
- `src/drivers/Qt/ppuViewer.h`
- `src/drivers/Qt/ppuViewerContext.cpp`
- `src/drivers/Qt/ppuViewerContext.h`
- `src/drivers/Qt/ppuViewerPalette.cpp`
- `src/drivers/Qt/ppuViewerPalette.h`
- `src/drivers/Qt/ppuViewerPatternTables.cpp`
- `src/drivers/Qt/ppuViewerPatternTables.h`
- `src/drivers/Qt/ppuViewerSpriteViewer.cpp`
- `src/drivers/Qt/ppuViewerSpriteViewer.h`
- `src/drivers/Qt/ppuViewerTileEditor.cpp`
- `src/drivers/Qt/ppuViewerTileEditor.h`
- `src/drivers/Qt/RamSearch.h`
- `src/drivers/Qt/RamWatch.h`
- `src/drivers/Qt/sdl.h`
- `src/drivers/Qt/sdl-joystick.h`
- `src/drivers/Qt/sdl-video.h`
- `src/drivers/Qt/SplashScreen.h`
- `src/drivers/Qt/StateRecorderConf.h`
- `src/drivers/Qt/SymbolicDebug.h`
- `src/drivers/Qt/TasEditor/bookmark.h`
- `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.cpp`
- `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.h`
- `src/drivers/Qt/TasEditor/bookmarks.h`
- `src/drivers/Qt/TasEditor/branches.h`
- `src/drivers/Qt/TasEditor/greenzone.h`
- `src/drivers/Qt/TasEditor/history.h`
- `src/drivers/Qt/TasEditor/inputlog.h`
- `src/drivers/Qt/TasEditor/laglog.h`
- `src/drivers/Qt/TasEditor/markerDragPopup.cpp`
- `src/drivers/Qt/TasEditor/markerDragPopup.h`
- `src/drivers/Qt/TasEditor/markers.h`
- `src/drivers/Qt/TasEditor/markers_manager.h`
- `src/drivers/Qt/TasEditor/playback.h`
- `src/drivers/Qt/TasEditor/recorder.h`
- `src/drivers/Qt/TasEditor/selection.h`
- `src/drivers/Qt/TasEditor/snapshot.h`
- `src/drivers/Qt/TasEditor/splicer.h`
- `src/drivers/Qt/TasEditor/TasColors.h`
- `src/drivers/Qt/TasEditor/taseditor_config.h`
- `src/drivers/Qt/TasEditor/taseditor_lua.h`
- `src/drivers/Qt/TasEditor/taseditor_project.h`
- `src/drivers/Qt/TasEditor/TasEditorContext.h`
- `src/drivers/Qt/TasEditor/TasEditorTimeline.h`
- `src/drivers/Qt/TasEditor/TasEditorWindow.h`
- `src/drivers/Qt/TasEditor/TasFindNoteWindow.cpp`
- `src/drivers/Qt/TasEditor/TasFindNoteWindow.h`
- `src/drivers/Qt/throttle.h`
- `src/drivers/Qt/TimingConf.h`
- `src/drivers/Qt/TraceLogger.h`
- `src/input/cursor.cpp`
- `src/input/fkb.h`
- `src/input/share.h`
- `src/input/suborkb.h`
- `src/input/zapper.h`
- `src/palettes/conv.c`
- `src/palettes/palettes.h`
- `src/palettes/rp2c04001.h`
- `src/palettes/rp2c04002.h`
- `src/palettes/rp2c04003.h`
- `src/palettes/rp2c05004.h`
- `src/platform/win11/DirectStorageProbe.cpp`
- `src/platform/win11/DirectStorageProbe.h`
- `src/platform/win11/TaskbarProgress.cpp`
- `src/asm.h`
- `src/cart.h`
- `src/compiler_attrs.h`
- `src/config.cpp`
- `src/debug.h`
- `src/drawing.h`
- `src/emufile_types.h`
- `src/fceu.h`
- `src/fceulua.h`
- `src/fds.h`
- `src/fds_sound.cpp`
- `src/file.h`
- `src/filter.h`
- `src/git.h`
- `src/ines.cpp`
- `src/ines_bmap.h`
- `src/ines_gi.cpp`
- `src/ines_save.cpp`
- `src/ines-bad.h`
- `src/ines-correct.h`
- `src/input.h`
- `src/movie_fm2.h`
- `src/movie_playback.h`
- `src/movie_record.h`
- `src/movie_settings.cpp`
- `src/movie_subtitles.cpp`
- `src/movie_taseditor_bridge.cpp`
- `src/netplay.h`
- `src/oldmovie.cpp`
- `src/oldmovie.h`
- `src/palette.h`
- `src/ppu_core.h`
- `src/ppu_rendering.h`
- `src/ppu_sprite_lut.cpp`
- `src/ppu_sprite_lut.h`
- `src/ppu_state.h`
- `src/pputile_template.cpp`
- `src/pputile_template.h`
- `src/unif.cpp`
- `src/unif_bmap.h`
- `src/video.h`
- `src/vsuni.h`
- `src/wave.h`
- `src/rust/crates/fceux11-core/src/bus.rs`
- `src/rust/crates/fceux11-core/src/lib.rs`
- `src/rust/crates/fceux11-core/src/state_file.rs`
- `src/rust/crates/fceux11-core/src/state_recorder.rs`
- `src/rust/crates/fceux11-debug/src/cheat.rs`
- `src/rust/crates/fceux11-debug/src/conddebug.rs`
- `src/rust/crates/fceux11-debug/src/debug.rs`
- `src/rust/crates/fceux11-debug/src/debugsym.rs`
- `src/rust/crates/fceux11-debug/src/ld65dbg.rs`
- `src/rust/crates/fceux11-debug/src/lib.rs`
- `src/rust/crates/fceux11-formats/src/emufile.rs`
- `src/rust/crates/fceux11-formats/src/ines/ines_data.rs`
- `src/rust/crates/fceux11-formats/src/lib.rs`
- `src/rust/crates/fceux11-formats/src/unif.rs`
- `src/rust/crates/fceux11-formats/src/vsuni.rs`
- `src/rust/crates/fceux11-lua/src/bindings/bit.rs`
- `src/rust/crates/fceux11-lua/src/bindings/gui.rs`
- `src/rust/crates/fceux11-lua/src/bindings/input.rs`
- `src/rust/crates/fceux11-lua/src/bindings/mod.rs`
- `src/rust/crates/fceux11-media/src/fcoeffs.rs`
- `src/rust/crates/fceux11-media/src/lib.rs`
- `src/rust/crates/fceux11-utils/src/convert_utf.rs`
- `src/rust/crates/fceux11-utils/src/lib.rs`
- `src/rust/crates/fceux11-utils/src/md5.rs`
- `src/rust/crates/fceux11-utils/src/profiler.rs`
- `src/rust/crates/fceux11-utils/src/slice.rs`
- `src/rust/crates/kagami-qa/src/adapter/subprocess.rs`
- `src/rust/crates/kagami-qa/src/core/config.rs`
- `src/rust/crates/kagami-qa/src/core/error.rs`
- `src/rust/crates/kagami-qa/src/core/mod.rs`
- `src/rust/crates/kagami-qa/src/manifest/mod.rs`
- `src/rust/crates/kagami-qa/src/oracle/mod.rs`
- `src/rust/crates/kagami-qa/src/oracle/regression.rs`
- `src/rust/crates/kagami-qa/src/report/baseline.rs`
- `src/rust/crates/kagami-qa/src/report/mod.rs`
- `src/rust/crates/kagami-qa/src/runner/mapper_byte_diff.rs`
- `src/rust/crates/kagami-qa/src/runner/mod.rs`
- `src/rust/crates/kagami-qa/src/runner/scheduler.rs`
- `src/tests/boards/mapper_load_test.cpp`
- `src/tests/boards/mapper_reset_test.cpp`
- `src/tests/git_info_stub.cpp`
- `src/tests/rom_regression_test.cpp`
- `src/utils/backward.cpp`
- `src/utils/ConvertUTF.h`
- `src/utils/crc32.h`
- `src/utils/endian.h`
- `src/utils/expected.hpp`
- `src/utils/general.h`
- `src/utils/guid.h`
- `src/utils/ioapi.cpp`
- `src/utils/ioapi.h`
- `src/utils/mutex.cpp`
- `src/utils/mutex.h`
- `src/utils/safe_string.h`
- `src/utils/timeStamp.h`
- `src/utils/tl/expected.hpp`
- `src/utils/unzip.cpp`
- `src/utils/unzip.h`
- `src/utils/valuearray.h`

## 5. Diff vs Previous Audit

| Status | Count |
|--------|------:|
| OK | 467 |
| New | 279 |
| Stale | 9 |
| Stale (Renamed) | 57 |
| Stale (Replaced by Rust) | 55 |
| Stale (Driver removed) | 228 |

### Removed / Renamed Files

| File | Status | Notes |
|------|--------|-------|
| `src/attic/fceustr.cpp` | Stale (Renamed) | -> src/archived/ |
| `src/attic/fceustr.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dface.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-joystick.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-joystick.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-keyboard.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-mouse.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-sound.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-sound.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-video.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/dos-video.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/input.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/input.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/keyscan.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/main.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/main.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-icon.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-joystick.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-netplay.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-netplay.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-opengl.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-opengl.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-sound.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-throttle.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-video.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/sdl-video.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/throttle.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/throttle.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/unix-netplay.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/unix-netplay.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/usage.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/pc/vgatweak.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/convert.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/convert.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/convertgen.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/drivers/dsound.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/drivers/oss.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/drivers/oss.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/drivers/osxcoreaudio.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/md5.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/md5.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/sexyal.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/sexyal.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/smallc.c` | Stale (Renamed) | -> src/archived/ |
| `src/attic/sexyal/smallc.h` | Stale (Renamed) | -> src/archived/ |
| `src/attic/soundexp.cpp` | Stale (Renamed) | -> src/archived/ |
| `src/boards/mapinc.h` | Stale |  |
| `src/drivers/Qt/nes_shm.cpp` | Stale |  |
| `src/drivers/Qt/nes_shm.h` | Stale |  |
| `src/drivers/Qt/unix-netplay.cpp` | Stale |  |
| `src/drivers/Qt/unix-netplay.h` | Stale |  |
| `src/drivers/sdl/cheat.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/cheat.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/config.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/config.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/debugger.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/debugger.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/dface.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/fceux_git_info.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/GamePadConf.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/GamePadConf.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/glxwin.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/glxwin.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/gui.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/gui.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/input.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/input.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/keyscan.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/main.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/memview.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/memview.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/ramwatch.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/ramwatch.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-icon.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-joystick.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-joystick.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-netplay.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-sound.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-throttle.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-video.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/sdl-video.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/throttle.h` | Stale (Driver removed) |  |
| `src/drivers/sdl/unix-netplay.cpp` | Stale (Driver removed) |  |
| `src/drivers/sdl/unix-netplay.h` | Stale (Driver removed) |  |
| `src/drivers/videolog/nesvideos-piece.cpp` | Stale (Driver removed) |  |
| `src/drivers/videolog/nesvideos-piece.h` | Stale (Driver removed) |  |
| `src/drivers/videolog/quantize.h` | Stale (Driver removed) |  |
| `src/drivers/videolog/rgbtorgb.cpp` | Stale (Driver removed) |  |
| `src/drivers/videolog/rgbtorgb.h` | Stale (Driver removed) |  |
| `src/drivers/videolog/simd.h` | Stale (Driver removed) |  |
| `src/drivers/win/7zip/IArchive.h` | Stale (Driver removed) |  |
| `src/drivers/win/7zip/IProgress.h` | Stale (Driver removed) |  |
| `src/drivers/win/7zip/IStream.h` | Stale (Driver removed) |  |
| `src/drivers/win/7zip/MyUnknown.h` | Stale (Driver removed) |  |
| `src/drivers/win/7zip/PropID.h` | Stale (Driver removed) |  |
| `src/drivers/win/7zip/Types.h` | Stale (Driver removed) |  |
| `src/drivers/win/afxres.h` | Stale (Driver removed) |  |
| `src/drivers/win/archive.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/archive.h` | Stale (Driver removed) |  |
| `src/drivers/win/args.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/args.h` | Stale (Driver removed) |  |
| `src/drivers/win/aviout.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/cdlogger.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/cdlogger.h` | Stale (Driver removed) |  |
| `src/drivers/win/cheat.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/cheat.h` | Stale (Driver removed) |  |
| `src/drivers/win/common.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/common.h` | Stale (Driver removed) |  |
| `src/drivers/win/config.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/config.h` | Stale (Driver removed) |  |
| `src/drivers/win/debug.h` | Stale (Driver removed) |  |
| `src/drivers/win/debugger.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/debugger.h` | Stale (Driver removed) |  |
| `src/drivers/win/debuggersp.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/debuggersp.h` | Stale (Driver removed) |  |
| `src/drivers/win/directories.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/directories.h` | Stale (Driver removed) |  |
| `src/drivers/win/directx/ddraw.h` | Stale (Driver removed) |  |
| `src/drivers/win/directx/dinput.h` | Stale (Driver removed) |  |
| `src/drivers/win/directx/dsound.h` | Stale (Driver removed) |  |
| `src/drivers/win/directx/x64/ddraw.h` | Stale (Driver removed) |  |
| `src/drivers/win/directx/x64/dinput.h` | Stale (Driver removed) |  |
| `src/drivers/win/directx/x64/dsound.h` | Stale (Driver removed) |  |
| `src/drivers/win/gui.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/gui.h` | Stale (Driver removed) |  |
| `src/drivers/win/guiconfig.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/guiconfig.h` | Stale (Driver removed) |  |
| `src/drivers/win/header_editor.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/header_editor.h` | Stale (Driver removed) |  |
| `src/drivers/win/help.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/help.h` | Stale (Driver removed) |  |
| `src/drivers/win/input.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/input.h` | Stale (Driver removed) |  |
| `src/drivers/win/joystick.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/joystick.h` | Stale (Driver removed) |  |
| `src/drivers/win/keyboard.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/keyboard.h` | Stale (Driver removed) |  |
| `src/drivers/win/keyscan.h` | Stale (Driver removed) |  |
| `src/drivers/win/log.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/log.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/lauxlib.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/llimits.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/lmem.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/lobject.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/lstate.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/ltm.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/lua.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/luaconf.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/lualib.h` | Stale (Driver removed) |  |
| `src/drivers/win/lua/include/lzio.h` | Stale (Driver removed) |  |
| `src/drivers/win/luaconsole.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/main.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/main.h` | Stale (Driver removed) |  |
| `src/drivers/win/mapinput.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/mapinput.h` | Stale (Driver removed) |  |
| `src/drivers/win/memview.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/memview.h` | Stale (Driver removed) |  |
| `src/drivers/win/memviewsp.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/memviewsp.h` | Stale (Driver removed) |  |
| `src/drivers/win/memwatch.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/memwatch.h` | Stale (Driver removed) |  |
| `src/drivers/win/monitor.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/monitor.h` | Stale (Driver removed) |  |
| `src/drivers/win/movieoptions.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/movieoptions.h` | Stale (Driver removed) |  |
| `src/drivers/win/netplay.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/netplay.h` | Stale (Driver removed) |  |
| `src/drivers/win/ntview.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/ntview.h` | Stale (Driver removed) |  |
| `src/drivers/win/oakra.h` | Stale (Driver removed) |  |
| `src/drivers/win/OutputDS.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/palette.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/palette.h` | Stale (Driver removed) |  |
| `src/drivers/win/ppuview.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/ppuview.h` | Stale (Driver removed) |  |
| `src/drivers/win/pref.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/pref.h` | Stale (Driver removed) |  |
| `src/drivers/win/ram_search.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/ram_search.h` | Stale (Driver removed) |  |
| `src/drivers/win/ramwatch.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/ramwatch.h` | Stale (Driver removed) |  |
| `src/drivers/win/replay.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/replay.h` | Stale (Driver removed) |  |
| `src/drivers/win/resource.h` | Stale (Driver removed) |  |
| `src/drivers/win/sound.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/sound.h` | Stale (Driver removed) |  |
| `src/drivers/win/state.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/state.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/bookmark.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/bookmark.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/bookmarks.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/bookmarks.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/branches.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/branches.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/editor.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/editor.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/greenzone.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/greenzone.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/history.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/history.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/inputlog.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/inputlog.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/laglog.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/laglog.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/markers.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/markers.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/markers_manager.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/markers_manager.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/piano_roll.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/piano_roll.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/playback.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/playback.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/popup_display.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/popup_display.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/recorder.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/recorder.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/selection.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/selection.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/snapshot.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/snapshot.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/splicer.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/splicer.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_config.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_config.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_lua.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_lua.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_project.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_project.h` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_window.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/taseditor/taseditor_window.h` | Stale (Driver removed) |  |
| `src/drivers/win/texthook.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/texthook.h` | Stale (Driver removed) |  |
| `src/drivers/win/throttle.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/throttle.h` | Stale (Driver removed) |  |
| `src/drivers/win/timing.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/timing.h` | Stale (Driver removed) |  |
| `src/drivers/win/tracer.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/tracer.h` | Stale (Driver removed) |  |
| `src/drivers/win/video.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/video.h` | Stale (Driver removed) |  |
| `src/drivers/win/wave.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/wave.h` | Stale (Driver removed) |  |
| `src/drivers/win/Win32InputBox.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/Win32InputBox.h` | Stale (Driver removed) |  |
| `src/drivers/win/window.cpp` | Stale (Driver removed) |  |
| `src/drivers/win/window.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/adler32.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/compress.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/crc32.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/deflate.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/deflate.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/example.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/gzio.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/infblock.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/infblock.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/infcodes.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/infcodes.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/inffast.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/inffast.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/inffixed.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/inflate.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/inftrees.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/inftrees.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/infutil.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/infutil.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/maketree.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/trees.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/trees.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/uncompr.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/unzip.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/unzip.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/zconf.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/zlib.h` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/zutil.c` | Stale (Driver removed) |  |
| `src/drivers/win/zlib/zutil.h` | Stale (Driver removed) |  |
| `src/fir/c44100ntsc.h` | Stale (Renamed) | renamed -> src/archived/fir/ |
| `src/fir/c44100pal.h` | Stale (Renamed) | renamed -> src/archived/fir/ |
| `src/fir/c48000ntsc.h` | Stale (Renamed) | renamed -> src/archived/fir/ |
| `src/fir/c48000pal.h` | Stale (Renamed) | renamed -> src/archived/fir/ |
| `src/fir/c96000ntsc.h` | Stale (Renamed) | renamed -> src/archived/fir/ |
| `src/fir/c96000pal.h` | Stale (Renamed) | renamed -> src/archived/fir/ |
| `src/fir/toh.c` | Stale (Renamed) | renamed -> src/archived/fir/ |
| `src/lua/src/lapi.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lapi.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lauxlib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lauxlib.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lbaselib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lcode.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lcode.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ldblib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ldebug.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ldebug.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ldo.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ldo.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ldump.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lfunc.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lfunc.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lgc.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lgc.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/linit.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/liolib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/llex.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/llex.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/llimits.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lmathlib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lmem.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lmem.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/loadlib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lobject.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lobject.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lopcodes.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lopcodes.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/loslib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lparser.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lparser.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lstate.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lstate.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lstring.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lstring.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lstrlib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ltable.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ltable.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ltablib.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ltm.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/ltm.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lua.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lua.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/luac.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/luaconf.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lualib.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lundump.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lundump.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lvm.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lvm.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lzio.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/lzio.h` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/lua/src/print.c` | Stale (Replaced by Rust) | see src/rust/crates/fceux11-lua/ |
| `src/fcoeffs.h` | Stale |  |
| `src/ld65dbg.cpp` | Stale |  |
| `src/ld65dbg.h` | Stale |  |
| `src/types-des.h` | Stale |  |

## 6. Full File Listing

| Status | Bucket | File | License | Copyrights |
|--------|--------|------|---------|-----------|
| New | archived | `src/archived/fceux-server/md5.cpp` | GPLv2+ |  |
| New | archived | `src/archived/fceux-server/md5.h` | GPLv2+ |  |
| New | archived | `src/archived/fceux-server/server.cpp` | GPLv2+ | * Copyright (C) 2004 Xodnizel |
| New | archived | `src/archived/fceux-server/throttle.cpp` | GPLv2+ | * Copyright (C) 2004 Xodnizel |
| New | archived | `src/archived/fceux-server/throttle.h` | GPLv2+ | * Copyright (C) 2004 Xodnizel |
| New | archived | `src/archived/fceux-server/types.h` | GPLv2+ | * Copyright (C) 2004 Xodnizel |
| New | archived | `src/archived/fir/c44100ntsc.h` | Unknown/None |  |
| New | archived | `src/archived/fir/c44100pal.h` | Unknown/None |  |
| New | archived | `src/archived/fir/c48000ntsc.h` | Unknown/None |  |
| New | archived | `src/archived/fir/c48000pal.h` | Unknown/None |  |
| New | archived | `src/archived/fir/c96000ntsc.h` | Unknown/None |  |
| New | archived | `src/archived/fir/c96000pal.h` | Unknown/None |  |
| New | archived | `src/archived/fir/toh.c` | Unknown/None |  |
| New | boards | `src/boards/_cart_helpers.cpp` | GPLv2+ |  |
| New | boards | `src/boards/_cart_helpers.h` | GPLv2+ |  |
| New | boards | `src/boards/datalatch_carts.h` | GPLv2+ |  |
| New | boards | `src/boards/irem_txc_bit_carts.h` | GPLv2+ |  |
| New | boards | `src/boards/legacy_expansion_audio.h` | GPLv2+ |  |
| New | boards | `src/boards/mapinc_audio.h` | Unknown/None |  |
| New | boards | `src/boards/mapinc_base.h` | Unknown/None |  |
| New | boards | `src/boards/mapinc_bus.h` | Unknown/None |  |
| New | boards | `src/boards/mapinc_mmc3.h` | Unknown/None |  |
| New | boards | `src/boards/mapinc_state.h` | Unknown/None |  |
| New | boards | `src/boards/mapper_strategy_a.h` | GPLv2+ |  |
| New | boards | `src/boards/mmc1_cart.h` | GPLv2+ |  |
| New | boards | `src/boards/mmc3_base_cart.cpp` | GPLv2+ |  |
| New | boards | `src/boards/mmc3_base_cart.h` | GPLv2+ |  |
| New | boards | `src/boards/mmc3_cart.h` | GPLv2+ |  |
| New | boards | `src/boards/mmc3_variants_carts.h` | GPLv2+ |  |
| New | boards | `src/boards/nrom_cart.h` | GPLv2+ |  |
| New | boards | `src/boards/registry.cpp` | Unknown/None |  |
| New | boards | `src/boards/registry.h` | Unknown/None |  |
| New | boards | `src/boards/simple_carts.h` | Unknown/None |  |
| New | boards | `src/boards/smb2j_carts.h` | GPLv2+ |  |
| New | boards | `src/boards/vrc2and4_carts.h` | GPLv2+ |  |
| New | boards | `src/boards/vrc6_cart.h` | GPLv2+ |  |
| New | drivers | `src/drivers/common/nes_shm.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/common/nes_shm.h` | Unknown/None |  |
| New | drivers | `src/drivers/null/null_driver.cpp` | GPLv2+ |  |
| New | drivers | `src/drivers/null/null_driver.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/AviAudioCodec.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/AviAudioCodec.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/AviOptionsDialog.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/AviOptionsDialog.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/AviRecordContext.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/AviRecordDiskThread.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/AviVideoCodec.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/AviVideoCodec.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConfigStore.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleActions.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleActions.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleCursor.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleDebugWindows.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleDebugWindows.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleEmuControl.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleEmuControl.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleEmulatorThread.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleFile.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleFile.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleHotKeys.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleMenu.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleMenu.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleMenuBar.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleRecentRom.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleRecentRom.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleRecording.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleRecording.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleTranslation.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleTranslation.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleVideo.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| New | drivers | `src/drivers/Qt/ConsoleVideo.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleVideoSetup.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ConsoleWindowContext.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/fceu_archive.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| New | drivers | `src/drivers/Qt/fceu_callbacks.cpp` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/fceu_globals.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| New | drivers | `src/drivers/Qt/input/input_backend.h` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/input_device.h` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/input_manager.cpp` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/input_manager.h` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/sdl_backend.cpp` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/sdl_backend.h` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/wgi_backend.cpp` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/wgi_backend.h` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/xinput_backend.cpp` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/input/xinput_backend.h` | GPLv2+ |  |
| New | drivers | `src/drivers/Qt/MenuCatalog.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerContext.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerContext.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerPalette.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerPalette.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerPatternTables.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerPatternTables.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerSpriteViewer.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerSpriteViewer.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerTileEditor.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/ppuViewerTileEditor.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/QtNetplay.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| New | drivers | `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/TasEditor/markerDragPopup.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/TasEditor/markerDragPopup.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/TasEditor/TasEditorContext.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/TasEditor/TasEditorTimeline.cpp` | GPLv2+ | * Copyright (C) 2021 mjbudd77 |
| New | drivers | `src/drivers/Qt/TasEditor/TasEditorTimeline.h` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/TasEditor/TasFindNoteWindow.cpp` | Unknown/None |  |
| New | drivers | `src/drivers/Qt/TasEditor/TasFindNoteWindow.h` | Unknown/None |  |
| New | platform | `src/platform/win11/DirectStorageProbe.cpp` | Unknown/None |  |
| New | platform | `src/platform/win11/DirectStorageProbe.h` | Unknown/None |  |
| New | platform | `src/platform/win11/TaskbarProgress.cpp` | Unknown/None |  |
| New | platform | `src/platform/win11/TaskbarProgress.h` | GPLv2+ |  |
| New | root | `src/apu.cpp` | GPLv2+ |  |
| New | root | `src/apu.h` | GPLv2+ |  |
| New | root | `src/bus.cpp` | GPLv2+ |  |
| New | root | `src/bus.h` | GPLv2+ |  |
| New | root | `src/cart_class.cpp` | GPLv2+ |  |
| New | root | `src/cart_class.h` | GPLv2+ |  |
| New | root | `src/compiler_attrs.h` | Unknown/None |  |
| New | root | `src/core_api.h` | GPLv2+ | * Copyright (C) 2001 Aaron Oneal; * Copyright (C) 2002 Xodnizel |
| New | root | `src/core_state.cpp` | GPLv2+ |  |
| New | root | `src/core_state.h` | GPLv2+ |  |
| New | root | `src/cpu.cpp` | GPLv2+ |  |
| New | root | `src/cpu.h` | GPLv2+ |  |
| New | root | `src/diag_api.h` | GPLv2+ | * Copyright (C) 2001 Aaron Oneal; * Copyright (C) 2002 Xodnizel |
| New | root | `src/driver_callbacks.cpp` | GPLv2+ |  |
| New | root | `src/driver_callbacks.h` | GPLv2+ |  |
| New | root | `src/expansion_audio.cpp` | GPLv2+ |  |
| New | root | `src/expansion_audio.h` | GPLv2+ |  |
| New | root | `src/fceu11_core_types.h` | GPLv2+ |  |
| New | root | `src/fds_sound.cpp` | Unknown/None |  |
| New | root | `src/ines_bmap.h` | Unknown/None |  |
| New | root | `src/ines_gi.cpp` | Unknown/None |  |
| New | root | `src/ines_init.cpp` | GPLv2+ |  |
| New | root | `src/ines_load.cpp` | GPLv2+ |  |
| New | root | `src/ines_save.cpp` | Unknown/None |  |
| New | root | `src/io_api.h` | GPLv2+ | * Copyright (C) 2001 Aaron Oneal; * Copyright (C) 2002 Xodnizel |
| New | root | `src/kagami_bridge.cpp` | GPLv2+ |  |
| New | root | `src/kagami_bridge.h` | GPLv2+ |  |
| New | root | `src/movie_fm2.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel |
| New | root | `src/movie_fm2.h` | Unknown/None |  |
| New | root | `src/movie_io.cpp` | GPLv2+ |  |
| New | root | `src/movie_playback.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel |
| New | root | `src/movie_playback.h` | Unknown/None |  |
| New | root | `src/movie_record.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel |
| New | root | `src/movie_record.h` | Unknown/None |  |
| New | root | `src/movie_settings.cpp` | Unknown/None |  |
| New | root | `src/movie_subtitles.cpp` | Unknown/None |  |
| New | root | `src/movie_taseditor_bridge.cpp` | Unknown/None |  |
| New | root | `src/net_api.h` | GPLv2+ | * Copyright (C) 2001 Aaron Oneal; * Copyright (C) 2002 Xodnizel |
| New | root | `src/nsf_load.cpp` | GPLv2+ |  |
| New | root | `src/nsf_runtime.cpp` | GPLv2+ |  |
| New | root | `src/nsf_ui.cpp` | GPLv2+ |  |
| New | root | `src/ppu_class.cpp` | GPLv2+ |  |
| New | root | `src/ppu_class.h` | GPLv2+ |  |
| New | root | `src/ppu_core.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel |
| New | root | `src/ppu_core.h` | Unknown/None |  |
| New | root | `src/ppu_rendering.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel |
| New | root | `src/ppu_rendering.h` | Unknown/None |  |
| New | root | `src/ppu_sprite_lut.cpp` | Unknown/None |  |
| New | root | `src/ppu_sprite_lut.h` | Unknown/None |  |
| New | root | `src/ppu_state.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel |
| New | root | `src/ppu_state.h` | Unknown/None |  |
| New | root | `src/pputile_template.cpp` | Unknown/None |  |
| New | root | `src/pputile_template.h` | Unknown/None |  |
| New | root | `src/unif_bmap.h` | Unknown/None |  |
| New | root | `src/unif_load.cpp` | GPLv2+ |  |
| New | rust | `src/rust/build.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-core/src/bus.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-core/src/lib.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-core/src/sformat.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-core/src/state_file.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-core/src/state_recorder.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-core/src/traits.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-debug/src/asm.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-debug/src/cheat.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-debug/src/conddebug.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-debug/src/debug.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-debug/src/debugsym.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-debug/src/ld65dbg.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-debug/src/lib.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-formats/src/cart.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-formats/src/emufile.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-formats/src/fds.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-formats/src/ines.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-formats/src/ines/ines_data.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-formats/src/lib.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-formats/src/movie.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-formats/src/nsf.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-formats/src/unif.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-formats/src/vsuni.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/bit.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/debugger.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/emu.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/gui.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/input.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/joypad.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/memory.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/mod.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/movie.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/ppu.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/rom.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/savestate.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/sound.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/bindings/zapper.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/ffi_stubs.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-lua/src/lib.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-media/src/drawing.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-media/src/fcoeffs.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-media/src/filter.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-media/src/lib.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-media/src/palette.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-media/src/video.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-media/src/wave.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-utils/src/convert_utf.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-utils/src/crc32.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-utils/src/general.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-utils/src/guid.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-utils/src/lib.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-utils/src/md5.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-utils/src/os_utils.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/fceux11-utils/src/profiler.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-utils/src/slice.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/fceux11-utils/src/timestamp.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/build.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/adapter/direct.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/adapter/mod.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/adapter/subprocess.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/adapter/trait_def.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/cli/args.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/cli/mod.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/cli/run_direct.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/cli/run_report.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/cli/run_subprocess.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/core/config.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/core/error.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/core/mod.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/lib.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/main.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/manifest/filter.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/manifest/mod.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/manifest/parser.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/manifest/schema.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/oracle/hardware.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/oracle/mod.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/oracle/regression.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/report/baseline.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/report/grade.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/report/matrix.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/report/mod.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/report/pdf.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/blargg.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/direct.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/lua.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/mapper_byte_diff.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/mod.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/rom_regression.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/savestate_regression.rs` | GPLv2+ |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/scheduler.rs` | Unknown/None |  |
| New | rust | `src/rust/crates/kagami-qa/src/runner/test_helpers.rs` | GPLv2+ |  |
| New | rust | `src/rust/fceux11_rust.h` | GPLv2+ |  |
| New | rust | `src/rust/src/lib.rs` | GPLv2+ |  |
| New | rust | `src/rust/tests/rom_tests/src/lib.rs` | GPLv2+ |  |
| New | tests | `src/tests/boards/mapper_load_test.cpp` | Unknown/None |  |
| New | tests | `src/tests/boards/mapper_reset_test.cpp` | Unknown/None |  |
| New | tests | `src/tests/git_info_stub.cpp` | Unknown/None |  |
| New | tests | `src/tests/rom_regression_test.cpp` | Unknown/None |  |
| New | tests | `src/tests/smoke_test.cpp` | GPLv2+ |  |
| New | utils | `src/utils/backward.hpp` | MIT | * Copyright 2013 Google Inc. All Rights Reserved. |
| New | utils | `src/utils/cache.h` | GPLv2+ |  |
| New | utils | `src/utils/enum_class_bitflags.h` | GPLv2+ |  |
| New | utils | `src/utils/expected.hpp` | Unknown/None |  |
| New | utils | `src/utils/fceu11_expected.cpp` | GPLv2+ |  |
| New | utils | `src/utils/fceu11_expected.h` | GPLv2+ |  |
| New | utils | `src/utils/fceu11_format.h` | GPLv2+ |  |
| New | utils | `src/utils/format.h` | GPLv2+ |  |
| New | utils | `src/utils/platform_compat.h` | GPLv2+ |  |
| New | utils | `src/utils/safe_string.h` | Unknown/None |  |
| New | utils | `src/utils/simd_fill.h` | GPLv2+ |  |
| New | utils | `src/utils/tl/expected.hpp` | Unknown/None |  |
| OK | boards | `src/boards/__dummy_mapper.cpp` | GPLv2+ | * Copyright (C) 2013 CaH4e3 |
| OK | boards | `src/boards/01-222.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/09-034a.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/103.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/106.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/108.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/112.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/116.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/117.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/120.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/121.cpp` | GPLv2+ | * Copyright (C) 2007-2008 Mad Dumper, CaH4e3 |
| OK | boards | `src/boards/12in1.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/15.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/151.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/156.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/158B.cpp` | GPLv2+ | * Copyright (C) 2015 CaH4e3 |
| OK | boards | `src/boards/164.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel 2006 CaH4e3 |
| OK | boards | `src/boards/168.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/170.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/175.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/176.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3; * Copyright (C) 2012 FCEUX team |
| OK | boards | `src/boards/177.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/178.cpp` | GPLv2+ | * Copyright (C) 2013 CaH4e3 |
| OK | boards | `src/boards/18.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/183.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/185.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/186.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/187.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/189.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/190.cpp` | GPLv2+ | * Copyright (C) 2017 FCEUX Team |
| OK | boards | `src/boards/193.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/199.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/206.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/208.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/222.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/225.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3; * Copyright (C) 2019 Libretro Team; * Copyright (C) 2020 |
| OK | boards | `src/boards/228.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/230.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3; * Copyright (C) 2009 qeed |
| OK | boards | `src/boards/232.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/234.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/235.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/244.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/246.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/252.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/253.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/28.cpp` | GPLv2+ | Copyright (C) 2012-2017 FCEUX team |
| OK | boards | `src/boards/32.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/33.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/34.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/354.cpp` | GPLv2+ | * Copyright (C) 2022 |
| OK | boards | `src/boards/36.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/3d-block.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/40.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/41.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/411120-c.cpp` | GPLv2+ | * Copyright (C) 2008 CaH4e3 |
| OK | boards | `src/boards/42.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/43.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/46.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/50.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/51.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/57.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/603-5052.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/62.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/65.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/67.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/68.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/69.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/71.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/72.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/77.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/79.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/80.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/80013-B.cpp` | GPLv2+ | * Copyright (C) 2017 CaH4e3 |
| OK | boards | `src/boards/8157.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/82.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/8237.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/830118C.cpp` | GPLv2+ | * Copyright (C) 2008 CaH4e3 |
| OK | boards | `src/boards/88.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/8in1.cpp` | GPLv2+ | * Copyright (C) 2016 CaH4e3 |
| OK | boards | `src/boards/90.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel; * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/91.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/96.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2002 Xodnizel; * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/99.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/a9746.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/ac-08.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/addrlatch.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/ax5705.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/bandai.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3; * Copyright (C) 2011 FCEUX team |
| OK | boards | `src/boards/bb.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/bmc13in1jy110.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/bmc42in1r.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3; * Copyright (C) 2009 qeed |
| OK | boards | `src/boards/bmc64in1nr.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/bmc70in1.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/BMW8544.cpp` | GPLv2+ | * Copyright (C) 2015 CaH4e3 |
| OK | boards | `src/boards/bonza.cpp` | GPLv2+ | * Copyright (C) 2002 CaH4e3 |
| OK | boards | `src/boards/bs4xxxr.cpp` | GPLv2+ | * Copyright (C) 2020 CaH4e3 |
| OK | boards | `src/boards/bs-5.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/cheapocabra.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/cityfighter.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/coolboy.cpp` | GPLv2+ | * Copyright (C) 2018 CaH4e3, Cluster |
| OK | boards | `src/boards/coolgirl.cpp` | GPLv2+ | * Copyright (C) 2022 Cluster |
| OK | boards | `src/boards/dance2000.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/datalatch.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/dream.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/edu2000.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/eh8813a.cpp` | GPLv2+ | * Copyright (C) 2015 CaH4e3 |
| OK | boards | `src/boards/emu2413.c` | Unknown/None |  |
| OK | boards | `src/boards/emu2413.h` | Unknown/None |  |
| OK | boards | `src/boards/et-100.cpp` | GPLv2+ | * Copyright (C) 2015 Cluster |
| OK | boards | `src/boards/et-4320.cpp` | GPLv2+ | * Copyright (C) 2016 Cluster |
| OK | boards | `src/boards/F-15.cpp` | GPLv2+ | * Copyright (C) 2015 CaH4e3 |
| OK | boards | `src/boards/famicombox.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/ffe.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/fk23c.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/fns.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2002 Xodnizel; * Copyright (C) 2020 CaH4e3 |
| OK | boards | `src/boards/ghostbusters63in1.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/gs-2004.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/gs-2013.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/h2288.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/hp10xx_hp20xx.cpp` | GPLv2+ | * Copyright (C) 2017 CaH4e3 |
| OK | boards | `src/boards/hp898f.cpp` | GPLv2+ | * Copyright (C) 2015 CaH4e3 |
| OK | boards | `src/boards/inlnsf.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/inx007t.cpp` | GPLv2+ | * Copyright (C) 2022 Cluster |
| OK | boards | `src/boards/karaoke.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/kof97.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/ks7010.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/ks7012.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/ks7013.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/ks7016.cpp` | GPLv2+ | * Copyright (C) 2016 CaH4e3 |
| OK | boards | `src/boards/ks7017.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/ks7030.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/ks7031.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/ks7032.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/ks7037.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/ks7057.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/le05.cpp` | GPLv2+ | * Copyright (C) 2011 CaH4e3 |
| OK | boards | `src/boards/lh32.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/lh53.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/malee.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/mihunche.cpp` | GPLv2+ | * Copyright (C) 2013 CaH4e3 |
| OK | boards | `src/boards/mmc1.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/mmc2and4.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3; * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/mmc3.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel; * Mapper 12 code Copyright (C) 2003 CaH4e3 |
| OK | boards | `src/boards/mmc3.h` | GPLv2+ |  |
| OK | boards | `src/boards/mmc5.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/n106.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/n625092.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/novel.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/onebus.cpp` | GPLv2+ | * Copyright (C) 2007-2010 CaH4e3 |
| OK | boards | `src/boards/pec-586.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/rt-01.cpp` | GPLv2+ | * Copyright (C) 2016 CaH4e3 |
| OK | boards | `src/boards/sa-9602b.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/sachen.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/sb-2000.cpp` | GPLv2+ | * Copyright (C) 2014 CaH4e3 |
| OK | boards | `src/boards/sc-127.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/sheroes.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/sl1632.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/subor.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/super24.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/supervision.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/t-227-1.cpp` | GPLv2+ | * Copyright (C) 2008 CaH4e3 |
| OK | boards | `src/boards/t-262.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | boards | `src/boards/tengen.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | boards | `src/boards/tf-1201.cpp` | GPLv2+ | * Copyright (C) 2005 CaH4e3 |
| OK | boards | `src/boards/transformer.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/unrom512.cpp` | GPLv2+ | * Copyright (C) 2014 CaitSith2, 2022 Cluster |
| OK | boards | `src/boards/vrc1.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/vrc2and4.cpp` | GPLv2+ | * Copyright (C) 2007 CaH4e3 |
| OK | boards | `src/boards/vrc3.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/vrc5.cpp` | GPLv2+ | * Copyright (C) 2005-2019 CaH4e3 |
| OK | boards | `src/boards/vrc6.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/vrc7.cpp` | GPLv2+ | * Copyright (C) 2012 CaH4e3 |
| OK | boards | `src/boards/vrc7p.cpp` | GPLv2+ | * Copyright (C) 2009 CaH4e3 |
| OK | boards | `src/boards/yoko.cpp` | GPLv2+ | * Copyright (C) 2006 CaH4e3 |
| OK | drivers | `src/drivers/common/args.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/common/args.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/cheat.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/common/cheat.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/common/config.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/common/config.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/configSys.cpp` | Unknown/None |  |
| OK | drivers | `src/drivers/common/configSys.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/hq2x.cpp` | Unknown/None | Copyright (C) 2003 MaxSt ( maxst@hiend3d.com ) |
| OK | drivers | `src/drivers/common/hq2x.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/hq3x.cpp` | Unknown/None | Copyright (C) 2003 MaxSt ( maxst@hiend3d.com ) |
| OK | drivers | `src/drivers/common/hq3x.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/nes_ntsc.c` | LGPL |  |
| OK | drivers | `src/drivers/common/nes_ntsc.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/nes_ntsc_config.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/nes_ntsc_impl.h` | LGPL |  |
| OK | drivers | `src/drivers/common/os_utils.cpp` | GPLv2+ |  |
| OK | drivers | `src/drivers/common/os_utils.h` | Unknown/None |  |
| OK | drivers | `src/drivers/common/scale2x.cpp` | GPLv2+ | * Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| OK | drivers | `src/drivers/common/scale2x.h` | GPLv2+ | * Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| OK | drivers | `src/drivers/common/scale3x.cpp` | GPLv2+ | * Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| OK | drivers | `src/drivers/common/scale3x.h` | GPLv2+ | * Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| OK | drivers | `src/drivers/common/scalebit.cpp` | GPLv2+ | * Copyright (C) 2003 Andrea Mazzoleni |
| OK | drivers | `src/drivers/common/scalebit.h` | GPLv2+ | * Copyright (C) 2003 Andrea Mazzoleni |
| OK | drivers | `src/drivers/common/vidblit.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/common/vidblit.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/Qt/AboutWindow.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/AboutWindow.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/avi/avi-utils.cpp` | BSD | * Copyright (c) 2008-2011, Michael Kohn; * Copyright (c) 2013, Robin Hahling |
| OK | drivers | `src/drivers/Qt/avi/fileio.cpp` | BSD | * Copyright (c) 2008-2011, Michael Kohn; * Copyright (c) 2013, Robin Hahling |
| OK | drivers | `src/drivers/Qt/avi/gwavi.cpp` | BSD | * Copyright (c) 2008-2011, Michael Kohn; * Copyright (c) 2013, Robin Hahling |
| OK | drivers | `src/drivers/Qt/avi/gwavi.h` | BSD | * Copyright (c) 2008-2011, Michael Kohn; * Copyright (c) 2013, Robin Hahling |
| OK | drivers | `src/drivers/Qt/AviRecord.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/AviRecord.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/AviRiffViewer.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/AviRiffViewer.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/CheatsConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/CheatsConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/CodeDataLogger.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/CodeDataLogger.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ColorMenu.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ColorMenu.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/config.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/config.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleDebugger.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleDebugger.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleSoundConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleSoundConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleUtilities.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleUtilities.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleVideoConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleVideoConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleViewerGL.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleViewerGL.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleViewerInterface.cpp` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleViewerInterface.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleViewerQWidget.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleViewerQWidget.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleViewerSDL.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleViewerSDL.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ConsoleWindow.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ConsoleWindow.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/dface.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/FamilyKeyboard.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/FamilyKeyboard.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/fceuWrapper.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/fceuWrapper.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/fceux_git_info.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/FrameTimingStats.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/FrameTimingStats.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/GameGenie.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/GameGenie.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/GamePadConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/GamePadConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/GuiConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/GuiConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/HelpPages.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/HelpPages.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/HexEditor.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/HexEditor.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/HotKeyConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/HotKeyConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/iNesHeaderEditor.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/iNesHeaderEditor.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/input.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/Qt/input.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/InputConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/InputConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/keyscan.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/keyscan.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/LuaControl.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/LuaControl.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/main.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/main.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/Qt/MovieOptions.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/MovieOptions.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/MoviePlay.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/MoviePlay.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/MovieRecord.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/MovieRecord.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/MsgLogViewer.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/MsgLogViewer.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/NameTableViewer.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/NameTableViewer.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/PaletteConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/PaletteConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/PaletteEditor.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/PaletteEditor.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/ppuViewer.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/ppuViewer.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/RamSearch.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/RamSearch.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/RamWatch.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/RamWatch.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/sdl.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/sdl-joystick.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel; * Copyright (C) 2002 Paul Kuliniewicz |
| OK | drivers | `src/drivers/Qt/sdl-joystick.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/sdl-sound.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/Qt/sdl-throttle.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/sdl-video.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | drivers | `src/drivers/Qt/sdl-video.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/SplashScreen.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/SplashScreen.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/StateRecorderConf.cpp` | GPLv2+ | * Copyright (C) 2022 thor2016 |
| OK | drivers | `src/drivers/Qt/StateRecorderConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/SymbolicDebug.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/SymbolicDebug.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/bookmark.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/bookmark.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/bookmarks.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/bookmarks.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/branches.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/branches.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/greenzone.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/greenzone.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/history.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/history.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/inputlog.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/inputlog.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/laglog.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/laglog.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/markers.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/markers.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/markers_manager.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/markers_manager.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/playback.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/playback.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/recorder.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/recorder.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/selection.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/selection.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/snapshot.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/snapshot.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/splicer.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/splicer.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/TasColors.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/taseditor_config.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/taseditor_config.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/taseditor_lua.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/taseditor_lua.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/taseditor_project.cpp` | MIT | Copyright (c) 2011-2013 AnS |
| OK | drivers | `src/drivers/Qt/TasEditor/taseditor_project.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TasEditor/TasEditorWindow.cpp` | GPLv2+ | * Copyright (C) 2021 mjbudd77 |
| OK | drivers | `src/drivers/Qt/TasEditor/TasEditorWindow.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/throttle.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TimingConf.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/TimingConf.h` | Unknown/None |  |
| OK | drivers | `src/drivers/Qt/TraceLogger.cpp` | GPLv2+ | * Copyright (C) 2020 mjbudd77 |
| OK | drivers | `src/drivers/Qt/TraceLogger.h` | Unknown/None |  |
| OK | input | `src/input/arkanoid.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/bworld.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | input | `src/input/cursor.cpp` | Unknown/None |  |
| OK | input | `src/input/fkb.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/fkb.h` | Unknown/None |  |
| OK | input | `src/input/fns.cpp` | GPLv2+ | * Copyright (C) 2019 CaH4e3 |
| OK | input | `src/input/ftrainer.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | input | `src/input/hypershot.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | input | `src/input/lcdcompzapper.cpp` | GPLv2+ |  |
| OK | input | `src/input/mahjong.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | input | `src/input/mouse.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/oekakids.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | input | `src/input/pec586kb.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/powerpad.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/quiz.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | input | `src/input/shadow.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/share.h` | Unknown/None |  |
| OK | input | `src/input/snesmouse.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/suborkb.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/suborkb.h` | Unknown/None |  |
| OK | input | `src/input/toprider.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | input | `src/input/virtualboy.cpp` | GPLv2+ |  |
| OK | input | `src/input/zapper.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | input | `src/input/zapper.h` | Unknown/None |  |
| OK | palettes | `src/palettes/conv.c` | Unknown/None |  |
| OK | palettes | `src/palettes/palettes.h` | Unknown/None |  |
| OK | palettes | `src/palettes/rp2c04001.h` | Unknown/None |  |
| OK | palettes | `src/palettes/rp2c04002.h` | Unknown/None |  |
| OK | palettes | `src/palettes/rp2c04003.h` | Unknown/None |  |
| OK | palettes | `src/palettes/rp2c05004.h` | Unknown/None |  |
| OK | root | `src/asm.cpp` | GPLv2+ |  |
| OK | root | `src/asm.h` | Unknown/None |  |
| OK | root | `src/cart.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/cart.h` | Unknown/None |  |
| OK | root | `src/cheat.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/cheat.h` | GPLv2+ |  |
| OK | root | `src/conddebug.cpp` | GPLv2+ | * Copyright (C) 2005 Sebastian Porst |
| OK | root | `src/conddebug.h` | GPLv2+ | * Copyright (C) 2005 Sebastian Porst |
| OK | root | `src/config.cpp` | Unknown/None |  |
| OK | root | `src/debug.cpp` | GPLv2+ |  |
| OK | root | `src/debug.h` | Unknown/None |  |
| OK | root | `src/debugsymboltable.cpp` | GPLv2+ |  |
| OK | root | `src/debugsymboltable.h` | GPLv2+ |  |
| OK | root | `src/drawing.cpp` | GPLv2+ |  |
| OK | root | `src/drawing.h` | Unknown/None |  |
| OK | root | `src/driver.h` | GPLv2+ | * Copyright (C) 2001 Aaron Oneal; * Copyright (C) 2002 Xodnizel |
| OK | root | `src/emufile.cpp` | GPLv2+ |  |
| OK | root | `src/emufile.h` | MIT | Copyright (C) 2009-2010 DeSmuME team |
| OK | root | `src/emufile_types.h` | Unknown/None |  |
| OK | root | `src/fceu.cpp` | GPLv2+ | * Copyright (C) 2003 Xodnizel |
| OK | root | `src/fceu.h` | Unknown/None |  |
| OK | root | `src/fceulua.h` | Unknown/None |  |
| OK | root | `src/fds.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/fds.h` | Unknown/None |  |
| OK | root | `src/file.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/file.h` | Unknown/None |  |
| OK | root | `src/filter.cpp` | GPLv2+ |  |
| OK | root | `src/filter.h` | Unknown/None |  |
| OK | root | `src/git.h` | Unknown/None |  |
| OK | root | `src/ines.cpp` | Unknown/None |  |
| OK | root | `src/ines.h` | GPLv2+ | * Copyright (C) 1998 Bero; * Copyright (C) 2002 Xodnizel |
| OK | root | `src/ines-bad.h` | Unknown/None |  |
| OK | root | `src/ines-correct.h` | Unknown/None |  |
| OK | root | `src/input.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2002 Xodnizel |
| OK | root | `src/input.h` | Unknown/None |  |
| OK | root | `src/lua-engine.cpp` | GPLv2+ |  |
| OK | root | `src/movie.cpp` | GPLv2+ |  |
| OK | root | `src/movie.h` | GPLv2+ |  |
| OK | root | `src/netplay.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/netplay.h` | Unknown/None |  |
| OK | root | `src/nsf.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/nsf.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/oldmovie.cpp` | Unknown/None |  |
| OK | root | `src/oldmovie.h` | Unknown/None |  |
| OK | root | `src/palette.cpp` | GPLv2+ | * Copyright (C) 2002,2003 Xodnizel |
| OK | root | `src/palette.h` | Unknown/None |  |
| OK | root | `src/ppu.cpp` | GPLv2+ | * Copyright (C) 1998 BERO; * Copyright (C) 2003 Xodnizel |
| OK | root | `src/ppu.h` | GPLv2+ |  |
| OK | root | `src/profiler.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/profiler.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/sound.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/sound.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/state.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/state.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/types.h` | GPLv2+ | * Copyright (C) 2001 Aaron Oneal; * Copyright (C) 2002 Xodnizel |
| OK | root | `src/unif.cpp` | Unknown/None |  |
| OK | root | `src/unif.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/version.h` | GPLv2+ | * Copyright (C) 2001 Aaron Oneal; * Copyright (C) 2002 Xodnizel; Copyright (C) 2026 FCEUX11 Contributors |
| OK | root | `src/video.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | root | `src/video.h` | Unknown/None |  |
| OK | root | `src/vsuni.cpp` | GPLv2+ |  |
| OK | root | `src/vsuni.h` | Unknown/None |  |
| OK | root | `src/wave.cpp` | GPLv2+ |  |
| OK | root | `src/wave.h` | Unknown/None |  |
| OK | utils | `src/utils/backward.cpp` | Unknown/None |  |
| OK | utils | `src/utils/ConvertUTF.c` | GPLv2+ |  |
| OK | utils | `src/utils/ConvertUTF.h` | Unknown/None | * Copyright 2001-2004 Unicode, Inc. |
| OK | utils | `src/utils/crc32.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | utils | `src/utils/crc32.h` | Unknown/None |  |
| OK | utils | `src/utils/endian.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | utils | `src/utils/endian.h` | Unknown/None |  |
| OK | utils | `src/utils/general.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | utils | `src/utils/general.h` | Unknown/None |  |
| OK | utils | `src/utils/guid.cpp` | GPLv2+ |  |
| OK | utils | `src/utils/guid.h` | Unknown/None |  |
| OK | utils | `src/utils/ioapi.cpp` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http:; Copyright (C) 2009-2010 Mathias Svensson ( http: |
| OK | utils | `src/utils/ioapi.h` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http:; Copyright (C) 2009-2010 Mathias Svensson ( http: |
| OK | utils | `src/utils/md5.cpp` | GPLv2+ |  |
| OK | utils | `src/utils/md5.h` | GPLv2+ |  |
| OK | utils | `src/utils/memory.cpp` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | utils | `src/utils/memory.h` | GPLv2+ | * Copyright (C) 2002 Xodnizel |
| OK | utils | `src/utils/mutex.cpp` | Unknown/None |  |
| OK | utils | `src/utils/mutex.h` | Unknown/None |  |
| OK | utils | `src/utils/timeStamp.cpp` | GPLv2+ |  |
| OK | utils | `src/utils/timeStamp.h` | Unknown/None |  |
| OK | utils | `src/utils/unzip.cpp` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http:; Copyright (C) 2007-2008 Even Rouault; Copyright (C) 2009-2010 Mathias Svensson ( http:; Copyright (c) 1990-2000 Info-ZIP. All rights reserved.; Copyright (C) 1998 - 2010 Gilles Vollant, Even Rouault, Mathias Svensson |
| OK | utils | `src/utils/unzip.h` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http:; Copyright (C) 2007-2008 Even Rouault; Copyright (C) 2009-2010 Mathias Svensson ( http: |
| OK | utils | `src/utils/valuearray.h` | Unknown/None |  |
| OK | utils | `src/utils/xstring.cpp` | GPLv2+ | * Copyright (C) 2004 Jason Oster (Parasyte) |
| OK | utils | `src/utils/xstring.h` | GPLv2+ | * Copyright (C) 2004 Jason Oster (Parasyte) |
| Stale | boards | `src/boards/mapinc.h` | Unknown/None |  |
| Stale | drivers | `src/drivers/Qt/nes_shm.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| Stale | drivers | `src/drivers/Qt/nes_shm.h` | Unknown/None |  |
| Stale | drivers | `src/drivers/Qt/unix-netplay.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale | drivers | `src/drivers/Qt/unix-netplay.h` | Unknown/None |  |
| Stale | root | `src/fcoeffs.h` | Unknown/None |  |
| Stale | root | `src/ld65dbg.cpp` | Unknown/None |  |
| Stale | root | `src/ld65dbg.h` | Unknown/None |  |
| Stale | root | `src/types-des.h` | GPLv2+ | Copyright (C) 2005 Guillaume Duhamel; Copyright (C) 2008-2009 DeSmuME team |
| Stale (Driver removed) | drivers | `src/drivers/sdl/cheat.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/cheat.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/config.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/config.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/debugger.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/debugger.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/dface.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/fceux_git_info.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/GamePadConf.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/GamePadConf.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/glxwin.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/glxwin.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/gui.cpp` | GPL |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/gui.h` | GPLv2+ |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/input.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/sdl/input.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/keyscan.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/main.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/sdl/memview.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/memview.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/ramwatch.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/ramwatch.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-icon.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-joystick.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel; Copyright (C) 2002 Paul Kuliniewicz |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-joystick.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-netplay.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-sound.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-throttle.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-video.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/sdl/sdl-video.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/throttle.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/sdl/unix-netplay.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/sdl/unix-netplay.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/videolog/nesvideos-piece.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/videolog/nesvideos-piece.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/videolog/quantize.h` | Unknown/None | Copyright (C) 1992,2008 Joel Yliluoma (http://iki.fi/bisqwit/) |
| Stale (Driver removed) | drivers | `src/drivers/videolog/rgbtorgb.cpp` | Unknown/None | Copyright (C) 1992,2008 Joel Yliluoma (http://iki.fi/bisqwit/) |
| Stale (Driver removed) | drivers | `src/drivers/videolog/rgbtorgb.h` | Unknown/None | Copyright (C) 1992,2008 Joel Yliluoma (http://iki.fi/bisqwit/) |
| Stale (Driver removed) | drivers | `src/drivers/videolog/simd.h` | Unknown/None | Copyright (C) 1992,2008 Joel Yliluoma (http://iki.fi/bisqwit/) |
| Stale (Driver removed) | drivers | `src/drivers/win/7zip/IArchive.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/7zip/IProgress.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/7zip/IStream.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/7zip/MyUnknown.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/7zip/PropID.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/7zip/Types.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/afxres.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/archive.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/archive.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/args.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/args.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/aviout.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/cdlogger.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/cdlogger.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/cheat.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/cheat.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/common.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/common.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/config.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/config.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/debug.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/debugger.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/debugger.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/debuggersp.cpp` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| Stale (Driver removed) | drivers | `src/drivers/win/debuggersp.h` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| Stale (Driver removed) | drivers | `src/drivers/win/directories.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/directories.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/directx/ddraw.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/directx/dinput.h` | Unknown/None | Copyright (C) 1996-2000 Microsoft Corporation.  All Rights Reserved. |
| Stale (Driver removed) | drivers | `src/drivers/win/directx/dsound.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/directx/x64/ddraw.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/directx/x64/dinput.h` | Unknown/None | Copyright (C) 1996-2000 Microsoft Corporation.  All Rights Reserved. |
| Stale (Driver removed) | drivers | `src/drivers/win/directx/x64/dsound.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/gui.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/gui.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/guiconfig.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/guiconfig.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/header_editor.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/header_editor.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/help.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/help.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/input.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/input.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/joystick.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/joystick.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/keyboard.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/keyboard.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/keyscan.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/log.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/log.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/lauxlib.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/llimits.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/lmem.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/lobject.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/lstate.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/ltm.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/lua.h` | Unknown/None | Copyright (C) 1994-2008 Lua.org, PUC-Rio"; Copyright (C) 1994-2008 Lua.org, PUC-Rio.  All rights reserved. |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/luaconf.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/lualib.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/lua/include/lzio.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/luaconsole.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/main.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/main.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/mapinput.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/mapinput.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/memview.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/memview.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/memviewsp.cpp` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| Stale (Driver removed) | drivers | `src/drivers/win/memviewsp.h` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| Stale (Driver removed) | drivers | `src/drivers/win/memwatch.cpp` | GPLv2+ | Copyright (C) 2006 Luke Gustafson |
| Stale (Driver removed) | drivers | `src/drivers/win/memwatch.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/monitor.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/monitor.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/movieoptions.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/movieoptions.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/netplay.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/netplay.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/ntview.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/ntview.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/oakra.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/OutputDS.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/palette.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/palette.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/ppuview.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/ppuview.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/pref.cpp` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| Stale (Driver removed) | drivers | `src/drivers/win/pref.h` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| Stale (Driver removed) | drivers | `src/drivers/win/ram_search.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/ram_search.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/ramwatch.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/ramwatch.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/replay.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/replay.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/resource.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/sound.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel and zeromus |
| Stale (Driver removed) | drivers | `src/drivers/win/sound.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/state.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/state.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/bookmark.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/bookmark.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/bookmarks.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/bookmarks.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/branches.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/branches.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/editor.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/editor.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/greenzone.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/greenzone.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/history.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/history.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/inputlog.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/inputlog.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/laglog.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/laglog.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/markers.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/markers.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/markers_manager.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/markers_manager.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/piano_roll.cpp` | GPL |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/piano_roll.h` | GPL |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/playback.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/playback.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/popup_display.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/popup_display.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/recorder.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/recorder.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/selection.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/selection.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/snapshot.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/snapshot.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/splicer.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/splicer.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_config.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_config.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_lua.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_lua.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_project.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_project.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_window.cpp` | GPL |  |
| Stale (Driver removed) | drivers | `src/drivers/win/taseditor/taseditor_window.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/texthook.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/texthook.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/throttle.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/throttle.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/timing.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/timing.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/tracer.cpp` | GPLv2+ | Copyright (C) 2002 Ben Parnell |
| Stale (Driver removed) | drivers | `src/drivers/win/tracer.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/video.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/video.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/wave.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/wave.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/Win32InputBox.cpp` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/Win32InputBox.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/window.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Driver removed) | drivers | `src/drivers/win/window.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/adler32.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/compress.c` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/crc32.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/deflate.c` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/deflate.h` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/example.c` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/gzio.c` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/infblock.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/infblock.h` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/infcodes.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/infcodes.h` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/inffast.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/inffast.h` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/inffixed.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/inflate.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/inftrees.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/inftrees.h` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/infutil.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/infutil.h` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/maketree.c` | Unknown/None | Copyright (C) 1995-2002 Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/trees.c` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/trees.h` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/uncompr.c` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/unzip.c` | Unknown/None |  |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/unzip.h` | Unknown/None | Copyright (C) 1998 Gilles Vollant |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/zconf.h` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/zlib.h` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly and Mark Adler |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/zutil.c` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Driver removed) | drivers | `src/drivers/win/zlib/zutil.h` | Unknown/None | Copyright (C) 1995-2002 Jean-loup Gailly. |
| Stale (Renamed) | attic | `src/attic/fceustr.cpp` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/fceustr.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/dface.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/dos.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/dos.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-joystick.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-joystick.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-keyboard.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-mouse.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-sound.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-sound.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-video.c` | GPLv2+ | Copyright (C) 1998 \Firebug\; Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/dos-video.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/input.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/input.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/keyscan.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/main.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/main.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/sdl.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl-icon.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl-joystick.c` | GPLv2+ | Copyright (C) 2002 Xodnizel; Copyright (C) 2002 Paul Kuliniewicz |
| Stale (Renamed) | attic | `src/attic/pc/sdl-netplay.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl-netplay.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl-opengl.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl-opengl.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl-sound.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/sdl-throttle.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/sdl-video.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/sdl-video.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/throttle.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/throttle.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/unix-netplay.c` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/unix-netplay.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/pc/usage.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| Stale (Renamed) | attic | `src/attic/pc/vgatweak.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/convert.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/convert.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/convertgen.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/drivers/dsound.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/drivers/oss.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/drivers/oss.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/drivers/osxcoreaudio.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/md5.c` | GPL |  |
| Stale (Renamed) | attic | `src/attic/sexyal/md5.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/sexyal.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/sexyal.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/smallc.c` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/sexyal/smallc.h` | Unknown/None |  |
| Stale (Renamed) | attic | `src/attic/soundexp.cpp` | Unknown/None |  |
| Stale (Renamed) | fir | `src/fir/c44100ntsc.h` | Unknown/None |  |
| Stale (Renamed) | fir | `src/fir/c44100pal.h` | Unknown/None |  |
| Stale (Renamed) | fir | `src/fir/c48000ntsc.h` | Unknown/None |  |
| Stale (Renamed) | fir | `src/fir/c48000pal.h` | Unknown/None |  |
| Stale (Renamed) | fir | `src/fir/c96000ntsc.h` | Unknown/None |  |
| Stale (Renamed) | fir | `src/fir/c96000pal.h` | Unknown/None |  |
| Stale (Renamed) | fir | `src/fir/toh.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lapi.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lapi.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lauxlib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lauxlib.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lbaselib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lcode.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lcode.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ldblib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ldebug.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ldebug.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ldo.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ldo.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ldump.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lfunc.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lfunc.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lgc.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lgc.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/linit.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/liolib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/llex.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/llex.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/llimits.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lmathlib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lmem.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lmem.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/loadlib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lobject.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lobject.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lopcodes.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lopcodes.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/loslib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lparser.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lparser.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lstate.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lstate.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lstring.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lstring.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lstrlib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ltable.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ltable.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ltablib.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ltm.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/ltm.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lua.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lua.h` | Unknown/None | Copyright (C) 1994-2008 Lua.org, PUC-Rio"; Copyright (C) 1994-2008 Lua.org, PUC-Rio.  All rights reserved. |
| Stale (Replaced by Rust) | lua | `src/lua/src/luac.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/luaconf.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lualib.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lundump.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lundump.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lvm.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lvm.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lzio.c` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/lzio.h` | Unknown/None |  |
| Stale (Replaced by Rust) | lua | `src/lua/src/print.c` | Unknown/None |  |
