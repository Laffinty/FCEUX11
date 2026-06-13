#ifndef __FCEU_SDL_VIDEO_H
#define __FCEU_SDL_VIDEO_H
#ifdef _SDL2
#include <SDL2/SDL.h>
#else
#include <SDL.h>
#endif

uint32 PtoV(double x, double y);
bool FCEUD_ShouldDrawInputAids();
namespace fceu11 {
    bool AviDisableMovieMessages();
    bool AviEnableHUDrecording();
    void SetAviEnableHUDrecording(bool enable);
    void SetAviDisableMovieMessages(bool disable);
} // namespace fceu11
#endif

