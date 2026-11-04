"""开发机运行：.venv-ref/Scripts/python tools/reference/gen_matplotlib.py。

先显式运行 cargo test --features serde -p scicanvas-core --test visual_cases -- --ignored。
仅复用原始列数据、轴范围和刻度值。样条按指南在 SciCanvas 的采样 X 上计算 SciPy 自然样条 Y。
"""
from datetime import datetime, timezone
import json
from pathlib import Path
import platform
import sys

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import dates, colors
import numpy as np
import scipy
from scipy.interpolate import BSpline, BPoly, CubicSpline

matplotlib.rcParams.update({"font.family": ["Arial", "Microsoft YaHei"], "font.size": 7,
                           "axes.linewidth": .6, "lines.linewidth": 1, "savefig.dpi": 300,
                           "figure.facecolor": "none", "axes.facecolor": "none", "savefig.transparent": True})
ROOT = Path(__file__).resolve().parents[2]
DIRECTORY = ROOT / "docs/parity/visual"
VERSIONS = {"Python": platform.python_version(), "NumPy": np.__version__, "SciPy": scipy.__version__, "matplotlib": matplotlib.__version__}


def runs(points):
    result, current = [], []
    for p in points:
        if np.isfinite(p).all():
            current.append(p)
        else:
            if current:
                result.append(np.asarray(current))
            current = []
    if current:
        result.append(np.asarray(current))
    return result


def generate(source):
    doc = json.loads(source.read_text(encoding="utf-8"))
    columns = {c["key"]: c for c in doc["data"]}
    raw = lambda key: np.asarray(columns[key]["values"], dtype=float)
    x_column = columns["A"]
    date = x_column["kind"] == "DateTime"
    category = x_column["category"]
    def xconvert(values):
        values = np.asarray(values, dtype=float)
        if date:
            return np.asarray([dates.date2num(datetime.fromtimestamp(v/1000, timezone.utc)) for v in values])
        if category:
            return values-1
        return values
    if category:
        x = np.asarray(category["codes"], dtype=float)+1
    else:
        x = raw("A")
    page = doc["page"]
    fig = plt.figure(figsize=(page["w"]/96, page["h"]/96))
    output = dict(case=doc["case"], page=page, data=doc["data"], versions=VERSIONS, layers=[])
    for layer in doc["layers"]:
        left, top, width, height = layer["plot"]
        ax = fig.add_axes([left/page["w"], 1-(top+height)/page["h"], width/page["w"], height/page["h"]])
        axes = layer["axes"]
        if axes["x"]["log"]:
            ax.set_xscale("log")
        if axes["y"]["log"]:
            ax.set_yscale("log")
        ax.set_xlim(xconvert([axes["x"]["lo"], axes["x"]["hi"]]))
        ax.set_ylim(axes["y"]["lo"], axes["y"]["hi"])
        right = None
        if axes["right_y"]:
            right = ax.twinx()
            right.set_ylim(axes["right_y"]["lo"], axes["right_y"]["hi"])
            if axes["right_y"]["log"]:
                right.set_yscale("log")
        if date:
            ax.xaxis.set_major_formatter(dates.DateFormatter("%m-%d", tz=timezone.utc))
        if category:
            ax.set_xticks(np.arange(len(category["levels"])), category["levels"])
        def transform(points, target=ax, convert_x=True):
            pts = np.array(points, dtype=float, copy=True).reshape(-1, 2)
            if convert_x:
                pts[:, 0] = xconvert(pts[:, 0])
            return target.transAxes.inverted().transform(target.transData.transform(pts)).tolist()
        def artist_points(line, target=ax):
            return target.transAxes.inverted().transform(line.get_transform().transform(line.get_path().vertices))
        marks = dict(symbols=[], polylines=[], bars=[], segments=[], area=[], colors=[])
        for name, a in axes.items():
            if a is None:
                continue
            target = right if name == "right_y" else ax
            for kind in ("ticks", "minor"):
                for tick in a[kind]:
                    value = tick["value"]
                    if name == "x":
                        tick["position"] = transform([[value, axes["y"]["lo"]]])[0][0]
                    else:
                        tick["position"] = transform([[axes["x"]["lo"], value]], target)[0][1]
        kind = layer["kind"]
        horizontal = kind == "HorizontalBar"
        keys = layer["y_keys"]
        arrays = [raw(k) for k in keys]
        sums = np.zeros((2, len(x)))
        totals = np.zeros((2, len(x)))
        for y in arrays:
            totals[0] += np.where(np.isfinite(y) & (y >= 0), y, 0)
            totals[1] += np.where(np.isfinite(y) & (y < 0), -y, 0)
        for si, (key, y) in enumerate(zip(keys, arrays)):
            target = right if key in layer["right_keys"] else ax
            valid = np.isfinite(x) & np.isfinite(y)
            if layer["rows"] is not None:
                lo, hi = layer["rows"]
                valid &= (np.arange(len(x)) >= lo) & (np.arange(len(x)) <= hi)
            if kind == "Histogram":
                # 与 SciCanvas 相同：各列共用区间，范围取全部有效值的最小、最大值，区间数按最长一列用 Sturges 规则。
                pools = [yy[np.isfinite(x) & np.isfinite(yy)] for yy in arrays]
                pooled = np.concatenate(pools)
                bins = int(np.ceil(np.log2(max(len(v) for v in pools)))) + 1
                counts, edges = np.histogram(y[valid], bins=bins, range=(pooled.min(), pooled.max()))
                patches = target.bar((edges[:-1]+edges[1:])/2, counts, width=np.diff(edges))
                for patch in patches:
                    a, b = target.transAxes.inverted().transform(patch.get_data_transform().transform(patch.get_bbox().get_points()))
                    marks["bars"].append([(a[0]+b[0])/2, min(a[1], b[1]), max(a[1], b[1])])
                continue
            if kind in ("Bar", "HorizontalBar", "StackedBar", "PercentStackedBar", "FloatingBar"):
                base = np.zeros(len(x))
                values = y.copy()
                if kind == "FloatingBar":
                    base = raw(layer["base_by_y"][key])
                    valid &= np.isfinite(base)
                elif kind in ("StackedBar", "PercentStackedBar"):
                    for i in np.flatnonzero(valid):
                        sign = int(y[i] < 0)
                        value = y[i]
                        if kind == "PercentStackedBar":
                            value = value/totals[sign, i]*100 if totals[sign, i] else 0
                        base[i] = sums[sign, i]
                        sums[sign, i] += value
                        values[i] = sums[sign, i]
                spacing = np.min(np.diff(np.unique(x)))
                offset = .7*spacing*((si+.5)/len(keys)-.5) if kind in ("Bar", "HorizontalBar", "FloatingBar") else 0
                center = x[valid]+offset
                if horizontal:
                    patches = target.barh(center, values[valid]-base[valid], left=base[valid], height=.7*spacing/len(keys))
                else:
                    patches = target.bar(xconvert(center), values[valid]-base[valid], bottom=base[valid], width=.7*spacing/len(keys))
                for patch in patches:
                    # 从实际 Matplotlib Rectangle 读取中心与上下端；宽度属于风格，不纳入比较。
                    a, b = target.transAxes.inverted().transform(patch.get_data_transform().transform(patch.get_bbox().get_points()))
                    if horizontal:
                        marks["bars"].append([min(a[0], b[0]), max(a[0], b[0]), (a[1]+b[1])/2])
                    else:
                        marks["bars"].append([(a[0]+b[0])/2, min(a[1], b[1]), max(a[1], b[1])])
                continue
            points = np.column_stack([x, y])
            points[~valid] = np.nan
            paths = runs(points)
            if kind == "VerticalDrop":
                collection = target.vlines(xconvert(x[valid]), 0, y[valid])
                for segment in collection.get_segments():
                    marks["segments"].append(sorted(transform(segment, target, convert_x=False)))
            elif kind == "Area":
                for run in paths:
                    if len(run) < 2:
                        continue
                    target.fill_between(xconvert(run[:, 0]), 0, run[:, 1], alpha=.35)
                    boundary = list(run)
                    for (xa, ya), (xb, yb) in zip(run[:-1], run[1:]):
                        if ya*yb < 0:
                            boundary.append(np.array([xa-ya*(xb-xa)/(yb-ya), 0]))
                    for xx, yy in boundary:
                        a, b = transform([[xx, 0], [xx, yy]], target)
                        marks["area"].append([a[0], min(a[1], b[1]), max(a[1], b[1])])
                    line, = target.plot(xconvert(run[:, 0]), run[:, 1])
                    marks["polylines"].append(artist_points(line, target).tolist())
                marks["area"].sort(key=lambda v: v[0])
            elif kind != "Scatter":
                for run in paths:
                    if len(run) < 2:
                        continue
                    xx, yy = run.T
                    if kind in ("Spline", "SplineSymbol"):
                        # 只从已提交场景取采样 X，Y 完全由 SciPy 和原始数据计算。
                        actual = layer["marks"]["polylines"][len(marks["polylines"])]
                        xx = np.array([p[0] for p in actual])
                        xx = target.transData.inverted().transform(target.transAxes.transform(np.column_stack([xx, np.zeros(len(xx))])))[:, 0]
                        yy = CubicSpline(run[:, 0], run[:, 1], bc_type="natural")(xx)
                    elif layer["connect"] == "BSpline":
                        n = len(run)
                        knots = np.r_[np.zeros(4), np.arange(1, n-3), np.repeat(n-3, 4)]
                        u = np.arange((n-3)*16+1)/16
                        xx, yy = BSpline(knots, run, 3)(u).T
                    elif layer["connect"] == "Bezier":
                        output_points = [run[0]]
                        for start in range(0, len(run)-1, 3):
                            control = run[start:min(start+4, len(run))]
                            output_points.extend(BPoly(control[:, None, :], [0, 1])(np.arange(1, 17)/16))
                        xx, yy = np.asarray(output_points).T
                    if kind in ("StepHorizontal", "StepVertical"):
                        line, = target.step(xconvert(xx), yy, where="post" if kind == "StepHorizontal" else "pre")
                    else:
                        line, = target.plot(xconvert(xx), yy)
                    marks["polylines"].append(artist_points(line, target).tolist())
            if kind in ("Scatter", "LineSymbol", "SplineSymbol", "VerticalDrop"):
                if doc["case"] == "color-size-shape-map":
                    normalize = colors.Normalize(vmin=0, vmax=1)
                    palette = colors.LinearSegmentedColormap.from_list("scicanvas-five-node-viridis", np.array([[68, 1, 84], [59, 82, 139], [33, 145, 140], [94, 201, 98], [253, 231, 37]])/255, N=257)
                    rgba = palette(normalize(raw("C")[valid]))
                    codes = np.asarray(columns["D"]["category"]["codes"])[valid]
                    sizes = 2+8*normalize(raw("C")[valid])
                    for xx, yy, color, code, size in zip(xconvert(x[valid]), y[valid], rgba, codes, sizes):
                        scatter = target.scatter([xx], [yy], c=[color], s=(size*72/96)**2, marker=["o", "s", "^"][code])
                        marks["symbols"].extend(target.transAxes.inverted().transform(scatter.get_offset_transform().transform(scatter.get_offsets())).tolist())
                    marks["colors"].extend(np.floor(rgba[:, :3]*255+.5).astype(int).tolist())
                elif doc["case"] == "symbol-fill-border":
                    # 只比较位置与内部填充；边框颜色和线宽只在并排图中展示。
                    fill = None if key == "B" else [31, 119, 180]
                    scatter = target.scatter(
                        xconvert(x[valid]), y[valid], s=36,
                        facecolors="none" if fill is None else [np.array(fill)/255],
                        edgecolors=[(200/255, 30/255, 30/255)] if fill is None else ["black"],
                        linewidths=1.2 if fill is None else .6)
                    marks["symbols"].extend(target.transAxes.inverted().transform(scatter.get_offset_transform().transform(scatter.get_offsets())).tolist())
                    marks["colors"].extend([fill]*int(valid.sum()))
                else:
                    scatter = target.scatter(xconvert(x[valid]), y[valid], s=10)
                    marks["symbols"].extend(target.transAxes.inverted().transform(scatter.get_offset_transform().transform(scatter.get_offsets())).tolist())
            if doc["case"] == "xy-errors":
                xerr, yerr = raw("D"), raw("E")
                for i in np.flatnonzero(valid):
                    bars = target.errorbar(xconvert([x[i]])[0], y[i], xerr=xerr[i], yerr=yerr[i] if np.isfinite(yerr[i]) else None, fmt="none", capsize=0)[2]
                    for collection in bars:
                        marks["segments"].extend(sorted(transform(segment, target, convert_x=False)) for segment in collection.get_segments())
        for reference in layer["reference_lines"]:
            value = reference["value"]
            if reference["axis"] == "X":
                line = ax.axvline(xconvert([value])[0])
            else:
                line = ax.axhline(value)
            marks["segments"].append(sorted(artist_points(line).tolist()))
        copy = {k: v for k, v in layer.items() if k != "marks"}
        copy["marks"] = marks
        output["layers"].append(copy)
    fig.savefig(source.with_name("matplotlib.png"), dpi=300, transparent=True)
    plt.close(fig)
    source.with_name("matplotlib.json").write_text(json.dumps(output, ensure_ascii=False, indent=2, allow_nan=False)+"\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    # 可只重生成指定样例：gen_matplotlib.py add-remove-curve symbol-fill-border
    names = sys.argv[1:]
    paths = [DIRECTORY / name / "scicanvas.json" for name in names] or sorted(DIRECTORY.glob("*/scicanvas.json"))
    for path in paths:
        generate(path)
