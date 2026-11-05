//! Discrete convolution follows NumPy alignment and does not multiply by dx.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvolutionMode {
    Full,
    Same,
    Valid,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SignalOperation {
    Convolve(ConvolutionMode),
    Deconvolve { epsilon: f64 },
}
fn direct(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut out = vec![0.; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            out[i + j] += x * y;
        }
    }
    out
}
fn transformed(a: &[f64], b: &[f64]) -> Vec<f64> {
    let full = a.len() + b.len() - 1;
    let n = full.next_power_of_two();
    let mut aa = vec![0.; n];
    let mut bb = vec![0.; n];
    aa[..a.len()].copy_from_slice(a);
    bb[..b.len()].copy_from_slice(b);
    let zero = vec![0.; n];
    let (ar, ai) = crate::fft::dft(&aa, &zero);
    let (br, bi) = crate::fft::dft(&bb, &zero);
    let re: Vec<_> = (0..n).map(|i| ar[i] * br[i] - ai[i] * bi[i]).collect();
    let im: Vec<_> = (0..n).map(|i| ar[i] * bi[i] + ai[i] * br[i]).collect();
    let (mut out, _) = crate::fft::inverse(&re, &im);
    out.truncate(full);
    out
}
pub fn convolve(a: &[f64], b: &[f64], mode: ConvolutionMode) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let full = if a.len() * b.len() <= 1_000_000 {
        direct(a, b)
    } else {
        transformed(a, b)
    };
    let (short, long) = (a.len().min(b.len()), a.len().max(b.len()));
    let (start, len) = match mode {
        ConvolutionMode::Full => (0, full.len()),
        ConvolutionMode::Same => ((short - 1) / 2, long),
        ConvolutionMode::Valid => (short - 1, long - short + 1),
    };
    full[start..start + len].to_vec()
}
pub fn deconvolve(y: &[f64], h: &[f64], epsilon: f64) -> Vec<f64> {
    if y.is_empty() || h.is_empty() {
        return Vec::new();
    }
    let n = (y.len() + h.len() - 1).next_power_of_two();
    let mut yy = vec![0.; n];
    let mut hh = vec![0.; n];
    yy[..y.len()].copy_from_slice(y);
    hh[..h.len()].copy_from_slice(h);
    let zero = vec![0.; n];
    let (yr, yi) = crate::fft::dft(&yy, &zero);
    let (hr, hi) = crate::fft::dft(&hh, &zero);
    let power: Vec<_> = (0..n).map(|i| hr[i] * hr[i] + hi[i] * hi[i]).collect();
    let max = power.iter().copied().fold(0., f64::max);
    let re: Vec<_> = (0..n)
        .map(|i| (yr[i] * hr[i] + yi[i] * hi[i]) / (power[i] + epsilon * max))
        .collect();
    let im: Vec<_> = (0..n)
        .map(|i| (yi[i] * hr[i] - yr[i] * hi[i]) / (power[i] + epsilon * max))
        .collect();
    let (mut result, _) = crate::fft::inverse(&re, &im);
    result.truncate(y.len());
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_and_fft_convolution_agree() {
        let a: Vec<_> = (0..47).map(|i| (i as f64 * 0.3).sin()).collect();
        let b: Vec<_> = (0..18).map(|i| (i as f64 * 0.7).cos()).collect();
        for (a, b) in direct(&a, &b).iter().zip(transformed(&a, &b)) {
            assert!((a - b).abs() < 1e-12);
        }
    }
}
