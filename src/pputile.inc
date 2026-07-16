uint8 *C;
uint8 cc;
uint32 vadr;
#ifdef PPU_VRC5FETCH
uint8 tmpd;
#endif

#ifndef PPUT_MMC5SP
	FCEU_MAYBE_UNUSED uint8 zz;
#else
	uint8 xs, ys;
	xs = X1;
	ys = ((g_cpu.scanline_ref() >> 3) + MMC5HackSPScroll) & 0x1F;
	if (ys >= 0x1E) ys -= 0x1E;
#endif

if (X1 >= 2) {
	uint8 *S = PALRAM.data();
	uint32 pixdata;

	pixdata = ppulut1[(pshift[0] >> (8 - XOffset)) & 0xFF] | ppulut2[(pshift[1] >> (8 - XOffset)) & 0xFF];

	pixdata |= ppulut3[XOffset | (atlatch << 3)];

	P[0] = S[pixdata & 0xF];
	pixdata >>= 4;
	P[1] = S[pixdata & 0xF];
	pixdata >>= 4;
	P[2] = S[pixdata & 0xF];
	pixdata >>= 4;
	P[3] = S[pixdata & 0xF];
	pixdata >>= 4;
	P[4] = S[pixdata & 0xF];
	pixdata >>= 4;
	P[5] = S[pixdata & 0xF];
	pixdata >>= 4;
	P[6] = S[pixdata & 0xF];
	pixdata >>= 4;
	P[7] = S[pixdata & 0xF];
	P += 8;
}

#ifdef PPUT_MMC5SP
	vadr = (MMC5HackExNTARAMPtr[xs | (ys << 5)] << 4) + (vofs & 7);
#else
	zz = RefreshAddr & 0x1F;
	// hotfix2 P3-5 (MAP-3) review conclusion (no code change):
	// `vnapage[(RefreshAddr >> 10) & 3]` is a 2-level indirection
	// (table-of-pointers lookup, then byte-fetch). Both modern
	// MSVC /O2 and Clang -O2 already CSE the index within the
	// inner loop (it only depends on `RefreshAddr`, which the
	// compiler sees as loop-invariant across the per-tile body),
	// and the four pointers typically map to four distinct 1 KiB
	// NTARAM slices that sit in the same few cache lines (NTARAM
	// is 2 KiB contiguous, MMC5fill is 1 KiB contiguous, etc.).
	// Plan §十 P3-5 option A (keep current code, rely on
	// compiler optimisation) is the right call here:
	//   * Option B (separate tight nt_ptrs[4]) would duplicate the
	//     bookkeeping that boards/*.cpp / ppu_class.cpp already do
	//     to keep vnapage_[] in sync, with no measurable win.
	//   * Option C (_mm_prefetch at RefreshLine entry) trades 4
	//     prefetch instructions for ~40 cycles of L1 fill each,
	//     which only helps when the nametables have been evicted
	//     since the last scanline — for the common case (warm
	//     NTARAM in L1 across adjacent scanlines) it is pure
	//     overhead.
	// Conclusion: leave as-is, document the rationale so a future
	// reviewer does not re-flag this site.
	C = vnapage[(RefreshAddr >> 10) & 3];
#ifdef PPU_VRC5FETCH
	tmpd = QTAINTRAM[((((RefreshAddr >> 10) & 3) >> ((qtaintramreg >> 1)) & 1) << 10) | (RefreshAddr & 0x3FF)];
	vofs = ((tmpd & 0x3F) << 12) | ((RefreshAddr >> 12) & 7);	// recalculate VROM offset
#endif
	vadr = (C[RefreshAddr & 0x3ff] << 4) + vofs;				// Fetch name table byte.
#endif

#ifdef PPUT_HOOK
	// hotfix2 P1-5 (INLINE-2): guard the indirect call so the compiler
	// can hoist the null-check and skip it on non-hooked paths. The
	// dispatcher in ppu_rendering.cpp:382 already only reaches here
	// when PPU_hook != nullptr, but the in-loop guard turns the
	// indirect call into a predictable conditional branch instead of
	// an unconditional call-then-bail.
	if (PPU_hook) [[unlikely]] PPU_hook(0x2000 | (RefreshAddr & 0xfff));
#endif

#ifdef PPUT_MMC5SP
	cc = MMC5HackExNTARAMPtr[0x3c0 + (xs >> 2) + ((ys & 0x1C) << 1)];
	cc = ((cc >> ((xs & 2) + ((ys & 0x2) << 1))) & 3);
#else
	#ifdef PPUT_MMC5CHR1
		cc = (MMC5HackExNTARAMPtr[RefreshAddr & 0x3ff] & 0xC0) >> 6;
	#else
		cc = C[0x3c0 + (zz >> 2) + ((RefreshAddr & 0x380) >> 4)];	// Fetch attribute table byte.
		cc = ((cc >> ((zz & 2) + ((RefreshAddr & 0x40) >> 4))) & 3);
	#endif
#endif

atlatch >>= 2;
atlatch |= cc << 2;

pshift[0] <<= 8;
pshift[1] <<= 8;

#ifdef PPUT_MMC5SP
	C = MMC5HackVROMPTR + vadr;
	C += ((MMC5HackSPPage & 0x3f & MMC5HackVROMMask) << 12);
#else
	#ifdef PPUT_MMC5CHR1
		C = MMC5HackVROMPTR;
		C += (((MMC5HackExNTARAMPtr[RefreshAddr & 0x3ff]) & 0x3f & MMC5HackVROMMask) << 12) + (vadr & 0xfff);
		C += (MMC50x5130 & 0x3) << 18; //11-jun-2009 for kuja_killer
	#elif defined(PPUT_MMC5)
		C = MMC5BGVRAMADR(vadr);
	#else

	#ifdef PPU_VRC5FETCH
	if(tmpd & 0x40)
		C = CHRptr[0] + vadr;
	else
		C = VRAMADR(vadr);
	#else
		C = VRAMADR(vadr);
#endif

	#endif
#endif

#ifdef PPUT_HOOK
	if (PPU_hook) [[unlikely]] PPU_hook(vadr);
#endif

#ifdef PPU_BGFETCH
	if (RefreshAddr & 1) {
		if(ScreenON)
			RENDER_LOGP(C + 8);
		pshift[0] |= C[8];
		pshift[1] |= C[8];
	} else {
		if(ScreenON)
			RENDER_LOGP(C);
		pshift[0] |= C[0];
		pshift[1] |= C[0];
	}
#else
	#ifdef PPU_VRC5FETCH
	pshift[0] |= C[0];
	if(tmpd & 0x40)
		pshift[1] |= (tmpd & 0x80) ? 0xFF : 0x00;
	else
		pshift[1] |= C[8];
	#else
	if(ScreenON)
		RENDER_LOGP(C);
	pshift[0] |= C[0];
	if(ScreenON)
		RENDER_LOGP(C + 8);
	pshift[1] |= C[8];
	#endif
#endif

if ((RefreshAddr & 0x1f) == 0x1f)
	RefreshAddr ^= 0x41F;
else
	RefreshAddr++;

#ifdef PPUT_HOOK
	// hotfix2 P1-5 (INLINE-2): guard the indirect call so the compiler
	// can hoist the null-check and skip it on non-hooked paths. The
	// dispatcher in ppu_rendering.cpp:382 already only reaches here
	// when PPU_hook != nullptr, but the in-loop guard turns the
	// indirect call into a predictable conditional branch instead of
	// an unconditional call-then-bail.
	if (PPU_hook) [[unlikely]] PPU_hook(0x2000 | (RefreshAddr & 0xfff));
#endif

