
set( APP_NAME fceux11 )
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

if (${PUBLIC_RELEASE})
	add_definitions( -DPUBLIC_RELEASE=1 )
endif()

message( STATUS "GUI Frontend: Qt6")
set( Qt Qt6 )

if ( ${QHELP} )
	set(QtHelpModule Help)
	add_definitions( -D_USE_QHELP )
endif()

if ( ${FCEU_PROFILER_ENABLE} )
	message( STATUS "FCEU Profiler Enabled")
	add_definitions( -D__FCEU_PROFILER_ENABLE__ )
endif()

find_package( Qt6 REQUIRED COMPONENTS Widgets OpenGL OpenGLWidgets ${QtHelpModule})

# v0.2.1: vcpkg-managed dependencies (MSVC toolchain only)
find_package(SDL2 CONFIG REQUIRED)
find_package(LibArchive REQUIRED)
find_package(ZLIB REQUIRED)
find_package(OpenGL REQUIRED)

# v0.3.6.6: Refactor the v0.3.6.5 sanitizer-only link bypass into a helper
# function so the same code path runs in any non-Release build (Debug,
# RelWithDebInfo) and during sanitizer builds. The root cause is that the
# vcpkg LibArchive port's CMake target only registers IMPORTED_LOCATION
# (no per-config suffix), and CMake's MAP_IMPORTED_CONFIG fallback does
# NOT engage in this scenario for some target types — generator-time fails
# with "IMPORTED_LOCATION not set for ...LibArchive... configuration X".
# OpenGL::GL and ZLIB::ZLIB have the same per-config IMPORTED_LOCATION
# issue. Bypass the imported target machinery in any non-Release build
# (and during sanitizer builds) and link by raw library file.
#
# fceux11_resolve_linked_lib(<out_var> <pkg> <fallback_target> <raw_libraries_var>)
#   - On Release (or sanitizer build on Release): use imported target
#   - On Debug / RelWithDebInfo: use raw ${pkg}_LIBRARIES to avoid the
#     IMPORTED_LOCATION_<CONFIG> lookup failure
function(fceux11_resolve_linked_lib out_var pkg fallback_target raw_libraries_var)
    if(CMAKE_BUILD_TYPE STREQUAL "Release" AND NOT (FCEUX11_ASAN OR FCEUX11_UBSAN))
        set(${out_var} ${fallback_target} PARENT_SCOPE)
    else()
        if(DEFINED ${raw_libraries_var})
            set(${out_var} "${${raw_libraries_var}}" PARENT_SCOPE)
        else()
            set(${out_var} ${fallback_target} PARENT_SCOPE)
        endif()
    endif()
endfunction()

if(FCEUX11_ASAN OR FCEUX11_UBSAN)
    message(STATUS "FCEUX11 v0.3.6.6: sanitizer build selects raw-library link path")
endif()

if(LibArchive_INCLUDE_DIRS)
    include_directories(${LibArchive_INCLUDE_DIRS})
endif()
fceux11_resolve_linked_lib(FCEUX11_LIBARCHIVE_LINK LibArchive LibArchive::LibArchive LibArchive_LIBRARIES)
fceux11_resolve_linked_lib(FCEUX11_OPENGL_LINK     OpenGL     OpenGL::GL             OPENGL_LIBRARIES)
fceux11_resolve_linked_lib(FCEUX11_ZLIB_LINK       ZLIB       ZLIB::ZLIB             ZLIB_LIBRARIES)

# For non-system libraries (LibArchive, ZLIB), the raw var may contain a
# full path. For OpenGL, prefer opengl32 (Windows SDK lib, config-independent).
if(CMAKE_BUILD_TYPE STREQUAL "Debug" OR CMAKE_BUILD_TYPE STREQUAL "RelWithDebInfo")
    set(FCEUX11_OPENGL_LINK opengl32)
    if(FCEUX11_ASAN OR FCEUX11_UBSAN)
        message(STATUS "FCEUX11 v0.3.6.6: non-Release build uses raw-library link path:"
                " LibArchive=${FCEUX11_LIBARCHIVE_LINK} ZLIB=${FCEUX11_ZLIB_LINK} OpenGL=${FCEUX11_OPENGL_LINK}")
    endif()
endif()

# Qt5 support removed in Phase 2 — Qt6 only
# v0.3.1: Removed _CRT_SECURE_NO_WARNINGS; using standard secure APIs
add_definitions( -D__QT_DRIVER__  -DQT_DEPRECATED_WARNINGS -DPSS_STYLE=2 )
add_definitions( -D_UNICODE -DUNICODE )
add_definitions( -DFCEUDEF_DEBUGGER )
add_definitions( -D_USE_LIBARCHIVE )
if(FCEUX11_WGI_BACKEND)
    add_definitions( -DFCEUX11_WGI_BACKEND )
endif()
# v0.3.1: Soft warning level /W4 /WX-; /wd4267/4244 removed from here (see types.h cleanup)
# v0.3.16 (LTS closure) / v1.0: /WX activated in top-level CMakeLists.txt
# (with documented /wd set for pre-existing baseline). src/ CMakeLists.txt
# does NOT re-add /WX (it would conflict); it inherits the top-level set.
# We only add the driver-specific /wd set here (additional categories
# surfaced by Qt + SDL bindings).
if(MSVC)
    add_compile_options(/W4 /WX-
        /wd4127  # pre-existing: conditional expression is constant (Q_OBJECT generated code)
        /wd4189  # pre-existing: local variable initialized but not referenced (Qt headers)
        /wd4505  # pre-existing: unreferenced local function (Qt template instantiation)
    )
endif()

set( SYS_LIBS  wsock32 ws2_32 vfw32 Htmlhelp )
set(APP_ICON_RESOURCES_WINDOWS  ${CMAKE_SOURCE_DIR}/icons/fceux11.rc )

     if ( ${USE_LIBAV} )
        add_definitions( -D_USE_LIBAV ${LIBAV_CFLAGS} )
        include_directories( ${FFMPEG_INSTALL_PREFIX}/ffmpeg/include )
        set( LIBAV_LDFLAGS  ${FFMPEG_INSTALL_PREFIX}/ffmpeg/lib/avcodec.lib
        	                 ${FFMPEG_INSTALL_PREFIX}/ffmpeg/lib/avformat.lib
                	         ${FFMPEG_INSTALL_PREFIX}/ffmpeg/lib/avutil.lib
                        	 ${FFMPEG_INSTALL_PREFIX}/ffmpeg/lib/swscale.lib
                        	 ${FFMPEG_INSTALL_PREFIX}/ffmpeg/lib/swresample.lib )
     endif()

option(FCEUX11_LUA_RUST_ENABLED "Use Rust Lua engine (mlua) instead of C++ Lua engine" ON)
option(FCEUX11_WGI_BACKEND "Enable Windows.Gaming.Input backend (requires WinRT/C++/WinRT)" OFF)
# v0.3.15.x PHASE-3 (preserved in v1.0): probe-only DirectStorage
# scaffold. The probe detects NvmeSdsSupported() + dstorage.dll
# availability at startup but does not take over any I/O. Actual
# takeover of .fc0/.fcs writes is deferred to v2.0 (next major cycle
# after v1.0).
option(FCEUX11_DIRECT_STORAGE_PROBE "Enable DirectStorage 1.2 NVMe probe (Windows only)" ON)

if(FCEUX11_LUA_RUST_ENABLED AND FCEUX11_RUST_ENABLED)
    message(STATUS "Using Rust Lua engine (mlua vendored Lua 5.1)")
    add_definitions(-D_S9XLUA_H -DFCEUX11_LUA_RUST_ENABLED)
    set(LUA_ENGINE_SOURCE ${CMAKE_CURRENT_SOURCE_DIR}/lua-engine.cpp)
elseif ( ${LUA_FOUND} )
   # Use System LUA (legacy fallback, requires old lua-engine.cpp)
        message( STATUS "Using System Lua ${LUA_VERSION} (legacy, not recommended)" )

        add_definitions( -D_S9XLUA_H  ${LUA_CFLAGS} )

        set( LUA_ENGINE_SOURCE  ${CMAKE_CURRENT_SOURCE_DIR}/lua-engine.cpp )

else ()

   # Use Internal Lua (legacy fallback, requires old lua-engine.cpp)
        message( STATUS "Using Internal Lua (legacy, not recommended)" )

   add_definitions( -D_S9XLUA_H  -I${CMAKE_CURRENT_SOURCE_DIR}/lua/src )

        set( LUA_ENGINE_SOURCE  
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lapi.c 
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lauxlib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lbaselib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lcode.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/ldblib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/ldebug.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/ldo.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/ldump.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lfunc.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lgc.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/linit.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/liolib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/llex.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lmathlib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lmem.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/loadlib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lobject.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lopcodes.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/loslib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lparser.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lstate.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lstring.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lstrlib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/ltable.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/ltablib.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/ltm.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lundump.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lvm.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/lzio.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua/src/print.c
      ${CMAKE_CURRENT_SOURCE_DIR}/lua-engine.cpp 
   )
endif()

if ( ${ZLIB_FOUND} )

  message( STATUS "Using System zlib ${ZLIB_VERSION_STRING}" )
  #add_definitions( ${ZLIB_CFLAGS} )
  include_directories( ${ZLIB_INCLUDE_DIRS} )
else()

   message( STATUS "Using Internal zlib" )

endif()

include_directories( ${CMAKE_SOURCE_DIR}/src )
include_directories( ${CMAKE_SOURCE_DIR}/src/drivers )


set(SRC_CORE
	${CMAKE_CURRENT_SOURCE_DIR}/asm.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/bus.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/cart.cpp
	${CMAKE_CURRENT_SOURCE_DIR}/cart_class.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/cheat.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/conddebug.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/config.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/core_state.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/cpu.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/debug.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/debugsymboltable.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/drawing.cpp
	${CMAKE_CURRENT_SOURCE_DIR}/driver_callbacks.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/fceu.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/fds.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/fds_sound.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/file.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/emufile.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/filter.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/ines.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/ines_load.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/ines_init.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/ines_gi.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/ines_save.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/movie.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/netplay.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/nsf.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/nsf_load.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/nsf_runtime.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/nsf_ui.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/oldmovie.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/palette.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/profiler.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/ppu.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/ppu_class.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/apu.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/expansion_audio.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/sound.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/state.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/unif.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/unif_load.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/video.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/vsuni.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/wave.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/x6502.cpp
	${LUA_ENGINE_SOURCE}
	${CMAKE_CURRENT_SOURCE_DIR}/utils/ioapi.cpp
	${CMAKE_CURRENT_SOURCE_DIR}/utils/unzip.cpp
)

set(SRC_BOARDS
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/01-222.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/09-034a.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/103.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/106.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/108.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/112.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/116.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/117.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/120.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/121.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/12in1.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/151.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/156.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/158B.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/15.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/164.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/168.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/170.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/175.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/176.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/177.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/178.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/183.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/185.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/186.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/187.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/189.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/18.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/190.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/193.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/199.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/206.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/208.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/222.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/225.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/228.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/230.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/232.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/234.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/235.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/244.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/246.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/252.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/253.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/28.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/32.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/33.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/34.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/354.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/36.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/3d-block.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/40.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/411120-c.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/41.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/42.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/43.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/46.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/50.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/51.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/57.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/603-5052.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/62.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/65.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/67.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/68.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/69.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/71.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/72.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/77.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/79.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/80013-B.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/80.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/8157.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/8237.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/82.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/830118C.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/88.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/8in1.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/90.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/91.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/96.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/99.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/a9746.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ac-08.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/addrlatch.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ax5705.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bandai.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bb.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bmc13in1jy110.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bmc42in1r.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bmc64in1nr.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bmc70in1.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/BMW8544.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bonza.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bs-5.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/bs4xxxr.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/cheapocabra.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/cityfighter.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/coolboy.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/coolgirl.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/dance2000.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/datalatch.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/dream.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/__dummy_mapper.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/edu2000.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/eh8813a.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/emu2413.c
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/et-100.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/et-4320.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/F-15.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/famicombox.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ffe.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/fk23c.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/fns.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ghostbusters63in1.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/gs-2004.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/gs-2013.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/h2288.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/hp10xx_hp20xx.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/hp898f.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/inlnsf.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/karaoke.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/kof97.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7010.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7012.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7013.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7016.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7017.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7030.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7031.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7032.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7037.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/ks7057.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/le05.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/lh32.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/lh53.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/malee.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/mihunche.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/mmc1.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/mmc2and4.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/mmc3.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/mmc3_base_cart.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/mmc5.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/n106.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/n625092.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/registry.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/_cart_helpers.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/novel.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/onebus.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/pec-586.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/rt-01.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/sa-9602b.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/sachen.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/sb-2000.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/sc-127.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/sheroes.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/sl1632.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/subor.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/super24.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/supervision.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/t-227-1.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/t-262.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/tengen.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/tf-1201.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/transformer.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/unrom512.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/vrc1.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/vrc2and4.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/vrc3.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/vrc5.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/vrc6.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/vrc7.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/vrc7p.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/yoko.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/boards/inx007t.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/arkanoid.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/bworld.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/cursor.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/fkb.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/fns.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/ftrainer.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/hypershot.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/mahjong.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/mouse.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/oekakids.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/pec586kb.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/powerpad.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/quiz.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/shadow.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/snesmouse.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/suborkb.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/toprider.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/virtualboy.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/lcdcompzapper.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/input/zapper.cpp
)

set(SRC_UTILS
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/backward.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/ConvertUTF.c
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/xstring.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/crc32.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/endian.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/general.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/guid.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/md5.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/memory.cpp
  	${CMAKE_CURRENT_SOURCE_DIR}/utils/mutex.cpp
	${CMAKE_CURRENT_SOURCE_DIR}/utils/timeStamp.cpp
)


set(SRC_DRIVERS_COMMON
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/args.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/cheat.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/configSys.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/hq2x.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/hq3x.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/scale2x.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/scale3x.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/scalebit.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/vidblit.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/os_utils.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/nes_ntsc.c
)

set(SRC_DRIVERS_QT
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleWindow.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleViewerGL.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleViewerSDL.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleViewerQWidget.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleViewerInterface.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/InputConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/GamePadConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/FamilyKeyboard.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/HotKeyConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TimingConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/FrameTimingStats.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/PaletteConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/PaletteEditor.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ColorMenu.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/GuiConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/HelpPages.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/MoviePlay.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/MovieRecord.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/MovieOptions.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/LuaControl.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/CheatsConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/GameGenie.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/HexEditor.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/MsgLogViewer.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/CodeDataLogger.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/SymbolicDebug.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleDebugger.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleUtilities.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleVideoConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ConsoleSoundConf.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/StateRecorderConf.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/iNesHeaderEditor.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/SplashScreen.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TraceLogger.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/AboutWindow.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/fceuWrapper.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/fceu_archive.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/fceu_globals.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/fceu_callbacks.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/ppuViewer.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/NameTableViewer.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/RamWatch.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/RamSearch.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/config.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/input.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/nes_shm.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/keyscan.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/sdl-sound.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/sdl-video.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/sdl-joystick.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/input/input_manager.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/input/sdl_backend.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/input/xinput_backend.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/sdl-throttle.cpp  
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/QtNetplay.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/AviRecord.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/AviRiffViewer.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/avi/avi-utils.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/avi/fileio.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/avi/gwavi.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/TasEditorWindow.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/taseditor_project.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/taseditor_config.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/taseditor_lua.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/markers_manager.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/greenzone.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/selection.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/playback.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/recorder.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/history.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/splicer.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/inputlog.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/laglog.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/branches.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/bookmarks.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/bookmark.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/snapshot.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/TasEditor/markers.cpp
)

if(FCEUX11_WGI_BACKEND)
  list(APPEND SRC_QT ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/input/wgi_backend.cpp)
endif()

# v0.3.15.x PHASE-3: DirectStorage 1.2 NVMe probe scaffold. Compiled
# into a separate library so the probe code can be disabled via
# -DFCEUX11_DIRECT_STORAGE_PROBE=OFF without touching the rest of
# the build. The library is linked in below; note that we do NOT
# link nvme.lib because the probe resolves NvmeSdsSupported() at
# runtime via GetProcAddress to keep the toolchain dependency-free.
if(FCEUX11_DIRECT_STORAGE_PROBE AND WIN32)
  add_library(fceu11_direct_storage_probe STATIC
    ${CMAKE_CURRENT_SOURCE_DIR}/platform/win11/DirectStorageProbe.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/platform/win11/TaskbarProgress.cpp)
  target_include_directories(fceu11_direct_storage_probe PUBLIC
    ${CMAKE_SOURCE_DIR}/src)
endif()

add_library(fceux11_utils STATIC ${SRC_UTILS})
target_link_libraries(fceux11_utils PUBLIC Qt6::Widgets ${ZLIB_LIBRARIES})
target_include_directories(fceux11_utils PUBLIC ${CMAKE_SOURCE_DIR}/src)

add_library(fceux11_boards STATIC ${SRC_BOARDS})
target_link_libraries(fceux11_boards PUBLIC fceux11_utils ${ZLIB_LIBRARIES})
target_include_directories(fceux11_boards PUBLIC ${CMAKE_SOURCE_DIR}/src)

add_library(fceux11_core STATIC ${SRC_CORE})
target_link_libraries(fceux11_core PUBLIC fceux11_boards fceux11_utils ${ZLIB_LIBRARIES} SDL2::SDL2)
target_include_directories(fceux11_core PUBLIC ${CMAKE_SOURCE_DIR}/src)
# v0.3.1: PCH for core — types.h + API headers + fceu.h (C++ only; C files skip PCH)
# v1.11: driver.h replaced with 4 peer headers (Phase H)
target_precompile_headers(fceux11_core PRIVATE
    ${CMAKE_SOURCE_DIR}/src/types.h
    ${CMAKE_SOURCE_DIR}/src/core_api.h
    ${CMAKE_SOURCE_DIR}/src/io_api.h
    ${CMAKE_SOURCE_DIR}/src/net_api.h
    ${CMAKE_SOURCE_DIR}/src/diag_api.h
    ${CMAKE_SOURCE_DIR}/src/fceu.h
)
# Exclude C sources from C++ PCH to avoid STL1003 error
set_source_files_properties(
    ${CMAKE_CURRENT_SOURCE_DIR}/utils/ConvertUTF.c
    ${CMAKE_CURRENT_SOURCE_DIR}/drivers/common/nes_ntsc.c
    ${CMAKE_CURRENT_SOURCE_DIR}/boards/emu2413.c
    ${LUA_ENGINE_SOURCE}
    PROPERTIES SKIP_PRECOMPILE_HEADERS ON
)

add_library(fceux11_drivers_common STATIC ${SRC_DRIVERS_COMMON})
target_link_libraries(fceux11_drivers_common PUBLIC fceux11_core ${ZLIB_LIBRARIES})
target_include_directories(fceux11_drivers_common PUBLIC ${CMAKE_SOURCE_DIR}/src)

add_library(fceux11_drivers_qt STATIC ${SRC_DRIVERS_QT})
target_link_libraries(fceux11_drivers_qt PUBLIC
    fceux11_core
    fceux11_drivers_common
    Qt6::Widgets
    Qt6::OpenGL
    Qt6::OpenGLWidgets
    SDL2::SDL2
    ${FCEUX11_LIBARCHIVE_LINK}
    ${FCEUX11_ZLIB_LINK}
    ${FCEUX11_OPENGL_LINK}
    ${LUA_LDFLAGS}
    ${LIBAV_LDFLAGS}
    ${SYS_LIBS}
)
if(${QHELP})
    target_link_libraries(fceux11_drivers_qt PUBLIC Qt6::Help)
endif()
if(FCEUX11_WGI_BACKEND)
    target_link_libraries(fceux11_drivers_qt PUBLIC WindowsApp)
endif()
if(FCEUX11_DIRECT_STORAGE_PROBE AND WIN32)
    target_link_libraries(fceux11_drivers_qt PUBLIC fceu11_direct_storage_probe)
endif()
target_include_directories(fceux11_drivers_qt PUBLIC
    ${CMAKE_SOURCE_DIR}/src
    ${CMAKE_SOURCE_DIR}/src/drivers
)
# v0.3.1: PCH for Qt drivers — QtWidgets + QtOpenGL
target_precompile_headers(fceux11_drivers_qt PRIVATE
    <QtWidgets>
    <QtOpenGL>
)

# i18n: Qt translation files (v0.3.15 PR-B)
# Source-of-truth: src/drivers/Qt/lang/fceux11_*.ts
# Note: assets/i18n/ pre-v0.3.15 hand-written files are deprecated.
# Lupdate is wired to the source tree so that re-running it refreshes
# the .ts file with all current tr() calls (no manual file sync).
#
# v0.3.15.x PHASE-5: the previous qt_add_resources() flow produced
# a separate static-lib target (fceux11_i18n) that the main fceux11
# executable never linked against, leaving QTranslator::load()
# returning false at runtime. We now run lrelease and copy the
# .qm output into assets/i18n/ at build time, where resources.qrc
# picks them up via CMAKE_AUTORCC — one less custom target, no
# link-step risk.
set(TS_FILES
    ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/lang/fceux11_en.ts
    ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/lang/fceux11_zh_CN.ts
    ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/lang/fceux11_zh_TW.ts
)

set(I18N_OUT_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../assets/i18n)

if(FCEUX11_ENABLE_I18N)
    find_package(Qt6 REQUIRED COMPONENTS LinguistTools)

    set_source_files_properties(${TS_FILES} PROPERTIES
        OUTPUT_LOCATION ${I18N_OUT_DIR})

    qt_add_lrelease(TS_FILES ${TS_FILES} QM_FILES_OUTPUT_VARIABLE qm_files)
endif()

set(SOURCES ${SRC_DRIVERS_QT})
list(APPEND SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/drivers/Qt/main.cpp)

# Phase 7: Rust module integration
add_subdirectory(rust)
if(FCEUX11_RUST_ENABLED)
    add_definitions(-DFCEUX11_RUST_ENABLED)
    target_link_libraries(fceux11_utils PUBLIC ${FCEUX11_RUST_STATIC_LIB})
endif()

# Put build timestamp into BUILD_TS environment variable and from there into
# the FCEUX_BUILD_TIMESTAMP preprocessor definition.
# Note: with CMake >= 3.8.0, this will respect SOURCE_DATE_EPOCH. For more info,
#       see <https://reproducible-builds.org/docs/source-date-epoch/>.
string(TIMESTAMP BUILD_TS "%H:%M:%S  %b %d %Y" UTC)
add_definitions( -DFCEUX_BUILD_TIMESTAMP=\"${BUILD_TS}\" )

add_custom_command( 
	OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/fceux_git_info.cpp  
	COMMAND  ${CMAKE_SOURCE_DIR}/scripts/genGitHdr.bat  ${CMAKE_CURRENT_BINARY_DIR} 
	VERBATIM )

set_property(SOURCE ${CMAKE_CURRENT_BINARY_DIR}/fceux_git_info.cpp PROPERTY SKIP_AUTOGEN ON)

add_executable(  ${APP_NAME}  ${SOURCES} ${CMAKE_SOURCE_DIR}/resources.qrc 
	${CMAKE_CURRENT_BINARY_DIR}/fceux_git_info.cpp
	${APP_ICON_RESOURCES_WINDOWS} )

if(FCEUX11_RUST_ENABLED)
    add_dependencies(${APP_NAME} fceux11_rust_build)
endif()

target_link_libraries( ${APP_NAME}
   fceux11_drivers_qt
)

# v0.3.6.5: link-time sanitizer flags. Use target_link_options (not
# target_link_libraries with raw flag variables) — the latter triggers
# a Ninja MSVC-generator bug where leading-`/` flags get put in the
# build input list instead of the link rule's FLAGS.
if(ASAN_LDFLAGS OR GPROF_LDFLAGS)
    target_link_options(${APP_NAME} PRIVATE ${ASAN_LDFLAGS} ${GPROF_LDFLAGS})
endif()

if (WIN32)
	#   target_link_libraries( ${APP_NAME} wsock32 ws2_32 )

   set_target_properties(${APP_NAME} PROPERTIES WIN32_EXECUTABLE TRUE)
   target_link_options(${APP_NAME} PRIVATE /MANIFEST:NO)
   target_link_libraries(${APP_NAME} ntdll)
endif()



