use super::*;
pub const MAX_GLOBAL_PARAMS: usize = 60;
pub type FitData = (Vec<f64>, Vec<f64>, Option<Vec<f64>>);

#[derive(Clone, Debug)]
pub struct GlobalSpec {
    pub base: FitFunction,
    pub groups: Vec<FitData>,
    pub shared: Vec<bool>,
}
#[derive(Clone, Debug)]
pub struct GlobalFit {
    pub per_group: Vec<NlFit>,
    pub joint: NlFit,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalLayout {
    pub names: Vec<String>,
    pub indices: Vec<Vec<usize>>,
}

pub fn global_layout(base: &FitFunction, groups: usize, shared: &[bool]) -> Result<GlobalLayout, String> {
    let params = base.params();
    if groups == 0 || shared.len() != params.len() {
        return Err("请选择数据集，并为每个参数设置共享方式".into());
    }
    let mut names = Vec::new();
    let mut indices = vec![vec![0; params.len()]; groups];
    for (j, name) in params.iter().enumerate() {
        if shared[j] {
            let index = names.len();
            names.push((*name).into());
            for group in &mut indices {
                group[j] = index;
            }
        } else {
            for (i, group) in indices.iter_mut().enumerate() {
                group[j] = names.len();
                names.push(if groups == 1 {
                    (*name).into()
                } else {
                    format!("{name}_{}", i + 1)
                });
            }
        }
    }
    if names.len() > MAX_GLOBAL_PARAMS {
        return Err(format!("全局拟合最多 {MAX_GLOBAL_PARAMS} 个参数"));
    }
    let unique: std::collections::HashSet<_> = names.iter().collect();
    if unique.len() != names.len() {
        return Err("参数展开后重名，请调整自定义参数名称".into());
    }
    Ok(GlobalLayout { names, indices })
}

#[doc(hidden)]
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedModel {
    pub(super) base: FitFunction,
    pub(super) names: Vec<String>,
    pub(super) initial: Vec<f64>,
    indices: Vec<Vec<usize>>,
    points: Vec<(usize, f64)>,
}
impl IndexedModel {
    pub(super) fn eval(&self, index: f64, parameters: &[f64]) -> f64 {
        let Some(&(group, x)) = self.points.get(index as usize) else {
            return f64::NAN;
        };
        let local: Vec<_> = self.indices[group].iter().map(|&i| parameters[i]).collect();
        self.base.eval(x, &local)
    }
}

fn initial_values(spec: &GlobalSpec, layout: &GlobalLayout) -> Result<Vec<f64>, String> {
    let guesses: Vec<_> = spec
        .groups
        .iter()
        .map(|(x, y, _)| spec.base.guess(x, y).ok_or("自定义全局模型请提供初值"))
        .collect::<Result<_, _>>()?;
    let mut values = vec![0.; layout.names.len()];
    for j in 0..spec.shared.len() {
        if spec.shared[j] {
            let mut candidates: Vec<_> = guesses.iter().map(|p| p[j]).collect();
            candidates.sort_by(f64::total_cmp);
            values[layout.indices[0][j]] =
                (candidates[(candidates.len() - 1) / 2] + candidates[candidates.len() / 2]) / 2.;
        } else {
            for (i, guess) in guesses.iter().enumerate() {
                values[layout.indices[i][j]] = guess[j];
            }
        }
    }
    Ok(values)
}

/// 单组独立参数直接调用普通拟合，保留逐位一致；其余场景只包装模型，不改求解器。
pub fn fit_global(
    mut spec: GlobalSpec,
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
    constraints: &str,
) -> Result<GlobalFit, String> {
    let weighted = spec.groups.iter().filter(|g| g.2.is_some()).count();
    if weighted != 0 && weighted != spec.groups.len() {
        return Err("全局拟合中各组需同时提供或同时不提供 Y 误差".into());
    }
    if spec.base == FitFunction::Builtin(Model::SineDamp) {
        let origin = spec
            .groups
            .iter()
            .map(|(x, y, s)| super::extended::data_origin(x, y, s.as_deref()))
            .min_by(f64::total_cmp)
            .unwrap_or(0.0);
        spec.base = FitFunction::SineDamp { origin };
    }
    let layout = global_layout(&spec.base, spec.groups.len(), &spec.shared)?;
    if spec
        .groups
        .iter()
        .any(|(x, y, s)| x.len() != y.len() || s.as_ref().is_some_and(|s| s.len() != x.len()))
    {
        return Err("各组 X、Y 和误差长度应相同".into());
    }
    if spec.groups.len() == 1 {
        let (x, y, s) = &spec.groups[0];
        let parsed = parse_constraints_for_function(&spec.base, constraints)?;
        let fit = fit_function_weighted(spec.base.clone(), x, y, s.as_deref(), initial, limits, &parsed)?;
        return Ok(GlobalFit {
            per_group: vec![fit.clone()],
            joint: fit,
        });
    }
    if spec.shared.iter().all(|s| *s) {
        let x: Vec<_> = spec.groups.iter().flat_map(|g| g.0.iter().copied()).collect();
        let y: Vec<_> = spec.groups.iter().flat_map(|g| g.1.iter().copied()).collect();
        let weighted = spec.groups.iter().any(|g| g.2.is_some());
        let sigma: Vec<_> = spec
            .groups
            .iter()
            .flat_map(|g| g.2.clone().unwrap_or_else(|| vec![1.; g.0.len()]))
            .collect();
        let parsed = parse_constraints_for_function(&spec.base, constraints)?;
        let joint = fit_function_weighted(
            spec.base.clone(),
            &x,
            &y,
            weighted.then_some(sigma.as_slice()),
            initial,
            limits,
            &parsed,
        )?;
        return split_result(spec, &layout, joint);
    }
    fit_indexed(spec, initial, limits, constraints)
}

pub(crate) fn fit_indexed(
    mut spec: GlobalSpec,
    initial: Option<&[f64]>,
    limits: &[ParamLimits],
    constraints: &str,
) -> Result<GlobalFit, String> {
    let layout = global_layout(&spec.base, spec.groups.len(), &spec.shared)?;
    for group in &mut spec.groups {
        let (x, y, w) = spec.base.points(&group.0, &group.1, group.2.as_deref())?;
        if x.is_empty() {
            return Err("每个数据集至少需要一个有效点".into());
        }
        let sigma = group.2.as_ref().map(|_| w.iter().map(|w| 1. / w).collect());
        *group = (x, y, sigma);
    }
    let starting = match initial {
        Some(p) => p.to_vec(),
        None => initial_values(&spec, &layout)?,
    };
    let points: Vec<_> = spec
        .groups
        .iter()
        .enumerate()
        .flat_map(|(g, data)| data.0.iter().map(move |&x| (g, x)))
        .collect();
    let x: Vec<_> = (0..points.len()).map(|i| i as f64).collect();
    let y: Vec<_> = spec.groups.iter().flat_map(|g| g.1.iter().copied()).collect();
    let weighted = spec.groups.iter().any(|g| g.2.is_some());
    let sigma: Vec<_> = spec
        .groups
        .iter()
        .flat_map(|g| g.2.clone().unwrap_or_else(|| vec![1.; g.0.len()]))
        .collect();
    let indexed = FitFunction::Indexed(std::sync::Arc::new(IndexedModel {
        base: spec.base.clone(),
        names: layout.names.clone(),
        initial: starting.clone(),
        indices: layout.indices.clone(),
        points,
    }));
    let parsed = parse_constraints_for_function(&indexed, constraints)?;
    let joint = fit_function_weighted(
        indexed,
        &x,
        &y,
        weighted.then_some(sigma.as_slice()),
        Some(&starting),
        limits,
        &parsed,
    )?;
    split_result(spec, &layout, joint)
}

fn split_result(spec: GlobalSpec, layout: &GlobalLayout, joint: NlFit) -> Result<GlobalFit, String> {
    let mut per_group = Vec::new();
    for (i, (x, y, sigma)) in spec.groups.iter().enumerate() {
        let (x, y, w) = spec.base.points(x, y, sigma.as_deref())?;
        if x.is_empty() {
            return Err("数据集中没有有效点".into());
        }
        let indices = &layout.indices[i];
        let mut fit = joint.clone();
        fit.model = spec.base.clone();
        fit.params = indices.iter().map(|&i| joint.params[i]).collect();
        fit.se = indices.iter().map(|&i| joint.se[i]).collect();
        fit.fixed = indices.iter().map(|&i| joint.fixed[i]).collect();
        fit.at_bound = indices.iter().map(|&i| joint.at_bound[i]).collect();
        fit.cov = indices
            .iter()
            .map(|&i| indices.iter().map(|&j| joint.cov[i][j]).collect())
            .collect();
        fit.n = x.len();
        fit.dof = fit.n.saturating_sub(fit.fixed.iter().filter(|fixed| !**fixed).count());
        fit.weighted = sigma.is_some();
        let total_w = w.iter().map(|w| w * w).sum::<f64>();
        let mean = y.iter().zip(&w).map(|(y, w)| y * w * w).sum::<f64>() / total_w;
        let tss = y.iter().zip(&w).map(|(y, w)| ((y - mean) * w).powi(2)).sum::<f64>();
        fit.rss = x
            .iter()
            .zip(&y)
            .zip(&w)
            .map(|((x, y), w)| ((y - fit.eval(*x)) * w).powi(2))
            .sum();
        fit.r2 = if tss > 0. { 1. - fit.rss / tss } else { f64::NAN };
        fit.reduced_chi2 = if fit.dof > 0 {
            fit.rss / fit.dof as f64
        } else {
            f64::NAN
        };
        fit.adj_r2 = 1. - (1. - fit.r2) * (fit.n - 1) as f64 / fit.dof as f64;
        fit.x_min = x[0];
        fit.x_max = x[x.len() - 1];
        per_group.push(fit);
    }
    Ok(GlobalFit { per_group, joint })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_indexed_matches_ordinary_weighted_and_unweighted() {
        for weighted in [false, true] {
            for count in [1, 2] {
                let mut groups = Vec::new();
                let mut direct = Vec::new();
                for group in 0..count {
                    let x: Vec<_> = (0..41).map(|i| i as f64 * 0.2).collect();
                    let y: Vec<_> = x
                        .iter()
                        .enumerate()
                        .map(|(i, x)| {
                            0.3 + group as f64 * 0.2
                                + (5. + group as f64) * (-x / (1.7 + group as f64 * 0.1)).exp()
                                + 0.01 * (i as f64).sin()
                        })
                        .collect();
                    let sigma = weighted.then(|| x.iter().map(|x| 0.05 + 0.01 * x).collect::<Vec<_>>());
                    direct.push(
                        fit_function_weighted(
                            FitFunction::Builtin(Model::ExpDec1),
                            &x,
                            &y,
                            sigma.as_deref(),
                            Some(&[0.2, 4., 1.5]),
                            &[],
                            &[],
                        )
                        .unwrap(),
                    );
                    groups.push((x, y, sigma));
                }
                let base = FitFunction::Builtin(Model::ExpDec1);
                let shared = vec![false; 3];
                let layout = global_layout(&base, count, &shared).unwrap();
                let mut initial = vec![0.; layout.names.len()];
                for indices in &layout.indices {
                    for (&i, value) in indices.iter().zip([0.2, 4., 1.5]) {
                        initial[i] = value;
                    }
                }
                let result = fit_indexed(GlobalSpec { base, groups, shared }, Some(&initial), &[], "").unwrap();
                for (actual, expected) in result.per_group.iter().zip(&direct) {
                    for (a, b) in actual.params.iter().zip(&expected.params) {
                        assert!(
                            (a - b).abs() / b.abs().max(1e-12) < 1e-10,
                            "weighted={weighted} count={count}: {a} != {b}"
                        );
                    }
                }
                let rss: f64 = direct.iter().map(|f| f.rss).sum();
                assert!(
                    (result.joint.rss - rss).abs() / rss < 1e-10,
                    "weighted={weighted} count={count}: {} != {rss}",
                    result.joint.rss
                );
            }
        }
    }
}
