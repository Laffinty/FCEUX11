#ifndef FCEUX11_RUST_H
#define FCEUX11_RUST_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

uint32_t fceux11_rust_crc32(uint32_t crc, const uint8_t *buf, uint32_t len);

#ifdef __cplusplus
}
#endif

#endif /* FCEUX11_RUST_H */
