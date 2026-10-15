# SciCanvas 功能对照清单

条目按 Origin 帮助文档的目录整理，“Origin 参考”一列是对应的帮助章节标题，方便查阅原文档。
本清单只记录功能点，不包含 Origin 文档正文。实现时按公开说明和观察到的行为编写，不复制 Origin 的代码、文字或图片。

共 488 项：P0 87 项，P1 136 项，P2 254 项，不做 11 项。
状态：已完成 26 项，原型 31 项，未开始 420 项，不做 11 项。

- **P0 / M1 日常够用**：一个人做日常数据处理和论文作图需要的最小集合。
- **P1 / M2 论文全流程**：拟合报告、统计检验、多面板、信号处理等，覆盖大部分论文工作。
- **P2 / M3 高级功能**：3D、专业图、Pro 版统计、图像处理等。
- **不做**：脚本兼容（LabTalk、Origin C、X-Function）、Origin 私有工程格式、与特定外部软件的连接。
- **状态“原型”**：网页原型已实现，Rust 版待移植或部分完成。

验证原则：分析结果与 Origin 及 NIST StRD 认证数据集、SciPy / R 交叉比对；图形与 Origin 同数据出图目视比较。

安装包体积估算：全部功能完成后约 100–130 MB。其中软件本体（含 3D、PDF / Excel、图像处理、模板与帮助）压缩后约 20–40 MB，内置 Python 运行时及 numpy、scipy、pandas 压缩后约 60–90 MB。


## 工作区与项目

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F001 | 多文档子窗口（拖动、缩放、最小化、最大化） | Basic Child Window Operations | P0 | 已完成 | egui Window | 手动操作 |
| F002 | 项目管理器（文件夹树、显示 / 隐藏窗口） | Project Explorer | P0 | 已完成 | egui SidePanel；文件夹层级待做 | 手动操作 |
| F003 | 项目管理器多级文件夹、拖动整理 | Using Project Explorer to Manage Child Windows | P1 | 未开始 | 树形数据模型 + 拖放 | 手动操作 |
| F004 | 窗口排列：层叠 / 平铺 | Arrange Windows | P1 | 原型 | 按工作区矩形计算位置 | 手动操作 |
| F005 | 项目文件保存 / 打开 | Opening, Closing and Backing Up the Origin Project File | P0 | 已完成 | serde_json（.scproj）；以后换成带二进制块的压缩格式 | 保存→打开→逐字段比较 |
| F006 | 自动备份与崩溃恢复 | Opening, Closing and Backing Up the Origin Project File | P1 | 未开始 | 定时写临时文件 | 强制退出后恢复 |
| F007 | 最近打开的文件 | Recent Origin Files | P1 | 未开始 | 配置文件记录 | 手动操作 |
| F008 | 撤销 / 重做 | — | P0 | 已完成 | 显式修改标记与编辑事务；100 步 / 300 MiB 上限 | 十万行事务复用、空闲帧、真实编辑控件回归测试 |
| F009 | 命令注册表（菜单、工具栏、脚本共用同一套命令） | — | P0 | 原型 | 参照 PhotoCraft：每个操作是一个命令 | 每个菜单项都有对应命令 |
| F010 | 结果日志（分析结果记录） | Analysis Results: The Results Log | P1 | 未开始 | 停靠面板 + 结构化记录 | 手动操作 |
| F011 | Notes 文本窗口 / Markdown 报告 | Notes Window | P2 | 未开始 | egui_commonmark | 手动操作 |
| F012 | 命令窗口（交互式脚本） | The Origin Command Window and Script Window | P1 | 未开始 | 嵌入 Python（PyO3） | 执行脚本修改工作表 |
| F013 | 深色模式 | Dark Mode | P0 | 已完成 | theme.rs 两套主题 | 目视检查 |
| F014 | 快捷键与自定义快捷键 | Hotkeys | P1 | 原型 | 固定快捷键已做，自定义待做 | 手动操作 |
| F015 | 全文搜索（在项目中查找） | Finding in Origin Project | P2 | 未开始 | 遍历项目数据 | 手动操作 |
| F016 | 工程保护（密码） | Protecting Origin Projects | P2 | 未开始 | age / chacha20poly1305 加密 | 加密→解密往返 |
| F017 | 中文界面 | — | P0 | 已完成 | 系统中文字体回退 | 目视检查 |
| F018 | 多语言界面（中 / 英） | — | P1 | 未开始 | fluent 或简单字符串表 | 切换语言检查 |
| F019 | 选项 / 偏好设置对话框 | Setting Your Preferences with the Options Dialog Box | P1 | 未开始 | 配置文件 + 设置面板 | 手动操作 |
| F020 | 自定义菜单与工具栏 | Custom Menus | P2 | 未开始 | 基于命令注册表 | 手动操作 |
| F021 | 模板中心 / 应用中心 | Template Center | 不做 | 不做 | — | — |
| F022 | 读取 Origin 工程文件 OPJU / OPJ | The Origin Project File | 不做 | 不做 | 私有格式无公开规范；旧 OPJ 可评估 liborigin | — |

## 工作表

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F023 | 单元格输入与编辑 | Basic Operations on Worksheet | P0 | 已完成 | egui_extras TableBuilder | 手动操作 |
| F024 | 长名称 / 单位 / 注释表头行 | Displaying Supporting Data in Worksheet Header Rows | P0 | 已完成 | 列元数据 | 轴标题自动取“长名称 (单位)” |
| F025 | 自定义表头行（参数、采样间隔、迷你图等） | Displaying Supporting Data in Worksheet Header Rows | P1 | 未开始 | 可扩展的表头行定义 | 手动操作 |
| F026 | 列属性：X / Y / Z / X误差 / Y误差 / 标签 / 忽略 | Setting Column Designation | P0 | 原型 | X / Y / X误差 / Y误差 / 忽略已做；Z 和标签待做 | 自动关联、插列、删除与撤销回归；Origin 目视对照待做 |
| F027 | 插入 / 删除 / 移动列 | Arranging the Worksheet: Columns and Rows | P0 | 原型 | 插入、删除已做，拖动移动待做 | 手动操作 |
| F028 | 插入 / 删除行 | Arranging the Worksheet: Columns and Rows | P0 | 未开始 | — | 手动操作 |
| F029 | 从 Excel 复制粘贴（多单元格） | Copying and Pasting Worksheet Data | P0 | 已完成 | 制表符解析 | 单元测试 |
| F030 | 鼠标拖动框选单元格、Shift 扩展、复制选区 | Scrolling and Simple Data Selection in the Worksheet | P0 | 原型 | 网页原型已做，待移植到 Rust | 手动操作 |
| F031 | 查找与替换 | Finding and Replacing Worksheet Data | P1 | 未开始 | — | 手动操作 |
| F032 | 数值显示格式（小数位、科学计数、工程单位） | Controlling How Numeric Data Display in the Worksheet Column | P0 | 未开始 | 列格式属性 | 单元测试 |
| F033 | 日期时间列与计算 | Dates and Times in the Worksheet | P1 | 未开始 | chrono / jiff | 单元测试 |
| F034 | 文本列与分类数据 | Setting Column Designation | P1 | 未开始 | 列数据类型 | 分类轴绘图 |
| F035 | 设置列值（公式列） | Using Formulae to Set or Transform Worksheet Column Values | P0 | 未开始 | 自写表达式解析器（col(A)*2+sin(col(B))） | 与 Origin 同公式结果逐行比较 |
| F036 | 单元格公式（=A1+B1） | Using a Formula to Set Cell Values | P2 | 未开始 | 依赖图 + 增量重算 | 单元测试 |
| F037 | 生成数据序列（等差、随机数、正态分布） | Generating Simple Worksheet Datasets | P1 | 未开始 | rand / rand_distr | 统计检验分布 |
| F038 | 排序（列、工作表、按多列） | Sorting Worksheet Data | P0 | 原型 | 稳定排序 | 单元测试 |
| F039 | 数据筛选 | Data Filter | P1 | 未开始 | — | 手动操作 |
| F040 | 条件格式（按值着色） | Conditional Formatting | P2 | 未开始 | — | 目视检查 |
| F041 | 转置 | Transposing Worksheets | P1 | 未开始 | — | 单元测试 |
| F042 | 堆叠 / 拆分列 | Stacking and Unstacking Worksheet Columns | P1 | 未开始 | — | 与 Origin 结果比较 |
| F043 | 合并工作表（按列、按标签） | Join Multiple Sheets by Column | P2 | 未开始 | polars join | 与 Origin 结果比较 |
| F044 | 数据精简（隔行抽取、平均合并） | Data Reduction | P1 | 未开始 | — | 与 Origin 结果比较 |
| F045 | 删除重复行 | Reduce Duplicated Rows | P2 | 未开始 | — | 单元测试 |
| F046 | 数据透视表 | Pivot Table | P2 | 未开始 | polars pivot | 与 Origin 结果比较 |
| F047 | 工作表查询（条件提取） | Worksheet Query - Quick Start | P2 | 未开始 | — | 与 Origin 结果比较 |
| F048 | 冻结行列 | Freeze Rows Columns | P1 | 未开始 | — | 手动操作 |
| F049 | 多工作表（一个工作簿多张表） | Workbooks | P0 | 未开始 | Book → Sheet → Column 三层模型 | 手动操作 |
| F050 | 工作簿模板 | What is Saved with the Workbook Template (*.OTW or *.OTWU) File? | P1 | 未开始 | 保存列定义与格式 | 套模板后结构一致 |
| F051 | 选中数据的状态栏统计 | Scrolling and Simple Data Selection in the Worksheet | P0 | 已完成 | — | 与手算比较 |
| F052 | 单元格内嵌图片 / 迷你图 | Inserting Graphs Images and Other Objects into Worksheet Cells | P2 | 未开始 | — | 目视检查 |
| F053 | 大数据量（百万行流畅滚动） | Origin Limitations | P0 | 原型 | 虚拟化表格已用；列式存储待改为 f64 数组 | 1e6 行滚动帧率 |

## 矩阵

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F054 | 矩阵窗口（二维网格数据） | Basic Matrixbook and Matrixsheet Operations | P1 | 未开始 | ndarray | 手动操作 |
| F055 | 矩阵 XY 坐标映射 | Matrixsheet Dimensions and XY Coordinates | P1 | 未开始 | — | 与 Origin 比较 |
| F056 | 设置矩阵值（公式） | Setting Matrix Values | P1 | 未开始 | 复用公式解析器 | 与 Origin 比较 |
| F057 | 矩阵转置、旋转、翻转、缩放 | Manipulating Matrix Data | P2 | 未开始 | — | 单元测试 |
| F058 | 工作表直接转矩阵 | Direct Worksheet to Matrix Conversion | P1 | 未开始 | — | 单元测试 |
| F059 | XYZ 网格化：规则数据 | Gridding of Regularly Spaced Data | P1 | 未开始 | — | 与 Origin 比较 |
| F060 | XYZ 网格化：随机点（Delaunay 插值、克里金、Renka-Cline） | Gridding Methods for Randomly Spaced Data | P1 | 未开始 | spade（Delaunay）+ 自写克里金 | 与 Origin 网格结果比较 |
| F061 | 虚拟矩阵 | Managing Virtual Matrix | P2 | 未开始 | — | — |

## 导入导出

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F062 | CSV / TXT 导入（自动识别分隔符） | Importing Simple ASCII Data Files | P0 | 已完成 | csv.rs | 单元测试 |
| F063 | GBK / UTF-8 编码识别 | Customizing the ASCII Import Options | P0 | 已完成 | encoding_rs | 单元测试 |
| F064 | 表头自动拆分长名称与单位 | Customizing the ASCII Import Options | P0 | 已完成 | — | 单元测试 |
| F065 | 导入向导（跳过表头行、指定分隔符、预览） | The Import Wizard | P0 | 未开始 | 预览对话框 | 手动操作 |
| F066 | 导入 Excel（.xlsx / .xls） | Importing from Excel | P0 | 未开始 | calamine | 与原表逐格比较 |
| F067 | 导出 Excel | Exporting Worksheets and Matrices | P1 | 未开始 | rust_xlsxwriter | 用 Excel 打开检查 |
| F068 | 导出 CSV | Exporting Worksheets and Matrices | P0 | 未开始 | — | 往返测试 |
| F069 | 拖放文件导入 | Importing Data by Drag-and-Drop | P0 | 未开始 | eframe dropped_files | 手动操作 |
| F070 | 批量导入多个文件 | Importing Simple ASCII Data Files | P1 | 未开始 | — | 手动操作 |
| F071 | 从文件名和表头提取变量 | Extracting Variables from the File Name and Header | P2 | 未开始 | — | 单元测试 |
| F072 | 二进制数据导入 | Importing Simple Binary Files with the Import Wizard | P2 | 未开始 | — | 单元测试 |
| F073 | 第三方格式：MATLAB .mat | Importing Third Party Files | P2 | 未开始 | matfile | 与 MATLAB 比较 |
| F074 | 第三方格式：HDF5 / NetCDF | Importing Third Party Files | P2 | 未开始 | hdf5 / netcdf（需 C 库） | 与 h5py 比较 |
| F075 | 第三方格式：仪器数据（SPC、JCAMP-DX 等） | Importing Third Party Files | P2 | 未开始 | 逐个格式实现 | 与 Origin 导入结果比较 |
| F076 | Parquet / Arrow | — | P2 | 未开始 | polars | 往返测试 |
| F077 | 数据库导入（SQL 查询） | Importing Data from a Database | P2 | 未开始 | sqlx / duckdb | 手动操作 |
| F078 | 数据连接器（文件更新后自动重新导入） | Data Connector | P2 | 未开始 | notify 监听文件 | 改文件后检查 |
| F079 | 导入 Origin 工程中的工作表 | Importing from Origin Project | 不做 | 不做 | 私有格式 | — |

## 绘图基础

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F080 | 按选列自动配对 X / Y 绘图 | Creating Graphs from Graph Templates | P0 | 已完成 | plot_from_selection | 单元测试 |
| F081 | 按选区（部分行）绘图 | Creating Graphs from Graph Templates | P0 | 原型 | 网页原型已做 | 手动操作 |
| F082 | 页面-图层-曲线 三层结构 | The Page-Layer-Plot Hierarchy | P0 | 已完成 | Graph → Page → Layer → PlotSpec；v0–v2 迁移与显式图层访问 | 迁移、往返和 SVG 版面回归测试 |
| F083 | 往图层添加 / 移除曲线 | Adding or Swapping Data Plots in the Graph Layer | P0 | 未开始 | 图层内容对话框 | 手动操作 |
| F084 | 多图层图形 | Creating Multi-Layer Graphs | P0 | 原型 | 多图层增删、各层属性与独立裁剪；分析与缩放跟随所选图层 | 跨工作簿、裁剪、保存与撤销测试；Origin 目视对照待做 |
| F085 | 函数绘图（y = f(x)） | Plotting Mathematical Functions | P1 | 未开始 | 复用公式解析器 | 与解析值比较 |
| F086 | 分组绘图（按分类列分组） | Plotting Data by Groups | P1 | 未开始 | — | 与 Origin 比较 |
| F087 | 图形缩放、平移 | Zooming on the Graph Page and inside Layer | P0 | 已完成 | egui_plot | 手动操作 |
| F088 | 批量绘图（同一模板换数据） | Batch Plotting | P1 | 未开始 | — | 手动操作 |
| F089 | 图形模板（保存样式，换数据一键出图） | Creating Graphs from Graph Templates | P0 | 未开始 | 图形样式与数据引用分离 | 套模板后版式一致 |
| F090 | 格式复制粘贴 | Copying and Pasting Formats | P1 | 未开始 | — | 手动操作 |
| F091 | 主题（图形样式主题） | Graph Formats and Themes | P1 | 未开始 | — | 目视检查 |
| F092 | 坐标轴范围与刻度 | Controlling the Graph Axis Display Range | P0 | 已完成 | ticks.rs（1/2/5 步长） | 与 Origin 自动刻度比较 |
| F093 | 对数轴 | Axes Dialog Reference | P0 | 已完成 | — | 与 Origin 比较 |
| F094 | 其他坐标类型：倒数、概率、log2、ln、Logit | Axes Dialog Reference | P1 | 未开始 | — | 与 Origin 比较 |
| F095 | 坐标轴断点 | Axes Dialog Reference | P1 | 未开始 | — | 目视比较 |
| F096 | 刻度标签格式（科学计数、日期、自定义） | Axes Dialog Reference | P0 | 原型 | a×10ⁿ 已做 | 与 Origin 比较 |
| F097 | 次刻度、刻度方向、四边框线 | Axes Dialog Reference | P0 | 原型 | SVG 导出已做，界面待做 | 目视比较 |
| F098 | 网格线 | Axes Dialog Reference | P0 | 已完成 | — | 目视检查 |
| F099 | 图例（自动更新） | Graph Legends | P0 | 已完成 | — | 目视检查 |
| F100 | 图例手动编辑与位置拖动 | Manual Control of the Graph Legend | P1 | 未开始 | — | 手动操作 |
| F101 | 色阶条（Color Scale） | Color Scales | P1 | 未开始 | — | 目视检查 |
| F102 | 文字标签（上下标、希腊字母、富文本） | Creating Text Labels | P0 | 未开始 | 迷你标记语言 + cosmic-text 排版 | 目视比较 |
| F103 | LaTeX 公式 | Creating Text Labels | P1 | 未开始 | 评估 typst 或 MathJax 转 SVG | 目视比较 |
| F104 | 绘图对象（直线、箭头、矩形、圆） | Creating and Editing Drawing Objects | P1 | 未开始 | — | 手动操作 |
| F105 | 数据标签 | Adding Labels to Plots | P1 | 未开始 | — | 目视检查 |
| F106 | 误差棒 | Adding Error Bars to Your Graph | P0 | 原型 | 对称 X/Y 误差列、稳定关联、端帽/线宽、共享 scene 与 SVG | 行对齐、对数裁剪、分组柱、项目往返和撤销回归；Origin 目视对照待做 |
| F107 | 插入图片 / 表格到图中 | Inserting Image into Graph | P2 | 未开始 | — | 目视检查 |
| F108 | 合并多张图 / 拆分图层 | The Merge Graph Dialog Box | P1 | 未开始 | — | 目视检查 |
| F109 | 图层排列（行列网格、间距） | The Arrange Layers Dialog Box | P0 | 原型 | 网格与自定义图层矩形、叠放；独立网格间距待做 | 布局、命中、透明背景、迁移与撤销测试；Origin 目视对照待做 |
| F110 | 布局页（多图拼版） | Using the Layout Page Window | P1 | 未开始 | — | 目视检查 |
| F111 | 曲线颜色、线型、符号递增 | Customizing Data Plot Colors | P0 | 已完成 | Origin 默认颜色 / 符号顺序 | 目视比较 |
| F112 | 按数据集控制颜色 / 大小（颜色映射） | Using a Dataset to Control Plot Color | P1 | 未开始 | — | 目视比较 |
| F113 | 绘图属性对话框（Plot Details） | Plot Details Controls | P0 | 原型 | 基础项已做，分页完整版待做 | 手动操作 |
| F114 | 显示范围（只画部分行） | Editing the Plots Display Range | P1 | 原型 | — | 手动操作 |
| F115 | 页面尺寸、背景、边距 | The (Plot Details) Print/Dimensions Tab | P0 | 原型 | 毫米尺寸、统一逻辑画布与图层背景；页面配色、边距待做 | 显示缩放比例、SVG 尺寸和项目往返测试 |

## 图形探索与小工具

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F116 | 读取坐标 / 数据点提示 | Reading Coordinate Values in the Graph | P0 | 原型 | egui_plot 自带悬停坐标 | 手动操作 |
| F117 | 数据高亮联动（图 ↔ 工作表） | Data Highlighter: Selecting Points in both Graph and Source Worksheet | P1 | 未开始 | — | 手动操作 |
| F118 | 屏蔽数据点（Mask） | Masking Data Points in the Graph or Worksheet | P1 | 未开始 | 掩码位图 | 屏蔽后拟合结果与 Origin 比较 |
| F119 | 框选 / 删除 / 移动数据点 | Drawing or Moving, or Deleting Data Points | P2 | 未开始 | — | 手动操作 |
| F120 | 异常值检测 | Detecting Outliers | P2 | 未开始 | — | 与 Origin 比较 |
| F121 | 局部放大视图 | Magnifying or Zooming In on a Portion of the Graph | P2 | 未开始 | — | 手动操作 |
| F122 | 快速拟合小工具（框选范围实时拟合） | The Quick Fit Gadget | P1 | 未开始 | 复用拟合引擎 | 与 Origin 比较 |
| F123 | 快速寻峰小工具 | The Quick Peaks Gadget | P1 | 未开始 | — | 与 Origin 比较 |
| F124 | 积分 / 微分 / 插值小工具 | The Integrate Gadget | P1 | 未开始 | — | 与 Origin 比较 |
| F125 | 交点小工具 | The Intersect Gadget | P2 | 未开始 | — | 与 Origin 比较 |
| F126 | FFT 小工具 | The FFT Gadget | P2 | 未开始 | — | 与 Origin 比较 |
| F127 | 统计小工具 | The Statistics Gadget | P2 | 未开始 | — | 与 Origin 比较 |
| F128 | 垂直游标 | The Vertical Cursor Gadget | P2 | 未开始 | — | 手动操作 |
| F129 | 曲线数字化（从图片取点） | Digitizer | P1 | 未开始 | image + 坐标标定 | 用已知图片检验 |

## 回归与拟合

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F130 | 线性回归（斜率、截距、标准误、R²、残差） | The Linear Regression Dialog Box | P0 | 原型 | faer / nalgebra 最小二乘 | NIST StRD 线性数据集 + Origin 结果 |
| F131 | 多项式回归 | The Polynomial Regression Dialog Box | P0 | 原型 | QR 分解 | NIST StRD（Wampler 系列） |
| F132 | 多元线性回归 | The Multiple Linear Regression Dialog Box | P1 | 未开始 | — | NIST StRD（Longley） |
| F133 | 含 X 误差的线性拟合（York 法） | The Linear Fit with X Error Dialog (Pro Only) | P2 | 未开始 | — | 与 Origin 比较 |
| F134 | 非线性拟合引擎（Levenberg–Marquardt） | Theory of Nonlinear Curve Fitting | P0 | 未开始 | 自写 LM 或 levenberg-marquardt crate | NIST StRD 非线性数据集（27 组，含高难度） |
| F135 | 参数初值自动估计 | Fitting Functions | P0 | 未开始 | 每个内置函数配初值估计 | 与 Origin 收敛结果比较 |
| F136 | 参数固定、上下界、线性约束 | Parameters,Bounds,Constraints and Weighting | P0 | 未开始 | — | 与 Origin 比较 |
| F137 | 加权拟合（误差列权重等） | Parameters,Bounds,Constraints and Weighting | P1 | 未开始 | — | 与 Origin 比较 |
| F138 | 拟合报告（参数表、ANOVA、置信带、预测带） | Interpreting Regression Results | P0 | 未开始 | statrs 计算 t / F 分布 | 参数、标准误、R² 与 Origin 差 < 1e-6 |
| F139 | 残差图（残差-自变量、正态概率图） | Residual Plot Analysis | P1 | 未开始 | — | 与 Origin 比较 |
| F140 | 多峰拟合 | Multiple Curves/Multiple Peaks | P1 | 未开始 | — | 与 Origin 比较 |
| F141 | 全局拟合（多组数据共享参数） | Multiple Curves/Multiple Peaks | P1 | 未开始 | — | 与 Origin 比较 |
| F142 | 用户自定义拟合函数 | Fitting Functions | P0 | 未开始 | 复用公式解析器，自动求导用数值差分或自动微分 | 与 Origin 比较 |
| F143 | 隐函数拟合（圆、椭圆等 5 个） | Nonlinear Implicit Curve Fitting | P2 | 未开始 | 正交距离回归 | 与 Origin 比较 |
| F144 | 曲面拟合（22 个内置函数） | Non-linear Surface Fitting | P2 | 未开始 | — | 与 Origin 比较 |
| F145 | 拟合比较（F 检验、AIC、BIC） | Fit Comparison | P1 | 未开始 | — | 与 Origin 比较 |
| F146 | 标准曲线反查 X / Y | Finding Y/X from X/Y Standard Curves | P1 | 未开始 | — | 与 Origin 比较 |
| F147 | 快速拟合（菜单一键） | Quick Fit | P0 | 原型 | — | 手动操作 |
| F148 | 内置拟合函数：峰函数（Gauss、Lorentz、Voigt、PsdVoigt、LogNormal 等） | Curve Fitting Functions（共 239 个） | P0 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F149 | 内置拟合函数：指数类（ExpDec1-3、ExpGrow、ExpAssoc 等） | Curve Fitting Functions（共 239 个） | P0 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F150 | 内置拟合函数：S 形与生长（Boltzmann、Logistic、DoseResp、Hill、Gompertz 等） | Curve Fitting Functions（共 239 个） | P0 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F151 | 内置拟合函数：幂函数与对数（Power、Allometric、Log 等） | Curve Fitting Functions（共 239 个） | P0 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F152 | 内置拟合函数：多项式与有理函数（Poly、Rational 等） | Curve Fitting Functions（共 239 个） | P0 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F153 | 内置拟合函数：波形（Sine、SineDamp、Pulse、SquareWave 等） | Curve Fitting Functions（共 239 个） | P1 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F154 | 内置拟合函数：酶动力学（MichaelisMenten、各类抑制模型等） | Curve Fitting Functions（共 239 个） | P1 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F155 | 内置拟合函数：吸附与结合（Langmuir、Freundlich、BET、OneSiteBind 等） | Curve Fitting Functions（共 239 个） | P1 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F156 | 内置拟合函数：其余专业函数（流变、热力学、统计分布等） | Curve Fitting Functions（共 239 个） | P2 | 未开始 | 函数表达式 + 初值估计，数据驱动注册 | 每个函数用 Origin 拟合结果做回归测试 |
| F157 | 寻峰分析专用函数（Bigaussian、FraserSuzuki 等 7 个） | Peak Analyzer Functions | P2 | 未开始 | — | 与 Origin 比较 |

## 数学运算

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F158 | 插值 / 外推（线性、三次样条、Akima） | Interpolate/Extrapolate | P0 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F159 | 按 X 求 Y 插值 | Interpolate/Extrapolate Y from X | P1 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F160 | 轨迹插值 | Trace Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F161 | XYZ 轨迹插值 | XYZ Trace Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F162 | 二维插值 | 2D Interpolate/Extrapolate | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F163 | 三维插值 | 3D Interpolation | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F164 | 数值积分（梯形法、曲线下面积） | Integrate | P0 | 原型 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F165 | 二维体积积分 | 2D Volume Integrate (Pro Only) | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F166 | 数值微分（含 Savitzky-Golay 平滑求导） | Differentiate | P0 | 原型 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F167 | 多条曲线平均 / 合并 | Average/Merge Multiple Curves | P1 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F168 | 简单列运算（加减乘除、两曲线相减） | Simple Column/Curve Math | P0 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F169 | 归一化（最大值、Z 分数、区间缩放） | Normalize | P0 | 原型 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F170 | 多边形面积 | Polygon Area | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F171 | 反函数（交换 X、Y 求逆） | Inverse | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F172 | 自定义滤波 / 中值滤波 | Median Filter | P2 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |
| F173 | 扣除直线基线 | Subtract Straight Line | P1 | 未开始 | 自写，数值算法参照 Numerical Recipes / SciPy | 与 Origin 及 SciPy 结果比较 |

## 统计

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F174 | 列统计（均值、标准差、中位数、分位数等） | Statistics on Columns | P0 | 原型 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F175 | 行统计 | Statistics on Rows | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F176 | 整表统计 | Statistics on Whole Sheet | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F177 | 频数统计 / 分箱 | Frequency Counts | P0 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F178 | 二维频数统计 | 2D Frequency Counts/Binning | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F179 | 正态性检验（Shapiro-Wilk、KS、Anderson-Darling 等） | Normality Test | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F180 | 相关系数（Pearson、Spearman、Kendall） | Correlation Coefficient (Pro Only) | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F181 | 偏相关系数 | Partial Correlation Coefficient (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F182 | 分布拟合 | Distribution Fit (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F183 | 列联表与卡方检验 | Cross Tabulation and Chi-square (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F184 | Grubbs / Dixon Q 异常值检验 | Grubbs' Test | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F185 | 单样本 t 检验 | One-Sample T-Test | P0 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F186 | 双样本 t 检验（含 Welch） | Two-Sample T-Test | P0 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F187 | 配对 t 检验 | Pair-Sample T-Test | P0 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F188 | 方差检验（单样本、F 检验） | Two Sample Test for Variances (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F189 | 比例检验、泊松率检验 | One-Sample Proportion Test (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F190 | 单因素 / 双因素 / 三因素方差分析（含多重比较） | One, Two, and Three Way ANOVA | P1 | 未开始 | statrs 分布函数 + 自写检验 | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F191 | 重复测量方差分析 | Repeated Measure ANOVA (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F192 | 非参数检验（Wilcoxon、Mann-Whitney、Kruskal-Wallis、Friedman 等） | Nonparametric Tests (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F193 | 生存分析（Kaplan-Meier、Cox 回归、Weibull） | Survival Analysis (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F194 | 主成分分析 PCA | Principal Component Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F195 | 偏最小二乘 PLS | Partial Least Squares | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F196 | 聚类分析 | Cluster Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F197 | 判别分析 | Discriminant Analysis | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F198 | 机器学习（分类、回归） | Machine Learning (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F199 | 功效与样本量计算 | Power and Sample Size (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |
| F200 | ROC 曲线 | ROC Curve (Pro Only) | P2 | 未开始 | statrs / linfa | 与 Origin 及 SciPy / R 结果比较（统计量、p 值） |

## 信号处理

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F201 | 平滑（移动平均、Savitzky-Golay、FFT 平滑、LOWESS） | Smoothing | P0 | 原型 | 自写 | 与 Origin 及 SciPy.signal 结果比较 |
| F202 | FFT 滤波（低通、高通、带通、带阻） | FFT Filters | P1 | 未开始 | rustfft / realfft | 与 Origin 及 SciPy.signal 结果比较 |
| F203 | IIR 滤波（巴特沃斯、切比雪夫等） | IIR Filters (Pro Only) | P2 | 未开始 | 自写或 biquad | 与 Origin 及 SciPy.signal 结果比较 |
| F204 | 傅里叶变换 FFT / IFFT（幅值、相位、功率谱、窗函数） | Fast Fourier Transform (FFT) | P0 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 |
| F205 | 短时傅里叶变换 STFT | Short-Time Fourier Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 |
| F206 | 卷积 / 反卷积 | Convolution | P1 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 |
| F207 | 相关（自相关、互相关） | Correlation | P1 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 |
| F208 | 相干性 | Coherence (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 |
| F209 | 希尔伯特变换、包络 | Hilbert Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 |
| F210 | 二维 FFT 与二维 FFT 滤波 | 2D Fourier Transform (Pro Only) | P2 | 未开始 | rustfft | 与 Origin 及 SciPy.signal 结果比较 |
| F211 | 小波变换（连续、分解、重构、去噪） | Wavelet Transforms (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 |
| F212 | 二维小波 | 2D Wavelet Transforms | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 |
| F213 | 二维相关 | 2D Correlation (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 |
| F214 | 矩阵平滑 | Matrix Smoothing | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 |
| F215 | 抽取 / 降采样 | Decimation (Pro Only) | P2 | 未开始 | 自写 | 与 Origin 及 SciPy.signal 结果比较 |

## 峰分析

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F216 | 基线：锚点法、自动基线（ALS、多项式等） | Peak Analyzer | P1 | 未开始 | 复用拟合引擎 | 与 Origin Peak Analyzer 结果比较 |
| F217 | 扣除基线 | Peak Analyzer | P1 | 未开始 | 复用拟合引擎 | 与 Origin Peak Analyzer 结果比较 |
| F218 | 寻峰（局部极值、导数法、阈值） | Peak Analyzer | P1 | 未开始 | 复用拟合引擎 | 与 Origin Peak Analyzer 结果比较 |
| F219 | 峰积分（面积、半峰宽、质心） | Peak Analyzer | P1 | 未开始 | 复用拟合引擎 | 与 Origin Peak Analyzer 结果比较 |
| F220 | 多峰拟合（重叠峰分峰） | Peak Analyzer | P1 | 未开始 | 复用拟合引擎 | 与 Origin Peak Analyzer 结果比较 |

## 图像处理

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F221 | 导入图片到矩阵 | Importing Images and Videos | P2 | 未开始 | image | 手动操作 |
| F222 | 图像调整（亮度、对比度、伽马） | Image Adjustment | P2 | 未开始 | image / imageproc | 与 Origin 比较 |
| F223 | 几何变换（旋转、裁剪、缩放） | Geometric Transformations | P2 | 未开始 | imageproc | 与 Origin 比较 |
| F224 | 空间滤波（边缘检测、锐化等） | Spatial Filters | P2 | 未开始 | imageproc | 与 Origin 比较 |
| F225 | 图像分析（粒子计数、剖面） | Image Analysis | P2 | 未开始 | imageproc | 与 Origin 比较 |

## 导出与发布

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F226 | 导出 SVG | Exporting Your Graph or Layout Page to an Image File | P0 | 已完成 | svg.rs | 目视比较 Origin 导出图 |
| F227 | 导出 PNG（指定 DPI） | The Export Graph as Image Dialog Box | P0 | 原型 | resvg / tiny-skia 渲染 SVG；网页原型已有 | 像素尺寸检查 |
| F228 | 导出 PDF（矢量，嵌入字体） | Which Export Format Should I Choose | P0 | 未开始 | svg2pdf / krilla | 用 PDF 阅读器检查 |
| F229 | 导出 EPS | Which Export Format Should I Choose | P1 | 未开始 | 自写 PostScript 输出 | LaTeX 编译检查 |
| F230 | 导出 TIFF（期刊要求） | Which Export Format Should I Choose | P1 | 未开始 | image | 尺寸与 DPI 检查 |
| F231 | 复制图形到剪贴板（粘贴到 Word / PPT） | Pasting or Embedding Graphs in Other Applications | P0 | 未开始 | arboard（位图）；矢量 EMF 待评估 | 粘贴到 Word 检查 |
| F232 | 导出 EMF（Office 可编辑矢量） | Which Export Format Should I Choose | P1 | 未开始 | 自写 EMF 写出器 | 插入 Word 检查 |
| F233 | 批量导出多张图 | The Export Graph Dialog Box | P1 | 未开始 | — | 手动操作 |
| F234 | 打印 | Printing Graphs, Layout Pages, and Analytical Results | P2 | 未开始 | 导出 PDF 后交给系统打印 | 手动操作 |
| F235 | 图形动画（导出视频 / GIF） | Creating Graph Animation | P2 | 未开始 | gif / ffmpeg | 手动操作 |

## 自动化与编程

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F236 | 内置 Python 运行时（预装 numpy、scipy、pandas） | Python Console | P1 | 未开始 | Windows 用官方嵌入版，macOS / Linux 用 python-build-standalone，随安装包分发；脚本在独立进程中运行，崩溃不影响软件 | 在没装 Python 的干净系统上安装后运行全部示例脚本 |
| F237 | scicanvas Python 包（连接运行中的软件；离线模式直接调用核心库） | Python Packages | P1 | 未开始 | 本机控制通道（带令牌）+ Arrow 批量传数据；离线模式用 PyO3 编译核心库 | 脚本示例回归测试，与界面操作结果比较 |
| F238 | 脚本窗口（编辑器、运行 / 停止、输出面板、报错行高亮） | The Origin Command Window and Script Window | P1 | 未开始 | egui 代码编辑面板，调用内置 python.exe 子进程，实时显示输出 | 运行、中途停止、报错定位到行 |
| F239 | 操作录制（把界面操作生成 Python 脚本，可回放） | Batch Processing | P1 | 未开始 | 命令注册表记录命令序列，翻译成 scicanvas Python 调用 | 录制 → 回放，结果与手动操作一致 |
| F240 | Python 包管理（装包到内置环境，可切换到系统 Python / conda） | Python Packages | P1 | 未开始 | 调用内置 pip；设置里选择解释器，脚本窗口标题显示当前环境 | 安装、卸载、切换环境后运行示例 |
| F241 | 命令行批处理（无界面） | Batch Processing | P1 | 未开始 | scicanvas-cli，调用核心库 | 脚本示例回归测试 |
| F242 | 分析模板（换数据自动重算） | Analysis Templates | P1 | 未开始 | 分析操作记录为命令序列 | 与 Origin 比较 |
| F243 | MCP 服务器（让 AI 代理驱动软件） | — | P1 | 未开始 | 参照 PhotoCraft：命令注册表暴露为 MCP 工具 | 代理完成示例任务 |
| F244 | 插件系统 | Apps | P2 | 未开始 | WebAssembly 插件（wasmtime）沙盒运行 | 示例插件 |
| F245 | R 控制台 | R Console and Rserve Console | P2 | 未开始 | — | — |
| F246 | MATLAB 控制台 | MATLAB Console | 不做 | 不做 | — | — |
| F247 | LabTalk 脚本兼容 | LabTalk Programming（1,018 页） | 不做 | 不做 | 改用 Python | — |
| F248 | Origin C 兼容 | Origin C（3,862 页） | 不做 | 不做 | 改用 Python / Rust 插件 | — |
| F249 | X-Function 兼容（约 680 个） | X-Function Reference | 不做 | 不做 | 功能按需在核心库实现，不做接口兼容 | — |
| F250 | COM 自动化服务器 | Automation Server | 不做 | 不做 | — | — |
| F251 | LabVIEW 虚拟仪器 | Using Origin from LabVIEW | 不做 | 不做 | — | — |
| F252 | Mathematica Link | Mathematica Link | 不做 | 不做 | — | — |
| F253 | Code Builder 集成开发环境 | The Code Builder | 不做 | 不做 | — | — |

## 图类型 · 2D 线图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F254 | 折线图 | Line Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F255 | 水平阶梯图 | Horizontal Step Graph | P1 | 原型 | 阶梯折线图元 + SVG 共享排版 | 拐点几何、拟合线与项目往返测试；Origin 目视对照待做 |
| F256 | 垂直阶梯图 | Vertical Step Graph | P1 | 原型 | 阶梯折线图元 + SVG 共享排版 | 拐点几何、拟合线与项目往返测试；Origin 目视对照待做 |
| F257 | Y 偏移堆叠线图 | Stacked Lines by Y Offsets Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F258 | 棒棒糖图 | Lollipop Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F259 | 颜色映射线系列 | Colormapped Line Series | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F260 | 样条曲线图 | Spline Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 2D 散点图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F261 | 散点图 | 2D Scatter Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F262 | 中心坐标散点图 | 2D Scatter Central Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F263 | 列散点图 | Column Scatter | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F264 | Y 误差棒图 | Y Error Bar Graph | P0 | 原型 | 对称误差列自动关联 + scene/SVG 共享图元 | 关联、端点几何、裁剪和撤销回归；Origin 目视对照待做 |
| F265 | XY 误差棒图 | XY Error Bar Graph | P0 | 原型 | 对称误差列自动关联 + scene/SVG 共享图元 | 关联、端点几何、裁剪和撤销回归；Origin 目视对照待做 |
| F266 | 散点 + 地毯图 | Scatter+Rug Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F267 | 垂线图 | Vertical Drop Line | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F268 | 气泡图 | Indexed Size Bubble Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F269 | 颜色映射散点图 | Color Mapped Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F270 | 气泡 + 颜色映射图 | Indexed Size (Bubble) and Color Map Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F271 | 误差带图 | Error Band Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F272 | 密度点图 | Density Dots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F273 | 彩色点图 | Color Dots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 2D 点线图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F274 | 点线图 | Line+Symbol Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F275 | 线系列图 | Line Series Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F276 | 两点线段图 | 2 Point Segment Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F277 | 三点线段图 | 3 Point Segment Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F278 | 样条连接点线图 | Spline Connected Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F279 | 按行绘图 | Row Wise Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F280 | 前后对比图 | Before and After Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 2D 柱状图与条形图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F281 | 柱状图 | Column Graph | P0 | 已完成 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F282 | 带标签柱状图 | Column + Label Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F283 | 条形图 | Bar Graph | P0 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F284 | 堆叠柱状图 | Stacked Column Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F285 | 浮动条形图 | Floating Bar Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F286 | 堆叠条形图 | Stacked Bar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F287 | 百分比堆叠柱状图 | 100% Stacked Column Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F288 | 百分比堆叠条形图 | 100% Stacked Bar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F289 | 浮动柱状图 | Floating Column Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F290 | 百分比丝带图 | 100% Ribbon Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F291 | 丝带图 | Ribbon Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F292 | 圆堆积图 | Circular Packing Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F293 | 螺旋条形图 | Spiral Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F294 | 频数图 | Frequency Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 饼图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F295 | 2D 饼图 | 2D Color Pie Chart | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F296 | 3D 饼图 | 3D Color Pie Chart | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F297 | 环形图 | Doughnut Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F298 | 复合饼图 | Pie Of Pie Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F299 | 复合条饼图 | Bar Of Pie Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F300 | 复合环形图 | Doughnut Of Pie | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F301 | 百分比环形图 | Doughnut with Total Wedges = 100 | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F302 | 不等半径饼图 | Pie with Different Radius Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F303 | 不等半径环形图 | Doughnut with Different Radius Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F304 | 旭日图 | Sunburst Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 多轴图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F305 | 双 Y 轴图 | Double Y Graph | P0 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F306 | 双 Y 轴柱状图 | Double-Y Column Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F307 | 双 Y 轴柱状 + 点线图 | Double-Y Column Line+Symbol Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F308 | 双 X 轴独立图 | Double X - Independent XY Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F309 | 双 X 轴公式换算图 | Double X - Custom Formula Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F310 | 共享 Y 的双 X 轴图 | Y-Shared Double X Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F311 | 阿伦尼乌斯双 X 轴图 | Double X - Arrhenius Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F312 | 2Ys Y-Y 图 | 2Ys Y-Y Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F313 | 2Ys 柱状图 | 2Ys Column | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F314 | 2Ys 柱状-点线图 | 2Ys Column-Line Symbol | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F315 | 3Ys Y-YY 图 | 3Ys Y-YY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F316 | 3Ys Y-Y-Y 图 | 3Ys Y-Y-Y | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F317 | 4Ys Y-YYY 图 | 4Ys Y-YYY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F318 | 4Ys YY-YY 图 | 4Ys YY-YY | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F319 | 多 Y 轴图 | Multiple Y Axes Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F320 | 放大图 | Zoom Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 多面板图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F321 | 上下两面板图 | Vertical 2 Panel Graph | P0 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F322 | 左右两面板图 | Horizontal 2 Panel Graph | P0 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F323 | 四面板图 | 4 Panel Graph | P0 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F324 | 九面板图 | 9 Panel Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F325 | 堆叠面板图 | Stack Panels Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F326 | 多面板图 | Multiple Panels Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F327 | 插图（图中图） | Inset Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F328 | 网格分面图 | Trellis | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F329 | 双 Y 网格分面图 | Trellis Double Y | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F330 | 簇图 | Cluster Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 瀑布图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F331 | 2D 瀑布图 | 2D Waterfall Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F332 | 2D 瀑布图（Y 颜色映射） | 2D Waterfall Y Color Mapping | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F333 | 2D 瀑布图（Z 颜色映射） | 2D Waterfall Z Color Mapping | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F334 | 3D 瀑布图 | 3D Waterfall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F335 | 3D 瀑布图（Y 颜色映射） | 3D Waterfall Y:Color Mapping | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F336 | 3D 瀑布图（Z 颜色映射） | 3D Waterfall Z:Color Mapping | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |

## 图类型 · 统计图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F337 | 二维核密度图 | 2D Kernel Density Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F338 | 均值条形图 | Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F339 | 蜂群图 | Beeswarm Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F340 | Bland-Altman 图 | Bland-Altman Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F341 | 箱线图 | Box Chart Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F342 | 双 Y 箱线图 | 2Ys Box Chart Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F343 | 直方图 / 分布图 | Histogram/Distribution Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F344 | 概率直方图 | Histogram Probabilities Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F345 | 直方图 + 地毯图 | Histogram + Rug/Distribution + Rug Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F346 | 区间图 | Interval Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F347 | 多面板直方图 | Multi-Panel Histogram Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F348 | 平行索引图 | Parallel Index Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F349 | 平行坐标图 | Parallel Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F350 | 平行集图 | Parallel Sets Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F351 | 帕累托图（分箱数据） | Pareto Chart - Binned Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F352 | 帕累托图（原始数据） | Pareto Chart - Raw Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F353 | 人口金字塔图（龙卷风图） | Population Pyramid Graph(Tornado Chart) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F354 | 概率图 | Probability Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F355 | Q-Q 图 | Q-Q Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F356 | 质量控制图（X̄-R） | QC(x bar R) Chart Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F357 | 点图 | Dot Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F358 | 堆叠点图 | Stacked Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F359 | 散点矩阵图 | Scatter Matrix Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F360 | 堆叠直方图 | Stacked Histograms Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F361 | 多变异图 | Multi-Vari Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F362 | 变异性图 | Variability Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F363 | 小提琴图 | Violin Plot | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F364 | 桑基图 | Sankey Diagrams | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F365 | 冲积图 | Alluvial Diagrams | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F366 | 网络图 | Network Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F367 | 和弦图 | Chord Diagram/Chord Diagram with Ratio Layout | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F368 | 层次边捆绑图 | Hierarchical Edge Bundling Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F369 | 边际箱线图 | Marginal Box Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F370 | 边际直方图 | Marginal Histograms Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F371 | 边际图 | Marginal Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F372 | 山脊图 | Ridgeline Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F373 | 对称图 | Symmetry Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F374 | 运行图 | Run Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F375 | 累积分布图 | CDF Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F376 | 交互作用图 | Interaction Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F377 | 主效应图 | Main Effects Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F378 | 因果图（鱼骨图） | Cause-and-Effect Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 分组图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F379 | 分组散点图 | Grouped Scatter | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F380 | 分组柱状图 | Grouped Columns | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F381 | 分组直方图 | Grouped Histogram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F382 | 分组箱线图（索引数据） | Grouped Box Charts-Indexed Data | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F383 | 分组箱线图（原始数据） | Grouped Box Charts-Raw Data | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F384 | 分组列散点图 | Grouped Column Scatter-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F385 | 分组区间图 | Grouped Interval Plot-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F386 | 分组均值条形图 | Grouped Mean Bar-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F387 | 分组点图 | Grouped Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F388 | 分组堆叠点图 | Grouped Stacked Dot Plots | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F389 | 分组小提琴图 | Grouped Violin-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F390 | 分组半小提琴图 | Grouped Half Violin-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F391 | 分组半箱线图 | Grouped Half Box-Indexed Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F392 | 分组浮动条形图 | Grouped Floating Bar | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 地图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F393 | 条形地图 | Bar Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 |
| F394 | 饼图地图 | Pie Map Chart | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 |
| F395 | 方格地图 | Tile Grid Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 |
| F396 | 桑基地图 | Sankey Map | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 |
| F397 | Stiff 地图 | Stiff Map Diagram | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 |
| F398 | 矩形树图 | Treemap | P2 | 未开始 | geojson / shapefile | 与 Origin 同数据出图目视比较 |

## 图类型 · 面积图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F399 | 面积图 | Area Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F400 | 堆叠面积图 | Stacked Area Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F401 | 百分比堆叠面积图 | 100% Stacked Area Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F402 | 填充面积图 | Fill Area Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F403 | 双色填充图 | Fill Two Colors Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F404 | 风筝图 | Kite Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 专业图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F405 | 窗口浏览图 | Window-view Browser Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F406 | 子弹图 | Bullet Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F407 | 极坐标等高线 + 梯度矢量 | Polar Contour + Gradient Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F408 | 等高线 + 梯度矢量 | Contour Line + Gradient Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F409 | 等高线 + 流线 | Contour + Streamline | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F410 | Piper 三线图 | Piper/Trilinear Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F411 | Durov 图 | Durov Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F412 | Stiff 图 | Stiff Diagram | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F413 | 极坐标图 | Polar Graph | P1 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F414 | 极坐标矢量图（θrθr） | Polar Vector theta r theta r graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F415 | 罗盘图 | Compass Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F416 | 极坐标矢量图（θrAM） | Polar Vector ?RAM graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F417 | 雷达图 | Radar Chart (Spider Plot) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F418 | 径向条形图 | Radial Bar and Radial Stacked Bar Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F419 | 直角三元图 | Right-Triangle Ternary | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F420 | 史密斯圆图 | Smith Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F421 | 堆叠径向图 | Stacked Radial Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F422 | 流线图 | Streamline Plot | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F423 | 三元图 | Ternary | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F424 | 三元矢量图 | Ternary Vector | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F425 | 风玫瑰图（分箱数据） | Wind Rose-Binned Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F426 | 风玫瑰图（原始数据） | Wind Rose-Raw Data | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F427 | 矢量图（XY + 角度 + 大小） | X, Y, Angle, Magnitude Vector Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F428 | 矢量图（起点 XY + 终点 XY） | X, Y, X, Y Vector Graph | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 金融图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F429 | 桥图（瀑布式） | Bridge Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F430 | 高低收图 | High-Low-Close Charts | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F431 | K 线图 | Japanese_Candlestick | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F432 | OHLC 条形图 | OHLC Bar Chart | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F433 | OHLC + 成交量图 | OHLC-Volume | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |
| F434 | 股票折线图 | Line (Stock) | P2 | 未开始 | 2D 渲染器 + SVG 导出 | 与 Origin 同数据出图目视比较 |

## 图类型 · 3D XYY 图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F435 | XYY 3D 柱状图 | XYY 3D Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F436 | XYY 3D 百分比堆叠柱 | XYY 3D 100% Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F437 | XYY 3D 并列柱 | XYY 3D Side by Side Bars | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F438 | XYY 3D 堆叠柱 | XYY 3D Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F439 | XYY 3D 柱（带误差棒） | XYY 3D Bar Graph with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F440 | 3D 丝带图 | 3D Ribbons Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F441 | 3D 百分比堆叠墙图 | 3D 100% Stacked Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F442 | 3D 堆叠墙图 | 3D Stacked Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F443 | 3D 墙图 | 3D Wall Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |

## 图类型 · 3D 曲面图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F444 | 填色曲面图 | Color Fill Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F445 | X 恒定带底曲面 | X Constant with Base Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F446 | Y 恒定带底曲面 | Y Constant with Base Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F447 | 颜色映射曲面图 | Color Map Surface | P1 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F448 | 颜色映射曲面（带投影） | Color Map Surface with Projection | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F449 | 3D 三元颜色映射曲面 | 3D Ternary Colormap Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F450 | 线框图 | Wire Frame | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F451 | 线框曲面 | Wire Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F452 | 填色曲面（带误差棒） | Color Fill Surface with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F453 | 颜色映射曲面（带误差棒） | Colormap Surface with Error Bar | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F454 | 多个填色曲面 | Multiple Color Fill Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F455 | 多个颜色映射曲面 | Multiple Color Map Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F456 | 参数曲面 | Parametric Surface | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F457 | 3D 堆叠曲面 | 3D Stacked Surfaces | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F458 | 3D 堆叠热图 | 3D Stacked Heatmaps | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |

## 图类型 · 3D 散点 / 柱 / 矢量图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F459 | 3D 散点图 | 3D Scatter Graph | P1 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F460 | 3D 散点（Z 误差棒） | 3D Scatter with Z Error Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F461 | 3D 线图 | 3D Line Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F462 | 轨迹图 | Trajectory Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F463 | 3D 柱状图 | 3D Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F464 | 3D 柱状图（分类 XY） | 3D Bar Graph with Categorical/Text XY | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F465 | 3D 堆叠柱状图 | 3D Stacked Bars Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F466 | 3D 百分比堆叠柱状图 | 3D 100% Stacked Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F467 | 3D 浮动柱状图 | 3D Floating Bar Graph | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F468 | 3D 矢量（XYZ XYZ） | 3D Vector XYZ XYZ | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F469 | 3D 矢量（XYZ dXdYdZ） | 3D Vector XYZ dXdYdZ | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F470 | 3D 三元散点 | 3D Ternary Symbol | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |
| F471 | 四面体图（四元相图） | 3D Tetrahedral(Quaternary Phase Diagram) | P2 | 未开始 | wgpu 3D 渲染（three-d 或自写） | 与 Origin 同数据出图目视比较 |

## 图类型 · 等高线与热图

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F472 | 填色等高线图 | Color Fill Contour | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F473 | 黑白等高线图（带标签） | Black and White Lines with Labels Contour | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F474 | 灰度等高线图 | Gray Scale Map Contour | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F475 | 分类 Z 等高线图 | Contour - Categorical Z | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F476 | 热图 | Heatmap | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F477 | 极坐标热图 | Polar Heatmap | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F478 | 带标签热图 | Heatmap with Labels | P1 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F479 | 分割热图 | Split Heatmap | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F480 | 分块热图 | Heatmap with Split Tiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F481 | 极坐标等高线（XYZ 数据） | Polar Contour from XYZ Data | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F482 | 极坐标等高线（矩阵数据） | Polar Contour from Matrix Data | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F483 | 三元等高线图 | Ternary Contour | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F484 | 叠加等高线图 | Overlaid Contour Plot | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |

## 图类型 · 图像与剖面

| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |
|---|---|---|---|---|---|---|
| F485 | 等高线剖面图 | Contour Profiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F486 | 图片背景 | Image as Background | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F487 | 图像图 | Image Plot | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
| F488 | 图像剖面图 | Image Profiles | P2 | 未开始 | contour crate（marching squares）+ 色阶 | 与 Origin 同数据出图目视比较 |
