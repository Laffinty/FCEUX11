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

#ifdef __cplusplus
}
#endif

#endif /* FCEUX11_RUST_H */
