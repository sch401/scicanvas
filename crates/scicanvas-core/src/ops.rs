//! 在项目上执行分析：确定输入数据、调用 analysis.rs 计算、把结果写成新列并更新图形。
//! 规则与 Origin 一致：结果列紧挨着原始列，拟合曲线加到图里显示为红色实线。

use crate::analysis::{self, Describe, NormalizeMethod, PolyFit, SmoothMethod};
use crate::interpolate::{self, InterpMethod};
use crate::model::{fmt_num, round_sig, Book, Column, Graph, PlotType, Project, Role};
use crate::page::{CurveStyle, Layer};
use crate::plotspec::{default_x, CurveMapping, DataRef, Plot};

mod baseline;
mod calculus;
mod columns;
mod fft;
mod fit;
mod recalc;
mod resolve;
mod smooth;
mod stats;

pub use calculus::AreaReport;
pub use columns::{ColumnOp, Operand};
pub use fit::{FitReport, NlFitReport};
pub use recalc::{OpKind, Operation, RecalcMode};
pub use resolve::{get_xy, AnalysisColumn, Item, Resolved, Source};

use columns::{analysis_plot, ResultLink};
use resolve::{name_of, x_for};

#[cfg(test)]
mod tests {
    use super::*;

    fn book_src(sel: &[&str]) -> Source {
        Source::Book {
            id: 1,
            selected: sel.iter().map(|s| s.to_string()).collect(),
            rows: None,
        }
    }

    #[test]
    fn linear_fit_adds_columns_and_graph_curve() {
        let mut p = Project::sample();
        let (gid, reps) = p.fit_poly(&Source::Graph(2), 1).unwrap();
        assert_eq!(gid, 2);
        assert_eq!(reps.len(), 2);
        let book = p.book(1).unwrap();
        // 原来 A B C，现在 B 后面跟着 B 的残差、拟合 X 和拟合值，C 后面同理
        let keys: Vec<&str> = book.columns.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, vec!["A", "B", "D", "E", "F", "C", "G", "H", "I"]);
        assert_eq!(book.column("E").unwrap().role, Role::X);
        assert_eq!(book.column("F").unwrap().data.len(), 200);
        // 残差逐行对齐：残差 = Y − 拟合值，残差之和为 0（含截距的最小二乘）
        let (xs, ys) = (book.column("A").unwrap().values(), book.column("B").unwrap().values());
        let res = book.column("D").unwrap().values();
        assert_eq!(res.len(), ys.len());
        for i in 0..ys.len() {
            assert!((res[i] - (ys[i] - reps[0].fit.eval(xs[i]))).abs() < 1e-6 * (1.0 + ys[i].abs()));
        }
        assert!(res.iter().sum::<f64>().abs() < 1e-6 * ys.iter().map(|y| y.abs()).sum::<f64>());
        assert_eq!(
            (reps[0].residual_key.as_str(), reps[0].x_key.as_deref()),
            ("D", Some("A"))
        );
        let g = p.graph(2).unwrap().first_layer();
        assert_eq!(g.draft().y_keys, vec!["B", "C", "F", "I"]);
        assert_eq!(g.draft().x_by_y.get("F").map(|s| s.as_str()), Some("E"));
        assert!(g.draft().line_only.contains(&"I".to_string()));
        // 拟合曲线不再参与下一次分析
        assert_eq!(p.resolve(&Source::Graph(2)).unwrap().items.len(), 2);
        // 图形曲线：结果曲线标记为只画线，并记录来源
        let (_, s) = g.series(p.book(1).unwrap());
        assert!(s[2].line_only && s[2].source.as_deref() == Some("B"));
        assert_eq!(s[2].points.len(), 200);
        // 置信带与预测带：四列，加到同一图层，预测带比置信带宽
        let bands = p.poly_bands(&reps[0], 0.95).unwrap();
        assert_eq!(bands.len(), 4);
        let book = p.book(1).unwrap();
        let v = |k: &str| book.column(k).unwrap().values();
        let (lo, hi, plo, phi, fitted) = (v(&bands[0]), v(&bands[1]), v(&bands[2]), v(&bands[3]), v("F"));
        for i in 0..fitted.len() {
            assert!(plo[i] < lo[i] && lo[i] < fitted[i] && fitted[i] < hi[i] && hi[i] < phi[i]);
        }
        let g = p.graph(2).unwrap().first_layer();
        assert!(bands
            .iter()
            .all(|k| g.draft().line_only.contains(k) && g.draft().x_by_y.get(k).map(|s| s.as_str()) == Some("E")));
        // 残差图：残差对原 X 的散点图
        let rid = p.residual_plot(&reps).unwrap();
        let rg = p.graph(rid).unwrap().first_layer();
        assert_eq!(rg.plot_type, PlotType::Scatter);
        assert_eq!(rg.draft().y_keys, vec!["D", "G"]);
        assert_eq!(rg.draft().x_by_y.get("G").map(|s| s.as_str()), Some("A"));
    }

    #[test]
    fn fit_from_book_creates_scatter_graph() {
        let mut p = Project::sample();
        p.graphs.clear();
        let (gid, reps) = p.fit_poly(&book_src(&["C"]), 3).unwrap();
        assert_eq!(reps[0].kind, "3 阶多项式拟合");
        let g = p.graph(gid).unwrap().first_layer();
        assert_eq!(g.plot_type, PlotType::Scatter);
        assert_eq!(g.x_key.as_deref(), Some("A"));
        assert!(reps[0].fit.r2 > 0.99);
    }

    #[test]
    fn fit_errors() {
        let mut p = Project::sample();
        assert!(p.fit_poly(&book_src(&["A"]), 1).is_err());
        let id = p.new_book();
        p.book_mut(id).unwrap().columns[1].data = vec!["1".into()].into();
        let src = Source::Book {
            id,
            selected: vec![],
            rows: None,
        };
        assert!(p.fit_poly(&src, 1).unwrap_err().contains("至少需要 3 个点"));
    }

    #[test]
    fn nonlinear_fit_adds_curve_columns_and_reports_each_series() {
        use crate::nlfit::Model;
        let mut p = Project::sample();
        // C 列是充电曲线 8(1 − e^{−t/4}) 加噪声，用 ExpDec1（A1 为负）拟合
        let (gid, reports) = p.nlfit(&book_src(&["C"]), Model::ExpDec1, &[], &[]).unwrap();
        assert_eq!(reports.len(), 1);
        let f = &reports[0].fit;
        assert!(f.converged && f.r2 > 0.99);
        assert!((f.params[0] - 8.0).abs() < 0.3 && (f.params[1] + 8.0).abs() < 0.4 && (f.params[2] - 4.0).abs() < 0.4);
        let book = p.book(1).unwrap();
        let y = book
            .columns
            .iter()
            .find(|c| c.long_name == "电压 B ExpDec1 拟合")
            .unwrap();
        assert_eq!(y.data.len(), 200);
        assert!(y.comment.contains("y = y0 + A1·exp(−x/t1)") && y.comment.contains("t1 = "));
        let layer = p.graph(gid).unwrap().first_layer();
        assert!(layer.draft().line_only.contains(&y.key) && layer.draft().x_by_y.contains_key(&y.key));
        assert!(p.nlfit(&book_src(&["C"]), Model::Gauss, &[], &[]).is_ok());
        let id = p.new_book();
        p.book_mut(id).unwrap().columns[1].data = vec!["1".into(), "2".into()].into();
        let src = Source::Book {
            id,
            selected: vec![],
            rows: None,
        };
        assert!(p.nlfit(&src, Model::Gauss, &[], &[]).unwrap_err().contains("至少需要"));
    }

    #[test]
    fn custom_nonlinear_fit_writes_a_named_curve_and_formula_report() {
        let mut project = Project::sample();
        let book = project.book_mut(1).unwrap();
        let xs: Vec<f64> = (0..40).map(|row| row as f64 * 0.25).collect();
        book.columns[0].data = xs.iter().map(|x| fmt_num(*x)).collect();
        book.columns[1].data = xs.iter().map(|x| fmt_num(2.2 * (-0.4 * x).exp() + 0.3)).collect();
        let function =
            crate::nlfit::CustomFitFunction::new("Decay", "a*exp(-b*x)+c", vec!["a".into(), "b".into(), "c".into()])
                .unwrap();
        let (graph, reports) = project
            .nlfit_custom(
                &book_src(&["B"]),
                function,
                &[2.0, 0.3, 0.2],
                &[crate::nlfit::ParamLimits::default(); 3],
                &[],
            )
            .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].fit.model.name(), "Decay");
        assert!(reports[0].fit.converged);
        assert!((reports[0].fit.params[0] - 2.2).abs() < 1e-5);
        let curve = project
            .book(1)
            .unwrap()
            .columns
            .iter()
            .find(|column| column.key == reports[0].curve.1)
            .unwrap();
        assert!(curve.comment.contains("y = a*exp(-b*x)+c") && curve.comment.contains("a = "));
        assert!(project
            .graph(graph)
            .unwrap()
            .first_layer()
            .draft()
            .line_only
            .contains(&curve.key));
    }

    #[test]
    fn fft_builds_a_spectrum_table_and_amplitude_graph_per_curve() {
        let mut p = Project::sample();
        // 10 Hz 采样（X 以 s 为单位、间隔 0.1），2 Hz、振幅 4 的正弦落在频点上
        let book = p.book_mut(1).unwrap();
        book.columns[0].data = (0..50).map(|i| fmt_num(i as f64 * 0.1)).collect();
        book.columns[1].data = (0..50)
            .map(|i| fmt_num(4.0 * (2.0 * std::f64::consts::PI * 2.0 * i as f64 * 0.1).sin()))
            .collect();
        book.columns[2].data = (0..50).map(|i| fmt_num(i as f64)).collect();
        let made = p.fft(&book_src(&["B"]), crate::fft::Window::Rect).unwrap();
        assert_eq!(made.len(), 1);
        let (bid, gid) = made[0];
        let out = p.book(bid).unwrap();
        let names: Vec<_> = out.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(names, ["频率", "实部", "虚部", "幅值", "相位", "功率"]);
        assert_eq!(
            (out.columns[0].units.as_str(), out.columns[3].units.as_str()),
            ("Hz", "mV")
        );
        assert_eq!(out.columns[5].units, "mV²");
        let f = out.columns[0].values();
        let amp = out.columns[3].values();
        assert_eq!(f.len(), 26);
        let peak = (0..amp.len()).max_by(|a, b| amp[*a].total_cmp(&amp[*b])).unwrap();
        assert!((f[peak] - 2.0).abs() < 1e-9 && (amp[peak] - 4.0).abs() < 1e-6);
        let layer = p.graph(gid).unwrap().first_layer();
        assert_eq!(layer.draft().y_keys, [out.columns[3].key.clone()]);
        assert_eq!(layer.x_key.as_deref(), Some(out.columns[0].key.as_str()));

        // 不等间距的 X 给出提示
        p.book_mut(1).unwrap().columns[0].data.set_text(3, "0.35".into());
        assert!(p
            .fft(&book_src(&["B"]), crate::fft::Window::Hann)
            .unwrap_err()
            .contains("不等间距"));
    }

    #[test]
    fn find_peaks_builds_a_peak_table_per_curve() {
        let mut p = Project::sample();
        // 两个高斯峰（X 递减，如波数），在 X = 30 和 X = 70
        let book = p.book_mut(1).unwrap();
        book.columns[0].units = "cm⁻¹".into();
        book.columns[0].data = (0..101).map(|i| fmt_num(100.0 - i as f64)).collect();
        let g = |x: f64| 2.0 * (-((x - 30.0) / 4.0).powi(2)).exp() + (-((x - 70.0) / 6.0).powi(2)).exp();
        book.columns[1].data = (0..101).map(|i| fmt_num(g(100.0 - i as f64))).collect();
        let made = p.find_peaks(&book_src(&["B"]), 0.05).unwrap();
        let out = p.book(made[0]).unwrap();
        let names: Vec<_> = out.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(
            names,
            [
                "峰位",
                "峰高",
                "突出度",
                "半峰宽",
                "面积",
                "质心",
                "积分起点",
                "积分终点"
            ]
        );
        assert_eq!(out.columns[0].values(), [30.0, 70.0]);
        assert_eq!(out.columns[4].units, "mV·cm⁻¹");
        // 高斯峰 A·exp(−((x−c)/s)²) 的半峰宽为 2s·√ln2、面积为 A·s·√π
        let fwhm = out.columns[3].values();
        assert!((fwhm[0] - 8.0 * 2f64.ln().sqrt()).abs() < 0.05, "{fwhm:?}");
        let area = out.columns[4].values();
        assert!((area[0] - 8.0 * std::f64::consts::PI.sqrt()).abs() < 0.05, "{area:?}");
        assert!((area[1] - 6.0 * std::f64::consts::PI.sqrt()).abs() < 0.05, "{area:?}");
        // 阈值过高时只留主峰；没有峰时给出提示
        let made = p.find_peaks(&book_src(&["B"]), 0.6).unwrap();
        assert_eq!(p.book(made[0]).unwrap().columns[0].values(), [30.0]);
        assert!(p.find_peaks(&book_src(&["B"]), 1.5).unwrap_err().contains("没有"));
        p.book_mut(1).unwrap().columns[0].data.set_text(5, "200".into());
        assert!(p.find_peaks(&book_src(&["B"]), 0.05).unwrap_err().contains("单调"));
    }

    #[test]
    fn baseline_subtraction_adds_baseline_and_corrected_columns() {
        use crate::baseline::BaselineMethod;
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        book.columns[0].data = (0..5).map(|i| fmt_num(i as f64)).collect();
        book.columns[1].data = ["1", "3", "", "6", "5"].iter().map(|s| s.to_string()).collect();
        let made = p.subtract_baseline(&book_src(&["B"]), BaselineMethod::Line).unwrap();
        let book = p.book(1).unwrap();
        let (b, c) = (book.column(&made[0].0).unwrap(), book.column(&made[0].1).unwrap());
        assert_eq!(book.column_index(&made[0].0), Some(2));
        assert_eq!(book.column_index(&made[0].1), Some(3));
        // 基线是 (0, 1) 到 (4, 5) 的连线；缺失行留空
        assert_eq!(b.data, ["1", "2", "", "4", "5"]);
        assert_eq!(c.data, ["0", "1", "", "2", "0"]);
        assert_eq!((c.long_name.as_str(), c.units.as_str()), ("电压 A 扣除基线", "mV"));

        // 来源是图形时，基线画进图里
        let mut p = Project::sample();
        let made = p
            .subtract_baseline(&Source::Graph(2), BaselineMethod::Als { lambda: 100.0, p: 0.05 })
            .unwrap();
        let layer = p.graph(2).unwrap().first_layer();
        assert!(made
            .iter()
            .all(|(b, _)| layer.draft().y_keys.contains(b) && layer.draft().line_only.contains(b)));
    }

    #[test]
    fn set_column_values_evaluates_row_by_row() {
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        book.columns[0].data = ["1", "2", "3", "4"].iter().map(|s| s.to_string()).collect();
        book.columns[1].data = ["10", "", "30", "-5"].iter().map(|s| s.to_string()).collect();
        // 引用自身：用计算前的值；缺失、负数开方处留空
        assert_eq!(p.set_column_values(1, "B", "B + sqrt(B) * A - i").unwrap(), 3);
        let b = &p.book(1).unwrap().columns[1].data;
        let want = 10.0 + 10f64.sqrt() - 1.0;
        assert_eq!(b.text_at(0), fmt_num(want));
        assert_eq!(b.text_at(1), "");
        assert_eq!(b.text_at(2), fmt_num(30.0 + 30f64.sqrt() * 3.0 - 3.0));
        assert_eq!(b.len(), 3, "trailing blanks trimmed");
        // 不引用列时按已有行数
        let rows = p.book(1).unwrap().nrows();
        assert_eq!(p.set_column_values(1, "C", "i^2").unwrap(), rows);
        assert_eq!(p.book(1).unwrap().columns[2].data.text_at(3), "16");
        assert!(p.set_column_values(1, "C", "Z * 2").unwrap_err().contains("没有列 Z"));
        assert!(p.set_column_values(1, "C", "2 +").is_err());
        assert!(p.set_column_values(1, "Q", "1").unwrap_err().contains("目标列"));
        // 列统计与按行序列：A = 1..4
        p.set_column_values(1, "C", "A - mean(A) + cumsum(A) * (A > 2)")
            .unwrap();
        let c = p.book(1).unwrap().columns[2].values();
        assert_eq!(&c[..4], &[-1.5, -0.5, 6.5, 11.5]);
    }

    #[test]
    fn analysis_result_columns_recalculate_by_mode() {
        let mut p = Project::sample();
        let keys = p.smooth(&book_src(&["B"]), SmoothMethod::SavitzkyGolay, 7).unwrap();
        let derived = p.differentiate(&book_src(&["B"])).unwrap();
        let (smooth, diff) = (keys[0].clone(), derived[0].clone());
        let col = |p: &Project, k: &str| p.book(1).unwrap().column(k).unwrap().values();
        assert_eq!(
            p.book(1).unwrap().column(&smooth).unwrap().op.as_ref().unwrap().mode,
            RecalcMode::Auto
        );
        // 改源数据：自动的平滑列与重新平滑一致
        p.book_mut(1).unwrap().columns[1].set(4, "50".into());
        assert!(p.recalc_computed(false).is_empty());
        let mut fresh = p.clone();
        let again = fresh.smooth(&book_src(&["B"]), SmoothMethod::SavitzkyGolay, 7).unwrap();
        assert_eq!(col(&p, &smooth), col(&fresh, &again[0]));
        // 手动：只在 manual 重算时更新；不重算：保持
        let set_mode = |p: &mut Project, k: &str, m: RecalcMode| {
            p.book_mut(1)
                .unwrap()
                .columns
                .iter_mut()
                .find(|c| c.key == k)
                .unwrap()
                .op
                .as_mut()
                .unwrap()
                .mode = m;
        };
        set_mode(&mut p, &diff, RecalcMode::Manual);
        set_mode(&mut p, &smooth, RecalcMode::None);
        let (old_diff, old_smooth) = (col(&p, &diff), col(&p, &smooth));
        p.book_mut(1).unwrap().columns[1].set(4, "-20".into());
        p.recalc_computed(false);
        assert_eq!(
            (col(&p, &diff), col(&p, &smooth)),
            (old_diff.clone(), old_smooth.clone())
        );
        p.recalc_computed(true);
        assert_ne!(col(&p, &diff), old_diff);
        assert_eq!(col(&p, &smooth), old_smooth);
        #[cfg(feature = "serde")]
        {
            let json = crate::project_file::to_json(&p).unwrap();
            let back = crate::project_file::from_json(&json).unwrap();
            assert_eq!(
                back.book(1).unwrap().column(&diff).unwrap().op,
                p.book(1).unwrap().column(&diff).unwrap().op
            );
        }
        // 多项式拟合：残差、拟合 X、拟合值随数据重算，与重新拟合一致
        let (_, reps) = p.fit_poly(&book_src(&["C"]), 2).unwrap();
        let (res, (fx, fy)) = (reps[0].residual_key.clone(), reps[0].curve.clone());
        p.book_mut(1).unwrap().columns[2].set(3, "9".into());
        assert!(p.recalc_computed(false).is_empty());
        let mut fresh = p.clone();
        let (_, again) = fresh.fit_poly(&book_src(&["C"]), 2).unwrap();
        assert_eq!(col(&p, &res), col(&fresh, &again[0].residual_key));
        assert_eq!(col(&p, &fx), col(&fresh, &again[0].curve.0));
        assert_eq!(col(&p, &fy), col(&fresh, &again[0].curve.1));
    }

    #[test]
    fn saved_formulas_recalculate_in_dependency_order() {
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        book.columns[0].data = ["1", "2", "3"].iter().map(|s| s.to_string()).collect();
        let d = book.push_column(Role::Y);
        let col = |p: &Project, k: &str| p.book(1).unwrap().column(k).unwrap().values();
        // D 依赖 C，C 依赖 A：先算 C 再算 D
        p.set_column_formula(1, "D", "C + 1", true).unwrap();
        p.set_column_formula(1, "C", "A * 2", true).unwrap();
        assert_eq!(col(&p, "C"), [2.0, 4.0, 6.0]);
        p.book_mut(1).unwrap().columns[0].set(1, "10".into());
        assert!(p.recalc_formulas().is_empty());
        assert_eq!(col(&p, "C"), [2.0, 20.0, 6.0]);
        assert_eq!(col(&p, &d), [3.0, 21.0, 7.0]);
        // 自引用与循环依赖拒绝
        assert!(p
            .set_column_formula(1, "B", "B + 1", true)
            .unwrap_err()
            .contains("自身"));
        assert!(p
            .set_column_formula(1, "C", "D * 2", true)
            .unwrap_err()
            .contains("循环"));
        // 去掉公式后不再重算
        p.set_column_formula(1, "C", "A", false).unwrap();
        assert_eq!(p.book(1).unwrap().column("C").unwrap().formula, None);
        p.book_mut(1).unwrap().columns[0].set(0, "5".into());
        p.recalc_formulas();
        assert_eq!(col(&p, "C")[0], 1.0);
        assert_eq!(col(&p, &d)[0], 2.0);
        #[cfg(feature = "serde")]
        {
            let json = crate::project_file::to_json(&p).unwrap();
            let back = crate::project_file::from_json(&json).unwrap();
            assert_eq!(
                back.book(1).unwrap().column(&d).unwrap().formula.as_deref(),
                Some("C + 1")
            );
        }
    }

    #[test]
    fn calibrate_writes_concentration_and_interval_beside_the_samples() {
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        let set = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        book.columns[0].data = set(&["0", "1", "2", "4", "6", "8", "10"]).into();
        book.columns[0].units = "mg/L".into();
        book.columns[1].data = set(&["0.02", "0.215", "0.41", "0.79", "1.22", "1.58", "2.01"]).into();
        book.columns[2].data = set(&["0.9", "", "1.5"]).into();
        let (cal, k0, k1) = p.calibrate(1, "A", "B", "C").unwrap();
        let book = p.book(1).unwrap();
        assert_eq!(book.column_index(&k0), Some(3));
        assert_eq!(book.column_index(&k1), Some(4));
        let (x0, ci) = (book.column(&k0).unwrap(), book.column(&k1).unwrap());
        assert_eq!(x0.units, "mg/L");
        assert_eq!(x0.data.text_at(1), "");
        // 与 numpy.polyfit 和 scipy.stats.t 计算一致
        let v = x0.values();
        assert!((v[0] - 4.468198640158359).abs() < 1e-6 && (v[2] - (1.5 - cal.intercept) / cal.slope).abs() < 1e-6);
        assert!((ci.values()[0] - 0.21355306685211103).abs() < 1e-6);
        assert!(x0.comment.contains("R²"));
        assert!(p.calibrate(1, "A", "B", "Z").is_err());
    }

    #[test]
    fn ifft_of_the_fft_table_recovers_the_signal() {
        // 32 点、dt = 0.01 s 的信号做矩形窗 FFT，再对频谱表的实部/虚部做 IFFT
        let mut p = Project::default();
        let id = p.new_book();
        let b = p.book_mut(id).unwrap();
        b.ensure_columns(2);
        b.columns[0].units = "s".into();
        let ys: Vec<f64> = (0..32)
            .map(|i| (2.0 * std::f64::consts::PI * 5.0 * i as f64 * 0.01).sin() + 0.3 + 0.01 * i as f64)
            .collect();
        b.columns[0].data = (0..32).map(|i| fmt_num(i as f64 * 0.01)).collect();
        b.columns[1].data = ys.iter().map(|y| format!("{y}")).collect();
        let src = Source::Book {
            id,
            selected: vec!["B".into()],
            rows: None,
        };
        let (spec_book, _) = p.fft(&src, crate::fft::Window::Rect).unwrap()[0];
        let sb = p.book(spec_book).unwrap();
        let key = |name: &str| sb.columns.iter().find(|c| c.long_name == name).unwrap().key.clone();
        let (re, im) = (key("实部"), key("虚部"));
        let (out, gid) = p
            .ifft(
                spec_book,
                &re,
                Some(&im),
                crate::fft::InverseInput::OneSided { odd: false },
            )
            .unwrap();
        let ob = p.book(out).unwrap();
        assert_eq!(ob.columns[0].units, "s");
        let t = ob.columns[0].values();
        let back = ob.columns[1].values();
        assert_eq!(back.len(), 32);
        assert!((t[1] - 0.01).abs() < 1e-12 && (t[31] - 0.31).abs() < 1e-12);
        for (b, y) in back.iter().zip(&ys) {
            // 频谱表按 fmt_num 存 8 位有效数字
            assert!((b - y).abs() < 1e-6, "{b} vs {y}");
        }
        assert_eq!(p.graph(gid).unwrap().first_layer().book_id, Some(out));
        // 奇数点数与双边谱
        let (odd, _) = p
            .ifft(
                spec_book,
                &re,
                Some(&im),
                crate::fft::InverseInput::OneSided { odd: true },
            )
            .unwrap();
        assert_eq!(p.book(odd).unwrap().columns[1].values().len(), 33);
        let (two, _) = p
            .ifft(spec_book, &re, None, crate::fft::InverseInput::TwoSided)
            .unwrap();
        assert_eq!(p.book(two).unwrap().columns.len(), 3);
        assert!(p
            .ifft(spec_book, "Z", None, crate::fft::InverseInput::TwoSided)
            .is_err());
    }

    #[test]
    fn fft_filter_adds_a_filtered_column_and_curve() {
        use crate::fft::FilterKind;
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        book.columns[0].data = (0..32).map(|i| fmt_num(i as f64 * 0.1)).collect();
        book.columns[1].data = (0..32)
            .map(|i| {
                fmt_num(
                    1.0 + (2.0 * std::f64::consts::PI * i as f64 / 32.0).sin()
                        + 0.3 * (i as f64 * std::f64::consts::PI).cos(),
                )
            })
            .collect();
        let keys = p.fft_filter(&book_src(&["B"]), FilterKind::LowPass(1.0)).unwrap();
        let c = p.book(1).unwrap().column(&keys[0]).unwrap();
        assert_eq!(c.long_name, "电压 A 低通");
        // 低通去掉了奈奎斯特频率处的 0.3·cos(πi)，留下直流与 1 周期正弦
        for (i, v) in c.values().iter().enumerate() {
            let want = 1.0 + (2.0 * std::f64::consts::PI * i as f64 / 32.0).sin();
            assert!((v - want).abs() < 1e-6, "{i}: {v} vs {want}");
        }
        let mut p = Project::sample();
        let keys = p.fft_filter(&Source::Graph(2), FilterKind::HighPass(0.1)).unwrap();
        assert!(keys
            .iter()
            .all(|k| p.graph(2).unwrap().first_layer().draft().line_only.contains(k)));
        p.book_mut(1).unwrap().columns[0].data.set_text(2, "9".into());
        assert!(p
            .fft_filter(&book_src(&["B"]), FilterKind::LowPass(1.0))
            .unwrap_err()
            .contains("等间距"));
    }

    #[test]
    fn row_stats_writes_mean_sd_se_and_n_after_the_last_column() {
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        let set = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        book.columns[0].data = set(&["1", "4", ""]).into();
        book.columns[1].data = set(&["2", "", ""]).into();
        book.columns[2].data = set(&["6", "5", ""]).into();
        let keys: Vec<String> = ["A", "B", "C"].iter().map(|s| s.to_string()).collect();
        let made = p.row_stats(1, &keys).unwrap();
        let book = p.book(1).unwrap();
        assert_eq!(book.column_index(&made[0]), Some(3));
        let col = |i: usize| book.column(&made[i]).unwrap();
        assert_eq!((col(0).role, col(1).role), (Role::Y, Role::YError));
        assert_eq!(col(0).data, ["3", "4.5"]);
        // 行 1：1、2、6 的样本标准差 √7，标准误 √(7/3)
        assert_eq!(col(1).data.text_at(0), fmt_num(7f64.sqrt()));
        assert_eq!(col(2).data.text_at(1), fmt_num(0.5));
        assert_eq!(col(3).data, ["3", "2", "0"]);
        assert!(p.row_stats(1, &keys[..1]).is_err());
    }

    #[test]
    fn correlation_function_builds_a_lag_table_and_graph() {
        let mut p = Project::sample();
        let keys: Vec<String> = vec!["B".into(), "C".into()];
        let (bid, gid) = p.correlation_function(1, &keys).unwrap();
        let out = p.book(bid).unwrap();
        let n = p.book(1).unwrap().column("B").unwrap().values().len() as f64;
        let lags = out.columns[0].values();
        assert_eq!((lags[0], *lags.last().unwrap()), (1.0 - n, n - 1.0));
        assert!(out.name.contains("互相关"));
        assert_eq!(
            p.graph(gid).unwrap().first_layer().draft().y_keys,
            [out.columns[1].key.clone()]
        );
        let (bid, _) = p.correlation_function(1, &keys[..1]).unwrap();
        let r = p.book(bid).unwrap().columns[1].values();
        assert_eq!(r[r.len() / 2], 1.0, "autocorrelation peaks at lag 0");
        assert!(p.correlation_function(1, &[]).is_err());
    }

    #[test]
    fn fill_sequence_overwrites_the_target_column() {
        use crate::stats::Sequence;
        let mut p = Project::sample();
        p.fill_sequence(1, "C", Sequence::Arithmetic { start: 1.0, step: 2.0 }, 5, 0)
            .unwrap();
        assert_eq!(p.book(1).unwrap().column("C").unwrap().data, ["1", "3", "5", "7", "9"]);
        assert!(p
            .fill_sequence(1, "C", Sequence::Arithmetic { start: 1.0, step: 2.0 }, 0, 0)
            .is_err());
        assert!(p
            .fill_sequence(1, "Q", Sequence::Uniform { min: 0.0, max: 1.0 }, 3, 0)
            .is_err());
    }

    #[test]
    fn average_curves_matches_numpy_interp_on_the_overlap() {
        let mut p = Project::sample();
        let book = p.book_mut(1).unwrap();
        let set = |v: &[f64]| v.iter().map(|x| fmt_num(*x)).collect::<Vec<_>>();
        // A、B 为第一条曲线；在 B 后插入第二条曲线的 X 与 Y
        book.columns[0].data = set(&[0.0, 1.0, 2.0, 3.0, 4.0]).into();
        book.columns[1].data = set(&[0.0, 2.0, 4.0, 6.0, 8.0]).into();
        book.columns[2].role = Role::X;
        book.columns[2].data = set(&[0.5, 1.5, 2.5, 3.5, 4.5]).into();
        let k = book.push_column(Role::Y);
        book.columns.iter_mut().find(|c| c.key == k).unwrap().data = set(&[1.0; 5]).into();
        let (bid, gid) = p.average_curves(&book_src(&["B", &k])).unwrap();
        let out = p.book(bid).unwrap();
        // numpy.interp 在 linspace(0.5, 4.0, 5) 上平均：((2x) + 1) / 2
        let grid = out.columns[0].values();
        assert_eq!(grid, [0.5, 1.375, 2.25, 3.125, 4.0]);
        for (x, m) in grid.iter().zip(out.columns[1].values()) {
            assert!((m - (2.0 * x + 1.0) / 2.0).abs() < 1e-12);
        }
        assert_eq!(out.columns[2].role, Role::YError);
        assert!(p.graph(gid).is_some());
        assert!(p.average_curves(&book_src(&["B"])).unwrap_err().contains("两条"));
    }

    #[test]
    fn column_math_works_row_by_row_with_constants_and_units() {
        let mut p = Project::sample();
        let key = p
            .column_math(1, "B", ColumnOp::Sub, &Operand::Column("C".into()))
            .unwrap();
        let book = p.book(1).unwrap();
        assert_eq!(book.column_index(&key), Some(2), "result sits right of A");
        let (b, c, r) = (
            book.column("B").unwrap(),
            book.column("C").unwrap(),
            book.column(&key).unwrap(),
        );
        assert_eq!(
            (r.long_name.as_str(), r.units.as_str(), r.role),
            ("电压 A − 电压 B", "mV", Role::Y)
        );
        for i in [0, 7, 40] {
            let want = b.values()[i] - c.values()[i];
            assert!((r.values()[i] - want).abs() < 1e-9);
        }

        let twice = p.column_math(1, "B", ColumnOp::Mul, &Operand::Constant(2.0)).unwrap();
        let book = p.book(1).unwrap();
        assert_eq!(
            book.column(&twice).unwrap().values()[0],
            2.0 * book.column("B").unwrap().values()[0]
        );
        assert_eq!(book.column(&twice).unwrap().units, "mV");
        let sq = p
            .column_math(1, "B", ColumnOp::Mul, &Operand::Column("C".into()))
            .unwrap();
        let ratio = p
            .column_math(1, "B", ColumnOp::Div, &Operand::Column("A".into()))
            .unwrap();
        let book = p.book(1).unwrap();
        assert_eq!(book.column(&sq).unwrap().units, "mV·mV");
        assert_eq!(book.column(&ratio).unwrap().units, "mV/s");
        // 第 0 行 A = 0，B ÷ A 不是有限数，留空
        assert_eq!(book.column(&ratio).unwrap().data.text_at(0), "");
        assert!(p
            .column_math(1, "B", ColumnOp::Div, &Operand::Constant(0.0))
            .unwrap_err()
            .contains("没有有效数值"));
        assert!(p
            .column_math(1, "B", ColumnOp::Add, &Operand::Constant(f64::NAN))
            .is_err());
    }

    #[test]
    fn subtracting_a_curve_on_another_x_grid_interpolates_it_first() {
        let mut p = Project::sample();
        let (_, reports) = p.fit_poly(&book_src(&["B"]), 1).unwrap();
        let fit = &reports[0].fit;
        let book = p.book(1).unwrap();
        let fit_y = book
            .columns
            .iter()
            .find(|c| c.long_name.ends_with("线性拟合"))
            .unwrap()
            .key
            .clone();
        let key = p.column_math(1, "B", ColumnOp::Sub, &Operand::Column(fit_y)).unwrap();
        let book = p.book(1).unwrap();
        let r = book.column(&key).unwrap();
        assert!(r.comment.contains("线性插值"));
        // 直线的线性插值是精确的，所以差值就是拟合残差
        let (x, b) = (book.column("A").unwrap().values(), book.column("B").unwrap().values());
        for i in 0..x.len() {
            let want = b[i] - fit.eval(x[i]);
            assert!((r.values()[i] - want).abs() < 1e-5, "row {i}");
        }

        // B 的 X 范围之外留空
        let id = p.new_book();
        let short = p.book_mut(id).unwrap();
        short.columns[0].data = ["0", "1", "2", "3"].map(String::from).to_vec().into();
        short.columns[1].data = ["1", "1", "1", "1"].map(String::from).to_vec().into();
        let x2 = short.push_column(Role::X);
        let y2 = short.push_column(Role::Y);
        short.columns.iter_mut().find(|c| c.key == x2).unwrap().data = ["1.5", "2.5"].map(String::from).to_vec().into();
        short.columns.iter_mut().find(|c| c.key == y2).unwrap().data = ["10", "20"].map(String::from).to_vec().into();
        let key = p.column_math(id, "B", ColumnOp::Add, &Operand::Column(y2)).unwrap();
        assert_eq!(p.book(id).unwrap().column(&key).unwrap().data, ["", "", "16"]);
    }

    #[test]
    fn frequency_counts_build_one_table_per_column_next_to_the_source() {
        let mut p = Project::sample();
        let folder = p.add_folder(None).unwrap();
        p.move_project_item(1, Some(folder)).unwrap();
        let ids = p.frequency_counts(&book_src(&["B", "C"]), Some(4), None).unwrap();
        assert_eq!(ids.len(), 2);
        let book = p.book(ids[0]).unwrap();
        assert_eq!(book.name, "电压 A 频数1");
        assert_eq!(p.folder_of(ids[0]), Some(folder), "results sit beside the source book");
        let names: Vec<_> = book.columns.iter().map(|c| c.long_name.as_str()).collect();
        assert_eq!(
            names,
            [
                "区间下限",
                "区间中点",
                "区间上限",
                "频数",
                "累积频数",
                "相对频率",
                "累积相对频率"
            ]
        );
        assert_eq!(book.columns[1].role, Role::X);
        assert_eq!(book.columns[0].units, "mV");
        let col = |i: usize| book.columns[i].values();
        assert_eq!(col(3).len(), 4);
        assert_eq!(col(3).iter().sum::<f64>(), 41.0, "every sample value is counted once");
        assert_eq!(col(4)[3], 41.0);
        assert_eq!(col(6)[3], 1.0);
        // 单元格按 8 位有效数字保存，相加只需接近 1
        assert!((col(5).iter().sum::<f64>() - 1.0).abs() < 1e-6);

        // 默认按 Sturges 规则；自定义范围外的值不计
        let auto = p.frequency_counts(&book_src(&["B"]), None, None).unwrap();
        assert_eq!(p.book(auto[0]).unwrap().nrows(), crate::stats::sturges_bins(41));
        let narrow = p
            .frequency_counts(&book_src(&["A"]), Some(2), Some((0.0, 4.75)))
            .unwrap();
        assert_eq!(p.book(narrow[0]).unwrap().columns[3].values(), [5.0, 5.0]);
        assert!(p
            .frequency_counts(&book_src(&["B"]), Some(3), Some((5.0, 1.0)))
            .is_err());
        assert_eq!(p.value_range(&book_src(&["A"])), Some((0.0, 20.0)));
    }

    #[test]
    fn interpolation_writes_xy_columns_and_adds_lines_to_the_graph() {
        let mut p = Project::sample();
        let (gid, keys) = p
            .interpolate(&Source::Graph(2), InterpMethod::Spline, 41, None)
            .unwrap();
        assert_eq!(gid, 2);
        assert_eq!(keys.len(), 2, "one result per data curve");
        let book = p.book(1).unwrap();
        let layer = p.graph(2).unwrap().first_layer();
        for key in &keys {
            assert!(layer.draft().line_only.contains(key) && layer.draft().y_keys.contains(key));
            let x = book.column(&layer.draft().x_by_y[key]).unwrap();
            let y = book.column(key).unwrap();
            assert_eq!((x.data.len(), y.data.len()), (41, 41));
            assert_eq!((x.data.text_at(0).as_ref(), x.data.text_at(40).as_ref()), ("0", "20"));
            assert!(y.long_name.ends_with("三次样条插值") && y.units == "mV");
        }
        // 网格落在原始 X 上（步长 0.5）时，插值结果就是原始数据
        let y = book.column(&keys[0]).unwrap();
        assert_eq!(
            y.data.iter().take(5).collect::<Vec<_>>(),
            book.column("B").unwrap().data.iter().take(5).collect::<Vec<_>>()
        );

        let (_, wide) = p
            .interpolate(&Source::Graph(2), InterpMethod::Linear, 3, Some((-10.0, 30.0)))
            .unwrap();
        let x = p
            .book(1)
            .unwrap()
            .column(&p.graph(2).unwrap().first_layer().draft().x_by_y[&wide[0]])
            .unwrap();
        assert_eq!(x.data, ["-10", "10", "30"], "custom ranges extrapolate");

        let id = p.new_book();
        p.book_mut(id).unwrap().columns[0].data = vec!["1".into(), "1".into()].into();
        p.book_mut(id).unwrap().columns[1].data = vec!["2".into(), "3".into()].into();
        let src = Source::Book {
            id,
            selected: vec![],
            rows: None,
        };
        assert!(p
            .interpolate(&src, InterpMethod::Akima, 10, None)
            .unwrap_err()
            .contains("重复"));
    }

    #[test]
    fn math_columns_align_with_rows() {
        let mut p = Project::sample();
        p.book_mut(1).unwrap().columns[1].data.set_text(5, "".into()); // 中间空一格
        let keys = p.differentiate(&book_src(&["B"])).unwrap();
        let col = p.book(1).unwrap().column(&keys[0]).unwrap().clone();
        assert_eq!(col.data.text_at(5), "");
        assert!(!col.data.text_at(4).is_empty());
        assert_eq!(col.long_name, "d(电压 A)/dX");
        assert_eq!(col.units, "mV/s");
        let (_, areas) = p.integrate(&book_src(&["C"])).unwrap();
        assert!(areas[0].area > 100.0 && (areas[0].area - areas[0].abs_area).abs() < 1e-9);
        let keys = p.normalize(&book_src(&["C"]), NormalizeMethod::MinMax).unwrap();
        let v = p.book(1).unwrap().column(&keys[0]).unwrap().values();
        assert_eq!(v.iter().copied().fold(f64::NEG_INFINITY, f64::max), 1.0);
    }

    #[test]
    fn fft_smoothing_needs_even_x_spacing() {
        let mut p = Project::sample();
        assert_eq!(
            p.smooth(&book_src(&["B"]), SmoothMethod::FftFilter, 5).unwrap().len(),
            1
        );
        assert_eq!(p.smooth(&book_src(&["B"]), SmoothMethod::Lowess, 7).unwrap().len(), 1);
        p.book_mut(1).unwrap().columns[0].set(3, "1.7".into());
        let err = p.smooth(&book_src(&["B"]), SmoothMethod::FftFilter, 5).unwrap_err();
        assert!(err.contains("不等间距"), "{err}");
    }

    #[test]
    fn smoothing_from_graph_adds_lines() {
        let mut p = Project::sample();
        let keys = p.smooth(&Source::Graph(2), SmoothMethod::SavitzkyGolay, 7).unwrap();
        assert_eq!(keys.len(), 2);
        let g = p.graph(2).unwrap().first_layer();
        assert!(keys
            .iter()
            .all(|k| g.draft().line_only.contains(k) && g.draft().y_keys.contains(k)));
    }

    #[test]
    fn stats_book() {
        let mut p = Project::sample();
        let id = p.stats_on_columns(&book_src(&[])).unwrap();
        let b = p.book(id).unwrap();
        assert_eq!(b.name, "列统计1");
        assert_eq!(b.columns.len(), 11);
        assert_eq!(b.nrows(), 2);
        assert_eq!(b.columns[1].data, vec!["41", "41"]);
        assert_eq!(b.columns[0].data.text_at(0), "B · 电压 A");
    }

    #[test]
    fn multi_key_and_selected_column_sorting() {
        let mut p = Project::default();
        let id = p.new_book();
        let b = p.book_mut(id).unwrap();
        b.ensure_columns(3);
        b.columns[0].data = ["b", "a", "b", "a", "c"].iter().map(|s| s.to_string()).collect();
        b.columns[1].data = ["2", "5", "1", "5", "0"].iter().map(|s| s.to_string()).collect();
        b.columns[2].data = ["r0", "r1", "r2", "r3", "r4"].iter().map(|s| s.to_string()).collect();
        let col = |p: &Project, i: usize| {
            (0..5)
                .map(|r| p.book(id).unwrap().columns[i].data.text_at(r).into_owned())
                .collect::<Vec<String>>()
        };
        // 先按 A 升序，再按 B 降序；A、B 都相同的两行保持原顺序（r1 在 r3 前）
        p.sort_rows(id, &[("A".into(), true), ("B".into(), false)], None)
            .unwrap();
        assert_eq!(col(&p, 0), ["a", "a", "b", "b", "c"]);
        assert_eq!(col(&p, 1), ["5", "5", "2", "1", "0"]);
        assert_eq!(col(&p, 2), ["r1", "r3", "r0", "r2", "r4"]);
        // 只排 B 列：A、C 不动
        p.sort_rows(id, &[("B".into(), true)], Some(&["B".into()])).unwrap();
        assert_eq!(col(&p, 1), ["0", "1", "2", "5", "5"]);
        assert_eq!(col(&p, 0), ["a", "a", "b", "b", "c"]);
        assert_eq!(col(&p, 2), ["r1", "r3", "r0", "r2", "r4"]);
        // 关键字不在要排序的列里、没有关键字时拒绝
        assert!(p.sort_rows(id, &[("A".into(), true)], Some(&["B".into()])).is_err());
        assert!(p.sort_rows(id, &[], None).is_err());
    }

    #[test]
    fn sorting_keeps_rows_together() {
        let mut p = Project::sample();
        let before: Vec<(String, String)> = (0..41)
            .map(|i| {
                (
                    p.book(1).unwrap().columns[0].data.text_at(i).into_owned(),
                    p.book(1).unwrap().columns[1].data.text_at(i).into_owned(),
                )
            })
            .collect();
        p.sort_book(1, 1, false).unwrap();
        let b = p.book(1).unwrap();
        let v = b.columns[1].values();
        assert!(v.windows(2).all(|w| w[0] >= w[1]));
        for i in 0..41 {
            let pair = (
                b.columns[0].data.text_at(i).into_owned(),
                b.columns[1].data.text_at(i).into_owned(),
            );
            assert!(before.contains(&pair));
        }
        // 文本和空单元格排在数值后面
        let id = p.new_book();
        p.book_mut(id).unwrap().columns[0].data = vec!["b".into(), "3".into(), "".into(), "1".into()].into();
        p.book_mut(id).unwrap().columns[1].data = vec!["x".into(), "y".into(), "z".into(), "w".into()].into();
        p.sort_book(id, 0, true).unwrap();
        assert_eq!(p.book(id).unwrap().columns[0].data, vec!["1", "3", "b"]);
        assert_eq!(p.book(id).unwrap().columns[1].data, vec!["w", "y", "x", "z"]);
    }
}
