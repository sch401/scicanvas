"""生成 SciCanvas 与 Origin 的功能对照清单。

条目来自 Origin 帮助文档的目录（只引用章节标题，用于对照查阅，不含文档正文）。
输出：parity.json（给对照表页面用）和 PARITY.md（放在仓库里）。

用法：python3 build_parity.py
"""

import json
from pathlib import Path

DONE, PROTO, TODO, SKIP = "已完成", "原型", "未开始", "不做"
MILESTONE = {"P0": "M1 日常够用", "P1": "M2 论文全流程", "P2": "M3 高级功能", "不做": "—"}

rows = []


def add(module, feature, ref, pri, status=TODO, how="", verify=""):
    rows.append(
        dict(module=module, feature=feature, ref=ref, pri=pri, status=status, how=how, verify=verify)
    )


# ---------------------------------------------------------------- 工作区与项目
M = "工作区与项目"
add(M, "多文档子窗口（拖动、缩放、最小化、最大化）", "Basic Child Window Operations", "P0", DONE, "egui Window", "手动操作")
add(M, "项目管理器（文件夹树、显示 / 隐藏窗口）", "Project Explorer", "P0", DONE, "egui SidePanel；文件夹层级待做", "手动操作")
add(M, "项目管理器多级文件夹、拖动整理", "Using Project Explorer to Manage Child Windows", "P1", TODO, "树形数据模型 + 拖放", "手动操作")
add(M, "窗口排列：层叠 / 平铺", "Arrange Windows", "P1", PROTO, "按工作区矩形计算位置", "手动操作")
add(M, "项目文件保存 / 打开", "Opening, Closing and Backing Up the Origin Project File", "P0", DONE, "serde_json（.scproj）；以后换成带二进制块的压缩格式", "保存→打开→逐字段比较")
add(M, "自动备份与崩溃恢复", "Opening, Closing and Backing Up the Origin Project File", "P1", TODO, "定时写临时文件", "强制退出后恢复")
add(M, "最近打开的文件", "Recent Origin Files", "P1", TODO, "配置文件记录", "手动操作")
add(M, "撤销 / 重做", "—", "P0", DONE, "显式修改标记与编辑事务；100 步 / 300 MiB 上限", "十万行事务复用、空闲帧、真实编辑控件回归测试")
add(M, "命令注册表（菜单、工具栏、脚本共用同一套命令）", "—", "P0", PROTO, "参照 PhotoCraft：每个操作是一个命令", "每个菜单项都有对应命令")
add(M, "结果日志（分析结果记录）", "Analysis Results: The Results Log", "P1", TODO, "停靠面板 + 结构化记录", "手动操作")
add(M, "Notes 文本窗口 / Markdown 报告", "Notes Window", "P2", TODO, "egui_commonmark", "手动操作")
add(M, "命令窗口（交互式脚本）", "The Origin Command Window and Script Window", "P1", TODO, "嵌入 Python（PyO3）", "执行脚本修改工作表")
add(M, "深色模式", "Dark Mode", "P0", DONE, "theme.rs 两套主题", "目视检查")
add(M, "快捷键与自定义快捷键", "Hotkeys", "P1", PROTO, "固定快捷键已做，自定义待做", "手动操作")
add(M, "全文搜索（在项目中查找）", "Finding in Origin Project", "P2", TODO, "遍历项目数据", "手动操作")
add(M, "工程保护（密码）", "Protecting Origin Projects", "P2", TODO, "age / chacha20poly1305 加密", "加密→解密往返")
add(M, "中文界面", "—", "P0", DONE, "系统中文字体回退", "目视检查")
add(M, "多语言界面（中 / 英）", "—", "P1", TODO, "fluent 或简单字符串表", "切换语言检查")
add(M, "选项 / 偏好设置对话框", "Setting Your Preferences with the Options Dialog Box", "P1", TODO, "配置文件 + 设置面板", "手动操作")
add(M, "自定义菜单与工具栏", "Custom Menus", "P2", TODO, "基于命令注册表", "手动操作")
add(M, "模板中心 / 应用中心", "Template Center", "不做", TODO, "", "")
add(M, "读取 Origin 工程文件 OPJU / OPJ", "The Origin Project File", "不做", TODO, "私有格式无公开规范；旧 OPJ 可评估 liborigin", "")

# ---------------------------------------------------------------- 工作表
M = "工作表"
add(M, "单元格输入与编辑", "Basic Operations on Worksheet", "P0", DONE, "egui_extras TableBuilder", "手动操作")
add(M, "长名称 / 单位 / 注释表头行", "Displaying Supporting Data in Worksheet Header Rows", "P0", DONE, "列元数据", "轴标题自动取“长名称 (单位)”")
add(M, "自定义表头行（参数、采样间隔、迷你图等）", "Displaying Supporting Data in Worksheet Header Rows", "P1", TODO, "可扩展的表头行定义", "手动操作")
add(M, "列属性：X / Y / Z / X误差 / Y误差 / 标签 / 忽略", "Setting Column Designation", "P0", PROTO, "X / Y / X误差 / Y误差 / 忽略已做；Z 和标签待做", "自动关联、插列、删除与撤销回归；Origin 目视对照待做")
add(M, "插入 / 删除 / 移动列", "Arranging the Worksheet: Columns and Rows", "P0", PROTO, "插入、删除已做，拖动移动待做", "手动操作")
add(M, "插入 / 删除行", "Arranging the Worksheet: Columns and Rows", "P0", TODO, "", "手动操作")
add(M, "从 Excel 复制粘贴（多单元格）", "Copying and Pasting Worksheet Data", "P0", DONE, "制表符解析", "单元测试")
add(M, "鼠标拖动框选单元格、Shift 扩展、复制选区", "Scrolling and Simple Data Selection in the Worksheet", "P0", PROTO, "网页原型已做，待移植到 Rust", "手动操作")
add(M, "查找与替换", "Finding and Replacing Worksheet Data", "P1", TODO, "", "手动操作")
add(M, "数值显示格式（小数位、科学计数、工程单位）", "Controlling How Numeric Data Display in the Worksheet Column", "P0", TODO, "列格式属性", "单元测试")
add(M, "日期时间列与计算", "Dates and Times in the Worksheet", "P1", TODO, "chrono / jiff", "单元测试")
add(M, "文本列与分类数据", "Setting Column Designation", "P1", TODO, "列数据类型", "分类轴绘图")
add(M, "设置列值（公式列）", "Using Formulae to Set or Transform Worksheet Column Values", "P0", TODO, "自写表达式解析器（col(A)*2+sin(col(B))）", "与 Origin 同公式结果逐行比较")
add(M, "单元格公式（=A1+B1）", "Using a Formula to Set Cell Values", "P2", TODO, "依赖图 + 增量重算", "单元测试")
add(M, "生成数据序列（等差、随机数、正态分布）", "Generating Simple Worksheet Datasets", "P1", TODO, "rand / rand_distr", "统计检验分布")
add(M, "排序（列、工作表、按多列）", "Sorting Worksheet Data", "P0", PROTO, "稳定排序", "单元测试")
add(M, "数据筛选", "Data Filter", "P1", TODO, "", "手动操作")
add(M, "条件格式（按值着色）", "Conditional Formatting", "P2", TODO, "", "目视检查")
add(M, "转置", "Transposing Worksheets", "P1", TODO, "", "单元测试")
add(M, "堆叠 / 拆分列", "Stacking and Unstacking Worksheet Columns", "P1", TODO, "", "与 Origin 结果比较")
add(M, "合并工作表（按列、按标签）", "Join Multiple Sheets by Column", "P2", TODO, "polars join", "与 Origin 结果比较")
add(M, "数据精简（隔行抽取、平均合并）", "Data Reduction", "P1", TODO, "", "与 Origin 结果比较")
add(M, "删除重复行", "Reduce Duplicated Rows", "P2", TODO, "", "单元测试")
add(M, "数据透视表", "Pivot Table", "P2", TODO, "polars pivot", "与 Origin 结果比较")
add(M, "工作表查询（条件提取）", "Worksheet Query - Quick Start", "P2", TODO, "", "与 Origin 结果比较")
add(M, "冻结行列", "Freeze Rows Columns", "P1", TODO, "", "手动操作")
add(M, "多工作表（一个工作簿多张表）", "Workbooks", "P0", TODO, "Book → Sheet → Column 三层模型", "手动操作")
add(M, "工作簿模板", "What is Saved with the Workbook Template (*.OTW or *.OTWU) File?", "P1", TODO, "保存列定义与格式", "套模板后结构一致")
add(M, "选中数据的状态栏统计", "Scrolling and Simple Data Selection in the Worksheet", "P0", DONE, "", "与手算比较")
add(M, "单元格内嵌图片 / 迷你图", "Inserting Graphs Images and Other Objects into Worksheet Cells", "P2", TODO, "", "目视检查")
add(M, "大数据量（百万行流畅滚动）", "Origin Limitations", "P0", PROTO, "虚拟化表格已用；列式存储待改为 f64 数组", "1e6 行滚动帧率")

# ---------------------------------------------------------------- 矩阵
M = "矩阵"
add(M, "矩阵窗口（二维网格数据）", "Basic Matrixbook and Matrixsheet Operations", "P1", TODO, "ndarray", "手动操作")
add(M, "矩阵 XY 坐标映射", "Matrixsheet Dimensions and XY Coordinates", "P1", TODO, "", "与 Origin 比较")
add(M, "设置矩阵值（公式）", "Setting Matrix Values", "P1", TODO, "复用公式解析器", "与 Origin 比较")
add(M, "矩阵转置、旋转、翻转、缩放", "Manipulating Matrix Data", "P2", TODO, "", "单元测试")
add(M, "工作表直接转矩阵", "Direct Worksheet to Matrix Conversion", "P1", TODO, "", "单元测试")
add(M, "XYZ 网格化：规则数据", "Gridding of Regularly Spaced Data", "P1", TODO, "", "与 Origin 比较")
add(M, "XYZ 网格化：随机点（Delaunay 插值、克里金、Renka-Cline）", "Gridding Methods for Randomly Spaced Data", "P1", TODO, "spade（Delaunay）+ 自写克里金", "与 Origin 网格结果比较")
add(M, "虚拟矩阵", "Managing Virtual Matrix", "P2", TODO, "", "")

# ---------------------------------------------------------------- 导入导出
M = "导入导出"
add(M, "CSV / TXT 导入（自动识别分隔符）", "Importing Simple ASCII Data Files", "P0", DONE, "csv.rs", "单元测试")
add(M, "GBK / UTF-8 编码识别", "Customizing the ASCII Import Options", "P0", DONE, "encoding_rs", "单元测试")
add(M, "表头自动拆分长名称与单位", "Customizing the ASCII Import Options", "P0", DONE, "", "单元测试")
add(M, "导入向导（跳过表头行、指定分隔符、预览）", "The Import Wizard", "P0", TODO, "预览对话框", "手动操作")
add(M, "导入 Excel（.xlsx / .xls）", "Importing from Excel", "P0", TODO, "calamine", "与原表逐格比较")
add(M, "导出 Excel", "Exporting Worksheets and Matrices", "P1", TODO, "rust_xlsxwriter", "用 Excel 打开检查")
add(M, "导出 CSV", "Exporting Worksheets and Matrices", "P0", TODO, "", "往返测试")
add(M, "拖放文件导入", "Importing Data by Drag-and-Drop", "P0", TODO, "eframe dropped_files", "手动操作")
add(M, "批量导入多个文件", "Importing Simple ASCII Data Files", "P1", TODO, "", "手动操作")
add(M, "从文件名和表头提取变量", "Extracting Variables from the File Name and Header", "P2", TODO, "", "单元测试")
add(M, "二进制数据导入", "Importing Simple Binary Files with the Import Wizard", "P2", TODO, "", "单元测试")
add(M, "第三方格式：MATLAB .mat", "Importing Third Party Files", "P2", TODO, "matfile", "与 MATLAB 比较")
add(M, "第三方格式：HDF5 / NetCDF", "Importing Third Party Files", "P2", TODO, "hdf5 / netcdf（需 C 库）", "与 h5py 比较")
add(M, "第三方格式：仪器数据（SPC、JCAMP-DX 等）", "Importing Third Party Files", "P2", TODO, "逐个格式实现", "与 Origin 导入结果比较")
add(M, "Parquet / Arrow", "—", "P2", TODO, "polars", "往返测试")
add(M, "数据库导入（SQL 查询）", "Importing Data from a Database", "P2", TODO, "sqlx / duckdb", "手动操作")
add(M, "数据连接器（文件更新后自动重新导入）", "Data Connector", "P2", TODO, "notify 监听文件", "改文件后检查")
add(M, "导入 Origin 工程中的工作表", "Importing from Origin Project", "不做", TODO, "私有格式", "")

# ---------------------------------------------------------------- 绘图基础
M = "绘图基础"
add(M, "按选列自动配对 X / Y 绘图", "Creating Graphs from Graph Templates", "P0", DONE, "plot_from_selection", "单元测试")
add(M, "按选区（部分行）绘图", "Creating Graphs from Graph Templates", "P0", PROTO, "网页原型已做", "手动操作")
add(M, "页面-图层-曲线 三层结构", "The Page-Layer-Plot Hierarchy", "P0", DONE, "Graph → Page → Layer → PlotSpec；v0–v2 迁移与显式图层访问", "迁移、往返和 SVG 版面回归测试")
add(M, "往图层添加 / 移除曲线", "Adding or Swapping Data Plots in the Graph Layer", "P0", TODO, "图层内容对话框", "手动操作")
add(M, "多图层图形", "Creating Multi-Layer Graphs", "P0", PROTO, "多图层增删、各层属性与独立裁剪；分析与缩放跟随所选图层", "跨工作簿、裁剪、保存与撤销测试；Origin 目视对照待做")
add(M, "函数绘图（y = f(x)）", "Plotting Mathematical Functions", "P1", TODO, "复用公式解析器", "与解析值比较")
add(M, "分组绘图（按分类列分组）", "Plotting Data by Groups", "P1", TODO, "", "与 Origin 比较")
add(M, "图形缩放、平移", "Zooming on the Graph Page and inside Layer", "P0", DONE, "egui_plot", "手动操作")
add(M, "批量绘图（同一模板换数据）", "Batch Plotting", "P1", TODO, "", "手动操作")
add(M, "图形模板（保存样式，换数据一键出图）", "Creating Graphs from Graph Templates", "P0", TODO, "图形样式与数据引用分离", "套模板后版式一致")
add(M, "格式复制粘贴", "Copying and Pasting Formats", "P1", TODO, "", "手动操作")
add(M, "主题（图形样式主题）", "Graph Formats and Themes", "P1", TODO, "", "目视检查")
add(M, "坐标轴范围与刻度", "Controlling the Graph Axis Display Range", "P0", DONE, "ticks.rs（1/2/5 步长）", "与 Origin 自动刻度比较")
add(M, "对数轴", "Axes Dialog Reference", "P0", DONE, "", "与 Origin 比较")
add(M, "其他坐标类型：倒数、概率、log2、ln、Logit", "Axes Dialog Reference", "P1", TODO, "", "与 Origin 比较")
add(M, "坐标轴断点", "Axes Dialog Reference", "P1", TODO, "", "目视比较")
add(M, "刻度标签格式（科学计数、日期、自定义）", "Axes Dialog Reference", "P0", PROTO, "a×10ⁿ 已做", "与 Origin 比较")
add(M, "次刻度、刻度方向、四边框线", "Axes Dialog Reference", "P0", PROTO, "SVG 导出已做，界面待做", "目视比较")
add(M, "网格线", "Axes Dialog Reference", "P0", DONE, "", "目视检查")
add(M, "图例（自动更新）", "Graph Legends", "P0", DONE, "", "目视检查")
add(M, "图例手动编辑与位置拖动", "Manual Control of the Graph Legend", "P1", TODO, "", "手动操作")
add(M, "色阶条（Color Scale）", "Color Scales", "P1", TODO, "", "目视检查")
add(M, "文字标签（上下标、希腊字母、富文本）", "Creating Text Labels", "P0", TODO, "迷你标记语言 + cosmic-text 排版", "目视比较")
add(M, "LaTeX 公式", "Creating Text Labels", "P1", TODO, "评估 typst 或 MathJax 转 SVG", "目视比较")
add(M, "绘图对象（直线、箭头、矩形、圆）", "Creating and Editing Drawing Objects", "P1", TODO, "", "手动操作")
add(M, "数据标签", "Adding Labels to Plots", "P1", PROTO, "数值/X-Y/行号/指定列文本，字号/偏移与 scene/SVG", "行对齐、文本转义、柱段值、图层与撤销回归；Origin 目视对照待做")
add(M, "误差棒", "Adding Error Bars to Your Graph", "P0", PROTO, "对称 X/Y 误差列、稳定关联、端帽/线宽、共享 scene 与 SVG", "行对齐、对数裁剪、分组柱、项目往返和撤销回归；Origin 目视对照待做")
add(M, "插入图片 / 表格到图中", "Inserting Image into Graph", "P2", TODO, "", "目视检查")
add(M, "合并多张图 / 拆分图层", "The Merge Graph Dialog Box", "P1", TODO, "", "目视检查")
add(M, "图层排列（行列网格、间距）", "The Arrange Layers Dialog Box", "P0", PROTO, "网格与自定义图层矩形、叠放；独立网格间距待做", "布局、命中、透明背景、迁移与撤销测试；Origin 目视对照待做")
add(M, "布局页（多图拼版）", "Using the Layout Page Window", "P1", TODO, "", "目视检查")
add(M, "曲线颜色、线型、符号递增", "Customizing Data Plot Colors", "P0", DONE, "Origin 默认颜色 / 符号顺序", "目视比较")
add(M, "按数据集控制颜色 / 大小（颜色映射）", "Using a Dataset to Control Plot Color", "P1", TODO, "", "目视比较")
add(M, "绘图属性对话框（Plot Details）", "Plot Details Controls", "P0", PROTO, "基础项已做，分页完整版待做", "手动操作")
add(M, "显示范围（只画部分行）", "Editing the Plots Display Range", "P1", PROTO, "", "手动操作")
add(M, "页面尺寸、背景、边距", "The (Plot Details) Print/Dimensions Tab", "P0", PROTO, "毫米尺寸、统一逻辑画布与图层背景；页面配色、边距待做", "显示缩放比例、SVG 尺寸和项目往返测试")

# ---------------------------------------------------------------- 图形探索与小工具
M = "图形探索与小工具"
add(M, "读取坐标 / 数据点提示", "Reading Coordinate Values in the Graph", "P0", PROTO, "egui_plot 自带悬停坐标", "手动操作")
add(M, "数据高亮联动（图 ↔ 工作表）", "Data Highlighter: Selecting Points in both Graph and Source Worksheet", "P1", TODO, "", "手动操作")
add(M, "屏蔽数据点（Mask）", "Masking Data Points in the Graph or Worksheet", "P1", TODO, "掩码位图", "屏蔽后拟合结果与 Origin 比较")
add(M, "框选 / 删除 / 移动数据点", "Drawing or Moving, or Deleting Data Points", "P2", TODO, "", "手动操作")
add(M, "异常值检测", "Detecting Outliers", "P2", TODO, "", "与 Origin 比较")
add(M, "局部放大视图", "Magnifying or Zooming In on a Portion of the Graph", "P2", TODO, "", "手动操作")
add(M, "快速拟合小工具（框选范围实时拟合）", "The Quick Fit Gadget", "P1", TODO, "复用拟合引擎", "与 Origin 比较")
add(M, "快速寻峰小工具", "The Quick Peaks Gadget", "P1", TODO, "", "与 Origin 比较")
add(M, "积分 / 微分 / 插值小工具", "The Integrate Gadget", "P1", TODO, "", "与 Origin 比较")
add(M, "交点小工具", "The Intersect Gadget", "P2", TODO, "", "与 Origin 比较")
add(M, "FFT 小工具", "The FFT Gadget", "P2", TODO, "", "与 Origin 比较")
add(M, "统计小工具", "The Statistics Gadget", "P2", TODO, "", "与 Origin 比较")
add(M, "垂直游标", "The Vertical Cursor Gadget", "P2", TODO, "", "手动操作")
add(M, "曲线数字化（从图片取点）", "Digitizer", "P1", TODO, "image + 坐标标定", "用已知图片检验")

# ---------------------------------------------------------------- 拟合
M = "回归与拟合"
add(M, "线性回归（斜率、截距、标准误、R²、残差）", "The Linear Regression Dialog Box", "P0", PROTO, "faer / nalgebra 最小二乘", "NIST StRD 线性数据集 + Origin 结果")
add(M, "多项式回归", "The Polynomial Regression Dialog Box", "P0", PROTO, "QR 分解", "NIST StRD（Wampler 系列）")
add(M, "多元线性回归", "The Multiple Linear Regression Dialog Box", "P1", TODO, "", "NIST StRD（Longley）")
add(M, "含 X 误差的线性拟合（York 法）", "The Linear Fit with X Error Dialog (Pro Only)", "P2", TODO, "", "与 Origin 比较")
add(M, "非线性拟合引擎（Levenberg–Marquardt）", "Theory of Nonlinear Curve Fitting", "P0", TODO, "自写 LM 或 levenberg-marquardt crate", "NIST StRD 非线性数据集（27 组，含高难度）")
add(M, "参数初值自动估计", "Fitting Functions", "P0", TODO, "每个内置函数配初值估计", "与 Origin 收敛结果比较")
add(M, "参数固定、上下界、线性约束", "Parameters,Bounds,Constraints and Weighting", "P0", TODO, "", "与 Origin 比较")
add(M, "加权拟合（误差列权重等）", "Parameters,Bounds,Constraints and Weighting", "P1", TODO, "", "与 Origin 比较")
add(M, "拟合报告（参数表、ANOVA、置信带、预测带）", "Interpreting Regression Results", "P0", TODO, "statrs 计算 t / F 分布", "参数、标准误、R² 与 Origin 差 < 1e-6")
add(M, "残差图（残差-自变量、正态概率图）", "Residual Plot Analysis", "P1", TODO, "", "与 Origin 比较")
add(M, "多峰拟合", "Multiple Curves/Multiple Peaks", "P1", TODO, "", "与 Origin 比较")
add(M, "全局拟合（多组数据共享参数）", "Multiple Curves/Multiple Peaks", "P1", TODO, "", "与 Origin 比较")
add(M, "用户自定义拟合函数", "Fitting Functions", "P0", TODO, "复用公式解析器，自动求导用数值差分或自动微分", "与 Origin 比较")
add(M, "隐函数拟合（圆、椭圆等 5 个）", "Nonlinear Implicit Curve Fitting", "P2", TODO, "正交距离回归", "与 Origin 比较")
add(M, "曲面拟合（22 个内置函数）", "Non-linear Surface Fitting", "P2", TODO, "", "与 Origin 比较")
add(M, "拟合比较（F 检验、AIC、BIC）", "Fit Comparison", "P1", TODO, "", "与 Origin 比较")
add(M, "标准曲线反查 X / Y", "Finding Y/X from X/Y Standard Curves", "P1", TODO, "", "与 Origin 比较")
add(M, "快速拟合（菜单一键）", "Quick Fit", "P0", PROTO, "", "手动操作")

FIT_GROUPS = [
    ("内置拟合函数：峰函数（Gauss、Lorentz、Voigt、PsdVoigt、LogNormal 等）", "P0"),
    ("内置拟合函数：指数类（ExpDec1-3、ExpGrow、ExpAssoc 等）", "P0"),
    ("内置拟合函数：S 形与生长（Boltzmann、Logistic、DoseResp、Hill、Gompertz 等）", "P0"),
    ("内置拟合函数：幂函数与对数（Power、Allometric、Log 等）", "P0"),
    ("内置拟合函数：多项式与有理函数（Poly、Rational 等）", "P0"),
    ("内置拟合函数：波形（Sine、SineDamp、Pulse、SquareWave 等）", "P1"),
    ("内置拟合函数：酶动力学（MichaelisMenten、各类抑制模型等）", "P1"),
    ("内置拟合函数：吸附与结合（Langmuir、Freundlich、BET、OneSiteBind 等）", "P1"),
    ("内置拟合函数：其余专业函数（流变、热力学、统计分布等）", "P2"),
]
for name, p in FIT_GROUPS:
    add(M, name, "Curve Fitting Functions（共 239 个）", p, TODO, "函数表达式 + 初值估计，数据驱动注册", "每个函数用 Origin 拟合结果做回归测试")
add(M, "寻峰分析专用函数（Bigaussian、FraserSuzuki 等 7 个）", "Peak Analyzer Functions", "P2", TODO, "", "与 Origin 比较")

# ---------------------------------------------------------------- 数学
M = "数学运算"
for cn, ref, p in [
    ("插值 / 外推（线性、三次样条、Akima）", "Interpolate/Extrapolate", "P0"),
    ("按 X 求 Y 插值", "Interpolate/Extrapolate Y from X", "P1"),
    ("轨迹插值", "Trace Interpolation", "P2"),
    ("XYZ 轨迹插值", "XYZ Trace Interpolation", "P2"),
    ("二维插值", "2D Interpolate/Extrapolate", "P2"),
    ("三维插值", "3D Interpolation", "P2"),
    ("数值积分（梯形法、曲线下面积）", "Integrate", "P0"),
    ("二维体积积分", "2D Volume Integrate (Pro Only)", "P2"),
    ("数值微分（含 Savitzky-Golay 平滑求导）", "Differentiate", "P0"),
    ("多条曲线平均 / 合并", "Average/Merge Multiple Curves", "P1"),
    ("简单列运算（加减乘除、两曲线相减）", "Simple Column/Curve Math", "P0"),
    ("归一化（最大值、Z 分数、区间缩放）", "Normalize", "P0"),
    ("多边形面积", "Polygon Area", "P2"),
    ("反函数（交换 X、Y 求逆）", "Inverse", "P2"),
    ("自定义滤波 / 中值滤波", "Median Filter", "P2"),
    ("扣除直线基线", "Subtract Straight Line", "P1"),
]:
    add(M, cn, ref, p, TODO, "自写，数值算法参照 Numerical Recipes / SciPy", "与 Origin 及 SciPy 结果比较")

# ---------------------------------------------------------------- 统计
M = "统计"
for cn, ref, p in [
    ("列统计（均值、标准差、中位数、分位数等）", "Statistics on Columns", "P0"),
    ("行统计", "Statistics on Rows", "P1"),
    ("整表统计", "Statistics on Whole Sheet", "P1"),
    ("频数统计 / 分箱", "Frequency Counts", "P0"),
    ("二维频数统计", "2D Frequency Counts/Binning", "P2"),
    ("正态性检验（Shapiro-Wilk、KS、Anderson-Darling 等）", "Normality Test", "P1"),
    ("相关系数（Pearson、Spearman、Kendall）", "Correlation Coefficient (Pro Only)", "P1"),
    ("偏相关系数", "Partial Correlation Coefficient (Pro Only)", "P2"),
    ("分布拟合", "Distribution Fit (Pro Only)", "P2"),
    ("列联表与卡方检验", "Cross Tabulation and Chi-square (Pro Only)", "P2"),
    ("Grubbs / Dixon Q 异常值检验", "Grubbs' Test", "P1"),
    ("单样本 t 检验", "One-Sample T-Test", "P0"),
    ("双样本 t 检验（含 Welch）", "Two-Sample T-Test", "P0"),
    ("配对 t 检验", "Pair-Sample T-Test", "P0"),
    ("方差检验（单样本、F 检验）", "Two Sample Test for Variances (Pro Only)", "P2"),
    ("比例检验、泊松率检验", "One-Sample Proportion Test (Pro Only)", "P2"),
    ("单因素 / 双因素 / 三因素方差分析（含多重比较）", "One, Two, and Three Way ANOVA", "P1"),
    ("重复测量方差分析", "Repeated Measure ANOVA (Pro Only)", "P2"),
    ("非参数检验（Wilcoxon、Mann-Whitney、Kruskal-Wallis、Friedman 等）", "Nonparametric Tests (Pro Only)", "P2"),
    ("生存分析（Kaplan-Meier、Cox 回归、Weibull）", "Survival Analysis (Pro Only)", "P2"),
    ("主成分分析 PCA", "Principal Component Analysis", "P2"),
    ("偏最小二乘 PLS", "Partial Least Squares", "P2"),
    ("聚类分析", "Cluster Analysis", "P2"),
    ("判别分析", "Discriminant Analysis", "P2"),
    ("机器学习（分类、回归）", "Machine Learning (Pro Only)", "P2"),
    ("功效与样本量计算", "Power and Sample Size (Pro Only)", "P2"),
    ("ROC 曲线", "ROC Curve (Pro Only)", "P2"),
]:
    how = "statrs 分布函数 + 自写检验" if p != "P2" else "statrs / linfa"
    add(M, cn, ref, p, TODO, how, "与 Origin 及 SciPy / R 结果比较（统计量、p 值）")

# ---------------------------------------------------------------- 信号处理
M = "信号处理"
for cn, ref, p, how in [
    ("平滑（移动平均、Savitzky-Golay、FFT 平滑、LOWESS）", "Smoothing", "P0", "自写"),
    ("FFT 滤波（低通、高通、带通、带阻）", "FFT Filters", "P1", "rustfft / realfft"),
    ("IIR 滤波（巴特沃斯、切比雪夫等）", "IIR Filters (Pro Only)", "P2", "自写或 biquad"),
    ("傅里叶变换 FFT / IFFT（幅值、相位、功率谱、窗函数）", "Fast Fourier Transform (FFT)", "P0", "rustfft"),
    ("短时傅里叶变换 STFT", "Short-Time Fourier Transform (Pro Only)", "P2", "rustfft"),
    ("卷积 / 反卷积", "Convolution", "P1", "rustfft"),
    ("相关（自相关、互相关）", "Correlation", "P1", "rustfft"),
    ("相干性", "Coherence (Pro Only)", "P2", "rustfft"),
    ("希尔伯特变换、包络", "Hilbert Transform (Pro Only)", "P2", "rustfft"),
    ("二维 FFT 与二维 FFT 滤波", "2D Fourier Transform (Pro Only)", "P2", "rustfft"),
    ("小波变换（连续、分解、重构、去噪）", "Wavelet Transforms (Pro Only)", "P2", "自写"),
    ("二维小波", "2D Wavelet Transforms", "P2", "自写"),
    ("二维相关", "2D Correlation (Pro Only)", "P2", "自写"),
    ("矩阵平滑", "Matrix Smoothing", "P2", "自写"),
    ("抽取 / 降采样", "Decimation (Pro Only)", "P2", "自写"),
]:
    add(M, cn, ref, p, TODO, how, "与 Origin 及 SciPy.signal 结果比较")

# ---------------------------------------------------------------- 峰分析
M = "峰分析"
for cn, p in [
    ("基线：锚点法、自动基线（ALS、多项式等）", "P1"),
    ("扣除基线", "P1"),
    ("寻峰（局部极值、导数法、阈值）", "P1"),
    ("峰积分（面积、半峰宽、质心）", "P1"),
    ("多峰拟合（重叠峰分峰）", "P1"),
]:
    add(M, cn, "Peak Analyzer", p, TODO, "复用拟合引擎", "与 Origin Peak Analyzer 结果比较")

# ---------------------------------------------------------------- 图像
M = "图像处理"
add(M, "导入图片到矩阵", "Importing Images and Videos", "P2", TODO, "image", "手动操作")
add(M, "图像调整（亮度、对比度、伽马）", "Image Adjustment", "P2", TODO, "image / imageproc", "与 Origin 比较")
add(M, "几何变换（旋转、裁剪、缩放）", "Geometric Transformations", "P2", TODO, "imageproc", "与 Origin 比较")
add(M, "空间滤波（边缘检测、锐化等）", "Spatial Filters", "P2", TODO, "imageproc", "与 Origin 比较")
add(M, "图像分析（粒子计数、剖面）", "Image Analysis", "P2", TODO, "imageproc", "与 Origin 比较")

# ---------------------------------------------------------------- 导出发布
M = "导出与发布"
add(M, "导出 SVG", "Exporting Your Graph or Layout Page to an Image File", "P0", DONE, "svg.rs", "目视比较 Origin 导出图")
add(M, "导出 PNG（指定 DPI）", "The Export Graph as Image Dialog Box", "P0", PROTO, "resvg / tiny-skia 渲染 SVG；网页原型已有", "像素尺寸检查")
add(M, "导出 PDF（矢量，嵌入字体）", "Which Export Format Should I Choose", "P0", TODO, "svg2pdf / krilla", "用 PDF 阅读器检查")
add(M, "导出 EPS", "Which Export Format Should I Choose", "P1", TODO, "自写 PostScript 输出", "LaTeX 编译检查")
add(M, "导出 TIFF（期刊要求）", "Which Export Format Should I Choose", "P1", TODO, "image", "尺寸与 DPI 检查")
add(M, "复制图形到剪贴板（粘贴到 Word / PPT）", "Pasting or Embedding Graphs in Other Applications", "P0", TODO, "arboard（位图）；矢量 EMF 待评估", "粘贴到 Word 检查")
add(M, "导出 EMF（Office 可编辑矢量）", "Which Export Format Should I Choose", "P1", TODO, "自写 EMF 写出器", "插入 Word 检查")
add(M, "批量导出多张图", "The Export Graph Dialog Box", "P1", TODO, "", "手动操作")
add(M, "打印", "Printing Graphs, Layout Pages, and Analytical Results", "P2", TODO, "导出 PDF 后交给系统打印", "手动操作")
add(M, "图形动画（导出视频 / GIF）", "Creating Graph Animation", "P2", TODO, "gif / ffmpeg", "手动操作")

# ---------------------------------------------------------------- 自动化与脚本
M = "自动化与编程"
add(M, "内置 Python 运行时（预装 numpy、scipy、pandas）", "Python Console", "P1", TODO, "Windows 用官方嵌入版，macOS / Linux 用 python-build-standalone，随安装包分发；脚本在独立进程中运行，崩溃不影响软件", "在没装 Python 的干净系统上安装后运行全部示例脚本")
add(M, "scicanvas Python 包（连接运行中的软件；离线模式直接调用核心库）", "Python Packages", "P1", TODO, "本机控制通道（带令牌）+ Arrow 批量传数据；离线模式用 PyO3 编译核心库", "脚本示例回归测试，与界面操作结果比较")
add(M, "脚本窗口（编辑器、运行 / 停止、输出面板、报错行高亮）", "The Origin Command Window and Script Window", "P1", TODO, "egui 代码编辑面板，调用内置 python.exe 子进程，实时显示输出", "运行、中途停止、报错定位到行")
add(M, "操作录制（把界面操作生成 Python 脚本，可回放）", "Batch Processing", "P1", TODO, "命令注册表记录命令序列，翻译成 scicanvas Python 调用", "录制 → 回放，结果与手动操作一致")
add(M, "Python 包管理（装包到内置环境，可切换到系统 Python / conda）", "Python Packages", "P1", TODO, "调用内置 pip；设置里选择解释器，脚本窗口标题显示当前环境", "安装、卸载、切换环境后运行示例")
add(M, "命令行批处理（无界面）", "Batch Processing", "P1", TODO, "scicanvas-cli，调用核心库", "脚本示例回归测试")
add(M, "分析模板（换数据自动重算）", "Analysis Templates", "P1", TODO, "分析操作记录为命令序列", "与 Origin 比较")
add(M, "MCP 服务器（让 AI 代理驱动软件）", "—", "P1", TODO, "参照 PhotoCraft：命令注册表暴露为 MCP 工具", "代理完成示例任务")
add(M, "插件系统", "Apps", "P2", TODO, "WebAssembly 插件（wasmtime）沙盒运行", "示例插件")
add(M, "R 控制台", "R Console and Rserve Console", "P2", TODO, "", "")
add(M, "MATLAB 控制台", "MATLAB Console", "不做", TODO, "", "")
add(M, "LabTalk 脚本兼容", "LabTalk Programming（1,018 页）", "不做", TODO, "改用 Python", "")
add(M, "Origin C 兼容", "Origin C（3,862 页）", "不做", TODO, "改用 Python / Rust 插件", "")
add(M, "X-Function 兼容（约 680 个）", "X-Function Reference", "不做", TODO, "功能按需在核心库实现，不做接口兼容", "")
add(M, "COM 自动化服务器", "Automation Server", "不做", TODO, "", "")
add(M, "LabVIEW 虚拟仪器", "Using Origin from LabVIEW", "不做", TODO, "", "")
add(M, "Mathematica Link", "Mathematica Link", "不做", TODO, "", "")
add(M, "Code Builder 集成开发环境", "The Code Builder", "不做", TODO, "", "")

# ---------------------------------------------------------------- 图类型
GT = {
    "2D Line Graphs": ("2D 线图", {
        "Line Graph": ("折线图", "P0"), "Horizontal Step Graph": ("水平阶梯图", "P1"), "Vertical Step Graph": ("垂直阶梯图", "P1"),
        "Stacked Lines by Y Offsets Graph": ("Y 偏移堆叠线图", "P1"), "Lollipop Plot": ("棒棒糖图", "P2"),
        "Colormapped Line Series": ("颜色映射线系列", "P2"), "Spline Graph": ("样条曲线图", "P1")}),
    "2D Scatter Graphs": ("2D 散点图", {
        "2D Scatter Graph": ("散点图", "P0"), "2D Scatter Central Graph": ("中心坐标散点图", "P2"), "Column Scatter": ("列散点图", "P1"),
        "Y Error Bar Graph": ("Y 误差棒图", "P0"), "XY Error Bar Graph": ("XY 误差棒图", "P0"), "Scatter+Rug Graph": ("散点 + 地毯图", "P2"),
        "Vertical Drop Line": ("垂线图", "P1"), "Indexed Size Bubble Graph": ("气泡图", "P1"), "Color Mapped Graph": ("颜色映射散点图", "P1"),
        "Indexed Size (Bubble) and Color Map Graph": ("气泡 + 颜色映射图", "P1"), "Error Band Graph": ("误差带图", "P1"),
        "Density Dots": ("密度点图", "P2"), "Color Dots": ("彩色点图", "P2")}),
    "2D Line Symbol Graphs": ("2D 点线图", {
        "Line+Symbol Graph": ("点线图", "P0"), "Line Series Graph": ("线系列图", "P1"), "2 Point Segment Graph": ("两点线段图", "P2"),
        "3 Point Segment Graph": ("三点线段图", "P2"), "Spline Connected Graph": ("样条连接点线图", "P1"), "Row Wise Graph": ("按行绘图", "P2"),
        "Before and After Graph": ("前后对比图", "P2")}),
    "2D Column and Bars Graphs": ("2D 柱状图与条形图", {
        "Column Graph": ("柱状图", "P0"), "Column + Label Graph": ("带标签柱状图", "P1"), "Bar Graph": ("条形图", "P0"),
        "Stacked Column Graph": ("堆叠柱状图", "P1"), "Floating Bar Graph": ("浮动条形图", "P2"), "Stacked Bar Graph": ("堆叠条形图", "P1"),
        "100% Stacked Column Graph": ("百分比堆叠柱状图", "P1"), "100% Stacked Bar Graph": ("百分比堆叠条形图", "P1"),
        "Floating Column Graph": ("浮动柱状图", "P2"), "100% Ribbon Chart": ("百分比丝带图", "P2"), "Ribbon Chart": ("丝带图", "P2"),
        "Circular Packing Graph": ("圆堆积图", "P2"), "Spiral Bar Chart": ("螺旋条形图", "P2"), "Frequency Chart": ("频数图", "P2")}),
    "Pie Charts": ("饼图", {
        "2D Color Pie Chart": ("2D 饼图", "P1"), "3D Color Pie Chart": ("3D 饼图", "P2"), "Doughnut Plot": ("环形图", "P2"),
        "Pie Of Pie Chart": ("复合饼图", "P2"), "Bar Of Pie Chart": ("复合条饼图", "P2"), "Doughnut Of Pie": ("复合环形图", "P2"),
        "Doughnut with Total Wedges = 100": ("百分比环形图", "P2"), "Pie with Different Radius Chart": ("不等半径饼图", "P2"),
        "Doughnut with Different Radius Chart": ("不等半径环形图", "P2"), "Sunburst Plot": ("旭日图", "P2")}),
    "Multi-Axis": ("多轴图", {
        "Double Y Graph": ("双 Y 轴图", "P0"), "Double-Y Column Graph": ("双 Y 轴柱状图", "P1"),
        "Double-Y Column Line+Symbol Graph": ("双 Y 轴柱状 + 点线图", "P1"), "Double X - Independent XY Graph": ("双 X 轴独立图", "P1"),
        "Double X - Custom Formula Graph": ("双 X 轴公式换算图", "P2"), "Y-Shared Double X Graph": ("共享 Y 的双 X 轴图", "P1"),
        "Double X - Arrhenius Graph": ("阿伦尼乌斯双 X 轴图", "P2"), "2Ys Y-Y Graph": ("2Ys Y-Y 图", "P1"), "2Ys Column": ("2Ys 柱状图", "P2"),
        "2Ys Column-Line Symbol": ("2Ys 柱状-点线图", "P2"), "3Ys Y-YY": ("3Ys Y-YY 图", "P2"), "3Ys Y-Y-Y": ("3Ys Y-Y-Y 图", "P2"),
        "4Ys Y-YYY": ("4Ys Y-YYY 图", "P2"), "4Ys YY-YY": ("4Ys YY-YY 图", "P2"), "Multiple Y Axes Graph": ("多 Y 轴图", "P1"),
        "Zoom Graph": ("放大图", "P2")}),
    "Multi-Panel": ("多面板图", {
        "Vertical 2 Panel Graph": ("上下两面板图", "P0"), "Horizontal 2 Panel Graph": ("左右两面板图", "P0"), "4 Panel Graph": ("四面板图", "P0"),
        "9 Panel Graph": ("九面板图", "P1"), "Stack Panels Graph": ("堆叠面板图", "P1"), "Multiple Panels Graph": ("多面板图", "P1"),
        "Inset Plot": ("插图（图中图）", "P1"), "Trellis": ("网格分面图", "P2"), "Trellis Double Y": ("双 Y 网格分面图", "P2"),
        "Cluster Plot": ("簇图", "P2")}),
    "Waterfall": ("瀑布图", {
        "2D Waterfall Graph": ("2D 瀑布图", "P1"), "2D Waterfall Y Color Mapping": ("2D 瀑布图（Y 颜色映射）", "P2"),
        "2D Waterfall Z Color Mapping": ("2D 瀑布图（Z 颜色映射）", "P2"), "3D Waterfall Graph": ("3D 瀑布图", "P2"),
        "3D Waterfall Y:Color Mapping": ("3D 瀑布图（Y 颜色映射）", "P2"), "3D Waterfall Z:Color Mapping": ("3D 瀑布图（Z 颜色映射）", "P2")}),
    "Statistics Graphs": ("统计图", {
        "2D Kernel Density Plot": ("二维核密度图", "P2"), "Bar Chart": ("均值条形图", "P2"), "Beeswarm Plot": ("蜂群图", "P2"),
        "Bland-Altman Plot": ("Bland-Altman 图", "P2"), "Box Chart Graph": ("箱线图", "P1"), "2Ys Box Chart Graph": ("双 Y 箱线图", "P2"),
        "Histogram/Distribution Graph": ("直方图 / 分布图", "P1"), "Histogram Probabilities Graph": ("概率直方图", "P2"),
        "Histogram + Rug/Distribution + Rug Graph": ("直方图 + 地毯图", "P2"), "Interval Plot": ("区间图", "P2"),
        "Multi-Panel Histogram Graph": ("多面板直方图", "P2"), "Parallel Index Plot": ("平行索引图", "P2"), "Parallel Plot": ("平行坐标图", "P2"),
        "Parallel Sets Plot": ("平行集图", "P2"), "Pareto Chart - Binned Data": ("帕累托图（分箱数据）", "P2"),
        "Pareto Chart - Raw Data": ("帕累托图（原始数据）", "P2"), "Population Pyramid Graph(Tornado Chart)": ("人口金字塔图（龙卷风图）", "P2"),
        "Probability Plot": ("概率图", "P2"), "Q-Q Plot": ("Q-Q 图", "P1"), "QC(x bar R) Chart Graph": ("质量控制图（X̄-R）", "P2"),
        "Dot Plot": ("点图", "P2"), "Stacked Dot Plots": ("堆叠点图", "P2"), "Scatter Matrix Graph": ("散点矩阵图", "P2"),
        "Stacked Histograms Graph": ("堆叠直方图", "P2"), "Multi-Vari Chart": ("多变异图", "P2"), "Variability Chart": ("变异性图", "P2"),
        "Violin Plot": ("小提琴图", "P1"), "Sankey Diagrams": ("桑基图", "P2"), "Alluvial Diagrams": ("冲积图", "P2"),
        "Network Plot": ("网络图", "P2"), "Chord Diagram/Chord Diagram with Ratio Layout": ("和弦图", "P2"),
        "Hierarchical Edge Bundling Diagram": ("层次边捆绑图", "P2"), "Marginal Box Charts": ("边际箱线图", "P2"),
        "Marginal Histograms Graph": ("边际直方图", "P2"), "Marginal Plots": ("边际图", "P2"), "Ridgeline Chart": ("山脊图", "P2"),
        "Symmetry Plot": ("对称图", "P2"), "Run Chart": ("运行图", "P2"), "CDF Plot": ("累积分布图", "P2"), "Interaction Plot": ("交互作用图", "P2"),
        "Main Effects Plot": ("主效应图", "P2"), "Cause-and-Effect Diagram": ("因果图（鱼骨图）", "P2")}),
    "Grouped Graphs": ("分组图", {
        "Grouped Scatter": ("分组散点图", "P1"), "Grouped Columns": ("分组柱状图", "P1"), "Grouped Histogram": ("分组直方图", "P2"),
        "Grouped Box Charts-Indexed Data": ("分组箱线图（索引数据）", "P1"), "Grouped Box Charts-Raw Data": ("分组箱线图（原始数据）", "P1"),
        "Grouped Column Scatter-Indexed Data": ("分组列散点图", "P2"), "Grouped Interval Plot-Indexed Data": ("分组区间图", "P2"),
        "Grouped Mean Bar-Indexed Data": ("分组均值条形图", "P2"), "Grouped Dot Plots": ("分组点图", "P2"),
        "Grouped Stacked Dot Plots": ("分组堆叠点图", "P2"), "Grouped Violin-Indexed Data": ("分组小提琴图", "P2"),
        "Grouped Half Violin-Indexed Data": ("分组半小提琴图", "P2"), "Grouped Half Box-Indexed Data": ("分组半箱线图", "P2"),
        "Grouped Floating Bar": ("分组浮动条形图", "P2")}),
    "Map": ("地图", {
        "Bar Map": ("条形地图", "P2"), "Pie Map Chart": ("饼图地图", "P2"), "Tile Grid Map": ("方格地图", "P2"), "Sankey Map": ("桑基地图", "P2"),
        "Stiff Map Diagram": ("Stiff 地图", "P2"), "Treemap": ("矩形树图", "P2")}),
    "Area Graphs": ("面积图", {
        "Area Graph": ("面积图", "P1"), "Stacked Area Graph": ("堆叠面积图", "P1"), "100% Stacked Area Graph": ("百分比堆叠面积图", "P2"),
        "Fill Area Graph": ("填充面积图", "P1"), "Fill Two Colors Graph": ("双色填充图", "P2"), "Kite Diagram": ("风筝图", "P2")}),
    "Specialized Graphs": ("专业图", {
        "Window-view Browser Graph": ("窗口浏览图", "P2"), "Bullet Chart": ("子弹图", "P2"), "Polar Contour + Gradient Vector": ("极坐标等高线 + 梯度矢量", "P2"),
        "Contour Line + Gradient Vector": ("等高线 + 梯度矢量", "P2"), "Contour + Streamline": ("等高线 + 流线", "P2"),
        "Piper/Trilinear Diagram": ("Piper 三线图", "P2"), "Durov Diagram": ("Durov 图", "P2"), "Stiff Diagram": ("Stiff 图", "P2"),
        "Polar Graph": ("极坐标图", "P1"), "Polar Vector theta r theta r graph": ("极坐标矢量图（θrθr）", "P2"), "Compass Plot": ("罗盘图", "P2"),
        "Polar Vector ?RAM graph": ("极坐标矢量图（θrAM）", "P2"), "Radar Chart (Spider Plot)": ("雷达图", "P2"),
        "Radial Bar and Radial Stacked Bar Plot": ("径向条形图", "P2"), "Right-Triangle Ternary": ("直角三元图", "P2"),
        "Smith Charts": ("史密斯圆图", "P2"), "Stacked Radial Plot": ("堆叠径向图", "P2"), "Streamline Plot": ("流线图", "P2"),
        "Ternary": ("三元图", "P2"), "Ternary Vector": ("三元矢量图", "P2"), "Wind Rose-Binned Data": ("风玫瑰图（分箱数据）", "P2"),
        "Wind Rose-Raw Data": ("风玫瑰图（原始数据）", "P2"), "X, Y, Angle, Magnitude Vector Graph": ("矢量图（XY + 角度 + 大小）", "P2"),
        "X, Y, X, Y Vector Graph": ("矢量图（起点 XY + 终点 XY）", "P2")}),
    "Financial Graphs": ("金融图", {
        "Bridge Charts": ("桥图（瀑布式）", "P2"), "High-Low-Close Charts": ("高低收图", "P2"), "Japanese_Candlestick": ("K 线图", "P2"),
        "OHLC Bar Chart": ("OHLC 条形图", "P2"), "OHLC-Volume": ("OHLC + 成交量图", "P2"), "Line (Stock)": ("股票折线图", "P2")}),
    "3D XYY Graphs": ("3D XYY 图", None),
    "3D Surfaces": ("3D 曲面图", None),
    "3D Symbol/Bar/Vector": ("3D 散点 / 柱 / 矢量图", None),
    "Contour Graphs": ("等高线与热图", {
        "Color Fill Contour": ("填色等高线图", "P1"), "Black and White Lines with Labels Contour": ("黑白等高线图（带标签）", "P1"),
        "Gray Scale Map Contour": ("灰度等高线图", "P2"), "Contour - Categorical Z": ("分类 Z 等高线图", "P2"), "Heatmap": ("热图", "P1"),
        "Polar Heatmap": ("极坐标热图", "P2"), "Heatmap with Labels": ("带标签热图", "P1"), "Split Heatmap": ("分割热图", "P2"),
        "Heatmap with Split Tiles": ("分块热图", "P2"), "Polar Contour from XYZ Data": ("极坐标等高线（XYZ 数据）", "P2"),
        "Polar Contour from Matrix Data": ("极坐标等高线（矩阵数据）", "P2"), "Ternary Contour": ("三元等高线图", "P2"),
        "Overlaid Contour Plot": ("叠加等高线图", "P2")}),
    "Image and Profiles": ("图像与剖面", {
        "Contour Profiles": ("等高线剖面图", "P2"), "Image as Background": ("图片背景", "P2"), "Image Plot": ("图像图", "P2"),
        "Image Profiles": ("图像剖面图", "P2")}),
}
THREE_D = {
    "XYY 3D Bar Graph": "XYY 3D 柱状图", "XYY 3D 100% Stacked Bars Graph": "XYY 3D 百分比堆叠柱", "XYY 3D Side by Side Bars": "XYY 3D 并列柱",
    "XYY 3D Stacked Bars Graph": "XYY 3D 堆叠柱", "XYY 3D Bar Graph with Error Bar": "XYY 3D 柱（带误差棒）", "3D Ribbons Graph": "3D 丝带图",
    "3D 100% Stacked Wall Graph": "3D 百分比堆叠墙图", "3D Stacked Wall Graph": "3D 堆叠墙图", "3D Wall Graph": "3D 墙图",
    "Color Fill Surface": "填色曲面图", "X Constant with Base Surface": "X 恒定带底曲面", "Y Constant with Base Surface": "Y 恒定带底曲面",
    "Color Map Surface": "颜色映射曲面图", "Color Map Surface with Projection": "颜色映射曲面（带投影）", "3D Ternary Colormap Surface": "3D 三元颜色映射曲面",
    "Wire Frame": "线框图", "Wire Surface": "线框曲面", "Color Fill Surface with Error Bar": "填色曲面（带误差棒）",
    "Colormap Surface with Error Bar": "颜色映射曲面（带误差棒）", "Multiple Color Fill Surfaces": "多个填色曲面", "Multiple Color Map Surfaces": "多个颜色映射曲面",
    "Parametric Surface": "参数曲面", "3D Stacked Surfaces": "3D 堆叠曲面", "3D Stacked Heatmaps": "3D 堆叠热图",
    "3D Scatter Graph": "3D 散点图", "3D Scatter with Z Error Bar Graph": "3D 散点（Z 误差棒）", "3D Line Graph": "3D 线图", "Trajectory Graph": "轨迹图",
    "3D Bar Graph": "3D 柱状图", "3D Bar Graph with Categorical/Text XY": "3D 柱状图（分类 XY）", "3D Stacked Bars Graph": "3D 堆叠柱状图",
    "3D 100% Stacked Bar Graph": "3D 百分比堆叠柱状图", "3D Floating Bar Graph": "3D 浮动柱状图", "3D Vector XYZ XYZ": "3D 矢量（XYZ XYZ）",
    "3D Vector XYZ dXdYdZ": "3D 矢量（XYZ dXdYdZ）", "3D Ternary Symbol": "3D 三元散点", "3D Tetrahedral(Quaternary Phase Diagram)": "四面体图（四元相图）",
}
PROTO_GRAPHS = {"Horizontal Step Graph", "Vertical Step Graph"}
ERROR_GRAPHS = {"Y Error Bar Graph", "XY Error Bar Graph"}
BAR_GRAPHS = {"Stacked Column Graph", "100% Stacked Column Graph", "Floating Column Graph"}
FILL_GRAPHS = {"Bar Graph", "Area Graph"}
SPLINE_GRAPHS = {"Spline Graph", "Spline Connected Graph", "Vertical Drop Line", "Column + Label Graph"}
DONE_GRAPHS = {"Line Graph", "2D Scatter Graph", "Line+Symbol Graph", "Column Graph"}
P1_3D = {"Color Map Surface", "3D Scatter Graph"}


def graph_how(cat, en):
    if cat.startswith("3D") or "3D" in en:
        return "wgpu 3D 渲染（three-d 或自写）"
    if cat in ("Contour Graphs", "Image and Profiles"):
        return "contour crate（marching squares）+ 色阶"
    if cat == "Map":
        return "geojson / shapefile"
    return "2D 渲染器 + SVG 导出"


def load_graph_names():
    """从帮助目录读取每一类下的图类型名称（按目录顺序）。"""
    import glob
    import html
    import re
    import sys

    chm = Path(sys.argv[1]) if len(sys.argv) > 1 else None
    if not chm:
        return None
    hhc = glob.glob(str(chm / "Origin" / "**" / "*.hhc"), recursive=True)[0]
    t = open(hhc, encoding="utf-8", errors="ignore").read()
    depth, items = 0, []
    for m in re.finditer(r'(<UL>|</UL>|<param name="Name" value="([^"]*)")', t, re.I):
        g = m.group(1).upper()
        if g == "<UL>":
            depth += 1
        elif g == "</UL>":
            depth -= 1
        else:
            items.append((depth, html.unescape(m.group(2))))
    out, cat, inside = {}, None, False
    for d, n in items:
        if n == "Appendix 2 - Graph Types":
            inside, base = True, d
            continue
        if inside:
            if d <= base:
                break
            if d == base + 1:
                cat = n
                out[cat] = []
            elif d == base + 2:
                out[cat].append(n)
    return out


names = load_graph_names()
for cat, (cn_cat, table) in GT.items():
    ens = names[cat] if names else list((table or {}).keys())
    for en in ens:
        if table is not None:
            cn, p = table.get(en, (en, "P2"))
        else:
            cn, p = THREE_D.get(en, en), ("P1" if en in P1_3D else "P2")
        status = DONE if en in DONE_GRAPHS else PROTO if en in PROTO_GRAPHS | ERROR_GRAPHS | BAR_GRAPHS | FILL_GRAPHS | SPLINE_GRAPHS else TODO
        how, verify = graph_how(cat, en), "与 Origin 同数据出图目视比较"
        if en in PROTO_GRAPHS:
            how, verify = "阶梯折线图元 + SVG 共享排版", "拐点几何、拟合线与项目往返测试；Origin 目视对照待做"
        elif en in ERROR_GRAPHS:
            how, verify = "对称误差列自动关联 + scene/SVG 共享图元", "关联、端点几何、裁剪和撤销回归；Origin 目视对照待做"
        elif en in BAR_GRAPHS:
            how, verify = "正负堆叠/百分比归一化/上下端配对 + scene/SVG", "缺失行、正负值、上下端、矩形几何、误差棒和撤销回归；Origin 目视对照待做"
        elif en in FILL_GRAPHS:
            how, verify = "条形方向与误差换向 / 分段面积填充 + scene/SVG", "轴方向、跨零面积、缺口、透明度、SVG 和撤销回归；Origin 目视对照待做"
        elif en in SPLINE_GRAPHS:
            how, verify = "自然三次样条 / 零基线垂线 / 数据标签 + scene/SVG", "SciPy 数值对照、分段、SVG、标签和撤销回归；Origin 目视对照待做"
        add("图类型 · " + cn_cat, cn, en, p, status, how, verify)

# 网页原型已实现、Rust 版待移植的条目
PROTO_FEATURES = ("数值积分", "数值微分", "归一化", "列统计", "平滑（移动平均")
for r in rows:
    if r["status"] == TODO and r["feature"].startswith(PROTO_FEATURES):
        r["status"] = PROTO

# ---------------------------------------------------------------- 输出
for i, r in enumerate(rows, 1):
    r["id"] = f"F{i:03d}"
    r["milestone"] = MILESTONE[r["pri"]]
    if r["pri"] == "不做":
        r["status"] = SKIP

here = Path(__file__).parent
(here / "parity.json").write_text(json.dumps(rows, ensure_ascii=False, indent=1), encoding="utf-8")

from collections import Counter

by_pri = Counter(r["pri"] for r in rows)
by_status = Counter(r["status"] for r in rows)
md = [
    "# SciCanvas 功能对照清单",
    "",
    "条目按 Origin 帮助文档的目录整理，“Origin 参考”一列是对应的帮助章节标题，方便查阅原文档。",
    "本清单只记录功能点，不包含 Origin 文档正文。实现时按公开说明和观察到的行为编写，不复制 Origin 的代码、文字或图片。",
    "",
    f"共 {len(rows)} 项：" + "，".join(f"{k} {by_pri[k]} 项" for k in ["P0", "P1", "P2", "不做"]) + "。",
    "状态：" + "，".join(f"{k} {by_status[k]} 项" for k in [DONE, PROTO, TODO, SKIP]) + "。",
    "",
    "- **P0 / M1 日常够用**：一个人做日常数据处理和论文作图需要的最小集合。",
    "- **P1 / M2 论文全流程**：拟合报告、统计检验、多面板、信号处理等，覆盖大部分论文工作。",
    "- **P2 / M3 高级功能**：3D、专业图、Pro 版统计、图像处理等。",
    "- **不做**：脚本兼容（LabTalk、Origin C、X-Function）、Origin 私有工程格式、与特定外部软件的连接。",
    "- **状态“原型”**：网页原型已实现，Rust 版待移植或部分完成。",
    "",
    "验证原则：分析结果与 Origin 及 NIST StRD 认证数据集、SciPy / R 交叉比对；图形与 Origin 同数据出图目视比较。",
    "",
    "安装包体积估算：全部功能完成后约 100–130 MB。其中软件本体（含 3D、PDF / Excel、图像处理、模板与帮助）压缩后约 20–40 MB，内置 Python 运行时及 numpy、scipy、pandas 压缩后约 60–90 MB。",
    "",
]
cur = None
for r in rows:
    if r["module"] != cur:
        cur = r["module"]
        md += ["", f"## {cur}", "", "| 编号 | 功能 | Origin 参考 | 优先级 | 状态 | 实现方案 | 验证方法 |", "|---|---|---|---|---|---|---|"]
    md.append(
        f"| {r['id']} | {r['feature']} | {r['ref']} | {r['pri']} | {r['status']} | {r['how'] or '—'} | {r['verify'] or '—'} |"
    )
(here / "PARITY.md").write_text("\n".join(md) + "\n", encoding="utf-8")
print(len(rows), dict(by_pri), dict(by_status))
