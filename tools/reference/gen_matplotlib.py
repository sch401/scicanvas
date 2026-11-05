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
from scipy import stats

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


def generate_b4(source,doc):
    layer=doc["layers"][0];kind=layer["kind"];page=doc["page"]
    columns={c["key"]:np.asarray(c["values"],dtype=float) for c in doc["data"]}
    fig=plt.figure(figsize=(page["w"]/96,page["h"]/96))
    l,t,w,h=layer["plot"];ax=fig.add_axes([l/page["w"],1-(t+h)/page["h"],w/page["w"],h/page["h"]],projection="polar" if kind.startswith("Polar") else None)
    if kind=="Pie":
        y=columns[layer["y_keys"][0]];y=y[np.isfinite(y)&(y>0)]
        wedges,_=ax.pie(y,startangle=90,counterclock=False)
        marks={"ends":[float((90-w.theta1)%360) for w in wedges]}
        if marks["ends"]:marks["ends"][-1]=360.
    elif kind=="Violin":
        ax.set_xlim(layer["axes"]["x"]["lo"],layer["axes"]["x"]["hi"]);ax.set_ylim(layer["axes"]["y"]["lo"],layer["axes"]["y"]["hi"])
        arrays=[columns[k][np.isfinite(columns[k])] for k in layer["y_keys"]]
        bodies=ax.violinplot(arrays,widths=.5,points=100,bw_method="scott",showmedians=True)["bodies"]
        contours=[]
        for body in bodies:
            points=np.unique(body.get_paths()[0].vertices,axis=0)
            points=ax.transAxes.inverted().transform(ax.transData.transform(points))
            points=points[np.lexsort((points[:,0],points[:,1]))]
            contours.append(points.tolist())
        marks={"contours":contours}
    else:
        ax.set_ylim(0,layer["axes"]["y"]["hi"]);lines=[];symbols=[]
        def projected(points):
            points=np.asarray(points);pixels=ax.transData.transform(points)
            center=ax.transAxes.transform([.5,.5]);radius=ax.bbox.width/2
            # Matplotlib polar axes are square after aspect adjustment.
            return ((pixels-center)/radius).tolist()
        fig.canvas.draw()
        theta=np.deg2rad(columns["A"])
        for key in layer["y_keys"]:
            y=columns[key];points=np.column_stack([theta,y]);points[(~np.isfinite(y))|(y<0)]=np.nan
            for run in runs(points):
                if kind=="PolarLine" and len(run)>1:
                    sampled=[]
                    for a,b in zip(run[:-1],run[1:]):
                        sampled.extend(a+(b-a)*np.arange(16)[:,None]/16);sampled.append(b)
                    line,=ax.plot(np.asarray(sampled)[:,0],np.asarray(sampled)[:,1]);lines.append(projected(line.get_xydata()))
            if kind=="PolarScatter":
                valid=points[np.isfinite(points).all(axis=1)];ax.scatter(valid[:,0],valid[:,1]);symbols.extend(projected(valid))
        marks={"lines":lines,"symbols":symbols}
    copy={k:v for k,v in layer.items() if k!="marks"};copy["marks"]=marks
    output=dict(case=doc["case"],page=page,data=doc["data"],versions=VERSIONS,layers=[copy])
    fig.savefig(source.with_name("matplotlib.png"),dpi=300,transparent=True);plt.close(fig)
    source.with_name("matplotlib.json").write_text(json.dumps(output,ensure_ascii=False,indent=2,allow_nan=False)+"\n",encoding="utf-8",newline="\n")

def generate_axis_scales(source,doc):
    import copy
    from scipy import special
    layer=doc["layers"][0];page=doc["page"];l,t,w,h=layer["plot"]
    fig=plt.figure(figsize=(page["w"]/96,page["h"]/96));ax=fig.add_axes([l/page["w"],1-(t+h)/page["h"],w/page["w"],h/page["h"]])
    for direction in ["x","y"]:
        scale=layer["axes"][direction]["scale"];setscale=getattr(ax,"set_"+direction+"scale")
        if scale=="Log2":setscale("log",base=2)
        elif scale=="Ln":setscale("log",base=np.e)
        elif scale=="Reciprocal":setscale("function",functions=(lambda x:1/x,lambda x:1/x))
        elif scale=="Probability":setscale("function",functions=(lambda x:special.ndtri(x/100),lambda x:100*special.ndtr(x)))
        else:setscale("logit")
        axis=layer["axes"][direction];getattr(ax,"set_"+direction+"lim")(axis["lo"],axis["hi"]);ticks=[p["value"] for p in axis["ticks"]];getattr(ax,"set_"+direction+"ticks")(ticks)
        if scale=="Reciprocal":getattr(ax,"invert_"+direction+"axis")()
        getattr(ax,"set_"+direction+"ticklabels")([f"e^{round(np.log(v))}" for v in ticks] if scale=="Ln" else [f"{v:g}" for v in ticks])
        getattr(ax,direction+"axis").set_minor_locator(matplotlib.ticker.NullLocator())
    columns={c["key"]:np.asarray(c["values"],dtype=float) for c in doc["data"]};x=columns["A"];y=columns["B"]
    line,=ax.plot(x,y);dots=ax.scatter(x,y,s=10);fig.canvas.draw()
    points=ax.transAxes.inverted().transform(ax.transData.transform(np.column_stack([x,y]))).tolist()
    marks=dict(symbols=points,polylines=[points],bars=[],segments=[],area=[],colors=[])
    layer=copy.deepcopy(layer);layer["marks"]=marks
    for direction in ["x","y"]:
        axis=layer["axes"][direction]
        for kind in ["ticks","minor"]:
            for tick in axis[kind]:
                p=[tick["value"],layer["axes"]["y"]["lo"]] if direction=="x" else [layer["axes"]["x"]["lo"],tick["value"]]
                tick["position"]=float(ax.transAxes.inverted().transform(ax.transData.transform(p))[0 if direction=="x" else 1])
    output=dict(case=doc["case"],page=page,data=doc["data"],versions=VERSIONS,layers=[layer])
    fig.savefig(source.with_name("matplotlib.png"),dpi=300,transparent=True);plt.close(fig)
    source.with_name("matplotlib.json").write_text(json.dumps(output,ensure_ascii=False,indent=2,allow_nan=False)+"\n",encoding="utf-8",newline="\n")


def draw_summary_annotations(fig,doc,output):
    import statsmodels
    from statsmodels.stats.diagnostic import normal_ad
    layer=doc["layers"][0];column=next(c for c in doc["data"] if c["key"]==layer["y_keys"][0]);raw=np.asarray(column["values"],dtype=float)
    if layer["rows"] is not None:raw=raw[layer["rows"][0]:layer["rows"][1]+1]
    v=raw[np.isfinite(raw)];n=len(v);sd=np.std(v,ddof=1);q=np.percentile(v,[25,50,75],method="linear");num=lambda v:f"{v:.8g}"
    sw=stats.shapiro(v).pvalue;ad=normal_ad(v)[1]
    text=f"描述统计\nN = {n}，缺失 = {len(raw)-n}\n均值 = {num(np.mean(v))}\nSD = {num(sd)}，SE = {num(sd/np.sqrt(n))}\n最小 = {num(np.min(v))}，最大 = {num(np.max(v))}\nQ1 = {num(q[0])}，中位数 = {num(q[1])}\nQ3 = {num(q[2])}，总和 = {num(np.sum(v))}\nShapiro–Wilk p = {num(sw)}\nAnderson–Darling p = {num(ad)}"
    output["versions"]=VERSIONS|{"statsmodels":statsmodels.__version__};output["annotations"]=[]
    for annotation in doc["annotations"]:
        x,y=annotation["position"];fig.text(x,1-y,text,fontsize=annotation["size_pt"],va="top",ha="left",color=np.asarray(annotation["color"])/255)
        output["annotations"].append(dict(position=[x,y],text=text))


def generate(source):
    doc = json.loads(source.read_text(encoding="utf-8"))
    if "scale" in doc["layers"][0]["axes"]["x"]:
        return generate_axis_scales(source,doc)
    if doc["layers"][0]["kind"] in ("Pie","Violin","PolarLine","PolarScatter"):
        return generate_b4(source,doc)
    columns = {c["key"]: c for c in doc["data"]}
    raw = lambda key: np.asarray(columns[key]["values"], dtype=float)
    x_column = columns["A"]
    date = x_column["kind"] == "DateTime"
    category = x_column["category"]
    source_date, source_category = date, category
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
        statistical = layer["kind"] in ("Histogram", "QQ", "Box", "ColumnScatter", "LineSeries")
        date = source_date and not statistical
        category = None if statistical else source_category
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
        horizontal = kind in ("HorizontalBar", "StackedHorizontalBar", "PercentStackedHorizontalBar")
        keys = layer["y_keys"]
        arrays = [raw(k) for k in keys]
        if kind == "Box":
            ax.set_xticks(np.arange(1, len(keys) + 1), keys)
        if kind=="LineSeries":
            ax.set_xticks(np.arange(1,len(keys)+1),keys)
            for row in range(max(len(y) for y in arrays)):
                yy=np.array([y[row] if row<len(y) else np.nan for y in arrays])
                points=np.column_stack([np.arange(1,len(keys)+1),yy])
                points[~np.isfinite(yy)]=np.nan
                for run in runs(points):
                    if len(run)>1:
                        line,=ax.plot(run[:,0],run[:,1],color="#0F4D92")
                        marks["polylines"].append(artist_points(line,ax).tolist())
        offset=layer.get("offset",{})
        if kind in ("OffsetLine","Waterfall"):
            if offset.get("auto",True):
                dy=max((np.ptp(y[np.isfinite(y)&np.isfinite(x)]) for y in arrays if np.any(np.isfinite(y)&np.isfinite(x))),default=0)
                dx=.05*np.ptp(x[np.isfinite(x)]) if kind=="Waterfall" else 0
            else:
                dx,dy=offset["dx"],offset["dy"]
        sums = np.zeros((2, len(x)))
        totals = np.zeros((2, len(x)))
        for y in arrays:
            totals[0] += np.where(np.isfinite(y) & (y >= 0), y, 0)
            totals[1] += np.where(np.isfinite(y) & (y < 0), -y, 0)
        for si, (key, y) in enumerate(zip(keys, arrays)):
            target = right if key in layer["right_keys"] else ax
            valid = np.isfinite(y)
            if kind not in ("QQ", "Box", "Histogram", "ColumnScatter", "LineSeries"):
                valid &= np.isfinite(x)
            if layer["rows"] is not None:
                lo, hi = layer["rows"]
                valid &= (np.arange(len(x)) >= lo) & (np.arange(len(x)) <= hi)
            if kind=="LineSeries":
                dots=target.scatter(np.full(np.count_nonzero(valid),si+1),y[valid],s=10,color="#0F4D92")
                marks["symbols"].extend(target.transAxes.inverted().transform(dots.get_offset_transform().transform(dots.get_offsets())).tolist())
                continue
            if kind == "ColumnScatter":
                center=si+1
                xx=center+.15*(2*(np.arange(np.count_nonzero(valid))*.618034 % 1)-1)
                scatter=target.scatter(xx,y[valid],s=10)
                marks["symbols"].extend(target.transAxes.inverted().transform(scatter.get_offset_transform().transform(scatter.get_offsets())).tolist())
                mean=np.mean(y[valid])
                line,=target.plot([center-.2,center+.2],[mean,mean])
                marks["polylines"].append(artist_points(line,target).tolist())
                continue
            if kind in ("Bubble", "ColorScatter", "BubbleColor"):
                palette_values=np.array([[15,77,146],[182,67,66],[66,148,158],[154,77,142],[118,118,118],[46,158,68],[39,39,39],[55,117,186],[233,166,161]])
                if kind != "Bubble":
                    vv=raw("D" if kind=="BubbleColor" else "C")
                    lo,hi=np.nanmin(vv),np.nanmax(vv)
                    t=(vv-lo)/(hi-lo) if hi>lo else np.full(len(vv),.5)
                    rgb=palette_values[np.floor(np.clip(t,0,1)*8+.5).astype(int)]
                else:
                    rgb=np.tile(palette_values[0],(len(x),1))
                size=raw("C") if kind!="ColorScatter" else np.ones(len(x))
                diameter=12*np.sqrt(np.maximum(size,0)/np.nanmax(size)) if kind!="ColorScatter" else np.full(len(x),4.5)
                scatter=target.scatter(xconvert(x[valid]),y[valid],s=(diameter[valid]*72/96)**2,c=rgb[valid]/255)
                marks["symbols"].extend(target.transAxes.inverted().transform(scatter.get_offset_transform().transform(scatter.get_offsets())).tolist())
                marks["colors"].extend(rgb[valid].tolist())
                if kind!="ColorScatter":
                    marks["size_ratios"]=(diameter[valid]/diameter[valid].max()).tolist()
                if kind!="Bubble":
                    sample=np.linspace(0,1,257)
                    palette=colors.ListedColormap(palette_values[np.floor(sample*8+.5).astype(int)]/255)
                    cax=fig.add_axes([(left+width+14*page["w"]/(89*96/25.4))/page["w"],1-(top+height)/page["h"],.04,height/page["h"]])
                    bar=fig.colorbar(matplotlib.cm.ScalarMappable(norm=colors.Normalize(lo,hi),cmap=palette),cax=cax)
                    ticks=[float(v) for v in bar.get_ticks() if lo<=v<=hi]
                    scale_colors=np.floor(palette((np.arange(64)+.5)/64)[:,:3]*255+.5).astype(int).tolist()
                    color_scale=dict(colors=scale_colors,ticks=ticks)
                continue
            if kind == "ErrorBand":
                errors=raw("C")
                band_valid=valid & np.isfinite(errors) & (errors>=0)
                band=np.column_stack([x,y+errors,y-errors]);band[~band_valid]=np.nan
                for run in runs(band):
                    if len(run)<2:
                        continue
                    target.fill_between(xconvert(run[:,0]),run[:,1],run[:,2],alpha=.3)
                    for xx,upper,lower in run:
                        a,b=transform([[xx,lower],[xx,upper]],target)
                        marks["area"].append([a[0],min(a[1],b[1]),max(a[1],b[1])])
                points=np.column_stack([x,y]);points[~valid]=np.nan
                for run in runs(points):
                    if len(run)>1:
                        line,=target.plot(xconvert(run[:,0]),run[:,1])
                        marks["polylines"].append(artist_points(line,target).tolist())
                marks["area"].sort(key=lambda p:p[0])
                continue
            if kind == "Box":
                if not np.any(valid):
                    continue
                artists = target.boxplot(y[valid], positions=[si + 1], widths=.6, whis=1.5,
                    patch_artist=True, showmeans=True, manage_ticks=False,
                    meanprops=dict(marker="D", markerfacecolor="none"))
                box = artist_points(artists["boxes"][0], target)
                marks["bars"].append([(box[:,0].min()+box[:,0].max())/2, box[:,1].min(), box[:,1].max()])
                for artist in artists["medians"] + artists["whiskers"] + artists["caps"]:
                    ends = artist_points(artist, target).tolist()
                    if ends[0] != ends[1]:
                        marks["segments"].append(sorted(ends))
                for artist in artists["fliers"]:
                    marks["symbols"].extend(artist_points(artist, target).tolist())
                for artist in artists["means"]:
                    points = artist_points(artist, target).tolist()
                    marks["symbols"].extend(points)
                    marks["colors"].extend([None] * len(points))
                continue
            if kind == "QQ":
                if np.count_nonzero(valid) < 3:
                    continue
                (osm, osr), (slope, intercept, _) = stats.probplot(y[valid], dist="norm")
                dots, = target.plot(osm, osr, linestyle="none", marker="o")
                marks["symbols"].extend(artist_points(dots, target).tolist())
                ends = np.array([osm[0], osm[-1]])
                line, = target.plot(ends, intercept + slope * ends, color=dots.get_color())
                marks["polylines"].append(artist_points(line, target).tolist())
                continue
            if kind == "Histogram":
                options = layer.get("histogram", {})
                pools = []
                for yy in arrays:
                    keep = np.isfinite(yy)
                    if layer["rows"] is not None:
                        first, last = layer["rows"]
                        keep &= (np.arange(len(yy)) >= first) & (np.arange(len(yy)) <= last)
                    pools.append(yy[keep])
                pooled = np.concatenate(pools)
                if not len(pooled):
                    continue
                requested = options.get("range")
                lo, hi = requested if requested is not None else (pooled.min(), pooled.max())
                if lo == hi:
                    lo, hi = lo-.5, hi+.5
                mode = options.get("bins", "Auto")
                if "Width" in mode:
                    width = mode["Width"]
                    bins = int(np.ceil((hi-lo)/width))
                    if lo+bins*width < hi:
                        bins += 1
                    edges = lo + np.arange(bins+1)*width
                else:
                    bins = mode["Count"] if "Count" in mode else int(np.ceil(np.log2(max(len(v) for v in pools))))+1
                    edges = np.linspace(lo, hi, bins+1)
                selected = y[valid]
                if requested is not None:
                    selected = selected[(selected >= lo) & (selected <= hi)]
                counts = np.histogram(selected, bins=edges)[0]
                if options.get("cumulative", False):
                    counts = np.cumsum(counts)
                patches = target.bar((edges[:-1]+edges[1:])/2, counts, width=np.diff(edges), alpha=.6 if len(keys)>1 else 1)
                for patch in patches:
                    a, b = target.transAxes.inverted().transform(patch.get_data_transform().transform(patch.get_bbox().get_points()))
                    marks["bars"].append([(a[0]+b[0])/2, min(a[1], b[1]), max(a[1], b[1])])
                values = y[valid]
                if options.get("normal_curve", False) and len(values)>1 and values.std(ddof=1)>0:
                    xx = np.linspace(edges[0], edges[-1], 100)
                    mean, sd = values.mean(), values.std(ddof=1)
                    yy = len(values)*stats.norm.cdf((xx-mean)/sd) if options.get("cumulative", False) else len(values)*(edges[1]-edges[0])*stats.norm.pdf((xx-mean)/sd)/sd
                    line, = target.plot(xx, yy)
                    marks["polylines"].append(artist_points(line, target).tolist())
                continue
            if kind in ("StackedArea", "FillBetween"):
                base = np.zeros(len(x))
                upper = y.copy()
                filled = kind == "StackedArea" or key in layer["base_by_y"]
                if kind == "StackedArea":
                    for i in np.flatnonzero(valid):
                        sign = int(y[i] < 0)
                        base[i] = sums[sign,i]
                        sums[sign,i] += y[i]
                        upper[i] = sums[sign,i]
                elif filled:
                    base = raw(layer["base_by_y"][key])
                    valid &= np.isfinite(base)
                points = np.column_stack([x,upper,base])
                points[~valid] = np.nan
                for run in runs(points):
                    run = run[np.argsort(run[:,0],kind="stable")]
                    if len(run) < 2:
                        continue
                    xx,yy,bb = run.T
                    if filled:
                        target.fill_between(xconvert(xx),yy,bb,alpha=.35)
                        boundary = list(run)
                        for a,b in zip(run[:-1],run[1:]):
                            da,db = a[1]-a[2],b[1]-b[2]
                            if da*db < 0:
                                t=da/(da-db)
                                cross=a+t*(b-a)
                                boundary.append(cross)
                        for xx_,yy_,bb_ in boundary:
                            a,b=transform([[xx_,bb_],[xx_,yy_]],target)
                            marks["area"].append([a[0],min(a[1],b[1]),max(a[1],b[1])])
                    line,=target.plot(xconvert(xx),yy)
                    marks["polylines"].append(artist_points(line,target).tolist())
                continue
            if kind in ("Bar", "HorizontalBar", "StackedBar", "PercentStackedBar", "FloatingBar", "StackedHorizontalBar", "PercentStackedHorizontalBar"):
                base = np.zeros(len(x))
                values = y.copy()
                if kind == "FloatingBar":
                    base = raw(layer["base_by_y"][key])
                    valid &= np.isfinite(base)
                elif kind in ("StackedBar", "PercentStackedBar", "StackedHorizontalBar", "PercentStackedHorizontalBar"):
                    for i in np.flatnonzero(valid):
                        sign = int(y[i] < 0)
                        value = y[i]
                        if kind in ("PercentStackedBar", "PercentStackedHorizontalBar"):
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
            points = np.column_stack([x+si*dx,y+si*dy]) if kind in ("OffsetLine","Waterfall") else np.column_stack([x,y])
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
        if kind in ("StackedArea", "FillBetween"):
            merged=[]
            for bound in sorted(marks["area"],key=lambda p:p[0]):
                if merged and abs(merged[-1][0]-bound[0]) < 1e-10:
                    merged[-1][1]=min(merged[-1][1],bound[1])
                    merged[-1][2]=max(merged[-1][2],bound[2])
                else:
                    merged.append(bound)
            marks["area"]=merged
        for reference in layer["reference_lines"]:
            value = reference["value"]
            if reference["axis"] == "X":
                line = ax.axvline(xconvert([value])[0])
            else:
                line = ax.axhline(value)
            marks["segments"].append(sorted(artist_points(line).tolist()))
        copy = {k: v for k, v in layer.items() if k != "marks"}
        copy["marks"] = marks
        if kind in ("ColorScatter","BubbleColor"):
            copy["color_scale"]=color_scale
        output["layers"].append(copy)
    if doc.get("annotations"):draw_summary_annotations(fig,doc,output)
    fig.savefig(source.with_name("matplotlib.png"), dpi=300, transparent=True)
    plt.close(fig)
    source.with_name("matplotlib.json").write_text(json.dumps(output, ensure_ascii=False, indent=2, allow_nan=False)+"\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    # 可只重生成指定样例：gen_matplotlib.py add-remove-curve symbol-fill-border
    names = sys.argv[1:]
    paths = [DIRECTORY / name / "scicanvas.json" for name in names] or sorted(DIRECTORY.glob("*/scicanvas.json"))
    for path in paths:
        generate(path)
