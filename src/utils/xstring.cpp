/* Extended string routines
 *
 * Copyright notice for this file:
 *  Copyright (C) 2004 Jason Oster (Parasyte)
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */

/// \file
/// \brief various string manipulation utilities

#include "xstring.h"
#include "utils/safe_string.h"
#include <array>
#include <string>

///Upper case routine. Returns number of characters modified
// R1.2 (refactor_plan.md §Phase R1): the original loop was O(n^2) because
// `while (i < strlen(str))` recomputed strlen on every iteration. Replaced
// with a single pointer-based pass.
int str_ucase(char *str) {
	if (!str) return 0;
	int j = 0;
	for (char *p = str; *p; ++p) {
		if ((*p >= 'a') && (*p <= 'z')) {
			*p &= static_cast<char>(~0x20);
			++j;
		}
	}
	return j;
}


///Lower case routine. Returns number of characters modified
// R1.2 (refactor_plan.md §Phase R1): same O(n^2) → O(n) fix as str_ucase.
int str_lcase(char *str) {
	if (!str) return 0;
	int j = 0;
	for (char *p = str; *p; ++p) {
		if ((*p >= 'A') && (*p <= 'Z')) {
			*p |= 0x20;
			++j;
		}
	}
	return j;
}


///White space-trimming routine

///Removes whitespace from left side of string, depending on the flags set (See STRIP_x definitions in xstring.h)
///Returns number of characters removed
// R1.1 (refactor_plan.md §Phase R1): the original 4 calls to
// `FCEU_strlcpy(str, sizeof(str), str+1)` were a silent data corruption BUG:
// `sizeof(str)` is `sizeof(char*)` (8 bytes on x64), so the copy was capped at
// 7 bytes regardless of the caller's actual buffer size. Rewrote as a
// single-pass scan that locates the first non-whitespace, then a single
// memmove (which correctly handles the overlapping src/dst range). This also
// incidentally eliminates the O(n^2) shift cost of the original loop.
int str_ltrim(char *str, int flags) {
	if (!str) return 0;
	const char *p = str;
	while (*p) {
		const int match =
			((flags & STRIP_SP) && (*p == ' '))  ||
			((flags & STRIP_TAB) && (*p == '\t')) ||
			((flags & STRIP_CR)  && (*p == '\r')) ||
			((flags & STRIP_LF)  && (*p == '\n'));
		if (!match) break;
		++p;
	}
	const size_t removed = static_cast<size_t>(p - str);
	if (removed == 0) return 0;
	// memmove handles the overlapping src/dst case (str+removed → str).
	memmove(str, p, strlen(p) + 1);
	return static_cast<int>(removed);
}


///White space-trimming routine

///Removes whitespace from right side of string, depending on the flags set (See STRIP_x definitions in xstring.h)
///Returns number of characters removed
// R1.1 (refactor_plan.md §Phase R1): the original tested `str[0]` instead of
// `str[strl-1]`, so it could only ever trim a leading space (and only one
// byte at a time) — it never actually trimmed trailing whitespace as the
// function name and docstring claim. Replaced with a single-pass tail scan.
int str_rtrim(char *str, int flags) {
	if (!str) return 0;
	size_t strl = strlen(str);
	unsigned int i = 0;
	while (strl > 0) {
		const char c = str[strl - 1];
		const int match =
			((flags & STRIP_SP) && (c == ' '))  ||
			((flags & STRIP_TAB) && (c == '\t')) ||
			((flags & STRIP_CR)  && (c == '\r')) ||
			((flags & STRIP_LF)  && (c == '\n'));
		if (!match) break;
		str[--strl] = '\0';
		++i;
	}
	return static_cast<int>(i);
}


///White space-stripping routine

///Removes whitespace depending on the flags set (See STRIP_x definitions in xstring.h)
///Returns number of characters removed, or -1 on error
// R1.1 (refactor_plan.md §Phase R1): the final `FCEU_strlcpy(str, sizeof(str),
// astr)` had the same sizeof(str) BUG as str_ltrim — silently truncating
// copies at sizeof(char*) = 8 bytes on x64. Replaced with `memcpy(str, astr,
// j+1)`, which is safe because astr is null-terminated and always ≤ str's
// old content.
// R1.3 (refactor_plan.md §Phase R1): replaced the `malloc`/`free` temp
// buffer with `std::vector<char>`. The vector is RAII-managed so the
// failure path is now exception-safe (the original `free(astr)` after an
// early return would have leaked; the new code has no early return after
// the buffer is created). `str.reserve(strl)` pre-allocates the worst-case
// size (output ≤ input length for a filter operation).
int str_strip(char *str, int flags) {
	if (!str) return -1;
	const size_t strl = strlen(str);
	if (strl == 0) return -1;
	if (!(flags & (STRIP_SP|STRIP_TAB|STRIP_CR|STRIP_LF))) return -1;
	std::vector<char> astr;
	astr.reserve(strl);
	for (size_t k = 0; k < strl; ++k) {
		char chr = str[k];
		if ((flags & STRIP_SP) && (chr == ' '))  chr = 0;
		if ((flags & STRIP_TAB) && (chr == '\t')) chr = 0;
		if ((flags & STRIP_CR)  && (chr == '\r')) chr = 0;
		if ((flags & STRIP_LF)  && (chr == '\n')) chr = 0;
		if (chr) astr.push_back(chr);
	}
	// R1.1 fix: was `FCEU_strlcpy(str, sizeof(str), astr)` (sizeof bug).
	// Write the filtered bytes + NUL terminator back into the caller's
	// buffer. The contract (caller-supplied buffer large enough) is
	// unchanged from pre-R1.1.
	const size_t j = astr.size();
	astr.push_back('\0');
	memcpy(str, astr.data(), astr.size());
	return static_cast<int>(j);
}


///Character replacement routine

///Replaces all instances of 'search' with 'replace'
///Returns number of characters modified
// R1.2 (refactor_plan.md §Phase R1): same O(n^2) → O(n) fix as str_ucase.
int chr_replace(char *str, char search, char replace) {
	if (!str) return 0;
	int j = 0;
	for (char *p = str; *p; ++p) {
		if (*p == search) {
			*p = replace;
			++j;
		}
	}
	return j;
}


///Sub-String replacement routine

///Replaces all instances of 'search' with 'replace'
///Returns number of sub-strings modified, or -1 on error
// R1.1 (refactor_plan.md §Phase R1): the final `FCEU_strlcpy(str, sizeof(str),
// astr)` had the same sizeof(str) BUG as str_ltrim/str_strip. Replaced with
// `memcpy(str, astr, j+1)`. Also cached `strlen(str)` in `strl` to make the
// loop termination check O(1) instead of recomputing on every iteration.
// R1.3 (refactor_plan.md §Phase R1): replaced the `malloc`/`free` temp
// buffer with `std::string tmp`. The std::string manages its own capacity
// (replaces the original `malloc(strl + 1)` upper bound, which silently
// overflowed when `replace` was longer than `search` on aggregate — a
// pre-existing UB). The new contract: caller's `str` buffer must be large
// enough to hold the post-replacement length; the function does not
// silently truncate.
int str_replace(char *str, const char *search, const char *replace) {
	if (!str) return -1;
	const size_t strl = strlen(str);
	if (strl == 0) return -1;
	const size_t searchlen = strlen(search);
	if (searchlen == 0) return -1; // note: allow *replace to have a length of zero!
	const size_t replacelen = strlen(replace);
	std::string tmp;
	tmp.reserve(strl); // upper bound if no match is found; std::string
	                   // grows on demand when replacement is longer
	                   // than search on aggregate.
	size_t i = 0;
	while (i < strl) {
		if ((i + searchlen <= strl) &&
		    (memcmp(str + i, search, searchlen) == 0)) {
			tmp.append(replace, replacelen);
			i += searchlen;
		} else {
			tmp.push_back(str[i]);
			++i;
		}
	}
	// R1.1 fix: was `FCEU_strlcpy(str, sizeof(str), astr)` (sizeof bug).
	// std::string::copy writes exactly the requested count of bytes
	// (it does not append a NUL — we do that explicitly). The new copy
	// is also correct in the rare case where the replacement expands
	// the result beyond the input length (the pre-R1.3 `memcpy(str,
	// astr, j+1)` had the same semantics; std::string just doesn't
	// impose the `strl+1` upper bound that made the original UB).
	const size_t j = tmp.size();
	tmp.copy(str, j);
	str[j] = '\0';
	return static_cast<int>(j);
}

// R1.4 (refactor_plan.md §Phase R1): Base64Table is now a C++20 constexpr
// `std::array` initialised at compile time by a constexpr factory. The
// previous static-initialisation constructor ran once at program start
// (microsecond cost, but more importantly it was an implicit runtime
// dependency for every translation unit that included this header). With
// the constexpr version, the table is a constant in `.rodata` and has
// zero startup cost. The factory is placed in an anonymous namespace so
// it does not pollute the surrounding translation unit.
namespace {
constexpr std::array<unsigned char, 256> make_base64_table() {
	std::array<unsigned char, 256> t{};
	t.fill(0xFF); // mark everything as invalid by default
	// value -> ascii mapping (0..25 = A..Z, 26..51 = a..z, 52..61 = 0..9)
	size_t a = 0;
	for (unsigned char c = 'A'; c <= 'Z'; ++c) t[a++] = c;
	for (unsigned char c = 'a'; c <= 'z'; ++c) t[a++] = c;
	for (unsigned char c = '0'; c <= '9'; ++c) t[a++] = c;
	t[62] = '+';
	t[63] = '/';
	// ascii -> value mapping (high-bit region to avoid clobbering the
	// 0..63 value->ascii slots we just wrote).
	for (a = 0; a < 64; ++a) t[t[a] ^ 0x80] = static_cast<unsigned char>(a);
	t[static_cast<unsigned char>('=') ^ 0x80] = 0;
	return t;
}
} // namespace

static constexpr std::array<unsigned char, 256> Base64Table = make_base64_table();

///Converts the provided data to a string in a standard, user-friendly, round-trippable format
std::string BytesToString(const void* data, int len)
{
	char temp[16];
	if(len==1) {
		snprintf( temp, sizeof(temp),"%d",*(const unsigned char*)data);
		return temp;
	} else if(len==2) {
		snprintf( temp, sizeof(temp),"%d",*(const unsigned short*)data);
		return temp;
	} else if(len==4) {
		snprintf( temp, sizeof(temp),"%d",*(const unsigned int*)data);
		return temp;		
	}
	
	std::string ret;
	if(1) // use base64
	{
		const unsigned char* src = (const unsigned char*)data;
		ret = "base64:";
		for(int n; len > 0; len -= n)
		{
			unsigned char input[3] = {0,0,0};
			for(n=0; n<3 && n<len; ++n)
				input[n] = *src++;
			unsigned char output[4] =
			{
				Base64Table[ input[0] >> 2 ],
				Base64Table[ ((input[0] & 0x03) << 4) | (input[1] >> 4) ],
				static_cast<unsigned char> (n<2 ? '=' : Base64Table[ ((input[1] & 0x0F) << 2) | (input[2] >> 6) ]),
				static_cast<unsigned char> (n<3 ? '=' : Base64Table[ input[2] & 0x3F ])
			};
			ret.append(output, output+4);
		}
	}
	else // use hex
	{
		ret.resize(len*2+2);
		ret[0] = '0';
		ret[1] = 'x';
		for(int i=0;i<len;i++)
		{
			int a = (((const unsigned char*)data)[i]>>4);
			int b = (((const unsigned char*)data)[i])&15;
			if(a>9) a += 'A'-10;
			else a += '0';
			if(b>9) b += 'A'-10;
			else b += '0';
			ret[2+i*2] = a;
			ret[2+i*2+1] = b;
		}
	}
	return ret;
}

///returns -1 if this is not a hex string
int HexStringToBytesLength(const std::string& str)
{
	if(str.size()>2 && str[0] == '0' && toupper(str[1]) == 'X')
		return str.size()/2-1;
	else return -1;
}

int Base64StringToBytesLength(const std::string& str)
{
	if(str.size() < 7 || (str.size()-7) % 4 || str.substr(0,7) != "base64:") return -1;
	
	size_t c = ((str.size() - 7) / 4) * 3;
	if(str[str.size()-1] == '=') { --c;
	if(str[str.size()-2] == '=') --c; }
	return c;
}

///parses a string in the same format as BytesToString
///returns true if success.
bool StringToBytes(const std::string& str, void* data, int len)
{
	if(str.substr(0,7) == "base64:")
	{
		// base64
		unsigned char* tgt = (unsigned char*)data;
		for(size_t pos = 7; pos < str.size() && len > 0; )
		{
			unsigned char input[4], converted[4];
			for(int i=0; i<4; ++i)
			{
				if(pos >= str.size() && i > 0) return false; // invalid data
				input[i]	 = str[pos++];
				if(input[i] & 0x80) return false;	  // illegal character
				converted[i] = Base64Table[input[i]^0x80];
				if(converted[i] & 0x80) return false; // illegal character
			}
			unsigned char outpacket[3] =
			{
				static_cast<unsigned char>((converted[0] << 2) | (converted[1] >> 4)),
				static_cast<unsigned char>((converted[1] << 4) | (converted[2] >> 2)),
				static_cast<unsigned char>((converted[2] << 6) | (converted[3]))
			};
			int outlen = (input[2] == '=') ? 1 : (input[3] == '=' ? 2 : 3);
			if(outlen > len) outlen = len;
			memcpy(tgt, outpacket, outlen);
			tgt += outlen;
			len -= outlen;
		}
		return true;
	}
	if(str.size()>2 && str[0] == '0' && toupper(str[1]) == 'X')
	{
		// hex
		int amt = len;
		int bytesAvailable = str.size()/2;
		if(bytesAvailable < amt)
			amt = bytesAvailable;
		const char* cstr = str.c_str()+2;
		for(int i=0;i<amt;i++) {
			char a = toupper(cstr[i*2]);
			char b = toupper(cstr[i*2+1]);
			if(a>='A') a=a-'A'+10;
			else a-='0';
			if(b>='A') b=b-'A'+10;
			else b-='0';
			unsigned char val = ((unsigned char)a<<4)|(unsigned char)b; 
			((unsigned char*)data)[i] = val;
		}
		return true;
	}
	
	if(len==1) {
		int x = atoi(str.c_str());
		*(unsigned char*)data = x;
		return true;
	} else if(len==2) {
		int x = atoi(str.c_str());
		*(unsigned short*)data = x;
		return true;
	} else if(len==4) {
		int x = atoi(str.c_str());
		*(unsigned int*)data = x;
		return true;
	}
	//we can't handle it
	return false;
}

#include <string>
#include <vector>
/// \brief convert input string into vector of string tokens
///
/// \note consecutive delimiters will be treated as single delimiter
/// \note delimiters are _not_ included in return data
///
/// \param input string to be parsed
/// \param delims list of delimiters.

std::vector<std::string> tokenize_str(const std::string & str,
                                      const std::string & delims=", \t")
{
  // R1.5 (refactor_plan.md §Phase R1): removed `using namespace std;` to
  // comply with the /W4 /WX policy of the project (no `using namespace`
  // in core/utility code). All std types below now carry the `std::`
  // prefix explicitly.
  // Skip delims at beginning, find start of first token
  std::string::size_type lastPos = str.find_first_not_of(delims, 0);
  // Find next delimiter @ end of token
  std::string::size_type pos     = str.find_first_of(delims, lastPos);

  // output vector
  std::vector<std::string> tokens;

  while (std::string::npos != pos || std::string::npos != lastPos)
    {
      // Found a token, add it to the vector.
      tokens.push_back(str.substr(lastPos, pos - lastPos));
      // Skip delims.  Note the "not_of". this is beginning of token
      lastPos = str.find_first_not_of(delims, pos);
      // Find next delimiter at end of token.
      pos     = str.find_first_of(delims, lastPos);
    }

  return tokens;
}

//this code was taken from WINE (LGPL)
//http://google.com/codesearch?hl=en&q=+lang:c+splitpath+show:CPvw9Z-euls:_RSotQzmLeU:KGzljMEYFbY&sa=N&cd=9&ct=rc&cs_p=http://gentoo.osuosl.org/distfiles/Wine-20050524.tar.gz&cs_f=wine-20050524/programs/winefile/splitpath.c
void splitpath(const char* path, char* drv, char* dir, char* name, char* ext)
{
    const char* end; /* end of processed string */
	const char* p;	 /* search pointer */
	const char* s;	 /* copy pointer */

	/* extract drive name */
	if (path[0] && path[1]==':') {
		if (drv) {
			*drv++ = *path++;
			*drv++ = *path++;
			*drv = '\0';
		} else path+=2;
	} else if (drv)
		*drv = '\0';

	/* search for end of string or stream separator */
	for(end=path; *end && *end!=':'; )
		end++;

	/* search for begin of file extension */
	for(p=end; p>path && *--p!='\\' && *p!='/'; )
		if (*p == '.') {
			end = p;
			break;
		}

	if (ext)
		for(s=end; (*ext=*s++); )
			ext++;
	else
		for(s=end; *s++; ) {}

	/* search for end of directory name */
	for(p=end; p>path; )
		if (*--p=='\\' || *p=='/') {
			p++;
			break;
		}

	if (name) {
		for(s=p; s<end; )
			*name++ = *s++;

		*name = '\0';
	} else
		for(s=p; s<end; )
			s++;

	if (dir) {
		for(s=path; s<p; )
			*dir++ = *s++;

		*dir = '\0';
	}
}

//mbg 5/12/08
//for the curious, I tested U16ToHexStr and it was 10x faster than printf.
//so the author of these dedicated functions is not insane, and I will leave them.

static char TempArray[11];

uint16 FastStrToU16(char* s, bool& valid)
{
	int i;
	uint16 v=0;
	for(i=0; i < 4; i++)
	{
		if(s[i] == 0) return v;
		v<<=4;
		if(s[i] >= '0' && s[i] <= '9')
		{
			v+=s[i]-'0';
		}
		else if(s[i] >= 'a' && s[i] <= 'f')
		{
			v+=s[i]-'a'+10;
		}
		else if(s[i] >= 'A' && s[i] <= 'F')
		{
			v+=s[i]-'A'+10;
		}
		else
		{
			valid = false;
			return 0xFFFF;
		}
	}
	valid = true;
	return v;
}

char *U8ToDecStr(uint8 a)
{
	TempArray[0] = '0' + a/100;
	TempArray[1] = '0' + (a%100)/10;
	TempArray[2] = '0' + (a%10);
	TempArray[3] = 0;
	return TempArray;
}

char *U16ToDecStr(uint16 a)
{
	TempArray[0] = '0' + a/10000;
	TempArray[1] = '0' + (a%10000)/1000;
	TempArray[2] = '0' + (a%1000)/100;
	TempArray[3] = '0' + (a%100)/10;
	TempArray[4] = '0' + (a%10);
	TempArray[5] = 0;
	return TempArray;
}

char *U32ToDecStr(char* buf, uint32 a)
{
	buf[0] = '0' + a/1000000000;
	buf[1] = '0' + (a%1000000000)/100000000;
	buf[2] = '0' + (a%100000000)/10000000;
	buf[3] = '0' + (a%10000000)/1000000;
	buf[4] = '0' + (a%1000000)/100000;
	buf[5] = '0' + (a%100000)/10000;
	buf[6] = '0' + (a%10000)/1000;
	buf[7] = '0' + (a%1000)/100;
	buf[8] = '0' + (a%100)/10;
	buf[9] = '0' + (a%10);
	buf[10] = 0;
	return buf;
}
char *U32ToDecStr(uint32 a)
{
	return U32ToDecStr(TempArray,a);
}
char *U32ToDecStr(char* buf, uint32 a, int digits)
{
	if (digits < 1)
		digits = 1;
	else if (digits > 10)
		digits = 10;

	for (int i = 1; i <= digits; ++i)
	{
		buf[digits - i] = '0' + (a % 10);
		a /= 10;
	}
	buf[digits] = 0;
	return buf;
}

char *U16ToHexStr(uint16 a)
{
	TempArray[0] = a/4096 > 9?'A'+a/4096-10:'0' + a/4096;
	TempArray[1] = (a%4096)/256 > 9?'A'+(a%4096)/256 - 10:'0' + (a%4096)/256;
	TempArray[2] = (a%256)/16 > 9?'A'+(a%256)/16 - 10:'0' + (a%256)/16;
	TempArray[3] = a%16 > 9?'A'+(a%16) - 10:'0' + (a%16);
	TempArray[4] = 0;
	return TempArray;
}

char *U8ToHexStr(uint8 a)
{
	TempArray[0] = a/16 > 9?'A'+a/16 - 10:'0' + a/16;
	TempArray[1] = a%16 > 9?'A'+(a%16) - 10:'0' + (a%16);
	TempArray[2] = 0;
	return TempArray;
}

std::string stditoa(int n)
{
	char tempbuf[16];
	snprintf( tempbuf, sizeof(tempbuf), "%d", n);
	return tempbuf;
}


std::string readNullTerminatedAscii(EMUFILE* is)
{
	std::string ret;
	ret.reserve(50);
	for(;;) 
	{
		int c = is->fgetc();
		if(c == 0) break;
		else ret += (char)c;
	}
	return ret;
}

// replace all instances of victim with replacement
// R1.6 (refactor_plan.md §Phase R1): the original loop was missing the
// `j += replacement.length()` increment after the replace. The intent
// is to skip past the just-inserted replacement so the next `find`
// starts looking AFTER it. Without the increment, if `replacement`
// itself contains `victim` as a substring, `find` re-matches inside
// the replacement and the loop runs forever, e.g.
//   mass_replace("hello", "ll", "Xll")
// would loop indefinitely, growing the string by one byte per
// iteration. Also added an early return for empty `victim`, which
// had the same infinite-loop bug (`find("", j)` always returns `j`).
std::string mass_replace(const std::string &source, const std::string &victim, const std::string &replacement)
{
	if (victim.empty()) {
		return source; // R1.6: avoid infinite loop on empty victim
	}
	std::string answer = source;
	std::string::size_type j = 0;
	while ((j = answer.find(victim, j)) != std::string::npos) {
		answer.replace(j, victim.length(), replacement);
		j += replacement.length(); // R1.6 fix: skip past replacement
	}
	return answer;
}

#ifdef __WIN_DRIVER__ // this code tends to crash on SDL.
//http://www.codeproject.com/KB/string/UtfConverter.aspx
#include "ConvertUTF.h"
namespace UtfConverter
{
    std::wstring FromUtf8(const std::string& utf8string)
    {
        size_t widesize = utf8string.length();
        if (sizeof(wchar_t) == 2)
        {
            wchar_t* widestringnative = new wchar_t[widesize+1];
            const UTF8* sourcestart = reinterpret_cast<const UTF8*>(utf8string.c_str());
            const UTF8* sourceend = sourcestart + widesize;
            UTF16* targetstart = reinterpret_cast<UTF16*>(widestringnative);
            UTF16* targetend = targetstart + widesize+1;
            ConversionResult res = ConvertUTF8toUTF16(&sourcestart, sourceend, &targetstart, targetend, strictConversion);
            if (res != conversionOK)
            {
                delete [] widestringnative;
                throw std::exception();
            }
            *targetstart = 0;
            std::wstring resultstring(widestringnative);
            delete [] widestringnative;
            return resultstring;
        }
        else if (sizeof(wchar_t) == 4) // somewhat pointless as it's always 2 on WIN32, but whatever.
        {
            wchar_t* widestringnative = new wchar_t[widesize];
            const UTF8* sourcestart = reinterpret_cast<const UTF8*>(utf8string.c_str());
            const UTF8* sourceend = sourcestart + widesize;
            UTF32* targetstart = reinterpret_cast<UTF32*>(widestringnative);
            UTF32* targetend = targetstart + widesize;
            ConversionResult res = ConvertUTF8toUTF32(&sourcestart, sourceend, &targetstart, targetend, strictConversion);
            if (res != conversionOK)
            {
                delete [] widestringnative;
                throw std::exception();
            }
            *targetstart = 0;
            std::wstring resultstring(widestringnative);
            delete [] widestringnative;
            return resultstring;
        }
        else
        {
            throw std::exception();
        }
        return L"";
    }

    std::string ToUtf8(const std::wstring& widestring)
    {
        size_t widesize = widestring.length();

        if (sizeof(wchar_t) == 2)
        {
            size_t utf8size = 3 * widesize + 1;
            char* utf8stringnative = new char[utf8size];
            const UTF16* sourcestart = reinterpret_cast<const UTF16*>(widestring.c_str());
            const UTF16* sourceend = sourcestart + widesize;
            UTF8* targetstart = reinterpret_cast<UTF8*>(utf8stringnative);
            UTF8* targetend = targetstart + utf8size;
            ConversionResult res = ConvertUTF16toUTF8(&sourcestart, sourceend, &targetstart, targetend, strictConversion);
            if (res != conversionOK)
            {
                delete [] utf8stringnative;
                throw std::exception();
            }
            *targetstart = 0;
            std::string resultstring(utf8stringnative);
            delete [] utf8stringnative;
            return resultstring;
        }
        else if (sizeof(wchar_t) == 4) // again, sizeof(wchar_t) == 2 in win32
        {
            size_t utf8size = 4 * widesize + 1;
            char* utf8stringnative = new char[utf8size];
            const UTF32* sourcestart = reinterpret_cast<const UTF32*>(widestring.c_str());
            const UTF32* sourceend = sourcestart + widesize;
            UTF8* targetstart = reinterpret_cast<UTF8*>(utf8stringnative);
            UTF8* targetend = targetstart + utf8size;
            ConversionResult res = ConvertUTF32toUTF8(&sourcestart, sourceend, &targetstart, targetend, strictConversion);
            if (res != conversionOK)
            {
                delete [] utf8stringnative;
                throw std::exception();
            }
            *targetstart = 0;
            std::string resultstring(utf8stringnative);
            delete [] utf8stringnative;
            return resultstring;
        }
        else
        {
            throw std::exception();
        }
        return "";
    }
}
#else
namespace UtfConverter
{
    void SeqValue(std::string& result, unsigned n)
    {
        if(n < 0x80)                 // <=7 bits
            result += (char)n;
        else
        {
            if(n < 0x800)            // <=11 bits
                result += (char)(0xC0 + (n>>6));
            else
            {
                if(n < 0x10000)      // <=16 bits
                    result += (char)(0xE0 + (n>>12));
                else                 // <=21 bits
                {
                    result += (char)(0xF0 + (n>>18));
                    result += (char)(0x80 + ((n>>12)&63));
                }
                result += (char)(0x80 + ((n>>6)&63));
            }
            result += (char)(0x80 + (n&63));
        }
    }

    unsigned DecData(const std::string& input, size_t& pos)
    {
        unsigned char headbyte = input[pos];
        static const char sizes[16] =  { 1,1,1,1,1,1,1,1, 0,0,0,0,2,2,3,4 };
        static const unsigned minimums[4] = { 0, 0x80, 0x800, 0x10000 };
        static const char     masks[4]    = { 0x7F, 0x1F, 0x0F, 0x07 };
        unsigned len = sizes[headbyte >> 4];
        if(len < 1 || pos+len > input.size()) { ++pos; return '?'; }
        unsigned result=0, shl=0;
        for(unsigned n = len; --n > 0; shl += 6)
        {
            unsigned char byte = input[pos+n];
            if((byte & 0xC0) != 0x80) { ++pos; return '?'; }
            result |= (byte & 0x3F) << shl;
        }
        result |= (headbyte & masks[len-1]) << shl;
        if(result < minimums[len-1]) { ++pos; return '?'; }
        pos += len;
        return result;
    }


    std::wstring FromUtf8(std::string& input) // string -> wstring
    {
        std::wstring result;
        for(std::string::size_type pos = 0; pos < input.size(); )
            result += DecData(input, pos);
        return result;
    }
    
    std::string ToUtf8(std::wstring& input) // wstring -> string
    {
        std::string result;
        for(std::string::size_type pos = 0; pos < input.size(); ++pos)
            SeqValue(result, input[pos]);
        return result;
    }
}
#endif

  
//convert a std::string to std::wstring
std::wstring mbstowcs(std::string str) // UTF8->UTF32
{
	try {
		return UtfConverter::FromUtf8(str);
	} catch(std::exception &e) {
		return L"(failed UTF-8 conversion)";
	}
}

//convert a std::wstring to std::string
std::string wcstombs(std::wstring str) // UTF32->UTF8
{
	return UtfConverter::ToUtf8(str);
}


//TODO - dont we already have another  function that can do this
std::string getExtension(const char* input) {
	char buf[1024];
	FCEU_strlcpy(buf, sizeof(buf), input);
	char* dot=strrchr(buf,'.');
	if(!dot)
		return "";
	char ext [512];
	FCEU_strlcpy(ext, sizeof(ext), dot+1);
	int k, extlen=strlen(ext);
	for(k=0;k<extlen;k++)
		ext[k]=tolower(ext[k]);
	return ext;
}

//strips the file extension off a filename
std::string StripExtension(std::string filename)
{
	return filename.substr(0, filename.find_last_of("."));
}

//strips the path off a filename
std::string StripPath(std::string filename)
{
	int x = filename.find_last_of("\\") + 1;
	return filename.substr(x, filename.length() - x);
}
