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


#include "types.h"
#include "file.h"
#include "utils/endian.h"
#include "netplay.h"
#include "fceu.h"
#include "state.h"
#include "cheat.h"
#include "input.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "utils/memory.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <sys/types.h>
#include <sys/stat.h>
//#include <unistd.h> //mbg merge 7/17/06 removed

#include <zlib.h>

int FCEUnetplay=0;

static uint8 netjoy[4]; // Controller cache.
static int numlocal;
static int netdivisor;
static int netdcount;

//NetError should only be called after a FCEUD_*Data function returned 0, in the function
//that called FCEUD_*Data, to prevent it from being called twice.

static void NetError(void)
{
	FCEU_DispMessage("Network error/connection lost!",0);
	FCEUD_NetworkClose();
}

void FCEUI_NetplayStop(void)
{
	if(FCEUnetplay)
	{
		FCEUnetplay = 0;
		FCEU_FlushGameCheats(0,1);  //Don't save netplay cheats.
		FCEU_LoadGameCheats(0);    //Reload our original cheats.
	}
	else puts("Check your code!");
}

int FCEUI_NetplayStart(int nlocal, int divisor)
{
	FCEU_FlushGameCheats(0, 0);  //Save our pre-netplay cheats.
	FCEU_LoadGameCheats(0);    // Load them again, for pre-multiplayer action.

	FCEUnetplay = 1;
	memset(netjoy,0,sizeof(netjoy));
	numlocal = nlocal;
	netdivisor = divisor;
	netdcount = 0;
	return(1);
}

int FCEUNET_SendCommand(uint8 cmd, uint32 len)
{
	//mbg merge 7/17/06 changed to alloca
	//uint8 buf[numlocal + 1 + 4];
	uint8 *buf = (uint8*)alloca(numlocal+1+4);


	buf[0] = 0xFF;
	FCEU_en32lsb(&buf[numlocal], len);
	buf[numlocal + 4] = cmd;
	if(!FCEUD_SendData(buf,numlocal + 1 + 4))
	{
		NetError();
		return(0);
	}
	return(1);
}

void FCEUI_NetplayText(uint8 *text)
{
	uint32 len;

	len = strlen((char*)text); //mbg merge 7/17/06 added cast

	if(!FCEUNET_SendCommand(FCEUNPCMD_TEXT,len)) return;

	if(!FCEUD_SendData(text,len))
		NetError();
}

int FCEUNET_SendFile(uint8 cmd, char *fn)
{
	uint32 len;
	uLongf clen;
	char *buf, *cbuf;
	FILE *fp;
	struct stat sb;

	if(!(fp=FCEUD_UTF8fopen(fn,"rb"))) return(0);

	FCEUX_fstat(fileno(fp),&sb);
	len = sb.st_size;
	buf = (char*)FCEU_malloc(len); //mbg merge 7/17/06 added cast
	if ( fread(buf, 1, len, fp) != static_cast<size_t>(len) )
	{
		FCEU_printf("Warning: FCEUNET_SendFile failed to load complete file.\n");
	}
	fclose(fp);

	cbuf = (char*)FCEU_malloc(4 + len + len / 1000 + 12); //mbg merge 7/17/06 added cast
	FCEU_en32lsb((uint8*)cbuf, len); //mbg merge 7/17/06 added cast
	compress2((uint8*)cbuf + 4, &clen, (uint8*)buf, len, 7); //mbg merge 7/17/06 added casts
	FCEU_free(buf);

	//printf("Sending file: %s, %d, %d\n",fn,len,clen);

	len = clen + 4;

	if(!FCEUNET_SendCommand(cmd,len))
	{
		FCEU_free(cbuf);
		return(0);
	}
	if(!FCEUD_SendData(cbuf, len))
	{
		NetError();
		FCEU_free(cbuf);
		return(0);
	}
	FCEU_free(cbuf);

	return(1);
}

static FILE *FetchFile(uint32 remlen)
{
	uint32 clen = remlen;
	char *cbuf;
	uLongf len;
	char *buf;
	FILE *fp;

	if(clen > 500000)  // Sanity check
	{
		NetError();
		return(0);
	}

	//printf("Receiving file: %d...\n",clen);
	if((fp = tmpfile()))
	{
		cbuf = (char *)FCEU_malloc(clen); //mbg merge 7/17/06 added cast
		if(!FCEUD_RecvData(cbuf, clen))
		{
			NetError();
			fclose(fp);
			FCEU_free(cbuf);
			return(0);
		}

		len = FCEU_de32lsb((uint8*)cbuf); //mbg merge 7/17/06 added cast
		if(len > 500000)    // Another sanity check
		{
			NetError();
			fclose(fp);
			FCEU_free(cbuf);
			return(0);
		}
		buf = (char *)FCEU_malloc(len); //mbg merge 7/17/06 added cast
		uncompress((uint8*)buf, &len, (uint8*)cbuf + 4, clen - 4); //mbg merge 7/17/06 added casts

		fwrite(buf, 1, len, fp);
		FCEU_free(buf);
		fseek(fp, 0, SEEK_SET);
		return(fp);
	}
	return(0);
}

void NetplayUpdate(uint8 *joyp)
{
	static uint8 buf[5];  /* 4 play states, + command/extra byte */
	static uint8 joypb[4];

	memcpy(joypb,joyp,4);

	/* This shouldn't happen, but just in case.  0xFF is used as a command escape elsewhere. */
	if(joypb[0] == 0xFF)
		joypb[0] = 0xF;
	if(!netdcount)
		if(!FCEUD_SendData(joypb,numlocal))
		{
			NetError();
			return;
		}

	if(!netdcount)
	{
		do
		{
			if(!FCEUD_RecvData(buf,5))
			{
				NetError();
				return;
			}

			switch(buf[4])
			{
			default: FCEU_DoSimpleCommand(buf[4]);break;
			case FCEUNPCMD_TEXT:
				{
					uint8 *tbuf;
					uint32 len = FCEU_de32lsb(buf);

					if(len > 100000)  // Insanity check!
					{
						NetError();
						return;
					}
					// v1.13 Purify F2d: FCEU_malloc replaces raw malloc; matching FCEU_free
					tbuf = (uint8*)FCEU_malloc(len + 1);
					tbuf[len] = 0;
					if(!FCEUD_RecvData(tbuf, len))
					{
						NetError();
						FCEU_free(tbuf);
						return;
					}
					FCEUD_NetplayText(tbuf);
					FCEU_free(tbuf);
				}
				break;
			case FCEUNPCMD_SAVESTATE:
				{
				}
				break;
			case FCEUNPCMD_LOADCHEATS:
				{
					FILE *fp = FetchFile(FCEU_de32lsb(buf));
					if(!fp) return;
					FCEU_FlushGameCheats(0,1);
					FCEU_LoadGameCheats(fp);
				}
				break;
			}
		} while(buf[4]);

		netdcount=(netdcount+1)%netdivisor;

		memcpy(netjoy,buf,4);
		*(uint32 *)joyp=*(uint32 *)netjoy;
	}
}
