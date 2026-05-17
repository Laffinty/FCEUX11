# FM2 电影文件格式规范

## 概述

FM2 是 FCEUX 使用的电影录制文件格式，用于记录和重放游戏输入序列。

## 格式结构

FM2 由两部分组成：**Header（头信息）** 和 **Input Log（输入日志）**。

### Header 部分

头信息始终为 ASCII 纯文本格式，包含多个键值对。键值对的格式为：`key value`，值以换行符结束。

#### 必需字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | 整数 | 电影文件格式版本，当前为 3 |
| `emuVersion` | 字符串 | 录制电影所用的模拟器版本 |
| `romFilename` | 字符串 | 录制电影时使用的 ROM 文件名 |
| `guid` | GUID | 电影的唯一标识符，用于验证 savestate 是否属于当前电影 |
| `romChecksum` | 字符串 | ROM 的 MD5 哈希值（base64 编码的十六进制） |
| `port0` | 整数 | 端口 0 的输入设备类型 |
| `port1` | 整数 | 端口 1 的输入设备类型 |
| `port2` | 整数 | FCExp 端口设备类型 |

#### 可选字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `rerecordCount` | 整数 | 重录次数 |
| `palFlag` | 布尔 | 是否使用 PAL 时序（1=是，0=否） |
| `NewPPU` | 布尔 | 是否使用 New PPU（1=是，0=否） |
| `FDS` | 布尔 | 是否为 FDS 游戏（1=是，0=否） |
| `fourscore` | 布尔 | 是否使用四玩家适配器（1=是，0=否） |
| `binary` | 布尔 | 输入日志是否为二进制格式（1=是，0=否） |
| `length` | 整数 | 电影帧数（输入日志记录数） |
| `comment` | 字符串 | 注释信息 |
| `subtitle` | 字符串 | 字幕信息（播放时显示） |
| `savestate` | 十六进制 | 存档状态数据（如果从存档开始录制） |

#### 设备类型值

**port0/port1 支持的值：**
- `SI_NONE = 0` - 无设备
- `SI_GAMEPAD = 1` - 游戏手柄
- `SI_ZAPPER = 2` - 光线枪

**port2 支持的值：**
- `SIFC_NONE = 0` - 无扩展设备

### Input Log 部分

输入日志以 `|`（竖线）开头，可以是文本格式或二进制格式。

#### 文本格式（默认）

每帧由一行文本表示，以 `|` 开头和结尾。

**标准格式（非 fourscore）：**
```
|commands|port0|port1|port2|
```

**fourscore 格式：**
```
|commands|player1|player2|player3|player4|port2|
```

**commands 位字段：**
- `1` = Soft Reset（软重置）
- `2` = Hard Reset（硬重置/电源）
- `4` = FDS Disk Insert（FDS 磁盘插入）
- `8` = FDS Disk Select（FDS 磁盘选择）
- `16` = VS Insert Coin（VS 投币）

**SI_GAMEPAD 格式：**
使用 8 个字符表示按钮状态，顺序为 `RLDUTSBA`（Right, Left, Down, Up, Start, Select, B, A）。
非空格或 `.` 的字符表示按钮被按下。

**SI_ZAPPER 格式：**
```
XXX YYY B Q Z
```
- `XXX`: X 坐标（%03d）
- `YYY`: Y 坐标（%03d）
- `B`: 鼠标按钮状态（1=按下，0=未按下）
- `Q`: 内部值
- `Z`: 内部值（变长十进制整数）

#### 二进制格式

输入日志以 `|` 开头，每帧由固定长度的记录表示。

**commands 字节（第 1 字节）：**
- bit 0 = Soft Reset
- bit 1 = Hard Reset
- bit 2 = FDS Disk Insert
- bit 3 = FDS Disk Select
- bit 4 = VS Insert Coin

**SI_GAMEPAD：** 1 字节，位 0-7 分别对应 A, B, Select, Start, Up, Down, Left, Right

**SI_ZAPPER：** 12 字节
- 第 1 字节：X 坐标
- 第 2 字节：Y 坐标
- 第 3 字节：按钮状态
- 第 4 字节：Q 值
- 第 5-12 字节：Z 值（uint64）

## 注意事项

1. 默认情况下，所有电影从电源开启开始录制，除非存在 `savestate` 字段。
2. 新行可以是 `\r\n` 或 `\n`。
3. 如果指定了 `length` 字段，则输入日志在指定帧数后结束。

## 帧率常量

- **NTSC**: 60.099822938442230224609375 fps
- **PAL**: 50.00698089599609375 fps