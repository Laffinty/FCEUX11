/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2020 mjbudd77
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "Qt/nes_shm.h"

nes_shm_t *nes_shm = NULL;

//************************************************************************
nes_shm_t *open_nes_shm(void)
{
	nes_shm_t *vaddr;

	vaddr = new nes_shm_t;

	// hotfix1 P1-12 (N-H03): zero-init the cross-thread atomic fields
	// explicitly via .store() (relaxed, no observers yet). The previous
	// memset() over the whole struct was unsafe because std::atomic<T>
	// members do not have to be trivially constructible and the spec
	// doesn't bless bulk-binary zeroing as a valid stand-in for value
	// initialisation.
	vaddr->runEmulator.store(0);
	vaddr->blitUpdated.store(0);
	vaddr->pixBufIdx.store(0);
	vaddr->sndBuf.head.store(0);
	vaddr->sndBuf.tail.store(0);
	vaddr->sndBuf.starveCounter.store(0);

	// POD fields left over from the old memset() bulk zero. These are
	// either unused outside the main thread or only ever observed as
	// gauges after a brief one-way crossing with no happens-before
	// requirement, so a plain memset is fine for them.
	memset( &vaddr->video, 0, sizeof(vaddr->video) );

	vaddr->video.ncol      = GL_NES_WIDTH;
	vaddr->video.nrow      = GL_NES_HEIGHT;
	vaddr->video.pitch     = GL_NES_WIDTH * 4;
	vaddr->video.xscale    = 1;
	vaddr->video.yscale    = 1;
	vaddr->video.xyRatio   = 1;
	vaddr->video.preScaler = 0;

	return vaddr;
}
//************************************************************************
void close_nes_shm(void)
{
	if ( nes_shm )
	{
		delete nes_shm; nes_shm = NULL;
	}

}
//************************************************************************
