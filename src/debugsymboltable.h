/* FCE Ultra - NES/Famicom Emulator
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
#ifndef _DEBUGSYMBOLTABLE_H_
#define _DEBUGSYMBOLTABLE_H_

#include <string>
#include <map>

#include "utils/mutex.h"

// Forward decl — defined in rust/fceux11_rust.h. We avoid that #include here
// so this header stays lightweight; the .cpp includes the Rust header.
struct FceuLd65Sym;

class debugSymbolPage_t;
class debugSymbolTable_t;

class debugSymbol_t
{
	public:
	debugSymbol_t(void)
	{
		ofs = 0;
		page = nullptr;
	};

	debugSymbol_t( int ofs, const char *name = nullptr, const char *comment = nullptr )
	{
		this->ofs = ofs;

		if (name)
		{
			this->_name.assign(name);
		}
		if ( comment )
		{
			this->_comment.assign( comment );
		}
		page = nullptr;
	}

	const std::string &name(void)
	{
		return _name;
	}

	const std::string &comment(void)
	{
		return _comment;
	}

	void commentAssign( std::string str )
	{
		_comment.assign(str);
		return;
	}

	void commentAssign( const char *str )
	{
		_comment.assign(str);
		return;
	}

	int offset(void)
	{
		return ofs;
	}

	void setOffset( int o )
	{
		if (o != ofs)
		{
			ofs = o;
		}
	}

	int  updateName( const char *name, int arrayIndex = -1 );

	void trimTrailingSpaces(void);

	private:

	int   ofs;
	std::string  _name;
	std::string  _comment;
	debugSymbolPage_t *page;

	friend class debugSymbolPage_t;
	friend class debugSymbolTable_t;
};

class debugSymbolPage_t
{
	public:
	debugSymbolPage_t(int page);
	~debugSymbolPage_t(void);

	int  save(void);
	void print(void);
	int size(void){ return static_cast<int>(symMap.size()); }

	int addSymbol( debugSymbol_t *sym );

	int deleteSymbolAtOffset( int ofs );

	int updateSymbol( debugSymbol_t *sym );

	debugSymbol_t *getSymbolAtOffset( int ofs );

	debugSymbol_t *getSymbol( const std::string &name );

	int pageNum(void)
	{
		return _pageNum;
	}

	const char *pageName(void)
	{
		return _pageName;
	}

	private:
	int _pageNum;
	char _pageName[8];
	std::map <int, debugSymbol_t*> symMap;
	std::map <std::string, debugSymbol_t*> symNameMap;

	friend class debugSymbolTable_t;
};

class debugSymbolTable_t
{

	public:
		debugSymbolTable_t(void);
		~debugSymbolTable_t(void);

		int loadFileNL( int addr );
		int loadRegisterMap(void);
		int loadGameSymbols(void);
		int numPages(void){ return pageMap.size(); }
		int numSymbols(void);

		void save(void);
		void clear(void);
		void print(void);

		debugSymbol_t *getSymbolAtBankOffset( int bank, int ofs );

		debugSymbol_t *getSymbol( int bank, const std::string& name);

		debugSymbol_t *getSymbolAtAnyBank( const std::string& name);

		int addSymbolAtBankOffset( int bank, int ofs, debugSymbol_t *sym );

		int addSymbolAtBankOffset(int bank, int ofs, const char* name, const char* comment = nullptr);

		int deleteSymbolAtBankOffset( int bank, int ofs );

		int updateSymbol( debugSymbol_t *sym );

		const char *errorMessage(void);

		int ld65LoadDebugFile( const char *dbgFilePath );

		void ld65_SymbolLoad( const struct FceuLd65Sym *s );

	private:
		std::map <int, debugSymbolPage_t*> pageMap;
		FCEU::mutex *cs;
};

extern  debugSymbolTable_t  debugSymbolTable;

#endif
