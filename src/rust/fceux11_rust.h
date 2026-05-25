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

#ifdef __cplusplus
}
#endif

#endif /* FCEUX11_RUST_H */
