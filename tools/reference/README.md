# 开发机参考值

Windows / Python 3.14。CI 和 release exe 均不安装这里的科学计算包。

```powershell
python -m venv .venv-ref
.venv-ref/Scripts/python -m pip install -r tools/reference/requirements-reference.txt
.venv-ref/Scripts/python tools/reference/gen_scipy.py
```

结果位于 `crates/scicanvas-core/tests/reference/*.json`。每个文件带生成器与 Python/NumPy/SciPy 版本，固定种子 20261101，输入完整保存。非有限参考值写成 JSON null。再次运行应逐字节相同（使用同一 Python 版本与固定依赖）。

| 条目 | 数据 | 口径 |
|---|---|---|
| F164 | integrate.json | NumPy 总面积、SciPy 累积梯形；缺失行成对过滤，结果恢复原行；跨零点分段积分绝对值 |
| F169 | normalize.json | NumPy 最大值/区间缩放；SciPy zscore(ddof=1)，与现有样本标准差口径相同 |
| F174 | describe.json | NumPy mean/std(ddof=1)/median/linear quantile/min/max/sum；SciPy sem |
| F168 | column_math.json | NumPy 逐元素加减乘除与 interp，范围外/除零留空 |
| F131 | polynomial.json | 标准化 X 后 NumPy polynomial.polyfit，SVD 协方差换回原 X；同时核对系数、标准误、RSS、稳定求值 |

默认相对误差 1e-12（参考为零时绝对误差），统计量 1e-10。六阶两组用例使用 5e-12，记录在 JSON 的 tolerance/tolerance_reason：现有标准化正规方程相对 NumPy 解的系数舍入差为 2.56e-12。X 偏移为 1e6 时仍在标准化坐标求参考解，避免把原量纲病态设计矩阵的舍入误差当成认证值。Z 分数使用 ddof=1，区别于 SciPy 默认的 ddof=0。

依据：NumPy/SciPy 的对应公开 API。输入与输出由本仓库脚本生成，旧的手写参考数字测试继续保留。

微分参考中，大基线样例记录并减去 `input_shift` 后计算导数，以减少参考结果的消减误差。接近零时使用 1e-10 绝对误差下限。SciPy 的含缺失窗口会报错，测试单独检查程序的非数值结果。

约束拟合参考通过消元处理固定参数及活动约束，并在大基线样例中减去固定常数；线性约束参考使用 1e-13 停止容差。

内置拟合参考从已知参数出发，使用 SciPy 拟合；大基线样例记录并减去 `input_shift` 后计算参考值，再还原参数。

平滑参考先按实际操作跳过缺失行；余下 X 非等距时 FFT 平滑拒绝执行，Savitzky–Golay 参考用实际 X 拟合窗口。百分比堆叠参考分别归一正负值，零值保留当前正向堆叠高度。

拟合报告参考包含方差分析、参数检验和置信/预测带。大基线样例记录并减去常数；非线性极小 p 值实测最大相对差 1.50e-6，参考文件记录 1e-5 容差及原因。
