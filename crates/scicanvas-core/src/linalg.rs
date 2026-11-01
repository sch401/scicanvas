//! Column pivoted Householder QR least squares.

pub struct LeastSquares {
    pub coef: Vec<f64>,
    pub cov_unscaled: Vec<Vec<f64>>,
    pub rss: f64,
    pub rank: usize,
}

#[derive(Debug)]
pub struct RankDeficient {
    pub rank: usize,
}

pub fn lstsq(a: &[Vec<f64>], y: &[f64]) -> Result<LeastSquares, RankDeficient> {
    let n = a.len();
    let k = a.first().map_or(0, Vec::len);
    if k == 0 || n < k || y.len() != n || a.iter().any(|row| row.len() != k) {
        return Err(RankDeficient { rank: 0 });
    }
    let mut a = a.to_vec();
    let mut b = y.to_vec();
    let mut perm: Vec<usize> = (0..k).collect();
    for j in 0..k {
        let pivot = (j..k)
            .max_by(|&x, &y| {
                let norm = |c| (j..n).fold(0.0_f64, |s, i| s.hypot(a[i][c]));
                norm(x).total_cmp(&norm(y))
            })
            .expect("nonempty column range");
        for row in &mut a {
            row.swap(j, pivot);
        }
        perm.swap(j, pivot);
        let norm = (j..n).fold(0.0_f64, |s, i| s.hypot(a[i][j]));
        if norm == 0.0 {
            continue;
        }
        let alpha = -norm.copysign(a[j][j]);
        let mut v: Vec<f64> = (j..n).map(|i| a[i][j]).collect();
        v[0] -= alpha;
        let vn = v.iter().fold(0.0_f64, |s, x| s.hypot(*x));
        for x in &mut v {
            *x /= vn;
        }
        let mut c = j;
        while c < k {
            let dot: f64 = (j..n).map(|i| v[i - j] * a[i][c]).sum();
            for i in j..n {
                a[i][c] -= 2.0 * v[i - j] * dot;
            }
            c += 1;
        }
        let dot: f64 = (j..n).map(|i| v[i - j] * b[i]).sum();
        for i in j..n {
            b[i] -= 2.0 * v[i - j] * dot;
        }
        a[j][j] = alpha;
    }
    let tol = n.max(k) as f64 * f64::EPSILON * a[0][0].abs();
    let rank = (0..k).filter(|&i| a[i][i].abs() > tol).count();
    if rank < k {
        return Err(RankDeficient { rank });
    }
    let mut z = vec![0.0; k];
    for i in (0..k).rev() {
        z[i] = (b[i] - (i + 1..k).map(|j| a[i][j] * z[j]).sum::<f64>()) / a[i][i];
    }
    let mut coef = vec![0.0; k];
    for i in 0..k {
        coef[perm[i]] = z[i];
    }
    let mut rinv = vec![vec![0.0; k]; k];
    let mut col = 0;
    while col < k {
        for i in (0..k).rev() {
            rinv[i][col] = (f64::from(i == col) - (i + 1..k).map(|j| a[i][j] * rinv[j][col]).sum::<f64>()) / a[i][i];
        }
        col += 1;
    }
    let mut cov_unscaled = vec![vec![0.0; k]; k];
    for i in 0..k {
        for j in 0..k {
            cov_unscaled[perm[i]][perm[j]] = (0..k).map(|h| rinv[i][h] * rinv[j][h]).sum();
        }
    }
    Ok(LeastSquares {
        coef,
        cov_unscaled,
        rss: b[k..].iter().map(|v| v * v).sum(),
        rank,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_and_rank_deficient() {
        let fit = lstsq(&[vec![1.0, 1.0], vec![1.0, 2.0], vec![1.0, 3.0]], &[1.0, 2.0, 3.0]).unwrap();
        assert!(fit.coef[0].abs() < 1e-12 && (fit.coef[1] - 1.0).abs() < 1e-12 && fit.rss < 1e-24);
        assert_eq!(
            lstsq(&[vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]], &[1.0, 2.0, 3.0])
                .err()
                .unwrap()
                .rank,
            1
        );
    }
}
