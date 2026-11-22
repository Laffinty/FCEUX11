/*
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
/*
 * ConvertUTF.c - Unicode UTF conversion wrapper
 *
 * Phase 6 (v0.2.7): Delegates to Rust.
 */

#include "ConvertUTF.h"

ConversionResult ConvertUTF8toUTF16(
		const UTF8** sourceStart, const UTF8* sourceEnd,
		UTF16** targetStart, UTF16* targetEnd, ConversionFlags flags)
{
	return (ConversionResult)fceux11_rust_convert_utf8_to_utf16(
		(const uint8_t**)sourceStart, sourceEnd,
		(uint16_t**)targetStart, targetEnd, (int)flags);
}

ConversionResult ConvertUTF16toUTF8(
		const UTF16** sourceStart, const UTF16* sourceEnd,
		UTF8** targetStart, UTF8* targetEnd, ConversionFlags flags)
{
	return (ConversionResult)fceux11_rust_convert_utf16_to_utf8(
		(const uint16_t**)sourceStart, sourceEnd,
		(uint8_t**)targetStart, targetEnd, (int)flags);
}

ConversionResult ConvertUTF8toUTF32(
		const UTF8** sourceStart, const UTF8* sourceEnd,
		UTF32** targetStart, UTF32* targetEnd, ConversionFlags flags)
{
	return (ConversionResult)fceux11_rust_convert_utf8_to_utf32(
		(const uint8_t**)sourceStart, sourceEnd,
		(uint32_t**)targetStart, targetEnd, (int)flags);
}

ConversionResult ConvertUTF32toUTF8(
		const UTF32** sourceStart, const UTF32* sourceEnd,
		UTF8** targetStart, UTF8* targetEnd, ConversionFlags flags)
{
	return (ConversionResult)fceux11_rust_convert_utf32_to_utf8(
		(const uint32_t**)sourceStart, sourceEnd,
		(uint8_t**)targetStart, targetEnd, (int)flags);
}

ConversionResult ConvertUTF16toUTF32(
		const UTF16** sourceStart, const UTF16* sourceEnd,
		UTF32** targetStart, UTF32* targetEnd, ConversionFlags flags)
{
	return (ConversionResult)fceux11_rust_convert_utf16_to_utf32(
		(const uint16_t**)sourceStart, sourceEnd,
		(uint32_t**)targetStart, targetEnd, (int)flags);
}

ConversionResult ConvertUTF32toUTF16(
		const UTF32** sourceStart, const UTF32* sourceEnd,
		UTF16** targetStart, UTF16* targetEnd, ConversionFlags flags)
{
	return (ConversionResult)fceux11_rust_convert_utf32_to_utf16(
		(const uint32_t**)sourceStart, sourceEnd,
		(uint16_t**)targetStart, targetEnd, (int)flags);
}

Boolean isLegalUTF8Sequence(const UTF8 *source, const UTF8 *sourceEnd)
{
	return (Boolean)fceux11_rust_is_legal_utf8_sequence(source, sourceEnd);
}
