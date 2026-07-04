/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2002 Xodnizel
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

// FCEUX11 v1.11 Bridge — fceu_archive.cpp
// Archive subsystem (minizip + libarchive backend).
// Split from fceuWrapper.cpp per v1.11_bridge_build_plan.md §4.1.

#include <stdio.h>
#include <stdlib.h>
#include <cstring>

#include <QFileInfo>
#include <string>
#include <vector>

#include "../../driver.h"
#include "../../emufile.h"
#include "utils/unzip.h"
#include "common/configSys.h"
#include "Qt/ConsoleWindow.h"
#include "Qt/ConsoleUtilities.h"

#ifdef _USE_LIBARCHIVE
#include <archive.h>
#include <archive_entry.h>
#endif

extern Config* g_config;

#ifdef _MSC_VER
#define strncasecmp _strnicmp
#define strcasecmp _stricmp
#endif

static int minizip_ScanArchive( const char *filepath, ArchiveScanRecord &rec)
{
	int idx=0, ret;
	unzFile zf;
	unz_file_info fi;
	char filename[512];

	zf = unzOpen( filepath );

	if ( zf == NULL )
	{
		return -1;
	}
	rec.type = 0;

	ret = unzGoToFirstFile( zf );

	while ( ret == 0 )
	{
		FCEUARCHIVEFILEINFO_ITEM item;

		unzGetCurrentFileInfo( zf, &fi, filename, sizeof(filename), NULL, 0, NULL, 0 );

		item.name.assign( filename );
		item.size  = fi.uncompressed_size;
		item.index = idx; idx++;

		rec.files.push_back( item );

		ret = unzGoToNextFile( zf );
	}
	rec.numFilesInArchive = idx;

	unzClose( zf );

	return 0;
}

#ifdef _USE_LIBARCHIVE

static int libarchive_ScanArchive( const char *filepath, ArchiveScanRecord &rec)
{
	int r, idx=0;
	struct archive *a;
	struct archive_entry *entry;

	a = archive_read_new();

	if (a == nullptr)
	{
		return -1;
	}

	r = archive_read_support_filter_all(a);
	if (r)
	{
		archive_read_free(a);
		return -1;
	}

	r = archive_read_support_format_all(a);
	if (r)
	{
		archive_read_free(a);
		return -1;
	}

	r = archive_read_open_filename(a, filepath, 10240);

	if (r)
	{
		archive_read_free(a);
		return -1;
	}
	rec.type = 1;

	while (1)
	{
		r = archive_read_next_header(a, &entry);
		if (r == ARCHIVE_EOF)
		{
			break;
		}
		else if (r != ARCHIVE_OK)
		{
			printf("archive_read_next_header() %s\n", archive_error_string(a));
			break;
		}
		const char *filename = archive_entry_pathname(entry);

		FCEUARCHIVEFILEINFO_ITEM item;
		item.name.assign( filename );
		item.size  = archive_entry_size(entry);
		item.index = idx; idx++;

		rec.files.push_back( item );
	}
	rec.numFilesInArchive = idx;

	archive_read_free(a);

	return 0;
}
#endif

ArchiveScanRecord fceWrapper_ScanArchive(std::string fname)
{
	int ret = -1;
	ArchiveScanRecord rec;
		
#ifdef _USE_LIBARCHIVE
	ret = libarchive_ScanArchive( fname.c_str(), rec );
#endif

	if (ret == -1)
	{
		minizip_ScanArchive( fname.c_str(), rec );
	}
	return rec;
}

static FCEUFILE* minizip_OpenArchive(ArchiveScanRecord& asr, std::string &fname, std::string *searchFile, int innerIndex )
{
	int ret, idx=0;
	FCEUFILE* fp = nullptr;
	void *tmpMem = nullptr;
	unzFile zf;
	unz_file_info fi;
	char filename[512];
	bool foundFile = false;

	zf = unzOpen( fname.c_str() );

	if ( zf == NULL )
	{
		return fp;
	}

	ret = unzGoToFirstFile( zf );

	while ( ret == 0 )
	{
		unzGetCurrentFileInfo( zf, &fi, filename, sizeof(filename), NULL, 0, NULL, 0 );

		if (searchFile)
		{
			if ( strcmp( searchFile->c_str(), filename ) == 0 )
			{
				foundFile = true; break;
			}
		}
		else if ((innerIndex != -1) && (idx == innerIndex))
		{
			foundFile = true; break;
		}

		ret = unzGoToNextFile( zf );

		idx++;
	}

	if ( !foundFile )
	{
		unzClose( zf );
		return fp;
	}

	tmpMem = ::malloc( fi.uncompressed_size );

	if ( tmpMem == NULL )
	{
		unzClose( zf );
		return fp;
	}

	EMUFILE_MEMORY* ms = new EMUFILE_MEMORY(fi.uncompressed_size);

	unzOpenCurrentFile( zf );
	unzReadCurrentFile( zf, tmpMem, fi.uncompressed_size );
	unzCloseCurrentFile( zf );

	ms->fwrite(std::span<const std::byte>(static_cast<const std::byte*>(tmpMem), fi.uncompressed_size));

	free( tmpMem );

	fp = new FCEUFILE();
	fp->archiveFilename = fname;
	fp->filename = filename;
	fp->fullFilename = fp->archiveFilename + "|" + fp->filename;
	fp->archiveIndex = idx;
	fp->mode = FCEUFILE::READ;
	fp->size = fi.uncompressed_size;
	fp->stream = ms;
	fp->archiveCount = (int)asr.numFilesInArchive;
	ms->fseek(0,SEEK_SET);

	unzClose( zf );

	return fp;
}

#ifdef _USE_LIBARCHIVE
static FCEUFILE* libarchive_OpenArchive( ArchiveScanRecord& asr, std::string& fname, std::string *searchFile, int innerIndex)
{
	int r, idx=0;
	struct archive *a;
	struct archive_entry *entry;
	const char *filename = nullptr;
	bool foundFile = false;
	int fileSize = 0;
	FCEUFILE* fp = nullptr;

	a = archive_read_new();

	if (a == nullptr)
	{
		archive_read_free(a);
		return nullptr;
	}

	r = archive_read_support_filter_all(a);
	if (r)
	{
		archive_read_free(a);
		return nullptr;
	}

	r = archive_read_support_format_all(a);
	if (r)
	{
		archive_read_free(a);
		return nullptr;
	}

	r = archive_read_open_filename(a, fname.c_str(), 10240);

	if (r)
	{
		archive_read_free(a);
		return nullptr;
	}

	while (1)
	{
		r = archive_read_next_header(a, &entry);
		if (r == ARCHIVE_EOF)
		{
			break;
		}
		else if (r != ARCHIVE_OK)
		{
			printf("archive_read_next_header() %s\n", archive_error_string(a));
			break;
		}
		filename = archive_entry_pathname(entry);
		fileSize = archive_entry_size(entry);

		if (searchFile)
		{
			if (strcmp( filename, searchFile->c_str() ) == 0)
			{
				foundFile = true; break;
			}
		}
		else if ((innerIndex != -1) && (idx == innerIndex))
		{
			foundFile = true; break;
		}
		idx++;
	}

	if (foundFile && (fileSize > 0))
	{
		const void *buff;
		size_t size, totalSize = 0;
		#if ARCHIVE_VERSION_NUMBER >= 3000000
			int64_t offset;
		#else
			off_t offset;
		#endif

		EMUFILE_MEMORY* ms = new EMUFILE_MEMORY(fileSize);

		while (1)
		{
			r = archive_read_data_block(a, &buff, &size, &offset);

			if (r == ARCHIVE_EOF)
			{
				break;
			}
			if (r != ARCHIVE_OK)
			{
				break;
			}
			ms->fwrite(std::span<const std::byte>(static_cast<const std::byte*>(buff), size));
			totalSize += size;
		}

		fp = new FCEUFILE();
		fp->archiveFilename = fname;
		fp->filename = filename;
		fp->fullFilename = fp->archiveFilename + "|" + fp->filename;
		fp->archiveIndex = idx;
		fp->mode = FCEUFILE::READ;
		fp->size = totalSize;
		fp->stream = ms;
		fp->archiveCount = (int)asr.numFilesInArchive;
		ms->fseek(0,SEEK_SET);
	}

	archive_read_free(a);

	return fp;
}

#endif

FCEUFILE* fceWrapper_OpenArchive(ArchiveScanRecord& asr, std::string& fname, std::string* innerFilename, int* userCancel)
{
	FCEUFILE* fp = nullptr;
	std::string searchFile;

	if ( innerFilename != NULL )
	{
		searchFile = *innerFilename;
	}
	else
	{
		std::vector <std::string> fileList;

		for (size_t i=0; i<asr.files.size(); i++)
		{
			char base[512], suffix[128];

			getFileBaseName( asr.files[i].name.c_str(), base, suffix );

			if ( (strcasecmp( suffix, ".nes" ) == 0) ||
			     (strcasecmp( suffix, ".nsf" ) == 0) ||
			     (strcasecmp( suffix, ".fds" ) == 0) ||
			     (strcasecmp( suffix, ".unf" ) == 0) ||
			     (strcasecmp( suffix, ".unif") == 0) )
			{
				fileList.push_back( asr.files[i].name );
			}
		}

		if ( fileList.size() > 1 )
		{
			if ( consoleWindow != NULL )
			{
				int sel = consoleWindow->showListSelectDialog( "Select ROM From Archive", fileList );

				if ( sel < 0 )
				{
					if ( userCancel )
					{
						*userCancel = 1;
					}
					return fp;
				}
				searchFile = fileList[sel];
			}
		}
		else if ( fileList.size() > 0 )
		{
			searchFile = fileList[0];
		}
	}

#ifdef _USE_LIBARCHIVE
	fp = libarchive_OpenArchive(asr, fname, &searchFile, -1 );
#endif

	if (fp == nullptr)
	{
		fp = minizip_OpenArchive(asr, fname, &searchFile, -1 );
	}
	return fp;
}

FCEUFILE* FCEUD_OpenArchive(ArchiveScanRecord& asr, std::string& fname, std::string* innerFilename)
{
	int userCancel = 0;

	return FCEUD_OpenArchive( asr, fname, innerFilename, &userCancel );
}

FCEUFILE* fceWrapper_OpenArchiveIndex(ArchiveScanRecord& asr, std::string &fname, int innerIndex, int* userCancel)
{
	FCEUFILE* fp = nullptr;

#ifdef _USE_LIBARCHIVE
	fp = libarchive_OpenArchive( asr, fname, nullptr, innerIndex );
#endif
	if (fp == nullptr)
	{
		fp = minizip_OpenArchive(asr, fname, nullptr, innerIndex);
	}

	return fp;
}

FCEUFILE* FCEUD_OpenArchiveIndex(ArchiveScanRecord& asr, std::string &fname, int innerIndex)
{
	int userCancel = 0;

	return FCEUD_OpenArchiveIndex( asr, fname, innerIndex, &userCancel );
}
