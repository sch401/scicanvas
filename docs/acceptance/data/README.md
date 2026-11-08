# 第二批验收数据

生成：在仓库根目录运行 `python tools/generate_acceptance_data.py`。依赖为 NumPy 2.4.4、SciPy 1.17.1（版本见 tools/reference/requirements-reference.txt）。脚本只写本目录的 CSV 和 expected.json，不改示例或既有参考。数据来自确定公式和固定正弦扰动，或逐值提取既有 SciPy 参考，没有随机数。

## 导入

在 SciCanvas 按 Ctrl+K 执行“导入数据”，选择 CSV，首行作为列名。各清单写明 A/B/C 的列类型，按条目设置后再选源列。空白必须保留为空；interpolation 的 target_X 第 2 行是非有限目标，输出对应行留空。不同长度的信号/核补空白，不把空白当成零。

完整机器可读参考在 [expected.json](expected.json)，人工比对值和容差在各验收表。冻结参考来自 `crates/scicanvas-core/tests/reference/` 的 builtin_fits.json、square_wave.json、inhibition_fits.json、peak_sums.json；脚本复用其中的数据和 SciPy 参数。其余数值分别用 NumPy convolve/interp/quantile、SciPy signal.deconvolve/find_peaks/savgol_filter、stats.shapiro/norm.ppf 和 optimize.curve_fit 计算。

反卷积采用核 [1,0.5]，它适合 ε=0 的频域除法；[1,1] 的 Nyquist 频响为零，仅用于 Full/Same/Valid 卷积验收。反卷积参考前三行恢复 [1,2,3]，第 4 行为补零。

## 条目与数据

| 条目 | 数据或示例 |
|---|---|
| F206 | [data/convolution.csv](convolution.csv) 和 [data/deconvolution.csv](deconvolution.csv)。两列都设为 Y，选择顺序为信号、核，空白保留。 |
| F159 | [data/interpolation.csv](interpolation.csv)，A 设 X、B/C 设 Y；只选择 B 作为源，C 为目标 X 列，C 的第 2 行留空（表示非有限目标）。Y=2X+1。 |
| F167 | [data/curve_merge.csv](curve_merge.csv)，A/C 设 X、B/D 设 Y，只选 B、D。X1=[0,2,4]，X2=[1,2,5]；合并时 X=2 重复两次。 |
| F216 | [data/baseline_peaks.csv](baseline_peaks.csv)（A=X，B=Y）和 [data/baseline_polynomial.csv](baseline_polynomial.csv)。第一份是 2+0.3X 加中心 3、7 的两个峰和固定小扰动；第二份是纯直线。 |
| F218 | [data/peak_detection.csv](peak_detection.csv)，A=X，B=Y，步长 0.02，峰中心 2/5/8，标准差 0.18，高度 1/2/3，固定扰动不超过 1e-6。 |
| F176 | [data/statistics.csv](statistics.csv)。A/B 都设 Y，A=1–10，B 的第 5 行为空。 |
| F506 | [data/statistics.csv](statistics.csv)，A/B 均设 Y，同时选 A、B 全部 10 行。 |
| F505 | [data/statistics.csv](statistics.csv)，A/B 均设 Y，只选 A 第 1–10 行。 |
| F153 | 五个 wave_*.csv，文件逐行见下表；列角色 A=X、B=Y。数据原样摘自已有 SciPy 参考，未重新计算既有参考文件。 |
| F154 | inhibition_*.csv 三份，A/B、C/D、E/F 为三组独立 X/Y，三组 I=0/1/4。 |
| F155 | binding_*.csv 三份，X、Y 原始数据见下表。 |
| F140 | [data/multipeak.csv](multipeak.csv)，A=X/B=Y；高斯分量参数顺序为每峰 xc,w,A，末尾为线性基线截距、斜率。A 是面积，不是峰高。 |
| F220 | [data/overlapping_peaks.csv](overlapping_peaks.csv)，A=X/B=Y；高斯分量参数顺序为每峰 xc,w,A，末尾为线性基线截距、斜率。A 是面积，不是峰高。 |
| F101 | `samples/25-颜色映射散点图.scv`，点击图形后打开“绘图属性”。 |
| F269 | `samples/25-颜色映射散点图.scv`。原 C=1–6，B 的响应值为2,4,3,6,4,5。 |
| F295 | `samples/30-饼图.scv`，A 是类别，B 为唯一正权重列。 |
| F363 | `samples/31-小提琴图.scv`，B/C 为两组9个值。 |
| F413 | `samples/32-极坐标线图.scv` 和 `samples/33-极坐标散点图.scv`。A为0,45,…,360度，B为2,3,4,3,2,1,2,3,2。 |
| F085 | `samples/01-折线图.scv` 可保留作对照，函数本身不需要工作表。 |
| F090 | `samples/01-折线图.scv`；再打开一次作目标，先把目标标题改“格式目标”、X手动范围0到60。 |
| F091 | `samples/01-折线图.scv`，先设轴线2 pt、曲线线宽2、符号大小6，标题保持原文。 |
| F114 | `samples/01-折线图.scv`；切成点线图方便数点，B/C各是一条曲线。 |
| F094 | 四份 axis_*.csv，分别见下表。新建一张图完成一种后再打开下一份，避免混用单位。 |
| F275 | `samples/29-线系列图.scv`，至少选择两个 Y 列；图例默认隐藏时点击“图例”打开。 |

F515 使用 samples/01-折线图.scv，按 plotting.md 的坐标轴外观和刻度步骤操作。显示类检查使用已有示例和页面位置/颜色说明，不为其虚构拟合数值。
