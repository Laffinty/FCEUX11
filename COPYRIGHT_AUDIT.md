﻿# FCEUX11 Source Code Copyright Audit

> **Audit Date**: 2026-05-17; **rescanned 2026-10-07** for the v2.0 tree (fixtures note added 2026-07-28)
> **Scope**: src/ directory (all .cpp, .h, .c files)
> **Excluded from scope**: tests/fixtures/ (third-party test ROMs — see DERIVATIVE_WORK_NOTICE.txt §"Third-Party Test ROM Attribution") and src/rust/target/ (generated build output, gitignored)
> **Methodology note**: the 2026-10-07 rescan re-classified every file from its *current* content with the same header-text rule as the original audit (a file is GPL-family when its text contains "GNU General Public License"; "any later version" makes it GPLv2+). Content that changed since 2026-05-17 — including files whose old GPL headers were replaced by rewritten project code — is therefore reclassified accordingly.
> **Total Files Scanned**: 
650

## 1. License Distribution

| License Type | File Count |
|--------------|-----------:|
| GPLv2 or later | 318 |
| Unknown/None | 332 |

## 2. Unique Copyright Holders

- Copyright (C) 1998 - 2010 Gilles Vollant, Even Rouault, Mathias Svensson
- Copyright (C) 1998 BERO
- Copyright (C) 1998 Bero
- Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http://www.winimage.com/zLibDll/minizip.html )
- Copyright (C) 2001 Aaron Oneal
- Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni
- Copyright (C) 2002 CaH4e3
- Copyright (C) 2002 Paul Kuliniewicz
- Copyright (C) 2002 Xodnizel
- Copyright (C) 2002 Xodnizel 2006 CaH4e3
- Copyright (C) 2002,2003 Xodnizel
- Copyright (C) 2003 Andrea Mazzoleni
- Copyright (C) 2003 CaH4e3
- Copyright (C) 2003 MaxSt ( maxst@hiend3d.com )
- Copyright (C) 2003 Xodnizel
- Copyright (C) 2004 Jason Oster (Parasyte)
- Copyright (C) 2004 Xodnizel
- Copyright (C) 2005 CaH4e3
- Copyright (C) 2005 Sebastian Porst
- Copyright (C) 2005-2019 CaH4e3
- Copyright (C) 2006 CaH4e3
- Copyright (C) 2006 Shay Green. This module is free software
- Copyright (C) 2006-2007 Shay Green. This module is free software
- Copyright (C) 2007 CaH4e3
- Copyright (C) 2007-2008 Even Rouault
- Copyright (C) 2007-2008 Mad Dumper, CaH4e3
- Copyright (C) 2007-2010 CaH4e3
- Copyright (C) 2008 CaH4e3
- Copyright (C) 2009 CaH4e3
- Copyright (C) 2009 qeed
- Copyright (C) 2009-2010 DeSmuME team
- Copyright (C) 2009-2010 Mathias Svensson ( http://result42.com )
- Copyright (C) 2011 CaH4e3
- Copyright (C) 2011 FCEUX team
- Copyright (C) 2012 CaH4e3
- Copyright (C) 2012 FCEUX team
- Copyright (C) 2012-2017 FCEUX team
- Copyright (C) 2013 CaH4e3
- Copyright (C) 2014 CaH4e3
- Copyright (C) 2014 CaitSith2, 2022 Cluster
- Copyright (C) 2015 CaH4e3
- Copyright (C) 2015 Cluster
- Copyright (C) 2016 CaH4e3
- Copyright (C) 2016 Cluster
- Copyright (C) 2017 CaH4e3
- Copyright (C) 2017 FCEUX Team
- Copyright (C) 2018 CaH4e3, Cluster
- Copyright (C) 2019 CaH4e3
- Copyright (C) 2019 Libretro Team
- Copyright (C) 2020
- Copyright (C) 2020 CaH4e3
- Copyright (C) 2020 mjbudd77
- Copyright (C) 2021 mjbudd77
- Copyright (C) 2022
- Copyright (C) 2022 Cluster
- Copyright (C) 2022 thor2016
- Copyright (C) 2026 FCEUX11 Contributors
- you

## 3. Files Without Explicit GPL Header (Require Review)

- `src/apu.cpp`
- `src/apu.h`
- `src/archived/fir/c44100ntsc.h`
- `src/archived/fir/c44100pal.h`
- `src/archived/fir/c48000ntsc.h`
- `src/archived/fir/c48000pal.h`
- `src/archived/fir/c96000ntsc.h`
- `src/archived/fir/c96000pal.h`
- `src/archived/fir/toh.c`
- `src/asm.cpp`
- `src/asm.h`
- `src/boards/_cart_helpers.cpp`
- `src/boards/_cart_helpers.h`
- `src/boards/datalatch_carts.h`
- `src/boards/emu2413.c`
- `src/boards/emu2413.h`
- `src/boards/irem_txc_bit_carts.h`
- `src/boards/legacy_expansion_audio.h`
- `src/boards/mapinc_audio.h`
- `src/boards/mapinc_base.h`
- `src/boards/mapinc_bus.h`
- `src/boards/mapinc_mmc3.h`
- `src/boards/mapinc_state.h`
- `src/boards/mapper_strategy_a.h`
- `src/boards/mmc1_cart.h`
- `src/boards/mmc3.h`
- `src/boards/mmc3_base_cart.cpp`
- `src/boards/mmc3_base_cart.h`
- `src/boards/mmc3_cart.h`
- `src/boards/mmc3_variants_carts.h`
- `src/boards/nrom_cart.h`
- `src/boards/registry.cpp`
- `src/boards/registry.h`
- `src/boards/simple_carts.h`
- `src/boards/smb2j_carts.h`
- `src/boards/vrc2and4_carts.h`
- `src/boards/vrc6_cart.h`
- `src/bus.cpp`
- `src/bus.h`
- `src/cart.h`
- `src/cart_class.cpp`
- `src/cart_class.h`
- `src/cheat.h`
- `src/compiler_attrs.h`
- `src/config.cpp`
- `src/core_state.cpp`
- `src/core_state.h`
- `src/cpu.cpp`
- `src/cpu.h`
- `src/debug.cpp`
- `src/debug.h`
- `src/debugsymboltable.cpp`
- `src/debugsymboltable.h`
- `src/drawing.cpp`
- `src/drawing.h`
- `src/driver_callbacks.cpp`
- `src/driver_callbacks.h`
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
- `src/drivers/Qt/ConfigStore.h`
- `src/drivers/Qt/ConsoleActions.h`
- `src/drivers/Qt/ConsoleCursor.cpp`
- `src/drivers/Qt/ConsoleDebugWindows.h`
- `src/drivers/Qt/ConsoleDebugger.h`
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
- `src/drivers/Qt/FamilyKeyboard.h`
- `src/drivers/Qt/FrameTimingStats.h`
- `src/drivers/Qt/GameGenie.h`
- `src/drivers/Qt/GamePadConf.h`
- `src/drivers/Qt/GuiConf.h`
- `src/drivers/Qt/HelpPages.h`
- `src/drivers/Qt/HexEditor.h`
- `src/drivers/Qt/HotKeyConf.h`
- `src/drivers/Qt/InputConf.h`
- `src/drivers/Qt/LuaControl.h`
- `src/drivers/Qt/MenuCatalog.h`
- `src/drivers/Qt/MovieOptions.h`
- `src/drivers/Qt/MoviePlay.h`
- `src/drivers/Qt/MovieRecord.h`
- `src/drivers/Qt/MsgLogViewer.h`
- `src/drivers/Qt/NameTableViewer.h`
- `src/drivers/Qt/PaletteConf.h`
- `src/drivers/Qt/PaletteEditor.h`
- `src/drivers/Qt/RamSearch.h`
- `src/drivers/Qt/RamWatch.h`
- `src/drivers/Qt/SplashScreen.h`
- `src/drivers/Qt/StateRecorderConf.h`
- `src/drivers/Qt/SymbolicDebug.h`
- `src/drivers/Qt/TasEditor/TasColors.h`
- `src/drivers/Qt/TasEditor/TasEditorContext.h`
- `src/drivers/Qt/TasEditor/TasEditorTimeline.h`
- `src/drivers/Qt/TasEditor/TasEditorWindow.h`
- `src/drivers/Qt/TasEditor/TasFindNoteWindow.cpp`
- `src/drivers/Qt/TasEditor/TasFindNoteWindow.h`
- `src/drivers/Qt/TasEditor/bookmark.cpp`
- `src/drivers/Qt/TasEditor/bookmark.h`
- `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.cpp`
- `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.h`
- `src/drivers/Qt/TasEditor/bookmarks.cpp`
- `src/drivers/Qt/TasEditor/bookmarks.h`
- `src/drivers/Qt/TasEditor/branches.cpp`
- `src/drivers/Qt/TasEditor/branches.h`
- `src/drivers/Qt/TasEditor/greenzone.cpp`
- `src/drivers/Qt/TasEditor/greenzone.h`
- `src/drivers/Qt/TasEditor/history.cpp`
- `src/drivers/Qt/TasEditor/history.h`
- `src/drivers/Qt/TasEditor/inputlog.cpp`
- `src/drivers/Qt/TasEditor/inputlog.h`
- `src/drivers/Qt/TasEditor/laglog.cpp`
- `src/drivers/Qt/TasEditor/laglog.h`
- `src/drivers/Qt/TasEditor/markerDragPopup.cpp`
- `src/drivers/Qt/TasEditor/markerDragPopup.h`
- `src/drivers/Qt/TasEditor/markers.cpp`
- `src/drivers/Qt/TasEditor/markers.h`
- `src/drivers/Qt/TasEditor/markers_manager.cpp`
- `src/drivers/Qt/TasEditor/markers_manager.h`
- `src/drivers/Qt/TasEditor/playback.cpp`
- `src/drivers/Qt/TasEditor/playback.h`
- `src/drivers/Qt/TasEditor/recorder.cpp`
- `src/drivers/Qt/TasEditor/recorder.h`
- `src/drivers/Qt/TasEditor/selection.cpp`
- `src/drivers/Qt/TasEditor/selection.h`
- `src/drivers/Qt/TasEditor/snapshot.cpp`
- `src/drivers/Qt/TasEditor/snapshot.h`
- `src/drivers/Qt/TasEditor/splicer.cpp`
- `src/drivers/Qt/TasEditor/splicer.h`
- `src/drivers/Qt/TasEditor/taseditor_config.cpp`
- `src/drivers/Qt/TasEditor/taseditor_config.h`
- `src/drivers/Qt/TasEditor/taseditor_lua.cpp`
- `src/drivers/Qt/TasEditor/taseditor_lua.h`
- `src/drivers/Qt/TasEditor/taseditor_project.cpp`
- `src/drivers/Qt/TasEditor/taseditor_project.h`
- `src/drivers/Qt/TimingConf.h`
- `src/drivers/Qt/TraceLogger.h`
- `src/drivers/Qt/avi/avi-utils.cpp`
- `src/drivers/Qt/avi/fileio.cpp`
- `src/drivers/Qt/avi/gwavi.cpp`
- `src/drivers/Qt/avi/gwavi.h`
- `src/drivers/Qt/config.h`
- `src/drivers/Qt/dface.h`
- `src/drivers/Qt/fceuWrapper.h`
- `src/drivers/Qt/fceu_callbacks.cpp`
- `src/drivers/Qt/fceux_git_info.h`
- `src/drivers/Qt/iNesHeaderEditor.h`
- `src/drivers/Qt/input.h`
- `src/drivers/Qt/input/input_backend.h`
- `src/drivers/Qt/input/input_device.h`
- `src/drivers/Qt/input/input_manager.cpp`
- `src/drivers/Qt/input/input_manager.h`
- `src/drivers/Qt/input/sdl_backend.cpp`
- `src/drivers/Qt/input/sdl_backend.h`
- `src/drivers/Qt/input/wgi_backend.cpp`
- `src/drivers/Qt/input/wgi_backend.h`
- `src/drivers/Qt/input/xinput_backend.cpp`
- `src/drivers/Qt/input/xinput_backend.h`
- `src/drivers/Qt/keyscan.h`
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
- `src/drivers/Qt/sdl-joystick.h`
- `src/drivers/Qt/sdl-video.h`
- `src/drivers/Qt/sdl.h`
- `src/drivers/Qt/throttle.h`
- `src/drivers/common/args.h`
- `src/drivers/common/config.h`
- `src/drivers/common/configSys.cpp`
- `src/drivers/common/configSys.h`
- `src/drivers/common/hq2x.cpp`
- `src/drivers/common/hq2x.h`
- `src/drivers/common/hq3x.cpp`
- `src/drivers/common/hq3x.h`
- `src/drivers/common/nes_ntsc.c`
- `src/drivers/common/nes_ntsc.h`
- `src/drivers/common/nes_ntsc_config.h`
- `src/drivers/common/nes_ntsc_impl.h`
- `src/drivers/common/nes_shm.h`
- `src/drivers/common/os_utils.cpp`
- `src/drivers/common/os_utils.h`
- `src/drivers/null/null_driver.cpp`
- `src/drivers/null/null_driver.h`
- `src/emufile.cpp`
- `src/emufile.h`
- `src/emufile_types.h`
- `src/expansion_audio.cpp`
- `src/expansion_audio.h`
- `src/f11qa_bridge.cpp`
- `src/f11qa_bridge.h`
- `src/fceu.h`
- `src/fceu11_core_types.h`
- `src/fceulua.h`
- `src/fds.h`
- `src/fds_sound.cpp`
- `src/file.h`
- `src/filter.cpp`
- `src/filter.h`
- `src/gba_load.cpp`
- `src/gba_load.h`
- `src/git.h`
- `src/ines-bad.h`
- `src/ines-correct.h`
- `src/ines.cpp`
- `src/ines_bmap.h`
- `src/ines_gi.cpp`
- `src/ines_init.cpp`
- `src/ines_load.cpp`
- `src/ines_save.cpp`
- `src/input.h`
- `src/input/cursor.cpp`
- `src/input/fkb.h`
- `src/input/share.h`
- `src/input/suborkb.h`
- `src/input/zapper.h`
- `src/lua-engine.cpp`
- `src/movie.cpp`
- `src/movie.h`
- `src/movie_fm2.h`
- `src/movie_io.cpp`
- `src/movie_playback.h`
- `src/movie_record.h`
- `src/movie_settings.cpp`
- `src/movie_subtitles.cpp`
- `src/movie_taseditor_bridge.cpp`
- `src/netplay.h`
- `src/nsf_load.cpp`
- `src/nsf_runtime.cpp`
- `src/nsf_ui.cpp`
- `src/oldmovie.cpp`
- `src/oldmovie.h`
- `src/palette.h`
- `src/palettes/conv.c`
- `src/palettes/palettes.h`
- `src/palettes/rp2c04001.h`
- `src/palettes/rp2c04002.h`
- `src/palettes/rp2c04003.h`
- `src/palettes/rp2c05004.h`
- `src/platform/win11/DirectStorageProbe.cpp`
- `src/platform/win11/DirectStorageProbe.h`
- `src/platform/win11/TaskbarProgress.cpp`
- `src/platform/win11/TaskbarProgress.h`
- `src/ppu.h`
- `src/ppu_class.cpp`
- `src/ppu_class.h`
- `src/ppu_core.h`
- `src/ppu_rendering.h`
- `src/ppu_sprite_lut.cpp`
- `src/ppu_sprite_lut.h`
- `src/ppu_state.h`
- `src/pputile_template.cpp`
- `src/pputile_template.h`
- `src/rust/fceux11_rust.h`
- `src/tests/boards/mapper_load_test.cpp`
- `src/tests/boards/mapper_reset_test.cpp`
- `src/tests/git_info_stub.cpp`
- `src/tests/rom_regression_test.cpp`
- `src/tests/smoke_test.cpp`
- `src/unif.cpp`
- `src/unif_bmap.h`
- `src/unif_load.cpp`
- `src/utils/ConvertUTF.c`
- `src/utils/ConvertUTF.h`
- `src/utils/backward.cpp`
- `src/utils/cache.h`
- `src/utils/crc32.h`
- `src/utils/endian.h`
- `src/utils/enum_class_bitflags.h`
- `src/utils/fceu11_expected.cpp`
- `src/utils/fceu11_expected.h`
- `src/utils/fceu11_format.h`
- `src/utils/format.h`
- `src/utils/general.h`
- `src/utils/guid.cpp`
- `src/utils/guid.h`
- `src/utils/ioapi.cpp`
- `src/utils/ioapi.h`
- `src/utils/md5.cpp`
- `src/utils/md5.h`
- `src/utils/mutex.cpp`
- `src/utils/mutex.h`
- `src/utils/platform_compat.h`
- `src/utils/safe_string.h`
- `src/utils/simd_fill.h`
- `src/utils/timeStamp.cpp`
- `src/utils/timeStamp.h`
- `src/utils/unzip.cpp`
- `src/utils/unzip.h`
- `src/utils/valuearray.h`
- `src/video.h`
- `src/vsuni.cpp`
- `src/vsuni.h`
- `src/wave.cpp`
- `src/wave.h`
- `src/x6502abbrev.h`
- `src/x6502struct.h`

## 4. Full File Listing

| File | License | Copyrights |
|------|---------|-----------|
| `src/apu.cpp` | Unknown/None |  |
| `src/apu.h` | Unknown/None |  |
| `src/archived/fceux-server/md5.cpp` | GPLv2+ |  |
| `src/archived/fceux-server/md5.h` | GPLv2+ |  |
| `src/archived/fceux-server/server.cpp` | GPLv2+ | Copyright (C) 2004 Xodnizel |
| `src/archived/fceux-server/throttle.cpp` | GPLv2+ | Copyright (C) 2004 Xodnizel |
| `src/archived/fceux-server/throttle.h` | GPLv2+ | Copyright (C) 2004 Xodnizel |
| `src/archived/fceux-server/types.h` | GPLv2+ | Copyright (C) 2004 Xodnizel |
| `src/archived/fir/c44100ntsc.h` | Unknown/None |  |
| `src/archived/fir/c44100pal.h` | Unknown/None |  |
| `src/archived/fir/c48000ntsc.h` | Unknown/None |  |
| `src/archived/fir/c48000pal.h` | Unknown/None |  |
| `src/archived/fir/c96000ntsc.h` | Unknown/None |  |
| `src/archived/fir/c96000pal.h` | Unknown/None |  |
| `src/archived/fir/toh.c` | Unknown/None |  |
| `src/asm.cpp` | Unknown/None |  |
| `src/asm.h` | Unknown/None |  |
| `src/boards/01-222.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/09-034a.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/103.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/106.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/108.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/112.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/116.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/117.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/120.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/121.cpp` | GPLv2+ | Copyright (C) 2007-2008 Mad Dumper, CaH4e3 |
| `src/boards/12in1.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/15.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/151.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/156.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/158B.cpp` | GPLv2+ | Copyright (C) 2015 CaH4e3 |
| `src/boards/164.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel 2006 CaH4e3 |
| `src/boards/168.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/170.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/175.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/176.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3; Copyright (C) 2012 FCEUX team |
| `src/boards/177.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/178.cpp` | GPLv2+ | Copyright (C) 2013 CaH4e3 |
| `src/boards/18.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/183.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/185.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/186.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/187.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/189.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/190.cpp` | GPLv2+ | Copyright (C) 2017 FCEUX Team |
| `src/boards/193.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/199.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/206.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/208.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/222.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/225.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3; Copyright (C) 2019 Libretro Team; Copyright (C) 2020 |
| `src/boards/228.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/230.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3; Copyright (C) 2009 qeed |
| `src/boards/232.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/234.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/235.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/244.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/246.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/252.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/253.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/28.cpp` | GPLv2+ | Copyright (C) 2012-2017 FCEUX team |
| `src/boards/32.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/33.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/34.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/354.cpp` | GPLv2+ | Copyright (C) 2022 |
| `src/boards/36.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/3d-block.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/40.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/41.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/411120-c.cpp` | GPLv2+ | Copyright (C) 2008 CaH4e3 |
| `src/boards/42.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/43.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/46.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/50.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/51.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/57.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/603-5052.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/62.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/65.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/67.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/68.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/69.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/71.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/72.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/77.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/79.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/80.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/80013-B.cpp` | GPLv2+ | Copyright (C) 2017 CaH4e3 |
| `src/boards/8157.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/82.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/8237.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/830118C.cpp` | GPLv2+ | Copyright (C) 2008 CaH4e3 |
| `src/boards/88.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/8in1.cpp` | GPLv2+ | Copyright (C) 2016 CaH4e3 |
| `src/boards/90.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel; Copyright (C) 2005 CaH4e3 |
| `src/boards/91.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/96.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2002 Xodnizel; Copyright (C) 2012 CaH4e3 |
| `src/boards/99.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/BMW8544.cpp` | GPLv2+ | Copyright (C) 2015 CaH4e3 |
| `src/boards/F-15.cpp` | GPLv2+ | Copyright (C) 2015 CaH4e3 |
| `src/boards/__dummy_mapper.cpp` | GPLv2+ | Copyright (C) 2013 CaH4e3 |
| `src/boards/_cart_helpers.cpp` | Unknown/None |  |
| `src/boards/_cart_helpers.h` | Unknown/None |  |
| `src/boards/a9746.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/ac-08.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/addrlatch.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/ax5705.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/bandai.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3; Copyright (C) 2011 FCEUX team |
| `src/boards/bb.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/bmc13in1jy110.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/bmc42in1r.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3; Copyright (C) 2009 qeed |
| `src/boards/bmc64in1nr.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/bmc70in1.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/bonza.cpp` | GPLv2+ | Copyright (C) 2002 CaH4e3 |
| `src/boards/bs-5.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/bs4xxxr.cpp` | GPLv2+ | Copyright (C) 2020 CaH4e3 |
| `src/boards/cheapocabra.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/cityfighter.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/coolboy.cpp` | GPLv2+ | Copyright (C) 2018 CaH4e3, Cluster |
| `src/boards/coolgirl.cpp` | GPLv2+ | Copyright (C) 2022 Cluster |
| `src/boards/dance2000.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/datalatch.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/datalatch_carts.h` | Unknown/None |  |
| `src/boards/dream.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/edu2000.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/eh8813a.cpp` | GPLv2+ | Copyright (C) 2015 CaH4e3 |
| `src/boards/emu2413.c` | Unknown/None |  |
| `src/boards/emu2413.h` | Unknown/None |  |
| `src/boards/et-100.cpp` | GPLv2+ | Copyright (C) 2015 Cluster |
| `src/boards/et-4320.cpp` | GPLv2+ | Copyright (C) 2016 Cluster |
| `src/boards/famicombox.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/ffe.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/fk23c.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/fns.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2002 Xodnizel; Copyright (C) 2020 CaH4e3 |
| `src/boards/ghostbusters63in1.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/gs-2004.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/gs-2013.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/h2288.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/hp10xx_hp20xx.cpp` | GPLv2+ | Copyright (C) 2017 CaH4e3 |
| `src/boards/hp898f.cpp` | GPLv2+ | Copyright (C) 2015 CaH4e3 |
| `src/boards/inlnsf.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/inx007t.cpp` | GPLv2+ | Copyright (C) 2022 Cluster |
| `src/boards/irem_txc_bit_carts.h` | Unknown/None |  |
| `src/boards/karaoke.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/kof97.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/ks7010.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/ks7012.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/ks7013.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/ks7016.cpp` | GPLv2+ | Copyright (C) 2016 CaH4e3 |
| `src/boards/ks7017.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/ks7030.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/ks7031.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/ks7032.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/ks7037.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/ks7057.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/le05.cpp` | GPLv2+ | Copyright (C) 2011 CaH4e3 |
| `src/boards/legacy_expansion_audio.h` | Unknown/None |  |
| `src/boards/lh32.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/lh53.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/malee.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/mapinc_audio.h` | Unknown/None |  |
| `src/boards/mapinc_base.h` | Unknown/None |  |
| `src/boards/mapinc_bus.h` | Unknown/None |  |
| `src/boards/mapinc_mmc3.h` | Unknown/None |  |
| `src/boards/mapinc_state.h` | Unknown/None |  |
| `src/boards/mapper_strategy_a.h` | Unknown/None |  |
| `src/boards/mihunche.cpp` | GPLv2+ | Copyright (C) 2013 CaH4e3 |
| `src/boards/mmc1.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2002 Xodnizel |
| `src/boards/mmc1_cart.h` | Unknown/None |  |
| `src/boards/mmc2and4.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3; Copyright (C) 2002 Xodnizel |
| `src/boards/mmc3.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel; Copyright (C) 2003 CaH4e3 |
| `src/boards/mmc3.h` | Unknown/None |  |
| `src/boards/mmc3_base_cart.cpp` | Unknown/None |  |
| `src/boards/mmc3_base_cart.h` | Unknown/None |  |
| `src/boards/mmc3_cart.h` | Unknown/None |  |
| `src/boards/mmc3_variants_carts.h` | Unknown/None |  |
| `src/boards/mmc5.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/n106.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/n625092.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/novel.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/nrom_cart.h` | Unknown/None |  |
| `src/boards/onebus.cpp` | GPLv2+ | Copyright (C) 2007-2010 CaH4e3 |
| `src/boards/pec-586.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/registry.cpp` | Unknown/None |  |
| `src/boards/registry.h` | Unknown/None |  |
| `src/boards/rt-01.cpp` | GPLv2+ | Copyright (C) 2016 CaH4e3 |
| `src/boards/sa-9602b.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/sachen.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/sb-2000.cpp` | GPLv2+ | Copyright (C) 2014 CaH4e3 |
| `src/boards/sc-127.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/sheroes.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/simple_carts.h` | Unknown/None |  |
| `src/boards/sl1632.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/smb2j_carts.h` | Unknown/None |  |
| `src/boards/subor.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/super24.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/supervision.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/t-227-1.cpp` | GPLv2+ | Copyright (C) 2008 CaH4e3 |
| `src/boards/t-262.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/boards/tengen.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/boards/tf-1201.cpp` | GPLv2+ | Copyright (C) 2005 CaH4e3 |
| `src/boards/transformer.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/unrom512.cpp` | GPLv2+ | Copyright (C) 2014 CaitSith2, 2022 Cluster |
| `src/boards/vrc1.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/vrc2and4.cpp` | GPLv2+ | Copyright (C) 2007 CaH4e3 |
| `src/boards/vrc2and4_carts.h` | Unknown/None |  |
| `src/boards/vrc3.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/vrc5.cpp` | GPLv2+ | Copyright (C) 2005-2019 CaH4e3 |
| `src/boards/vrc6.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/vrc6_cart.h` | Unknown/None |  |
| `src/boards/vrc7.cpp` | GPLv2+ | Copyright (C) 2012 CaH4e3 |
| `src/boards/vrc7p.cpp` | GPLv2+ | Copyright (C) 2009 CaH4e3 |
| `src/boards/yoko.cpp` | GPLv2+ | Copyright (C) 2006 CaH4e3 |
| `src/bus.cpp` | Unknown/None |  |
| `src/bus.h` | Unknown/None |  |
| `src/cart.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/cart.h` | Unknown/None |  |
| `src/cart_class.cpp` | Unknown/None |  |
| `src/cart_class.h` | Unknown/None |  |
| `src/cheat.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/cheat.h` | Unknown/None |  |
| `src/compiler_attrs.h` | Unknown/None |  |
| `src/conddebug.cpp` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| `src/conddebug.h` | GPLv2+ | Copyright (C) 2005 Sebastian Porst |
| `src/config.cpp` | Unknown/None |  |
| `src/core_api.h` | GPLv2+ | Copyright (C) 2001 Aaron Oneal; Copyright (C) 2002 Xodnizel |
| `src/core_state.cpp` | Unknown/None |  |
| `src/core_state.h` | Unknown/None |  |
| `src/cpu.cpp` | Unknown/None |  |
| `src/cpu.h` | Unknown/None |  |
| `src/debug.cpp` | Unknown/None |  |
| `src/debug.h` | Unknown/None |  |
| `src/debugsymboltable.cpp` | Unknown/None |  |
| `src/debugsymboltable.h` | Unknown/None |  |
| `src/diag_api.h` | GPLv2+ | Copyright (C) 2001 Aaron Oneal; Copyright (C) 2002 Xodnizel |
| `src/drawing.cpp` | Unknown/None |  |
| `src/drawing.h` | Unknown/None |  |
| `src/driver.h` | GPLv2+ | Copyright (C) 2001 Aaron Oneal; Copyright (C) 2002 Xodnizel |
| `src/driver_callbacks.cpp` | Unknown/None |  |
| `src/driver_callbacks.h` | Unknown/None |  |
| `src/drivers/Qt/AboutWindow.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/AboutWindow.h` | Unknown/None |  |
| `src/drivers/Qt/AviAudioCodec.cpp` | Unknown/None |  |
| `src/drivers/Qt/AviAudioCodec.h` | Unknown/None |  |
| `src/drivers/Qt/AviOptionsDialog.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/AviOptionsDialog.h` | Unknown/None |  |
| `src/drivers/Qt/AviRecord.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/AviRecord.h` | Unknown/None |  |
| `src/drivers/Qt/AviRecordContext.h` | Unknown/None |  |
| `src/drivers/Qt/AviRecordDiskThread.cpp` | Unknown/None |  |
| `src/drivers/Qt/AviRiffViewer.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/AviRiffViewer.h` | Unknown/None |  |
| `src/drivers/Qt/AviVideoCodec.cpp` | Unknown/None |  |
| `src/drivers/Qt/AviVideoCodec.h` | Unknown/None |  |
| `src/drivers/Qt/CheatsConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/CheatsConf.h` | Unknown/None |  |
| `src/drivers/Qt/CodeDataLogger.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/CodeDataLogger.h` | Unknown/None |  |
| `src/drivers/Qt/ColorMenu.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ColorMenu.h` | Unknown/None |  |
| `src/drivers/Qt/ConfigStore.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleActions.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleActions.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleCursor.cpp` | Unknown/None |  |
| `src/drivers/Qt/ConsoleDebugWindows.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleDebugWindows.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleDebugger.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleDebugger.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleEmuControl.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleEmuControl.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleEmulatorThread.cpp` | Unknown/None |  |
| `src/drivers/Qt/ConsoleFile.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleFile.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleHotKeys.cpp` | Unknown/None |  |
| `src/drivers/Qt/ConsoleMenu.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleMenu.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleMenuBar.cpp` | Unknown/None |  |
| `src/drivers/Qt/ConsoleRecentRom.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleRecentRom.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleRecording.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleRecording.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleSoundConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleSoundConf.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleTranslation.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleTranslation.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleUtilities.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleUtilities.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleVideo.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleVideo.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleVideoConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleVideoConf.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleVideoSetup.cpp` | Unknown/None |  |
| `src/drivers/Qt/ConsoleViewerGL.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleViewerGL.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleViewerInterface.cpp` | Unknown/None |  |
| `src/drivers/Qt/ConsoleViewerInterface.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleViewerQWidget.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleViewerQWidget.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleViewerSDL.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleViewerSDL.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleWindow.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ConsoleWindow.h` | Unknown/None |  |
| `src/drivers/Qt/ConsoleWindowContext.h` | Unknown/None |  |
| `src/drivers/Qt/FamilyKeyboard.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/FamilyKeyboard.h` | Unknown/None |  |
| `src/drivers/Qt/FrameTimingStats.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/FrameTimingStats.h` | Unknown/None |  |
| `src/drivers/Qt/GameGenie.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/GameGenie.h` | Unknown/None |  |
| `src/drivers/Qt/GamePadConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/GamePadConf.h` | Unknown/None |  |
| `src/drivers/Qt/GuiConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/GuiConf.h` | Unknown/None |  |
| `src/drivers/Qt/HelpPages.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/HelpPages.h` | Unknown/None |  |
| `src/drivers/Qt/HexEditor.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/HexEditor.h` | Unknown/None |  |
| `src/drivers/Qt/HotKeyConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/HotKeyConf.h` | Unknown/None |  |
| `src/drivers/Qt/InputConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/InputConf.h` | Unknown/None |  |
| `src/drivers/Qt/LuaControl.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/LuaControl.h` | Unknown/None |  |
| `src/drivers/Qt/MenuCatalog.h` | Unknown/None |  |
| `src/drivers/Qt/MovieOptions.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/MovieOptions.h` | Unknown/None |  |
| `src/drivers/Qt/MoviePlay.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/MoviePlay.h` | Unknown/None |  |
| `src/drivers/Qt/MovieRecord.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/MovieRecord.h` | Unknown/None |  |
| `src/drivers/Qt/MsgLogViewer.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/MsgLogViewer.h` | Unknown/None |  |
| `src/drivers/Qt/NameTableViewer.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/NameTableViewer.h` | Unknown/None |  |
| `src/drivers/Qt/PaletteConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/PaletteConf.h` | Unknown/None |  |
| `src/drivers/Qt/PaletteEditor.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/PaletteEditor.h` | Unknown/None |  |
| `src/drivers/Qt/QtNetplay.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/Qt/RamSearch.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/RamSearch.h` | Unknown/None |  |
| `src/drivers/Qt/RamWatch.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/RamWatch.h` | Unknown/None |  |
| `src/drivers/Qt/SplashScreen.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/SplashScreen.h` | Unknown/None |  |
| `src/drivers/Qt/StateRecorderConf.cpp` | GPLv2+ | Copyright (C) 2022 thor2016 |
| `src/drivers/Qt/StateRecorderConf.h` | Unknown/None |  |
| `src/drivers/Qt/SymbolicDebug.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/SymbolicDebug.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/TasColors.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/TasEditorContext.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/TasEditorTimeline.cpp` | GPLv2+ | Copyright (C) 2021 mjbudd77 |
| `src/drivers/Qt/TasEditor/TasEditorTimeline.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/TasEditorWindow.cpp` | GPLv2+ | Copyright (C) 2021 mjbudd77 |
| `src/drivers/Qt/TasEditor/TasEditorWindow.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/TasFindNoteWindow.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/TasFindNoteWindow.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/bookmark.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/bookmark.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/bookmarkPreviewPopup.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/bookmarks.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/bookmarks.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/branches.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/branches.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/greenzone.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/greenzone.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/history.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/history.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/inputlog.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/inputlog.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/laglog.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/laglog.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/markerDragPopup.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/markerDragPopup.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/markers.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/markers.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/markers_manager.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/markers_manager.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/playback.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/playback.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/recorder.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/recorder.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/selection.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/selection.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/snapshot.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/snapshot.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/splicer.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/splicer.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/taseditor_config.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/taseditor_config.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/taseditor_lua.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/taseditor_lua.h` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/taseditor_project.cpp` | Unknown/None |  |
| `src/drivers/Qt/TasEditor/taseditor_project.h` | Unknown/None |  |
| `src/drivers/Qt/TimingConf.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/TimingConf.h` | Unknown/None |  |
| `src/drivers/Qt/TraceLogger.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/TraceLogger.h` | Unknown/None |  |
| `src/drivers/Qt/avi/avi-utils.cpp` | Unknown/None |  |
| `src/drivers/Qt/avi/fileio.cpp` | Unknown/None |  |
| `src/drivers/Qt/avi/gwavi.cpp` | Unknown/None |  |
| `src/drivers/Qt/avi/gwavi.h` | Unknown/None |  |
| `src/drivers/Qt/config.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/config.h` | Unknown/None |  |
| `src/drivers/Qt/dface.h` | Unknown/None |  |
| `src/drivers/Qt/fceuWrapper.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/fceuWrapper.h` | Unknown/None |  |
| `src/drivers/Qt/fceu_archive.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/Qt/fceu_callbacks.cpp` | Unknown/None |  |
| `src/drivers/Qt/fceu_globals.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/Qt/fceux_git_info.h` | Unknown/None |  |
| `src/drivers/Qt/iNesHeaderEditor.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/iNesHeaderEditor.h` | Unknown/None |  |
| `src/drivers/Qt/input.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/Qt/input.h` | Unknown/None |  |
| `src/drivers/Qt/input/input_backend.h` | Unknown/None |  |
| `src/drivers/Qt/input/input_device.h` | Unknown/None |  |
| `src/drivers/Qt/input/input_manager.cpp` | Unknown/None |  |
| `src/drivers/Qt/input/input_manager.h` | Unknown/None |  |
| `src/drivers/Qt/input/sdl_backend.cpp` | Unknown/None |  |
| `src/drivers/Qt/input/sdl_backend.h` | Unknown/None |  |
| `src/drivers/Qt/input/wgi_backend.cpp` | Unknown/None |  |
| `src/drivers/Qt/input/wgi_backend.h` | Unknown/None |  |
| `src/drivers/Qt/input/xinput_backend.cpp` | Unknown/None |  |
| `src/drivers/Qt/input/xinput_backend.h` | Unknown/None |  |
| `src/drivers/Qt/keyscan.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/keyscan.h` | Unknown/None |  |
| `src/drivers/Qt/main.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/main.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/Qt/ppuViewer.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/ppuViewer.h` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerContext.cpp` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerContext.h` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerPalette.cpp` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerPalette.h` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerPatternTables.cpp` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerPatternTables.h` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerSpriteViewer.cpp` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerSpriteViewer.h` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerTileEditor.cpp` | Unknown/None |  |
| `src/drivers/Qt/ppuViewerTileEditor.h` | Unknown/None |  |
| `src/drivers/Qt/sdl-joystick.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel; Copyright (C) 2002 Paul Kuliniewicz |
| `src/drivers/Qt/sdl-joystick.h` | Unknown/None |  |
| `src/drivers/Qt/sdl-sound.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/Qt/sdl-throttle.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/Qt/sdl-video.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/Qt/sdl-video.h` | Unknown/None |  |
| `src/drivers/Qt/sdl.h` | Unknown/None |  |
| `src/drivers/Qt/throttle.h` | Unknown/None |  |
| `src/drivers/common/args.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/common/args.h` | Unknown/None |  |
| `src/drivers/common/cheat.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/common/cheat.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/common/config.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/common/config.h` | Unknown/None |  |
| `src/drivers/common/configSys.cpp` | Unknown/None |  |
| `src/drivers/common/configSys.h` | Unknown/None |  |
| `src/drivers/common/hq2x.cpp` | Unknown/None | Copyright (C) 2003 MaxSt ( maxst@hiend3d.com ) |
| `src/drivers/common/hq2x.h` | Unknown/None |  |
| `src/drivers/common/hq3x.cpp` | Unknown/None | Copyright (C) 2003 MaxSt ( maxst@hiend3d.com ) |
| `src/drivers/common/hq3x.h` | Unknown/None |  |
| `src/drivers/common/nes_ntsc.c` | Unknown/None | Copyright (C) 2006-2007 Shay Green. This module is free software; you |
| `src/drivers/common/nes_ntsc.h` | Unknown/None |  |
| `src/drivers/common/nes_ntsc_config.h` | Unknown/None |  |
| `src/drivers/common/nes_ntsc_impl.h` | Unknown/None | Copyright (C) 2006 Shay Green. This module is free software; you |
| `src/drivers/common/nes_shm.cpp` | GPLv2+ | Copyright (C) 2020 mjbudd77 |
| `src/drivers/common/nes_shm.h` | Unknown/None |  |
| `src/drivers/common/os_utils.cpp` | Unknown/None |  |
| `src/drivers/common/os_utils.h` | Unknown/None |  |
| `src/drivers/common/scale2x.cpp` | GPLv2+ | Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| `src/drivers/common/scale2x.h` | GPLv2+ | Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| `src/drivers/common/scale3x.cpp` | GPLv2+ | Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| `src/drivers/common/scale3x.h` | GPLv2+ | Copyright (C) 2001, 2002, 2003, 2004 Andrea Mazzoleni |
| `src/drivers/common/scalebit.cpp` | GPLv2+ | Copyright (C) 2003 Andrea Mazzoleni |
| `src/drivers/common/scalebit.h` | GPLv2+ | Copyright (C) 2003 Andrea Mazzoleni |
| `src/drivers/common/vidblit.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/common/vidblit.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/drivers/null/null_driver.cpp` | Unknown/None |  |
| `src/drivers/null/null_driver.h` | Unknown/None |  |
| `src/emufile.cpp` | Unknown/None |  |
| `src/emufile.h` | Unknown/None | Copyright (C) 2009-2010 DeSmuME team |
| `src/emufile_types.h` | Unknown/None |  |
| `src/expansion_audio.cpp` | Unknown/None |  |
| `src/expansion_audio.h` | Unknown/None |  |
| `src/f11qa_bridge.cpp` | Unknown/None |  |
| `src/f11qa_bridge.h` | Unknown/None |  |
| `src/fceu.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/fceu.h` | Unknown/None |  |
| `src/fceu11_core_types.h` | Unknown/None |  |
| `src/fceulua.h` | Unknown/None |  |
| `src/fds.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/fds.h` | Unknown/None |  |
| `src/fds_sound.cpp` | Unknown/None |  |
| `src/file.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/file.h` | Unknown/None |  |
| `src/filter.cpp` | Unknown/None |  |
| `src/filter.h` | Unknown/None |  |
| `src/gba_load.cpp` | Unknown/None |  |
| `src/gba_load.h` | Unknown/None |  |
| `src/git.h` | Unknown/None |  |
| `src/ines-bad.h` | Unknown/None |  |
| `src/ines-correct.h` | Unknown/None |  |
| `src/ines.cpp` | Unknown/None |  |
| `src/ines.h` | GPLv2+ | Copyright (C) 1998 Bero; Copyright (C) 2002 Xodnizel |
| `src/ines_bmap.h` | Unknown/None |  |
| `src/ines_gi.cpp` | Unknown/None |  |
| `src/ines_init.cpp` | Unknown/None |  |
| `src/ines_load.cpp` | Unknown/None |  |
| `src/ines_save.cpp` | Unknown/None |  |
| `src/input.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2002 Xodnizel |
| `src/input.h` | Unknown/None |  |
| `src/input/arkanoid.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/bworld.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/input/cursor.cpp` | Unknown/None |  |
| `src/input/fkb.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/fkb.h` | Unknown/None |  |
| `src/input/fns.cpp` | GPLv2+ | Copyright (C) 2019 CaH4e3 |
| `src/input/ftrainer.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/input/hypershot.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/input/lcdcompzapper.cpp` | GPLv2+ |  |
| `src/input/mahjong.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/input/mouse.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/oekakids.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/input/pec586kb.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/powerpad.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/quiz.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/input/shadow.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/share.h` | Unknown/None |  |
| `src/input/snesmouse.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/suborkb.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/suborkb.h` | Unknown/None |  |
| `src/input/toprider.cpp` | GPLv2+ | Copyright (C) 2003 Xodnizel |
| `src/input/virtualboy.cpp` | GPLv2+ |  |
| `src/input/zapper.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/input/zapper.h` | Unknown/None |  |
| `src/io_api.h` | GPLv2+ | Copyright (C) 2001 Aaron Oneal; Copyright (C) 2002 Xodnizel |
| `src/lua-engine.cpp` | Unknown/None |  |
| `src/movie.cpp` | Unknown/None |  |
| `src/movie.h` | Unknown/None |  |
| `src/movie_fm2.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel |
| `src/movie_fm2.h` | Unknown/None |  |
| `src/movie_io.cpp` | Unknown/None |  |
| `src/movie_playback.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel |
| `src/movie_playback.h` | Unknown/None |  |
| `src/movie_record.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel |
| `src/movie_record.h` | Unknown/None |  |
| `src/movie_settings.cpp` | Unknown/None |  |
| `src/movie_subtitles.cpp` | Unknown/None |  |
| `src/movie_taseditor_bridge.cpp` | Unknown/None |  |
| `src/net_api.h` | GPLv2+ | Copyright (C) 2001 Aaron Oneal; Copyright (C) 2002 Xodnizel |
| `src/netplay.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/netplay.h` | Unknown/None |  |
| `src/nsf.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/nsf.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/nsf_load.cpp` | Unknown/None |  |
| `src/nsf_runtime.cpp` | Unknown/None |  |
| `src/nsf_ui.cpp` | Unknown/None |  |
| `src/oldmovie.cpp` | Unknown/None |  |
| `src/oldmovie.h` | Unknown/None |  |
| `src/palette.cpp` | GPLv2+ | Copyright (C) 2002,2003 Xodnizel |
| `src/palette.h` | Unknown/None |  |
| `src/palettes/conv.c` | Unknown/None |  |
| `src/palettes/palettes.h` | Unknown/None |  |
| `src/palettes/rp2c04001.h` | Unknown/None |  |
| `src/palettes/rp2c04002.h` | Unknown/None |  |
| `src/palettes/rp2c04003.h` | Unknown/None |  |
| `src/palettes/rp2c05004.h` | Unknown/None |  |
| `src/platform/win11/DirectStorageProbe.cpp` | Unknown/None |  |
| `src/platform/win11/DirectStorageProbe.h` | Unknown/None |  |
| `src/platform/win11/TaskbarProgress.cpp` | Unknown/None |  |
| `src/platform/win11/TaskbarProgress.h` | Unknown/None |  |
| `src/ppu.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel |
| `src/ppu.h` | Unknown/None |  |
| `src/ppu_class.cpp` | Unknown/None |  |
| `src/ppu_class.h` | Unknown/None |  |
| `src/ppu_core.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel |
| `src/ppu_core.h` | Unknown/None |  |
| `src/ppu_rendering.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel |
| `src/ppu_rendering.h` | Unknown/None |  |
| `src/ppu_sprite_lut.cpp` | Unknown/None |  |
| `src/ppu_sprite_lut.h` | Unknown/None |  |
| `src/ppu_state.cpp` | GPLv2+ | Copyright (C) 1998 BERO; Copyright (C) 2003 Xodnizel |
| `src/ppu_state.h` | Unknown/None |  |
| `src/pputile_template.cpp` | Unknown/None |  |
| `src/pputile_template.h` | Unknown/None |  |
| `src/profiler.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/profiler.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/rust/fceux11_rust.h` | Unknown/None |  |
| `src/sound.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/sound.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/state.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/state.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/tests/boards/mapper_load_test.cpp` | Unknown/None |  |
| `src/tests/boards/mapper_reset_test.cpp` | Unknown/None |  |
| `src/tests/git_info_stub.cpp` | Unknown/None |  |
| `src/tests/rom_regression_test.cpp` | Unknown/None |  |
| `src/tests/smoke_test.cpp` | Unknown/None |  |
| `src/types.h` | GPLv2+ | Copyright (C) 2001 Aaron Oneal; Copyright (C) 2002 Xodnizel |
| `src/unif.cpp` | Unknown/None |  |
| `src/unif.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/unif_bmap.h` | Unknown/None |  |
| `src/unif_load.cpp` | Unknown/None |  |
| `src/utils/ConvertUTF.c` | Unknown/None |  |
| `src/utils/ConvertUTF.h` | Unknown/None |  |
| `src/utils/backward.cpp` | Unknown/None |  |
| `src/utils/cache.h` | Unknown/None |  |
| `src/utils/crc32.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/utils/crc32.h` | Unknown/None |  |
| `src/utils/endian.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/utils/endian.h` | Unknown/None |  |
| `src/utils/enum_class_bitflags.h` | Unknown/None |  |
| `src/utils/fceu11_expected.cpp` | Unknown/None |  |
| `src/utils/fceu11_expected.h` | Unknown/None |  |
| `src/utils/fceu11_format.h` | Unknown/None |  |
| `src/utils/format.h` | Unknown/None |  |
| `src/utils/general.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/utils/general.h` | Unknown/None |  |
| `src/utils/guid.cpp` | Unknown/None |  |
| `src/utils/guid.h` | Unknown/None |  |
| `src/utils/ioapi.cpp` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http://www.winimage.com/zLibDll/minizip.html )... |
| `src/utils/ioapi.h` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http://www.winimage.com/zLibDll/minizip.html )... |
| `src/utils/md5.cpp` | Unknown/None |  |
| `src/utils/md5.h` | Unknown/None |  |
| `src/utils/memory.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/utils/memory.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/utils/mutex.cpp` | Unknown/None |  |
| `src/utils/mutex.h` | Unknown/None |  |
| `src/utils/platform_compat.h` | Unknown/None |  |
| `src/utils/safe_string.h` | Unknown/None |  |
| `src/utils/simd_fill.h` | Unknown/None |  |
| `src/utils/timeStamp.cpp` | Unknown/None |  |
| `src/utils/timeStamp.h` | Unknown/None |  |
| `src/utils/unzip.cpp` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http://www.winimage.com/zLibDll/minizip.html )... |
| `src/utils/unzip.h` | Unknown/None | Copyright (C) 1998-2010 Gilles Vollant (minizip) ( http://www.winimage.com/zLibDll/minizip.html )... |
| `src/utils/valuearray.h` | Unknown/None |  |
| `src/utils/xstring.cpp` | GPLv2+ | Copyright (C) 2004 Jason Oster (Parasyte) |
| `src/utils/xstring.h` | GPLv2+ | Copyright (C) 2004 Jason Oster (Parasyte) |
| `src/version.h` | GPLv2+ | Copyright (C) 2001 Aaron Oneal; Copyright (C) 2002 Xodnizel; Copyright (C) 2026 FCEUX11 Contributors |
| `src/video.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/video.h` | Unknown/None |  |
| `src/vsuni.cpp` | Unknown/None |  |
| `src/vsuni.h` | Unknown/None |  |
| `src/wave.cpp` | Unknown/None |  |
| `src/wave.h` | Unknown/None |  |
| `src/x6502.cpp` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/x6502.h` | GPLv2+ | Copyright (C) 2002 Xodnizel |
| `src/x6502abbrev.h` | Unknown/None |  |
| `src/x6502struct.h` | Unknown/None |  |

## 5. Rust Vendored Sources (Outside §1–§4 Scope)

Sections 1–4 scan the `src/` C/C++ sources only. FCEUX11 v2.0 vendors one
third-party Rust crate outside that scope, so it does not appear in the counts
above; its provenance is recorded in its own authoritative file, which the v2.0
build plan (section 11 item 3) points at instead.

| Vendored crate | Upstream | Licence | Baseline commit | Attribution record |
|---|---|---|---|---|
| `src/rust/crates/gba-core/` (GBAEUX11 v2.0 S0, vendored 2026-09-28) | [clementine](https://github.com/RIP-Comm/clementine) — `emu/` hardware core only | MIT | `ee77922dd293b70e945458e104f3b2de794f0151` | [`src/rust/crates/gba-core/ATTRIBUTION.md`](../src/rust/crates/gba-core/ATTRIBUTION.md) · verbatim `LICENSE` copy · local patch set (**27** live edits across **5** source files — §3.2: 13 in `cpu/arm7tdmi.rs` incl. the SWI hook seam and the halt guard that alters `step()` (risk R15); 4 in `bus.rs` incl. S-cycle billing, prefetch-hit criterion and DMA unit-drain timing; 3 in `cpu/hardware/timers.rs`; 3 in `cpu/hardware/rtc.rs`; 4 in `cpu/hardware/internal_memory.rs` — two further reverted edits keep their numbers but are not counted) listed in that file, which is the authoritative per-change record |

Local modifications are the project's own work and carry FCEUX11's licence;
the upstream terms continue to apply to the vendored files they are applied to.

