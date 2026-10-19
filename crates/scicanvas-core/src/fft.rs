//! 傅里叶变换：2 的幂长度用基 2 FFT，其余长度用 Bluestein 算法，结果与 `numpy.fft` 一致。
//! `spectrum` 给出实数信号的单边频谱：幅值按正弦振幅归一（加窗时按相干增益补偿），相位单位为度。

use std::f64::consts::PI;

/// 原地基 2 FFT（`re.len()` 必须是 2 的幂）。`inverse` 为真时做不归一化的逆变换。
fn radix2(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let angle = sign * 2.0 * PI / len as f64;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (angle * k as f64).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * c - im[b] * s;
                let ti = re[b] * s + im[b] * c;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

/// 任意长度的离散傅里叶变换 X_k = Σ x_n e^{-2πikn/N}，返回 (实部, 虚部)。
pub fn dft(re_in: &[f64], im_in: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = re_in.len();
    if n == 0 {
        return (vec![], vec![]);
    }
    if n.is_power_of_two() {
        let (mut re, mut im) = (re_in.to_vec(), im_in.to_vec());
        radix2(&mut re, &mut im, false);
        return (re, im);
    }
    // Bluestein：kn = (k² + n² − (k−n)²)/2，把变换写成与线性调频序列的卷积
    let chirp: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            // k² 对 2N 取模，避免大角度丢精度
            let a = PI * ((k * k) % (2 * n)) as f64 / n as f64;
            (a.cos(), -a.sin())
        })
        .collect();
    let m = (2 * n - 1).next_power_of_two();
    let (mut ar, mut ai) = (vec![0.0; m], vec![0.0; m]);
    for k in 0..n {
        let (c, s) = chirp[k];
        ar[k] = re_in[k] * c - im_in[k] * s;
        ai[k] = re_in[k] * s + im_in[k] * c;
    }
    let (mut br, mut bi) = (vec![0.0; m], vec![0.0; m]);
    for k in 0..n {
        let (c, s) = chirp[k];
        br[k] = c;
        bi[k] = -s;
        if k > 0 {
            br[m - k] = c;
            bi[m - k] = -s;
        }
    }
    radix2(&mut ar, &mut ai, false);
    radix2(&mut br, &mut bi, false);
    for k in 0..m {
        let (r, i) = (ar[k] * br[k] - ai[k] * bi[k], ar[k] * bi[k] + ai[k] * br[k]);
        ar[k] = r;
        ai[k] = i;
    }
    radix2(&mut ar, &mut ai, true);
    let scale = 1.0 / m as f64;
    (0..n)
        .map(|k| {
            let (r, i) = (ar[k] * scale, ai[k] * scale);
            let (c, s) = chirp[k];
            (r * c - i * s, r * s + i * c)
        })
        .unzip()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Rect,
    Hann,
    Hamming,
}

impl Window {
    pub const ALL: [Window; 3] = [Window::Rect, Window::Hann, Window::Hamming];

    pub fn label(self) -> &'static str {
        match self {
            Window::Rect => "矩形（不加窗）",
            Window::Hann => "Hann",
            Window::Hamming => "Hamming",
        }
    }

    /// 对称窗，与 `numpy.hanning` / `numpy.hamming` 相同。
    fn weights(self, n: usize) -> Vec<f64> {
        let a = match self {
            Window::Rect => return vec![1.0; n],
            Window::Hann => 0.5,
            Window::Hamming => 0.54,
        };
        if n == 1 {
            return vec![1.0];
        }
        (0..n)
            .map(|i| a - (1.0 - a) * (2.0 * PI * i as f64 / (n - 1) as f64).cos())
            .collect()
    }
}

/// 实数信号的单边频谱，共 ⌊N/2⌋ + 1 个频点。
#[derive(Clone, Debug, PartialEq)]
pub struct Spectrum {
    pub freq: Vec<f64>,
    pub re: Vec<f64>,
    pub im: Vec<f64>,
    /// 单边振幅：振幅为 A 的正弦（落在频点上时）峰值为 A，直流为平均值。
    pub amplitude: Vec<f64>,
    pub phase_deg: Vec<f64>,
    /// 振幅的平方。
    pub power: Vec<f64>,
}

/// `ys` 为等间距采样（间隔 `dt`），先乘窗函数再变换。
pub fn spectrum(ys: &[f64], dt: f64, window: Window) -> Result<Spectrum, String> {
    let n = ys.len();
    if n < 2 {
        return Err("FFT 至少需要 2 个数据点".into());
    }
    if ys.iter().any(|y| !y.is_finite()) {
        return Err("数据里有缺失值或非数值，FFT 需要连续的有效采样".into());
    }
    if !(dt.is_finite() && dt > 0.0) {
        return Err("采样间隔必须是正数".into());
    }
    let w = window.weights(n);
    let gain: f64 = w.iter().sum();
    let x: Vec<f64> = ys.iter().zip(&w).map(|(y, w)| y * w).collect();
    let (re, im) = dft(&x, &vec![0.0; n]);
    let half = n / 2 + 1;
    let mut s = Spectrum {
        freq: (0..half).map(|k| k as f64 / (n as f64 * dt)).collect(),
        re: re[..half].to_vec(),
        im: im[..half].to_vec(),
        amplitude: vec![],
        phase_deg: vec![],
        power: vec![],
    };
    for k in 0..half {
        // 直流和偶数长度的奈奎斯特频点只出现一次，其余频点合并正负频率
        let both = k > 0 && !(n.is_multiple_of(2) && k == n / 2);
        let amp = s.re[k].hypot(s.im[k]) / gain * if both { 2.0 } else { 1.0 };
        s.amplitude.push(amp);
        s.phase_deg.push(s.im[k].atan2(s.re[k]).to_degrees());
        s.power.push(amp * amp);
    }
    Ok(s)
}

/// 由 X 列求采样间隔；不等间距（相对偏差超过 1e-6）时报错。
/// FFT 滤波类型；频率单位与 1/X 相同（X 以 s 为单位时为 Hz）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterKind {
    LowPass(f64),
    HighPass(f64),
    BandPass(f64, f64),
    BandStop(f64, f64),
}

impl FilterKind {
    pub fn label(self) -> String {
        match self {
            FilterKind::LowPass(f) => format!("低通 ≤ {f}"),
            FilterKind::HighPass(f) => format!("高通 ≥ {f}"),
            FilterKind::BandPass(a, b) => format!("带通 {a}–{b}"),
            FilterKind::BandStop(a, b) => format!("带阻 {a}–{b}"),
        }
    }

    fn passes(self, f: f64) -> bool {
        match self {
            FilterKind::LowPass(c) => f <= c,
            FilterKind::HighPass(c) => f >= c,
            FilterKind::BandPass(a, b) => f >= a && f <= b,
            FilterKind::BandStop(a, b) => f < a || f > b,
        }
    }
}

/// 理想 FFT 滤波：变换后把不通过的频率分量置零再逆变换（与 numpy rfft / irfft 掩模相同）。
/// `dt` 是采样间隔。
pub fn filter(ys: &[f64], dt: f64, kind: FilterKind) -> Result<Vec<f64>, String> {
    let n = ys.len();
    if n < 2 {
        return Err("至少需要 2 个数据点".into());
    }
    let (mut re, mut im) = dft(ys, &vec![0.0; n]);
    for k in 0..n {
        let f = k.min(n - k) as f64 / (n as f64 * dt);
        if !kind.passes(f) {
            re[k] = 0.0;
            im[k] = 0.0;
        }
    }
    // 逆变换：共轭后正变换再共轭，除以 n
    let neg: Vec<f64> = im.iter().map(|v| -v).collect();
    let (out, _) = dft(&re, &neg);
    Ok(out.iter().map(|v| v / n as f64).collect())
}

/// IFFT 的输入：单边谱（实数信号，`odd` 表示原信号点数为奇数）或双边谱（复数，按 numpy.fft 顺序）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InverseInput {
    OneSided { odd: bool },
    TwoSided,
}

impl InverseInput {
    /// 由频谱行数 m 得到输出点数。
    pub fn points(self, m: usize) -> usize {
        match self {
            InverseInput::OneSided { odd: false } => 2 * m.saturating_sub(1),
            InverseInput::OneSided { odd: true } => (2 * m).saturating_sub(1),
            InverseInput::TwoSided => m,
        }
    }
}

/// 逆离散傅里叶变换 x_n = (1/N) Σ X_k e^{2πikn/N}（与 `numpy.fft.ifft` 相同），返回 (实部, 虚部)。
pub fn inverse(re: &[f64], im: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = re.len();
    // 共轭后正变换再共轭，除以 N
    let neg: Vec<f64> = im.iter().map(|v| -v).collect();
    let (r, i) = dft(re, &neg);
    (
        r.iter().map(|v| v / n as f64).collect(),
        i.iter().map(|v| -v / n as f64).collect(),
    )
}

/// 单边谱逆变换为 `n` 点实数信号（与 `numpy.fft.irfft(X, n)` 相同）：
/// 用前 ⌊n/2⌋+1 个频点（不足补零），按共轭对称补全负频率；直流和偶数长度奈奎斯特频点只取实部。
pub fn inverse_real(re: &[f64], im: &[f64], n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    let half = n / 2 + 1;
    let at = |v: &[f64], k: usize| v.get(k).copied().unwrap_or(0.0);
    let (mut fr, mut fi) = (vec![0.0; n], vec![0.0; n]);
    for k in 0..half.min(n) {
        let nyquist = n.is_multiple_of(2) && k == n / 2;
        fr[k] = at(re, k);
        fi[k] = if k == 0 || nyquist { 0.0 } else { at(im, k) };
        if k > 0 && !nyquist {
            fr[n - k] = fr[k];
            fi[n - k] = -fi[k];
        }
    }
    inverse(&fr, &fi).0
}

pub fn sample_interval(xs: &[f64]) -> Result<f64, String> {
    if xs.len() < 2 {
        return Err("FFT 至少需要 2 个数据点".into());
    }
    let dt = (xs[xs.len() - 1] - xs[0]) / (xs.len() - 1) as f64;
    if !(dt.is_finite() && dt > 0.0) {
        return Err("X 必须严格递增".into());
    }
    if xs.windows(2).any(|w| ((w[1] - w[0]) - dt).abs() > 1e-6 * dt.abs()) {
        return Err("X 不等间距；FFT 需要等间距采样，可先用插值重采样到等间距网格".into());
    }
    Ok(dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_transforms_match_numpy_and_round_trip() {
        // numpy.fft.irfft / ifft 参考值（numpy 2.4.4）
        let re = [1.0, -0.5, 2.0, 0.25, -1.0];
        let im = [0.3, 1.5, -0.75, 0.5, 0.2];
        let cases: [(usize, &[f64]); 3] = [
            (
                8,
                &[
                    0.4375,
                    -0.04863591206575146,
                    -0.75,
                    -0.1584708691207961,
                    0.5625,
                    0.9236359120657515,
                    -0.25,
                    0.2834708691207961,
                ],
            ),
            (
                9,
                &[
                    0.2777777777777778,
                    0.12266098316272755,
                    -0.6703066826041761,
                    -0.36039160872708326,
                    0.21602554342684321,
                    0.7632131316682589,
                    0.5826138309493054,
                    -0.3773616611843837,
                    0.4457686855307301,
                ],
            ),
            (
                7,
                &[
                    0.6428571428571428,
                    -0.3258634699834446,
                    -0.694769642782539,
                    0.1191916078746647,
                    1.1047134230851052,
                    0.10345260371854043,
                    0.05041833523053019,
                ],
            ),
        ];
        for (n, want) in cases {
            let got = inverse_real(&re, &im, n);
            assert_eq!(got.len(), n);
            for (g, w) in got.iter().zip(want) {
                assert!((g - w).abs() < 1e-12, "n={n}: {got:?}");
            }
        }
        let (cr, ci) = inverse(&re, &im);
        let want_re = [
            0.35000000000000003,
            -0.35709112694483225,
            0.1911744511113793,
            0.9723510404510421,
            -0.15643436461758897,
        ];
        let want_im = [
            0.35000000000000003,
            0.5063471177381105,
            -0.5046078832802859,
            0.043574627667826965,
            -0.09531386212565147,
        ];
        for i in 0..5 {
            assert!((cr[i] - want_re[i]).abs() < 1e-12 && (ci[i] - want_im[i]).abs() < 1e-12);
        }
        // 往返：任意长度的实数信号，正变换后取单边谱再逆变换回原信号
        for n in [7usize, 8, 12, 13] {
            let ys: Vec<f64> = (0..n).map(|i| (i as f64 * 0.7).sin() + 0.1 * i as f64).collect();
            let (fr, fi) = dft(&ys, &vec![0.0; n]);
            let back = inverse_real(&fr[..n / 2 + 1], &fi[..n / 2 + 1], n);
            let (cr, ci) = inverse(&fr, &fi);
            for i in 0..n {
                assert!((back[i] - ys[i]).abs() < 1e-12 && (cr[i] - ys[i]).abs() < 1e-12 && ci[i].abs() < 1e-12);
            }
        }
    }

    #[test]
    fn filters_match_numpy_masked_rfft() {
        let ys: Vec<f64> = (0..64)
            .map(|i| {
                let t = i as f64 * 0.01;
                (2.0 * std::f64::consts::PI * 3.0 * t).sin() + 0.5 * (2.0 * std::f64::consts::PI * 30.0 * t).sin() + 0.2
            })
            .collect();
        // numpy.fft.irfft(np.where(mask, rfft(y), 0))
        for (kind, want) in [
            (
                FilterKind::LowPass(10.0),
                [
                    -0.0017716005757181623,
                    1.0586138974607586,
                    0.1392815482693044,
                    -0.2634090143164475,
                ],
            ),
            (
                FilterKind::HighPass(10.0),
                [
                    0.20177160057571816,
                    -0.04959690308581079,
                    0.29182055834761683,
                    -0.467907601578485,
                ],
            ),
            (
                FilterKind::BandPass(2.0, 20.0),
                [
                    -0.21660408898630626,
                    0.7557373300394807,
                    -0.004729463128946093,
                    -0.6059354982932909,
                ],
            ),
        ] {
            let out = filter(&ys, 0.01, kind).unwrap();
            for (i, w) in [0, 5, 17, 63].into_iter().zip(want) {
                assert!((out[i] - w).abs() < 1e-12, "{kind:?} {i}: {} vs {w}", out[i]);
            }
        }
        // 带阻与带通互补
        let pass = filter(&ys, 0.01, FilterKind::BandPass(2.0, 20.0)).unwrap();
        let stop = filter(&ys, 0.01, FilterKind::BandStop(2.0, 20.0)).unwrap();
        for i in 0..64 {
            assert!((pass[i] + stop[i] - ys[i]).abs() < 1e-12);
        }
    }

    fn signal(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| {
                let t = i as f64 * 0.01;
                3.0 * (2.0 * PI * 5.0 * t).sin() + 1.5 * (2.0 * PI * 12.5 * t).cos() + 0.5
            })
            .collect()
    }

    fn close(got: f64, want: f64) {
        assert!((got - want).abs() <= 1e-9 * want.abs().max(1.0), "{got} vs {want}");
    }

    #[test]
    fn matches_numpy_rfft_for_bluestein_and_radix2_lengths() {
        // numpy.fft.rfft(signal(N)) 在频点 0, 1, 5, 7, 12, N/2 的值
        let cases: [(usize, [f64; 6], [f64; 6]); 2] = [
            (
                100,
                [
                    51.5,
                    1.4999999999999922,
                    1.49999999999992,
                    1.4999999999999878,
                    1.499999999999956,
                    1.5000000000000036,
                ],
                [
                    0.0,
                    0.32375153435495807,
                    -148.09991393777875,
                    3.230164176969293,
                    46.968615910193314,
                    0.0,
                ],
            ),
            (
                64,
                [
                    37.11745772312711,
                    5.831544087049775,
                    -6.059362240707004,
                    -3.2254913723673697,
                    -1.9901023765236996,
                    -1.5907462093722344,
                ],
                [
                    0.0,
                    -3.168715858491079,
                    9.727127457284041,
                    5.0830430355548275,
                    2.318886153768056,
                    0.0,
                ],
            ),
        ];
        for (n, re, im) in cases {
            let s = spectrum(&signal(n), 0.01, Window::Rect).unwrap();
            assert_eq!(s.freq.len(), n / 2 + 1);
            close(s.freq[1], 100.0 / n as f64);
            for (j, k) in [0, 1, 5, 7, 12, n / 2].into_iter().enumerate() {
                close(s.re[k], re[j]);
                close(s.im[k], im[j]);
            }
        }
    }

    #[test]
    fn amplitudes_recover_sine_amplitudes_and_windows_match_numpy() {
        // N = 200、间隔 0.01：5 Hz 和 12.5 Hz 都落在频点上
        let s = spectrum(&signal(200), 0.01, Window::Rect).unwrap();
        close(s.amplitude[0], 0.5);
        close(s.amplitude[10], 3.0);
        close(s.amplitude[25], 1.5);
        close(s.phase_deg[10], -90.0);
        close(s.power[10], 9.0);
        // 加窗：numpy |rfft(y·w)| 除以窗权和，再乘 2
        let hann = spectrum(&signal(100), 0.01, Window::Hann).unwrap();
        close(hann.amplitude[5], 2.0 * 74.26726275402766 / 49.5);
        close(hann.amplitude[6], 2.0 * 37.64741427348304 / 49.5);
        let hamming = spectrum(&signal(64), 0.01, Window::Hamming).unwrap();
        close(hamming.amplitude[5], 2.0 * 1.5449562470663016 / 34.1);
    }

    #[test]
    fn odd_lengths_and_invalid_input() {
        // 奇数长度没有奈奎斯特频点，最后一个频点也合并正负频率
        let s = spectrum(&[1.0, -1.0, 1.0], 1.0, Window::Rect).unwrap();
        assert_eq!(s.freq.len(), 2);
        close(s.amplitude[1], 2.0 * 2.0 / 3.0);
        assert!(spectrum(&[1.0], 1.0, Window::Rect).is_err());
        assert!(spectrum(&[1.0, f64::NAN], 1.0, Window::Rect).is_err());
        close(sample_interval(&[0.0, 0.5, 1.0, 1.5]).unwrap(), 0.5);
        assert!(sample_interval(&[0.0, 0.5, 1.2]).unwrap_err().contains("不等间距"));
        assert!(sample_interval(&[1.0, 0.0]).is_err());
    }
}
