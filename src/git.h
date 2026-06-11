#ifndef __FCEU_GIT
#define __FCEU_GIT

#include <cstdint>

enum EGIT
{
	GIT_CART	= 0,  //Cart
	GIT_VSUNI	= 1,  //VS Unisystem
	GIT_FDS		= 2,  // Famicom Disk System
	GIT_NSF		= 3,  //NES Sound Format
};

enum EGIV
{
	GIV_NTSC	= 0,  //NTSC emulation.
	GIV_PAL		= 1,  //PAL emulation.
	GIV_USER	= 2,  //What was set by FCEUI_SetVidSys().
};

enum EGIPPU
{
	GIPPU_USER			= 0,
	GIPPU_RP2C04_0001	= 1,
	GIPPU_RP2C04_0002	= 2,
	GIPPU_RP2C04_0003	= 3,
	GIPPU_RP2C04_0004	= 4,
	GIPPU_RC2C03B		= 5,
	GIPPU_RC2C05_01		= 6,
	GIPPU_RC2C05_02		= 7,
	GIPPU_RC2C05_03		= 8,
	GIPPU_RC2C05_04		= 9,
};

enum EGIVS
{
	EGIVS_NORMAL	= 0,
	EGIVS_RBI		= 1, // RBI Baseball protection
	EGIVS_TKO		= 2, // TKO Boxing protection
	EGIVS_XEVIOUS	= 3, // Super Xevious protection
};

enum ESIS
{
	SIS_NONE		= 0,
	SIS_DATACH		= 1,
	SIS_NWC			= 2,
	SIS_VSUNISYSTEM	= 3, // Is it used?
	SIS_NSF			= 4,
};

//input device types for the standard joystick port.
//
// v0.3.8: per plan v3 §5 v0.3.8 task 1, the pre-v0.3.x `enum ESI {…}`
// is now `enum class fceu11::InputDevice : int8_t`. Plan §5 specifies
// `: u8` but the value list includes SI_UNSET = -1 (used by ROM
// detection paths to mean "unknown desired input"), so we use int8_t
// to preserve the sentinel. Byte-level savestate consistency (plan
// §1.3 iron-rule 1) is unaffected because FCEUGI::input[2] / inputfc
// are not in any SFORMAT serialization map.
//
// The legacy `ESI` type name and the SI_* enumerator names are
// preserved as global `using` / `inline constexpr` aliases below per
// plan §6.1 phase 1 ("only NEW symbols enter fceu11::; OLD symbols
// stay global"). All 200+ in-tree call sites — switch labels,
// strcmp("SI_GAMEPAD") round-trips in Qt config persistence, and the
// static_cast in src/ines.cpp:153,166 — continue to compile unchanged.
// Qt QComboBox::addItem(text, userData) call sites are the one
// exception: QVariant rejects enum class values implicitly, so the
// affected `addItem(..., SI_*)` lines in src/drivers/Qt/InputConf.cpp
// and src/drivers/Qt/input.cpp gain explicit `(int)` casts.
namespace fceu11 {
    enum class InputDevice : int8_t
    {
        Unset        = -1,
        None         = 0,
        Gamepad      = 1,
        Zapper       = 2,
        PowerPadA    = 3,
        PowerPadB    = 4,
        Arkanoid     = 5,
        Mouse        = 6,
        Snes         = 7,
        SnesMouse    = 8,
        VirtualBoy   = 9,
        LcdCompZapper = 10,
        Count        = LcdCompZapper, // sentinel: highest valid value
    };
} // namespace fceu11

using ESI = fceu11::InputDevice;
// Legacy SI_* constants — `inline constexpr int` (not ESI-typed). The
// pre-v0.3.8 codebase routinely stores ESI values in `int` arrays
// (e.g. src/drivers/Qt/input.cpp:63-64 `static int UsrInputType[…]` /
// `CurInputType[…]`) and uses SI_* as case labels in `switch(int)` blocks.
// Keeping the legacy constants as int — same pattern as FCEUIOD_*
// (src/driver.h) — preserves the existing 100+ in-tree call sites
// without a per-site cast cascade. Type safety lives at the API
// boundary (FCEUI_SetInput / FCEUGI::input[]), which uses the
// ESI / fceu11::InputDevice typed form.
inline constexpr int SI_UNSET         = static_cast<int>(ESI::Unset);
inline constexpr int SI_NONE          = static_cast<int>(ESI::None);
inline constexpr int SI_GAMEPAD       = static_cast<int>(ESI::Gamepad);
inline constexpr int SI_ZAPPER        = static_cast<int>(ESI::Zapper);
inline constexpr int SI_POWERPADA     = static_cast<int>(ESI::PowerPadA);
inline constexpr int SI_POWERPADB     = static_cast<int>(ESI::PowerPadB);
inline constexpr int SI_ARKANOID      = static_cast<int>(ESI::Arkanoid);
inline constexpr int SI_MOUSE         = static_cast<int>(ESI::Mouse);
inline constexpr int SI_SNES          = static_cast<int>(ESI::Snes);
inline constexpr int SI_SNES_MOUSE    = static_cast<int>(ESI::SnesMouse);
inline constexpr int SI_VIRTUALBOY    = static_cast<int>(ESI::VirtualBoy);
inline constexpr int SI_LCDCOMP_ZAPPER = static_cast<int>(ESI::LcdCompZapper);
inline constexpr int SI_COUNT         = static_cast<int>(ESI::Count);



inline const char* ESI_Name(ESI esi)
{
	static const char * const names[] =
	{
		"<none>",
		"Gamepad",
		"Zapper",
		"Power Pad A",
		"Power Pad B",
		"Arkanoid Paddle",
		"Subor Mouse",
		"SNES Pad",
		"SNES Mouse",
		"Virtual Boy",
		"LCD Zapper (Advance)"
	};

	// v0.3.8: enum class doesn't implicitly convert to size_t, so cast
	// explicitly. The >= 0 guard rejects ESI::Unset (= -1) without UB.
	const int idx = static_cast<int>(esi);
	if (idx >= SI_NONE && idx <= SI_COUNT)
		return names[idx];
	else return "<invalid ESI>";
}


//input device types for the expansion port.
// v0.3.8: same treatment as InputDevice — see comments above.
namespace fceu11 {
    enum class InputDeviceFC : int8_t
    {
        Unset            = -1,
        None             = 0,
        Arkanoid         = 1,
        Shadow           = 2,
        FourPlayer       = 3,
        Fkb              = 4,
        SuborKb          = 5,
        Pec586Kb         = 6,
        Hypershot        = 7,
        Mahjong          = 8,
        QuizKing         = 9,
        FTrainerA        = 10,
        FTrainerB        = 11,
        OekaKids         = 12,
        BWorld           = 13,
        TopRider         = 14,
        FamicomNetSystem = 15,
        Hori4Player      = 16,
        Count            = Hori4Player, // sentinel: highest valid value
    };
} // namespace fceu11

using ESIFC = fceu11::InputDeviceFC;
// Legacy SIFC_* constants — `inline constexpr int` (see SI_* comment).
inline constexpr int SIFC_UNSET       = static_cast<int>(ESIFC::Unset);
inline constexpr int SIFC_NONE        = static_cast<int>(ESIFC::None);
inline constexpr int SIFC_ARKANOID    = static_cast<int>(ESIFC::Arkanoid);
inline constexpr int SIFC_SHADOW      = static_cast<int>(ESIFC::Shadow);
inline constexpr int SIFC_4PLAYER     = static_cast<int>(ESIFC::FourPlayer);
inline constexpr int SIFC_FKB         = static_cast<int>(ESIFC::Fkb);
inline constexpr int SIFC_SUBORKB     = static_cast<int>(ESIFC::SuborKb);
inline constexpr int SIFC_PEC586KB    = static_cast<int>(ESIFC::Pec586Kb);
inline constexpr int SIFC_HYPERSHOT   = static_cast<int>(ESIFC::Hypershot);
inline constexpr int SIFC_MAHJONG     = static_cast<int>(ESIFC::Mahjong);
inline constexpr int SIFC_QUIZKING    = static_cast<int>(ESIFC::QuizKing);
inline constexpr int SIFC_FTRAINERA   = static_cast<int>(ESIFC::FTrainerA);
inline constexpr int SIFC_FTRAINERB   = static_cast<int>(ESIFC::FTrainerB);
inline constexpr int SIFC_OEKAKIDS    = static_cast<int>(ESIFC::OekaKids);
inline constexpr int SIFC_BWORLD      = static_cast<int>(ESIFC::BWorld);
inline constexpr int SIFC_TOPRIDER    = static_cast<int>(ESIFC::TopRider);
inline constexpr int SIFC_FAMINETSYS  = static_cast<int>(ESIFC::FamicomNetSystem);
inline constexpr int SIFC_HORI4PLAYER = static_cast<int>(ESIFC::Hori4Player);
inline constexpr int SIFC_COUNT       = static_cast<int>(ESIFC::Count);


inline const char* ESIFC_Name(ESIFC esifc)
{
	static const char * const names[] =
	{
		"<none>",
		"Arkanoid Paddle",
		"Hyper Shot gun",
		"4-Player Adapter",
		"Family Keyboard",
		"Subor Keyboard",
		"PEC586 Keyboard",
		"HyperShot Pads",
		"Mahjong",
		"Quiz King Buzzers",
		"Family Trainer A",
		"Family Trainer B",
		"Oeka Kids Tablet",
		"Barcode World",
		"Top Rider",
		"Famicom Network Controller",
		"Hori 4-Player Adapter"
	};

	// v0.3.8: enum class doesn't implicitly convert to size_t — see ESI_Name.
	const int idx = static_cast<int>(esifc);
	if (idx >= SIFC_NONE && idx <= SIFC_COUNT)
		return names[idx];
	else return "<invalid ESIFC>";
}


#include "utils/md5.h"

struct FCEUGI
{
	FCEUGI();
	~FCEUGI();

	uint8 *name;	//Game name, UTF8 encoding
	int mappernum;

	EGIT type;
	EGIV vidsys;    //Current emulated video system;
	ESI input[2];   //Desired input for emulated input ports 1 and 2; -1 for unknown desired input.
	ESIFC inputfc;  //Desired Famicom expansion port device. -1 for unknown desired input.
	ESIS cspecial;  //Special cart expansion: DIP switches, barcode reader, etc.
	EGIPPU vs_ppu;	//PPU type for Vs. System
	EGIVS vs_type;	//Vs. System type
	uint8 vs_cswitch; // Switch first and second controllers for Vs. System

	MD5DATA MD5;

	//mbg 6/8/08 - ???
	int soundrate;  //For Ogg Vorbis expansion sound wacky support.  0 for default.
	int soundchan;  //Number of sound channels.

	char* filename;
	char* archiveFilename;
	int archiveCount;
};

#endif
