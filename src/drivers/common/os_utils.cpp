// os_util.cpp
#include <stdio.h>
#include <stdlib.h>

#ifndef WIN32
#error "Platform not supported"
#endif

#include <windows.h>
#include <direct.h>
#include <io.h>

#include "common/os_utils.h"
//************************************************************
int fceu_mkdir( const char *path )
{
	int retval;
	retval = _mkdir(path);
	_chmod(path, 755);
	return retval;
}
//************************************************************
int fceu_mkpath( const char *path )
{
	int i, retval = 0;
	char p[512];

	i=0;
	while ( path[i] != 0 )
	{
		if ( path[i] == '/' )
		{
			if ( i > 0 )
			{
				p[i] = 0;

				retval = fceu_mkdir( p );

				if ( retval )
				{
					return retval;
				}
			}
		}
		p[i] = path[i]; i++;
	}
	p[i] = 0;

	retval = fceu_mkdir( p );

	return retval;
}
//************************************************************
bool fceu_file_exists( const char *filepath )
{
	FILE *fp;
	fp = ::fopen( filepath, "r" );

	if ( fp != NULL )
	{
		::fclose(fp);
		return true;
	}
	return false;
}
//************************************************************
int msleep( int ms )
{
	Sleep(ms);
	return 0;
}
//************************************************************
