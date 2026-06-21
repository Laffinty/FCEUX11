//! Auto-generated static data for iNES parsing.
//! Extracted from src/ines.cpp, src/ines-correct.h, src/ines-bad.h.

// Input-controller constants (mirrors src/git.h)
pub const SI_UNSET: i32 = -1;
pub const SI_NONE: i32 = 0;
pub const SI_GAMEPAD: i32 = 1;
pub const SI_ZAPPER: i32 = 2;
pub const SI_POWERPADA: i32 = 3;
pub const SI_POWERPADB: i32 = 4;
pub const SI_ARKANOID: i32 = 5;
pub const SI_MOUSE: i32 = 6;
pub const SI_SNES_MOUSE: i32 = 8;

pub const SIFC_UNSET: i32 = -1;
pub const SIFC_NONE: i32 = 0;
pub const SIFC_ARKANOID: i32 = 1;
pub const SIFC_SHADOW: i32 = 2;
pub const SIFC_4PLAYER: i32 = 3;
pub const SIFC_FKB: i32 = 4;
pub const SIFC_SUBORKB: i32 = 5;
pub const SIFC_PEC586KB: i32 = 6;
pub const SIFC_HYPERSHOT: i32 = 7;
pub const SIFC_MAHJONG: i32 = 8;
pub const SIFC_QUIZKING: i32 = 9;
pub const SIFC_FTRAINERA: i32 = 10;
pub const SIFC_FTRAINERB: i32 = 11;
pub const SIFC_OEKAKIDS: i32 = 12;
pub const SIFC_BWORLD: i32 = 13;
pub const SIFC_TOPRIDER: i32 = 14;
pub const SIFC_FAMINETSYS: i32 = 15;

/// Mapper number to human-readable name (extracted from bmap[])
pub static MAPPER_NAMES: &[(i32, &str)] = &[
    (0, "NROM"),
    (1, "MMC1"),
    (2, "UNROM"),
    (3, "CNROM"),
    (4, "MMC3"),
    (5, "MMC5"),
    (6, "FFE Rev. A"),
    (7, "ANROM"),
    (9, "MMC2"),
    (10, "MMC4"),
    (11, "Color Dreams"),
    (12, "REX DBZ 5"),
    (13, "CPROM"),
    (14, "REX SL-1632"),
    (15, "100-in-1"),
    (16, "BANDAI 24C02"),
    (17, "FFE Rev. B"),
    (18, "JALECO SS880006"),
    (19, "Namcot 106"),
    (21, "Konami VRC2/VRC4 A"),
    (22, "Konami VRC2/VRC4 B"),
    (23, "Konami VRC2/VRC4 C"),
    (24, "Konami VRC6 Rev. A"),
    (25, "Konami VRC2/VRC4 D"),
    (26, "Konami VRC6 Rev. B"),
    (27, "CC-21 MI HUN CHE"),
    (29, "RET-CUFROM"),
    (30, "UNROM 512"),
    (31, "infiniteneslives-NSF"),
    (32, "IREM G-101"),
    (33, "TC0190FMC/TC0350FMR"),
    (34, "IREM I-IM/BNROM"),
    (35, "Wario Land 2"),
    (36, "TXC Policeman"),
    (37, "PAL-ZZ SMB/TETRIS/NWC"),
    (38, "Bit Corp."),
    (40, "SMB2j FDS"),
    (41, "CALTRON 6-in-1"),
    (42, "BIO MIRACLE FDS"),
    (43, "FDS SMB2j LF36"),
    (44, "MMC3 BMC PIRATE A"),
    (45, "MMC3 BMC PIRATE B"),
    (46, "RUMBLESTATION 15-in-1"),
    (47, "NES-QJ SSVB/NWC"),
    (48, "TAITO TCxxx"),
    (49, "MMC3 BMC PIRATE C"),
    (50, "SMB2j FDS Rev. A"),
    (51, "11-in-1 BALL SERIES"),
    (52, "MMC3 BMC PIRATE D"),
    (53, "SUPERVISION 16-in-1"),
    (57, "SIMBPLE BMC PIRATE A"),
    (58, "SIMBPLE BMC PIRATE B"),
    (60, "SIMBPLE BMC PIRATE C"),
    (61, "20-in-1 KAISER Rev. A"),
    (62, "700-in-1"),
    (64, "TENGEN RAMBO1"),
    (65, "IREM-H3001"),
    (66, "MHROM"),
    (67, "SUNSOFT-FZII"),
    (68, "Sunsoft Mapper #4"),
    (69, "SUNSOFT-5/FME-7"),
    (70, "BA KAMEN DISCRETE"),
    (71, "CAMERICA BF9093"),
    (72, "JALECO JF-17"),
    (73, "KONAMI VRC3"),
    (74, "TW MMC3+VRAM Rev. A"),
    (75, "KONAMI VRC1"),
    (76, "NAMCOT 108 Rev. A"),
    (77, "IREM LROG017"),
    (78, "Irem 74HC161/32"),
    (79, "AVE/C&E/TXC BOARD"),
    (80, "TAITO X1-005 Rev. A"),
    (82, "TAITO X1-017"),
    (83, "YOKO VRC Rev. B"),
    (85, "KONAMI VRC7"),
    (86, "JALECO JF-13"),
    (87, "74*139/74 DISCRETE"),
    (88, "NAMCO 3433"),
    (89, "SUNSOFT-3"),
    (90, "HUMMER/JY BOARD"),
    (91, "EARLY HUMMER/JY BOARD"),
    (92, "JALECO JF-19"),
    (93, "SUNSOFT-3R"),
    (94, "HVC-UN1ROM"),
    (95, "NAMCOT 108 Rev. B"),
    (96, "BANDAI OEKAKIDS"),
    (97, "IREM TAM-S1"),
    (99, "VS Uni/Dual- system"),
    (103, "FDS DOKIDOKI FULL"),
    (105, "NES-EVENT NWC1990"),
    (106, "SMB3 PIRATE A"),
    (107, "MAGIC CORP A"),
    (108, "FDS UNROM BOARD"),
    (111, "Cheapocabra"),
    (112, "ASDER/NTDEC BOARD"),
    (113, "HACKER/SACHEN BOARD"),
    (114, "MMC3 SG PROT. A"),
    (115, "MMC3 PIRATE A"),
    (116, "MMC1/MMC3/VRC PIRATE"),
    (117, "FUTURE MEDIA BOARD"),
    (118, "TSKROM"),
    (119, "NES-TQROM"),
    (120, "FDS TOBIDASE"),
    (121, "MMC3 PIRATE PROT. A"),
    (123, "MMC3 PIRATE H2288"),
    (125, "FDS LH32"),
    (132, "TXC/MGENIUS 22111"),
    (133, "SA72008"),
    (134, "MMC3 BMC PIRATE"),
    (136, "TCU02"),
    (137, "S8259D"),
    (138, "S8259B"),
    (139, "S8259C"),
    (140, "JALECO JF-11/14"),
    (141, "S8259A"),
    (142, "UNLKS7032"),
    (143, "TCA01"),
    (144, "AGCI 50282"),
    (145, "SA72007"),
    (146, "SA0161M"),
    (147, "TCU01"),
    (148, "SA0037"),
    (149, "SA0036"),
    (150, "S74LS374N"),
    (153, "BANDAI SRAM"),
    (157, "BANDAI BARCODE"),
    (159, "BANDAI 24C01"),
    (160, "SA009"),
    (166, "SUBOR Rev. A"),
    (167, "SUBOR Rev. B"),
    (174, "NTDec 5-in-1"),
    (176, "BMCFK23C"),
    (192, "TW MMC3+VRAM Rev. B"),
    (193, "NTDEC TC-112"),
    (194, "TW MMC3+VRAM Rev. C"),
    (195, "TW MMC3+VRAM Rev. D"),
    (198, "TW MMC3+VRAM Rev. E"),
    (205, "JC-016-2"),
    (206, "NAMCOT 108 Rev. C"),
    (207, "TAITO X1-005 Rev. B"),
    (219, "UNLA9746"),
    (220, "Debug Mapper"),
    (221, "UNLN625092"),
    (226, "BMC 22+20-in-1"),
    (230, "BMC Contra+22-in-1"),
    (232, "BMC QUATTRO"),
    (233, "BMC 22+20-in-1 RST"),
    (234, "BMC MAXI"),
    (238, "UNL6035052"),
    (243, "S74LS374NA"),
    (244, "DECATHLON"),
    (246, "FONG SHEN BANG"),
    (252, "SAN GUO ZHI PIRATE"),
    (253, "DRAGON BALL PIRATE"),
    (256, "ONE-BUS Systems"),
    (257, "PEC-586 Computer"),
    (258, "158B Prot Board"),
    (259, "F-15 MMC3 Based"),
    (260, "HP10xx/H20xx Boards"),
    (261, "810544-CA-1"),
    (268, "AA6023/AA6023B"),
    (342, "COOLGIRL"),
    (354, "FAM250/81-01-39-C/SCHI-24"),
    (361, "OK-411"),
    (366, "GN-45"),
    (406, "Impact Soft MMC3 Flash Board"),
    (470, "INX_007T_V01"),
    (547, "KONAMI QTAi Board"),
];

/// Mappers whose PRG-ROM size is not a power of two.
pub static NOT_POWER2: &[i32] = &[53, 198, 228, 547];

/// CRC32-based controller selection (formerly SetInput)
pub struct InpSel {
    pub crc32: u32,
    pub input1: i32,
    pub input2: i32,
    pub inputfc: i32,
}
pub static INPSEL_CRC: &[InpSel] = &[
    InpSel {
        crc32: 0x19b0a9f1,
        input1: SI_GAMEPAD,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x29de87af,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERB,
    },
    InpSel {
        crc32: 0xd89e5a67,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_ARKANOID,
    },
    InpSel {
        crc32: 0x0f141525,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_ARKANOID,
    },
    InpSel {
        crc32: 0x32fb0583,
        input1: SI_UNSET,
        input2: SI_ARKANOID,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x60ad090a,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERA,
    },
    InpSel {
        crc32: 0x48ca0ee1,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_BWORLD,
    },
    InpSel {
        crc32: 0x4318a2f8,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x6cca1c1f,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERB,
    },
    InpSel {
        crc32: 0x24598791,
        input1: SI_GAMEPAD,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xd5d6eac4,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0xe9a7fe9e,
        input1: SI_UNSET,
        input2: SI_MOUSE,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x8f7b1669,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0xf7606810,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FKB,
    },
    InpSel {
        crc32: 0x895037bc,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FKB,
    },
    InpSel {
        crc32: 0xb2530afc,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FKB,
    },
    InpSel {
        crc32: 0xea90f3e2,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERB,
    },
    InpSel {
        crc32: 0xbba58be5,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERB,
    },
    InpSel {
        crc32: 0x3e58a87e,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xd9f45be9,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_QUIZKING,
    },
    InpSel {
        crc32: 0x1545bd13,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_QUIZKING,
    },
    InpSel {
        crc32: 0x4e959173,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xbeb8ab01,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xff24d794,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x21f85681,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_HYPERSHOT,
    },
    InpSel {
        crc32: 0x980be936,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_HYPERSHOT,
    },
    InpSel {
        crc32: 0x915a53a7,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_HYPERSHOT,
    },
    InpSel {
        crc32: 0x9fae4d46,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_MAHJONG,
    },
    InpSel {
        crc32: 0x7b44fb2a,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_MAHJONG,
    },
    InpSel {
        crc32: 0x2f128512,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERA,
    },
    InpSel {
        crc32: 0xbb33196f,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FKB,
    },
    InpSel {
        crc32: 0x8587ee00,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FKB,
    },
    InpSel {
        crc32: 0x543ab532,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x368c19a8,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x5ee6008e,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x370ceb65,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERB,
    },
    InpSel {
        crc32: 0x3a1694f9,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_4PLAYER,
    },
    InpSel {
        crc32: 0x9d048ea4,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_OEKAKIDS,
    },
    InpSel {
        crc32: 0x2a6559a1,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xedc3662b,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x912989dc,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FKB,
    },
    InpSel {
        crc32: 0x9044550e,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERA,
    },
    InpSel {
        crc32: 0xea90f3e2,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERB,
    },
    InpSel {
        crc32: 0x851eb9be,
        input1: SI_GAMEPAD,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x6435c095,
        input1: SI_GAMEPAD,
        input2: SI_POWERPADB,
        inputfc: SIFC_UNSET,
    },
    InpSel {
        crc32: 0xc043a8df,
        input1: SI_UNSET,
        input2: SI_MOUSE,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x2cf5db05,
        input1: SI_UNSET,
        input2: SI_MOUSE,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xad9c63e2,
        input1: SI_GAMEPAD,
        input2: SI_UNSET,
        inputfc: SIFC_SHADOW,
    },
    InpSel {
        crc32: 0x61d86167,
        input1: SI_GAMEPAD,
        input2: SI_POWERPADB,
        inputfc: SIFC_UNSET,
    },
    InpSel {
        crc32: 0xabb2f974,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x41ef9ac4,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x8b265862,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x82f1fb96,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x9f8f200a,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERA,
    },
    InpSel {
        crc32: 0xc7bcc981,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERA,
    },
    InpSel {
        crc32: 0xd74b2719,
        input1: SI_GAMEPAD,
        input2: SI_POWERPADB,
        inputfc: SIFC_UNSET,
    },
    InpSel {
        crc32: 0x74bea652,
        input1: SI_GAMEPAD,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x5e073a1b,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x589b6b0d,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x41401c6d,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSel {
        crc32: 0x23d17f5e,
        input1: SI_GAMEPAD,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xc3c0811d,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_OEKAKIDS,
    },
    InpSel {
        crc32: 0xde8fd935,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x47232739,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_TOPRIDER,
    },
    InpSel {
        crc32: 0x8a12a7d9,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FTRAINERB,
    },
    InpSel {
        crc32: 0xb8b9aca3,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0x5112dc21,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSel {
        crc32: 0xaf4010ea,
        input1: SI_GAMEPAD,
        input2: SI_POWERPADB,
        inputfc: SIFC_UNSET,
    },
    InpSel {
        crc32: 0x67b126b9,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_FAMINETSYS,
    },
    InpSel {
        crc32: 0x00000000,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_UNSET,
    },
];

/// NES 2.0 expansion-byte controller selection (formerly SetInputNes20)
pub struct InpSelNes20 {
    pub expansion_id: u8,
    pub input1: i32,
    pub input2: i32,
    pub inputfc: i32,
}
pub static INPSEL_NES20: &[InpSelNes20] = &[
    InpSelNes20 {
        expansion_id: 0x01,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_UNSET,
    },
    InpSelNes20 {
        expansion_id: 0x02,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_NONE,
    },
    InpSelNes20 {
        expansion_id: 0x03,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_4PLAYER,
    },
    InpSelNes20 {
        expansion_id: 0x04,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_NONE,
    },
    InpSelNes20 {
        expansion_id: 0x05,
        input1: SI_GAMEPAD,
        input2: SI_GAMEPAD,
        inputfc: SIFC_NONE,
    },
    InpSelNes20 {
        expansion_id: 0x07,
        input1: SI_ZAPPER,
        input2: SI_NONE,
        inputfc: SIFC_NONE,
    },
    InpSelNes20 {
        expansion_id: 0x08,
        input1: SI_UNSET,
        input2: SI_ZAPPER,
        inputfc: SIFC_NONE,
    },
    InpSelNes20 {
        expansion_id: 0x0a,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SHADOW,
    },
    InpSelNes20 {
        expansion_id: 0x0b,
        input1: SI_UNSET,
        input2: SI_POWERPADA,
        inputfc: SIFC_UNSET,
    },
    InpSelNes20 {
        expansion_id: 0x0c,
        input1: SI_UNSET,
        input2: SI_POWERPADB,
        inputfc: SIFC_UNSET,
    },
    InpSelNes20 {
        expansion_id: 0x0d,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FTRAINERA,
    },
    InpSelNes20 {
        expansion_id: 0x0e,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FTRAINERB,
    },
    InpSelNes20 {
        expansion_id: 0x0f,
        input1: SI_UNSET,
        input2: SI_ARKANOID,
        inputfc: SIFC_UNSET,
    },
    InpSelNes20 {
        expansion_id: 0x10,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_ARKANOID,
    },
    InpSelNes20 {
        expansion_id: 0x12,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_HYPERSHOT,
    },
    InpSelNes20 {
        expansion_id: 0x15,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_MAHJONG,
    },
    InpSelNes20 {
        expansion_id: 0x17,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_OEKAKIDS,
    },
    InpSelNes20 {
        expansion_id: 0x18,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_BWORLD,
    },
    InpSelNes20 {
        expansion_id: 0x1b,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_TOPRIDER,
    },
    InpSelNes20 {
        expansion_id: 0x23,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_FKB,
    },
    InpSelNes20 {
        expansion_id: 0x24,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_PEC586KB,
    },
    InpSelNes20 {
        expansion_id: 0x26,
        input1: SI_UNSET,
        input2: SI_UNSET,
        inputfc: SIFC_SUBORKB,
    },
    InpSelNes20 {
        expansion_id: 0x27,
        input1: SI_UNSET,
        input2: SI_MOUSE,
        inputfc: SIFC_SUBORKB,
    },
    InpSelNes20 {
        expansion_id: 0x28,
        input1: SI_UNSET,
        input2: SI_MOUSE,
        inputfc: SIFC_SUBORKB,
    },
    InpSelNes20 {
        expansion_id: 0x29,
        input1: SI_UNSET,
        input2: SI_SNES_MOUSE,
        inputfc: SIFC_UNSET,
    },
];

/// Bad ROM image database (formerly CheckBad)
pub struct BadRomEntry<'a> {
    pub md5partial: u64,
    pub name: &'a str,
}
pub static BAD_ROMS: &[BadRomEntry<'_>] = &[
    BadRomEntry {
        md5partial: 0xecf78d8a13a030a6u64,
        name: "Ai Sensei no Oshiete",
    },
    BadRomEntry {
        md5partial: 0x4712856d3e12f21fu64,
        name: "Akumajou Densetsu",
    },
    BadRomEntry {
        md5partial: 0x10f90ba5bd55c22eu64,
        name: "Alien Syndrome",
    },
    BadRomEntry {
        md5partial: 0x0d69ab3ad28ad1c2u64,
        name: "Banana",
    },
    BadRomEntry {
        md5partial: 0x85d2c348a161cdbfu64,
        name: "Bio Senshi Dan",
    },
    BadRomEntry {
        md5partial: 0x18fdb7c16aa8cb5cu64,
        name: "Bucky O'Hare",
    },
    BadRomEntry {
        md5partial: 0xe27c48302108d11bu64,
        name: "Chibi Maruko Chan",
    },
    BadRomEntry {
        md5partial: 0x9d1f505c6ba507bfu64,
        name: "Contra",
    },
    BadRomEntry {
        md5partial: 0x60936436d3ea0ab6u64,
        name: "Crisis Force",
    },
    BadRomEntry {
        md5partial: 0xcf31097ddbb03c5du64,
        name: "Crystalis (Prototype)",
    },
    BadRomEntry {
        md5partial: 0x92080a8ce94200eau64,
        name: "Digital Devil Story II",
    },
    BadRomEntry {
        md5partial: 0x6c2a2f95c2fe4b6eu64,
        name: "Dragon Ball",
    },
    BadRomEntry {
        md5partial: 0x767aaff62963c58fu64,
        name: "Dragon Ball",
    },
    BadRomEntry {
        md5partial: 0x97f133d8bc1c28dbu64,
        name: "Dragon Ball",
    },
    BadRomEntry {
        md5partial: 0x500b267abb323005u64,
        name: "Dragon Warrior 4",
    },
    BadRomEntry {
        md5partial: 0x02bdcf375704784bu64,
        name: "Erika to Satoru no Yume Bouken",
    },
    BadRomEntry {
        md5partial: 0xd4fea9d2633b9186u64,
        name: "Famista 91",
    },
    BadRomEntry {
        md5partial: 0xfdf8c812839b61f0u64,
        name: "Famista 92",
    },
    BadRomEntry {
        md5partial: 0xb5bb1d0fb47d0850u64,
        name: "Famista 93",
    },
    BadRomEntry {
        md5partial: 0x30471e773f7cdc89u64,
        name: "Famista 94",
    },
    BadRomEntry {
        md5partial: 0x76c5c44ffb4a0bd7u64,
        name: "Fantasy Zone",
    },
    BadRomEntry {
        md5partial: 0xb470bfb90e2b1049u64,
        name: "Fire Emblem Gaiden",
    },
    BadRomEntry {
        md5partial: 0x27da2b0c500dc346u64,
        name: "Fire Emblem",
    },
    BadRomEntry {
        md5partial: 0x23214fe456fba2ceu64,
        name: "Ganbare Goemon 2",
    },
    BadRomEntry {
        md5partial: 0xbf8b22524e8329d9u64,
        name: "Ganbare Goemon Gaiden",
    },
    BadRomEntry {
        md5partial: 0xa97041c3da0134e3u64,
        name: "Gegege no Kitarou 2",
    },
    BadRomEntry {
        md5partial: 0x805db49a86db5449u64,
        name: "Goonies",
    },
    BadRomEntry {
        md5partial: 0xc5abdaa65ac49b6bu64,
        name: "Gradius 2",
    },
    BadRomEntry {
        md5partial: 0x04afae4ad480c11cu64,
        name: "Gradius 2",
    },
    BadRomEntry {
        md5partial: 0x9b4bad37b5498992u64,
        name: "Gradius 2",
    },
    BadRomEntry {
        md5partial: 0xb068d4ac10ef848eu64,
        name: "Highway Star",
    },
    BadRomEntry {
        md5partial: 0xbf5175271e5019c3u64,
        name: "Kaiketsu Yanchamaru 3",
    },
    BadRomEntry {
        md5partial: 0x81c1de64550a1531u64,
        name: "Nobunaga no Yabou Zenkokuban",
    },
    BadRomEntry {
        md5partial: 0xfb4b508a236bbba3u64,
        name: "Salamander",
    },
    BadRomEntry {
        md5partial: 0x1895afc6eef26c7du64,
        name: "Super Mario Bros.",
    },
    BadRomEntry {
        md5partial: 0x3716c4bebf885344u64,
        name: "Super Mario Bros.",
    },
    BadRomEntry {
        md5partial: 0xfffda4407d80885au64,
        name: "Sweet Home",
    },
    BadRomEntry {
        md5partial: 0x103fc85d978b861bu64,
        name: "Sweet Home",
    },
    BadRomEntry {
        md5partial: 0x7979dc51da86f19fu64,
        name: "110-in-1",
    },
    BadRomEntry {
        md5partial: 0x001c0bb9c358252au64,
        name: "110-in-1",
    },
];

/// Master ROM info database (formerly CheckHInfo -> sMasterRomInfo)
#[allow(dead_code)]
pub struct MasterRomInfo<'a> {
    pub md5lower: u64,
    pub params: &'a str,
}
#[allow(dead_code)]
pub static MASTER_ROM_INFO: &[MasterRomInfo<'_>] = &[
    MasterRomInfo {
        md5lower: 0x62b51b108a01d2beu64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0x8bb48490d8d22711u64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0xc75888d7b48cd378u64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0xf81a376fa54fdd69u64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0xa37eb9163e001a46u64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0xde5ce25860233f7eu64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0x5b3aa4cdc484a088u64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0x9342bf9bae1c798au64,
        params: "bonus=0",
    },
    MasterRomInfo {
        md5lower: 0x164eea6097a1e313u64,
        params: "busc=1",
    },
];

/// Battery-backed RAM whitelist (formerly CheckHInfo -> savie)
pub static SAVIE_WHITELIST: &[u64] = &[
    0xc04361e499748382u64,
    0xb72ee2337ced5792u64,
    0x2b7103b7a27bd72fu64,
    0x498c10dc463cfe95u64,
    0x854d7947a3177f57u64,
    0xfad22d265cd70820u64,
    0x4a1f5336b86851b6u64,
    0xb0bcc02c843c1b79u64,
    0x2dcf3a98c7937c22u64,
    0x98e55e09dfcc7533u64,
    0x733026b6b72f2470u64,
    0x6917ffcaca2d8466u64,
    0x8da46db592a1fcf4u64,
    0xedba17a2c4608d20u64,
    0x91a6846d3202e3d6u64,
    0x012df596e2b31174u64,
    0xf6b359a720549ecdu64,
    0x5a30da1d9b4af35du64,
    0xd63dcc68c2b20adcu64,
    0x2ee3417ba8b69706u64,
    0xebbce5a54cf3ecc0u64,
    0x6a858da551ba239eu64,
    0x2db8f5d16c10b925u64,
    0x04a31647de80fdabu64,
    0x94b9484862a26cbau64,
    0xa40666740b7d22feu64,
    0x82000965f04a71bbu64,
    0x77b811b2760104b9u64,
    0x11b69122efe86e8cu64,
    0x9aa1dc16c05e7de5u64,
    0x1b084107d0878bd0u64,
    0xa70b495314f4d075u64,
    0x836c0ff4f3e06e45u64,
];

/// ROM correction database (formerly CheckHInfo -> CHINF moo from ines-correct.h)
pub struct RomCorrection {
    pub crc32: u32,
    pub mapper: i32,
    pub mirror: i32,
}
pub static ROM_CORRECTIONS: &[RomCorrection] = &[
    RomCorrection {
        crc32: 0xaf5d7aa2,
        mapper: -1,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xcfb224e6,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x4f2f1846,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x82f204ae,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x684afccd,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xad9c63e2,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xe1526228,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xaf5d7aa2,
        mapper: -1,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xcfb224e6,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x4f2f1846,
        mapper: -1,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xfcdaca80,
        mapper: 0,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xc05a365b,
        mapper: 0,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x32fa246f,
        mapper: 0,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xb3c30bea,
        mapper: 0,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xe492d45a,
        mapper: 0,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xe28f2596,
        mapper: 0,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xd8ee7669,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x5b837e8d,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x37ba3261,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x5b6ca654,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x61a852ea,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xf6fa4453,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x391aa1b8,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xa5e8d2cd,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x3f56a392,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x078ced30,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xfe364be5,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x57c12280,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xd09b74dc,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xe8baa782,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x970bd9c2,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xcd7a2fd7,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x63469396,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xe94d5181,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x7156cb4d,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x70f67ab7,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x291bcd7d,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xa9a4ea4c,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xcc3544b0,
        mapper: 1,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x934db14a,
        mapper: 1,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xf74dfc91,
        mapper: 1,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x9ea1dc76,
        mapper: 2,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x6d65cac6,
        mapper: 2,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xe1b260da,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x1d0f4d6b,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x266ce198,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x804f898a,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x55773880,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x6e0eb43e,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x2bb6a0f8,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x28c11d24,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x02863604,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x419461d0,
        mapper: 2,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xdbf90772,
        mapper: 3,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xd858033d,
        mapper: 3,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x9bde3267,
        mapper: 3,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xd8eff0df,
        mapper: 3,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x1d41cc8c,
        mapper: 3,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xcf322bb3,
        mapper: 3,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xb5d28ea2,
        mapper: 3,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x02cc3973,
        mapper: 3,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xbc065fc3,
        mapper: 3,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xc9ee15a7,
        mapper: 3,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x13e09d7a,
        mapper: 4,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x22d6d5bd,
        mapper: 4,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xd97c31b0,
        mapper: 4,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x404b2e8b,
        mapper: 4,
        mirror: 2,
    },
    RomCorrection {
        crc32: 0x15141401,
        mapper: 4,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x4cccd878,
        mapper: 4,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x59280bec,
        mapper: 4,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x7474ac92,
        mapper: 4,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x5337f73c,
        mapper: 4,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x9eefb4b4,
        mapper: 4,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x21a653c7,
        mapper: 4,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x9cbadc25,
        mapper: 5,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xf518dd58,
        mapper: 7,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x84382231,
        mapper: 9,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xbe939fce,
        mapper: 9,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x345d3a1a,
        mapper: 11,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x5e66eaea,
        mapper: 13,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xcd373baa,
        mapper: 14,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xbfc7a2e9,
        mapper: 16,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x6e68e31a,
        mapper: 16,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x33b899c9,
        mapper: 16,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xa262a81f,
        mapper: 16,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xe4a291ce,
        mapper: 23,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x51e9cd33,
        mapper: 23,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x105dd586,
        mapper: 27,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xbc9bb6c1,
        mapper: 27,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x43753886,
        mapper: 27,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x5b3de3d1,
        mapper: 27,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x511e73f8,
        mapper: 27,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x5555fca3,
        mapper: 32,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x283ad224,
        mapper: 32,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x243a8735,
        mapper: 32,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xbc7b1d0f,
        mapper: 33,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xc2730c30,
        mapper: 34,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x4c7c1af3,
        mapper: 34,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x932ff06e,
        mapper: 34,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xf46ef39a,
        mapper: 37,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x7ccb12a3,
        mapper: 43,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x6c71feae,
        mapper: 45,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xe2c94bc2,
        mapper: 48,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xaebd6549,
        mapper: 48,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x6cdc0cd9,
        mapper: 48,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x99c395f9,
        mapper: 48,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xa7b0536c,
        mapper: 48,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x40c0ad47,
        mapper: 48,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x1500e835,
        mapper: 48,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xb19a55dd,
        mapper: 64,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xf92be3ec,
        mapper: 64,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xe84274c5,
        mapper: 66,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xbde3ae9b,
        mapper: 66,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x9552e8df,
        mapper: 66,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x811f06d9,
        mapper: 66,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xd26efd78,
        mapper: 66,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xdd8ed0f7,
        mapper: 70,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xbba58be5,
        mapper: 70,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x370ceb65,
        mapper: 70,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xe62e3382,
        mapper: 71,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xac7b0742,
        mapper: 71,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x054bd3e9,
        mapper: 74,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x496ac8f7,
        mapper: 74,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xae854cef,
        mapper: 74,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x3d1c3137,
        mapper: 78,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xa4fbb438,
        mapper: 79,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xd4a76b07,
        mapper: 79,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x1eb4a920,
        mapper: 79,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x3e1271d5,
        mapper: 79,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xd2699893,
        mapper: 88,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xbb7c5f7a,
        mapper: 89,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x0da5e32e,
        mapper: 101,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x8eab381c,
        mapper: 113,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x6a03d3f3,
        mapper: 114,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x0d98db53,
        mapper: 114,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x4e7729ff,
        mapper: 114,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xc5e5c5b2,
        mapper: 115,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xa1dc16c0,
        mapper: 116,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xe40dfb7e,
        mapper: 116,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xc9371ebb,
        mapper: 116,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xcbf4366f,
        mapper: 118,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x78b657ac,
        mapper: 118,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x90c773c1,
        mapper: 118,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xb9b4d9e0,
        mapper: 118,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x07d92c31,
        mapper: 118,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x37b62d04,
        mapper: 118,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x318e5502,
        mapper: 121,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xddcfb058,
        mapper: 121,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x5aefbc94,
        mapper: 133,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xc2df0a00,
        mapper: 140,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xe46b1c5d,
        mapper: 140,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x3293afea,
        mapper: 140,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x6bc65d7e,
        mapper: 140,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x5caa3e61,
        mapper: 144,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x48239b42,
        mapper: 146,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xb6a727fa,
        mapper: 146,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xa62b79e1,
        mapper: 146,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xcc868d4e,
        mapper: 149,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x29582ca1,
        mapper: 150,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x40dbf7a2,
        mapper: 150,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x73fb55ac,
        mapper: 150,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xddcbda16,
        mapper: 150,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x47918d84,
        mapper: 150,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x0f141525,
        mapper: 152,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xbda8f8e4,
        mapper: 152,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xb1a94b82,
        mapper: 152,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x026c5fca,
        mapper: 152,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x3f15d20d,
        mapper: 153,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xd1691028,
        mapper: 154,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xcfd4a281,
        mapper: 155,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x2f27cdef,
        mapper: 155,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xccc03440,
        mapper: 156,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x983d8175,
        mapper: 157,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x894efdbc,
        mapper: 157,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x19e81461,
        mapper: 157,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xbe06853f,
        mapper: 157,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x0be0a328,
        mapper: 157,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x5b457641,
        mapper: 157,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xf51a7f46,
        mapper: 157,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0xe170404c,
        mapper: 159,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x276ac722,
        mapper: 159,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x0cf42e69,
        mapper: 159,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xdcb972ce,
        mapper: 159,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xb7f28915,
        mapper: 159,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x183859d2,
        mapper: 159,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x58152b42,
        mapper: 160,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x1c098942,
        mapper: 162,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x081caaff,
        mapper: 163,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x02c41438,
        mapper: 176,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x558c0dc3,
        mapper: 178,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xc68363f6,
        mapper: 180,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x0f05ff0a,
        mapper: 181,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x96ce586e,
        mapper: 189,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x555a555e,
        mapper: 191,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x2cc381f6,
        mapper: 191,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xa145fae6,
        mapper: 192,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xa9115bc1,
        mapper: 192,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x4c7bbb0e,
        mapper: 192,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x98c1cd4b,
        mapper: 192,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xee810d55,
        mapper: 192,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x442f1a29,
        mapper: 192,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x637134e8,
        mapper: 193,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xa925226c,
        mapper: 194,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x7f3dbf1b,
        mapper: 195,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0xb616885c,
        mapper: 195,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x33c5df92,
        mapper: 195,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x1bc0be6c,
        mapper: 195,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xd5224fde,
        mapper: 195,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xfdec419f,
        mapper: 196,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x700705f4,
        mapper: 198,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x9a2cf02c,
        mapper: 198,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xd8b401a7,
        mapper: 198,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x28192599,
        mapper: 198,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x19b9e732,
        mapper: 198,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xdd431ba7,
        mapper: 198,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xd871d3e6,
        mapper: 199,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xed481b7c,
        mapper: 199,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x44c20420,
        mapper: 199,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x4e1c1e3c,
        mapper: 206,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x276237b3,
        mapper: 206,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x7678f1d5,
        mapper: 207,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x07eb2c12,
        mapper: 208,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xdd8ced31,
        mapper: 209,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x063b1151,
        mapper: 209,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xdd4d9a62,
        mapper: 209,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x0c47946d,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xc247cc80,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x6ec51de5,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xadffd64f,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x429103c9,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x81b7f1a8,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x2447e03b,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0x1dc0f740,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xd323b806,
        mapper: 210,
        mirror: 1,
    },
    RomCorrection {
        crc32: 0xbd523011,
        mapper: 210,
        mirror: 0,
    },
    RomCorrection {
        crc32: 0x5daae69a,
        mapper: 211,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x1ec1dfeb,
        mapper: 217,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x046d70cc,
        mapper: 217,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x12f86a4d,
        mapper: 217,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xd09f778d,
        mapper: 217,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x62ef6c79,
        mapper: 232,
        mirror: 8,
    },
    RomCorrection {
        crc32: 0x2705eaeb,
        mapper: 234,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x6f12afc5,
        mapper: 235,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xfb2b6b10,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xb5e83c9a,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x2537b3e6,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x11611e89,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x81a37827,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xc2730c30,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x368c19a8,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0xa21e675c,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x54d98b79,
        mapper: 241,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x6bea1235,
        mapper: 245,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x345ee51a,
        mapper: 245,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x57514c6c,
        mapper: 245,
        mirror: -1,
    },
    RomCorrection {
        crc32: 0x00000000,
        mapper: -1,
        mirror: -1,
    },
];
