/// \file
/// \brief Implements debug symbol table (from .nl files)
///
/// v0.2.25: All `.nl` parsing/serialization, register-map data, filename
/// generation, and ld65 `.dbg` parsing migrated to Rust
/// (`fceux11-debug::debugsym` and `fceux11-debug::ld65dbg`). The C++ side
/// retains the `std::map`-based storage and `debugSymbol_t*` pointer API
/// required by the Qt GUI (`dbg_asm_entry_t::sym` value-type embedding).

#include "debugsymboltable.h"
#include "utils/safe_string.h"

#include "types.h"
#include "debug.h"
#include "fceu.h"
#include "cart.h"
#include "rust/fceux11_rust.h"

#include "Qt/ConsoleUtilities.h"

#include <cstdio>
#include <cstring>
#include <vector>

extern FCEUGI *GameInfo;

debugSymbolTable_t  debugSymbolTable;

static char dbgSymTblErrMsg[256] = {0};
static bool dbgSymAllowDuplicateNames = true;
//--------------------------------------------------------------
// debugSymbol_t
//--------------------------------------------------------------
int debugSymbol_t::updateName( const char *name, int arrayIndex )
{
	std::string newName;

	if (name)
	{
		newName.assign( name );
	}

	// Trim trailing whitespace via Rust.
	if (!newName.empty())
	{
		std::vector<char> buf(newName.begin(), newName.end());
		buf.push_back(0);
		int new_len = fceux11_rust_debugsym_trim_trailing_inplace(buf.data());
		if (new_len < 0) new_len = 0;
		newName.assign(buf.data(), static_cast<size_t>(new_len));
	}

	if (arrayIndex >= 0)
	{
		char outBuf[300];
		fceux11_rust_debugsym_format_array_index(newName.c_str(), arrayIndex, outBuf, sizeof(outBuf));
		newName.assign(outBuf);
	}

	if (page)
	{
		debugSymbol_t *dupSym = debugSymbolTable.getSymbol( page->pageNum(), newName );

		if (!dbgSymAllowDuplicateNames && dupSym != nullptr && dupSym != this)
		{
			snprintf( dbgSymTblErrMsg, sizeof(dbgSymTblErrMsg), "Error: debug symbol '%s' already exists in %s page.\n", newName.c_str(), page->pageName() );
			return -1;
		}
	}
	_name = newName;

	debugSymbolTable.updateSymbol(this);

	return 0;
}
//--------------------------------------------------------------
void debugSymbol_t::trimTrailingSpaces(void)
{
	// Reuse Rust helper for both fields.
	auto trim = [](std::string &s) {
		if (s.empty()) return;
		std::vector<char> buf(s.begin(), s.end());
		buf.push_back(0);
		int new_len = fceux11_rust_debugsym_trim_trailing_inplace(buf.data());
		if (new_len < 0) new_len = 0;
		s.assign(buf.data(), static_cast<size_t>(new_len));
	};
	trim(_name);
	trim(_comment);
}
//--------------------------------------------------------------
// debugSymbolPage_t
//--------------------------------------------------------------
debugSymbolPage_t::debugSymbolPage_t(int page)
{
	_pageNum = page;

	_pageName[0] = 0;

	if (page == -2)
	{
		FCEU_strlcpy(_pageName, sizeof(_pageName), "REG");
	}
	else if (page == -1)
	{
		FCEU_strlcpy(_pageName, sizeof(_pageName), "RAM");
	}
	else
	{
		snprintf( _pageName, sizeof(_pageName), "%X", page);
		_pageName[sizeof(_pageName)-1] = 0;
	}
}
//--------------------------------------------------------------
debugSymbolPage_t::~debugSymbolPage_t(void)
{
	for (auto it=symMap.begin(); it!=symMap.end(); it++)
	{
		delete it->second;
	}
}
//--------------------------------------------------------------
int debugSymbolPage_t::addSymbol( debugSymbol_t*sym )
{
	// Check if symbol already is loaded by that name or offset
	if ( symMap.count( sym->offset() ) )
	{
		snprintf( dbgSymTblErrMsg, sizeof(dbgSymTblErrMsg), "Error: symbol offset 0x%04X already has an entry on %s page\n", sym->offset(), _pageName );
		return -1;
	}
	if ( !dbgSymAllowDuplicateNames && (sym->name().size() > 0) && symNameMap.count( sym->name() ) )
	{
		snprintf( dbgSymTblErrMsg, sizeof(dbgSymTblErrMsg), "Error: symbol name '%s' already exists on %s page\n", sym->name().c_str(), _pageName );
		return -1;
	}

	symMap[ sym->offset() ] = sym;

	sym->page = this;

	// Comment only lines don't need to have a name.
	if (sym->name().size() > 0)
	{
		symNameMap[ sym->name() ] = sym;
	}

	return 0;
}
//--------------------------------------------------------------
debugSymbol_t *debugSymbolPage_t::getSymbolAtOffset( int ofs )
{
	auto it = symMap.find( ofs );
	return it != symMap.end() ? it->second : nullptr;
}
//--------------------------------------------------------------
debugSymbol_t *debugSymbolPage_t::getSymbol( const std::string &name )
{
	auto it = symNameMap.find( name );
	return it != symNameMap.end() ? it->second : nullptr;
}
//--------------------------------------------------------------
int debugSymbolPage_t::deleteSymbolAtOffset( int ofs )
{
	auto it = symMap.find( ofs );

	if ( it != symMap.end() )
	{
		auto sym = it->second;

		if ( sym->name().size() > 0 )
		{
			auto itName = symNameMap.find( sym->name() );

			if ( (itName != symNameMap.end()) && (itName->second == sym) )
			{
				symNameMap.erase(itName);
			}
		}
		symMap.erase(it);
		delete sym;

		return 0;
	}
	return -1;
}
//--------------------------------------------------------------
int debugSymbolPage_t::updateSymbol(debugSymbol_t *sym)
{
	auto itName = symNameMap.begin();

	while (itName != symNameMap.end())
	{
		if (itName->second == sym)
		{
			if (sym->name().size() == 0 || sym->name().compare( itName->first ) )
			{
				itName = symNameMap.erase(itName);
			}
			break;
		}
		else
		{
			itName++;
		}
	}
	if (sym->name().size() > 0)
	{
		symNameMap[ sym->name() ] = sym;
	}

	// Sanity Check
	auto it = symMap.find( sym->offset() );

	if ( it == symMap.end() )
	{	// This shouldn't happen
		return -1;
	}
	return 0;
}
//--------------------------------------------------------------
// Static utility — generate the .nl file path for this page from
// the current ROM filename. Uses the Rust helper.
//--------------------------------------------------------------
static int generateNLFilenameForBank(int bank, std::string &NLfilename);

int debugSymbolPage_t::save(void)
{
	if ( symMap.size() == 0 )
	{
		return 0;
	}
	if ( _pageNum == -2 )
	{
		// Register page is never serialised.
		return 0;
	}

	std::string filename;
	if (generateNLFilenameForBank(_pageNum, filename))
	{
		return -1;
	}

	// Marshal entries through a stable array-of-arrays for the Rust FFI.
	const size_t n = symMap.size();
	std::vector<uint32_t> ofs_arr;
	std::vector<const char *> name_arr;
	std::vector<const char *> comment_arr;
	ofs_arr.reserve(n);
	name_arr.reserve(n);
	comment_arr.reserve(n);
	for (auto it = symMap.begin(); it != symMap.end(); it++)
	{
		debugSymbol_t *sym = it->second;
		ofs_arr.push_back(static_cast<uint32_t>(sym->offset()));
		name_arr.push_back(sym->_name.c_str());
		comment_arr.push_back(sym->_comment.c_str());
	}

	int rc = fceux11_rust_debugsym_save_nl_file(
		filename.c_str(),
		ofs_arr.data(),
		name_arr.data(),
		comment_arr.data(),
		n);

	if (rc != 0)
	{
		FCEU_printf("Error: Could not open file '%s' for writing\n", filename.c_str());
		return -1;
	}
	return 0;
}
//--------------------------------------------------------------
void debugSymbolPage_t::print(void)
{
	FILE *fp = stdout;
	fprintf( fp, "Page: %X \n", _pageNum );

	for (auto it=symMap.begin(); it!=symMap.end(); it++)
	{
		debugSymbol_t *sym = it->second;
		fprintf( fp, "   Sym: $%04X '%s' \n", sym->ofs, sym->name().c_str() );
	}
}
//--------------------------------------------------------------
// debugSymbolTable_t
//--------------------------------------------------------------
debugSymbolTable_t::debugSymbolTable_t(void)
{
	cs = new FCEU::mutex();

	dbgSymTblErrMsg[0] = 0;
}
//--------------------------------------------------------------
debugSymbolTable_t::~debugSymbolTable_t(void)
{
	this->clear();

	if (cs)
	{
		delete cs;
	}
}
//--------------------------------------------------------------
void debugSymbolTable_t::clear(void)
{
	FCEU::autoScopedLock alock(cs);

	for (auto it=pageMap.begin(); it!=pageMap.end(); it++)
	{
		delete it->second;
	}
	pageMap.clear();
}
//--------------------------------------------------------------
int debugSymbolTable_t::numSymbols(void)
{
	int n = 0;
	FCEU::autoScopedLock alock(cs);

	for (auto it=pageMap.begin(); it!=pageMap.end(); it++)
	{
		n += it->second->size();
	}
	return n;
}
//--------------------------------------------------------------
static int generateNLFilenameForBank(int bank, std::string &NLfilename)
{
	const char *romFile = getRomFile();
	if (romFile == nullptr)
	{
		return -1;
	}

	#ifdef DW3_NL_0F_1F_HACK
	if (bank == 0x0F)
		bank = 0x1F;
	#endif

	char buf[4128];
	int len = fceux11_rust_debugsym_nl_filename_for_bank(romFile, bank, buf, sizeof(buf));
	if (len < 0)
	{
		return -1;
	}
	NLfilename.assign(buf, static_cast<size_t>(len));
	return 0;
}
//--------------------------------------------------------------
int generateNLFilenameForAddress(int address, std::string &NLfilename)
{
	int bank;

	if (address < 0x8000)
	{
		bank = -1;
	}
	else
	{
		bank = getBank(address);
		#ifdef DW3_NL_0F_1F_HACK
		if(bank == 0x0F)
			bank = 0x1F;
		#endif
	}
	return generateNLFilenameForBank( bank, NLfilename );
}
//--------------------------------------------------------------
int debugSymbolTable_t::loadFileNL( int bank )
{
	std::string fileName;
	FCEU::autoScopedLock alock(cs);

	if ( generateNLFilenameForBank( bank, fileName ) )
	{
		return -1;
	}

	// Read entire file into a buffer.
	FILE *fp = ::fopen(fileName.c_str(), "rb");
	if (fp == nullptr)
	{
		return -1;
	}
	::fseek(fp, 0, SEEK_END);
	long file_size = ::ftell(fp);
	if (file_size < 0)
	{
		::fclose(fp);
		return -1;
	}
	::fseek(fp, 0, SEEK_SET);
	std::vector<char> content(static_cast<size_t>(file_size) + 1);
	if (file_size > 0)
	{
		size_t got = ::fread(content.data(), 1, static_cast<size_t>(file_size), fp);
		(void)got;
	}
	content[static_cast<size_t>(file_size)] = 0;
	::fclose(fp);

	// Create the page.
	debugSymbolPage_t *page = new debugSymbolPage_t(bank);
	pageMap[page->pageNum()] = page;

	// Parse via Rust iterator.
	NlParseIter *it = fceux11_rust_debugsym_parse_begin(content.data(), static_cast<size_t>(file_size));
	if (!it)
	{
		return 0; // empty file is success
	}

	char nameBuf[1024];
	char commentBuf[4096];
	uint32_t ofs;
	while (fceux11_rust_debugsym_parse_next(it, &ofs, nameBuf, sizeof(nameBuf), commentBuf, sizeof(commentBuf)))
	{
		debugSymbol_t *sym = new debugSymbol_t(static_cast<int>(ofs), nameBuf, commentBuf);
		if (sym == nullptr)
		{
			FCEU_printf("Error: Failed to allocate memory for offset $%04X File %s\n", ofs, fileName.c_str());
			continue;
		}
		if (page->addSymbol(sym))
		{
			FCEU_printf("Error: Failed to add symbol for offset $%04X Name '%s' File %s\n", ofs, sym->name().c_str(), fileName.c_str());
			FCEU_printf("%s\n", errorMessage());
			delete sym;
		}
	}

	fceux11_rust_debugsym_parse_end(it);
	return 0;
}
//--------------------------------------------------------------
int debugSymbolTable_t::loadRegisterMap(void)
{
	FCEU::autoScopedLock alock(cs);
	debugSymbolPage_t *page = new debugSymbolPage_t(-2);

	uint32_t count = fceux11_rust_debugsym_register_map_count();
	for (uint32_t i = 0; i < count; i++)
	{
		uint32_t ofs = 0;
		char name[64];
		if (fceux11_rust_debugsym_register_map_get(i, &ofs, name, sizeof(name)))
		{
			page->addSymbol(new debugSymbol_t(static_cast<int>(ofs), name));
		}
	}

	pageMap[page->pageNum()] = page;
	return 0;
}
//--------------------------------------------------------------
int debugSymbolTable_t::loadGameSymbols(void)
{
	int nPages, pageSize, romSize = 0x10000;

	this->clear();

	if ( GameInfo != nullptr )
	{
		romSize = NES_HEADER_SIZE + CHRsize[0] + PRGsize[0];
	}

	loadFileNL( -1 );

	loadRegisterMap();

	pageSize = (1<<debuggerPageSize);

	nPages = romSize / pageSize;

	for(int i=0;i<nPages;i++)
	{
		loadFileNL( i );
	}

	return 0;
}
//--------------------------------------------------------------
int debugSymbolTable_t::addSymbolAtBankOffset(int bank, int ofs, const char *name, const char *comment)
{
	int result = -1;
	debugSymbol_t *sym = new debugSymbol_t(ofs, name, comment);

	result = addSymbolAtBankOffset(bank, ofs, sym);

	if (result)
	{	// Symbol add failed
		delete sym;
	}
	return result;
}
//--------------------------------------------------------------
int debugSymbolTable_t::addSymbolAtBankOffset( int bank, int ofs, debugSymbol_t *sym )
{
	int result = -1;
	debugSymbolPage_t *page;
	std::map <int, debugSymbolPage_t*>::iterator it;
	FCEU::autoScopedLock alock(cs);

	it = pageMap.find( bank );

	if ( it == pageMap.end() )
	{
		page = new debugSymbolPage_t(bank);
		pageMap[ bank ] = page;
	}
	else
	{
		page = it->second;
	}
	result = page->addSymbol( sym );

	return result;
}
//--------------------------------------------------------------
int debugSymbolTable_t::deleteSymbolAtBankOffset( int bank, int ofs )
{
	debugSymbolPage_t *page;
	std::map <int, debugSymbolPage_t*>::iterator it;
	FCEU::autoScopedLock alock(cs);

	it = pageMap.find( bank );

	if ( it == pageMap.end() )
	{
		return -1;
	}
	else
	{
		page = it->second;
	}

	return page->deleteSymbolAtOffset( ofs );
}
//--------------------------------------------------------------
int debugSymbolTable_t::updateSymbol(debugSymbol_t *sym)
{
	FCEU::autoScopedLock alock(cs);

	if (sym->page == nullptr)
	{
		return -1;
	}
	return sym->page->updateSymbol(sym);
}
//--------------------------------------------------------------
debugSymbol_t *debugSymbolTable_t::getSymbolAtBankOffset( int bank, int ofs )
{
	FCEU::autoScopedLock alock(cs);

	auto it = pageMap.find( bank );

	return it != pageMap.end() ? it->second->getSymbolAtOffset( ofs ) : nullptr;
}
//--------------------------------------------------------------
debugSymbol_t *debugSymbolTable_t::getSymbol( int bank, const std::string &name )
{
	FCEU::autoScopedLock alock(cs);

	auto it = pageMap.find( bank );

	return it != pageMap.end() ? it->second->getSymbol( name ) : nullptr;
}
//--------------------------------------------------------------
debugSymbol_t *debugSymbolTable_t::getSymbolAtAnyBank( const std::string &name )
{
	FCEU::autoScopedLock alock(cs);

	for (auto &page : pageMap)
	{
		auto sym = getSymbol( page.first, name );

		if ( sym )
		{
			return sym;
		}
	}

	return nullptr;
}
//--------------------------------------------------------------
void debugSymbolTable_t::save(void)
{
	debugSymbolPage_t *page;
	std::map <int, debugSymbolPage_t*>::iterator it;
	FCEU::autoScopedLock alock(cs);

	for (it=pageMap.begin(); it!=pageMap.end(); it++)
	{
		page = it->second;
		page->save();
	}
}
//--------------------------------------------------------------
void debugSymbolTable_t::print(void)
{
	debugSymbolPage_t *page;
	std::map <int, debugSymbolPage_t*>::iterator it;
	FCEU::autoScopedLock alock(cs);

	for (it=pageMap.begin(); it!=pageMap.end(); it++)
	{
		page = it->second;
		page->print();
	}
}
//--------------------------------------------------------------
const char *debugSymbolTable_t::errorMessage(void)
{
	return dbgSymTblErrMsg;
}
//--------------------------------------------------------------
static void ld65_iterate_cb( void *userData, const FceuLd65Sym *s )
{
	debugSymbolTable_t *tbl = static_cast<debugSymbolTable_t*>(userData);

	if (tbl)
	{
		tbl->ld65_SymbolLoad(s);
	}
}
//--------------------------------------------------------------
void debugSymbolTable_t::ld65_SymbolLoad( const FceuLd65Sym *s )
{
	if (s == nullptr)
	{
		return;
	}
	// Only labels become symbols (matches original behaviour).
	// sym_type: 0=IMPORT, 1=LABEL, 2=EQU
	if (s->sym_type != 1)
	{
		return;
	}

	int bank = -1;
	if (s->has_segment != 0)
	{
		int romAddr = s->segment_ofs - NES_HEADER_SIZE;
		bank = romAddr >= 0 ? romAddr / (1 << debuggerPageSize) : -1;
	}

	auto pageIt = pageMap.find(bank);
	debugSymbolPage_t *page;
	if (pageIt == pageMap.end())
	{
		page = new debugSymbolPage_t(bank);
		pageMap[bank] = page;
	}
	else
	{
		page = pageIt->second;
	}

	std::string name;
	if (s->has_scope != 0 && s->scope_full_name_ptr != nullptr)
	{
		name.assign(s->scope_full_name_ptr, s->scope_full_name_len);
	}
	if (s->name_ptr != nullptr)
	{
		name.append(s->name_ptr, s->name_len);
	}

	debugSymbol_t *sym = new debugSymbol_t(s->value, name.c_str());
	if (page->addSymbol(sym))
	{
		delete sym;
	}
}
//--------------------------------------------------------------
int debugSymbolTable_t::ld65LoadDebugFile( const char *dbgFilePath )
{
	void *db = fceux11_rust_ld65_open(dbgFilePath);
	if (!db)
	{
		return -1;
	}
	FCEU::autoScopedLock alock(cs);
	fceux11_rust_ld65_iterate(db, this, ld65_iterate_cb);
	fceux11_rust_ld65_close(db);
	return 0;
}
//--------------------------------------------------------------
