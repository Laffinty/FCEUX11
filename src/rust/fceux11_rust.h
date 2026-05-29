#ifndef FCEUX11_RUST_H
#define FCEUX11_RUST_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* === CRC32 (v0.2.1) === */
uint32_t fceux11_rust_crc32(uint32_t crc, const uint8_t *buf, uint32_t len);

/* === MD5 (v0.2.2) === */
struct md5_context;

void fceux11_rust_md5_starts(struct md5_context *ctx);
void fceux11_rust_md5_update(struct md5_context *ctx, uint8_t *input, uint32_t length);
void fceux11_rust_md5_finish(struct md5_context *ctx, uint8_t digest[16]);
char *fceux11_rust_md5_asciistr(uint8_t md5[16]);

/* === General Utilities (v0.2.4) === */
uint32_t fceux11_rust_uppow2(uint32_t n);

/* === GUID (v0.2.3) === */
struct FceuGuid {
    uint8_t data[16];
};

void fceux11_rust_guid_new(struct FceuGuid *guid);
char *fceux11_rust_guid_to_string(const struct FceuGuid *guid);
void fceux11_rust_guid_scan(struct FceuGuid *guid, const char *str);

/* === Wave Audio Export (v0.2.5) === */
int fceux11_rust_wave_begin(const char *path, uint32_t sample_rate);
int fceux11_rust_wave_running(void);
int64_t fceux11_rust_wave_write(const int16_t *buffer, int32_t count);
int32_t fceux11_rust_wave_end(void);

/* === OS Utilities (v0.2.6) === */
int fceux11_rust_mkdir(const char *path);
int fceux11_rust_mkpath(const char *path);
int fceux11_rust_file_exists(const char *filepath);
int fceux11_rust_msleep(int ms);

/* === Unicode Conversion (v0.2.7) === */
int fceux11_rust_convert_utf8_to_utf16(const uint8_t **sourceStart, const uint8_t *sourceEnd, uint16_t **targetStart, uint16_t *targetEnd, int flags);
int fceux11_rust_convert_utf16_to_utf8(const uint16_t **sourceStart, const uint16_t *sourceEnd, uint8_t **targetStart, uint8_t *targetEnd, int flags);
int fceux11_rust_convert_utf8_to_utf32(const uint8_t **sourceStart, const uint8_t *sourceEnd, uint32_t **targetStart, uint32_t *targetEnd, int flags);
int fceux11_rust_convert_utf32_to_utf8(const uint32_t **sourceStart, const uint32_t *sourceEnd, uint8_t **targetStart, uint8_t *targetEnd, int flags);
int fceux11_rust_convert_utf16_to_utf32(const uint16_t **sourceStart, const uint16_t *sourceEnd, uint32_t **targetStart, uint32_t *targetEnd, int flags);
int fceux11_rust_convert_utf32_to_utf16(const uint32_t **sourceStart, const uint32_t *sourceEnd, uint16_t **targetStart, uint16_t *targetEnd, int flags);
int fceux11_rust_is_legal_utf8_sequence(const uint8_t *source, const uint8_t *sourceEnd);

/* === Time Stamp (v0.2.8) === */
uint64_t fceux11_rust_timestamp_now(void);
uint64_t fceux11_rust_timestamp_freq(void);
int32_t fceux11_rust_timestamp_init(void);

/* === Profiler (v0.2.9) === */
void *fceux11_rust_profiler_map_create(void);
void fceux11_rust_profiler_map_destroy(void *handle);
int fceux11_rust_profiler_map_add_record(void *handle, const char *file, int line, const char *func, const char *comment, void *rec);
void fceux11_rust_profiler_map_push_stack(void *handle, void *rec);
void fceux11_rust_profiler_map_pop_stack(void *handle, void *rec);
void *fceux11_rust_profiler_map_iterate_begin(void *handle);
void *fceux11_rust_profiler_map_iterate_next(void *handle);
int fceux11_rust_profiler_mgr_add(void *cpp_ptr);
int fceux11_rust_profiler_mgr_remove(void *cpp_ptr, int should_destroy);
void fceux11_rust_profiler_mgr_clear(void);

/* === Audio Filter (v0.2.10) === */
struct FceuFilterState;
struct FceuFilterState *fceux11_rust_filter_state_create(void);
void fceux11_rust_filter_state_destroy(struct FceuFilterState *state);
void fceux11_rust_filter_make(struct FceuFilterState *state, int32_t rate, int soundq, int is_pal, double ntsc_cpu, double pal_cpu);
int32_t fceux11_rust_filter_neo(struct FceuFilterState *state, int32_t *in, int32_t *out, uint32_t inlen, int32_t *leftover, int soundq, int lowpass, void (*neo_fill)(int32_t *, int32_t), int32_t snd_rate, int32_t sound_volume);
void fceux11_rust_filter_sexy(struct FceuFilterState *state, int32_t *in, int32_t *out, int32_t count, int32_t snd_rate, int32_t sound_volume, int soundq);
void fceux11_rust_filter_sexy2(struct FceuFilterState *state, int32_t *in, int32_t count);

/* === Palette (v0.2.11) === */
struct Pal {
    uint8_t r, g, b;
};

void fceux11_rust_palette_calc_ntsc(int32_t tint, int32_t hue, struct Pal *out);
void fceux11_rust_palette_apply_deemphasis(const struct Pal *src, struct Pal *dst);
void fceux11_rust_palette_make_grayscale(const struct Pal *src, struct Pal *dst);
void fceux11_rust_palette_draw_control_bars(uint8_t *xbuf, int32_t width, int32_t which);

#ifdef __cplusplus
}
#endif

#endif /* FCEUX11_RUST_H */
