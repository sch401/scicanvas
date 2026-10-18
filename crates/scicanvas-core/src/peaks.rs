//! 寻峰与峰积分：局部极大值、突出度与基点、半峰宽的算法与 `scipy.signal`
//! （`find_peaks`、`peak_prominences`、`peak_widths(rel_height=0.5)`）相同；
//! 面积与质心在积分区间（两侧到相邻峰之间的最低点）内、按两端连线以上的部分做梯形积分。

/// 一个峰。`index`、`left_base`、`right_base` 是样本序号；`left_ip` / `right_ip` 是半高处的分数序号。
#[derive(Clone, Debug, PartialEq)]
pub struct Peak {
    pub index: usize,
    pub x: f64,
    pub height: f64,
    pub prominence: f64,
    pub left_base: usize,
    pub right_base: usize,
    pub left_ip: f64,
    pub right_ip: f64,
    /// 半峰宽（样本数）。
    pub width: f64,
    /// 半峰宽（X 单位）。
    pub fwhm: f64,
    /// 积分区间（样本序号）：两侧到相邻峰之间（没有相邻峰时到突出度基点）的最低点。
    pub start: usize,
    pub end: usize,
    /// 积分区间两端连线以上的面积。
    pub area: f64,
    pub centroid: f64,
}

/// 局部极大值；平顶峰取平台中点（与 SciPy `_local_maxima_1d` 相同）。
pub fn local_maxima(y: &[f64]) -> Vec<usize> {
    let mut out = vec![];
    if y.len() < 3 {
        return out;
    }
    let last = y.len() - 1;
    let mut i = 1;
    while i < last {
        if y[i - 1] < y[i] {
            let mut ahead = i + 1;
            while ahead < last && y[ahead] == y[i] {
                ahead += 1;
            }
            if y[ahead] < y[i] {
                out.push((i + ahead - 1) / 2);
                i = ahead;
            }
        }
        i += 1;
    }
    out
}

/// 峰的突出度与左右基点。
fn prominence(y: &[f64], peak: usize) -> (f64, usize, usize) {
    let top = y[peak];
    let (mut left_min, mut left_base) = (top, peak);
    let mut i = peak as isize;
    while i >= 0 && y[i as usize] <= top {
        if y[i as usize] < left_min {
            left_min = y[i as usize];
            left_base = i as usize;
        }
        i -= 1;
    }
    let (mut right_min, mut right_base) = (top, peak);
    let mut i = peak;
    while i < y.len() && y[i] <= top {
        if y[i] < right_min {
            right_min = y[i];
            right_base = i;
        }
        i += 1;
    }
    (top - left_min.max(right_min), left_base, right_base)
}

/// 分数序号处的 X（相邻样本间线性插值）。
fn x_at(xs: &[f64], ip: f64) -> f64 {
    let i = (ip.floor() as usize).min(xs.len() - 1);
    if i + 1 >= xs.len() {
        return xs[i];
    }
    xs[i] + (xs[i + 1] - xs[i]) * (ip - i as f64)
}

/// 找出突出度不小于 `min_prominence` 的峰。`xs` 应按递增排列，长度与 `ys` 相同。
pub fn find_peaks(xs: &[f64], ys: &[f64], min_prominence: f64) -> Vec<Peak> {
    let found: Vec<(usize, f64, usize, usize)> = local_maxima(ys)
        .into_iter()
        .map(|p| {
            let (prom, lb, rb) = prominence(ys, p);
            (p, prom, lb, rb)
        })
        .filter(|f| f.1 >= min_prominence)
        .collect();
    let lowest = |a: usize, b: usize| (a..=b).min_by(|i, j| ys[*i].total_cmp(&ys[*j])).unwrap();
    let mut out = vec![];
    for (n, &(peak, prom, lb, rb)) in found.iter().enumerate() {
        // 半突出度高度处的左右交点
        let level = ys[peak] - 0.5 * prom;
        let mut i = peak;
        while lb < i && level < ys[i] {
            i -= 1;
        }
        let mut left_ip = i as f64;
        if ys[i] < level {
            left_ip += (level - ys[i]) / (ys[i + 1] - ys[i]);
        }
        let mut i = peak;
        while i < rb && level < ys[i] {
            i += 1;
        }
        let mut right_ip = i as f64;
        if ys[i] < level {
            right_ip -= (level - ys[i]) / (ys[i - 1] - ys[i]);
        }
        // 积分区间：到相邻峰（没有时到基点）之间的最低点
        let start = lowest(if n > 0 { found[n - 1].0.max(lb) } else { lb }, peak);
        let end = lowest(
            peak,
            if n + 1 < found.len() {
                found[n + 1].0.min(rb)
            } else {
                rb
            },
        );
        let base = |x: f64| {
            if xs[end] == xs[start] {
                ys[start]
            } else {
                ys[start] + (ys[end] - ys[start]) * (x - xs[start]) / (xs[end] - xs[start])
            }
        };
        let (mut area, mut moment) = (0.0, 0.0);
        for k in start..end {
            let (x0, x1) = (xs[k], xs[k + 1]);
            let (h0, h1) = (ys[k] - base(x0), ys[k + 1] - base(x1));
            area += 0.5 * (h0 + h1) * (x1 - x0);
            moment += 0.5 * (h0 * x0 + h1 * x1) * (x1 - x0);
        }
        out.push(Peak {
            index: peak,
            x: xs[peak],
            height: ys[peak],
            prominence: prom,
            left_base: lb,
            right_base: rb,
            left_ip,
            right_ip,
            width: right_ip - left_ip,
            fwhm: x_at(xs, right_ip) - x_at(xs, left_ip),
            start,
            end,
            area,
            centroid: if area != 0.0 { moment / area } else { xs[peak] },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spectrum() -> (Vec<f64>, Vec<f64>) {
        let xs: Vec<f64> = (0..201).map(|i| i as f64 * 0.1).collect();
        let mut ys: Vec<f64> = xs
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let i = i as f64;
                5.0 * (-((x - 4.0) / 0.6).powi(2)).exp()
                    + 3.0 * (-((x - 9.5) / 1.2).powi(2)).exp()
                    + 4.2 * (-((x - 15.0) / 0.8).powi(2)).exp()
                    + 0.5
                    + 0.05 * ((1.7 * i).sin() * 0.6 + (0.37 * i * i).cos() * 0.4)
            })
            .collect();
        ys[100] = ys[101];
        (xs, ys)
    }

    #[test]
    fn peaks_prominences_and_widths_match_scipy() {
        let (xs, ys) = spectrum();
        let peaks = find_peaks(&xs, &ys, 0.5);
        // scipy.signal.find_peaks(prominence=0.5) 与 peak_widths(rel_height=0.5)
        let idx: Vec<_> = peaks.iter().map(|p| p.index).collect();
        assert_eq!(idx, [40, 94, 150]);
        let bases: Vec<_> = peaks.iter().map(|p| (p.left_base, p.right_base)).collect();
        assert_eq!(bases, [(14, 173), (62, 124), (62, 173)]);
        let near = |got: f64, want: f64| assert!((got - want).abs() <= 1e-9 * want.abs(), "{got} vs {want}");
        for (p, (prom, w, l, r)) in peaks.iter().zip([
            (
                5.022371395716862,
                10.1635048349006,
                34.92089376877182,
                45.08439860367242,
            ),
            (
                3.001536723468866,
                20.122830202724714,
                85.01250810597298,
                105.1353383086977,
            ),
            (
                4.2454170121631885,
                13.39626859873124,
                143.31634099202995,
                156.7126095907612,
            ),
        ]) {
            near(p.prominence, prom);
            near(p.width, w);
            near(p.left_ip, l);
            near(p.right_ip, r);
            near(p.fwhm, 0.1 * w);
        }
        // numpy.trapezoid：区间两端连线以上的面积与质心
        for (p, (s, e, area, centroid)) in peaks.iter().zip([
            (14, 62, 5.552831392816941, 3.9884421169709565),
            (62, 124, 6.536641124760196, 9.486573716607879),
            (124, 173, 6.115590674300046, 15.003192938773305),
        ]) {
            assert_eq!((p.start, p.end), (s, e));
            near(p.area, area);
            near(p.centroid, centroid);
        }
        assert!(find_peaks(&xs, &ys, 4.5).iter().map(|p| p.index).eq([40]));
    }

    #[test]
    fn plateaus_and_edges_follow_scipy() {
        let z = [0.0, 1.0, 3.0, 3.0, 3.0, 1.0, 0.0, 2.0, 2.0, 0.5, 0.0];
        assert_eq!(local_maxima(&z), [3, 7]);
        let xs: Vec<f64> = (0..z.len()).map(|i| i as f64).collect();
        let p: Vec<_> = find_peaks(&xs, &z, 0.1).iter().map(|p| p.prominence).collect();
        assert_eq!(p, [3.0, 2.0]);
        // 端点不算峰，单调序列没有峰
        assert!(local_maxima(&[3.0, 2.0, 1.0]).is_empty());
        assert!(local_maxima(&[1.0, 2.0]).is_empty());
    }
}
