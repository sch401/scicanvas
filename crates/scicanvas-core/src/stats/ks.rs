//! KS probabilities follow SciPy 1.17.1 (_stats_py.py, _ksstats.py), BSD-3-Clause.
//! Copyright (c) 2001-2002 Enthought, Inc.; 2003-2026 SciPy Developers.
//! Full license: licenses/supplemental/scipy-1.17.1/LICENSE.txt.
use super::ln_gamma;
use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct Ks2 {
    pub n1: usize,
    pub n2: usize,
    pub d: f64,
    pub p: f64,
    pub method: &'static str,
}

/// 双侧两样本 KS；与 SciPy auto 相同，样本均 ≤10000 时使用精确概率。
pub fn ks_2samp(a: &[f64], b: &[f64]) -> Result<Ks2, String> {
    let sorted = |values: &[f64]| {
        let mut values: Vec<_> = values.iter().copied().filter(|v| v.is_finite()).collect();
        values.sort_by(f64::total_cmp);
        values
    };
    let (a, b) = (sorted(a), sorted(b));
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 {
        return Err("KS 双样本检验的两列都需要有效数值".into());
    }
    let (mut i, mut j, mut distance) = (0, 0, 0_u128);
    let mut d = 0.0_f64;
    while i < n || j < m {
        let next = if j == m || (i < n && a[i] < b[j]) { a[i] } else { b[j] };
        while i < n && a[i] == next {
            i += 1;
        }
        while j < m && b[j] == next {
            j += 1;
        }
        distance = distance.max((i as u128 * m as u128).abs_diff(j as u128 * n as u128));
        d = d.max((i as f64 / n as f64 - j as f64 / m as f64).abs());
    }
    let exact = n <= 10000 && m <= 10000;
    let p = if distance == 0 {
        1.0
    } else if exact {
        // 精确路径概率；直接累计越界概率，避免用 1-CDF 损失小 p 值。
        d = distance as f64 / (n * m) as f64;
        if n == m {
            let h = (distance / n as u128) as usize;
            let mut p = 0.0;
            for k in (0..=n / h).rev() {
                let mut term = 1.0;
                for j in 0..h {
                    term *= (n as f64 - (k * h + j) as f64) / (n + k * h + j + 1) as f64;
                }
                p = term * (1.0 - p);
            }
            2.0 * p
        } else {
            // ponytail: O(n*m) 时间、O(m) 空间，auto 精确分支最多 10000² 格。
            // 若更大的精确样本成为需求，改为 Hodges 带状路径递推。
            let mut outside = vec![0.0; m + 1];
            for i in 0..=n {
                for j in 0..=m {
                    if (i as u128 * m as u128).abs_diff(j as u128 * n as u128) >= distance {
                        outside[j] = 1.0;
                    } else if i + j != 0 {
                        outside[j] = (i as f64 * outside[j] + if j > 0 { j as f64 * outside[j - 1] } else { 0.0 })
                            / (i + j) as f64;
                    }
                }
            }
            outside[m]
        }
    } else {
        let effective = (n as f64 * m as f64 / (n as f64 + m as f64)).round_ties_even().max(1.0) as usize;
        kstwo_sf(effective, d)
    };
    Ok(Ks2 {
        n1: n,
        n2: m,
        d,
        p: p.clamp(0.0, 1.0),
        method: if exact { "exact" } else { "asymptotic" },
    })
}

fn smirnov_sf(n: usize, x: f64) -> f64 {
    let nf = n as f64;
    let mut logbinom = 0.0;
    let mut sum = (nf * (-x).ln_1p()).exp();
    for j in 1..=(nf * (1.0 - x)).floor() as usize {
        logbinom += ((n - j + 1) as f64 / j as f64).ln();
        let t = j as f64 / nf;
        let right = 1.0 - x - t;
        if right <= 0.0 {
            continue;
        }
        sum += (x.ln() + logbinom + (j - 1) as f64 * (x + t).ln() + (n - j) as f64 * right.ln()).exp();
    }
    sum
}

fn kstwo_sf(n: usize, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x >= 1.0 {
        return 0.0;
    }
    let nf = n as f64;
    let t = nf * x;
    if t <= 0.5 {
        return 1.0;
    }
    if t <= 1.0 {
        return -((ln_gamma(nf + 1.0) - nf * nf.ln()) + nf * (2.0 * t - 1.0).ln()).exp_m1();
    }
    if t >= nf - 1.0 {
        return 2.0 * (1.0 - x).powf(nf);
    }
    let nx2 = t * x;
    if nx2 >= 370.0 {
        return 0.0;
    }
    if x >= 0.5 || (n <= 140 && nx2 > 4.0) || (n > 140 && nx2 >= 2.2) {
        return 2.0 * smirnov_sf(n, x);
    }
    if n <= 140 || (n <= 100000 && nf * x.powf(1.5) <= 1.4) {
        return 1.0 - durbin_cdf(n, x);
    }
    1.0 - pelz_good_cdf(nf, x)
}

fn matrix_product(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let size = a.len();
    let mut c = vec![vec![0.0; size]; size];
    for (i, row) in a.iter().enumerate() {
        for (k, &v) in row.iter().enumerate() {
            for (j, out) in c[i].iter_mut().enumerate() {
                *out += v * b[k][j];
            }
        }
    }
    c
}

fn durbin_cdf(n: usize, d: f64) -> f64 {
    let nf = n as f64;
    let k = (nf * d).ceil() as usize;
    let h = k as f64 - nf * d;
    let m = 2 * k - 1;
    let mut v = vec![0.0; m];
    let mut w = vec![0.0; m];
    let mut fac = 1.0;
    for j in 1..=m {
        w[j - 1] = fac;
        fac /= j as f64;
        v[j - 1] = (1.0 - h.powi(j as i32)) * fac;
    }
    v[m - 1] = (1.0 + (2.0 * h - 1.0).max(0.0).powi(m as i32) - 2.0 * h.powi(m as i32)) * fac;
    let mut matrix = vec![vec![0.0; m]; m];
    for col in 1..m {
        for row in col - 1..m {
            matrix[row][col] = w[row + 1 - col];
        }
    }
    for i in 0..m {
        matrix[i][0] = v[i];
        matrix[m - 1][i] = v[m - 1 - i];
    }
    let mut power = vec![vec![0.0; m]; m];
    for (i, row) in power.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    let (mut exponent, mut matrix_exponent, mut nn) = (0, 0, n);
    let scale = 2.0_f64.powi(128);
    while nn > 0 {
        if nn % 2 == 1 {
            power = matrix_product(&power, &matrix);
            exponent += matrix_exponent;
        }
        matrix = matrix_product(&matrix, &matrix);
        matrix_exponent *= 2;
        if matrix[k - 1][k - 1] > scale {
            for row in &mut matrix {
                for v in row {
                    *v /= scale;
                }
            }
            matrix_exponent += 128;
        }
        nn /= 2;
    }
    let mut p = power[k - 1][k - 1];
    for i in 1..=n {
        p *= i as f64 / nf;
        if p.abs() < 1.0 / scale {
            p *= scale;
            exponent -= 128;
        }
    }
    p * 2.0_f64.powi(exponent)
}

fn pelz_good_cdf(n: f64, x: f64) -> f64 {
    let z = n.sqrt() * x;
    let z2 = z * z;
    let pi2 = PI * PI;
    let qlog = -pi2 / (8.0 * z2);
    if qlog < -708.0 {
        return 0.0;
    }
    let q = qlog.exp();
    let maxk = (16.0 * z / PI).ceil() as usize;
    let mut terms = [0.0; 4];
    for k in (1..=maxk).rev() {
        let m2 = ((2 * k - 1) as f64).powi(2);
        let coeff = [
            1.0,
            -z2 + pi2 * m2 / 4.0,
            6.0 * z.powi(6)
                + 2.0 * z.powi(4)
                + (2.0 * z.powi(4) - 5.0 * z2) * pi2 * m2 / 4.0
                + pi2 * pi2 * (1.0 - 2.0 * z2) * m2 * m2 / 16.0,
            -30.0 * z.powi(6) - 90.0 * z.powi(8)
                + pi2 * (135.0 * z.powi(4) - 96.0 * z.powi(6)) * m2 / 4.0
                + pi2 * pi2 * (-60.0 * z2 + 212.0 * z.powi(4)) * m2 * m2 / 16.0
                + pi2.powi(3) * (5.0 - 30.0 * z2) * m2.powi(3) / 64.0,
        ];
        for j in 0..4 {
            terms[j] = terms[j] * q.powi((8 * k) as i32) + coeff[j];
        }
    }
    let sqrt2pi = (2.0 * PI).sqrt();
    for (term, denominator) in terms
        .iter_mut()
        .zip([z, 6.0 * z.powi(4), 72.0 * z.powi(7), 6480.0 * z.powi(10)])
    {
        *term *= q * sqrt2pi / denominator;
    }
    let q = (-pi2 / (2.0 * z2)).exp();
    for k in (1..=maxk).rev() {
        let k2 = (k as f64).powi(2);
        let power = q.powf(k2);
        terms[2] -= pi2 * sqrt2pi * k2 * power / (36.0 * z.powi(3));
        terms[3] += pi2 * sqrt2pi * (3.0 * z2 - pi2 * k2) * k2 * power / (216.0 * z.powi(6));
    }
    terms.iter().enumerate().map(|(i, t)| t / n.powf(i as f64 / 2.0)).sum()
}
