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

#include "common/nes_shm.h"

nes_shm_t *nes_shm = NULL;

//************************************************************************
nes_shm_t *open_nes_shm(void)
{
	nes_shm_t *vaddr;

	vaddr = new nes_shm_t;

	// hotfix1 P1-12 (N-H03) + hotfix3 A-3 (QT-CRASH-01): zero-init every
	// cross-thread atomic field explicitly via .store() (relaxed, no
	// observers yet). The previous memset() over the whole struct was
	// unsafe because std::atomic<T> members do not have to be trivially
	// constructible and the spec doesn't bless bulk-binary zeroing as a
	// valid stand-in for value initialisation. After A-3 the entire
	// `video` sub-struct + the bookkeeping counters are atomic, so the
	// seed writes are per-field .store() calls below.
	vaddr->runEmulator.store(0);
	vaddr->blitUpdated.store(0);
	vaddr->pixBufIdx.store(0);
	vaddr->render_count.store(0);
	vaddr->blit_count.store(0);
	vaddr->pid.store(0);
	vaddr->run.store(0);
	vaddr->sndBuf.head.store(0);
	vaddr->sndBuf.tail.store(0);
	vaddr->sndBuf.starveCounter.store(0);

	vaddr->video.ncol.store(      GL_NES_WIDTH,     std::memory_order_relaxed);
	vaddr->video.nrow.store(      GL_NES_HEIGHT,    std::memory_order_relaxed);
	vaddr->video.pitch.store(     GL_NES_WIDTH * 4, std::memory_order_relaxed);
	vaddr->video.xscale.store(    1, std::memory_order_relaxed);
	vaddr->video.yscale.store(    1, std::memory_order_relaxed);
	vaddr->video.xyRatio.store(   1, std::memory_order_relaxed);
	vaddr->video.preScaler.store( 0, std::memory_order_relaxed);
	vaddr->video.test.store(      0, std::memory_order_relaxed);

	// hotfix3 D-1: replace the 24 MiB static pixbuf[5][1048576] +
	// avibuf[1048576] with heap-allocated pools sized to the actual
	// video dimensions (256x240 at NES res = 1.2 MiB, down from 24 MiB).
	// No GUI consumer is alive at open_nes_shm time, so this initial
	// resize is the safest resize point (no race on the underlying
	// buffer during this call). Subsequent resizes during video mode
	// changes must coordinate with the existing blitUpdated release /
	// acquire contract (see PixBufPool class doc in nes_shm.h).
	vaddr->pixBufPool.resize(GL_NES_WIDTH, GL_NES_HEIGHT);
	vaddr->aviBuf.resize(GL_NES_WIDTH, GL_NES_HEIGHT);

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
