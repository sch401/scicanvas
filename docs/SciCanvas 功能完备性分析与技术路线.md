# SciCanvas 功能完备性分析与技术路线（第三阶段）

> 原始设计分析基准（历史快照；当前统计见下文 2026-10-18 核对）：仓库 `scicanvas` @ `ad11c63`（14 个提交，Rust 13,446 行）；对照清单 `docs/parity/parity.json`（488 条，与 `SciCanvas 对照清单.html` 一致）；Origin 自带帮助 `Origin.chm`（目录 1199 项）、`X-Function.chm`（615 个 X-Function）、内置拟合函数库（239 个显函数 + 5 隐函数 + 22 曲面 + 7 峰函数）。
> 读者：负责写代码的 GPT/Codex。本文的第 7 节是可以直接分派的任务卡。


## 类型化列与 v9（2026-10-18）

T1.1 已实现待验收：数值、文本、UTC 日期时间、分类字典及缺失位图已接入。v9 使用小端数值字节/Base64、稀疏原文和严格校验，直接打开 v0–v8 项目。14 份旧 v8 示例保留，CSV/SVG 预览与新版逐字节一致，18 份 SVG 基准及旧版布局测试继续通过。

界面列头右键和 Ctrl+K 可转换类型，转换失败保持原数据。单格类型冲突先确认转文本，取消保持原样；批量粘贴冲突保持整块原数据，提示先转换列类型。转换/确认写入复用撤销事务，公式与序列填充保持目标列类型。

工作表组织遵照用户决定：每个 Book 就是一张独立表，多表分别打开为窗口；Excel 多 Sheet 继续拆分导入独立 Book。F049 改为不做。T1.2 只完善列属性/格式，不引入 Sheet 层或底部标签。

本地完整 Rust 测试现为 **244 项**（核心单元 93、集成 86、界面 65），另有 10 项清单/许可回归。历史 231 项继续通过，新增 10 项类型/迁移与 3 项界面回归。格式、Clippy、无默认特性及依赖/版权材料检查通过。Windows CI 以本次提交的实际运行记录为准。

百万行十列同机基准：导入 **1461.616 ms**、单曲线排版 **184.746 ms**、整表排序 **54.457 ms**。原基线为 2367/175/685 ms。容量测试验证单列百万规范数值 <12 MiB，完整首帧和峰值内存另行验收，详见 `docs/perf.md`。稀疏原文在整列特殊写法时仍保留逐格文本，后续统一格式可优化。

当前日期/分类功能属于基础列能力。完整显示格式、本地时区/日历函数、分类轴与类别顺序编辑仍在后续任务。项目数据版本为 **v9**，v10–v12 仍为后续设计版本。

## 最新代码核对（2026-10-18）

功能基线来自 `d928ad3`，并按本次 v9 类型化列实现更新。按桌面命令、核心实现、结果写回与现有测试核对。完整 Rust 测试 **244 项通过**（核心单元 93、集成 86、界面 65）；3 项显式更新基准/示例的任务默认忽略。沿用这次已通过的运行结果；本轮只修改文档和清单。

清单共 **514 项**：已完成26、已实现待验收51、部分实现44、未开始381、不做12。其中 121 项有实现进展；该数量包含部分实现，按条目计数，不代表功能完整度。P0 共 97 项：已完成 26、待验收 27、部分实现 29、未开始 15。15 条目记录子项计数，已完成与待验收分别统计。

| 模块 | 已接入桌面工作流，仍待专项验收 | 明确剩余范围 |
| --- | --- | --- |
| 数据与文件 | 插入/删除行；CSV 导出；多文件拖入；xlsx/xlsm 各非空工作表分别导入；`.scv` 包含项目 JSON、CSV 与 SVG | 导入向导、旧 xls、Excel 日期/样式、类型化列、数字格式 |
| 列计算 | 一次性公式设置列值、列与常数/列间运算、曲线相减、等差/均匀/正态序列 | 公式持久化、依赖与自动重算、整列聚合、120 函数完整目录 |
| 拟合 | 线性/2–6 阶多项式、多元线性回归；11 种 LM 非线性模型、自动初值、固定参数；线性标准曲线反查浓度 | 权重、参数上下界/线性约束、自定义函数、全局/多峰拟合、完整报告与置信/预测带、NIST 验收 |
| 统计 | 列/行统计、频数统计、三类 t 检验、Pearson/Spearman/Kendall、单因素 ANOVA 与 Tukey、Shapiro–Wilk、Grubbs | 双/三因素及重复测量 ANOVA、KS/AD、Dixon Q、非参数检验、统计图 |
| 曲线与信号 | 线性/自然样条/Akima 插值、曲线平均及标准差；FFT/窗/谱、四类理想频域滤波、自/互相关；连线/ALS 基线、突出度寻峰与积分 | 独立 IFFT、SG 直接求导、LOWESS、导数寻峰、多峰拟合、STFT/IIR/小波 |
| 绘图与导出 | 14 种预设、误差棒/标签；上下/左右/四面板；Nature 默认样式、图例自动避让；PNG/DPI、300 dpi 位图剪贴板 | 同层独立 Plot/多轴与轴联动、完整面板/插图工作流、富文本/真实统一测宽、PDF/TIFF/EMF、模板与批量导出 |
| 界面与工程 | 项目树拖放和慢速双击/F2 重命名、焦点分流、窗口吸附/候选补位、可移动对话框、中文字体与表头居中；依赖审计/Windows CI 已配置 | 项目搜索、自动恢复、最近文件、窗口状态持久化、安装包与大数据验收 |

**状态口径**：已实现待验收＝当前条目范围已接入，剩参考软件、专项数值、人工或性能验收；部分实现＝条目范围有明确缺口；未开始＝缺少对应可用工作流。保留已有已完成项的验收口径，F082 因独立曲线结构尚未完成改为部分实现。函数库以当前 11 模型和 239 项参考目录分开记录，公式为 24 个调用名/23 个独立函数，不将别名重复计数。

**底层进度**：`Column.data` 已为四种类型化存储，`Layer` 仍持有单个 `PlotSpec`，项目数据版本为 v9。公式已能计算但没有保存为依赖；PNG 已实现但 A6 真实字体测宽、富文本、PDF/TIFF 尚待推进。A1-a 类型化列/v9 已接入，A1-b 列属性/格式及 A2/A3/A5/A6 的剩余任务继续推进，v10–v12 为规划版本。

**下一步**：先验收 v9 类型化列并完善独立表的列属性/格式；按依赖推进 A3/A2 和 A6；现有分析功能补专项数值/Origin 验收。各项剩余范围以清单和下文任务卡为准。

## 1. 当前结论

科研分析、文件交互和导出已扩展，当前进度按下面的实际清单统计。A1/A2/A3/A6 的结构性任务继续推进；新增公式和 PNG 分别完成 A4/A6 的一部分。已有测试的参考值对照与完整 NIST/Origin 验收分别记录。

## 2. 分模块现状

| 模块 | 条目 | 已完成 | 待验收 | 部分实现 | 未开始 | 不做 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 工作区与项目 | 24 | 7 | 2 | 1 | 12 | 2 |
| 工作表 | 33 | 4 | 4 | 8 | 16 | 1 |
| 矩阵 | 8 | 0 | 0 | 0 | 8 | 0 |
| 导入导出 | 18 | 3 | 3 | 1 | 10 | 1 |
| 绘图基础 | 36 | 7 | 4 | 7 | 18 | 0 |
| 图形探索与小工具 | 17 | 0 | 0 | 2 | 15 | 0 |
| 回归与拟合 | 29 | 0 | 4 | 12 | 13 | 0 |
| 数学运算 | 17 | 0 | 5 | 4 | 8 | 0 |
| 统计 | 31 | 0 | 7 | 3 | 21 | 0 |
| 信号处理 | 15 | 0 | 2 | 2 | 11 | 0 |
| 峰分析 | 5 | 0 | 2 | 2 | 1 | 0 |
| 图像处理 | 5 | 0 | 0 | 0 | 5 | 0 |
| 导出与发布 | 10 | 1 | 2 | 0 | 7 | 0 |
| 自动化与编程 | 18 | 0 | 0 | 0 | 10 | 8 |
| 图类型 · 2D 线图 | 7 | 1 | 3 | 0 | 3 | 0 |
| 图类型 · 2D 散点图 | 13 | 1 | 3 | 0 | 9 | 0 |
| 图类型 · 2D 点线图 | 7 | 1 | 1 | 0 | 5 | 0 |
| 图类型 · 2D 柱状图与条形图 | 14 | 1 | 5 | 0 | 8 | 0 |
| 图类型 · 饼图 | 10 | 0 | 0 | 0 | 10 | 0 |
| 图类型 · 多轴图 | 16 | 0 | 0 | 0 | 16 | 0 |
| 图类型 · 多面板图 | 10 | 0 | 3 | 0 | 7 | 0 |
| 图类型 · 瀑布图 | 6 | 0 | 0 | 0 | 6 | 0 |
| 图类型 · 统计图 | 42 | 0 | 0 | 0 | 42 | 0 |
| 图类型 · 分组图 | 14 | 0 | 0 | 0 | 14 | 0 |
| 图类型 · 地图 | 6 | 0 | 0 | 0 | 6 | 0 |
| 图类型 · 面积图 | 6 | 0 | 1 | 0 | 5 | 0 |
| 图类型 · 专业图 | 24 | 0 | 0 | 0 | 24 | 0 |
| 图类型 · 金融图 | 6 | 0 | 0 | 0 | 6 | 0 |
| 图类型 · 3D XYY 图 | 9 | 0 | 0 | 0 | 9 | 0 |
| 图类型 · 3D 曲面图 | 15 | 0 | 0 | 0 | 15 | 0 |
| 图类型 · 3D 散点 / 柱 / 矢量图 | 13 | 0 | 0 | 0 | 13 | 0 |
| 图类型 · 等高线与热图 | 13 | 0 | 0 | 0 | 13 | 0 |
| 图类型 · 图像与剖面 | 4 | 0 | 0 | 0 | 4 | 0 |
| 分析工作流 | 3 | 0 | 0 | 1 | 2 | 0 |
| 作图核心 | 10 | 0 | 0 | 1 | 9 | 0 |
| **合计** | 514 | 26 | 51 | 44 | 381 | 12 |

### 2.1 P0 未开始（15 条）

- **F032 数值显示格式（小数位、科学计数、工程单位）**：列格式属性
- **F065 导入向导（跳过表头行、指定分隔符、预览）**：预览对话框
- **F083 往图层添加 / 移除曲线**：图层内容对话框
- **F089 图形模板（保存样式，换数据一键出图）**：图形样式与数据引用分离
- **F102 文字标签（上下标、希腊字母、富文本）**：迷你标记语言 + cosmic-text 排版
- **F142 用户自定义拟合函数**：复用公式解析器，自动求导用数值差分或自动微分
- **F152 内置拟合函数：多项式与有理函数（Poly、Rational 等）**：函数表达式 + 初值估计，数据驱动注册
- **F228 导出 PDF（矢量，嵌入字体）**：svg2pdf / krilla
- **F305 双 Y 轴图**：2D 渲染器 + SVG 导出
- **F492 参考线（水平 / 垂直）**：A2/A7 按轴值定位，属性与导出共用场景
- **F495 同一图层每条曲线独立图类型（柱 / 线 / 散点混合）**：A2 Layer 包含多个 Plot，每条曲线单独指定类型
- **F497 符号边框与内部填充独立设置**：A2/A3 每条曲线边框颜色、宽度和填充
- **F499 按列映射曲线或符号颜色**：A2 按数据列解析逐点颜色，兼容图例
- **F500 按列映射符号大小**：A2 按数据列映射逐点尺寸与范围
- **F501 按列映射符号形状**：A2 按分类列映射符号形状

### 2.2 P0 部分实现（29 条）

- **F026 列属性：X / Y / Z / X误差 / Y误差 / 标签 / 忽略**：X/Y/X误差/Y误差/忽略及误差自动关联已实现；Z 与专用标签列角色待做
- **F027 插入 / 删除 / 移动列**：列插入、删除、清空与稳定列标识已实现；移动列待做
- **F035 设置列值（公式列）**：Ctrl+Q 设置列值：表达式、列引用/col(A)、行号 i、pi/e、24 个函数调用名；按当前数据一次性覆盖结果并支持撤销。列公式持久化、依赖图和自动重算待做
- **F038 排序（列、工作表、按多列）**：按一列稳定排序整张工作表，同行联动，数值/文本/空值规则已实现；独立列排序与多关键字排序待做
- **F053 大数据量（百万行流畅滚动）**：类型化数值列、缺失位图和可见行表格已接入；本机百万行十列导入 1.462 秒、排版 185 ms、整表排序 54 ms；完整首帧/进程峰值内存和绘图抽点待验收
- **F066 导入 Excel（.xlsx / .xls）**：导入 .xlsx / .xlsm：自写读取（zip + roxmltree），共享 / 内联 / 富文本字符串、公式缓存值、布尔与错误值，每张非空工作表一个工作簿；.xls、日期格式与合并单元格待做
- **F082 页面-图层-曲线 三层结构**：已有 Graph→Page→Layer→PlotSpec 与多 Y 曲线映射；每层持有一个 PlotSpec。持久化独立 Plot 对象、各曲线样式与 AxisSet 仍属 A2
- **F096 刻度标签格式（科学计数、日期、自定义）**：ticks.rs 已实现普通数值、a×10ⁿ 科学计数及对数刻度；日期与用户自定义格式待做
- **F097 次刻度、刻度方向、四边框线**：屏幕/SVG 共用主次刻度；默认 Nature 风格，只画左轴和下轴、刻度朝外；四边框、朝内刻度和独立方向设置待做
- **F109 图层排列（行列网格、间距）**：已有上下/左右/四面板快捷创建、网格列数、自由矩形与透明叠放；独立网格间距/对齐与轴联动待做
- **F113 绘图属性对话框（Plot Details）**：图层级图型、列映射、轴范围、线宽/符号、误差/标签等属性已接入；逐曲线样式和完整分页待做
- **F115 页面尺寸、背景、边距**：页面毫米尺寸、固定逻辑画布、图层透明背景已实现；页面配色与可调边距待做
- **F116 读取坐标 / 数据点提示**：自绘 plot.rs 已实现悬停坐标、十字光标与图层定位；最近数据点吸附及点信息提示待做
- **F130 线性回归（斜率、截距、标准误、R²、残差）**：已实现斜率/截距、标准误、R²、残差平方和及报告；逐点残差输出/残差图待做
- **F134 非线性拟合引擎（Levenberg–Marquardt）**：Levenberg–Marquardt（中心差分雅可比、自适应阻尼），标准误 (JᵀJ)⁻¹·RSS/(n−p)；目前只用内置函数，参数约束与自定义函数待做
- **F136 参数固定、上下界、线性约束**：非线性拟合对话框可把任意参数固定为指定值，固定参数不计入自由度、标准误标为“固定”；上下界与线性约束待做
- **F138 拟合报告（参数表、ANOVA、置信带、预测带）**：线性/多项式、非线性和多元回归已有临时报告，含参数/标准误、R²/RSS 等；多元回归还有 t/p 与整体 F 检验。完整拟合 ANOVA、置信/预测带、持久化报告待做
- **F148 内置拟合函数：峰函数（Gauss、Lorentz、Voigt、PsdVoigt、LogNormal 等）**：峰函数：Gauss、Lorentz（Origin 公式与参数名）；Voigt、PsdVoigt、LogNormal 等待做
- **F149 内置拟合函数：指数类（ExpDec1-3、ExpGrow、ExpAssoc 等）**：指数类：ExpDec1；ExpDec2/3、ExpGrow、ExpAssoc 等待做
- **F150 内置拟合函数：S 形与生长（Boltzmann、Logistic、DoseResp、Hill、Gompertz 等）**：S 形：Boltzmann、Logistic；DoseResp、Hill、Gompertz 等待做
- **F151 内置拟合函数：幂函数与对数（Power、Allometric、Log 等）**：幂函数：Allometric1；Power、Log 等待做
- **F166 数值微分（含 Savitzky-Golay 平滑求导）**：有限差分微分和独立 Savitzky–Golay 平滑已实现；直接 SG 平滑求导入口待做
- **F201 平滑（移动平均、Savitzky-Golay、FFT 平滑、LOWESS）**：移动平均与二阶 Savitzky–Golay 已接入；FFT 平滑与 LOWESS 待做
- **F204 傅里叶变换 FFT / IFFT（幅值、相位、功率谱、窗函数）**：任意点数 FFT（基 2/Bluestein）、矩形/Hann/Hamming 窗、单边幅值/相位/功率谱及幅值图已接入；滤波内部已有逆变换，独立 IFFT 命令与复数输入输出待做
- **F489 分析报告表与重算（自动 / 手动 / 不重算）**：已有独立分析结果工作簿与临时报表；统一持久化分析报告表、Operation 依赖记录及 Auto/Manual/None 三种重算模式待做
- **F498 曲线连接方式（直线 / 样条 / B 样条 / 阶梯 / Bezier）**：已有直线、自然样条、水平/垂直阶梯；B 样条与 Bezier 待实现
- **F509 列公式函数库**：表达式引擎已有 24 个函数调用名，对应 23 个独立函数（log/log10 同义），首批 120 函数目标、聚合函数及函数目录仍待补齐
- **F510 日期时间列**：DateTime 基础列已接入：UTC 毫秒存储、ISO8601/常见年月日写法/四位年份解析、位图/原文保留、转换/排序/v9 保存；本地时区配置、显示格式与日期轴待做
- **F511 分类列及类别排序**：Category 采用 u32 编码和类别字典，支持缺失、编辑、转换、保存与标签排序；ordered 字典顺序已有核心支持，类别顺序编辑界面和分类轴待做

### 2.3 P0 已实现待验收（27 条）

- **F028 插入 / 删除行**：工作表“行”分组与单元格右键菜单：在选区上方插入同数量空行、删除选区覆盖的整行；图形行范围随之平移，整段删除时改回全部行；一步撤销
- **F030 鼠标拖动框选单元格、Shift 扩展、复制选区**：Rust app.rs/sheet.rs 已接入拖动框选、Shift 扩展、复制/剪切/粘贴与清空选区
- **F068 导出 CSV**：工作表 → 导出 CSV：UTF-8 BOM、CRLF，表头为“长名称 (单位)”，与 .scv 内的 CSV 同一实现
- **F069 拖放文件导入**：把文件拖进窗口：数据文件逐个导入为工作簿并合为一步撤销，项目文件（.scv / .scproj）直接打开；拖动时显示提示层
- **F081 按选区（部分行）绘图**：工作表矩形选区经 picked/PlotSpec.rows 接入作图、图库与保存，按原始行截取
- **F084 多图层图形**：多图层增删、跨工作簿数据、独立属性/裁剪/视图、分析写回目标层已实现
- **F106 误差棒**：对称 X/Y 误差列、稳定自动关联、端帽/线宽、分组定位与屏幕/SVG 已实现；本条当前范围为对称误差
- **F131 多项式回归**：X 标准化后的正规方程与高斯–约当求逆；2–6 阶拟合、参数标准误、R²、曲线及撤销已接入
- **F135 参数初值自动估计**：当前 11 种内置非线性模型均有数据驱动初值估计；支持固定参数并报告未收敛状态
- **F147 快速拟合（菜单一键）**：线性拟合一键命令、多项式阶数对话框、报告和结果写回已接入
- **F158 插值 / 外推（线性、三次样条、Akima）**：分析 → 插值：线性、自然三次样条、Akima，等间距 X 网格、可自定义范围并按端点段外推；结果写成新 X/Y 列并加到图上，一步撤销
- **F164 数值积分（梯形法、曲线下面积）**：analysis.rs/ops.rs 已实现梯形积分、累计积分结果与绝对面积选项
- **F168 简单列运算（加减乘除、两曲线相减）**：分析 → 列运算：两列或列与常数的加减乘除，结果新列与单位推导；不同 X 网格的曲线先线性插值后相减，范围外留空；一步撤销
- **F169 归一化（最大值、Z 分数、区间缩放）**：已接入除以最大值、Z 分数、区间缩放，结果写入派生列并支持撤销
- **F174 列统计（均值、标准差、中位数、分位数等）**：已接入 N、均值、样本标准差、标准误、中位数、线性插值分位数、极值与总和，生成结果工作簿
- **F177 频数统计 / 分箱**：统计 → 频数统计：等宽分箱（Sturges 或指定区间数、可选范围），每列生成区间、频数、累积与相对频率表，一步撤销
- **F185 单样本 t 检验**：统计 → t 检验（单样本）：自写正则化不完全 Beta 求 t 分布，报告 t、自由度、双侧 p、置信区间
- **F186 双样本 t 检验（含 Welch）**：统计 → t 检验（双样本）：方差相等的 Student 检验与 Welch 检验，置信区间针对均值差
- **F187 配对 t 检验**：统计 → t 检验（配对）：按行配对，只用两列都有数值的行
- **F227 导出 PNG（指定 DPI）**：图形 → 导出 PNG：SVG 版面经 resvg 栅格化，可选 150/300/600/1200 dpi 或自定义；写入 pHYs 物理分辨率，像素尺寸按页面毫米换算；中文回退到系统字体
- **F231 复制图形到剪贴板（粘贴到 Word / PPT）**：复制图形（Ctrl+J）：按 300 dpi 栅格化后经 arboard 写入剪贴板位图；矢量 EMF 待评估
- **F264 Y 误差棒图**：Y 对称误差列自动关联、端帽/线宽、裁剪与共享线段图元
- **F265 XY 误差棒图**：对称 X/Y 误差列自动关联、方向转换、端帽/线宽与共享图元
- **F283 条形图**：数值型条形图、并排分组、误差换向、物理轴范围与共享矩形
- **F321 上下两面板图**：绘图 → 多面板 → 上下两面板：Y 列按顺序分到两个图层，单列网格，页面 89 × 120 mm
- **F322 左右两面板图**：绘图 → 多面板 → 左右两面板：Y 列按顺序分到两个图层，两列网格，页面 183 × 70 mm
- **F323 四面板图**：绘图 → 多面板 → 四面板：Y 列按顺序分到 2 × 2 网格的四个图层，页面 183 × 140 mm

## 3. 对照清单相对 CHM 的遗漏

以下第三阶段新增需求已登记为 F489–F514，保留为范围说明。当前已实现部分会在清单中标注；未接入的项目继续按路线推进：

| 新增项 | CHM 出处 | 建议优先级 |
| --- | --- | --- |
| 分析报告表 + 重算机制（Auto / Manual / None，源数据变化后自动重算） | Origin.chm › Analysis › Recalculating Analysis Results | **P0**（结构性） |
| 分析主题 / 对话框设置保存（Dialog Theme） | Analysis Themes | P1 |
| 批处理（Batch Processing，用分析模板跑多个数据集） | Batch Processing | P1 |
| 参考线、轴断点、轴上的对象（Reference Lines, Axis Break） | Graphing › Axes | P0（参考线）/ P1（轴断点） |
| 每条曲线独立的图类型（同一图层里柱 + 线 + 散点） | Plot Details › Plot Type | **P0**（结构性） |
| 曲线细节：填充图案、符号边/内填充、连线方式（直线/样条/B 样条/阶梯/Bezier）、按列映射颜色/大小/形状（Modifiers） | Plot Details | P0 / P1 |
| Graph Data Slicer、带浏览面板的图（Browser Graph） | Graph Exploration | P2 |
| Data Extraction / Digitizer 小工具 | Gadgets / Digitizer | P2 |
| 图形摘要（Graphical Summary）、并排统计、双样本 KS、Running 非参数检验 | Statistics | P1 |
| 列公式函数库（Origin 内置函数与 Excel 函数对照表，约 300 个） | Set Column Values › Functions | **P0**（F035 的依赖） |
| 日期时间列、分类列（Categorical）及其排序 | Worksheet › Column Properties | **P0**（结构性） |
| 附加文件、OPX 打包分发 | Project › Attach File | P2 |

**粒度与计数口径**：拟合函数 239 个参考函数原先只归成 10 条，导入格式约 40 种只归成 3 条，X-Function 615 个大部分没有单独对应。验收时不能用"条目已完成"代表"能力已完成"。现已在 parity.json 加入 `sub_items` 计数字段；HTML 显示已完成、待验收与未开始。当前 11 个模型单独计数，239 项参考目录的逐项映射仍待核对。

---

## 4. 已经做对的部分（保留，不要改）

- core 不依赖 UI；命令注册表 + 定义处 `.modifies()`。
- `scene::layout_page` 屏幕和导出共用，固定页面画布，屏幕只缩放。保持 WYSIWYG。
- `History::begin / changed / commit` 事务。
- 项目文件 v8 + v0–v7 迁移链。以后每次模型改动都必须加一次迁移和测试（第 5 节的重构会让版本走到 v9–v12）。
- 显式 `first_layer()` / `first_layer_mut()`，不再 Deref。

---

## 5. 结构性缺口（必须先做）

每项给出：为什么、设计、接口、依赖 crate、验收标准。A1、A2、A3、A6 是**前置重构**，必须在铺图类型之前完成。

### A1 列式类型化数据模型 + 独立工作表 + 矩阵

**原问题（现已完成类型化列改造）**：旧 `Column.data: Vec<String>` 每次画图都要 parse；日期、分类、缺失值都没法表达；公式列、统计分组、3D 都依赖类型。本项目按用户决定保留一个 Book 一张独立表，矩阵是后续独立对象；不沿用 Origin 的多 Sheet 容器。

**设计**：

```rust
// scicanvas-core/src/data/mod.rs
pub enum ColumnData {
    Num { values: Vec<f64>, missing: BitVec },   // NaN 不表示缺失，缺失用位图
    Text(Vec<String>),
    DateTime { values: Vec<i64>, missing: BitVec }, // 毫秒 UTC，显示时按格式转换
    Category { codes: Vec<u32>, levels: Vec<String>, ordered: bool },
}
pub struct Column {
    pub key: String, pub long_name: String, pub units: String, pub comments: String,
    pub role: Role,          // 新增 Z、Label
    pub format: DisplayFormat, // 小数位、科学计数、日期格式、百分比
    pub formula: Option<String>, // F035，见 A4
    pub data: ColumnData,
}
pub struct Book { pub id: BookId, pub name: String, pub columns: Vec<Column> } // 一份 Book 就是一张表
pub struct Matrix { pub id: MatrixId, pub name: String, pub objects: Vec<MatrixObject> }
pub struct MatrixObject { pub nx: usize, pub ny: usize, pub x: (f64,f64), pub y: (f64,f64), pub z: Vec<f64> }
```

- 新增 `trait ColumnView { fn len(); fn f64_at(i) -> Option<f64>; fn text_at(i) -> Cow<str>; }`，画图和分析只通过它取数据。
- 单元格编辑：输入字符串时按列类型 parse；Text 列允许任意内容；Num 列 parse 失败时提示"转为文本列？"。
- 列类型推断：导入消费原字符串并完整核对，后段出现非数值时保留为文本，避免仅采样前 1000 行造成丢值。
- `PlotSpec` 的列引用从 `book_id + x_key` 改为 `DataRef { book: BookId, col: ColKey }`（后续需要统一引用时引入；不增加 Sheet 层）。
- 文件格式 v9 已实现：旧字符串数组推断为数值/文本，保留原始写法；Num/DateTime 用 Base64 小端字节、缺失位图与可选稀疏原文。日期/分类由用户明确转换，布局不改。

**依赖**：`bitvec`（或自写 `Vec<u64>` 位图）、`chrono`（只用于格式化/解析）。暂不引入 arrow，等 1.0 后需要 Parquet 再说。

**验收**：
- 1,000,000 行 × 10 列 Num 数据：导入 < 2 s，散点图首次渲染 < 500 ms，内存 < 150 MB（Windows release 构建）。
- v8 → v9 迁移测试，旧项目打开后图完全一致（对比 SVG 输出）。
- 工作表各自使用独立 Book/窗口，可新建、删除和重命名；不增加底部 Sheet 标签页。
- 保持当前 231 项完整 Rust 测试通过；新增结构与迁移按需补回归。

### A2 Layer → 多个 Plot + 轴对象

**为什么**：Origin 每个图层里可以有多条曲线，各自不同类型和样式（F083、双 Y 柱+线、分组图都依赖它）。现在一个 Layer 只有一个 `spec`，这是图类型扩展的最大瓶颈。

**设计**：

```rust
pub struct Layer {
    pub placement: Option<LayerRect>,
    pub transparent: bool,
    pub axes: AxisSet,                 // 多根轴
    pub plots: Vec<Plot>,              // 替代 spec
    pub groups: Vec<PlotGroup>,        // 颜色/符号递增的分组
    pub objects: Vec<GraphObjectId>,   // 文本、箭头、参考线等（A7）
    pub legend: Legend,
}
pub struct Plot {
    pub id: PlotId,
    pub mark: MarkKind,                // 见 A3
    pub stat: StatKind,                // Identity / Mean / Bin / Kde / Box …
    pub mapping: Mapping,              // x, y, z, err_x, err_y, color, size, shape, label, group → DataRef
    pub x_axis: AxisId, pub y_axis: AxisId,
    pub style: PlotStyle,              // 线、符号、填充；None 表示继承分组
    pub rows: RowRange,
    pub visible: bool,
}
pub struct AxisSet { pub axes: Vec<Axis> }   // 每根轴：方向、位置(左/右/上/下/偏移)、刻度、范围、标题、链接
pub struct Axis { pub id: AxisId, pub dir: AxisDir, pub pos: AxisPos, pub scale: Scale, pub range: AxisRange, pub link: Option<AxisLink> }
pub struct AxisLink { pub to: (LayerId, AxisId), pub kind: LinkKind /* Straight | Custom(a,b) */ }
```

- 图层之间轴链接（Origin 的 Layer Link）用 `AxisLink`；多面板共享 X 轴就靠这个。
- 现有 14 种 `PlotType` 变成"预设"：生成一组 `Plot`（如 StackedBar = N 个 Bar mark + Stack stat）。
- 旧 `PlotSpec` 迁移（v10）：一个 spec → N 个 Plot（每个 y_key 一条）。
- 命令：`plot.add_curve`、`plot.remove_curve`、`plot.change_type`、`plot.move_to_axis`、`layer.add_right_y`、`layer.link_axes`。
- UI：图层内容对话框（Layer Contents，左边数据列表，右边图层曲线列表，中间加/删按钮）。

**验收**：
- 同一图层：柱（左 Y）+ 线+符号（右 Y），可以导出 SVG 并与参考图像素差异 < 1%。
- 图层内增删曲线、改单条曲线类型，可撤销。
- 两个图层共享 X 轴，缩放一个另一个跟着变。
- v9 → v10 迁移测试。

### A3 Mark / Stat / Coord 可组合渲染

**为什么**：`scene.rs` 1213 行，`build_layer` 按 `PlotType` 分支。照这个写法加 235 种图会变成上万行的 match。

**设计**：

```rust
pub trait Stat { fn compute(&self, data: &MappedData) -> StatOutput; }          // Identity, Mean, Bin, Kde, BoxSummary, Density2D, Contour, Regression …
pub trait Mark { fn build(&self, ctx: &MarkCtx, data: &StatOutput, out: &mut Vec<Prim>); fn legend(&self) -> LegendSwatch; }
pub trait Coord { fn to_page(&self, p: [f64;2]) -> [f32;2]; fn gridlines(&self) -> …; }  // Cartesian, Polar, Ternary, Smith, Projection3D
pub enum Position { Identity, Dodge{width}, Stack, Fill, Jitter{w,h,seed} }
```

- Marks：Line、Point、Bar、Area、Step、Spline、Text、ErrorBar、Drop、Box、Violin、Heatmap、Contour、Surface、Pie、Vector、Candlestick、Polygon。
- 预设（`presets/*.csv` → `presets/*.toml`）：`{ name, category, marks: [...], stat, position, coord, data_shape: "XY" | "XYY" | "XYZ" | "Matrix" | "Groups" }`。新增图类型应该只是加预设文件，少量加 Mark。
- `scene::build_layer` 只做：对每个 Plot → Stat → Position → Coord → Mark，最后统一裁剪和排序。
- `scene.rs` 拆成 `scene/{layout,axis,legend,marks/*,stats/*,coords/*}.rs`，每个文件 < 400 行。

**验收**：
- 现有 14 种图的 SVG 输出与重构前逐字节一致（先把现有输出存成 golden 文件，再重构）。
- 加一个新图类型（如带抖动的点图）只需新增 1 个预设 + 0 个 Mark。
- `scene/` 下没有 `match plot_type` 之类的总分支。

### A4 表达式引擎

**为什么**：设置列值（F035）、用户自定义拟合函数（F142）、函数图、筛选条件、参数约束都要它。

**设计**：
- `scicanvas-core/src/expr/`：词法 → Pratt 解析 → AST → 向量化求值（对整列一次求值，不逐行解释）。
- 语法兼容 Origin 列公式的常用子集：`col(A)`、`A`、`col("Long Name")`、`i`（行号）、`wcol(n)`、`+ - * / ^`、比较和逻辑、`if(c,a,b)`。
- 函数库：数学（sin…atanh、exp、ln、log、sqrt、abs、mod、floor/ceil/round…）、统计（mean、sd、sum、min、max、median、percentile、count、cumsum、diff、rank）、字符串（left、right、mid、len、upper、lower、find、replace、value、text）、日期（date、year、month、day、hour、now）、随机（rnd、normal，指定种子）。目标 1.0 前约 120 个。
- 符号求导：AST 求导 + 化简，供 LM 拟合用解析雅可比；失败时退回数值差分。
- 列公式依赖图：列 → 依赖的列，变化后按拓扑序重算，检测循环引用。

**依赖**：自己写（解析器 ~800 行），不要用 `meval`/`evalexpr`（不支持向量化和求导）。

**验收**：
- 300+ 条公式单元测试，与手算/NumPy 结果比对（相对误差 < 1e-12）。
- 100 万行 `sin(A)*exp(-B/2)` 求值 < 100 ms。
- 公式列在源列编辑后自动更新，循环引用给出明确错误。

### A5 操作对象 + 重算 + 报告表

**为什么**：Origin 的分析结果（拟合、统计、FFT）都是"操作"：记录输入、参数、输出；源数据变化后可以重算（Auto/Manual）；结果写在报告表里，可折叠表格 + 嵌入图。没有这个，后面的批处理、分析模板、Python 录制都没法做。

**设计**：

```rust
pub struct Operation {
    pub id: OpId, pub kind: String,            // "fit.nonlinear", "stats.ttest2" …
    pub inputs: Vec<DataRef>, pub params: serde_json::Value,
    pub outputs: Vec<OutputRef>,               // 写出的列、报告表、图
    pub recalc: RecalcMode,                    // Auto | Manual | None
    pub stale: bool,
}
pub trait Analysis { fn id(&self)->&'static str; fn run(&self, inputs: &[ColumnView], params: &Value) -> Result<AnalysisOutput>; fn dialog(&self) -> DialogSchema; }
pub struct Report { pub title: String, pub sections: Vec<ReportSection> }  // Table / KeyValue / Notes / EmbeddedGraph，可嵌套
```

- 每个分析功能实现 `Analysis`，参数对话框由 `DialogSchema` 自动生成（字段类型、范围、默认值），不要每个分析手写 UI。
- 报告表作为 Book 的一种特殊 Sheet（`SheetKind::Report`），只读，带锁形图标；"Change Parameters" 重新打开对话框。
- 数据编辑时标记依赖 Operation 为 stale；Auto 模式下一帧末尾重算，Manual 模式显示黄色锁。
- 文件格式 v11：存 Operation 列表。

**验收**：
- 线性拟合改为 Operation：改源数据后报告和拟合曲线自动更新；Manual 模式下锁变黄，点击后更新。
- 同一个分析对话框框架至少服务 3 个分析（线性拟合、描述统计、t 检验）。

### A6 发布级导出 + 真实字体度量 + 剪贴板

**为什么**：现在只能导出 SVG，`text_width` 是估算值，导致标签对齐和 PNG/PDF 都没法做准。期刊投稿需要 PNG/TIFF 600 dpi、PDF/EPS 嵌入字体。

**设计**：
- 字体：`fontdb` 加载系统字体（Windows `C:\Windows\Fonts`）+ 内置 1 套开源字体（Noto Sans / Noto Sans CJK SC 子集、STIX Two Math）作为回退；`ttf-parser` + `rustybuzz` 做真实测宽。scene 里所有文本测量走 `FontMetrics` trait，屏幕和导出用同一实现。
- 富文本：Origin 转义 `\b()`、`\i()`、`\+()` 上标、`\-()` 下标、`\g()` 希腊字母、`\f:Font()`。解析成 `RichText { runs: Vec<Run> }`，scene 按 run 排版（F102）。
- PNG/TIFF/JPG：SVG → `resvg` + `tiny-skia` 光栅化；DPI 可选 72–1200；TIFF 用 `tiff` crate，支持 LZW 和 CMYK 选项可后做。
- PDF：`svg2pdf`（或 `krilla`）直接从 scene 生成，字体子集嵌入；EPS 后做（P2）。
- 剪贴板：Windows 下同时放 PNG（CF_DIB）和 EMF；EMF 可先用 PNG 代替，Word/PPT 粘贴可用。用 `arboard` + `clipboard-win`。
- 导出对话框：格式、尺寸（按页面 / 自定义 mm/in）、DPI、背景透明、导出范围（当前图 / 所有图 / 选中图批量）。

**验收**：
- 同一图的 SVG、PNG(300 dpi)、PDF 三者对比，文本位置偏差 < 0.5 px（300 dpi 下）。
- 中文标签、希腊字母、上下标在 PDF 中可选中复制，字体已嵌入。
- 粘贴到 Word / PowerPoint 正常显示。

### A7 图形对象 + 拾取编辑

**设计**：`GraphObject { id, kind: Text(RichText) | Line | Arrow | Rect | Ellipse | Polygon | Image | RefLine{axis,value}, anchor: Anchor(Page|Layer|Data), style, z }`；scene 输出 `Prim` 时带 `HitId`（Plot / Axis / Legend / Object / Layer），屏幕点击通过命中测试选中对象，双击打开对应属性面板，拖动移动并写入历史。

**验收**：点击曲线 → 打开该曲线样式；点击轴 → 轴对话框；拖动图例和文本，可撤销；参考线随数据坐标移动。

### A8 数值基础库

| 用途 | 方案 | 说明 |
| --- | --- | --- |
| 线性代数 | `faer`（首选）或 `nalgebra` | QR/SVD 用于线性拟合和协方差 |
| 分布与特殊函数 | `statrs` + 自写补充（非中心 t/F 等） | 所有 p 值与 R 对比到 1e-10 |
| FFT | `rustfft` + `realfft` | |
| 三角剖分 | `spade` | 等高线、XYZ 网格化 |
| 等值线 | 自写 Marching Squares（或 `contour` crate） | |
| 优化 | 自写 Levenberg–Marquardt（带边界与线性约束），参考 MINPACK | 不要依赖 `argmin` 的 LM，控制力不足 |
| 样条/插值 | 已有 `spline.rs`，扩展 Akima、PCHIP、B 样条、平滑样条 | |

全部依赖用 `cargo deny` 检查许可证，禁止 GPL/LGPL。

---

## 6. 分模块缺口与路线

下面每个模块列出"缺什么 → 怎么做 → 依赖哪个 A"。

### 6.1 工作区与项目

**2026-10-18 核对**：已实现：.scv/旧项目、目录拖放/重命名、窗口吸附与辅助排列、快捷键焦点分流。剩余：项目搜索、自动恢复、最近文件、附件与窗口状态持久化。 以下保留完整设计目标。

- 缺：项目搜索、项目内对象重命名冲突处理、自动保存与崩溃恢复、最近文件、附加文件、窗口状态持久化。
- 做法：自动保存每 5 分钟写 `.sccproj.autosave`，启动时检测；最近文件存在 `%APPDATA%\SciCanvas\recent.json`。
- 依赖：无。

### 6.2 工作表

**2026-10-18 核对**：已实现待验收：行增删、公式一次性计算、序列填充、列运算、行统计。部分实现：列角色、列移动/排序、大数据、公式列。剩余：类型化列、格式、查找筛选等。 以下保留完整设计目标。

- 剩余：F032 数字格式、F035 公式持久化与重算、列属性对话框（长名、单位、注释、类型、格式）、筛选、查找替换、冻结、条件格式（P2）、百万行专项性能验收（已有可见行虚拟化）。
- 做法：基于 A1；表格绘制只渲染可见行（egui `ScrollArea::show_rows`）；列头三行（长名/单位/注释）可开关；F035 基于 A4 的公式对话框（左侧函数树，右侧编辑框，下方预览前 10 行）。
- 依赖：A1、A4。

### 6.3 矩阵与网格化

- 缺：Matrix 对象、矩阵编辑、XYZ → 矩阵网格化（规则网格、Kriging 可后做、Delaunay 线性、Shepard、薄板样条）、矩阵运算（转置、翻转、缩放、扩展）、矩阵 ↔ 图像。
- 依赖：A1、A8（spade）。

### 6.4 导入导出

**2026-10-18 核对**：已有 xlsx/xlsm 值导入、CSV 导出和多文件拖入。旧 xls、Excel 日期/样式、向导和后台处理仍待做；下面的格式清单和导入接口为完整目标。 以下保留完整设计目标。

| 优先级 | 格式 | crate |
| --- | --- | --- |
| P0 | CSV/TXT 导入向导（分隔符、编码、表头行、跳过行、预览）；CSV 导出；拖放导入 | 已有 csv + `encoding_rs` |
| P0 | Excel xlsx/xls 导入 | `calamine` |
| P1 | Excel xlsx 导出 | `rust_xlsxwriter` |
| P1 | 图片导入（作为矩阵 / 图层背景） | `image` |
| P1 | MATLAB .mat v5 | 自写或 `matfile` |
| P2 | HDF5 / NetCDF / Parquet | `hdf5`（需要 C 库，考虑后做）、`netcdf`、`parquet` |
| P2 | 仪器格式（JCAMP-DX、SPC、pCLAMP…） | 自写，按用户需求 |
| 不做 | Origin .opju 读写（专有格式） | |

导入统一实现 `trait Importer { fn sniff(&[u8]) -> f32; fn preview(...) -> Preview; fn import(...) -> Vec<Sheet>; }`，导入设置可保存为"导入过滤器"复用。

### 6.5 绘图基础

**2026-10-18 核对**：已有多层自由布局、图例更新/避让、Nature 默认样式及三种多面板快捷创建。图形主题保存/切换、图例手工编辑、独立 Plot/AxisSet、轴联动与对象拾取待做。 以下保留完整设计目标。

- 缺：F083 图层内增删曲线（A2）、F089 图形模板、F102 富文本（A6）、曲线样式细节（填充图案、符号边/内、连线方式、Modifiers 按列映射颜色/大小）、颜色递增分组、轴断点、参考线、轴刻度标签格式（日期/分类/自定义）、图例手动编辑（自动更新/避让已有）、对象拾取（A7）。
- 图形模板：模板 = `Page` 去掉 DataRef 的序列化（保留样式、轴、图层布局、对象），另存为 `.sctpl`；套用时按 data_shape 把新数据绑定进去。"克隆图并换数据"同理。
- 依赖：A2、A3、A6、A7。

### 6.6 图类型

**2026-10-18 核对**：14 种预设保留，另有上下/左右/四面板快捷创建。B1/B2 为部分实现；其余批次与更多面板组合仍待推进。频数统计结果表按分析能力记录，不等同直方图图型实现。 以下保留完整设计目标。

**按 A3 完成后，以"预设 + 少量新 Mark/Stat/Coord"批量铺。**沿用上一版路线中的 6 批，修订如下：

| 批次 | 内容 | 新增 Mark / Stat / Coord | 约数量 |
| --- | --- | --- | --- |
| B1 | 剩余 2D：点线变体、散点变体（气泡、颜色映射、带标签）、柱状变体（分组、带误差、条形堆叠）、面积变体、高低线、Drop line、多轴（双 Y、多 Y、双 X、Y 偏移） | Bubble 映射、Dodge | ~45 |
| B2 | 多面板（垂直、水平、4/9 面板、堆叠、合并图）、瀑布、插图（Inset）、饼图/环图 | Pie Mark、Polar 布局 | ~30 |
| B3 | 统计图：直方图、带分布曲线直方图、箱线图、小提琴图、QQ/PP、Pareto、散点矩阵、边际图、平均值条、Raincloud、Beeswarm、Ridgeline、热图 | Bin、Kde、BoxSummary、Jitter；Box/Violin Mark | ~42 + 分组 14 |
| B4 | 等高线与热图（XYZ 和矩阵）、彩色填充、Profile 图、图像图 | Contour Stat、Heatmap Mark、颜色条 | ~17 |
| B5 | 专业图：极坐标、三元图、Smith 圆图、风玫瑰、雷达、向量场、Polar 等高线、Piper/Durov（P2）、金融（K 线、OHLC、成交量） | Polar/Ternary/Smith Coord、Vector、Candlestick | ~30 |
| B6 | 3D：散点、柱、曲面（彩色、线框、带投影）、XYY 3D 柱/墙、3D 向量 | Projection3D Coord（CPU 投影 + 深度排序先做；wgpu 交互旋转后做） | ~37 |
| — | 地图（6 条） | 建议标"不做"或 P2，依赖 Shapefile/GeoJSON | — |

每批验收：每种图一个 golden SVG + 一张 Origin 同数据截图的人工对照；"统计图"另外要求统计量与 R 结果一致。

### 6.7 图形探索与小工具

**2026-10-18 核对**：现有坐标读数/十字光标与独立缩放；FFT、寻峰和统计菜单不等同图内 ROI 小工具，Gadget/刷选/拾取仍待做。 以下保留完整设计目标。

- 缺：数据读取器（Data Reader/屏幕读数）、数据选择器（Data Selector，选范围）、刷选（Data Highlighter）、缩放平移已有部分；Gadgets：积分、统计、拟合、FFT、微分、基线、四分位、聚类（P2）。
- 做法：Gadget = 一个"感兴趣区域（ROI 矩形）+ 一个 Analysis"，结果显示在 ROI 旁边，可输出到报告表。基于 A5 + A7，所有 Gadget 共享一个框架。

### 6.8 回归与拟合

**2026-10-18 核对**：已接入多元回归、LM 与 11 种模型/自动初值/固定参数、临时拟合报告。参数边界/约束/权重、自定义/全局拟合、完整函数库、完整报告和 NIST 验收仍属下述目标。 以下保留完整设计目标。

这是 Origin 用户最看重的模块，建议投入最大。

- 线性：简单线性、带权重、多元线性（MLR）、多项式、过原点、固定斜率、比较两条直线；输出 R²、调整 R²、ANOVA、参数 SE/t/p/置信区间、残差图、置信带和预测带。
- 非线性（NLFit）：
  - 算法：Levenberg–Marquardt（带参数上下界）、线性约束（投影或罚函数）、Orthogonal Distance Regression（P1）、单纯形（P2）。
  - 初值：每个内置函数带"参数初始化代码"（Origin 每个 FDF 都有），用 Rust 闭包实现，如高斯峰从最大值和半高宽估计。
  - 权重：无、仪器（误差列）、统计、任意列。
  - 共享参数 / 全局拟合（多数据集共享部分参数）：P1。
  - 报告：参数表、统计量（χ²、R²、RSS、AIC/BIC）、协方差/相关矩阵、残差分析、拟合曲线。
- 函数库：内置 239 个函数分 10 个族（Exponential、Growth/Sigmoidal、Peak、Power、Rational、Polynomial、Waveform、Pharmacology、Statistics、Hyperbola…）。**以数据形式**存 `fitfuncs/*.toml`：`{ name, category, formula, params, bounds, init: "builtin:gauss" | 表达式 }`，不要每个函数写一个 Rust 结构。P0 先做最常用 40 个（Gauss、Lorentz、Voigt、ExpDec1/2/3、ExpGrow、Boltzmann、Logistic、Hill、Allometric、Poly、Sine…），其余按族逐批补。
- 用户自定义函数（F142）：基于 A4，函数编辑器带参数表、初值、导数自动推导，保存到用户目录。
- 隐函数、曲面拟合（XYZ / 矩阵）：P1。
- 依赖：A4、A5、A8。
- **验收：NIST StRD 非线性回归全部 27 个数据集**，参数与认证值至少 6 位有效数字一致（低难度 / 中难度要求 7 位）；线性回归 11 个数据集到 1e-10。

### 6.9 数学运算

**2026-10-18 核对**：已接入三种 1D 插值/外推、列运算/曲线相减、曲线平均、微积分和归一化；SG 直接导数、PCHIP、多维插值、用户 X 查询入口与通用曲线合并待补。 以下保留完整设计目标。

- 剩余：PCHIP、多维插值、用户 X 查询入口、通用合并、Simpson 积分、SG 直接求导及更多归一化/运算方法。线性/样条/Akima、列算术、曲线相减/平均、梯形积分、有限差分和基线扣除已有。

### 6.10 统计

**2026-10-18 核对**：已接入列/行/频数统计、三类 t 检验（双样本含 Welch）、三种相关、单因素 ANOVA/Tukey、Shapiro–Wilk 和 Grubbs；下面完整目标中的其他检验/统计图尚待推进。 以下保留完整设计目标。

- 描述统计：按列/按行/按组、频数分布（F177）、离散频数、交叉表、相关系数（Pearson/Spearman/Kendall）、正态性检验（Shapiro-Wilk、KS、Lilliefors、Anderson-Darling、D'Agostino）。
- 假设检验：单样本/双样本/配对 t（F185–F187）、方差齐性（Levene、Brown-Forsythe、F）、单/双因素 ANOVA、重复测量 ANOVA、均值比较（Tukey、Bonferroni、Scheffe、Dunnett…）。
- 非参数：Mann-Whitney、Wilcoxon 符号秩、Kruskal-Wallis、Friedman、Mood 中位数、双样本 KS。
- 多元（P2）：PCA、聚类（K-means、层次）、判别分析、PLS。
- 生存（P2）：Kaplan-Meier、Cox、Weibull。
- 功效与样本量（P2）。
- 输出全部走 A5 报告表。
- **验收**：每个检验对照 R（`t.test`、`aov`、`TukeyHSD`、`shapiro.test` 等）生成的参考值，p 值相对误差 < 1e-8。参考值由 `tests/reference/gen_r.R` 脚本生成并提交到仓库。

### 6.11 信号处理

**2026-10-18 核对**：已有 FFT 三种窗和谱、四类理想频域滤波、自/互相关。独立 IFFT 命令、IIR/STFT/卷积/小波及其他滤波仍待做；频域滤波内部的逆变换单独注明。 以下保留完整设计目标。

- 平滑（已有部分：补 Savitzky-Golay、Percentile、FFT 滤波、LOWESS/LOESS）、FFT（F204，振幅/相位/功率谱、窗函数、单边/双边）、IFFT、STFT（P1）、相关与卷积、去卷积（P2）、FFT 滤波器（低/高/带通/带阻/阈值）、IIR/FIR 滤波器设计（P1）、包络、Hilbert 变换、小波（P2）。
- 依赖：A8（rustfft）。
- 验收：与 SciPy `numpy.fft`、`scipy.signal.savgol_filter`、`scipy.signal.butter`+`filtfilt` 输出对比到 1e-10。

### 6.12 峰分析

**2026-10-18 核对**：已有连线/ALS 基线、扣除、局部极大/突出度寻峰、半突出度宽度及基线以上峰积分/质心；导数寻峰、锚点/多项式基线、多峰拟合与完整峰分析器仍待补。 以下保留完整设计目标。

- 基线（锚点 + 插值、ALS、多项式拟合、Asymmetric Least Squares）、寻峰（局部最大、二阶导、高度/宽度/距离阈值）、峰积分（面积、中心、FWHM）、多峰拟合（基于 NLFit 的复合函数 Gauss/Lorentz/Voigt/PearsonVII，共享基线）。
- UI：Peak Analyzer 向导（基线 → 寻峰 → 积分 / 拟合 → 报告），可视化拖动锚点。
- 依赖：A5、A7、6.8 NLFit。

### 6.13 图像处理

- 1.0 只做：导入图像为矩阵、灰度/伪彩色显示、剖面（Profile）、裁剪/旋转/缩放、阈值二值化、直方图。其余 52 个 Image X-Function 和 13 个 Vision X-Function 标为 P2。
- 依赖：A1 矩阵、`image`、`imageproc`。

### 6.14 导出与发布

**2026-10-18 核对**：已有 SVG、PNG/DPI 与 300 dpi 位图剪贴板。PDF/TIFF/EMF、矢量剪贴板、打印和批量导出仍待实现。 以下保留完整设计目标。

- 见 A6。另外：批量导出所有图、按期刊预设尺寸（单栏 85 mm / 双栏 175 mm）导出、字体统一替换、"出版样式"一键套用（SCI 风格模板已在默认样式中，扩展为可选主题）。

### 6.15 自动化与编程

保持原路线：内置 Python 运行时（PyO3 + 嵌入 CPython）、`scicanvas` Python 包、脚本窗口、操作录制、包管理。前置条件是 A5（所有操作都是可序列化的 Operation + 命令），否则录制做不了。建议放在 1.0 之后的 1.1。

---

## 7. 分阶段任务卡（给 GPT 的工作单）

**规则（每张卡都适用）**：
1. 一张卡一个 PR，PR 内必须包含测试，`tools/check.py` 和 `cargo clippy -D warnings` 全通过。
2. 改了模型就要加文件格式迁移和迁移测试。
3. 改了 parity 状态就同时更新 `docs/parity/parity.json` 和 HTML 清单。
4. 不要在 UI 里写业务逻辑；core 先实现、先测试，UI 只调命令。
5. 渲染相关改动必须有 golden SVG 测试。

### 阶段 0：重构前准备（第 1 周）

| 卡 | 内容 | 验收 | 2026-10-18 状态 |
| --- | --- | --- | --- |
| T0.1 | 为现有 14 种图 + 误差棒 + 标签 + 多层生成 golden SVG（`tests/golden/*.svg`），加 `cargo test` 比较 | golden 测试稳定通过 | 已实现待验收：18 SVG，人工/Origin 对照待做 |
| T0.2 | 加 `cargo deny` 配置和 CI 步骤 | CI 绿 | 已完成：审计入口/许可例外已记录 |
| T0.3 | 加 parity 新增条目（第 3 节表格）和 `sub_items` 字段，HTML 同步 | 清单可显示子项进度 | 已完成：514 条目，15 条含子项 |
| T0.4 | 性能基准 `benches/`：10 万 / 100 万行导入、渲染、排序 | 输出记录到 docs/perf.md | 部分实现：导入/排版/排序基线，首帧/内存待测 |

### 阶段 1：结构重构（第 2–8 周）

| 卡 | 内容 | 依赖 | 周 | 2026-10-18 状态 |
| --- | --- | --- | --- | --- |
| T1.1 | A1-a：`ColumnData` 类型化列 + `ColumnView` + v9 迁移 | T0.1 | 2 | 已实现待验收：四种存储、ColumnView、v9 迁移/旧 SVG 回归已接入，人工与峰值内存待验收 |
| T1.2 | A1-b：独立工作表的列属性对话框 + 数字格式（F032）；F049 多 Sheet 按用户要求不做 | T1.1 | 1 | 部分实现：列类型转换已有，完整属性与显示格式待做；各表独立 Book |
| T1.3 | A3：scene 拆分为 Mark/Stat/Position/Coord，14 种图迁移到预设，golden 不变 | T0.1 | 2 | 未开始：保留 PlotType 场景分支 |
| T1.4 | A2：Layer → Vec\<Plot\> + AxisSet + v10 迁移；图层内容对话框（F083）；双 Y（F305） | T1.3 | 2 | 未开始：单 PlotSpec，无 AxisSet |
| T1.5 | A6-a：fontdb + rustybuzz 真实测宽，替换 `text_width`；内置回退字体 | T1.3 | 1 | 未开始：PNG 栅格字体不等同共享场景测宽 |

> T1.1 与 T1.3 可并行。

### 阶段 2：P0 收口（第 9–16 周）

| 卡 | 内容 | 依赖 | 2026-10-18 状态 |
| --- | --- | --- | --- |
| T2.1 | A4 表达式引擎 + 120 函数 + 设置列值对话框（F035、F168） | T1.1 | 部分实现：23 独立函数/一次性列公式，120 函数与依赖重算待补 |
| T2.2 | 插入/删除行（F028）、虚拟滚动、查找替换 | T1.1 | 部分实现：行增删/虚拟表格已有，查找替换与百万行验收待补 |
| T2.3 | 导入向导 + Excel 导入 + CSV 导出 + 拖放（F065、F066、F068、F069） | T1.2 | 部分实现：xlsx/xlsm、CSV 导出/拖入已有，向导与旧 xls 待做 |
| T2.4 | A6-b：PNG/TIFF/PDF 导出 + 剪贴板 + 导出对话框（F227、F228、F231） | T1.5 | 部分实现：PNG/DPI、位图复制和导出对话框已有；TIFF/PDF 待做 |
| T2.5 | A6-c：富文本标签（F102） | T1.5 | 未开始：现有纯文本标签不等同富文本 |
| T2.6 | A5：Operation + 报告表 + 重算 + 对话框 Schema；把线性拟合、多项式拟合改成 Operation | T1.2 | 未开始：现有结果表/临时报告尚未改造为 Operation 和依赖重算 |
| T2.7 | NLFit：LM + 约束 + 初值 + 报告（F134–F138）；NIST StRD 测试 | T2.1、T2.6 | 部分实现：LM/初值/固定参数已有，约束/完整报告/NIST 待做 |
| T2.8 | 拟合函数库 TOML + 前 40 个函数（F148–F152）+ 用户自定义函数（F142） | T2.7 | 部分实现：11 模型；函数库目标未收口 |
| T2.9 | 统计：描述统计报告、频数（F177）、t 检验三种（F185–F187）、R 参考值脚本 | T2.6 | 部分实现：描述/频数/t 检验已有，统一报告与完整 R 验收脚本待补 |
| T2.10 | 插值（F158）、FFT（F204） | T2.6 | 部分实现：三种插值和 FFT 谱已有；F204 独立 IFFT 命令待做 |
| T2.11 | 多面板预设（F321–F323）+ 轴链接 | T1.4 | 部分实现：上下/左右/四面板已有；轴链接待做 |
| T2.12 | 图形模板（F089） | T1.4 | 未开始，按原任务范围推进 |
| T2.13 | 原 13 条及当前 P0 部分实现项逐条补完 | — | 部分实现：原 13 项仍有缺口；当前 P0 部分实现范围见 2.2 |

**阶段 2 结束 = 公测（约第 16–20 周）**：P0 87 条全部"已完成"。

### 阶段 3：图类型批次 + 分析扩展（第 17–36 周）

| 卡 | 内容 | 2026-10-18 状态 |
| --- | --- | --- |
| T3.1 | A7 图形对象 + 拾取编辑 + 参考线 | 未开始：图形对象标签、拾取编辑与参考线待做 |
| T3.2 | 图类型 B1（剩余 2D + 多轴） | 部分实现：现有 2D 预设，多轴待做 |
| T3.3 | 图类型 B2（多面板全集、瀑布、插图、饼图） | 部分实现：2/4 面板已有，全集/瀑布/饼图待做 |
| T3.4 | 图类型 B3（统计图 + 分组图）+ ANOVA、非参数、正态性检验 | 部分实现：单因素/Tukey/Shapiro 已有，统计图和其余检验待做 |
| T3.5 | 矩阵对象 + 网格化 + 图类型 B4（等高线/热图） | 未开始：矩阵/网格化/等高线待做 |
| T3.6 | 拟合函数库补全到 239 + 全局拟合 + 曲面拟合 | 部分实现：11 模型，全局/曲面与完整库待做 |
| T3.7 | 信号处理（滤波、相关、STFT）+ 峰分析器 | 部分实现：FFT 滤波/相关/基线/寻峰已有，STFT/完整峰分析器待做 |
| T3.8 | 图类型 B5（专业图、金融） | 未开始 |
| T3.9 | 图形探索：数据读取器、选择器、刷选、Gadget 框架 + 5 个 Gadget | 部分实现：坐标读数已有，数据点读取/选择/刷选与 ROI 工具待做 |

### 阶段 4：1.0（第 37–44 周）

| 卡 | 内容 | 2026-10-18 状态 |
| --- | --- | --- |
| T4.1 | 图类型 B6（3D，CPU 投影版） | 未开始，按原任务范围推进 |
| T4.2 | 批处理、分析模板、对话框主题 | 未开始，按原任务范围推进 |
| T4.3 | 图像处理 1.0 子集 | 未开始，按原任务范围推进 |
| T4.4 | 性能与稳定性：百万行目标、崩溃恢复、内存泄漏检查 | 部分实现：已有性能基准；完整首帧/峰值内存、崩溃恢复与泄漏专项待补 |
| T4.5 | 安装包、文档、示例项目 | 部分实现：文档与 14 份 .scv 示例已有；安装包/发布自动化待做 |

相比上一版时间线（41 周到 1.0），因为插入了阶段 1 的结构重构，**1.0 推迟到约第 44 周，公测仍在约第 20 周**。重构多花 3 周，但后面每个图类型和分析功能都会更快，整体不亏。

---

## 8. 验证体系

| 类别 | 方法 | 位置 |
| --- | --- | --- |
| 渲染 | golden SVG 逐字节比较；PNG 用像素差 < 1% | `tests/golden/` |
| 拟合 | NIST StRD（线性 11 + 非线性 27 数据集） | `tests/nist/` |
| 统计 | R 脚本生成参考值（JSON），测试比对 | `tests/reference/gen_r.R` |
| 信号/数学 | SciPy 脚本生成参考值 | `tests/reference/gen_scipy.py` |
| 与 Origin 对照 | 同一数据在 Origin 中出图、截图，人工比对，记录在 `docs/parity/visual/` | 每批图类型 |
| 性能 | `benches/` + 记录表 `docs/perf.md`，回归超过 20% CI 告警 | |
| 文件兼容 | 每个历史版本保存一个示例项目，迁移测试读取并对比输出 | `tests/fixtures/v*/` |

参考值脚本只在开发者机器上运行，生成的 JSON 提交到仓库，CI 不需要装 R/Python。

---

## 9. 给 GPT 的提示词模板

```
你在 SciCanvas 仓库（Rust，scicanvas-core + scicanvas 两个 crate，egui 0.29，Windows 优先）中工作。
请先阅读 AGENTS.md、docs/ROADMAP.md、docs/SciCanvas 功能完备性分析与技术路线.md 中的任务卡 {卡号}。

任务：{卡号} {标题}
范围：{涉及的文件/模块}
接口：{按文档第 5 节中对应的结构体/trait 定义实现；如需偏离，在 PR 描述中说明理由}
约束：
- core 不得依赖 UI；UI 只通过命令注册表调用 core。
- 模型改动必须提升文件格式版本并添加迁移和迁移测试。
- 渲染改动必须更新或新增 golden SVG 测试；未改变的图必须保持逐字节一致。
- 新增依赖必须是 MIT/Apache-2.0/BSD 许可，并通过 cargo deny。
- 更新 docs/parity/parity.json 和对照清单 HTML 中相关条目的状态。
验收：{复制任务卡的验收标准}
完成后运行 cargo fmt、cargo clippy --all-targets -- -D warnings、cargo test --all-features，全部通过再提交。
```

---

## 附：数据来源

- 历史分析基线：`scicanvas` @ ad11c63；最新代码核对为 d928ad3，测试 93 + 76 + 62 = 231。Clippy 历史结果与当前测试结果分别记录。
- 清单：`docs/parity/parity.json` 488 条。
- Origin.chm 目录 1199 项；X-Function.chm 615 个函数（数据操作 161、导入导出 71、实用工具 61、图像 52、统计 47、图形操作 44、绘图 44、数学 35、信号处理 30、拟合 25、数据探索 16、视觉 13、其他 6、光谱 6、数据库 4）；拟合函数 239 + 5 + 22 + 7。
