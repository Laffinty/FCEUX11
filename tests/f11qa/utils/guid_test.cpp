// FCEUX11 v2.0.4 — V19: GUID parse-failure zeroing, C++ half.
//
// Plan: docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md §V19.
//
// Contract under test: FCEU_Guid::fromString on garbage input returns a
// DETERMINISTIC all-zero GUID (never stack garbage), and the same input
// gives the same result on repeated calls. Before V19 `fromString` default-
// initialized `FCEU_Guid ret;` — an aggregate with no user-provided ctor —
// so a failed parse returned whatever was on the stack; that garbage flowed
// into movie GUID comparisons (movie_playback.cpp:186) and into saved .fm2
// files (movie_fm2.cpp:327).
//
// Expected values are hand-assembled constants (discipline 5): all-zero is
// 16 x 0x00; the positive-control bytes are derived by hand from the
// documented "big-endian parse per group, little-endian store" rule, never
// computed by calling the implementation.
//
// Mutation legs (discipline 3) — see §V19 and the bottom-of-file comment:
// with the Rust-side zeroing (v2.0.4 V24) in place, either layer alone
// masks a revert of the other, so the C++-leg mutation is a DOUBLE mutation
// (C++ `ret{}` -> `ret` AND remove a `guid.data = [0u8; 16];` from a
// guid.rs failure branch); both assertions below must then go red.

#include <cstdio>
#include <cstring>
#include <string>

#include "utils/guid.h"

static const uint8 GUID_ALL_ZERO[16] = {0};

// Positive control, hand-derived (NOT from the implementation):
//   "01234567-89ab-cdef-0123-456789abcdef"
//   group 1 "01234567"     -> 0x01234567 -> LE 67 45 23 01
//   group 2 "89ab"         -> 0x89ab     -> LE ab 89
//   group 3 "cdef"         -> 0xcdef     -> LE ef cd
//   group 4 "0123"         -> 0x0123     -> LE 23 01
//   group 5 "456789abcdef" -> 45 67 89 ab cd ef
// (Same constant as the Rust-side lock test test_guid_scan_byte_layout_unchanged
// in src/rust/crates/fceux11-utils/src/guid.rs.)
static const uint8 GUID_VALID_EXPECTED[16] = {
    0x67, 0x45, 0x23, 0x01, 0xAB, 0x89, 0xEF, 0xCD,
    0x23, 0x01, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
};
static const char* GUID_VALID_TEXT = "01234567-89ab-cdef-0123-456789abcdef";

static bool data_is(const FCEU_Guid& g, const uint8 expected[16]) {
    return std::memcmp(g.data, expected, 16) == 0;
}

static void print_data(const char* tag, const uint8 d[16]) {
    printf("      %s =", tag);
    for (int i = 0; i < 16; ++i) printf(" %02X", (unsigned)d[i]);
    printf("\n");
}

// V19 acceptance 1, assertion A: garbage input -> all-zero GUID.
// Covers the Rust failure branches reachable from C++: too short ("0"),
// empty, arbitrary junk, a truncated 36-char string, and a non-UTF-8 byte
// sequence (std::string can carry those; the Rust scan rejects them).
static bool test_garbage_input_yields_all_zero() {
    printf("[test] fromString garbage -> all-zero ... \n");

    struct Case {
        const char* name;
        std::string input;
    };
    const Case cases[] = {
        {"\"0\"              ", std::string("0")},
        {"empty              ", std::string("")},
        {"junk \"not-a-guid\" ", std::string("not-a-guid")},
        {"truncated 35 chars ", std::string("01234567-89ab-cdef-0123-456789abcde")},
        {"non-UTF-8 (0xFF)   ", std::string("01234567-89ab-cdef-0123-456789abcde\xff")},
    };

    bool ok = true;
    for (const Case& c : cases) {
        FCEU_Guid g = FCEU_Guid::fromString(c.input);
        if (!data_is(g, GUID_ALL_ZERO)) {
            printf("   FAIL: input %s (len %zu)\n", c.name, c.input.size());
            print_data("returned", g.data);
            ok = false;
        } else {
            printf("   OK:   input %s -> all-zero\n", c.name);
        }
    }
    return ok;
}

// V19 acceptance 1, assertion B: the same garbage input gives the same
// result on repeated calls. A valid parse is interleaved between the two
// calls so that, if BOTH zeroing layers were ever broken again, the two
// results would carry DIFFERENT stack residue (the second call's stack
// frame is dirtied by the valid parse) instead of plausibly identical
// leftovers — this keeps assertion B able to go red on the mutation leg.
static bool test_repeated_calls_agree() {
    printf("[test] fromString repeated calls agree ... \n");

    const std::string garbage = "0";
    FCEU_Guid a = FCEU_Guid::fromString(garbage);

    // Stack-dirtying interleaving (see comment above).
    FCEU_Guid valid = FCEU_Guid::fromString(GUID_VALID_TEXT);

    FCEU_Guid b = FCEU_Guid::fromString(garbage);

    if (!(a == b)) {
        printf("   FAIL: same input, different results\n");
        print_data("first ", a.data);
        print_data("second", b.data);
        return false;
    }
    printf("   OK:   repeated calls agree (interleaved valid parse in between)\n");

    // Both must still be the all-zero GUID — agreement alone would also
    // hold for two identical garbage blobs.
    if (!data_is(a, GUID_ALL_ZERO) || !data_is(b, GUID_ALL_ZERO)) {
        printf("   FAIL: repeated-call results are not all-zero\n");
        print_data("first ", a.data);
        print_data("second", b.data);
        return false;
    }
    printf("   OK:   both repeated-call results are all-zero\n");
    return true;
}

// V19 acceptance 2 (C++-side positive control): a valid 36-char UUID parses
// to the hand-assembled expected bytes and round-trips through toString().
// Without this, a hypothetical fromString that wrote nothing at all would
// pass the two tests above trivially (all-zero object == all-zero expected).
static bool test_valid_uuid_roundtrip() {
    printf("[test] fromString valid UUID roundtrip ... \n");

    FCEU_Guid g = FCEU_Guid::fromString(GUID_VALID_TEXT);
    if (!data_is(g, GUID_VALID_EXPECTED)) {
        printf("   FAIL: parsed bytes differ from the hand-derived constant\n");
        print_data("returned", g.data);
        print_data("expected", GUID_VALID_EXPECTED);
        return false;
    }
    printf("   OK:   parsed bytes match the hand-derived constant\n");

    std::string out = g.toString();
    if (out != GUID_VALID_TEXT) {
        printf("   FAIL: toString roundtrip: \"%s\" != \"%s\"\n",
               out.c_str(), GUID_VALID_TEXT);
        return false;
    }
    printf("   OK:   toString roundtrip: \"%s\"\n", out.c_str());
    return true;
}

int main() {
    printf("=== V19 guid_test ===\n");

    bool failed = false;
    if (!test_garbage_input_yields_all_zero()) failed = true;
    if (!test_repeated_calls_agree()) failed = true;
    if (!test_valid_uuid_roundtrip()) failed = true;

    printf("\n=== Test Complete ===\n");
    if (failed) {
        printf("\nRESULT: FAILED\n");
        return 1;
    }
    printf("\nRESULT: PASSED - V19 GUID parse-failure zeroing holds\n");
    return 0;
}

// ---------------------------------------------------------------------------
// Mutation legs (discipline 3) — for the main session to run by hand:
//
// The two zeroing layers (Rust failure-branch writes, C++ `ret{}`) overlap
// on every C++-reachable input, so each masks a single-leg revert of the
// other through fromString:
//   - Revert ONLY C++ (`FCEU_Guid ret{};` -> `FCEU_Guid ret;`): Rust still
//     zeroes every failure branch => test_garbage_input_yields_all_zero and
//     test_repeated_calls_agree stay GREEN (masked). This is expected and
//     is NOT a test defect.
//   - Revert ONLY Rust (delete one `guid.data = [0u8; 16];` from a guid.rs
//     failure branch): C++ `ret{}` still supplies the zeros => GREEN
//     (masked). The Rust-side lock tests in guid.rs (0xA5-prefill style)
//     are the single-leg detector for that layer, already in place.
//
// Double mutation (both reverts at once) — every assertion below must turn
// RED:
//   1. guid.cpp: `FCEU_Guid ret{};` -> `FCEU_Guid ret;`
//   2. src/rust/crates/fceux11-utils/src/guid.rs: delete the
//      `guid.data = [0u8; 16];` in the `clean.len() != 32` branch
//      (temporary mutation; refresh the file mtime afterwards so cargo
//      rebuilds — AGENTS.md twelve-pitfalls ① in v2.0.3 r4/r5 notes).
//   3. cargo test -p fceux11-utils --lib  => the Rust lock tests
//      (test_guid_scan_invalid_length_zeroes_guid, ...) go red: the
//      single-leg Rust detector.
//   4. Build tests and run ctest -R guid_test => BOTH garbage assertions
//      go red:
//      - test_garbage_input_yields_all_zero: after the interleaved valid
//        parse and printf calls, the stack under fromString is dirtied with
//        non-zero residue, so `ret` (uninitialized) cannot plausibly come
//        back all-zero.
//      - test_repeated_calls_agree: the two garbage calls see different
//        stack residue (the second frame sits on top of the valid parse's
//        writes) => a != b.
//      Restore either layer and the suite returns green.
// ---------------------------------------------------------------------------
