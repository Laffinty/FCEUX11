// FCEUX11 v0.3.9 — driver.h is now a thin shim header. The pre-v0.3.9
// 371-line monolith has been physically split into four peer headers
// per plan v3 §5 v0.3.9:
//
//   core_api.h  — game lifecycle, state, frame, cheats, debug, control
//   io_api.h    — file I/O, input devices, audio, video, AVI, IoDir
//   net_api.h   — netplay start/stop, send/receive, network close
//   diag_api.h  — version / compiler identifier string
//
// All four are peer headers (no layering). The 33 existing
// `#include "driver.h"` call sites compile unchanged because this shim
// re-includes the four new headers. Removal of the shim is scheduled
// for v0.4.0.
#ifndef __DRIVER_H_
#define __DRIVER_H_
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#endif //__DRIVER_H_
