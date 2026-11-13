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
    // v2.0.4 (V19): value-initialize so a failed parse can never return
    // stack garbage. Upstream master does the same via
    // memset(data,0,sizeof(data)) on the failure path of scan(); here the
    // guarantee lives in the initialization because the parse itself is
    // delegated to Rust (fceux11_rust_guid_scan), which zeroes its own
    // failure paths — this `ret{}` is the belt-and-suspenders underneath.
    // See docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-构建计划.md §V19.
    FCEU_Guid ret{};
    ret.scan(str);
    return ret;
}

void FCEU_Guid::scan(std::string& str)
{
    fceux11_rust_guid_scan(reinterpret_cast<FceuGuid*>(this), str.c_str());
}
