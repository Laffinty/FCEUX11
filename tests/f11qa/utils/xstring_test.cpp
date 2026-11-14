// FCEUX11 v2.0.4 — V16: StringToBytes "0X"-prefix byte-count fix (hex
// over-read), C++ side.
//
// Plan: docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md §V16.
// Upstream: issue #857, fixed by commit 683c3f0 (2026-11-01) with
//   const size_t bytesAvailable = (str.size() - 2) / 2; // exclude the "0X" prefix
//
// Contract under test: for a "0X"-prefixed hex string, StringToBytes treats
// exactly the digits after the "0X" prefix as payload. The old code computed
// bytesAvailable = str.size()/2, counting the two prefix characters as one
// phantom byte, so for an even-length value with a large-enough `len` the
// last loop iteration read cstr[N] (the NUL terminator, in bounds) AND
// cstr[N+1] — one byte past the string (upstream #857: "one byte past the
// string's NUL terminator"). Odd-length values only ever reached the NUL,
// which is why they never tripped ASan.
//
// Reachability note (plan §V16 现状): every current caller of the hex branch
// is dead code — MovieData::installValue (src/movie_fm2.cpp) has no callers,
// and src/drivers/common/config.cpp is not in src/CMakeLists.txt's build
// list. The live FM2 parser is the Rust one (fceux11-formats decode_hex,
// which rejects a "0X" prefix outright). This test exists to (a) lock the
// corrected decode semantics, (b) make the over-read observable again if the
// dead path is ever revived, and (c) pin the no-prefix atoi branch.
//
// Window-clamp note (discipline 11): the over-read requires
// amt = min(len, bytesAvailable) to exceed N/2 — i.e. len >= str.size()/2.
// The plan's acceptance form StringToBytes("0XABAB", buf, 2) does NOT
// over-read even pre-fix (amt clamps to 2 <= N/2); it only locks decode
// semantics. The red legs below therefore use len=3 ("0XABAB") and len=20
// (a 34-char heap string) to actually open the window.
//
// Expected values are hand-assembled constants (discipline 5): each expected
// byte sequence is written out by hand from the hex digits of the input
// string (or from the documented atoi + little-endian-store rule), never
// computed by calling the implementation.
//
// Mutation legs (discipline 3) — see the bottom-of-file comment.

#include <cstdio>
#include <cstring>
#include <string>

#include "utils/xstring.h"

static const unsigned char SENTINEL = 0x5A;

static void print_bytes(const char* tag, const unsigned char* d, int n) {
    printf("      %s =", tag);
    for (int i = 0; i < n; ++i) printf(" %02X", (unsigned)d[i]);
    printf("\n");
}

// Assertion 1 — the plan's POC shape ("0XABAB", len=2): decodes the two hex
// digits exactly. GREEN both before and after the fix (len clamps amt to
// 2 <= N/2, so no over-read happens here); this locks DECODE SEMANTICS, not
// the byte count. The red legs are assertions 4 and 5 below.
static bool test_0x_short_value_decodes_exactly() {
    printf("[test] StringToBytes \"0XABAB\" len=2 decodes exactly ... \n");

    // Hand-assembled from the digits after the prefix: 'A','B' -> 0xAB each.
    static const unsigned char EXPECTED[2] = {0xAB, 0xAB};
    unsigned char buf[2] = {0x11, 0x22};

    if (!StringToBytes(std::string("0XABAB"), buf, 2)) {
        printf("   FAIL: StringToBytes returned false\n");
        return false;
    }
    if (std::memcmp(buf, EXPECTED, sizeof(EXPECTED)) != 0) {
        printf("   FAIL: decoded bytes differ from the hand-derived constant\n");
        print_bytes("returned", buf, 2);
        print_bytes("expected", EXPECTED, 2);
        return false;
    }
    printf("   OK:   decoded {0xAB, 0xAB}\n");
    return true;
}

// Assertion 2 — positive control: a well-formed "0X" string with an
// exact-size buffer decodes every digit pair (the live path installValue
// would drive via HexStringToBytesLength).
static bool test_0x_wellformed_positive_control() {
    printf("[test] StringToBytes well-formed hex positive control ... \n");

    // "0X0123456789ABCDEF": hand-assembled pair by pair from the digits
    // after the prefix.
    static const unsigned char EXPECTED[8] = {
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
    };
    unsigned char buf[8] = {0};

    if (!StringToBytes(std::string("0X0123456789ABCDEF"), buf, 8)) {
        printf("   FAIL: StringToBytes returned false\n");
        return false;
    }
    if (std::memcmp(buf, EXPECTED, sizeof(EXPECTED)) != 0) {
        printf("   FAIL: decoded bytes differ from the hand-derived constant\n");
        print_bytes("returned", buf, 8);
        print_bytes("expected", EXPECTED, 8);
        return false;
    }
    printf("   OK:   decoded 8 bytes exactly\n");
    return true;
}

// Assertion 3 — positive control for the NO-prefix branch: without "0X" and
// with len==2, StringToBytes routes through atoi ("4660" == 0x1234) and
// stores *(unsigned short*)data, i.e. little-endian bytes 34 12 on the
// platforms this repo builds for. Pins that the V16 change did not disturb
// the neighboring branches.
static bool test_no_prefix_atoi_positive_control() {
    printf("[test] StringToBytes no-prefix atoi positive control ... \n");

    // Hand-derived from the contract, not from the implementation:
    // atoi("4660") == 0x1234, little-endian short store -> 34 12.
    static const unsigned char EXPECTED[2] = {0x34, 0x12};
    unsigned char buf[2] = {0x11, 0x22};

    if (!StringToBytes(std::string("4660"), buf, 2)) {
        printf("   FAIL: StringToBytes returned false\n");
        return false;
    }
    if (std::memcmp(buf, EXPECTED, sizeof(EXPECTED)) != 0) {
        printf("   FAIL: decoded bytes differ from the hand-derived constant\n");
        print_bytes("returned", buf, 2);
        print_bytes("expected", EXPECTED, 2);
        return false;
    }
    printf("   OK:   decoded {0x34, 0x12}\n");
    return true;
}

// Assertion 4 — THE V16 RED LEG (primary, deterministic under ASan).
// "0X" + 32 hex digits: L=34, N=32, correct bytesAvailable=(34-2)/2=16,
// old = 34/2 = 17. len=20 exceeds both, so amt clamps to bytesAvailable —
// 16 after the fix, 17 before. Pre-fix the 17th byte is derived from the
// NUL (cstr[32] = str[34]) AND the loop reads cstr[33] = str[35]: one byte
// past the string data. The 34-char string (32 hex digits after the prefix)
// forces a heap-allocated std::string (> 15-char SSO threshold), and MSVC
// STL's string ASan
// annotations poison the bytes after the NUL — so under build-asan this
// read is a deterministic container/heap-buffer-overflow READ of size 1.
// Post-fix: 16 decoded bytes, sentinels at [16..23] untouched.
static bool test_0x_prefix_does_not_overcount_bytes() {
    printf("[test] StringToBytes \"0X\"+32 digits len=20: no phantom byte ... \n");

    // 34 chars total, heap-allocated. Hand-assembled expectation: the same
    // 8 bytes as assertion 2, twice.
    static const char* const HEX32 = "0X0123456789ABCDEF0123456789ABCDEF";
    static const unsigned char EXPECTED[16] = {
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
    };
    unsigned char buf[24];
    std::memset(buf, SENTINEL, sizeof(buf));

    std::string s(HEX32);
    if (!StringToBytes(s, buf, 20)) {
        printf("   FAIL: StringToBytes returned false\n");
        return false;
    }
    if (std::memcmp(buf, EXPECTED, sizeof(EXPECTED)) != 0) {
        printf("   FAIL: decoded bytes differ from the hand-derived constant\n");
        print_bytes("returned", buf, 16);
        print_bytes("expected", EXPECTED, 16);
        return false;
    }
    for (int i = 16; i < 24; ++i) {
        if (buf[i] != SENTINEL) {
            printf("   FAIL: buf[%d] was written (0x%02X) - bytesAvailable "
                   "over-counts (the pre-fix behavior; this is also the ASan "
                   "red leg)\n", i, (unsigned)buf[i]);
            print_bytes("buffer", buf, 24);
            return false;
        }
    }
    printf("   OK:   decoded 16 bytes, sentinels [16..23] intact\n");
    return true;
}

// Assertion 5 — the same defect on the plan's POC string, with the len that
// actually opens the window: len=3 >= L/2=3 for "0XABAB". Post-fix amt
// clamps to (6-2)/2 = 2 and the sentinels at [2..3] survive; pre-fix amt=3
// writes a NUL-derived garbage byte into buf[2] after reading cstr[4] (NUL)
// and cstr[5] (one byte past the string). Secondary ASan red leg: the read
// lands in SSO slack, which fires under ASan when MSVC STL annotates the
// small buffer; without ASan the buf[2] sentinel check is only
// probabilistically red (garbage byte could collide with the sentinel).
static bool test_0x_plan_poc_len3() {
    printf("[test] StringToBytes \"0XABAB\" len=3: sentinels survive ... \n");

    static const unsigned char EXPECTED[2] = {0xAB, 0xAB};
    unsigned char buf[4] = {SENTINEL, SENTINEL, SENTINEL, SENTINEL};

    if (!StringToBytes(std::string("0XABAB"), buf, 3)) {
        printf("   FAIL: StringToBytes returned false\n");
        return false;
    }
    if (std::memcmp(buf, EXPECTED, sizeof(EXPECTED)) != 0) {
        printf("   FAIL: first two bytes differ from the hand-derived constant\n");
        print_bytes("buffer", buf, 4);
        return false;
    }
    for (int i = 2; i < 4; ++i) {
        if (buf[i] != SENTINEL) {
            printf("   FAIL: buf[%d] was written (0x%02X) - the pre-fix "
                   "byte count read past the NUL\n", i, (unsigned)buf[i]);
            print_bytes("buffer", buf, 4);
            return false;
        }
    }
    printf("   OK:   decoded {0xAB, 0xAB}, sentinels [2..3] intact\n");
    return true;
}

int main() {
    printf("=== V16 xstring_test ===\n");

    bool failed = false;
    if (!test_0x_short_value_decodes_exactly()) failed = true;
    if (!test_0x_wellformed_positive_control()) failed = true;
    if (!test_no_prefix_atoi_positive_control()) failed = true;
    if (!test_0x_prefix_does_not_overcount_bytes()) failed = true;
    if (!test_0x_plan_poc_len3()) failed = true;

    printf("\n=== Test Complete ===\n");
    if (failed) {
        printf("\nRESULT: FAILED\n");
        return 1;
    }
    printf("\nRESULT: PASSED - V16 StringToBytes byte-count fix holds\n");
    return 0;
}

// ---------------------------------------------------------------------------
// Mutation legs (discipline 3) — for the main session to run by hand.
//
// Red leg (ASan build `build-asan`, RelWithDebInfo): revert the hex branch
// of StringToBytes in src/utils/xstring.cpp to the pre-V16 form, rebuild,
// run ctest -R xstring_test. Verbatim old/new:
//
//   OLD (pre-V16, upstream defect #857):
// 		int amt = len;
// 		int bytesAvailable = str.size()/2;
// 		if(bytesAvailable < amt)
// 			amt = bytesAvailable;
//
//   NEW (v2.0.4 V16, upstream commit 683c3f0):
// 		int amt = len;
// 		const size_t bytesAvailable = (str.size() - 2) / 2;
// 		if((int)bytesAvailable < amt)
// 			amt = (int)bytesAvailable;
//
//   (Only the byte-count lines differ; the comparison was recast into the
//   int domain to keep the old write-nothing behavior for negative len and
//   avoid the signed/unsigned comparison warning.)
//
// Expected result with OLD restored:
//   - test_0x_prefix_does_not_overcount_bytes: heap-buffer-overflow /
//     container-overflow READ of size 1 inside StringToBytes (MSVC STL
//     string annotations poison the bytes after the NUL terminator; the
//     34-char string is heap-allocated). Without ASan the same test is
//     still red through the buf[16] sentinel check (probabilistic — the
//     out-of-bounds byte would have to be 0x5A to mask it).
//   - test_0x_plan_poc_len3: red through the buf[2] sentinel check; under
//     ASan it additionally trips when the SSO slack is annotated.
//   - Assertions 1-3 stay GREEN with OLD restored, by design: their len
//     never opens the over-read window (amt <= N/2), so they are
//     decode-semantics locks, not byte-count locks. A run where ALL five go
//     red would mean the fix broke decoding, not that the mutation was
//     caught.
//
// Restore the NEW form (refresh the file so Ninja rebuilds it — a header-
// only edit can be skipped by the build) and the suite returns green.
// ---------------------------------------------------------------------------
