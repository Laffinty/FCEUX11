# translations.pro - v0.3.15 PR-B
# Qt Linguist project for FCEUX11 i18n.
#
# SOURCES:  all C++/H files that may contain tr() calls
# TRANSLATIONS:  one .ts file per language
#
# Usage:
#   lupdate -project translations.pro        # sync .ts from sources
#   lrelease -project translations.pro        # compile .ts -> .qm
#
# In this project, the CMake build wires both lupdate and lrelease via
# the qt_add_lupdate / qt_add_lrelease modern Qt6 API. This .pro file
# is kept for manual lupdate runs in Qt Linguist (Qt Creator).

SOURCES = \
    $$absolute_path(../)/*.cpp \
    $$absolute_path(../)/*.h \
    $$absolute_path(../TasEditor)/*.cpp \
    $$absolute_path(../TasEditor)/*.h

TRANSLATIONS = \
    $$absolute_path(fceux11_en.ts) \
    $$absolute_path(fceux11_zh_CN.ts) \
    $$absolute_path(fceux11_zh_TW.ts)

CODECFORSRC = UTF-8
