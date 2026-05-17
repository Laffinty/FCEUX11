# FCM 到 FM2 格式转换指南

## 概述

FCEUX 使用 `.fm2` 格式，不直接支持旧版 FCE Ultra 的 `.fcm` 格式。需要将 `.fcm` 文件转换为 `.fm2` 格式才能在 FCEUX 中播放。

## 转换方法

### GUI 方式
1. 打开 FCEUX
2. 选择菜单 `Tools` > `Convert .fcm to .fm2`
3. 在打开对话框中选择要转换的 `.fcm` 文件（支持多选）

### 命令行方式（SDL 版本）
```bash
./fceux --fcmconvert filename.fcm
```

## FM2 格式改进

与旧版 FCM 格式相比，FM2 具有以下改进：

1. **文本格式**：默认采用文本格式，便于手动编辑和拼接
2. **GUID 标识**：内置 GUID，确保 savestate 与电影文件的对应关系
3. **电源开启录制**：从电源开启录制的电影不再包含冗余的 savestate
4. **鼠标输入支持**：支持记录 Zapper 和 Arkanoid Paddle 的鼠标输入

## 转换问题与解决方案

### 常见不同步问题

**原因分析**：
- 旧版 FCE Ultra 录制电影时，WRAM（工作 RAM）或存档 RAM 已处于初始化状态
- 电影录制者可能在标题画面等待一段时间后才开始录制
- FCEUX 电影从全新电源开启开始，WRAM 和主 RAM 都未初始化

**解决方案**：
由于 FM2 是文本格式，可以在电影开头插入额外的空帧来模拟旧版行为。

### 已知转换失败情况

| 问题类型 | 说明 |
|----------|------|
| FCEU 0.10 或更早版本的电影 | 如 Dragon's Lair、Powerblade |
| Dragon Warrior 4 电影 | VRAM 处理方式特殊，只能在创建它的版本中同步 |
| 从 savestate 开始的电影 | FCEU 和 FCEUX 的 savestate 不兼容 |
| Battle Toads (FCEU 0.12) | Mapper 7 在 FCEU 0.13 后有重大改动 |

### 通过添加帧修复的不同步

| 电影 | 需要添加的帧数 |
|------|---------------|
| Legend of Zelda "1st Quest" | 3 帧 |
| Vice: Project Doom | 10 帧 |

### 无法修复的不同步

| 电影 | 状态 |
|------|------|
| Addams Family | 尝试添加 16 帧内均无效 |
| Kattou Ninden Teyandee | 尝试添加 16 帧内均无效 |
| Dragon Warrior | 添加 2 帧后可同步到约 46540 帧 |

## 转换技巧

### 模拟旧版行为

在电影开头添加空帧，然后在第一帧添加重置标志：

```
|0|||
|0|||
|0|||
|2|||
```

这样可以模拟旧版 FCE Ultra 的录制方式，给游戏时间初始化 WRAM。

### RNG 因素

游戏初始化期间，RNG（随机数生成器）通常未运行，因此转换后同步的可能性较高。