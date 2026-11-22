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
#ifndef _aosdfjk02fmasf
#define _aosdfjk02fmasf

#include <stdint.h>
#include <list>

#include <QWidget>
#include <QKeySequence>
#include <QShortcut>
#include <QAction>

#include "common/configSys.h"

//#define MAXBUTTCONFIG   4

enum {
  BUTTC_KEYBOARD  = 0,
  BUTTC_JOYSTICK  = 1,
  BUTTC_MOUSE     = 2
};
struct ButtConfig
{
	int    ButtType; 
	int    DeviceNum; 
	int    ButtonNum; 
	int    state;
};

extern int NoWaiting;
extern CFGSTRUCT InputConfig[];
extern ARGPSTRUCT InputArgs[];
void ParseGIInput(FCEUGI *GI);
void setHotKeys(void);
int getKeyState( int k );

// Whether a binding is held right now, whatever kind of binding it is.
//
// v2.0 S4. `DTestButton` knows both shapes a binding can take -- a keyboard
// key, and a joystick axis or button packed with flags into `ButtonNum` -- and
// the keyboard-only `getKeyState` path silently reads an axis as a keycode and
// answers "not pressed" forever. The d-pad is bound to axes by default here, so
// a GBA reading the pad through the keyboard path alone got a pad that could
// never move.
//
// Exposed rather than duplicated: the alternative is a second implementation
// of "is this binding down", which is the sort of copy that drifts.
int testButtonBinding(const ButtConfig* bc);
int ButtonConfigBegin();
void ButtonConfigEnd();
void ConfigButton(char *text, ButtConfig *bc);
int DWaitButton(const uint8_t *text, ButtConfig *bc, int *buttonConfigStatus = NULL);

class hotkey_t
{
	public:
		// Methods
		hotkey_t(void);

		int init( QWidget *parent );

		int readConfig(void);

		int getState(void);

		int getRisingEdge(void);

		int getString( char *s, size_t size );

		void setModifierFromString( const char *s );

		void setConfigName(const char *cName);
		void setAction( QAction *act );

		// hotfix4 D-4/D-5: re-apply the "\t<shortcut>" suffix using the
		// current action text (i.e. the latest tr()-translated label).
		// Called from readConfig() and from retranslateUi().
		void refreshText(void);

		const char *getConfigName(void);
		QShortcut *getShortcut(void);

		QKeySequence getKeySeq(void){ return keySeq; };

		// Member variables
		struct 
		{
			int value;
			int modifier;
		} sdl;

		struct
		{
			int value;
			int modifier;
		} qkey;

		char prevState;

	private:
		void conv2SDL(void);

		const char *configName;
		QKeySequence keySeq;
		QShortcut *shortcut;
		QAction   *act;
		// hotfix4 D-4: removed actText snapshot. Use refreshText() instead.
};
extern class hotkey_t Hotkeys[];

struct gamepad_function_key_t
{
	struct {
		int  key;
		unsigned int modifier;
		std::string  name;

	} keySeq[2];

	int  hk[2];
	char  keyRelReq[2];

	struct ButtConfig  bmap[2];

	gamepad_function_key_t(void);
	~gamepad_function_key_t(void);

	void sendKeyPressEvent(int idx);
	void sendKeyReleaseEvent(int idx);

	void updateStatus(void);
};

//extern std::list <gamepad_function_key_t*> gpKeySeqList;

#define FCFGD_GAMEPAD   1
#define FCFGD_POWERPAD  2
#define FCFGD_HYPERSHOT 3
#define FCFGD_QUIZKING  4

#define SDL_FCEU_HOTKEY_EVENT	SDL_USEREVENT

void InitInputInterface(void);
void InputUserActiveFix(void);

extern bool replaceP2StartWithMicrophone;
//extern ButtConfig GamePadConfig[4][10];
//extern ButtConfig powerpadsc[2][12];
//extern ButtConfig QuizKingButtons[6];
//extern ButtConfig FTrainerButtons[12];
extern ButtConfig fkbmap[FAMILYKEYBOARD_NUM_BUTTONS];

void IncreaseEmulationSpeed(void);
void DecreaseEmulationSpeed(void);
int CustomEmulationSpeed(int spdPercent);

int DTestButtonJoy(ButtConfig *bc);

void FCEUD_UpdateInput(void);

// v2.0.4 V15: per-frame handoff of the GUI-written gamepad state to the
// emulation thread's snapshot (JSreturnGui -> JSreturnEmu). Must be called
// from the emulation thread, once per frame, before the core runs.
void emuThreadPollInputs(void);

void UpdateInput(Config *config);

const char* ButtonName(const ButtConfig* bc);

int getInputSelection( int port, int *cur, int *usr );
int saveInputSettingsToFile( const char *fileBase = NULL );
int loadInputSettingsFromFile( const char *filename = NULL );
void toggleFamilyKeyboardFunc(void);
bool isFamilyKeyboardActv(void);
const uint8_t *getFamilyKeyboardState(void);
const uint8_t *QtSDL_getKeyboardState( int *bufSize );

#endif

