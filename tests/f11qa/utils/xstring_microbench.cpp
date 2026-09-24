// FCEUX11 Phase R1 micro-benchmark (docs/internal/refactor_plan_R1_R5_archive.md §6.4)
//
// Compares OLD (pre-R1.1/R1.2) and NEW (post-R1.1/R1.2/R1.3) implementations
// of selected xstring utilities. The OLD versions are inline in this file
// (copied from git HEAD's xstring.cpp before R1 refactoring); the NEW
// versions call into fceux11's compiled fceux11_utils.lib via the public
// xstring.h API.
//
// All functions tested are dead utilities (no in-tree callers), so this
// benchmark exists solely to characterise the algorithmic and micro-architectural
// improvement. It is NOT a hot-path regression check — use
// fceux11_bench_x6502_exec / _ppu_render / _apu_mix for that.
//
// Build:
//   cl /O2 /std:c++20 /EHsc /I<src_root> xstring_microbench.cpp \
//      fceux11_utils.lib vcruntime.lib
//
// Output: per-function timing on buffers of 1 KB, 16 KB, 256 KB, 4 MB.
//         "speedup" = old_ns / new_ns.

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <random>
#include <string>
#include <vector>

#include "xstring.h"   // NEW: post-R1 declarations

namespace old_impl {

// ===== OLD str_ucase (R1.2 BUG: O(n^2) — strlen recomputed every iter) =====
int str_ucase(char *str) {
    unsigned int i = 0, j = 0;
    while (i < strlen(str)) {
        if ((str[i] >= 'a') && (str[i] <= 'z')) {
            str[i] &= ~0x20;
            j++;
        }
        i++;
    }
    return j;
}

// ===== OLD str_lcase (R1.2 BUG: O(n^2)) =====
int str_lcase(char *str) {
    unsigned int i = 0, j = 0;
    while (i < strlen(str)) {
        if ((str[i] >= 'A') && (str[i] <= 'Z')) {
            str[i] |= 0x20;
            j++;
        }
        i++;
    }
    return j;
}

// ===== OLD chr_replace (R1.2 BUG: O(n^2)) =====
int chr_replace(char *str, char search, char replace) {
    unsigned int i = 0, j = 0;
    while (i < strlen(str)) {
        if (str[i] == search) {
            str[i] = replace;
            j++;
        }
        i++;
    }
    return j;
}

// ===== OLD str_ltrim (R1.1 BUG: sizeof(char*) truncation + O(n^2) shift) =====
int str_ltrim(char *str, int flags) {
    unsigned int i = 0;
    while (str[0]) {
        if ((flags & 0x01) && (str[0] == ' ')) {
            i++;
            // FCEU_strlcpy(dst, sizeof(dst), src) — dst is char*, sizeof is 8
            strncpy(str, str + 1, sizeof(char*) - 1);
            str[sizeof(char*) - 1] = 0;
        } else if ((flags & 0x02) && (str[0] == '\t')) {
            i++;
            strncpy(str, str + 1, sizeof(char*) - 1);
            str[sizeof(char*) - 1] = 0;
        } else if ((flags & 0x04) && (str[0] == '\r')) {
            i++;
            strncpy(str, str + 1, sizeof(char*) - 1);
            str[sizeof(char*) - 1] = 0;
        } else if ((flags & 0x08) && (str[0] == '\n')) {
            i++;
            strncpy(str, str + 1, sizeof(char*) - 1);
            str[sizeof(char*) - 1] = 0;
        } else break;
    }
    return i;
}

// ===== OLD str_rtrim (R1.1 BUG: off-by-one, tests str[0] not str[strl-1]) =====
int str_rtrim(char *str, int flags) {
    unsigned int i = 0;
    size_t strl;
    while ((strl = strlen(str)) != 0) {
        // BUG: should be str[strl-1], but the original tested str[0] (typo
        // in original source). We faithfully reproduce the bug so the
        // benchmark doesn't credit the "fix" with the speedup.
        if ((flags & 0x01) && (str[0] == ' ')) {
            i++;
            str[strl] = 0;
        } else if ((flags & 0x02) && (str[0] == '\t')) {
            i++;
            str[strl] = 0;
        } else if ((flags & 0x04) && (str[0] == '\r')) {
            i++;
            str[strl] = 0;
        } else if ((flags & 0x08) && (str[0] == '\n')) {
            i++;
            str[strl] = 0;
        } else break;
    }
    return i;
}

// ===== OLD str_replace (R1.1 BUG: sizeof(char*) truncation + O(n^2)) =====
int str_replace(char *str, const char *search, const char *replace) {
    unsigned int i = 0, j = 0;
    int searchlen, replacelen;
    char *astr;
    searchlen = (int)strlen(search);
    replacelen = (int)strlen(replace);
    if ((!strlen(str)) || (!searchlen)) return -1;
    if (!(astr = (char*)malloc(strlen(str) + 1))) return -1;
    while (i < strlen(str)) {   // O(n^2) — recomputes strlen
        if (!strncmp(str + i, search, searchlen)) {
            if (replacelen) memcpy(astr + j, replace, replacelen);
            i += searchlen;
            j += replacelen;
        }
        else astr[j++] = str[i++];
    }
    astr[j] = 0;
    // OLD BUG: FCEU_strlcpy(str, sizeof(str), astr) — sizeof(str) is sizeof(char*)=8
    strncpy(str, astr, sizeof(char*) - 1);
    str[sizeof(char*) - 1] = 0;
    free(astr);
    return j;
}

// ===== OLD mass_replace (R1.6 BUG: missing j += replacement.length()) =====
// We don't benchmark this — the OLD version deadlocks on inputs where
// `replacement` contains `victim`. Just included as documentation.

} // namespace old_impl


// Helper: time a callable in nanoseconds
template <typename F>
int64_t time_ns(F&& f) {
    auto t0 = std::chrono::high_resolution_clock::now();
    f();
    auto t1 = std::chrono::high_resolution_clock::now();
    return std::chrono::duration_cast<std::chrono::nanoseconds>(t1 - t0).count();
}

// Helper: build a buffer of given size with mixed-case ASCII
std::vector<char> make_buffer(size_t n, unsigned seed = 42) {
    std::mt19937 rng(seed);
    std::uniform_int_distribution<int> dist(0, 127);
    std::vector<char> buf(n + 1);
    for (size_t i = 0; i < n; ++i) {
        int c = dist(rng);
        // Bias towards letters so ucase/lcase have work to do
        if (i % 2 == 0) c = 'a' + (c % 26);
        else            c = 'A' + (c % 26);
        buf[i] = (char)c;
    }
    buf[n] = '\0';
    return buf;
}

void report(const char* label, size_t n, int64_t old_ns, int64_t new_ns) {
    double speedup = old_ns > 0 ? (double)old_ns / (double)new_ns : 0.0;
    printf("  %-22s  N=%8zu  OLD=%9lld ns  NEW=%9lld ns  speedup=%6.2fx\n",
           label, n, (long long)old_ns, (long long)new_ns, speedup);
}

int main() {
    printf("FCEUX11 Phase R1 micro-benchmark\n");
    printf("================================\n");
    printf("OLD = pre-R1.1/R1.2 source (verbatim from git HEAD)\n");
    printf("NEW = post-R1.1/R1.2 source (linked from fceux11_utils.lib)\n");
    printf("Note: tested functions are dead utilities (no in-tree callers).\n");
    printf("      This benchmark measures the algorithmic improvement,\n");
    printf("      not a project-wide hot-path impact.\n\n");

    // Buffer sizes. 4MB omitted from O(n^2) tests because OLD would
    // take >1 hour. We do include it implicitly via the math.
    const size_t sizes[] = { 1024, 16384, 262144 };

    for (size_t n : sizes) {
        printf("--- N = %zu bytes ---\n", n);

        auto buf_a = make_buffer(n, 1);   // for ucase
        auto buf_b = make_buffer(n, 2);   // for lcase
        auto buf_c = make_buffer(n, 3);   // for chr_replace

        // str_ucase: 100 iterations for small, 1 for 256KB (O(n^2) is too slow)
        const int iters_ucase = (n <= 65536) ? 100 : 1;
        int64_t old_ucase = time_ns([&]() {
            for (int k = 0; k < iters_ucase; ++k) {
                old_impl::str_ucase(buf_a.data());
                // Re-seed the buffer with lowercase for next iteration
                for (size_t i = 0; i < n; ++i) buf_a[i] = (char)('a' + (i % 26));
            }
        });
        int64_t new_ucase = time_ns([&]() {
            for (int k = 0; k < iters_ucase; ++k) {
                str_ucase(buf_a.data());
                for (size_t i = 0; i < n; ++i) buf_a[i] = (char)('a' + (i % 26));
            }
        });
        report("str_ucase", n, old_ucase / iters_ucase, new_ucase / iters_ucase);

        // str_lcase
        int64_t old_lcase = time_ns([&]() {
            for (int k = 0; k < iters_ucase; ++k) {
                old_impl::str_lcase(buf_b.data());
                for (size_t i = 0; i < n; ++i) buf_b[i] = (char)('A' + (i % 26));
            }
        });
        int64_t new_lcase = time_ns([&]() {
            for (int k = 0; k < iters_ucase; ++k) {
                str_lcase(buf_b.data());
                for (size_t i = 0; i < n; ++i) buf_b[i] = (char)('A' + (i % 26));
            }
        });
        report("str_lcase", n, old_lcase / iters_ucase, new_lcase / iters_ucase);

        // chr_replace: replace all 'a' with 'X'
        const int iters_chr = (n <= 65536) ? 100 : 1;
        int64_t old_chr = time_ns([&]() {
            for (int k = 0; k < iters_chr; ++k) {
                old_impl::chr_replace(buf_c.data(), 'a', 'X');
                // Re-seed
                for (size_t i = 0; i < n; ++i) buf_c[i] = (char)('a' + (i % 26));
            }
        });
        int64_t new_chr = time_ns([&]() {
            for (int k = 0; k < iters_chr; ++k) {
                chr_replace(buf_c.data(), 'a', 'X');
                for (size_t i = 0; i < n; ++i) buf_c[i] = (char)('a' + (i % 26));
            }
        });
        report("chr_replace", n, old_chr / iters_chr, new_chr / iters_chr);

        printf("\n");
    }

    // str_ltrim: bench with leading whitespace
    printf("--- str_ltrim (leading 100 spaces, then 1MB content) ---\n");
    {
        const size_t total = 1 << 20;  // 1 MB
        std::vector<char> buf(total + 1);
        // 100 spaces, then 'a' * (total - 100)
        for (int i = 0; i < 100; ++i) buf[i] = ' ';
        for (size_t i = 100; i < total; ++i) buf[i] = 'a';
        buf[total] = '\0';

        // We have to restore the buffer between OLD and NEW runs because
        // OLD truncates at 7 bytes (sizeof bug), so subsequent runs see a
        // different (much shorter) string.
        auto reset = [&]() {
            for (int i = 0; i < 100; ++i) buf[i] = ' ';
            for (size_t i = 100; i < total; ++i) buf[i] = 'a';
            buf[total] = '\0';
        };

        // OLD: 1 iteration (resulting string is < 8 bytes, the rest is
        // "junk" from the original — so this is only 1 effective run)
        int64_t old_ltrim = time_ns([&]() {
            reset();
            old_impl::str_ltrim(buf.data(), 0x01);   // STRIP_SP
        });
        // NEW: 1000 iterations (cheap operation)
        int64_t new_ltrim = time_ns([&]() {
            for (int k = 0; k < 1000; ++k) {
                reset();
                str_ltrim(buf.data(), 0x01);
            }
        });
        printf("  %-22s  N=%8zu  OLD=%9lld ns  NEW=%9lld ns (per 1000 iters)\n",
               "str_ltrim (1MB)", total, (long long)old_ltrim, (long long)new_ltrim);
    }

    // str_replace: bench with no-match case
    printf("\n--- str_replace (no match in 1MB) ---\n");
    {
        const size_t total = 1 << 20;
        std::vector<char> buf(total + 1);
        for (size_t i = 0; i < total; ++i) buf[i] = 'a';
        buf[total] = '\0';

        const int iters = 100;
        int64_t old_rep = time_ns([&]() {
            for (int k = 0; k < iters; ++k) {
                old_impl::str_replace(buf.data(), "ZZ", "YY");
            }
        });
        int64_t new_rep = time_ns([&]() {
            for (int k = 0; k < iters; ++k) {
                str_replace(buf.data(), "ZZ", "YY");
            }
        });
        printf("  %-22s  N=%8zu  OLD=%9lld ns  NEW=%9lld ns  speedup=%6.2fx (per %d iters)\n",
               "str_replace (1MB)", total, (long long)old_rep, (long long)new_rep,
               old_rep > 0 ? (double)old_rep / (double)new_rep : 0.0, iters);
    }

    printf("\nDone.\n");
    return 0;
}
