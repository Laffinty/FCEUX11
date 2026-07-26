// KagamiQA P1 — Null Driver (headless test harness foundation).
//
// Provides globals that the core engine expects (dendy, pal_emulation)
// without linking Qt or SDL. Registers all-nullptr DriverCallbacks so
// that every FCEUD_* callback is a safe no-op.
//
// Usage from a headless test binary:
//   null_driver_init();          // install null callbacks
//   fceu11::Initialize();        // bring up the engine
//   ... run frames / load ROM ...
//   fceu11::Kill();              // tear down

#pragma once

extern int dendy;
extern int pal_emulation;

/// Register all-nullptr DriverCallbacks and initialise driver globals.
/// Call once before fceu11::Initialize().
void null_driver_init();
