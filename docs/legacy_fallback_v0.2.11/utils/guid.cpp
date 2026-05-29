/// \file
/// \brief GUID wrapper — delegates to Rust when FCEUX11_RUST_ENABLED,
/// otherwise falls back to the original C++ implementation.
///
/// Phase 2 (v0.2.3): Rust module provides UUID v4 generation via the `uuid` crate,
/// replacing the original `rand()`-based GUID generation with RFC 4122 compliant UUIDs.

#include "guid.h"

#ifdef FCEUX11_RUST_ENABLED
#include "../rust/fceux11_rust.h"

void FCEU_Guid::newGuid()
{
    fceux11_rust_guid_new(reinterpret_cast<FceuGuid*>(this));
}

std::string FCEU_Guid::toString()
{
    const char* ptr = fceux11_rust_guid_to_string(reinterpret_cast<const FceuGuid*>(this));
    if (!ptr) return std::string();
    return std::string(ptr);
}

FCEU_Guid FCEU_Guid::fromString(std::string str)
{
    FCEU_Guid ret;
    ret.scan(str);
    return ret;
}

uint8 FCEU_Guid::hexToByte(char** ptrptr)
{
    char a = toupper(**ptrptr);
    (*ptrptr)++;
    char b = toupper(**ptrptr);
    (*ptrptr)++;
    if (a >= 'A') a = a - 'A' + 10;
    else a -= '0';
    if (b >= 'A') b = b - 'A' + 10;
    else b -= '0';
    return ((unsigned char)a << 4) | (unsigned char)b;
}

void FCEU_Guid::scan(std::string& str)
{
    fceux11_rust_guid_scan(reinterpret_cast<FceuGuid*>(this), str.c_str());
}

#else // !FCEUX11_RUST_ENABLED — original C++ implementation

#include <stdlib.h>

void FCEU_Guid::newGuid()
{
    for (int i = 0; i < size; i++)
        data[i] = rand();
}

std::string FCEU_Guid::toString()
{
    char buf[37];
    sprintf(buf, "%08X-%04X-%04X-%04X-%02X%02X%02X%02X%02X%02X",
        FCEU_de32lsb(data), FCEU_de16lsb(data + 4), FCEU_de16lsb(data + 6), FCEU_de16lsb(data + 8), data[10], data[11], data[12], data[13], data[14], data[15]);
    return std::string(buf);
}

FCEU_Guid FCEU_Guid::fromString(std::string str)
{
    FCEU_Guid ret;
    ret.scan(str);
    return ret;
}

uint8 FCEU_Guid::hexToByte(char** ptrptr)
{
    char a = toupper(**ptrptr);
    (*ptrptr)++;
    char b = toupper(**ptrptr);
    (*ptrptr)++;
    if (a >= 'A') a = a - 'A' + 10;
    else a -= '0';
    if (b >= 'A') b = b - 'A' + 10;
    else b -= '0';
    return ((unsigned char)a << 4) | (unsigned char)b;
}

void FCEU_Guid::scan(std::string& str)
{
    char* endptr = (char*)str.c_str();
    FCEU_en32lsb(data, strtoul(endptr, &endptr, 16));
    FCEU_en16lsb(data + 4, strtoul(endptr + 1, &endptr, 16));
    FCEU_en16lsb(data + 6, strtoul(endptr + 1, &endptr, 16));
    FCEU_en16lsb(data + 8, strtoul(endptr + 1, &endptr, 16));
    endptr++;
    for (int i = 0; i < 6; i++)
        data[10 + i] = hexToByte(&endptr);
}

#endif // !FCEUX11_RUST_ENABLED