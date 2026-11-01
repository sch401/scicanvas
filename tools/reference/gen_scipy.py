"""仅开发机：python -m venv .venv-ref
.venv-ref/Scripts/python -m pip install -r tools/reference/requirements-reference.txt
.venv-ref/Scripts/python tools/reference/gen_scipy.py

CI 只读取提交的 JSON。输入使用固定种子并完整写入文件。Z 分数使用样本标准差 ddof=1。
"""
import json
from pathlib import Path
import platform
import warnings

import numpy as np
import scipy
from scipy import integrate, interpolate, optimize, signal, special, stats

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates/scicanvas-core/tests/reference"
RNG = np.random.default_rng(20261101)
VERSIONS = {"Python": platform.python_version(), "NumPy": np.__version__, "SciPy": scipy.__version__}


def clean(value):
    if isinstance(value, np.ndarray):
        return clean(value.tolist())
    if isinstance(value, dict):
        return {key: clean(v) for key, v in value.items()}
    if isinstance(value, (tuple, list)):
        return [clean(v) for v in value]
    if isinstance(value, (float, np.floating)):
        return float(value) if np.isfinite(value) else None
    if isinstance(value, np.integer):
        return int(value)
    return value


def write(name, cases):
    OUT.mkdir(parents=True, exist_ok=True)
    data = dict(generator="tools/reference/gen_scipy.py", versions=VERSIONS, cases=cases)
    (OUT / f"{name}.json").write_text(json.dumps(clean(data), ensure_ascii=False, indent=2, allow_nan=False) + "\n", encoding="utf-8", newline="\n")


def integrals():
    cases = []
    for name, xs, ys in [
        ("equal-spacing", [0, 1, 2, 3, 4], [2, 4, 1, 3, 2]),
        ("unequal-spacing", [0, 0.25, 1.5, 3, 4.5], [2, 4, 1, 3, 2]),
        ("missing-rows", [0, 1, 2, 3, 4, 5], [2, None, -2, 3, None, -1]),
        ("negative-area", [0, 1, 2, 3], [-2, -4, -1, -3]),
        ("absolute-crossing", [0, 1, 3, 5], [-3, 2, -4, 3]),
    ]:
        x, y = np.array(xs, dtype=float), np.array(ys, dtype=float)
        valid = np.isfinite(x) & np.isfinite(y)
        x, y = x[valid], y[valid]
        cum = integrate.cumulative_trapezoid(y, x, initial=0)
        # 对原分段线性曲线显式插入过零点，再用 NumPy 积分 |y|。
        xx, yy = [x[0]], [y[0]]
        for a, b, ya, yb in zip(x[:-1], x[1:], y[:-1], y[1:]):
            if ya * yb < 0:
                xx.append(a - ya * (b-a) / (yb-ya))
                yy.append(0.0)
            xx.append(b)
            yy.append(yb)
        aligned = np.full(len(xs), np.nan)
        aligned[valid] = cum
        cases.append(dict(name=name, x=xs, y=ys, cumulative=aligned, area=np.trapezoid(y, x), absolute=np.trapezoid(np.abs(yy), xx)))
    write("integrate", cases)


def normalization():
    cases = []
    for name, raw in [
        ("mixed-signs", [-4, -1, 0, 3, 7]),
        ("missing", [None, 2, 4, None, 8]),
        ("constant", [3, 3, 3, 3]),
        ("single", [7]),
        ("seeded", RNG.normal(2, 3, 11)),
    ]:
        values = np.asarray(raw, dtype=float)
        y = values[np.isfinite(values)]
        with warnings.catch_warnings(), np.errstate(all="ignore"):
            warnings.simplefilter("ignore")
            expected = {"max": y / np.max(y), "zscore": stats.zscore(y, ddof=1), "minmax": (y-np.min(y))/(np.max(y)-np.min(y))}
        cases.append(dict(name=name, input=raw, ddof=1, **expected))
    write("normalize", cases)


def descriptive():
    cases = []
    for name, raw in [
        ("odd", [2, 7, -1, 4, 11]), ("even", [1, 2, 6, 9]),
        ("missing", [None, 2, 7, None, -1, 4]), ("constant", [5, 5, 5, 5]),
        ("large", [1e12, 1e12+4, 1e12-3, 1e12+9]), ("single", [3]),
        ("seeded", RNG.normal(1, 3, 17)),
    ]:
        y = np.asarray(raw, dtype=float)
        y = y[np.isfinite(y)]
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            result = dict(n=len(y), mean=np.mean(y), sd=np.std(y, ddof=1), se=stats.sem(y, ddof=1),
                          median=np.median(y), q1=np.quantile(y, .25, method="linear"), q3=np.quantile(y, .75, method="linear"),
                          min=np.min(y), max=np.max(y), sum=np.sum(y))
        cases.append(dict(name=name, input=raw, result=result))
    write("describe", cases)


def column_math():
    cases = []
    a = RNG.integers(-16, 17, 13) / 4
    b = np.array([1, 2, 0, 4, -2, 1, 2, 0, 4, 1, -2, 2, 1], dtype=float)
    for op, function in [("add", np.add), ("sub", np.subtract), ("mul", np.multiply), ("div", np.divide)]:
        with np.errstate(all="ignore"):
            result = function(a, b)
        cases.append(dict(name="elementwise-"+op, op=op, x=np.arange(len(a)), a=a, b=b, result=result))
    x = np.array([-1, 0, .5, 1, 1.5, 2, 2.5, 3, 4])
    bx = np.array([.5, 1.5, 2.5])
    by = np.array([10, 20, 16])
    a = np.arange(len(x), dtype=float)
    result = a - np.interp(x, bx, by, left=np.nan, right=np.nan)
    cases.append(dict(name="different-x-grid", op="sub", x=x, a=a, bx=bx, b=by, result=result))
    write("column_math", cases)


def polynomial():
    cases = []
    for order in range(2, 7):
        for offset in (0.0, 1e6):
            xs = offset + np.linspace(-16, 16, 41)
            mx, sx = np.mean(xs), np.std(xs, ddof=0)
            t = (xs-mx)/sx
            ys = np.polynomial.polynomial.polyval(t, RNG.normal(0, 1, order+1)) + RNG.normal(0, .05, len(xs))
            bt = np.polynomial.polynomial.polyfit(t, ys, order)
            fitted = np.polynomial.polynomial.polyval(t, bt)
            rss = np.sum((ys-fitted)**2)
            vand = np.polynomial.polynomial.polyvander(t, order)
            # SVD 协方差，避免参考实现复用 SciCanvas 的正规方程求逆。
            _, singular, vt = np.linalg.svd(vand, full_matrices=False)
            cov = (vt.T / singular**2) @ vt * (rss/(len(xs)-order-1))
            transform = np.zeros((order+1, order+1))
            base = np.polynomial.Polynomial([-mx/sx, 1/sx])
            for i in range(order+1):
                v = (base**i).coef
                transform[:len(v), i] = v
            coef = transform @ bt
            se = np.sqrt(np.diag(transform @ cov @ transform.T))
            case = dict(name=f"degree-{order}-offset-{int(offset)}", order=order, x=xs, y=ys, coef=coef, se=se, fitted=fitted, rss=rss)
            if order == 6:
                case.update(tolerance=5e-12, tolerance_reason="六阶标准化正规方程与 NumPy SVD 解的舍入差；offset=0 的系数 2 实测相对误差 2.56e-12。其余阶数保持 1e-12。")
            cases.append(case)
    write("polynomial", cases)


def statistical_tests():
    a = np.array([5.1, 4.9, 6.2, 5.8, 6.0, 5.5, 5.3, 6.1])
    b = np.array([4.6, 5.0, 4.4, 5.2, 4.8, 4.1, 5.4, 4.7, 4.3, 5.0])
    x = np.array([1.2, 2.4, 2.4, 3.1, 4.8, 5.0, 5.5, 6.1, 7.3, 7.3, 8.0, 9.6])
    y = np.array([2.0, 2.9, 3.5, 3.1, 4.2, 6.0, 5.1, 6.6, 6.6, 8.1, 7.7, 9.0])
    raw = np.array([-2, -1, 0, 1, 1, 2, 3, 4, 5, np.nan])
    counts, edges = np.histogram(raw[np.isfinite(raw)], bins=4, range=(-1, 3))
    cases = [dict(name="histogram", values=raw, counts=counts, edges=edges)]
    for name, fn in [("pearson", stats.pearsonr), ("spearman", stats.spearmanr),
                     ("kendall", lambda a, b: stats.kendalltau(a, b, method="asymptotic"))]:
        r = fn(x, y)
        cases.append(dict(name=name, x=x, y=y, r=r.statistic, p=r.pvalue))
    for name, result in [("one", stats.ttest_1samp(a, 5.0)),
                         ("student", stats.ttest_ind(a, b, equal_var=True)),
                         ("welch", stats.ttest_ind(a, b, equal_var=False)),
                         ("paired", stats.ttest_rel(a, b[:len(a)]))]:
        ci = result.confidence_interval(confidence_level=.95)
        cases.append(dict(name=name, a=a, b=b[:len(a)] if name == "paired" else b,
                          t=result.statistic, df=result.df, p=result.pvalue, ci=[ci.low, ci.high]))
    for suffix, va, vb, mu in [
        ("small", [1., 2., 4.], [1.2, 2.2, 3.7], 2.),
        ("missing", [1., 2., 4., np.nan], [1.2, 2.2, 3.7, np.nan], 2.),
        ("large", [1e6+1, 1e6+4, 1e6+3], [1e6-2, 1e6+2, 1e6+.5], 1e6),
    ]:
        va, vb = np.asarray(va), np.asarray(vb)
        for kind, result in [("one", stats.ttest_1samp(va, mu, nan_policy="omit")),
                             ("student", stats.ttest_ind(va, vb, equal_var=True, nan_policy="omit")),
                             ("welch", stats.ttest_ind(va, vb, equal_var=False, nan_policy="omit")),
                             ("paired", stats.ttest_rel(va, vb, nan_policy="omit"))]:
            ci = result.confidence_interval(confidence_level=.95)
            cases.append(dict(name=kind+"-"+suffix, a=va, b=vb, mu=mu,
                              t=result.statistic, df=result.df, p=result.pvalue, ci=[ci.low, ci.high]))
    for suffix, xx, yy in [
        ("small", [1., 2., 4.], [4., 1., 3.]),
        ("missing", [1., 2., 4., np.nan], [4., 1., 3., 8.]),
        ("large", [1e6+1, 1e6+2, 1e6+4], [-1e6+4, -1e6+1, -1e6+3]),
    ]:
        for kind, fn in [("pearson", stats.pearsonr), ("spearman", stats.spearmanr),
                         ("kendall", lambda a, b: stats.kendalltau(a, b, method="asymptotic"))]:
            xx, yy = np.asarray(xx), np.asarray(yy)
            valid = np.isfinite(xx) & np.isfinite(yy)
            r = fn(xx[valid], yy[valid])
            cases.append(dict(name=kind+"-"+suffix, x=xx, y=yy, r=r.statistic, p=r.pvalue))
    for suffix, raw, limits in [("small", [-1., 1., 3.], (-1., 3.)),
                                ("extreme", [-1e9, -1e9, 0., 1e9], (-1e9, 1e9))]:
        counts, edges = np.histogram(raw, bins=4, range=limits)
        cases.append(dict(name="histogram-"+suffix, values=raw, limits=limits, counts=counts, edges=edges))
    write("statistics", cases)


def signal_tests():
    y = np.array([0.1, 0.7, -0.2, 1.4, 0.3, -0.6, 1.1, 0.8, -0.4,
                  0.2, 0.9, -0.8, 0.5, 1.3, -0.1, 0.6, -0.3])
    dt = 0.1
    spectrum = np.fft.rfft(y)
    freq = np.fft.rfftfreq(len(y), dt)
    cases = [dict(name="fft", y=y, dt=dt, re=spectrum.real, im=spectrum.imag, freq=freq)]
    for name, mask in [("low", freq <= 1.5), ("high", freq >= 1.5),
                       ("band", (freq >= 1.0) & (freq <= 3.0)),
                       ("stop", (freq < 1.0) | (freq > 3.0))]:
        cases.append(dict(name=name, y=y, dt=dt,
                          result=np.fft.irfft(np.where(mask, spectrum, 0), n=len(y))))
    x = np.array([1.2, -0.7, 2.0, 3.4, 0.5, -1.0])
    z = np.array([2.5, 1.1, -0.2, 1.7, 3.2, 0.3])
    a, b = x - x.mean(), z - z.mean()
    corr = np.correlate(b, a, "full") / np.sqrt(np.sum(a*a)*np.sum(b*b))
    auto = np.correlate(a, a, "full") / np.sum(a*a)
    cases.append(dict(name="correlation", x=x, z=z, lags=np.arange(-len(x)+1,len(x)), corr=corr, auto=auto))
    for yy in ([1.2, -0.3, 2.5], [1., 1., 2.], [1e6+1, -1e6-2, 2e6+3]):
        yy = np.asarray(yy)
        f = np.fft.rfft(yy)
        case = dict(name="fft", y=yy, dt=dt, re=f.real, im=f.imag,
                    freq=np.fft.rfftfreq(len(yy), dt))
        if np.max(np.abs(yy)) > 1e5:
            case.update(tolerance=1e-9, tolerance_reason="输入量级约 2e6，理论零虚部实测绝对舍入 1.75e-10，约为输入量级的 1e-16。")
        cases.append(case)
    cases.append(dict(name="fft-missing", y=[1., np.nan, 3.], dt=dt))
    write("signals", cases)


def interpolation_cases():
    cases = []
    for name, x, y, at in [
        ("normal", [2.5, 0, 4, .5, 1.5], [1, 1, 2, 2, 3], [-.3, .25, 1, 3, 4.4]),
        ("missing", [0, 1, None, 2, 3], [1, 2, 9, None, 4], [-.2, .5, 2.5, 3.2]),
        ("small", [0, 1, 2], [1, 4, 2], [-.5, .5, 1.5, 2.5]),
        ("large", [1e6, 1e6+1, 1e6+2, 1e6+3], [1e5, 1e5+2, 1e5-1, 1e5+3],
         [1e6-.5, 1e6+.5, 1e6+2.5, 1e6+3.5]),
    ]:
        xx, yy = np.asarray(x, float), np.asarray(y, float)
        valid = np.isfinite(xx) & np.isfinite(yy)
        order = np.argsort(xx[valid]); xx, yy = xx[valid][order], yy[valid][order]
        case = dict(name=name, x=x, y=y, at=at,
                    linear=interpolate.interp1d(xx, yy, fill_value="extrapolate")(at),
                    spline=interpolate.CubicSpline(xx, yy, bc_type="natural", extrapolate=True)(at),
                    akima=interpolate.Akima1DInterpolator(xx, yy, extrapolate=True)(at))
        cases.append(case)
    cases.append(dict(name="duplicate", x=[0, 1, 1, 2], y=[1, 2, 3, 4], at=[.5]))
    write("interpolation", cases)


def derivative_cases():
    cases = []
    for name, y, window, order, derivatives in [
        ("normal", [1., 2., 4., 3., 8., 7., 6., 9., 8.], 5, 2, [1, 2]),
        ("ties", [1., 1., 2., 2., 3., 3., 4., 4., 5.], 5, 2, [1]),
        ("small", [1., 4., 2.], 3, 1, [1]),
        ("missing", [1., 2., 4., None, 8., 7., 6., 9., 8.], 5, 2, [1]),
        ("large", [1e6+1, 1e6+2, 1e6+4, 1e6+3, 1e6+8, 1e6+7, 1e6+6, 1e6+9, 1e6+8], 5, 2, [1]),
    ]:
        yy = np.asarray(y, float); x = np.arange(len(yy), dtype=float)*.5
        for derivative in derivatives:
            case = dict(name=name, x=x, y=y, window=window, order=order, derivative=derivative,
                        gradient=np.gradient(yy, .5))
            if not np.isfinite(yy).all():
                try:
                    signal.savgol_filter(yy, window, order, deriv=derivative, delta=.5, mode="interp")
                except ValueError:
                    case["scipy_error"] = "SciPy rejects NaN in the interpolation window"
            else:
                # Constant offsets have zero derivatives; centering avoids cancellation for large baselines.
                source = yy - yy[0] if name == "large" else yy
                if name == "large":
                    case["input_shift"] = float(yy[0])
                case["result"] = signal.savgol_filter(source, window, order, deriv=derivative, delta=.5, mode="interp")
            cases.append(case)
    write("derivatives", cases)


def peak_cases():
    cases = []
    for name, y in [("normal", [0., 1., 4., 1., 0., 1., 3., 1., 0.]),
                    ("ties", [0., 3., 3., 3., 0.]), ("small", [0., 1., 0.]),
                    ("missing", [0., None, 0.]), ("large", [0., 1e5, 2e5, 1e5, 0.])]:
        yy = np.asarray(y, float); x = np.arange(len(yy), dtype=float)
        indices, props = signal.find_peaks(yy, prominence=.5)
        widths, _, left, right = signal.peak_widths(yy, indices, rel_height=.5)
        peaks = []
        for j, idx in enumerate(indices):
            lb, rb = props['left_bases'][j], props['right_bases'][j]
            lower = max(indices[j-1], lb) if j else lb
            upper = min(indices[j+1], rb) if j+1 < len(indices) else rb
            start = int(lower + np.argmin(yy[lower:idx+1]))
            end = int(idx + np.argmin(yy[idx:upper+1]))
            xx = x[start:end+1]; segment = yy[start:end+1]
            base = np.interp(xx, [x[start], x[end]], [yy[start], yy[end]])
            area = np.trapezoid(segment-base, xx)
            moment = np.trapezoid((segment-base)*xx, xx)
            peaks.append(dict(index=int(idx), prominence=props['prominences'][j], width=widths[j],
                              left=left[j], right=right[j], start=start, end=end,
                              area=area, centroid=moment/area if area else x[idx]))
        cases.append(dict(name=name, x=x, y=y, peaks=peaks))
    write("peaks", cases)


def constrained_fit_cases():
    cases = []
    def exp1(x, y0, a, t):
        return y0 + a*np.exp(-x/t)
    def exp2(x, y0, a1, t1, a2, t2):
        return y0 + a1*np.exp(-x/t1) + a2*np.exp(-x/t2)
    def noise(n):
        i = np.arange(n, dtype=float)
        return np.sin(1.7*i)*.6 + np.cos(.37*i*i)*.4
    x = np.linspace(0, 8, 41); y = exp1(x, .3, 5., 1.7) + .05*noise(len(x))
    for name, xx, yy, fixed in [
        ("fixed", x, y, .3),
        ("missing", np.r_[x, np.nan], np.r_[y, 2.], .3),
        ("ties", np.r_[x, x[10]], np.r_[y, y[10]], .3),
        ("large", x, y-.3+1e6, 1e6),
    ]:
        valid = np.isfinite(xx) & np.isfinite(yy)
        # Removing a fixed constant leaves the fitted amplitudes unchanged and avoids cancellation.
        def reduced(x, a, t): return a*np.exp(-x/t)
        p, cov = optimize.curve_fit(reduced, xx[valid], yy[valid]-fixed, p0=[5., 1.7], maxfev=50000)
        full = [fixed, *p]; se = [0., *np.sqrt(np.diag(cov))]
        rss = np.sum((yy[valid]-exp1(xx[valid], *full))**2)
        cases.append(dict(name=name, model="ExpDec1", x=xx, y=yy, fixed=fixed,
                          initial=[fixed, 5., 1.7], params=full, se=se, rss=rss))
    upper = [np.inf, np.inf, 1.5]
    p, cov = optimize.curve_fit(exp1, x, y, p0=[.3, 5., 1.4], bounds=([-np.inf, -np.inf, .01], upper), maxfev=50000)
    cases.append(dict(name="bound", model="ExpDec1", x=x, y=y, upper=1.5, initial=[.3, 5., 1.4],
                      params=p, se=np.sqrt(np.diag(cov)), rss=np.sum((y-exp1(x,*p))**2)))
    x2=np.linspace(0,10,41); y2=exp2(x2,.5,4.,.8,2.,4.)+.02*noise(len(x2))
    for name, constraint, reduction, p0 in [
        ("equality", "A1 + A2 = 6.5", lambda x,y0,a1,t1,t2: exp2(x,y0,a1,t1,6.5-a1,t2), [.5,4.,.8,4.]),
        ("active", "t2 >= t1 + 3.5", lambda x,y0,a1,t1,a2: exp2(x,y0,a1,t1,a2,t1+3.5), [.5,4.,.8,2.]),
        ("inactive", "t2 - t1 >= 1", exp2, [.5,4.,.8,2.,4.]),
    ]:
        p,cov=optimize.curve_fit(reduction,x2,y2,p0=p0,maxfev=100000,ftol=1e-13,xtol=1e-13,gtol=1e-13)
        err=np.sqrt(np.diag(cov))
        if name=="equality": full=[p[0],p[1],p[2],6.5-p[1],p[3]]; se=[err[0],err[1],err[2],err[1],err[3]]
        elif name=="active": full=[p[0],p[1],p[2],p[3],p[2]+3.5]; se=[err[0],err[1],err[2],err[3],err[2]]
        else: full=p; se=err
        cases.append(dict(name=name, model="ExpDec2", x=x2, y=y2, constraint=constraint,
                          params=full, se=se, rss=np.sum((y2-exp2(x2,*full))**2)))
    cases.append(dict(name="small", model="ExpDec1", x=[0.,1.,2.], y=[5.,3.,2.]))
    write("constraints", cases)


def builtin_fit_cases():
    pi = np.pi
    ln2 = np.log(2)
    models = {
        "Gauss": lambda x,y0,xc,w,a: y0+a/(w*np.sqrt(pi/2))*np.exp(-2*((x-xc)/w)**2),
        "Lorentz": lambda x,y0,xc,w,a: y0+2*a/pi*w/(4*(x-xc)**2+w*w),
        "Voigt": lambda x,y0,xc,a,wg,wl: y0+a*special.voigt_profile(x-xc,wg/(2*np.sqrt(2*ln2)),wl/2),
        "PsdVoigt1": lambda x,y0,xc,a,w,mu: y0+a*(mu*(2/pi*w/(4*(x-xc)**2+w*w))+(1-mu)*(np.sqrt(4*ln2)/(np.sqrt(pi)*w)*np.exp(-4*ln2*(x-xc)**2/w**2))),
        "LogNormal": lambda x,y0,xc,w,a: y0+a/(np.sqrt(2*pi)*w*x)*np.exp(-(np.log(x/xc)**2)/(2*w*w)),
        "ExpDec1": lambda x,y0,a,t: y0+a*np.exp(-x/t),
        "ExpDec2": lambda x,y0,a1,t1,a2,t2: y0+a1*np.exp(-x/t1)+a2*np.exp(-x/t2),
        "ExpDec3": lambda x,y0,a1,t1,a2,t2,a3,t3: y0+a1*np.exp(-x/t1)+a2*np.exp(-x/t2)+a3*np.exp(-x/t3),
        "ExpGro1": lambda x,y0,a,t: y0+a*np.exp(x/t),
        "ExpAssoc": lambda x,y0,a1,t1,a2,t2: y0+a1*(1-np.exp(-x/t1))+a2*(1-np.exp(-x/t2)),
        "Boltzmann": lambda x,a1,a2,x0,dx: a2+(a1-a2)/(1+np.exp((x-x0)/dx)),
        "Logistic": lambda x,a1,a2,x0,p: a2+(a1-a2)/(1+(x/x0)**p),
        "DoseResp": lambda x,a1,a2,x0,p: a1+(a2-a1)/(1+10**((x0-x)*p)),
        "Hill": lambda x,a1,a2,k,n: a1+(a2-a1)*x**n/(k**n+x**n),
        "Gompertz": lambda x,a,xc,k: a*np.exp(-np.exp(-k*(x-xc))),
        "Allometric1": lambda x,a,b: a*x**b,
        "Allometric2": lambda x,y0,a,b: y0+a*x**b,
        "Power1": lambda x,a,b,c: a*(x-b)**c,
        "Log3P1": lambda x,a,b,c: a-b*np.log(x+c),
    }
    specs = [
        ("Gauss",0,10,[1,4.2,1.5,6],.05), ("Lorentz",0,10,[.5,5.5,1.2,4],.05),
        ("Voigt",0,10,[.5,5,8,1.2,.8],.03), ("PsdVoigt1",0,10,[.3,4.5,6,1.5,.4],.03),
        ("LogNormal",.2,10,[.2,3,.4,10],.03), ("ExpDec1",0,8,[.3,5,1.7],.05),
        ("ExpDec2",0,10,[.5,4,.8,2,4],.02), ("ExpDec3",0,20,[.2,3,.5,2,2.5,1.5,9],.01),
        ("ExpGro1",0,5,[1,.5,1.5],.05), ("ExpAssoc",0,10,[.5,3,.7,2,4],.02),
        ("Boltzmann",-5,5,[1,9,.7,.8],.1), ("Logistic",.1,10,[.5,8,3,2.5],.1),
        ("DoseResp",-3,3,[1,9,.3,1.2],.05), ("Hill",.1,10,[1,8,2.5,2.2],.05),
        ("Gompertz",0,10,[6,3,.9],.05), ("Allometric1",.5,10,[2,.75],.05),
        ("Allometric2",.5,10,[1,.8,1.6],.05), ("Power1",1,10,[2,.5,.7],.02),
        ("Log3P1",.5,10,[3,1.2,.5],.02),
    ]
    i=np.arange(41,dtype=float); noise=np.sin(1.7*i)*.6+np.cos(.37*i*i)*.4
    cases=[]
    for model,lo,hi,truth,amp in specs:
        x=np.linspace(lo,hi,41); fn=models[model]; y=fn(x,*truth)+amp*noise
        p,cov=optimize.curve_fit(fn,x,y,p0=truth,maxfev=100000,ftol=1e-12,xtol=1e-12,gtol=1e-12)
        cases.append(dict(name=model,model=model,x=x,y=y,params=p,se=np.sqrt(np.diag(cov)),rss=np.sum((y-fn(x,*p))**2)))
    base=next(c for c in cases if c['model']=='ExpDec1'); x,y=base['x'],base['y']
    for name,xx,yy,shift in [("missing",np.r_[x,np.nan],np.r_[y,2.],0.),
                             ("ties",np.r_[x,x[10]],np.r_[y,y[10]],0.),
                             ("large",x,y+1e6,1e6)]:
        valid=np.isfinite(xx)&np.isfinite(yy); p,cov=optimize.curve_fit(models['ExpDec1'],xx[valid],yy[valid]-shift,
                                  p0=[.3,5,1.7],maxfev=100000,ftol=1e-12,xtol=1e-12,gtol=1e-12)
        p[0]+=shift
        cases.append(dict(name=name,model="ExpDec1",x=xx,y=yy,input_shift=shift,params=p,se=np.sqrt(np.diag(cov)),
                          rss=np.sum((yy[valid]-models['ExpDec1'](xx[valid],*p))**2)))
    cases.append(dict(name="small",model="ExpDec1",x=[0.,1.,2.],y=[5.,3.,2.]))
    write("builtin_fits",cases)


if __name__ == "__main__":
    integrals()
    normalization()
    descriptive()
    column_math()
    polynomial()
    statistical_tests()
    signal_tests()
    interpolation_cases()
    derivative_cases()
    peak_cases()
    constrained_fit_cases()
    builtin_fit_cases()
