
cmake_minimum_required(VERSION 4.0)

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

project(FCEUX11 VERSION 0.2.26 LANGUAGES CXX C)

# v0.2.1: Enforce MSVC toolchain; MinGW-w64 / MSYS2 support removed
if(NOT MSVC)
    message(FATAL_ERROR "FCEUX11 v0.2.1+ requires MSVC 2022+ toolchain. "
                        "MinGW-w64 / MSYS2 support has been removed.")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# v0.2.1: MSVC security and conformance options
# v0.3.1: /W4 soft-start with /WX- (warnings are not errors yet; /WX pushed to v0.3.16)
# v0.3.1: /wd4996 is transitional — will be removed after strcpy/sprintf/vsprintf cleanup in v0.3.5
add_compile_options(/W4 /permissive- /guard:cf /GS /sdl /wd4996)
add_link_options(/GUARD:CF /CETCOMPAT)

# v0.3.1: ccache probe (Windows chocolatey path)
find_program(CCACHE_PROGRAM ccache PATHS "C:/ProgramData/chocolatey/bin" "C:/Program Files/ccache" "C:/msys64/usr/bin")
if(CCACHE_PROGRAM)
    message(STATUS "ccache found: ${CCACHE_PROGRAM}")
    set(CMAKE_C_COMPILER_LAUNCHER   "${CCACHE_PROGRAM}")
    set(CMAKE_CXX_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
else()
    message(STATUS "ccache not found — incremental builds will use compiler cache only")
endif()

option(FCEUX11_BUILD_TESTS "Build smoke and regression tests" ON)
option(FCEUX11_ENABLE_I18N "Enable internationalization (i18n) support" ON)
option(ENABLE_LINT "Enable static analysis targets (clang-tidy, cppcheck)" OFF)

# v0.3.2: Export compile_commands.json for clang-tidy / IDE integration
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# i18n: Qt Linguist tools
if(FCEUX11_ENABLE_I18N)
    find_package(Qt6 REQUIRED COMPONENTS LinguistTools)
endif()

add_subdirectory( src )

if(FCEUX11_BUILD_TESTS)
    enable_testing()
    add_subdirectory( tests )
    include(CTest)
endif()

# v0.3.2: Optional lint targets (clang-tidy / cppcheck)
if(ENABLE_LINT)
    find_program(CLANG_TIDY_PROG clang-tidy)
    find_program(CPPCHECK_PROG cppcheck)

    if(CLANG_TIDY_PROG AND EXISTS "${CMAKE_BINARY_DIR}/compile_commands.json")
        add_custom_target(lint-clang-tidy
            COMMAND ${CLANG_TIDY_PROG}
                -p ${CMAKE_BINARY_DIR}
                ${CMAKE_SOURCE_DIR}/src/*.cpp
                ${CMAKE_SOURCE_DIR}/src/drivers/Qt/*.cpp
                ${CMAKE_SOURCE_DIR}/src/utils/*.cpp
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Running clang-tidy..."
        )
    endif()

    if(CPPCHECK_PROG)
        add_custom_target(lint-cppcheck
            COMMAND ${CPPCHECK_PROG}
                --enable=warning,style,performance,portability
                --std=c++20 --language=c++ --template=gcc
                --suppress=missingIncludeSystem
                --suppress=unusedFunction
                -I ${CMAKE_SOURCE_DIR}/src
                -I ${CMAKE_SOURCE_DIR}/src/drivers
                ${CMAKE_SOURCE_DIR}/src/
            WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
            COMMENT "Running cppcheck..."
        )
    endif()
endif()
