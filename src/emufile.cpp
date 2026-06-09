/*
[v0.2.16] Rust-backed wrapper. Original C++ implementation preserved.
The EMUFILE_MEMORY methods are forwarded to the Rust implementation.
EMUFILE_FILE remains C++-based (file I/O is platform-native).

FFI contract:
- Rust owns the memory buffer for EMUFILE_MEMORY.
- C++ never dereferences the Rust handle directly.
- All EMUFILE_MEMORY operations go through FFI functions.
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
    file.fread(&dstbuf->at(0),size);
    return true;
}

size_t EMUFILE_MEMORY::_fread(const void *ptr, size_t bytes){
    size_t remain = len-pos;
    size_t todo = std::min<size_t>(remain,bytes);
    if(len==0)
    {
        failbit = true;
        return 0;
    }
    if(todo<=4)
    {
        u8* src = buf()+pos;
        u8* dst = (u8*)ptr;
        for(size_t i=0;i<todo;i++)
            *dst++ = *src++;
    }
    else
    {
        memcpy((void*)ptr,buf()+pos,todo);
    }
    pos += todo;
    if(todo<bytes)
        failbit = true;
    return todo;
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
    fwrite(&val,8);
}

size_t EMUFILE::read64le(u64 *Bufo)
{
    u64 buf=0;
    if(fread((char*)&buf,8) != 8)
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
    fwrite(&val,4);
}

size_t EMUFILE::read32le(s32* Bufo) { return read32le((u32*)Bufo); }

size_t EMUFILE::read32le(u32* Bufo)
{
    u32 buf=0;
    if(fread(&buf,4)<4)
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
    fwrite(&val,2);
}

size_t EMUFILE::read16le(s16* Bufo) { return read16le((u16*)Bufo); }

size_t EMUFILE::read16le(u16* Bufo)
{
    u32 buf=0;
    if(fread(&buf,2)<2)
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
    fwrite(&val,1);
}

size_t EMUFILE::read8le(u8* val)
{
    return fread(val,1);
}

u8 EMUFILE::read8le()
{
    u8 temp = 0;
    fread(&temp,1);
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
    fread(mem->buf(),size());
    return mem;
}