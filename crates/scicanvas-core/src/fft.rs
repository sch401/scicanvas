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
