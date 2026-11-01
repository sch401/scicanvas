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
