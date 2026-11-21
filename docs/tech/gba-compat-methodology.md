# GBA 游戏兼容性排查方法论

> 本文沉淀 2026-11-21 MKSC（美版 AMKE）兼容性取证 + bit15 帧缓冲崩溃修复的完整
> 操作规程。每条规则都在本次实战中至少走过一遍（部分是踩坑验证），条目后附本次
> 实例锚点。配套阅读：
> - 探针触发方式索引：`docs/tech/gba_probe_howto.md`
> - 方法论裁定（**通用机制主线、不做逐卡 hack**）：`docs/history/plans/FCEUX11-GBA兼容性方法论与MKSC取证-构建计划-2026-11-21.md` §〇
> - 研究区示例（全部工件可复跑）：`Research_only/mksc_study/`（gitignored，README 有命令）
>
> 总路线一句话：**单卡故障是通用机制缺口的探测器**——四路取证 → 差距分析 →
> 无头证据闭环 → 机制层修复（锁 + 变异）→ 门禁。任何一步想把「这张卡」特殊化，
> 回去重读裁定 §〇。

## §〇 四路取证与产出物

| 路 | 手段 | 产出物 | 本次实例 |
|---|---|---|---|
| ① ROM 静态 | 普查 + 反汇编 | 游戏硬件需求画像 | FLASH_V124、DMA0 HDMA（CNT_H=0xA660）、Thumb SWI 面 |
| ② 权威信源 | 联网检索 + 本地快照 | 已知缺陷/零 override 判定 | mGBA/VBA-M 对 AMKE 零 override；GBADotnet #78 |
| ③ 实现实读 | 逐条 file:line 对表 | 差距分析表 | halt 语义/DMA 触发/flash 模型逐项 ✅ |
| ④ 动态探针 | probe 族 + harness | 运行期证据 | DMA0 武装于测量帧 1541 实测 |

四路**互相校正**：路①的 0x04000308 引用被路①自身的数据假阳性结案程序否掉；
路③的结构性 ✅ 不能替代路④的运行期实证（W2 的 DMA0 就是结构满足 + 运行期才
看到的武装）。**四路都过了才算证据闭环。**

## §一 路① ROM 静态分析：先普查，后解剖

1. **普查先行**（一个脚本，`scan1.py` 模式）：header（title/code/maker/补码校验）、
   保存类型字符串（`*_V*`）、SWI 普查、I/O 字面池普查。MKSC 一轮就定位了
   `FLASH_V124` 与 flash 命令池（`0x0E005555`/`0x0E002AAA`）。
2. **递归下降 + 种子**（`scan2.py` 模式）：从入口与 IRQ 指针（搜池词 `0x03007FFC`
   的邻字）双种子；BL/B 入工作表，线性扫到无条件跳转/返回。
3. **⚠️ SWI 编号双约定**：Thumb = 低 8 位（`0xDFnn`）；ARM = bits 16-23
   （v2.0.1 P0 的教训）。MKSC 两者共存（Thumb 主体 + ARM crt0）。**普查工具的
   svc 注释别把 ARM 约定硬套 Thumb 输出**——scan3.py 在这点上错标过一次。
4. **⚠️ Thumb 模式坑**：反汇编喂数据要喂 4 字节（BL 是 4 字节指令，喂 2 字节
   BL 全解不出来）；窗口首字解码不可靠时用「垃圾计分」自动判模式（`ldm/stm …^`
   与协处理器指令是 ARM 误读 Thumb 的标志）。
5. **字面池定寄存器画像**：池词直方图（`0x0400xxxx` 引用计数）比逐指令读快得多——
   DISPSTAT 28 次、TM3 18 次、KEYINPUT 8 次立刻给出硬件使用画像。但注意
   **变址访问不进池**（SDK 声音驱动用基址+偏移），普查是下界不是全集。
6. **目标函数定点解剖**：普查给出的 first@ 偏移处开窗（±0x60），池词 + 指令
   联合读。MKSC 的 DMA0 HDMA 装载函数（0x0804AF74）就是这样逐位读出的：
   CNT 字 `0xA6600004` → CNT_H=0xA660（timing=2 HBlank、32 位、repeat、
   源递增、目的递增/回卷）、DAD=0x04000020（BG2PA）、随后 ack IF bit1 + IE|=2。
7. **假阳性结案纪律**：普查命中的可疑寄存器（如 0x04000308）先开窗验证——两种
   反汇编模式都是垃圾解码 = 图形数据，**当场结案不追**。

## §二 路② 权威信源：只读不抄，引用到行

- 优先级：**模拟器源码**（overrides/savedata 的实读）> **issue tracker** > 硬件
  数据库（Gekkio gbhwdb，含实卡芯片型号与 ID）> 反编译项目（符号参照）> 论坛帖。
- 「某模拟器对本卡零 override」本身就是**结论**：mGBA/VBA-M 对 AMKE 零条目 +
  CHANGES 无专属修复 ⇒ 通用机制即可完美跑，不存在需要照抄的魔法。
- issue tracker 里其他模拟器的缺陷报告（GBADotnet #78「CPU halted 时 HDMA 仍须
  触发」、#64 OAM 优先级、notorious_beeg #45 帧内模式切换）是**需求清单**——
  逐条变成对照表的一行。
- 位级格式一律引 **本地 GBATEK 快照** 的行号（纪律 4；快照在
  `Research_only/w205/lt16_gbatek_full.txt`）。bit15 的修复引注就是
  `:7336-7337`「the upper bit in each halfword is unused」。
- 外部参照实现**只读不抄**（mGBA 是 MPL-2.0）。芯片 ID 这类「报假 ID 反而走已
  实现路径」的技巧（Panasonic 1B32 vs 真卡 Atmel 1F3D）以 mGBA 先例背书。

## §三 路③ 实现实读：逐条对表，标注已知盲区

把路①②的需求逐行映射到本仓坐标（file:line），形成三态表：✅ 结构满足 /
⚠️ 待运行期验证 / ❌ 缺口。MKSC 的对照表在归档计划 §四⑦。
**两条本次学到的盲区标注法**：
- **GUI-only 路径**：帧拉取（`gba_frame_buffer`）只在 GUI 每帧跑——全部探针直连
  `Gba::step`，从不经过。结构性 ✅ + 探针全绿**不能**代表这条路径的健康
  （bit15 崩溃正是活在这里）。§五 的 FFI 模式是补洞手段。
- **FFI 层**：machine-slot 互斥锁、错误恢复（machine 丢弃）、音频 advance——
  与直连 `Gba::step` 的探针语义不同，值得单独覆盖。

## §四 路④ 动态探针：证据闭环与导航

- **分工**：`probe_a_real_cartridge`（PC 分布/画面变化粗判）→ `cartprobe`
  （逐帧哈希/DMA 武装/SWI 记录器/verdict）→ `dump_frames`（⚠️ 语义坑：默认模式
  把 FRAMES 当**步数**循环且从不步进到帧边界——基线用 cartprobe，别用它）。
- **bracketing 铁律**（howto 已载）：PC 不做退出条件；「armed」要确认而不是假设。
- **输入注入导航法**：`GBA_PROBE_INPUT="frame:BUTTONS;..."`（1 基、对测量窗口、
  按住到下一条目、`GbaButton::name()` 拼写）。**迭代姿势**：每轮 25-50 帧一张
  截图（BMP → Pillow 转 PNG 肉眼看），菜单转场会吞键——被吞就加冗余按键或
  后移时点。MKSC 实测时点：标题菜单 ~m100 就绪、路线图要再按一次 A（~m2250）、
  飞掠 ~600 帧、GO ~m3320。
- **IWRAM 差分定位活表**：想监视游戏内部状态（按键表/标志位）时，**不要猜
  地址**——按键前后对 IWRAM 全量快照差分，单比特变化的地址就是活表
  （MKSC 按键表 `0x03004F30` 由此定位；ROM 里可见的 poll `0x080113F4` 是死代码，
  它写的表从不变化）。
- **无头跑不完的玩法要有止损线**：盲驾 12000 帧 REVERSE+草地未完赛 ⇒ 触发类
  目标降级为业主人工 + 工具观测，写进计划 §八。

## §五 无头复现 GUI 路径：FFI 模式（本次最大教训）

**症状**：GUI 卡死在启动动画；全部探针/测试全绿。
**成因**：探针直连 `Gba::step`，GUI 每帧走 `gba_set_buttons → gba_step_frame →
gba_frame_buffer → gba_render_audio`（machine-slot 互斥锁 + 帧拉取 + 音频推进）
——这条路径此前**零覆盖**。bit15 像素在帧拉取的 LUT 转换处越界 panic，
release `panic="abort"` 直接杀进程。
**手段**：harness 的 **FFI 模式**（`MKSC_FFI=1`）——逐帧复刻 GUI 生产序列，
逐帧打印 step 返回码/耗时/帧哈希并检测冻结。MKSC 帧 218 处逐字复现
`index out of bounds: len 32768, index 61987`，修复后 2500 帧干净。
**规程**：任何「GUI 冻结/异常但探针全绿」的报告，**先跑 FFI 模式再下结论**；
新增 GUI-only 路径时同步扩 FFI 模式的覆盖。
**编码配方**（五个坑，配方固化在研究区 `harness_compile.bat`）：root crate 是
`crate-type=["staticlib"]` 不能作 Rust 依赖 ⇒ 隔离 `CARGO_TARGET_DIR` +
`CARGO_PROFILE_RELEASE_LTO=off` + `cargo rustc -- --crate-type rlib`（产物带
hash，glob 定位）+ feature `fceux11-lua/ffi-stubs`（lua 桥接桩）+ `rustc --extern`
带 `-C target-feature=+crt-static -C panic=abort`（匹配 repo rustflags）+
**两条 `-L dependency`**（triple 目录 + 主机态目录——proc-macro dylib 在主机态，
漏了就是 E0463）。

## §六 根因修复纪律

1. **修机制，不修卡**：bit15 的修复是索引前 `& 0x7FFF`（硬件在颜色数据通路忽略
   该位），不是「MKSC 专判」。逐卡 hack 的禁止清单与四门槛见裁定 §〇。
2. **位级语义引用 GBATEK 行号**，不凭记忆（纪律 4）。
3. **锁测试双断言**：与实现同源的数据结构断言（LUT 条目）+ **独立参照断言**
   （`bgr555_to_rgba_scalar`——标量位运算，与被测表不同源）。只查表会复制表的
   错误（纪律 12 同族）。
4. **变异验证**：摘 mask → 测试以与生产症状**逐字相同**的越界数字转红；还原 →
   绿。⚠️ 变异还原用 `git checkout` 会把**同文件未提交的新测试一起吞掉**——
   本次第三次踩雷，恢复靠上下文重放。铁律：**先提交再做变异腿**。
5. **门禁**：Rust 全套（root/gba-core/debug/media/formats/utils/lua + 双态
   check）+ 重链后 `ctest -LE perf` 49 项。改生产 Rust 后的重链是 LTCG 长跑
   （全量 ~1 小时），后台跑、勿中途取消（坑 12/13）。提交信息携带门禁读数。

## §七 与既有纪律的衔接

- 纪律 1（先调查后动文件）：本规程的 §一–§五 全部只读；修复在证据链闭合后。
- 纪律 3（变异验证）：本次实例 = bit15 锁 + W4 OAM 优先级锁（两条镜像腿）。
- 纪律 4（不凭记忆写位级格式）：SWI 双约定、CNT_H 逐位、bit15 三处全部引
  GBATEK 快照或本仓已验证常量。
- 纪律 10（编不过的测试等于不存在）的**新形态**：测试存在且会跑，但**覆盖的
  路径 ≠ 生产走的路径**——GUI-only 路径的覆盖缺口靠 §五 的 FFI 模式补。
- 业主协作：静态排查穷尽后，把一次可判别的运行步骤（带诊断环境变量的启动
  命令 + 要观察的现象清单）交给业主，比继续盲试便宜。

## §八 本次全程对照（速查）

| 阶段 | 输入 | 动作 | 产出/判据 |
|---|---|---|---|
| 立项 | 业主 ROM | scan1 普查 + 信源 agent + 实现审计 agent | §四 事实底座（冻结） |
| 差距 | 事实底座 | 对照表三态 | DMA0/flash/SWI ✅，DMA 交错等 ⚠️ |
| W2 | cartprobe + 注入 | 菜单导航迭代 | DMA0 armed timing=2（帧 1541） |
| W3 | harness 电池监视 | 触发探索 | flash 零写入证实；触发降级业主 |
| W4 | GBADotnet #64 | 夹具 + 变异 | 双腿出生即绿（本仓无此缺陷） |
| 用户报告 | GUI 冻结 | exe 核对 + 无输入长跑 + FFI 模式 | 帧 218 逐字复现 panic |
| 修复 | GBATEK :7336-7337 | mask + 锁 + 变异 | FFI 2500 帧干净；ctest 49/49 |
