// nes_shm.h
//

#ifndef __NES_SHM_H__
#define __NES_SHM_H__

#include <stdint.h>
#include <atomic>
#include <cstring>
#include <vector>

#define  GL_WIN_PIXEL_LINEAR_FILTER  0x0001
#define  GL_WIN_DOUBLE_BUFFER        0x0002

#define  GL_NES_WIDTH   256
#define  GL_NES_HEIGHT  240
#define  NES_VIDEO_BUFLEN   5
#define  NES_AUDIO_BUFLEN   480000

// hotfix3 D-1 (QT-CRASH-02 / QT-PERF-01): heap-allocated video-frame
// ring buffer pool sized to the actual video dimensions.
//
// Pre-fix layout carried `uint32_t pixbuf[NES_VIDEO_BUFLEN][1048576]` and
// `uint32_t avibuf[1048576]` as static arrays inside the shared
// memory region - 24 MiB of BSS-resident storage for what is, in a
// 256x240 game, 1.2 MiB of actually-used frame data. `clear_pixbuf()`
// walked 24 MiB through memset per call, and `setprg8` style callers
// had to know the constant 1048576 stride that nothing actually fills.
//
// API:
//   resize(ncol, nrow)  - emulator thread only; called from
//                          video-mode-change / PowerNES / open_nes_shm.
//                          Allocates new buffer, swaps, frees old.
//   slot(i)             - returns uint32_t* of `cap_ = ncol*nrow`
//                          valid 32-bit entries (1.2 MiB at NES res).
//                          Buffer is laid out 1 RGBA pixel per uint32_t.
//   clear()             - zeros active area only (slot_i for all i).
//                          Replaces the old `memset(pixbuf, 0, 20 MiB)`.
//   bytes()             - total bytes in pool across all slots.
//   generation()        - bumped per resize; GUI consumers can compare
//                          against a cached value to detect shrink
//                          events. Not enforced in this PR; the GUI's
//                          release/acquire on `blitUpdated` already
//                          acts as the resize barrier.
//
// Cross-thread coordination:
//   resize() must run when no GUI viewer is mid-transfer. The natural
//   safe points are open_nes_shm (no GUI yet) and video-mode-change /
//   PowerNES (GUI is paused on `runEmulator=0`). Consumer reads via
//   `slot(i)` are protected by the same release/acquire contract on
//   `blitUpdated` / `pixBufIdx` that the pre-fix `pixbuf[i]` reads
//   relied on (sdl-video.cpp:544 release on producer, ConsoleViewer*
//   .cpp transfer2LocalBuffer acquire on consumer).
class PixBufPool
{
	std::vector<uint32_t> buf_;
	size_t               ncol_{0};
	size_t               nrow_{0};
	size_t               cap_{0};
	std::atomic<uint32_t> gen_{0};

public:
	// Pre-fix noted: initialised via static; we keep generation init
	// atomic so consumers can rely on gen_ being defined post-resize.
	PixBufPool() noexcept : ncol_(0), nrow_(0), cap_(0), gen_(0) {}

	// Resize the pool to ncol x nrow. Re-allocates a fresh std::vector
	// (swap-then-dealloc pattern), then atomically bumps the generation.
	// Call sites: open_nes_shm (emulator thread, pre-GUI), sdl-video.cpp
	// video mode change, fceu.cpp PowerNES/ResetNES. Callers MUST be on
	// the emulator thread (or equivalent idle state where no GUI
	// consumer is mid-transfer).
	void resize(int ncol, int nrow)
	{
		if (ncol <= 0 || nrow <= 0) return;
		std::vector<uint32_t> fresh(static_cast<size_t>(ncol) * nrow * NES_VIDEO_BUFLEN);
		buf_.swap(fresh);                    // RAII frees `fresh` here.
		ncol_ = static_cast<size_t>(ncol);
		nrow_ = static_cast<size_t>(nrow);
		cap_  = ncol_ * nrow_;
		gen_.fetch_add(1, std::memory_order_release);
	}

	// Return pointer to slot `i` (0 <= i < NES_VIDEO_BUFLEN). The slot
	// holds `cap_` uint32_t entries (= ncol_*nrow_ RGBA pixels).
	uint32_t* slot(int i) noexcept
	{
		return buf_.data() + static_cast<size_t>(i) * cap_;
	}

	// Zero all 5 slots' active area only. Replaces memset(pixbuf, 0,
	// 20 MiB). At NES res this is 1.2 MiB; at 1024x1024 it's 20 MiB
	// (degenerate case but matches the pre-fix ceiling).
	void clear() noexcept
	{
		if (cap_ == 0 || buf_.empty()) return;
		for (int i = 0; i < NES_VIDEO_BUFLEN; ++i)
			memset(slot(i), 0, cap_ * sizeof(uint32_t));
	}

	size_t bytes() const noexcept { return buf_.size() * sizeof(uint32_t); }
	uint32_t generation() const noexcept { return gen_.load(std::memory_order_acquire); }
};

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

	// hotfix3 D-1: 20 MiB static `pixbuf[5][1048576]` replaced with a
	// heap-pool sized to actual video dimensions (1.2 MiB at NES res).
	// See PixBufPool above. Call sites use `pixBufPool.slot(i)` instead
	// of `pixbuf[i]`; both refer to the same 5-entry ring used by the
	// producer/consumer release/acquire contract on blitUpdated.
	PixBufPool          pixBufPool;

	// hotfix3 D-1: 4 MiB static `avibuf[1048576]` replaced with a single-
	// slot pool sized the same as pixBufPool. AVI recording reads back
	// into aviBuf.slot(0)[i] (AviRecord.cpp:554).
	PixBufPool          aviBuf;

	void clear_pixbuf(void)
	{
		pixBufPool.clear();
		aviBuf.clear();
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
