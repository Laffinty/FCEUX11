/*
[v0.2.16] Rust-backed wrapper. Original C++ implementation preserved.
The EMUFILE_MEMORY methods are forwarded to the Rust implementation.
EMUFILE_FILE remains C++-based (file I/O is platform-native).

FFI contract:
- Rust owns the memory buffer for EMUFILE_MEMORY.
- C++ never dereferences the Rust handle directly.
- All EMUFILE_MEMORY operations go through FFI functions.

[v0.3.10] EMUFILE::fread/fwrite virtuals migrated to std::span<std::byte>.
The (void*, size_t) overloads remain as [[deprecated]] inline shims in
emufile.h. Internal framework callers (write64le, fprintf, fputc, memwrap,
etc.) bypass the shim and call the new virtuals directly to avoid
self-deprecation warnings.
*/

#include "emufile.h"
#include "utils/safe_string.h"
#include "emufile_types.h"
#include "rust/fceux11_rust.h"
#include "utils/xstring.h"

#include <bit>

#include <stdio.h>
#include <vector>

bool EMUFILE::readAllBytes(std::vector<u8>* dstbuf, const std::string& fname)
{
    EMUFILE_FILE file(fname.c_str(),"rb");
    if(file.fail()) return false;
    size_t size = file.size();
    dstbuf->resize(size);
    // v0.3.10: span-boundary conversion. dstbuf holds u8; EMUFILE::fread
    // accepts std::span<std::byte>. reinterpret_cast is safe here because
    // std::byte and uint8_t have identical size/alignment and we round-trip
    // back via the same span.
    std::span<std::byte> dst(reinterpret_cast<std::byte*>(dstbuf->data()), dstbuf->size());
    file.fread(dst);
    return true;
}

size_t EMUFILE_MEMORY::fread(std::span<std::byte> dst){
    size_t remain = len-pos;
    size_t todo = std::min<size_t>(remain, dst.size());
    if(len==0)
    {
        failbit = true;
        return 0;
    }
    if(todo<=4)
    {
        std::byte* src = buf()+pos;
        std::byte* out = dst.data();
        for(size_t i=0;i<todo;i++)
            *out++ = *src++;
    }
    else
    {
        memcpy(dst.data(), buf()+pos, todo);
    }
    pos += todo;
    if(todo<dst.size())
        failbit = true;
    return todo;
}

size_t EMUFILE_MEMORY::fwrite(std::span<const std::byte> src){
    reserve(pos+src.size());
    memcpy(buf()+pos, src.data(), src.size());
    pos += static_cast<long>(src.size());
    len = std::max<size_t>(pos,len);
    return src.size();
}

EMUFILE* EMUFILE_MEMORY::memwrap()
{
    return this;
}

void EMUFILE::write64le(u64* val)
{
    write64le(*val);
}

void EMUFILE::write64le(u64 val)
{
    // v0.3.10: direct std::span virtual call to avoid the [[deprecated]]
    // (void*, size_t) shim. u64 -> std::byte is well-defined via bit_cast.
    fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(&val), sizeof(val)));
}

size_t EMUFILE::read64le(u64 *Bufo)
{
    u64 buf=0;
    if(fread(std::span<std::byte>(reinterpret_cast<std::byte*>(&buf), sizeof(buf))) != sizeof(buf))
        return 0;
    *Bufo=buf;
    return 1;
}

u64 EMUFILE::read64le()
{
    u64 temp;
    read64le(&temp);
    return temp;
}

void EMUFILE::write32le(u32* val)
{
    write32le(*val);
}

void EMUFILE::write32le(u32 val)
{
    fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(&val), sizeof(val)));
}

size_t EMUFILE::read32le(s32* Bufo) { return read32le((u32*)Bufo); }

size_t EMUFILE::read32le(u32* Bufo)
{
    u32 buf=0;
    if(fread(std::span<std::byte>(reinterpret_cast<std::byte*>(&buf), sizeof(buf)))<sizeof(buf))
        return 0;
    *(u32*)Bufo=buf;
    return 1;
}

u32 EMUFILE::read32le()
{
    u32 ret=0;
    read32le(&ret);
    return ret;
}

void EMUFILE::write16le(u16* val)
{
    write16le(*val);
}

void EMUFILE::write16le(u16 val)
{
    fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(&val), sizeof(val)));
}

size_t EMUFILE::read16le(s16* Bufo) { return read16le((u16*)Bufo); }

size_t EMUFILE::read16le(u16* Bufo)
{
    u32 buf=0;
    if(fread(std::span<std::byte>(reinterpret_cast<std::byte*>(&buf), sizeof(buf)))<sizeof(buf))
        return 0;
    *(u16*)Bufo=buf;
    return 1;
}

u16 EMUFILE::read16le()
{
    u16 ret=0;
    read16le(&ret);
    return ret;
}

void EMUFILE::write8le(u8* val)
{
    write8le(*val);
}

void EMUFILE::write8le(u8 val)
{
    fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(&val), sizeof(val)));
}

size_t EMUFILE::read8le(u8* val)
{
    return fread(std::span<std::byte>(reinterpret_cast<std::byte*>(val), sizeof(*val)));
}

u8 EMUFILE::read8le()
{
    u8 temp = 0;
    fread(std::span<std::byte>(reinterpret_cast<std::byte*>(&temp), sizeof(temp)));
    return temp;
}

void EMUFILE::writedouble(double* val)
{
    write64le(std::bit_cast<u64>(*val));
}
void EMUFILE::writedouble(double val)
{
    write64le(std::bit_cast<u64>(val));
}

double EMUFILE::readdouble()
{
    double temp=0.0;
    readdouble(&temp);
    return temp;
}

size_t EMUFILE::readdouble(double* val)
{
    u64 temp=0;
    size_t ret = read64le(&temp);
    *val = std::bit_cast<double>(temp);
    return ret;
}

// EMUFILE_FILE methods - keep C++ implementation
void EMUFILE_FILE::open(const char* fname, const char* mode)
{
    fp = fopen(fname,mode);
    if(!fp)
    {
#ifdef _MSC_VER
        std::wstring wfname = mbstowcs((std::string)fname);
        std::wstring wfmode = mbstowcs((std::string)mode);
        fp = _wfopen(wfname.c_str(),wfmode.c_str());
#endif
        if(!fp)
            failbit = true;
    }
    this->fname = fname;
    FCEU_strlcpy(this->mode, sizeof(this->mode), mode);
}

void EMUFILE_FILE::truncate(size_t length)
{
    ::fflush(fp);
    #ifdef _MSC_VER
        _chsize(_fileno(fp),length);
    #else
        if ( ftruncate(fileno(fp),length) != 0 )
        {
            printf("Warning: EMUFILE_FILE::truncate failed\n");
        }
    #endif
    fclose(fp);
    fp = NULL;
    open(fname.c_str(),mode);
}

EMUFILE* EMUFILE_FILE::memwrap()
{
    EMUFILE_MEMORY* mem = new EMUFILE_MEMORY(size());
    if(size()==0) return mem;
    // v0.3.10: direct std::span virtual call (avoids [[deprecated]] shim).
    fread(std::span<std::byte>(mem->buf(), mem->size()));
    return mem;
}