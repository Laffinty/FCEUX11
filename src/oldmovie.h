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
#ifndef _OLDMOVIE_H_
#define _OLDMOVIE_H_

#include "movie.h"

enum EFCM_CONVERTRESULT
{
	FCM_CONVERTRESULT_SUCCESS,
	FCM_CONVERTRESULT_FAILOPEN,
	FCM_CONVERTRESULT_OLDVERSION,
	FCM_CONVERTRESULT_UNSUPPORTEDVERSION,
	FCM_CONVERTRESULT_STARTFROMSAVESTATENOTSUPPORTED,
	FCM_CONVERTRESULT_REALLOC_FAIL,
};

inline const char * EFCM_CONVERTRESULT_message(EFCM_CONVERTRESULT e)
{
	static const char * errmsg[] = {
		"Success",
		"Failed to open input file",
		"This is a 'version 1' movie file. These are not supported yet.",
		"This is an unsupported ancient version",
		"Movies starting from savestates are not supported"
	};

	return errmsg[e];
}

EFCM_CONVERTRESULT convert_fcm(MovieData& md, std::string fname);

#endif
