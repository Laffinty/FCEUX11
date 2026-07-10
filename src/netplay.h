int InitNetplay(void);
void NetplayUpdate(uint8 *joyp);
extern int FCEUnetplay;

// v1.13 Purify H: #define → constexpr (netplay command codes)
inline constexpr uint8_t FCEUNPCMD_RESET      = 0x01;
inline constexpr uint8_t FCEUNPCMD_POWER      = 0x02;
inline constexpr uint8_t FCEUNPCMD_VSUNICOIN  = 0x07;
inline constexpr uint8_t FCEUNPCMD_VSUNIDIP0  = 0x08;
inline constexpr uint8_t FCEUNPCMD_VSUNICOIN2 = 0x20;
inline constexpr uint8_t FCEUNPCMD_VSUNISERVICE = 0x21;
inline constexpr uint8_t FCEUNPCMD_FDSINSERTx = 0x10;
inline constexpr uint8_t FCEUNPCMD_FDSINSERT  = 0x18;
inline constexpr uint8_t FCEUNPCMD_FDSSELECT  = 0x1A;
inline constexpr uint8_t FCEUNPCMD_LOADSTATE   = 0x80;
inline constexpr uint8_t FCEUNPCMD_SAVESTATE   = 0x81;
inline constexpr uint8_t FCEUNPCMD_LOADCHEATS  = 0x82;
inline constexpr uint8_t FCEUNPCMD_TEXT        = 0x90;

int FCEUNET_SendCommand(uint8, uint32);
int FCEUNET_SendFile(uint8 cmd, char *);
