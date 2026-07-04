# translations.pro - v0.3.15 PR-B / v1.11 §11.5 expanded
# Qt Linguist project for FCEUX11 i18n.
#
# SOURCES:  all C++/H files that may contain tr() calls
# TRANSLATIONS:  one .ts file per language (12 languages)
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
    $$absolute_path(fceux11_zh_TW.ts) \
    $$absolute_path(fceux11_ja.ts) \
    $$absolute_path(fceux11_ko.ts) \
    $$absolute_path(fceux11_es.ts) \
    $$absolute_path(fceux11_fr.ts) \
    $$absolute_path(fceux11_de.ts) \
    $$absolute_path(fceux11_vi.ts) \
    $$absolute_path(fceux11_th.ts) \
    $$absolute_path(fceux11_hi.ts) \
    $$absolute_path(fceux11_ar.ts)

CODECFORSRC = UTF-8
