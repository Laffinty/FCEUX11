#ifndef _guid_h_
#define _guid_h_

#include <string>
#include "../types.h"
#include "valuearray.h"

struct FCEU_Guid : public ValueArray<uint8,16>
{
	void newGuid();
	std::string toString();
	static FCEU_Guid fromString(std::string str);
	static uint8 hexToByte(char** ptrptr);
	void scan(std::string& str);
};

// R2.1 (refactor_plan.md §Phase R2): layout-assert that the const-correctness
// refactor of ValueArray did not change the byte layout. SFORMAT
// savestate serialization in src/state.cpp writes FCEU_Guid as a raw
// 16-byte blob, so any size change here is a savestate-compat break.
// Note: ValueArray<T,N> is `T data[N]`, so sizeof(ValueArray<uint8,16>)
// is exactly 16; this assert is paranoid but cheap.
static_assert(sizeof(FCEU_Guid) == 16,
              "FCEU_Guid layout changed: 16-byte SFORMAT serialization in "
              "src/state.cpp will break. Check valuearray.h refactor.");


#endif
