// nes_shm.h
//

#ifndef __NES_SHM_H__
#define __NES_SHM_H__

#include <stdint.h>
#include <atomic>
#include <cstring>

#define  GL_WIN_PIXEL_LINEAR_FILTER  0x0001
#define  GL_WIN_DOUBLE_BUFFER        0x0002

#define  GL_NES_WIDTH   256
#define  GL_NES_HEIGHT  240
#define  NES_VIDEO_BUFLEN   5
#define  NES_AUDIO_BUFLEN   480000

struct  nes_shm_t
{
	int   pid;
	int   run;
	uint32_t  render_count;
	uint32_t  blit_count;

	struct
	{
		int   ncol;
		int   nrow;
		int   pitch;
		int   xscale;
		int   yscale;
		int   xyRatio;
		int   preScaler;
		int   test;
	} video;

	// hotfix1 P1-12 (N-H03): cross-thread fields promoted to std::atomic
	//   so single-byte / single-int reads/writes are lock-free and
	//   well-defined under the memory model. The producer/consumer use
	//   sites still treat these as plain values for the most part
	//   (`if (runEmulator.load())`, `pixBufIdx.store(...)`); the helpers
	//   below funnel every write through a relaxed-or-acquire load so
	//   downstream readers see consistent state without per-call site
	//   ceremony. Ordering choice: runEmulator and blitUpdated are
	//   bookkeeping flags so we use relaxed; pixBufIdx participates in
	//   the producer/consumer frame-buffer handoff so the producer
	//   does release and the consumer acquire; sound head/starveCounter
	//   use relaxed too (the underlying int16_t sample array is single-
	//   producer by design).
	std::atomic<char>  runEmulator{0};
	std::atomic<char>  blitUpdated{0};
	std::atomic<int>   pixBufIdx{0};
	uint32_t  pixbuf[NES_VIDEO_BUFLEN][1048576]; // 1024 x 1024
	uint32_t  avibuf[1048576]; // 1024 x 1024

	void clear_pixbuf(void)
	{
		memset( pixbuf, 0, sizeof(pixbuf) );
		memset( avibuf, 0, sizeof(avibuf) );
	}

	struct sndBuf_t
	{
		std::atomic<int>  head{0};
		// tail is currently unused by readers (see sdl-sound.cpp) but
		// kept as atomic so future readers can adopt it without further
		// surgery on this struct's layout.
		std::atomic<int>  tail{0};
		int16_t  data[NES_AUDIO_BUFLEN];
		std::atomic<unsigned int> starveCounter{0};
	} sndBuf;

	void  push_sound_sample( int16_t sample )
	{
		// 32-bit scalar atomic loads are lock-free on x86_64 MSVC, so a
		// single fetch_add() gives us the wrapping head update in one
		// RMW. Avoids the read-modify-write data race that the previous
		// pair of plain `head` reads + plain `head =` writes had.
		const int idx = sndBuf.head.fetch_add(1, std::memory_order_relaxed) % NES_AUDIO_BUFLEN;
		sndBuf.data[idx] = sample;
	}
};

extern nes_shm_t *nes_shm;

nes_shm_t *open_nes_shm(void);

void close_nes_shm(void);

#endif
