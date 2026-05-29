/// \file
/// \brief GUID wrapper — delegates to Rust.
///
/// Phase 2 (v0.2.3): Rust module provides UUID v4 generation via the `uuid` crate,
/// replacing the original `rand()`-based GUID generation with RFC 4122 compliant UUIDs.

#include "guid.h"
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
