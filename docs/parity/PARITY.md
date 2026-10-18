# SciCanvas 功能对照清单

条目按 Origin 帮助文档的目录整理，只引用章节标题；实现按公开说明和观察到的行为编写。
数据源为 `parity.json`，页面模板为 `tracker_template.html`。运行 `python docs/parity/build_parity.py` 生成文档与页面；`--check` 检查同步，`--desktop` 同步桌面副本。新增条目追加稳定编号，已有编号保留。

共 514 项：P0 97 项，P1 146 项，P2 259 项，不做 12 项。
状态：已完成 26 项，已实现待验收 52 项，部分实现 45 项，未开始 379 项，不做 12 项。

- **P0 / M1 日常够用**：日常数据处理和论文作图的最小集合。
- **P1 / M2 论文全流程**：统计、多面板、信号处理等。
- **P2 / M3 高级功能**：3D、专业图与高级分析。
- **部分实现**：范围内仍有明确缺口；**已实现待验收**：已接入工作流，专项或参考对照待完成。
- **已完成**：完成当前条目的实现与既定验证；**未开始**按桌面代码判断，旧网页演示单独说明。
- **子项**：`sub_items` 按名称记录 `total`、`done`、可选 `review` / `partial`、`note`。已完成与待验收分别计数；未开始为总数减去以上计数。
- 子项独立于共享存储中的条目状态，修改条目状态不会自动增加子项完成数。规划目标与路线参考基数在备注中标明；重叠目录分别查看，避免重复加总。
- F489 起补充第三阶段需求。F514 为 F148–F156 的目录总览，239 是路线参考数；当前通用线性/多项式回归单独计数。
- Origin 私有格式与脚本兼容保留原范围；扩展包采用 SciCanvas 自有格式。

验证原则：分析与 Origin、NIST StRD、SciPy / R 对照；图形与 Origin 同数据出图比较。

## 工作区与项目

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F001 | 多文档子窗口（拖动、缩放、最小化、最大化） | Basic Child Window Operations | P0 | 已完成 | 自绘 MDI；拖动/缩放、最小化换行、最大化/还原与统一窗口命令 | 真实 egui 拖动、缩放、窗口按钮与几何恢复回归 | — |
| F002 | 项目管理器（文件夹树、显示 / 隐藏窗口） | Project Explorer | P0 | 已完成 | 真实多级目录；拖放/菜单移动、慢速双击/F2 就地重命名、删除保留内容、显示/隐藏窗口；快捷键按焦点分流；Ctrl 多选、Shift 连续选择、Ctrl+Shift 追加范围；按名称/类型排序，跳过折叠内容 | 项目树、移动/重命名历史、F2 与工作表键盘隔离回归已通过 | — |
| F003 | 项目管理器多级文件夹、拖动整理 | Using Project Explorer to Manage Child Windows | P1 | 已实现待验收 | 真实多级目录；工作簿、图形与文件夹拖入目录/拖回根目录，目标高亮；名称慢速双击或 F2 就地编辑；菜单移动保留，均复用撤销事务；多选批量移动保留内部层级，整批失败回滚；Note 同样支持目录和移动 | 真实 egui 鼠标拖放、根目录移动、自身/下级拒绝、同目录无操作、慢/快速点击、Enter/Esc/失焦与撤销重做回归通过；Windows 手工交互验收待做 | — |
| F004 | 窗口排列：层叠 / 平铺 | Arrange Windows | P1 | 已实现待验收 | 层叠/平铺、边缘及四角吸附、顶部吸附布局与 Snap Assist 候选补位已接入 | 窗口几何、吸附和候选补位回归通过；Windows 不同缩放、窄窗口手工验收待做 | — |
| F005 | 项目文件保存 / 打开 | Opening, Closing and Backing Up the Origin Project File | P0 | 已完成 | v10 项目：类型化数据列与 Note 文本；兼容 v0–v9，JSON/SCV 共用校验和原子保存，保留原始数值写法及 notes/*.txt 预览 | 旧版迁移与损坏输入、JSON/SCV 往返、14 份旧示例 CSV/SVG 字节一致及原有原子保存回归通过 | — |
| F006 | 自动备份与崩溃恢复 | Opening, Closing and Backing Up the Origin Project File | P1 | 未开始 | 定时写临时文件 | 强制退出后恢复 | — |
| F007 | 最近打开的文件 | Recent Origin Files | P1 | 未开始 | 配置文件记录 | 手动操作 | — |
| F008 | 撤销 / 重做 | — | P0 | 已完成 | 编辑事务；100 步 / 300 MiB；文件夹新建/重命名/移动/删除均可撤销 | 项目快照、真实编辑控件与文件夹事务回归 | — |
| F009 | 命令注册表（菜单、工具栏、脚本共用同一套命令） | — | P0 | 已完成 | 统一命令注册表、可用条件、开关状态与修改标记；菜单/按钮/程序调用共用入口，供后续脚本集成复用 | 命令 ID 唯一、状态条件、窗口/文件夹命令与事务回归 | — |
| F010 | 结果日志（分析结果记录） | Analysis Results: The Results Log | P1 | 未开始 | 停靠面板 + 结构化记录 | 手动操作 | — |
| F011 | Notes 文本窗口 / Markdown 报告 | Notes Window | P2 | 部分实现 | Note 独立文本窗口，编辑、重命名、移动、删除、撤销重做、UTF-8 导出与 v10 项目保存已接入；拟合/非线性拟合/t 检验/方差分析/相关系数/通用结果表可保存报告快照；Markdown 渲染和富文本待做 | 文本编辑及撤销、拟合报告真实按钮、目录与 ID、JSON/SCV 往返、旧版迁移与文本预览回归；Windows 人工验收待做 | — |
| F012 | 命令窗口（交互式脚本） | The Origin Command Window and Script Window | P1 | 未开始 | 嵌入 Python（PyO3） | 执行脚本修改工作表 | — |
| F013 | 深色模式 | Dark Mode | P0 | 已完成 | 浅色、深色、跟随系统三种选择；统一主题命令 | 主题切换命令与项目数据不变回归 | — |
| F014 | 快捷键与自定义快捷键 | Hotkeys | P1 | 部分实现 | 固定快捷键与命令面板已实现；用户自定义快捷键与冲突配置待做 | 固定快捷键、文字框撤销回归已通过；自定义部分待实现 | — |
| F015 | 全文搜索（在项目中查找） | Finding in Origin Project | P2 | 未开始 | 遍历项目数据 | 手动操作 | — |
| F016 | 工程保护（密码） | Protecting Origin Projects | P2 | 未开始 | age / chacha20poly1305 加密 | 加密→解密往返 | — |
| F017 | 中文界面 | — | P0 | 已完成 | 简体中文界面，Windows 系统中文字体回退 | 中文菜单与项目树控件回归 | — |
| F018 | 多语言界面（中 / 英） | — | P1 | 未开始 | fluent 或简单字符串表 | 切换语言检查 | — |
| F019 | 选项 / 偏好设置对话框 | Setting Your Preferences with the Options Dialog Box | P1 | 未开始 | 配置文件 + 设置面板 | 手动操作 | — |
| F020 | 自定义菜单与工具栏 | Custom Menus | P2 | 未开始 | 基于命令注册表 | 手动操作 | — |
| F021 | 模板中心 / 应用中心 | Template Center | 不做 | 不做 | — | — | — |
| F022 | 读取 Origin 工程文件 OPJU / OPJ | The Origin Project File | 不做 | 不做 | 私有格式无公开规范；旧 OPJ 可评估 liborigin | — | — |
| F512 | 项目附加文件 | Project / Attach File | P2 | 未开始 | 原生项目附件索引和导入/导出 | 附件往返、重名与损坏输入 | — |
| F513 | 扩展包打包与分发（OPX 用途对照） | Project / OPX Packaging | P2 | 未开始 | SciCanvas 原生扩展包与资源清单；Origin OPX 二进制兼容不在范围内 | 安装/卸载、包版本、资源完整性 | — |

## 工作表

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F023 | 单元格输入与编辑 | Basic Operations on Worksheet | P0 | 已完成 | egui_extras TableBuilder | 手动操作 | — |
| F024 | 长名称 / 单位 / 注释表头行 | Displaying Supporting Data in Worksheet Header Rows | P0 | 已完成 | 列元数据 | 轴标题自动取“长名称 (单位)” | — |
| F025 | 自定义表头行（参数、采样间隔、迷你图等） | Displaying Supporting Data in Worksheet Header Rows | P1 | 未开始 | 可扩展的表头行定义 | 手动操作 | — |
| F026 | 列属性：X / Y / Z / X误差 / Y误差 / 标签 / 忽略 | Setting Column Designation | P0 | 部分实现 | X/Y/X误差/Y误差/忽略及误差自动关联已实现；Z 与专用标签列角色待做 | 误差关联、删除/插列与撤销回归已通过；补齐其余角色后验收 | — |
| F027 | 插入 / 删除 / 移动列 | Arranging the Worksheet: Columns and Rows | P0 | 部分实现 | 列插入、删除、清空与稳定列标识已实现；移动列待做 | 列操作、引用清理与撤销回归已通过；列移动待实现 | — |
| F028 | 插入 / 删除行 | Arranging the Worksheet: Columns and Rows | P0 | 已实现待验收 | 工作表“行”分组与单元格右键菜单：在选区上方插入同数量空行、删除选区覆盖的整行；图形行范围随之平移，整段删除时改回全部行；一步撤销 | 核心库行操作与行范围回归、命令撤销重做回归；界面手动操作待验收 | — |
| F029 | 从 Excel 复制粘贴（多单元格） | Copying and Pasting Worksheet Data | P0 | 已完成 | 制表符解析 | 单元测试 | — |
| F030 | 鼠标拖动框选单元格、Shift 扩展、复制选区 | Scrolling and Simple Data Selection in the Worksheet | P0 | 已实现待验收 | Rust app.rs/sheet.rs 已接入拖动框选、Shift 扩展、复制/剪切/粘贴与清空选区 | 选区边界、键盘、剪贴板及撤销回归已通过；真实鼠标与长表人工验收待做 | — |
| F031 | 查找与替换 | Finding and Replacing Worksheet Data | P1 | 未开始 | — | 手动操作 | — |
| F032 | 数值显示格式（小数位、科学计数、工程单位） | Controlling How Numeric Data Display in the Worksheet Column | P0 | 已实现待验收 | 工作表 → 列属性（双击列标题或右键“列属性…”）：数值列可选原样、固定小数位、科学计数、工程单位（SI 词头）、百分比及小数位，日期列可选 4 种显示格式；只改变显示，存储值与原始写法不变，格式随项目保存 | 格式化单元测试（舍入进位、负零、非有限值、SI 词头越界）、项目往返与未设格式时文件不变、列属性对话框一步撤销及转换失败不改动的界面测试 | — |
| F033 | 日期时间列与计算 | Dates and Times in the Worksheet | P1 | 部分实现 | UTC 毫秒日期类型、ISO8601/常见年月日写法/四位年份输入、缺失位图、排序、类型转换、v9 保存已接入；本地时区/完整日期格式和日历运算待做 | 日期时区等价、行操作、排序/往返、失败转换原子性回归通过；完整日期计算与人工验收待补 | — |
| F034 | 文本列与分类数据 | Setting Column Designation | P1 | 已实现待验收 | Text 与 Category 字典列，列头右键/命令转换；编辑、缺失、复制、排序、保存和撤销已接入；图形的分类轴/分组绘图另按相关条目推进 | 类型、字典、空码、行重排、默认标签排序/有序字典排序、JSON/SCV 与界面转换回归通过；Windows 人工操作待验收 | — |
| F035 | 设置列值（公式列） | Using Formulae to Set or Transform Worksheet Column Values | P0 | 部分实现 | Ctrl+Q 设置列值：表达式、列引用/col(A)、行号 i、pi/e、24 个函数调用名；按当前数据一次性覆盖结果并支持撤销。列公式持久化、依赖图和自动重算待做 | 公式解析/优先级/错误、写回与撤销回归已通过；源列变化自动重算与循环依赖验收待实现 | — |
| F036 | 单元格公式（=A1+B1） | Using a Formula to Set Cell Values | P2 | 未开始 | 依赖图 + 增量重算 | 单元测试 | — |
| F037 | 生成数据序列（等差、随机数、正态分布） | Generating Simple Worksheet Datasets | P1 | 已实现待验收 | 工作表 → 填充序列：等差序列、均匀分布、正态分布随机数（SplitMix64 + Box–Muller），覆盖目标列，可撤销 | 单元测试：等差值精确；20000 个随机数的均值、标准差落在理论值附近；同种子可复现 | — |
| F038 | 排序（列、工作表、按多列） | Sorting Worksheet Data | P0 | 部分实现 | 按一列稳定排序整张工作表，同行联动，数值/文本/空值规则已实现；独立列排序与多关键字排序待做 | sort_book 行对齐、升降序与撤销回归已通过；其余排序模式待实现 | — |
| F039 | 数据筛选 | Data Filter | P1 | 未开始 | — | 手动操作 | — |
| F040 | 条件格式（按值着色） | Conditional Formatting | P2 | 未开始 | — | 目视检查 | — |
| F041 | 转置 | Transposing Worksheets | P1 | 未开始 | — | 单元测试 | — |
| F042 | 堆叠 / 拆分列 | Stacking and Unstacking Worksheet Columns | P1 | 未开始 | — | 与 Origin 结果比较 | — |
| F043 | 合并工作表（按列、按标签） | Join Multiple Sheets by Column | P2 | 未开始 | polars join | 与 Origin 结果比较 | — |
| F044 | 数据精简（隔行抽取、平均合并） | Data Reduction | P1 | 未开始 | — | 与 Origin 结果比较 | — |
| F045 | 删除重复行 | Reduce Duplicated Rows | P2 | 未开始 | — | 单元测试 | — |
| F046 | 数据透视表 | Pivot Table | P2 | 未开始 | polars pivot | 与 Origin 结果比较 | — |
| F047 | 工作表查询（条件提取） | Worksheet Query - Quick Start | P2 | 未开始 | — | 与 Origin 结果比较 | — |
| F048 | 冻结行列 | Freeze Rows Columns | P1 | 未开始 | — | 手动操作 | — |
| F049 | 多工作表（一个工作簿多张表） | Workbooks | 不做 | 不做 | 用户 2026-10-18 决定：每个 Book 只有一张表，多表使用独立窗口；Excel 多工作表导入继续拆成独立 Book，不采用多 Sheet 容器或底部标签页 | 后续保持一表一窗口、独立命名/目录/保存/绘图引用；Excel 拆表导入沿用现有回归 | — |
| F050 | 工作簿模板 | What is Saved with the Workbook Template (*.OTW or *.OTWU) File? | P1 | 未开始 | 保存列定义与格式 | 套模板后结构一致 | — |
| F051 | 选中数据的状态栏统计 | Scrolling and Simple Data Selection in the Worksheet | P0 | 已完成 | — | 与手算比较 | — |
| F052 | 单元格内嵌图片 / 迷你图 | Inserting Graphs Images and Other Objects into Worksheet Cells | P2 | 未开始 | — | 目视检查 | — |
| F053 | 大数据量（百万行流畅滚动） | Origin Limitations | P0 | 部分实现 | 类型化数值列、缺失位图和可见行表格已接入；本机百万行十列导入 1.462 秒、排版 185 ms、整表排序 54 ms；完整首帧/进程峰值内存和绘图抽点待验收 | 百万数值单列容量估算 <12 MiB；同机基准见 docs/perf.md，不等同完整界面首帧/峰值内存验收 | — |
| F510 | 日期时间列 | Worksheet / Column Properties / Date and Time | P0 | 部分实现 | DateTime 基础列已接入：UTC 毫秒存储、ISO8601/常见年月日写法/四位年份解析、位图/原文保留、转换/排序/v9 保存；本地时区配置、显示格式与日期轴待做 | typed_columns 与界面命令/错误输入回归通过；完整格式及日期轴人工/参考验收继续推进 | — |
| F511 | 分类列及类别排序 | Worksheet / Column Properties / Categorical | P0 | 部分实现 | Category 采用 u32 编码和类别字典，支持缺失、编辑、转换、保存与标签排序；ordered 字典顺序已有核心支持，类别顺序编辑界面和分类轴待做 | 字典合法性、行增删/排序、保持编码/顺序及往返回归通过；类别编辑与分类轴后续验收 | — |

## 矩阵

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F054 | 矩阵窗口（二维网格数据） | Basic Matrixbook and Matrixsheet Operations | P1 | 未开始 | ndarray | 手动操作 | — |
| F055 | 矩阵 XY 坐标映射 | Matrixsheet Dimensions and XY Coordinates | P1 | 未开始 | — | 与 Origin 比较 | — |
| F056 | 设置矩阵值（公式） | Setting Matrix Values | P1 | 未开始 | 复用公式解析器 | 与 Origin 比较 | — |
| F057 | 矩阵转置、旋转、翻转、缩放 | Manipulating Matrix Data | P2 | 未开始 | — | 单元测试 | — |
| F058 | 工作表直接转矩阵 | Direct Worksheet to Matrix Conversion | P1 | 未开始 | — | 单元测试 | — |
| F059 | XYZ 网格化：规则数据 | Gridding of Regularly Spaced Data | P1 | 未开始 | — | 与 Origin 比较 | — |
| F060 | XYZ 网格化：随机点（Delaunay 插值、克里金、Renka-Cline） | Gridding Methods for Randomly Spaced Data | P1 | 未开始 | spade（Delaunay）+ 自写克里金 | 与 Origin 网格结果比较 | — |
| F061 | 虚拟矩阵 | Managing Virtual Matrix | P2 | 未开始 | — | — | — |

## 导入导出

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F062 | CSV / TXT 导入（自动识别分隔符） | Importing Simple ASCII Data Files | P0 | 已完成 | csv.rs | 单元测试 | — |
| F063 | GBK / UTF-8 编码识别 | Customizing the ASCII Import Options | P0 | 已完成 | encoding_rs | 单元测试 | — |
| F064 | 表头自动拆分长名称与单位 | Customizing the ASCII Import Options | P0 | 已完成 | — | 单元测试 | — |
| F065 | 导入向导（跳过表头行、指定分隔符、预览） | The Import Wizard | P0 | 未开始 | 预览对话框 | 手动操作 | — |
| F066 | 导入 Excel（.xlsx / .xls） | Importing from Excel | P0 | 部分实现 | 导入 .xlsx / .xlsm：自写读取（zip + roxmltree），共享 / 内联 / 富文本字符串、公式缓存值、布尔与错误值，每张非空工作表一个工作簿；.xls、日期格式与合并单元格待做 | openpyxl 生成文件与手工构造的边界文件回归、拖放导入回归；Excel 实际保存文件逐格比较待做 | — |
| F067 | 导出 Excel | Exporting Worksheets and Matrices | P1 | 未开始 | rust_xlsxwriter | 用 Excel 打开检查 | — |
| F068 | 导出 CSV | Exporting Worksheets and Matrices | P0 | 已实现待验收 | 工作表 → 导出 CSV：UTF-8 BOM、CRLF，表头为“长名称 (单位)”，与 .scv 内的 CSV 同一实现 | 导出后重新导入的往返测试；Excel 打开待人工验收 | — |
| F069 | 拖放文件导入 | Importing Data by Drag-and-Drop | P0 | 已实现待验收 | 把文件拖进窗口：数据文件逐个导入为工作簿并合为一步撤销，项目文件（.scv / .scproj）直接打开；拖动时显示提示层 | 拖放事件回归（导入、撤销、打开项目）；资源管理器实际拖放待人工验收 | — |
| F070 | 批量导入多个文件 | Importing Simple ASCII Data Files | P1 | 已实现待验收 | 已有多文件拖入入口：数据文件逐个创建工作簿并合为一步撤销；混入项目时只打开第一个项目，其余忽略。当前文件对话框为单选，批量入口使用拖放 | 多文件拖入、结果写回、项目混入规则及一步撤销回归通过；Windows 多种格式、大批量文件与部分失败手工验收待做 | — |
| F071 | 从文件名和表头提取变量 | Extracting Variables from the File Name and Header | P2 | 未开始 | — | 单元测试 | — |
| F072 | 二进制数据导入 | Importing Simple Binary Files with the Import Wizard | P2 | 未开始 | — | 单元测试 | — |
| F073 | 第三方格式：MATLAB .mat | Importing Third Party Files | P2 | 未开始 | matfile | 与 MATLAB 比较 | — |
| F074 | 第三方格式：HDF5 / NetCDF | Importing Third Party Files | P2 | 未开始 | hdf5 / netcdf（需 C 库） | 与 h5py 比较 | — |
| F075 | 第三方格式：仪器数据（SPC、JCAMP-DX 等） | Importing Third Party Files | P2 | 未开始 | 逐个格式实现 | 与 Origin 导入结果比较 | — |
| F076 | Parquet / Arrow | — | P2 | 未开始 | polars | 往返测试 | — |
| F077 | 数据库导入（SQL 查询） | Importing Data from a Database | P2 | 未开始 | sqlx / duckdb | 手动操作 | — |
| F078 | 数据连接器（文件更新后自动重新导入） | Data Connector | P2 | 未开始 | notify 监听文件 | 改文件后检查 | — |
| F079 | 导入 Origin 工程中的工作表 | Importing from Origin Project | 不做 | 不做 | 私有格式 | — | — |

## 绘图基础

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F080 | 按选列自动配对 X / Y 绘图 | Creating Graphs from Graph Templates | P0 | 已完成 | plot_from_selection | 单元测试 | — |
| F081 | 按选区（部分行）绘图 | Creating Graphs from Graph Templates | P0 | 已实现待验收 | 工作表矩形选区经 picked/PlotSpec.rows 接入作图、图库与保存，按原始行截取 | 选区行范围、无效行、项目往返与 SVG 回归已通过；Origin 同数据人工对照待做 | — |
| F082 | 页面-图层-曲线 三层结构 | The Page-Layer-Plot Hierarchy | P0 | 部分实现 | 已有 Graph→Page→Layer→PlotSpec 与多 Y 曲线映射；每层持有一个 PlotSpec。持久化独立 Plot 对象、各曲线样式与 AxisSet 仍属 A2 | 多层/跨工作簿/保存与共享版面回归通过；独立 Plot 和同层混合图型待实现 | — |
| F083 | 往图层添加 / 移除曲线 | Adding or Swapping Data Plots in the Graph Layer | P0 | 未开始 | 图层内容对话框 | 手动操作 | — |
| F084 | 多图层图形 | Creating Multi-Layer Graphs | P0 | 已实现待验收 | 多图层增删、跨工作簿数据、独立属性/裁剪/视图、分析写回目标层已实现 | 跨工作簿、布局、裁剪、视图、保存与撤销回归已通过；Origin 多层图人工对照待做 | — |
| F085 | 函数绘图（y = f(x)） | Plotting Mathematical Functions | P1 | 未开始 | 复用公式解析器 | 与解析值比较 | — |
| F086 | 分组绘图（按分类列分组） | Plotting Data by Groups | P1 | 未开始 | — | 与 Origin 比较 | — |
| F087 | 图形缩放、平移 | Zooming on the Graph Page and inside Layer | P0 | 已完成 | 自绘 plot.rs/coord.rs：图层独立缩放、平移和视图复位；固定页面画布整体缩放 | 手动操作 | — |
| F088 | 批量绘图（同一模板换数据） | Batch Plotting | P1 | 未开始 | — | 手动操作 | — |
| F089 | 图形模板（保存样式，换数据一键出图） | Creating Graphs from Graph Templates | P0 | 未开始 | 图形样式与数据引用分离 | 套模板后版式一致 | — |
| F090 | 格式复制粘贴 | Copying and Pasting Formats | P1 | 未开始 | — | 手动操作 | — |
| F091 | 主题（图形样式主题） | Graph Formats and Themes | P1 | 未开始 | 已有 Nature 默认配色与版式；图形主题的保存、切换和应用入口待做，默认样式单独计入现有绘图能力 | 目视检查 | — |
| F092 | 坐标轴范围与刻度 | Controlling the Graph Axis Display Range | P0 | 已完成 | ticks.rs（1/2/5 步长） | 与 Origin 自动刻度比较 | — |
| F093 | 对数轴 | Axes Dialog Reference | P0 | 已完成 | — | 与 Origin 比较 | — |
| F094 | 其他坐标类型：倒数、概率、log2、ln、Logit | Axes Dialog Reference | P1 | 未开始 | — | 与 Origin 比较 | — |
| F095 | 坐标轴断点 | Axes Dialog Reference | P1 | 未开始 | — | 目视比较 | — |
| F096 | 刻度标签格式（科学计数、日期、自定义） | Axes Dialog Reference | P0 | 部分实现 | ticks.rs 已实现普通数值、a×10ⁿ 科学计数及对数刻度；日期与用户自定义格式待做 | 刻度边界与标签测试已通过；日期/自定义格式待实现 | — |
| F097 | 次刻度、刻度方向、四边框线 | Axes Dialog Reference | P0 | 部分实现 | 屏幕/SVG 共用主次刻度；默认 Nature 风格，只画左轴和下轴、刻度朝外；四边框、朝内刻度和独立方向设置待做 | 共享 scene、刻度与显示比例回归已通过；方向配置及人工目视验收待做 | — |
| F098 | 网格线 | Axes Dialog Reference | P0 | 已完成 | — | 目视检查 | — |
| F099 | 图例（自动更新） | Graph Legends | P0 | 已完成 | 图例按曲线名称和当前图层更新，并按实际图元避让；图例手动编辑/拖动另见 F100 | 目视检查 | — |
| F100 | 图例手动编辑与位置拖动 | Manual Control of the Graph Legend | P1 | 未开始 | 已有自动图例避让，手动位置拖动和文字编辑入口待做 | 手动操作 | — |
| F101 | 色阶条（Color Scale） | Color Scales | P1 | 未开始 | — | 目视检查 | — |
| F102 | 文字标签（上下标、希腊字母、富文本） | Creating Text Labels | P0 | 未开始 | 迷你标记语言 + cosmic-text 排版 | 目视比较 | — |
| F103 | LaTeX 公式 | Creating Text Labels | P1 | 未开始 | 评估 typst 或 MathJax 转 SVG | 目视比较 | — |
| F104 | 绘图对象（直线、箭头、矩形、圆） | Creating and Editing Drawing Objects | P1 | 未开始 | — | 手动操作 | — |
| F105 | 数据标签 | Adding Labels to Plots | P1 | 已实现待验收 | 数值/X-Y/原始行号/指定列文本，字号/偏移、边缘定位、屏幕/SVG 和撤销均已接入 | 行对齐、文本转义、柱段值、图层与真实开关回归已通过；密集标签与 Origin 目视验收待做 | 标签来源：已完成 0/4，待验收 4，部分实现 0；数值、X/Y、行号、指定列；自动化回归已通过，Origin 对照待验收 |
| F106 | 误差棒 | Adding Error Bars to Your Graph | P0 | 已实现待验收 | 对称 X/Y 误差列、稳定自动关联、端帽/线宽、分组定位与屏幕/SVG 已实现；本条当前范围为对称误差 | 行对齐、对数裁剪、分组柱、往返与撤销回归已通过；Origin 对称误差图目视验收待做 | — |
| F107 | 插入图片 / 表格到图中 | Inserting Image into Graph | P2 | 未开始 | — | 目视检查 | — |
| F108 | 合并多张图 / 拆分图层 | The Merge Graph Dialog Box | P1 | 未开始 | — | 目视检查 | — |
| F109 | 图层排列（行列网格、间距） | The Arrange Layers Dialog Box | P0 | 部分实现 | 已有上下/左右/四面板快捷创建、网格列数、自由矩形与透明叠放；独立网格间距/对齐与轴联动待做 | 布局、命中、透明背景、迁移与撤销回归已通过；间距配置待实现 | — |
| F110 | 布局页（多图拼版） | Using the Layout Page Window | P1 | 未开始 | — | 目视检查 | — |
| F111 | 曲线颜色、线型、符号递增 | Customizing Data Plot Colors | P0 | 已完成 | Nature 风格默认色系（深蓝、砖红、青、紫……）与圆点优先的符号顺序 | 目视比较 | — |
| F112 | 按数据集控制颜色 / 大小（颜色映射） | Using a Dataset to Control Plot Color | P1 | 未开始 | — | 目视比较 | — |
| F113 | 绘图属性对话框（Plot Details） | Plot Details Controls | P0 | 部分实现 | 图层级图型、列映射、轴范围、线宽/符号、误差/标签等属性已接入；逐曲线样式和完整分页待做 | 真实属性控件与撤销回归已通过；完整 Plot Details 范围待补齐 | — |
| F114 | 显示范围（只画部分行） | Editing the Plots Display Range | P1 | 部分实现 | 从工作表选区创建并保存部分行绘图已实现；图形中的独立行范围编辑入口待做 | PlotSpec.rows 截取/保存回归已通过；补齐独立编辑入口后验收 | — |
| F115 | 页面尺寸、背景、边距 | The (Plot Details) Print/Dimensions Tab | P0 | 部分实现 | 页面毫米尺寸、固定逻辑画布、图层透明背景已实现；页面配色与可调边距待做 | 屏幕缩放、SVG 尺寸与项目往返已通过；配色/边距配置待实现 | — |

## 图形探索与小工具

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F116 | 读取坐标 / 数据点提示 | Reading Coordinate Values in the Graph | P0 | 部分实现 | 自绘 plot.rs 已实现悬停坐标、十字光标与图层定位；最近数据点吸附及点信息提示待做 | 坐标逆变换、图层命中与缩放回归已通过；数据点提示待实现 | — |
| F117 | 数据高亮联动（图 ↔ 工作表） | Data Highlighter: Selecting Points in both Graph and Source Worksheet | P1 | 未开始 | — | 手动操作 | — |
| F118 | 屏蔽数据点（Mask） | Masking Data Points in the Graph or Worksheet | P1 | 未开始 | 掩码位图 | 屏蔽后拟合结果与 Origin 比较 | — |
| F119 | 框选 / 删除 / 移动数据点 | Drawing or Moving, or Deleting Data Points | P2 | 未开始 | — | 手动操作 | — |
| F120 | 异常值检测 | Detecting Outliers | P2 | 部分实现 | 统计菜单已有 Grubbs 检验，报告最远点及显著性；图内异常点标记、交互筛选及更多检测方法待做 | Grubbs 数值与界面输出回归通过；图内标记、联动与范围交互待实现 | — |
| F121 | 局部放大视图 | Magnifying or Zooming In on a Portion of the Graph | P2 | 未开始 | — | 手动操作 | — |
| F122 | 快速拟合小工具（框选范围实时拟合） | The Quick Fit Gadget | P1 | 未开始 | 复用拟合引擎 | 与 Origin 比较 | — |
| F123 | 快速寻峰小工具 | The Quick Peaks Gadget | P1 | 未开始 | — | 与 Origin 比较 | — |
| F124 | 积分 / 微分 / 插值小工具 | The Integrate Gadget | P1 | 未开始 | — | 与 Origin 比较 | — |
| F125 | 交点小工具 | The Intersect Gadget | P2 | 未开始 | — | 与 Origin 比较 | — |
| F126 | FFT 小工具 | The FFT Gadget | P2 | 未开始 | — | 与 Origin 比较 | — |
| F127 | 统计小工具 | The Statistics Gadget | P2 | 未开始 | — | 与 Origin 比较 | — |
| F128 | 垂直游标 | The Vertical Cursor Gadget | P2 | 未开始 | — | 手动操作 | — |
| F129 | 曲线数字化（从图片取点） | Digitizer | P1 | 未开始 | image + 坐标标定 | 用已知图片检验 | — |
| F502 | 图形数据切片器 | Graph Exploration / Graph Data Slicer | P2 | 未开始 | A1 数据筛选视图驱动图形和分析 | 筛选、复位、源数据更新与保存 | — |
| F503 | 带浏览面板的图形 | Graph Exploration / Browser Graph | P2 | 未开始 | 图形浏览面板切换数据集与范围 | 数据切换、图形状态和保存 | — |
| F504 | 图像曲线数字化与数据提取 | Gadgets / Digitizer | P2 | 未开始 | 导入图片、坐标标定、点选与曲线采样 | 已知坐标图片的恢复误差和输出 | — |

## 回归与拟合

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F130 | 线性回归（斜率、截距、标准误、R²、残差） | The Linear Regression Dialog Box | P0 | 部分实现 | 已实现斜率/截距、标准误、R²、残差平方和及报告；逐点残差输出/残差图待做 | 教材参考值与拟合写回回归已通过；补齐残差输出后做 NIST/Origin 对照 | — |
| F131 | 多项式回归 | The Polynomial Regression Dialog Box | P0 | 已实现待验收 | X 标准化后的正规方程与高斯–约当求逆；2–6 阶拟合、参数标准误、R²、曲线及撤销已接入 | 解析/仿真回归已通过；NIST Wampler、Origin 及高阶数值稳定性验收待做 | 多项式阶次：已完成 0/5，待验收 5，部分实现 0；2–6 阶；认证数据与稳定性验收待完成 |
| F132 | 多元线性回归 | The Multiple Linear Regression Dialog Box | P1 | 已实现待验收 | 分析 → 多元线性回归：先点选自变量、最后点选因变量；参数表（值、标准误、t、p）与 R²、调整 R²、F 检验、残差标准差 | 与 numpy.linalg.lstsq 及 scipy.stats.t / f 计算的参数、标准误、p 值一致（1e-9），含缺失行与共线提示 | — |
| F133 | 含 X 误差的线性拟合（York 法） | The Linear Fit with X Error Dialog (Pro Only) | P2 | 未开始 | — | 与 Origin 比较 | — |
| F134 | 非线性拟合引擎（Levenberg–Marquardt） | Theory of Nonlinear Curve Fitting | P0 | 部分实现 | Levenberg–Marquardt（中心差分雅可比、自适应阻尼），标准误 (JᵀJ)⁻¹·RSS/(n−p)；目前只用内置函数，参数约束与自定义函数待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值收敛；Origin 对照待做 | — |
| F135 | 参数初值自动估计 | Fitting Functions | P0 | 已实现待验收 | 当前 11 种内置非线性模型均有数据驱动初值估计；支持固定参数并报告未收敛状态 | nlfit.rs 模型估计和拟合回归已通过；退化数据、复杂初值与 Origin 实例验收待做 | 当前模型自动初值：已完成 0/11，待验收 11，部分实现 0；11 为当前内置模型数；完整函数库仍见 F514，退化数据与 Origin 实例待验收 |
| F136 | 参数固定、上下界、线性约束 | Parameters,Bounds,Constraints and Weighting | P0 | 部分实现 | 非线性拟合对话框可把任意参数固定为指定值，固定参数不计入自由度、标准误标为“固定”；上下界与线性约束待做 | 与 SciPy curve_fit 固定参数后的拟合一致（参数 1e-6、标准误 1e-4）；上下界待做 | — |
| F137 | 加权拟合（误差列权重等） | Parameters,Bounds,Constraints and Weighting | P1 | 未开始 | — | 与 Origin 比较 | — |
| F138 | 拟合报告（参数表、ANOVA、置信带、预测带） | Interpreting Regression Results | P0 | 部分实现 | 线性/多项式、非线性和多元回归已有临时报告，含参数/标准误、R²/RSS 等；多元回归还有 t/p 与整体 F 检验。完整拟合 ANOVA、置信/预测带、持久化报告待做 | 现有报告格式、数值及复制回归通过；完整拟合报告与带状输出待验收 | — |
| F139 | 残差图（残差-自变量、正态概率图） | Residual Plot Analysis | P1 | 未开始 | — | 与 Origin 比较 | — |
| F140 | 多峰拟合 | Multiple Curves/Multiple Peaks | P1 | 未开始 | — | 与 Origin 比较 | — |
| F141 | 全局拟合（多组数据共享参数） | Multiple Curves/Multiple Peaks | P1 | 未开始 | — | 与 Origin 比较 | — |
| F142 | 用户自定义拟合函数 | Fitting Functions | P0 | 未开始 | 复用公式解析器，自动求导用数值差分或自动微分 | 与 Origin 比较 | — |
| F143 | 隐函数拟合（圆、椭圆等 5 个） | Nonlinear Implicit Curve Fitting | P2 | 未开始 | 正交距离回归 | 与 Origin 比较 | 隐函数：已完成 0/5，待验收 0，部分实现 0；第三阶段路线参考数量，目录待逐项核对 |
| F144 | 曲面拟合（22 个内置函数） | Non-linear Surface Fitting | P2 | 未开始 | — | 与 Origin 比较 | 曲面函数：已完成 0/22，待验收 0，部分实现 0；第三阶段路线参考数量，目录待逐项核对 |
| F145 | 拟合比较（F 检验、AIC、BIC） | Fit Comparison | P1 | 未开始 | — | 与 Origin 比较 | — |
| F146 | 标准曲线反查 X / Y | Finding Y/X from X/Y Standard Curves | P1 | 部分实现 | 分析 → 标准曲线反查：按点选顺序取标准浓度、标准响应、样品响应，线性标准曲线反查浓度并给出 95% 置信区间半宽（Miller & Miller 公式）；由 X 求 Y、非线性标准曲线待做 | 与 numpy.polyfit 和 scipy.stats.t 计算的反查值、标准差、置信区间一致（1e-9） | — |
| F147 | 快速拟合（菜单一键） | Quick Fit | P0 | 已实现待验收 | 线性拟合一键命令、多项式阶数对话框、报告和结果写回已接入 | 命令、对话框确认/取消、目标图层与撤销回归已通过；完整人工工作流验收待做 | — |
| F148 | 内置拟合函数：峰函数（Gauss、Lorentz、Voigt、PsdVoigt、LogNormal 等） | Curve Fitting Functions（共 239 个） | P0 | 部分实现 | 峰函数：Gauss、Lorentz（Origin 公式与参数名）；Voigt、PsdVoigt、LogNormal 等待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值收敛；Origin 对照待做 | — |
| F149 | 内置拟合函数：指数类（ExpDec1-3、ExpGrow、ExpAssoc 等） | Curve Fitting Functions（共 239 个） | P0 | 部分实现 | 指数类：ExpDec1；ExpDec2/3、ExpGrow、ExpAssoc 等待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值收敛；Origin 对照待做 | — |
| F150 | 内置拟合函数：S 形与生长（Boltzmann、Logistic、DoseResp、Hill、Gompertz 等） | Curve Fitting Functions（共 239 个） | P0 | 部分实现 | S 形：Boltzmann、Logistic；DoseResp、Hill、Gompertz 等待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值收敛；Origin 对照待做 | — |
| F151 | 内置拟合函数：幂函数与对数（Power、Allometric、Log 等） | Curve Fitting Functions（共 239 个） | P0 | 部分实现 | 幂函数：Allometric1；Power、Log 等待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值收敛；Origin 对照待做 | — |
| F152 | 内置拟合函数：多项式与有理函数（Poly、Rational 等） | Curve Fitting Functions（共 239 个） | P0 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 | — |
| F153 | 内置拟合函数：波形（Sine、SineDamp、Pulse、SquareWave 等） | Curve Fitting Functions（共 239 个） | P1 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 | — |
| F154 | 内置拟合函数：酶动力学（MichaelisMenten、各类抑制模型等） | Curve Fitting Functions（共 239 个） | P1 | 部分实现 | 酶动力学：MichaelisMenten（Hanes–Woolf 线性化估计初值）；各类抑制模型待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值（线性化回归）收敛；Origin 对照待做 | — |
| F155 | 内置拟合函数：吸附与结合（Langmuir、Freundlich、BET、OneSiteBind 等） | Curve Fitting Functions（共 239 个） | P1 | 部分实现 | 吸附：Langmuir、Freundlich 等温线，准一级、准二级吸附动力学（线性化估计初值）；BET、OneSiteBind 等待做 | 与 SciPy 1.17.1 curve_fit 的参数（1e-6）、标准误（1e-4）一致，从自动初值（线性化回归）收敛；Origin 对照待做 | — |
| F156 | 内置拟合函数：其余专业函数（流变、热力学、统计分布等） | Curve Fitting Functions（共 239 个） | P2 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 | — |
| F157 | 寻峰分析专用函数（Bigaussian、FraserSuzuki 等 7 个） | Peak Analyzer Functions | P2 | 未开始 | — | 与 Origin 比较 | 寻峰函数：已完成 0/7，待验收 0，部分实现 0；第三阶段路线参考数量，目录待逐项核对 |
| F514 | 内置拟合函数库总览 | Fitting Function Library | P1 | 部分实现 | 当前内置 11 种非线性模型：Gauss、Lorentz、ExpDec1、Boltzmann、Logistic、Allometric1、Langmuir、Freundlich、准一级/准二级动力学、MichaelisMenten；239 项参考目录及其余函数族待补 | 已有模型的解析/SciPy 固定参考值、固定参数及界面写回回归通过；完整 NIST/Origin、函数目录逐项映射和稳定性验收待做 | 显函数参考目录：已完成 0/239，待验收 0，部分实现 0；239 为第三阶段路线参考数；当前 11 模型逐项目录归属未确认，不直接折算为该目录完成数；与 F148–F156 重叠；当前内置模型：已完成 0/11，待验收 11，部分实现 0；当前 Model::ALL 的全部模型，已有模型回归；与 239 项参考目录逐项映射尚待核对，两个口径不相加 |

## 数学运算

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F158 | 插值 / 外推（线性、三次样条、Akima） | Interpolate/Extrapolate | P0 | 已实现待验收 | 分析 → 插值：线性、自然三次样条、Akima，等间距 X 网格、可自定义范围并按端点段外推；结果写成新 X/Y 列并加到图上，一步撤销 | 与 SciPy 1.17.1 interp1d / CubicSpline / Akima1DInterpolator 的区间内与外推值一致（1e-12）；Origin 对照待做 | 插值方法：已完成 0/3，待验收 3，部分实现 0；线性、自然三次样条、Akima；均匀输出网格入口已接入 |
| F159 | 按 X 求 Y 插值 | Interpolate/Extrapolate Y from X | P1 | 部分实现 | interpolate.rs 已可在指定 X 数组计算 Y；桌面插值对话框提供均匀网格及范围。按用户指定 X 列或单点查询的独立入口待做 | 线性/样条/Akima 参考值和外推回归通过；用户 X 列查询入口待实现 | — |
| F160 | 轨迹插值 | Trace Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F161 | XYZ 轨迹插值 | XYZ Trace Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F162 | 二维插值 | 2D Interpolate/Extrapolate | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F163 | 三维插值 | 3D Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F164 | 数值积分（梯形法、曲线下面积） | Integrate | P0 | 已实现待验收 | analysis.rs/ops.rs 已实现梯形积分、累计积分结果与绝对面积选项 | 解析积分、行对齐与结果写回回归已通过；Origin/SciPy 多组参考对照待做 | — |
| F165 | 二维体积积分 | 2D Volume Integrate (Pro Only) | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F166 | 数值微分（含 Savitzky-Golay 平滑求导） | Differentiate | P0 | 部分实现 | 有限差分微分和独立 Savitzky–Golay 平滑已实现；直接 SG 平滑求导入口待做 | 解析函数微分与结果列回归已通过；组合求导功能待补齐 | — |
| F167 | 多条曲线平均 / 合并 | Average/Merge Multiple Curves | P1 | 部分实现 | 已把多曲线线性插值到共同 X 范围求均值/标准差，输出结果表、Y 误差及线图；通用曲线合并工作流待做 | 平均结果、缺失值、结果输出和一步撤销回归通过；通用合并入口与参考软件验收待做 | — |
| F168 | 简单列运算（加减乘除、两曲线相减） | Simple Column/Curve Math | P0 | 已实现待验收 | 分析 → 列运算：两列或列与常数的加减乘除，结果新列与单位推导；不同 X 网格的曲线先线性插值后相减，范围外留空；一步撤销 | 逐行、常数、单位、除零与“原始减拟合”残差回归；Origin 对照待做 | — |
| F169 | 归一化（最大值、Z 分数、区间缩放） | Normalize | P0 | 已实现待验收 | 已接入除以最大值、Z 分数、区间缩放，结果写入派生列并支持撤销 | 算法参考值、写回及撤销回归已通过；Origin/SciPy 多组数值验收待做 | — |
| F170 | 多边形面积 | Polygon Area | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F171 | 反函数（交换 X、Y 求逆） | Inverse | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F172 | 自定义滤波 / 中值滤波 | Median Filter | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 | — |
| F173 | 扣除直线基线 | Subtract Straight Line | P1 | 已实现待验收 | 扣除基线对话框选“两端连线”：首末两点连线作为基线 | 单元测试；Origin 对照待做 | — |
| F509 | 列公式函数库 | Set Column Values / Functions | P0 | 部分实现 | 表达式引擎已有 24 个函数调用名，对应 23 个独立函数（log/log10 同义），首批 120 函数目标、聚合函数及函数目录仍待补齐 | 已有解析和计算回归；当前函数按实现计入待验收，逐函数参考值覆盖与大数据验收待补 | 规划函数（独立功能）：已完成 0/120，待验收 23，部分实现 0；当前 24 个调用名中 log/log10 为同一函数；23 个功能按代码枚举计入待验收，逐函数参考验收待补；120 为 SciCanvas 首批目标 |

## 统计

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F174 | 列统计（均值、标准差、中位数、分位数等） | Statistics on Columns | P0 | 已实现待验收 | 已接入 N、均值、样本标准差、标准误、中位数、线性插值分位数、极值与总和，生成结果工作簿 | 描述统计与结果表回归已通过；Origin/SciPy/R 整组参考值对照待做 | — |
| F175 | 行统计 | Statistics on Rows | P1 | 已实现待验收 | 统计 → 行统计：对点选的各列逐行求均值、标准差、标准误、N，写在最后一列右边；标准差设为 Y 误差列，可直接画误差棒 | 单元测试覆盖缺失值、单值行与列属性 | — |
| F176 | 整表统计 | Statistics on Whole Sheet | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F177 | 频数统计 / 分箱 | Frequency Counts | P0 | 已实现待验收 | 统计 → 频数统计：等宽分箱（Sturges 或指定区间数、可选范围），每列生成区间、频数、累积与相对频率表，一步撤销 | 与 numpy.histogram 的边界与计数一致；Origin 对照待做 | — |
| F178 | 二维频数统计 | 2D Frequency Counts/Binning | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F179 | 正态性检验（Shapiro-Wilk、KS、Anderson-Darling 等） | Normality Test | P1 | 部分实现 | 统计 → 正态性检验：Shapiro–Wilk（Royston AS R94，3 ≤ N ≤ 5000），每个点选列一行；KS、Anderson–Darling 待做 | 与 SciPy 1.17.1 shapiro 一致（W 1e-6，SciPy 内部为单精度；p 1e-4） | 列明的正态性方法：已完成 0/3，待验收 1，部分实现 0；Shapiro–Wilk 已接入；KS、Anderson–Darling 未开始；仅计本条列明方法 |
| F180 | 相关系数（Pearson、Spearman、Kendall） | Correlation Coefficient (Pro Only) | P1 | 已实现待验收 | 统计 → 相关系数：点选列两两计算 Pearson、Spearman（平均秩）、Kendall τ-b（并列校正）及双侧 p 值 | 与 SciPy 1.17.1 pearsonr、spearmanr、kendalltau(method="asymptotic") 一致（1e-9），含并列值与负相关；Origin 对照待做 | — |
| F181 | 偏相关系数 | Partial Correlation Coefficient (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F182 | 分布拟合 | Distribution Fit (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F183 | 列联表与卡方检验 | Cross Tabulation and Chi-square (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F184 | Grubbs / Dixon Q 异常值检验 | Grubbs' Test | P1 | 部分实现 | 统计 → Grubbs 异常值：双侧检验离均值最远的点，给出 G、临界值 G(0.05) 与 p 值；Dixon Q 待做 | 临界值与 p 值与按 scipy.stats.t 计算的 Grubbs 公式一致（1e-9） | 列明的异常值方法：已完成 0/2，待验收 1，部分实现 0；Grubbs 已接入；Dixon Q 未开始 |
| F185 | 单样本 t 检验 | One-Sample T-Test | P0 | 已实现待验收 | 统计 → t 检验（单样本）：自写正则化不完全 Beta 求 t 分布，报告 t、自由度、双侧 p、置信区间 | 与 SciPy 1.17.1 ttest_1samp 一致（1e-9）；Origin 对照待做 | — |
| F186 | 双样本 t 检验（含 Welch） | Two-Sample T-Test | P0 | 已实现待验收 | 统计 → t 检验（双样本）：方差相等的 Student 检验与 Welch 检验，置信区间针对均值差 | 与 SciPy 1.17.1 ttest_ind（equal_var 真/假）一致（1e-9）；Origin 对照待做 | — |
| F187 | 配对 t 检验 | Pair-Sample T-Test | P0 | 已实现待验收 | 统计 → t 检验（配对）：按行配对，只用两列都有数值的行 | 与 SciPy 1.17.1 ttest_rel 一致（1e-9）；Origin 对照待做 | — |
| F188 | 方差检验（单样本、F 检验） | Two Sample Test for Variances (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F189 | 比例检验、泊松率检验 | One-Sample Proportion Test (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F190 | 单因素 / 双因素 / 三因素方差分析（含多重比较） | One, Two, and Three Way ANOVA | P1 | 部分实现 | 统计 → 单因素方差分析：每列一组，ANOVA 表、各组描述统计与 Tukey HSD 多重比较（组大小不等时为 Tukey–Kramer）；双因素、三因素待做 | 与 SciPy 1.17.1 f_oneway、f.sf（1e-9）及 tukey_hsd（置信限 1e-7、p 值 1e-5）、studentized_range 一致；Origin 对照待做 | 列明的 ANOVA 维数：已完成 0/3，待验收 1，部分实现 0；单因素及 Tukey/Tukey–Kramer 已接入；双因素、三因素未开始 |
| F191 | 重复测量方差分析 | Repeated Measure ANOVA (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F192 | 非参数检验（Wilcoxon、Mann-Whitney、Kruskal-Wallis、Friedman 等） | Nonparametric Tests (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F193 | 生存分析（Kaplan-Meier、Cox 回归、Weibull） | Survival Analysis (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F194 | 主成分分析 PCA | Principal Component Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F195 | 偏最小二乘 PLS | Partial Least Squares | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F196 | 聚类分析 | Cluster Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F197 | 判别分析 | Discriminant Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F198 | 机器学习（分类、回归） | Machine Learning (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F199 | 功效与样本量计算 | Power and Sample Size (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F200 | ROC 曲线 | ROC Curve (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） | — |
| F505 | 图形摘要报告 | Statistics / Graphical Summary | P1 | 未开始 | 描述统计与分布图组合报告 | 参考统计值与图形、报告往返 | — |
| F506 | 并排统计比较 | Statistics / Side-by-Side Statistics | P1 | 未开始 | 多组数据统计与并列结果 | SciPy/R 数值对照与缺失值处理 | — |
| F507 | 双样本 Kolmogorov–Smirnov 检验 | Statistics / Two-Sample KS Test | P1 | 未开始 | A8 双样本统计量与 p 值 | SciPy/R 对照、重复值和小样本 | — |
| F508 | 滑动窗口非参数检验 | Statistics / Running Nonparametric Tests | P1 | 未开始 | A8 窗口范围、方法选择和结果列 | 窗口边界、缺失值、SciPy/R 对照 | — |

## 信号处理

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F201 | 平滑（移动平均、Savitzky-Golay、FFT 平滑、LOWESS） | Smoothing | P0 | 部分实现 | 移动平均与二阶 Savitzky–Golay 已接入；FFT 平滑与 LOWESS 待做 | 已有算法、窗口边界、写回与撤销回归已通过；补齐其余方法后验收 | 平滑方法：已完成 0/4，待验收 2，部分实现 0；移动平均、Savitzky–Golay 已实现待验收；FFT、LOWESS 未开始 |
| F202 | FFT 滤波（低通、高通、带通、带阻） | FFT Filters | P1 | 已实现待验收 | 分析 → 信号处理 → FFT 滤波：低通、高通、带通、带阻理想滤波（频域置零后逆变换），结果写成新列，来源是图形时画进图里 | 与 numpy rfft / irfft 掩模滤波逐点一致（1e-12）；带通 + 带阻 = 原信号 | 频域滤波类型：已完成 0/4，待验收 4，部分实现 0；低通、高通、带通、带阻理想滤波；不代表 IIR 或其他滤波器实现 |
| F203 | IIR 滤波（巴特沃斯、切比雪夫等） | IIR Filters (Pro Only) | P2 | 未开始 | 自写或 biquad | 与 Origin 及 SciPy.signal 结果比较 | — |
| F204 | 傅里叶变换 FFT / IFFT（幅值、相位、功率谱、窗函数） | Fast Fourier Transform (FFT) | P0 | 部分实现 | 任意点数 FFT（基 2/Bluestein）、矩形/Hann/Hamming 窗、单边幅值/相位/功率谱及幅值图已接入；滤波内部已有逆变换，独立 IFFT 命令与复数输入输出待做 | 与 numpy.fft.rfft 的实部 / 虚部（基 2 与 Bluestein 两种长度）及 numpy 窗函数结果一致（1e-9）；IFFT 与 Origin 对照待做 | — |
| F205 | 短时傅里叶变换 STFT | Short-Time Fourier Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F206 | 卷积 / 反卷积 | Convolution | P1 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F207 | 相关（自相关、互相关） | Correlation | P1 | 已实现待验收 | 分析 → 信号处理 → 相关函数：选一列求自相关、两列求互相关（归一化，滞后以点计），结果表与线图 | 与 numpy.correlate 归一化结果一致（1e-12） | — |
| F208 | 相干性 | Coherence (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F209 | 希尔伯特变换、包络 | Hilbert Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F210 | 二维 FFT 与二维 FFT 滤波 | 2D Fourier Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 | — |
| F211 | 小波变换（连续、分解、重构、去噪） | Wavelet Transforms (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F212 | 二维小波 | 2D Wavelet Transforms | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F213 | 二维相关 | 2D Correlation (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F214 | 矩阵平滑 | Matrix Smoothing | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |
| F215 | 抽取 / 降采样 | Decimation (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 | — |

## 峰分析

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F216 | 基线：锚点法、自动基线（ALS、多项式等） | Peak Analyzer | P1 | 部分实现 | 分析 → 信号处理 → 扣除基线：两端连线与 ALS 自动基线（非对称最小二乘，五对角 LDLᵀ 求解）；锚点法、多项式基线待做 | ALS 与 numpy 稠密求解结果一致（1e-9） | — |
| F217 | 扣除基线 | Peak Analyzer | P1 | 已实现待验收 | 每条曲线在 Y 右边写入“基线”和“扣除基线”两列；来源是图形时基线画进图里，一步撤销 | 单元测试覆盖缺失行、列位置与图层曲线 | — |
| F218 | 寻峰（局部极值、导数法、阈值） | Peak Analyzer | P1 | 部分实现 | 分析 → 信号处理 → 寻峰：局部极大值（平顶取中点）+ 最小突出度阈值（占 Y 范围百分比），算法同 scipy.signal.find_peaks；导数法待做 | 单元测试与 SciPy 1.17.1 find_peaks / peak_prominences 逐值一致（含平顶峰） | — |
| F219 | 峰积分（面积、半峰宽、质心） | Peak Analyzer | P1 | 已实现待验收 | 峰表给出峰高、突出度、半峰宽（半突出度处，同 peak_widths）、线性基线以上的面积与质心；积分区间到相邻峰之间的最低点 | 单元测试与 SciPy peak_widths、numpy.trapezoid 一致；高斯峰面积与 A·s·√π 吻合 | — |
| F220 | 多峰拟合（重叠峰分峰） | Peak Analyzer | P1 | 未开始 | 复用拟合引擎 | 与 Origin Peak Analyzer 结果比较 | — |

## 图像处理

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F221 | 导入图片到矩阵 | Importing Images and Videos | P2 | 未开始 | image | 手动操作 | — |
| F222 | 图像调整（亮度、对比度、伽马） | Image Adjustment | P2 | 未开始 | image / imageproc | 与 Origin 比较 | — |
| F223 | 几何变换（旋转、裁剪、缩放） | Geometric Transformations | P2 | 未开始 | imageproc | 与 Origin 比较 | — |
| F224 | 空间滤波（边缘检测、锐化等） | Spatial Filters | P2 | 未开始 | imageproc | 与 Origin 比较 | — |
| F225 | 图像分析（粒子计数、剖面） | Image Analysis | P2 | 未开始 | imageproc | 与 Origin 比较 | — |

## 导出与发布

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F226 | 导出 SVG | Exporting Your Graph or Layout Page to an Image File | P0 | 已完成 | svg.rs | 目视比较 Origin 导出图 | — |
| F227 | 导出 PNG（指定 DPI） | The Export Graph as Image Dialog Box | P0 | 已实现待验收 | 图形 → 导出 PNG：SVG 版面经 resvg 栅格化，可选 150/300/600/1200 dpi 或自定义；写入 pHYs 物理分辨率，像素尺寸按页面毫米换算；中文回退到系统字体 | 自动化回归核对像素尺寸、pHYs、可解码和绘制内容；插入 Word 的实际尺寸待人工验收 | — |
| F228 | 导出 PDF（矢量，嵌入字体） | Which Export Format Should I Choose | P0 | 未开始 | svg2pdf / krilla | 用 PDF 阅读器检查 | — |
| F229 | 导出 EPS | Which Export Format Should I Choose | P1 | 未开始 | 自写 PostScript 输出 | LaTeX 编译检查 | — |
| F230 | 导出 TIFF（期刊要求） | Which Export Format Should I Choose | P1 | 未开始 | image | 尺寸与 DPI 检查 | — |
| F231 | 复制图形到剪贴板（粘贴到 Word / PPT） | Pasting or Embedding Graphs in Other Applications | P0 | 已实现待验收 | 复制图形（Ctrl+J）：按 300 dpi 栅格化后经 arboard 写入剪贴板位图；矢量 EMF 待评估 | 与 PNG 共用栅格化回归；粘贴到 Word / PPT 待人工验收 | — |
| F232 | 导出 EMF（Office 可编辑矢量） | Which Export Format Should I Choose | P1 | 未开始 | 自写 EMF 写出器 | 插入 Word 检查 | — |
| F233 | 批量导出多张图 | The Export Graph Dialog Box | P1 | 未开始 | — | 手动操作 | — |
| F234 | 打印 | Printing Graphs, Layout Pages, and Analytical Results | P2 | 未开始 | 导出 PDF 后交给系统打印 | 手动操作 | — |
| F235 | 图形动画（导出视频 / GIF） | Creating Graph Animation | P2 | 未开始 | gif / ffmpeg | 手动操作 | — |

## 自动化与编程

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F236 | 内置 Python 运行时（预装 numpy、scipy、pandas） | Python Console | P1 | 未开始 | Windows 用官方嵌入版，macOS / Linux 用 python-build-standalone，随安装包分发；脚本在独立进程中运行，崩溃不影响软件 | 在没装 Python 的干净系统上安装后运行全部示例脚本 | — |
| F237 | scicanvas Python 包（连接运行中的软件；离线模式直接调用核心库） | Python Packages | P1 | 未开始 | 本机控制通道（带令牌）+ Arrow 批量传数据；离线模式用 PyO3 编译核心库 | 脚本示例回归测试，与界面操作结果比较 | — |
| F238 | 脚本窗口（编辑器、运行 / 停止、输出面板、报错行高亮） | The Origin Command Window and Script Window | P1 | 未开始 | egui 代码编辑面板，调用内置 python.exe 子进程，实时显示输出 | 运行、中途停止、报错定位到行 | — |
| F239 | 操作录制（把界面操作生成 Python 脚本，可回放） | Batch Processing | P1 | 未开始 | 命令注册表记录命令序列，翻译成 scicanvas Python 调用 | 录制 → 回放，结果与手动操作一致 | — |
| F240 | Python 包管理（装包到内置环境，可切换到系统 Python / conda） | Python Packages | P1 | 未开始 | 调用内置 pip；设置里选择解释器，脚本窗口标题显示当前环境 | 安装、卸载、切换环境后运行示例 | — |
| F241 | 命令行批处理（无界面） | Batch Processing | P1 | 未开始 | scicanvas-cli，调用核心库 | 脚本示例回归测试 | — |
| F242 | 分析模板（换数据自动重算） | Analysis Templates | P1 | 未开始 | 分析操作记录为命令序列 | 与 Origin 比较 | — |
| F243 | MCP 服务器（让 AI 代理驱动软件） | — | P1 | 未开始 | 参照 PhotoCraft：命令注册表暴露为 MCP 工具 | 代理完成示例任务 | — |
| F244 | 插件系统 | Apps | P2 | 未开始 | WebAssembly 插件（wasmtime）沙盒运行 | 示例插件 | — |
| F245 | R 控制台 | R Console and Rserve Console | P2 | 未开始 | — | — | — |
| F246 | MATLAB 控制台 | MATLAB Console | 不做 | 不做 | — | — | — |
| F247 | LabTalk 脚本兼容 | LabTalk Programming（1,018 页） | 不做 | 不做 | 改用 Python | — | — |
| F248 | Origin C 兼容 | Origin C（3,862 页） | 不做 | 不做 | 改用 Python / Rust 插件 | — | — |
| F249 | X-Function 兼容（约 680 个） | X-Function Reference | 不做 | 不做 | 功能按需在核心库实现，不做接口兼容 | — | — |
| F250 | COM 自动化服务器 | Automation Server | 不做 | 不做 | — | — | — |
| F251 | LabVIEW 虚拟仪器 | Using Origin from LabVIEW | 不做 | 不做 | — | — | — |
| F252 | Mathematica Link | Mathematica Link | 不做 | 不做 | — | — | — |
| F253 | Code Builder 集成开发环境 | The Code Builder | 不做 | 不做 | — | — | — |

## 图类型 · 2D 线图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F254 | 折线图 | Line Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F255 | 水平阶梯图 | Horizontal Step Graph | P1 | 已实现待验收 | 水平阶梯：先横后竖，共享折线图元与 SVG | 拐点几何、拟合线、保存与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F256 | 垂直阶梯图 | Vertical Step Graph | P1 | 已实现待验收 | 垂直阶梯：先竖后横，共享折线图元与 SVG | 拐点几何、拟合线、保存与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F257 | Y 偏移堆叠线图 | Stacked Lines by Y Offsets Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F258 | 棒棒糖图 | Lollipop Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F259 | 颜色映射线系列 | Colormapped Line Series | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F260 | 样条曲线图 | Spline Graph | P1 | 已实现待验收 | 自然三次样条、缺失/重复 X 分段、自动范围与共享折线图元 | 解析值和 SciPy 1.17.1 自然边界对照、分段及 SVG 回归已通过；Origin 同数据目视验收待做 | — |

## 图类型 · 2D 散点图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F261 | 散点图 | 2D Scatter Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F262 | 中心坐标散点图 | 2D Scatter Central Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F263 | 列散点图 | Column Scatter | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F264 | Y 误差棒图 | Y Error Bar Graph | P0 | 已实现待验收 | Y 对称误差列自动关联、端帽/线宽、裁剪与共享线段图元 | 端点几何、无效值、保存与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F265 | XY 误差棒图 | XY Error Bar Graph | P0 | 已实现待验收 | 对称 X/Y 误差列自动关联、方向转换、端帽/线宽与共享图元 | 关联、端点几何、裁剪及撤销回归已通过；Origin 同数据目视验收待做 | — |
| F266 | 散点 + 地毯图 | Scatter+Rug Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F267 | 垂线图 | Vertical Drop Line | P1 | 已实现待验收 | 原始点到零基线的垂线，对数轴使用显示下界；符号/线宽与 SVG 共用 | 正负值、对数基线、SVG 线段和撤销回归已通过；Origin 同数据目视验收待做 | — |
| F268 | 气泡图 | Indexed Size Bubble Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F269 | 颜色映射散点图 | Color Mapped Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F270 | 气泡 + 颜色映射图 | Indexed Size (Bubble) and Color Map Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F271 | 误差带图 | Error Band Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F272 | 密度点图 | Density Dots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F273 | 彩色点图 | Color Dots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 2D 点线图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F274 | 点线图 | Line+Symbol Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F275 | 线系列图 | Line Series Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F276 | 两点线段图 | 2 Point Segment Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F277 | 三点线段图 | 3 Point Segment Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F278 | 样条连接点线图 | Spline Connected Graph | P1 | 已实现待验收 | 自然三次样条连接，符号与误差保持原始点；派生线独立绘制 | SciPy 数值、符号数、分段、自动范围及撤销回归已通过；Origin 同数据目视验收待做 | — |
| F279 | 按行绘图 | Row Wise Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F280 | 前后对比图 | Before and After Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 2D 柱状图与条形图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F281 | 柱状图 | Column Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F282 | 带标签柱状图 | Column + Label Graph | P1 | 已实现待验收 | 柱图配合数值/X-Y/行号/指定列数据标签，字号与偏移可调 | 标签行对齐、段值、边缘定位、SVG 与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F283 | 条形图 | Bar Graph | P0 | 已实现待验收 | 数值型条形图、并排分组、误差换向、物理轴范围与共享矩形 | 轴方向、正负值、分组误差、SVG 与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F284 | 堆叠柱状图 | Stacked Column Graph | P1 | 已实现待验收 | 按原始行与 X 对齐，正负柱段分别累加，共享矩形与 SVG | 缺失行、正负值、段几何、误差与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F285 | 浮动条形图 | Floating Bar Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F286 | 堆叠条形图 | Stacked Bar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F287 | 百分比堆叠柱状图 | 100% Stacked Column Graph | P1 | 已实现待验收 | 按行正负组分别归一至 ±100%，零总量与极端尺度处理，共享 SVG | 归一化参考值、极端数值、误差缩放及撤销回归已通过；Origin 同数据目视验收待做 | — |
| F288 | 百分比堆叠条形图 | 100% Stacked Bar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F289 | 浮动柱状图 | Floating Column Graph | P2 | 已实现待验收 | 上下端列稳定配对、完整行过滤、反向区间、多组排列与共享 SVG | 配对、删列/复用、矩形、对数裁剪与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F290 | 百分比丝带图 | 100% Ribbon Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F291 | 丝带图 | Ribbon Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F292 | 圆堆积图 | Circular Packing Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F293 | 螺旋条形图 | Spiral Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F294 | 频数图 | Frequency Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 饼图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F295 | 2D 饼图 | 2D Color Pie Chart | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F296 | 3D 饼图 | 3D Color Pie Chart | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F297 | 环形图 | Doughnut Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F298 | 复合饼图 | Pie Of Pie Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F299 | 复合条饼图 | Bar Of Pie Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F300 | 复合环形图 | Doughnut Of Pie | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F301 | 百分比环形图 | Doughnut with Total Wedges = 100 | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F302 | 不等半径饼图 | Pie with Different Radius Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F303 | 不等半径环形图 | Doughnut with Different Radius Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F304 | 旭日图 | Sunburst Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 多轴图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F305 | 双 Y 轴图 | Double Y Graph | P0 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F306 | 双 Y 轴柱状图 | Double-Y Column Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F307 | 双 Y 轴柱状 + 点线图 | Double-Y Column Line+Symbol Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F308 | 双 X 轴独立图 | Double X - Independent XY Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F309 | 双 X 轴公式换算图 | Double X - Custom Formula Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F310 | 共享 Y 的双 X 轴图 | Y-Shared Double X Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F311 | 阿伦尼乌斯双 X 轴图 | Double X - Arrhenius Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F312 | 2Ys Y-Y 图 | 2Ys Y-Y Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F313 | 2Ys 柱状图 | 2Ys Column | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F314 | 2Ys 柱状-点线图 | 2Ys Column-Line Symbol | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F315 | 3Ys Y-YY 图 | 3Ys Y-YY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F316 | 3Ys Y-Y-Y 图 | 3Ys Y-Y-Y | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F317 | 4Ys Y-YYY 图 | 4Ys Y-YYY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F318 | 4Ys YY-YY 图 | 4Ys YY-YY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F319 | 多 Y 轴图 | Multiple Y Axes Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F320 | 放大图 | Zoom Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 多面板图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F321 | 上下两面板图 | Vertical 2 Panel Graph | P0 | 已实现待验收 | 绘图 → 多面板 → 上下两面板：Y 列按顺序分到两个图层，单列网格，页面 210 × 280 mm（A4 纸宽） | 核心库分配与网格排版回归；面板字母标注、坐标轴联动与 Origin 目视对照待做 | — |
| F322 | 左右两面板图 | Horizontal 2 Panel Graph | P0 | 已实现待验收 | 绘图 → 多面板 → 左右两面板：Y 列按顺序分到两个图层，两列网格，页面 210 × 105 mm（A4 纸宽） | 核心库分配与网格排版回归；面板字母标注、坐标轴联动与 Origin 目视对照待做 | — |
| F323 | 四面板图 | 4 Panel Graph | P0 | 已实现待验收 | 绘图 → 多面板 → 四面板：Y 列按顺序分到 2 × 2 网格的四个图层，页面 210 × 157.5 mm（A4 纸宽） | 核心库分配与网格排版回归；面板字母标注、坐标轴联动与 Origin 目视对照待做 | — |
| F324 | 九面板图 | 9 Panel Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F325 | 堆叠面板图 | Stack Panels Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F326 | 多面板图 | Multiple Panels Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F327 | 插图（图中图） | Inset Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F328 | 网格分面图 | Trellis | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F329 | 双 Y 网格分面图 | Trellis Double Y | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F330 | 簇图 | Cluster Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 瀑布图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F331 | 2D 瀑布图 | 2D Waterfall Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F332 | 2D 瀑布图（Y 颜色映射） | 2D Waterfall Y Color Mapping | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F333 | 2D 瀑布图（Z 颜色映射） | 2D Waterfall Z Color Mapping | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F334 | 3D 瀑布图 | 3D Waterfall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F335 | 3D 瀑布图（Y 颜色映射） | 3D Waterfall Y:Color Mapping | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F336 | 3D 瀑布图（Z 颜色映射） | 3D Waterfall Z:Color Mapping | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 统计图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F337 | 二维核密度图 | 2D Kernel Density Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F338 | 均值条形图 | Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F339 | 蜂群图 | Beeswarm Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F340 | Bland-Altman 图 | Bland-Altman Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F341 | 箱线图 | Box Chart Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F342 | 双 Y 箱线图 | 2Ys Box Chart Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F343 | 直方图 / 分布图 | Histogram/Distribution Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F344 | 概率直方图 | Histogram Probabilities Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F345 | 直方图 + 地毯图 | Histogram + Rug/Distribution + Rug Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F346 | 区间图 | Interval Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F347 | 多面板直方图 | Multi-Panel Histogram Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F348 | 平行索引图 | Parallel Index Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F349 | 平行坐标图 | Parallel Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F350 | 平行集图 | Parallel Sets Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F351 | 帕累托图（分箱数据） | Pareto Chart - Binned Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F352 | 帕累托图（原始数据） | Pareto Chart - Raw Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F353 | 人口金字塔图（龙卷风图） | Population Pyramid Graph(Tornado Chart) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F354 | 概率图 | Probability Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F355 | Q-Q 图 | Q-Q Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F356 | 质量控制图（X̄-R） | QC(x bar R) Chart Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F357 | 点图 | Dot Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F358 | 堆叠点图 | Stacked Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F359 | 散点矩阵图 | Scatter Matrix Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F360 | 堆叠直方图 | Stacked Histograms Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F361 | 多变异图 | Multi-Vari Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F362 | 变异性图 | Variability Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F363 | 小提琴图 | Violin Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F364 | 桑基图 | Sankey Diagrams | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F365 | 冲积图 | Alluvial Diagrams | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F366 | 网络图 | Network Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F367 | 和弦图 | Chord Diagram/Chord Diagram with Ratio Layout | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F368 | 层次边捆绑图 | Hierarchical Edge Bundling Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F369 | 边际箱线图 | Marginal Box Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F370 | 边际直方图 | Marginal Histograms Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F371 | 边际图 | Marginal Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F372 | 山脊图 | Ridgeline Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F373 | 对称图 | Symmetry Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F374 | 运行图 | Run Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F375 | 累积分布图 | CDF Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F376 | 交互作用图 | Interaction Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F377 | 主效应图 | Main Effects Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F378 | 因果图（鱼骨图） | Cause-and-Effect Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 分组图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F379 | 分组散点图 | Grouped Scatter | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F380 | 分组柱状图 | Grouped Columns | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F381 | 分组直方图 | Grouped Histogram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F382 | 分组箱线图（索引数据） | Grouped Box Charts-Indexed Data | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F383 | 分组箱线图（原始数据） | Grouped Box Charts-Raw Data | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F384 | 分组列散点图 | Grouped Column Scatter-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F385 | 分组区间图 | Grouped Interval Plot-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F386 | 分组均值条形图 | Grouped Mean Bar-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F387 | 分组点图 | Grouped Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F388 | 分组堆叠点图 | Grouped Stacked Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F389 | 分组小提琴图 | Grouped Violin-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F390 | 分组半小提琴图 | Grouped Half Violin-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F391 | 分组半箱线图 | Grouped Half Box-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F392 | 分组浮动条形图 | Grouped Floating Bar | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 地图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F393 | 条形地图 | Bar Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F394 | 饼图地图 | Pie Map Chart | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F395 | 方格地图 | Tile Grid Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F396 | 桑基地图 | Sankey Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F397 | Stiff 地图 | Stiff Map Diagram | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |
| F398 | 矩形树图 | Treemap | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 面积图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F399 | 面积图 | Area Graph | P1 | 已实现待验收 | 按连续有效行分段、跨零三角剖分、透明度、边界线与共享 SVG | 面积积分、缺口、对数裁剪、网格缩放与撤销回归已通过；Origin 同数据目视验收待做 | — |
| F400 | 堆叠面积图 | Stacked Area Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F401 | 百分比堆叠面积图 | 100% Stacked Area Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F402 | 填充面积图 | Fill Area Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F403 | 双色填充图 | Fill Two Colors Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F404 | 风筝图 | Kite Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 专业图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F405 | 窗口浏览图 | Window-view Browser Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F406 | 子弹图 | Bullet Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F407 | 极坐标等高线 + 梯度矢量 | Polar Contour + Gradient Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F408 | 等高线 + 梯度矢量 | Contour Line + Gradient Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F409 | 等高线 + 流线 | Contour + Streamline | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F410 | Piper 三线图 | Piper/Trilinear Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F411 | Durov 图 | Durov Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F412 | Stiff 图 | Stiff Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F413 | 极坐标图 | Polar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F414 | 极坐标矢量图（θrθr） | Polar Vector theta r theta r graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F415 | 罗盘图 | Compass Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F416 | 极坐标矢量图（θrAM） | Polar Vector ?RAM graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F417 | 雷达图 | Radar Chart (Spider Plot) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F418 | 径向条形图 | Radial Bar and Radial Stacked Bar Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F419 | 直角三元图 | Right-Triangle Ternary | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F420 | 史密斯圆图 | Smith Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F421 | 堆叠径向图 | Stacked Radial Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F422 | 流线图 | Streamline Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F423 | 三元图 | Ternary | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F424 | 三元矢量图 | Ternary Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F425 | 风玫瑰图（分箱数据） | Wind Rose-Binned Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F426 | 风玫瑰图（原始数据） | Wind Rose-Raw Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F427 | 矢量图（XY + 角度 + 大小） | X, Y, Angle, Magnitude Vector Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F428 | 矢量图（起点 XY + 终点 XY） | X, Y, X, Y Vector Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 金融图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F429 | 桥图（瀑布式） | Bridge Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F430 | 高低收图 | High-Low-Close Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F431 | K 线图 | Japanese_Candlestick | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F432 | OHLC 条形图 | OHLC Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F433 | OHLC + 成交量图 | OHLC-Volume | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |
| F434 | 股票折线图 | Line (Stock) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 3D XYY 图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F435 | XYY 3D 柱状图 | XYY 3D Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F436 | XYY 3D 百分比堆叠柱 | XYY 3D 100% Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F437 | XYY 3D 并列柱 | XYY 3D Side by Side Bars | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F438 | XYY 3D 堆叠柱 | XYY 3D Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F439 | XYY 3D 柱（带误差棒） | XYY 3D Bar Graph with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F440 | 3D 丝带图 | 3D Ribbons Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F441 | 3D 百分比堆叠墙图 | 3D 100% Stacked Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F442 | 3D 堆叠墙图 | 3D Stacked Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F443 | 3D 墙图 | 3D Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 3D 曲面图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F444 | 填色曲面图 | Color Fill Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F445 | X 恒定带底曲面 | X Constant with Base Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F446 | Y 恒定带底曲面 | Y Constant with Base Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F447 | 颜色映射曲面图 | Color Map Surface | P1 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F448 | 颜色映射曲面（带投影） | Color Map Surface with Projection | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F449 | 3D 三元颜色映射曲面 | 3D Ternary Colormap Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F450 | 线框图 | Wire Frame | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F451 | 线框曲面 | Wire Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F452 | 填色曲面（带误差棒） | Color Fill Surface with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F453 | 颜色映射曲面（带误差棒） | Colormap Surface with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F454 | 多个填色曲面 | Multiple Color Fill Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F455 | 多个颜色映射曲面 | Multiple Color Map Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F456 | 参数曲面 | Parametric Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F457 | 3D 堆叠曲面 | 3D Stacked Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F458 | 3D 堆叠热图 | 3D Stacked Heatmaps | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 3D 散点 / 柱 / 矢量图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F459 | 3D 散点图 | 3D Scatter Graph | P1 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F460 | 3D 散点（Z 误差棒） | 3D Scatter with Z Error Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F461 | 3D 线图 | 3D Line Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F462 | 轨迹图 | Trajectory Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F463 | 3D 柱状图 | 3D Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F464 | 3D 柱状图（分类 XY） | 3D Bar Graph with Categorical/Text XY | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F465 | 3D 堆叠柱状图 | 3D Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F466 | 3D 百分比堆叠柱状图 | 3D 100% Stacked Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F467 | 3D 浮动柱状图 | 3D Floating Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F468 | 3D 矢量（XYZ XYZ） | 3D Vector XYZ XYZ | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F469 | 3D 矢量（XYZ dXdYdZ） | 3D Vector XYZ dXdYdZ | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F470 | 3D 三元散点 | 3D Ternary Symbol | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |
| F471 | 四面体图（四元相图） | 3D Tetrahedral(Quaternary Phase Diagram) | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 等高线与热图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F472 | 填色等高线图 | Color Fill Contour | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F473 | 黑白等高线图（带标签） | Black and White Lines with Labels Contour | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F474 | 灰度等高线图 | Gray Scale Map Contour | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F475 | 分类 Z 等高线图 | Contour - Categorical Z | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F476 | 热图 | Heatmap | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F477 | 极坐标热图 | Polar Heatmap | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F478 | 带标签热图 | Heatmap with Labels | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F479 | 分割热图 | Split Heatmap | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F480 | 分块热图 | Heatmap with Split Tiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F481 | 极坐标等高线（XYZ 数据） | Polar Contour from XYZ Data | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F482 | 极坐标等高线（矩阵数据） | Polar Contour from Matrix Data | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F483 | 三元等高线图 | Ternary Contour | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F484 | 叠加等高线图 | Overlaid Contour Plot | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |

## 图类型 · 图像与剖面

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F485 | 等高线剖面图 | Contour Profiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F486 | 图片背景 | Image as Background | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F487 | 图像图 | Image Plot | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |
| F488 | 图像剖面图 | Image Profiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 | — |

## 分析工作流

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F489 | 分析报告表与重算（自动 / 手动 / 不重算） | Recalculating Analysis Results | P0 | 部分实现 | 已有独立分析结果工作簿与临时报表；统一持久化分析报告表、Operation 依赖记录及 Auto/Manual/None 三种重算模式待做 | 现有结果表/报告与历史回归通过；保存操作依赖、修改源数据触发重算尚待实现 | — |
| F490 | 分析主题与对话框设置保存 | Analysis Themes | P1 | 未开始 | A5 保存分析参数主题，按操作恢复设置 | 跨会话恢复参数并验证默认值 | — |
| F491 | 分析模板批处理多个数据集 | Batch Processing | P1 | 未开始 | A5 模板输入映射、批量执行与错误报告 | 多文件运行、部分失败、结果关联 | — |

## 作图核心

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 | 子项进度 |
|---|---|---|---|---|---|---|---|
| F492 | 参考线（水平 / 垂直） | Graphing / Axes / Reference Lines | P0 | 未开始 | A2/A7 按轴值定位，属性与导出共用场景 | 轴变化、对数轴、SVG 和撤销 | — |
| F493 | 坐标轴断点 | Graphing / Axes / Axis Break | P1 | 未开始 | A2 分段坐标映射、断点标记与拾取逆变换 | 断点两侧几何、导出与交互 | — |
| F494 | 附着坐标轴的对象 | Graphing / Axes / Objects | P1 | 未开始 | A7 对象锚点引用轴与数值位置 | 缩放后定位、保存和撤销 | — |
| F495 | 同一图层每条曲线独立图类型（柱 / 线 / 散点混合） | Plot Details / Plot Type | P0 | 未开始 | A2 Layer 包含多个 Plot，每条曲线单独指定类型 | 同层柱线散点混合、删除与撤销、SVG | — |
| F496 | 曲线填充图案 | Plot Details / Pattern | P1 | 未开始 | A3 共用填充图元，区分实色与图案 | 屏幕与 SVG 图案一致、保存和撤销 | — |
| F497 | 符号边框与内部填充独立设置 | Plot Details / Symbol | P0 | 未开始 | A2/A3 每条曲线边框颜色、宽度和填充 | 空心/实心符号、导出与撤销 | — |
| F498 | 曲线连接方式（直线 / 样条 / B 样条 / 阶梯 / Bezier） | Plot Details / Line / Connect | P0 | 部分实现 | 已有直线、自然样条、水平/垂直阶梯；B 样条与 Bezier 待实现 | 现有连接方式已有几何和 SVG 回归；补齐其余方式后对照验收 | 连接方式：已完成 0/6，待验收 4，部分实现 0；直线、自然样条、水平/垂直阶梯待验收；B 样条、Bezier 未开始 |
| F499 | 按列映射曲线或符号颜色 | Plot Details / Modifiers / Color | P0 | 未开始 | A2 按数据列解析逐点颜色，兼容图例 | 缺失值、色阶、SVG 和撤销 | — |
| F500 | 按列映射符号大小 | Plot Details / Modifiers / Size | P0 | 未开始 | A2 按数据列映射逐点尺寸与范围 | 无效值、极端值、导出和撤销 | — |
| F501 | 按列映射符号形状 | Plot Details / Modifiers / Shape | P0 | 未开始 | A2 按分类列映射符号形状 | 类别排序、图例、导出和撤销 | — |
