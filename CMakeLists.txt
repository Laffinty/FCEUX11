
cmake_minimum_required(VERSION 3.28)

# v0.2.1: Auto-detect vcpkg toolchain
if(DEFINED ENV{VCPKG_ROOT} AND NOT DEFINED CMAKE_TOOLCHAIN_FILE)
    # Prefer local vcpkg_installed (avoids manifest-mode re-build of Qt6 etc.)
    if(EXISTS "${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows")
        list(APPEND CMAKE_PREFIX_PATH "${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows")
        message(STATUS "Using local vcpkg_installed: ${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows")
    else()
        set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake"
            CACHE STRING "")
    endif()
endif()

project(FCEUX11 VERSION 0.2.24.1 LANGUAGES CXX C)

# v0.2.1: Enforce MSVC toolchain; MinGW-w64 / MSYS2 support removed
if(NOT MSVC)
    message(FATAL_ERROR "FCEUX11 v0.2.1+ requires MSVC 2022+ toolchain. "
                        "MinGW-w64 / MSYS2 support has been removed.")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# v0.2.1: MSVC security and conformance options
add_compile_options(/W4 /permissive- /guard:cf /GS /sdl)
add_link_options(/GUARD:CF /CETCOMPAT)

option(FCEUX11_BUILD_TESTS "Build smoke and regression tests" ON)
option(FCEUX11_ENABLE_I18N "Enable internationalization (i18n) support" ON)

# i18n: Qt Linguist tools
if(FCEUX11_ENABLE_I18N)
    find_package(Qt6 REQUIRED COMPONENTS LinguistTools)
endif()

add_subdirectory( src )

if(FCEUX11_BUILD_TESTS)
    enable_testing()
endif()
