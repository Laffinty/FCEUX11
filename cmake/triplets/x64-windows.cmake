# FCEUX11 CI overlay triplet — x64-windows (release-only)
#
# WHY THIS FILE EXISTS
# --------------------
# GitHub-hosted runners start with an empty vcpkg binary cache, so
# `kagami-qa.yml` / `ci.yml` used to build all 33 manifest ports from source
# on every run — including Qt 6.8.0 in BOTH debug and release. Run
# 82956632293 (2026-07-31, commit 10f1e05) was killed by
# `timeout-minutes: 45` while still inside the CMake configure step, at
# "Installing 30/33 qtbase[...] -> Building x64-windows-dbg". No build, no
# ctest, no migration matrix.
#
# `set(VCPKG_BUILD_TYPE release)` drops the debug half of every port. Local
# measurement of a full `vcpkg_installed/x64-windows`: 2.4 GB total, of which
# `debug/` alone is 1.2 GB. So this roughly halves both the cold build time
# and the size of the GitHub Actions cache entry.
#
# WHY THE NAME IS DELIBERATELY `x64-windows`
# ------------------------------------------
# This overlay intentionally SHADOWS vcpkg's builtin `x64-windows` triplet
# rather than introducing a new name such as `x64-windows-rel`. vcpkg's
# documented behaviour for a same-named overlay triplet is "use the overlay
# instead of the builtin". Keeping the name matters because the install
# directory is derived from the triplet name, and two places in this repo
# hard-code `vcpkg_installed/x64-windows`:
#
#   * CMakeLists.txt:7  — the "prefer local vcpkg_installed" branch probes
#     `EXISTS ${CMAKE_SOURCE_DIR}/vcpkg_installed/x64-windows` and, when it
#     hits, appends that prefix to CMAKE_PREFIX_PATH WITHOUT setting
#     CMAKE_TOOLCHAIN_FILE — i.e. vcpkg/manifest mode is skipped entirely.
#     This is what makes a cache-warm CI configure step finish in seconds.
#   * tests/CMakeLists.txt:431 — the Windows test-runtime PATH injection
#     prepends `vcpkg_installed/x64-windows/bin` so ctest executables can
#     resolve Qt6Core.dll / SDL2.dll instead of dying with
#     STATUS_DLL_NOT_FOUND (0xc0000135).
#
# A differently-named triplet would install to `vcpkg_installed/<other>` and
# silently miss both, so do NOT rename this file.
#
# HOW IT IS ACTIVATED
# -------------------
# Only when a build explicitly passes the overlay directory:
#
#   cmake -S . -B build \
#     -DVCPKG_INSTALLED_DIR=<workspace>/vcpkg_installed \
#     -DVCPKG_OVERLAY_TRIPLETS=<workspace>/cmake/triplets
#
# Both `.github/workflows/kagami-qa.yml` and `.github/workflows/ci.yml` pass
# these. `scripts/do_build.ps1` does NOT, so local developer builds keep
# using whatever `vcpkg_installed/x64-windows` they already have (debug half
# included) and are unaffected by this file.
#
# DEVELOPER NOTE
# --------------
# Because this overlay changes VCPKG_BUILD_TYPE, its per-port ABI hash
# differs from vcpkg's builtin `x64-windows`. If a local `vcpkg install` /
# `vcpkg upgrade` ever complains about a mismatch between the install tree
# and the recorded status, delete the metadata directory
# `vcpkg_installed/vcpkg/` — NOT the artifact directory
# `vcpkg_installed/x64-windows/`, which is the expensive one to rebuild.

set(VCPKG_TARGET_ARCHITECTURE x64)
set(VCPKG_CRT_LINKAGE dynamic)
set(VCPKG_LIBRARY_LINKAGE dynamic)

# Release only — see rationale above. CI configures with
# -DCMAKE_BUILD_TYPE=Release exclusively, and src/CMakeLists.txt:47-57
# (fceux11_resolve_linked_lib) only reaches for raw vcpkg library paths on
# non-Release or sanitizer builds, neither of which CI enables.
set(VCPKG_BUILD_TYPE release)
