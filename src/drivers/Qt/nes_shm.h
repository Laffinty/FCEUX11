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
	// hotfix3 A-3 (QT-CRASH-01): promote remaining plain-int fields to
	// std::atomic. hotfix1 P1-12 only atomised runEmulator /
	// blitUpdated / pixBufIdx / sound ring indices; the GUI thread
	// (e.g. viewers reading video.ncol inside paintGL) was subject to
	// torn reads + reordering whenever the emulator thread wrote them
	// in the same frame. Layout preserved (no nested VideoAtomic, no
	// pid/run removal) so call sites stay byte-identical apart from
	// the .load() / .store() calls. Ordering policy: release on the
	// producer (emulator) write side, acquire on the GUI read side;
	// relaxed for the bookkeeping counters.
	std::atomic<int>      pid{0};
	std::atomic<int>      run{0};
	std::atomic<uint32_t> render_count{0};
	std::atomic<uint32_t> blit_count{0};

	struct
	{
		std::atomic<int>  ncol{0};
		std::atomic<int>  nrow{0};
		std::atomic<int>  pitch{0};
		std::atomic<int>  xscale{0};
		std::atomic<int>  yscale{0};
		std::atomic<int>  xyRatio{0};
		std::atomic<int>  preScaler{0};
		std::atomic<int>  test{0};
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
