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
/// \file
/// \brief Contains methods related to the build configuration

#include "types.h"
#include "version.h"
#include "fceu.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "utils/memory.h"

#include <cstring>
#include <cstdio>
#include <cstdlib>

static std::string aboutString;

#ifndef FCEUX_BUILD_TIMESTAMP
#define FCEUX_BUILD_TIMESTAMP  __TIME__ " " __DATE__
#endif

//#pragma message( "Compiling using C++ Std: " __FCEU_STRINGIZE(__cplusplus) )

// returns a string suitable for use in an aboutbox
const char *FCEUI_GetAboutString(void) 
{
	const char *aboutTemplate =
		FCEU_NAME_AND_VERSION "\n\n"
		"Administrators:\n"
		"zeromus, feos\n"
		"\n"
		"Current Contributors:\n"
		"CaH4e3, rainwarrior, owomomo, punkrockguy318, Cluster\n"
		"\n"
		"Past Contributors:\n"
		"xhainingx, gocha, AnS, mjbudd77\n"
		"\n"
		"FCEUX 2.0:\n"
		"mz, nitsujrehtona, SP, Ugly Joe,\n"
		"Plombo, qeed, QFox, Shinydoofy\n"
		"ugetab, Acmlm, DWEdit\n"
		"\n"
		"Previous versions:\n"
		"FCE - Bero\n"
		"FCEU - Xodnizel\n"
		"FCEU XD - Bbitmaster & Parasyte\n"
		"FCEU XD SP - Sebastian Porst\n"
		"FCEU MM - CaH4e3\n"
		"FCEU TAS - blip & nitsuja\n"
		"FCEU TAS+ - Luke Gustafson\n"
		"\n"
		"Logo/icon:\n"
		"Terwilf\n"
		"\n"
		"FCEUX is dedicated to the fallen heroes\n"
		"of NES emulation. In Memoriam --\n"
		"ugetab\n"
		"\n"
		"\n"
		FCEUX_BUILD_TIMESTAMP "\n";

	if (aboutString.size() > 0) return aboutString.c_str();

	const char *compilerString = FCEUD_GetCompilerString();

	char  cppVersion[128];

	snprintf( cppVersion, sizeof(cppVersion), "\nCompiled using C++ Language Standard: %li\n", __cplusplus);

	aboutString.assign( aboutTemplate  );
	aboutString.append( compilerString );
	aboutString.append( cppVersion     );

	return aboutString.c_str();
}
