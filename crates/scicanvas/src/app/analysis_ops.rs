use super::*;

impl SciApp {
    /// 结果写进工作簿后，确保相关窗口可见。
    pub(super) fn after_analysis(&mut self, graph: Option<u64>) {
        if let Some(g) = graph {
            if !self.open.get(&g).copied().unwrap_or(false) {
                self.show_item(g);
            }
        }
    }

    pub(crate) fn fit(&mut self, order: usize) {
        self.fit_order(order, if order == 1 { "线性拟合" } else { "多项式拟合" });
    }

    pub(super) fn fit_order(&mut self, order: usize, label: &str) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            label,
            |project| project.fit_poly(&src, order).map(Some),
            |app, (gid, reports)| {
                app.after_analysis(Some(gid));
                app.status = format!("已完成 {} 条数据的{}", reports.len(), reports[0].kind);
                app.dialog = Some(Dialog::FitReport(reports));
            },
        );
    }

    pub(crate) fn open_nlfit_dialog(&mut self) {
        self.dialog = Some(Dialog::NlFit {
            model: scicanvas_core::nlfit::Model::Gauss,
            initial: String::new(),
            target: None,
            limits: [(false, 0.0, false, 0.0, false, 1.0); scicanvas_core::nlfit::MAX_PARAMS],
            constraints: String::new(),
        });
    }

    pub(crate) fn open_custom_nlfit_dialog(&mut self) {
        let editor = self
            .project
            .fit_functions
            .first()
            .map(dialogs::CustomFitEditor::saved)
            .unwrap_or_default();
        self.dialog = Some(Dialog::CustomNlFit(editor));
    }

    pub(super) fn edit_custom_fit_definition(&mut self, definition: scicanvas_core::nlfit::CustomFitDefinition) {
        let mut editor = dialogs::CustomFitEditor::from_definition(&definition);
        editor.before = self
            .project
            .fit_functions
            .iter()
            .find(|entry| entry.definition.name == definition.name)
            .cloned();
        self.dialog = Some(Dialog::CustomNlFit(editor));
    }

    pub(super) fn save_custom_fit_definition(
        &mut self,
        before: Option<scicanvas_core::nlfit::SavedFitFunction>,
        definition: scicanvas_core::nlfit::CustomFitDefinition,
    ) -> bool {
        self.edit_with(
            "保存拟合函数",
            |project| project.save_fit_function(before.as_ref(), definition).map(Some),
            |app, id| {
                let saved = app.project.fit_functions.iter().find(|entry| entry.id == id).unwrap();
                app.status = format!("已保存函数：{}", saved.definition.name);
                app.dialog = Some(Dialog::CustomNlFit(dialogs::CustomFitEditor::saved(saved)));
            },
        )
        .is_some()
    }

    pub(super) fn import_custom_fit_function(&mut self, path: &std::path::Path) -> bool {
        let definition = match scicanvas_core::project_file::load_fit_function(path) {
            Ok(definition) => definition,
            Err(error) => {
                self.status = error;
                return false;
            }
        };
        if self
            .project
            .fit_functions
            .iter()
            .any(|entry| entry.definition.name == definition.name)
        {
            self.edit_custom_fit_definition(definition);
            self.status = "已载入同名函数，保存函数后更新当前项目".into();
            true
        } else {
            self.save_custom_fit_definition(None, definition)
        }
    }

    pub(super) fn apply_custom_fit_library_action(&mut self, action: dialogs::CustomFitAction) -> bool {
        use dialogs::CustomFitAction;
        match action {
            CustomFitAction::Save { before, definition } => match definition {
                Ok(definition) => self.save_custom_fit_definition(before, definition),
                Err(error) => {
                    self.status = error;
                    false
                }
            },
            CustomFitAction::Edit(definition) => {
                self.edit_custom_fit_definition(definition);
                true
            }
            CustomFitAction::LoadFile => {
                let Some(path) = self
                    .file_dialog()
                    .set_title("载入拟合函数")
                    .add_filter("SciCanvas 拟合函数", &["scfn"])
                    .pick_file()
                else {
                    return true;
                };
                self.import_custom_fit_function(&path)
            }
            CustomFitAction::Export(definition) => {
                let definition = match definition {
                    Ok(definition) => definition,
                    Err(error) => {
                        self.status = error;
                        return false;
                    }
                };
                let Some(path) = self
                    .file_dialog()
                    .set_title("导出拟合函数")
                    .add_filter("SciCanvas 拟合函数", &["scfn"])
                    .set_file_name(format!("{}.scfn", definition.name))
                    .save_file()
                else {
                    return true;
                };
                match scicanvas_core::project_file::save_fit_function(&path, &definition) {
                    Ok(()) => {
                        self.status = format!("已导出函数：{}", path.display());
                        true
                    }
                    Err(error) => {
                        self.status = error;
                        false
                    }
                }
            }
        }
    }

    pub(super) fn nlfit(
        &mut self,
        model: scicanvas_core::nlfit::Model,
        limits: &[scicanvas_core::nlfit::ParamLimits],
        constraints: &[scicanvas_core::nlfit::LinearConstraint],
    ) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "非线性拟合",
            |project| project.nlfit(&src, model, limits, constraints).map(Some),
            |app, (gid, reports)| {
                app.after_analysis(Some(gid));
                let unconverged = reports.iter().filter(|r| !r.fit.converged).count();
                app.status = if unconverged == 0 {
                    format!("已完成 {} 条数据的 {} 拟合", reports.len(), model.name())
                } else {
                    format!("{} 拟合有 {unconverged} 条未收敛，请查看结果", model.name())
                };
                app.dialog = Some(Dialog::NlFitReport(reports));
            },
        );
    }

    pub(super) fn custom_nlfit(
        &mut self,
        function: scicanvas_core::nlfit::CustomFitFunction,
        initial: Vec<f64>,
        limits: Vec<scicanvas_core::nlfit::ParamLimits>,
        constraints: Vec<scicanvas_core::nlfit::LinearConstraint>,
    ) -> bool {
        let Some(src) = self.analysis_source() else {
            self.status = "请选择要拟合的数据曲线".into();
            return false;
        };
        let function_name = function.name().to_owned();
        self.edit_with(
            "自定义非线性拟合",
            |project| {
                project
                    .nlfit_custom(&src, function, &initial, &limits, &constraints)
                    .map(Some)
            },
            move |app, (gid, reports)| {
                app.after_analysis(Some(gid));
                let unconverged = reports.iter().filter(|report| !report.fit.converged).count();
                app.status = if unconverged == 0 {
                    format!("已完成 {} 条数据的 {} 拟合", reports.len(), function_name)
                } else {
                    format!("{} 拟合有 {unconverged} 条未收敛，请查看结果", function_name)
                };
                app.dialog = Some(Dialog::NlFitReport(reports));
            },
        )
        .is_some()
    }

    pub(crate) fn open_poly_dialog(&mut self) {
        self.dialog = Some(Dialog::Poly { order: 2 });
    }

    /// IFFT：选中的前两列作实部、虚部；没选列时找 FFT 结果表里的“实部”“虚部”。
    pub(crate) fn open_ifft_dialog(&mut self) {
        let Some(id) = self.active_book() else { return };
        let Some(book) = self.project.book(id) else { return };
        let named = |name: &str| book.columns.iter().find(|c| c.long_name == name).map(|c| c.key.clone());
        let sel = self.selected_keys(id);
        let (re, im) = if sel.is_empty() {
            (named("实部"), named("虚部"))
        } else {
            (sel.first().cloned(), sel.get(1).cloned())
        };
        let Some(re) = re else {
            self.status = "请选中实部列（和虚部列），或在 FFT 结果表里运行".into();
            return;
        };
        let label = |k: &String| {
            (
                k.clone(),
                book.column(k)
                    .map_or(k.clone(), |c| format!("{}（{}）", c.key, c.display_name())),
            )
        };
        let rows = [Some(&re), im.as_ref()]
            .into_iter()
            .flatten()
            .filter_map(|k| book.column(k))
            .map(|c| c.values().iter().rposition(|v| v.is_finite()).map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0);
        self.dialog = Some(Dialog::Ifft {
            book: id,
            re: label(&re),
            im: im.as_ref().map(label),
            rows,
            two_sided: false,
            odd: false,
        });
    }

    pub(crate) fn open_sg_diff_dialog(&mut self) {
        self.dialog = Some(Dialog::SgDiff {
            window: 7,
            order: 2,
            deriv: 1,
        });
    }

    pub(crate) fn open_smooth_dialog(&mut self) {
        self.dialog = Some(Dialog::Smooth {
            method: SmoothMethod::SavitzkyGolay,
            window: 7,
        });
    }

    pub(crate) fn open_baseline_dialog(&mut self) {
        self.dialog = Some(Dialog::Baseline {
            als: true,
            log_lambda: 5.0,
            p: 0.01,
        });
    }

    pub(super) fn subtract_baseline(&mut self, method: scicanvas_core::baseline::BaselineMethod) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "扣除基线",
            |project| project.subtract_baseline(&src, method).map(Some),
            |app, made| {
                let keys: Vec<&str> = made.iter().map(|(_, k)| k.as_str()).collect();
                app.status = format!("已扣除基线（{}），结果在新列 {}", method.label(), keys.join(", "));
            },
        );
    }

    pub(crate) fn open_peaks_dialog(&mut self) {
        self.dialog = Some(Dialog::Peaks { percent: 5.0 });
    }

    pub(super) fn find_peaks(&mut self, min_prominence: f64) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "寻峰",
            |project| project.find_peaks(&src, min_prominence).map(Some),
            |app, made| {
                let mut parts = vec![];
                for id in &made {
                    if let Some(book) = app.project.book(*id) {
                        parts.push(format!("{}（{} 个峰）", book.name, book.columns[0].data.len()));
                    }
                }
                if let Some(id) = made.last() {
                    app.show_item(*id);
                }
                app.status = format!("峰表在 {}", parts.join("、"));
            },
        );
    }

    pub(crate) fn open_fft_dialog(&mut self) {
        self.dialog = Some(Dialog::Fft {
            window: scicanvas_core::fft::Window::Hann,
        });
    }

    pub(super) fn fft(&mut self, window: scicanvas_core::fft::Window) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "FFT",
            |project| project.fft(&src, window).map(Some),
            |app, made| {
                let names: Vec<String> = made
                    .iter()
                    .filter_map(|(b, _)| app.project.name_of(*b).map(str::to_owned))
                    .collect();
                if let Some((_, g)) = made.last() {
                    app.show_item(*g);
                }
                app.status = format!("FFT 结果在 {}，幅值谱已画成图", names.join("、"));
            },
        );
    }

    pub(crate) fn open_column_math_dialog(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(book) = self.project.book(id) else { return };
        let name = |k: Option<&String>| k.and_then(|k| book.column(k)).map(|c| c.display_name().to_owned());
        let Some(a) = name(keys.first()) else {
            self.status = "请先单击列标题选中一列或两列".into();
            return;
        };
        let b = name(keys.get(1));
        self.dialog = Some(Dialog::ColumnMath {
            a,
            use_constant: b.is_none(),
            b,
            op: scicanvas_core::ops::ColumnOp::Sub,
            constant: 1.0,
        });
    }

    pub(super) fn column_math(&mut self, op: scicanvas_core::ops::ColumnOp, constant: Option<f64>) {
        use scicanvas_core::ops::Operand;
        let Some((id, keys)) = self.ttest_columns() else { return };
        let Some(a) = keys.first().cloned() else { return };
        let operand = match (constant, keys.get(1)) {
            (Some(c), _) => Operand::Constant(c),
            (None, Some(b)) => Operand::Column(b.clone()),
            (None, None) => {
                self.status = "请再选中第二列，或改用常数".into();
                return;
            }
        };
        self.edit_with(
            "列运算",
            |project| project.column_math(id, &a, op, &operand).map(Some),
            |app, key| app.status = format!("列运算结果在新列 {key}"),
        );
    }

    pub(crate) fn open_frequency_dialog(&mut self) {
        // 默认范围：要统计的各列里全部数值的最小、最大值
        let (from, to) = self
            .analysis_source()
            .and_then(|src| self.project.value_range(&src))
            .unwrap_or((0.0, 1.0));
        self.dialog = Some(Dialog::Frequency {
            auto: true,
            bins: 10,
            custom: false,
            from,
            to,
        });
    }

    pub(super) fn frequency_counts(&mut self, bins: Option<usize>, range: Option<(f64, f64)>) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "频数统计",
            |project| project.frequency_counts(&src, bins, range).map(Some),
            |app, ids| {
                let names: Vec<String> = ids
                    .iter()
                    .filter_map(|id| app.project.name_of(*id).map(str::to_owned))
                    .collect();
                if let Some(last) = ids.last() {
                    app.show_item(*last);
                }
                app.status = format!("频数统计结果在 {}", names.join("、"));
            },
        );
    }

    /// 当前工作簿里按点选顺序参与检验的列（没有点选列时用单元格选区覆盖的列）。
    pub(crate) fn open_analysis_curves(&mut self) {
        let Some(graph) = self.active_graph_id() else { return };
        let layer = self.layer_index(graph);
        match self.project.resolve(&Source::Layer {
            graph_id: graph,
            index: layer,
        }) {
            Ok(resolved) => {
                let selected = self.analysis_curves.get(&(graph, layer));
                let curves = resolved
                    .items
                    .iter()
                    .map(|item| {
                        let reference = scicanvas_core::plotspec::DataRef::new(item.book_id, &item.y);
                        let book = self.project.book(item.book_id).unwrap();
                        let name = format!("{}：{}", book.name, book.column(&item.y).unwrap().display_name());
                        let checked = selected.is_none_or(|s| s.contains(&reference));
                        (reference, name, checked)
                    })
                    .collect();
                self.dialog = Some(Dialog::AnalysisCurves { graph, layer, curves });
            }
            Err(error) => self.status = error,
        }
    }

    pub(super) fn statistical_columns(&self) -> Option<Vec<scicanvas_core::ops::AnalysisColumn>> {
        if self.active_book_id().is_some_and(|id| self.picked(id).0.is_empty()) {
            return Some(Vec::new());
        }
        self.project.analysis_columns(&self.analysis_source()?).ok()
    }

    pub(super) fn ttest_columns(&self) -> Option<(u64, Vec<String>)> {
        let id = self.active_book_id()?;
        let book = self.project.book(id)?;
        let keys: Vec<String> = self
            .picked(id)
            .0
            .into_iter()
            .filter(|k| book.column(k).is_some())
            .collect();
        Some((id, keys))
    }

    pub(crate) fn open_fft_filter_dialog(&mut self) {
        // 用第一条曲线的采样间隔估计奈奎斯特频率，给出默认截止频率
        let dt = self
            .analysis_source()
            .and_then(|src| self.project.resolve(&src).ok())
            .and_then(|r| {
                let item = r.items.first()?;
                let book = self.project.book(item.book_id)?;
                let (xs, _, _) = scicanvas_core::ops::get_xy(book, item);
                match item.x {
                    Some(_) => scicanvas_core::fft::sample_interval(&xs).ok(),
                    None => Some(1.0),
                }
            })
            .unwrap_or(1.0);
        let nyquist = 0.5 / dt;
        self.dialog = Some(Dialog::FftFilter {
            kind: 0,
            lo: round_sig(nyquist / 10.0, 3),
            hi: round_sig(nyquist / 4.0, 3),
            nyquist,
        });
    }

    pub(super) fn fft_filter(&mut self, kind: scicanvas_core::fft::FilterKind) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "FFT 滤波",
            |project| project.fft_filter(&src, kind).map(Some),
            |app, keys| app.status = format!("已 FFT 滤波（{}），结果在新列 {}", kind.label(), keys.join(", ")),
        );
    }

    /// 自相关（选一列）或互相关（选两列）。
    pub(crate) fn correlation_function(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        match self.project.correlation_function(id, &keys) {
            Ok((bid, gid)) => {
                let name = self.project.name_of(bid).unwrap_or_default().to_owned();
                self.show_item(gid);
                self.status = format!("相关函数结果在 {name}，已画成图");
            }
            Err(e) => self.status = e,
        }
    }

    /// 当前图形的各条曲线（或选中的 Y 列）插值到公共 X 上求平均。
    pub(crate) fn average_curves(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.average_curves(&src) {
            Ok((bid, gid)) => {
                let name = self.project.name_of(bid).unwrap_or_default().to_owned();
                self.show_item(gid);
                self.status = format!("平均曲线在 {name}（标准差为 Y 误差列）");
            }
            Err(e) => self.status = e,
        }
    }

    /// 对点选的各列逐行统计（平行样的均值、标准差等）。
    pub(crate) fn row_stats(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        match self.project.row_stats(id, &keys) {
            Ok(made) => self.status = format!("行统计结果在新列 {}（标准差已设为 Y 误差列）", made.join(", ")),
            Err(e) => self.status = format!("{e}：请 Ctrl+单击选中平行样的各列"),
        }
    }

    /// 多元线性回归：最后点选的列为因变量，其余为自变量。
    pub(crate) fn regression(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.len() < 2 {
            self.status = "请先点选自变量列，最后点选因变量（Y）列".into();
            return;
        }
        let book = self.project.book(id).unwrap();
        let (ykey, xkeys) = keys.split_last().unwrap();
        let xs: Vec<Vec<f64>> = xkeys.iter().map(|k| book.column(k).unwrap().values()).collect();
        let y = book.column(ykey).unwrap();
        match scicanvas_core::stats::multiple_regression(&xs, &y.values()) {
            Ok(r) => {
                let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
                let names = std::iter::once("截距".to_owned())
                    .chain(xkeys.iter().map(|k| book.column(k).unwrap().display_name().to_owned()));
                for (i, name) in names.enumerate() {
                    rows.push(vec![
                        name,
                        fmt_num(round_sig(r.coef[i], 8)),
                        fmt_num(round_sig(r.se[i], 6)),
                        fmt_num(round_sig(r.t[i], 6)),
                        fmt_num(round_sig(r.p[i], 6)),
                    ]);
                }
                let note = format!(
                    "因变量 {}，N = {}。R² = {}，调整 R² = {}，残差标准差 = {}；\nF = {}，p = {}（检验所有斜率是否同时为 0）。参数的 p 为双侧 t 检验；任一变量缺失的行已跳过。",
                    y.display_name(),
                    r.n,
                    fmt_num(round_sig(r.r2, 6)),
                    fmt_num(round_sig(r.adj_r2, 6)),
                    fmt_num(round_sig(r.s, 6)),
                    fmt_num(round_sig(r.f, 6)),
                    fmt_num(round_sig(r.f_p, 6))
                );
                self.status = format!("多元线性回归：R² = {}", fmt_num(round_sig(r.r2, 6)));
                self.dialog = Some(Dialog::Table {
                    title: "多元线性回归".into(),
                    rows,
                    note,
                });
            }
            Err(e) => self.status = e,
        }
    }

    /// 点选的各列分别做 Shapiro–Wilk 正态性检验。
    pub(crate) fn normality(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.is_empty() {
            self.status = "请先在工作表里单击选中要检验的列".into();
            return;
        }
        let mut rows = vec![["列", "N", "W", "p", "α = 0.05"].map(String::from).to_vec()];
        let mut problems = vec![];
        for c in &columns {
            match scicanvas_core::stats::shapiro_wilk(&c.values.clone()) {
                Ok(r) => rows.push(vec![
                    c.name.as_str().to_owned(),
                    r.n.to_string(),
                    fmt_num(round_sig(r.w, 6)),
                    fmt_num(round_sig(r.p, 6)),
                    if r.p < 0.05 {
                        "偏离正态"
                    } else {
                        "不能拒绝正态"
                    }
                    .into(),
                ]),
                Err(e) => problems.push(format!("{}：{e}", c.name.as_str())),
            }
        }
        if rows.len() == 1 {
            self.status = problems.join("；");
            return;
        }
        self.status = if problems.is_empty() {
            format!("已对 {} 列做正态性检验", rows.len() - 1)
        } else {
            problems.join("；")
        };
        self.dialog = Some(Dialog::Table {
            title: "正态性检验（Shapiro–Wilk）".into(),
            rows,
            note: "原假设：数据来自正态分布。p < 0.05 时认为偏离正态；缺失值已跳过，3 ≤ N ≤ 5000。".into(),
        });
    }

    /// 点选的各列分别做 Grubbs 异常值检验（双侧，α = 0.05）。
    pub(crate) fn grubbs(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.is_empty() {
            self.status = "请先在工作表里单击选中要检验的列".into();
            return;
        }
        let mut rows = vec![["列", "N", "可疑值", "所在行", "G", "临界值 G(0.05)", "p", "结论"]
            .map(String::from)
            .to_vec()];
        let mut problems = vec![];
        for c in &columns {
            match scicanvas_core::stats::grubbs(&c.values.clone(), 0.05) {
                Ok(r) => rows.push(vec![
                    c.name.as_str().to_owned(),
                    r.n.to_string(),
                    fmt_num(r.value),
                    (r.index + 1).to_string(),
                    fmt_num(round_sig(r.g, 6)),
                    fmt_num(round_sig(r.critical, 6)),
                    fmt_num(round_sig(r.p, 6)),
                    if r.g > r.critical {
                        "是异常值"
                    } else {
                        "不是异常值"
                    }
                    .into(),
                ]),
                Err(e) => problems.push(format!("{}：{e}", c.name.as_str())),
            }
        }
        if rows.len() == 1 {
            self.status = problems.join("；");
            return;
        }
        self.status = if problems.is_empty() {
            format!("已对 {} 列做 Grubbs 检验", rows.len() - 1)
        } else {
            problems.join("；")
        };
        self.dialog = Some(Dialog::Table {
            title: "Grubbs 异常值检验".into(),
            rows,
            note: "检验离均值最远的一个点（双侧，α = 0.05），假设其余数据近似正态。\n剔除异常值后如需继续检验，请再运行一次。".into(),
        });
    }

    /// 标准曲线反查：按点选顺序取标准浓度、标准响应、样品响应三列。
    pub(crate) fn calibrate(&mut self) {
        let Some((id, keys)) = self.ttest_columns() else { return };
        if keys.len() != 3 {
            self.status = "请按顺序 Ctrl+单击选中三列：标准浓度（X）、标准响应（Y）、样品响应".into();
            return;
        }
        match self.project.calibrate(id, &keys[0], &keys[1], &keys[2]) {
            Ok((cal, k0, k1)) => {
                self.status = format!(
                    "标准曲线 y = {} + {}·x，R² = {}；反查结果在新列 {k0}、{k1}",
                    fmt_num(round_sig(cal.intercept, 6)),
                    fmt_num(round_sig(cal.slope, 6)),
                    fmt_num(round_sig(cal.r2, 6))
                );
            }
            Err(e) => self.status = e,
        }
    }

    /// 每个点选的列作为一组做单因素方差分析。
    pub(crate) fn anova(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.len() < 2 {
            self.status = "请先在工作表里单击选中至少两列（每列一组）".into();
            return;
        }
        let groups: Vec<Vec<f64>> = columns.iter().map(|c| c.values.clone()).collect();
        let names: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
        match scicanvas_core::stats::one_way_anova(&groups) {
            Ok(result) => {
                self.status = format!(
                    "单因素方差分析：F = {}，p = {}",
                    round_sig(result.f, 4),
                    round_sig(result.p, 4)
                );
                let tukey = scicanvas_core::stats::tukey_hsd(&result, 0.95);
                self.dialog = Some(Dialog::Anova { result, names, tukey });
            }
            Err(e) => self.status = e,
        }
    }

    /// 点选的列两两计算 Pearson、Spearman、Kendall 相关系数。
    pub(crate) fn correlations(&mut self) {
        use scicanvas_core::stats::{correlation, CorrMethod};
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        if columns.len() < 2 {
            self.status = "请先在工作表里单击选中至少两列（Ctrl+单击多选）".into();
            return;
        }
        let mut pairs = vec![];
        let mut problems = vec![];
        for i in 0..columns.len() {
            for j in i + 1..columns.len() {
                let (a, b) = (&columns[i], &columns[j]);
                let found: Result<Vec<_>, String> = CorrMethod::ALL
                    .iter()
                    .map(|m| correlation(&a.values.clone(), &b.values.clone(), *m))
                    .collect();
                match found {
                    Ok(cs) => pairs.push((a.name.as_str().to_owned(), b.name.as_str().to_owned(), cs)),
                    Err(e) => problems.push(format!("{} 与 {}：{e}", a.name.as_str(), b.name.as_str())),
                }
            }
        }
        if pairs.is_empty() {
            self.status = problems.join("；");
            return;
        }
        self.status = if problems.is_empty() {
            format!("已计算 {} 对列的相关系数", pairs.len())
        } else {
            format!("已计算 {} 对列；{}", pairs.len(), problems.join("；"))
        };
        self.dialog = Some(Dialog::Correlations(pairs));
    }

    pub(crate) fn open_ttest_dialog(&mut self) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        let count = columns.len();
        let columns = columns.iter().take(2).map(|c| c.name.clone()).collect();
        self.dialog = Some(Dialog::TTest {
            columns,
            kind: if count >= 2 { 1 } else { 0 },
            mu: 0.0,
            equal_var: false,
            level: 0.95,
        });
    }

    pub(super) fn run_ttest(&mut self, kind: scicanvas_core::stats::TTestKind, level: f64) {
        let Some(columns) = self.statistical_columns() else {
            return;
        };
        let names = columns.iter().take(2).map(|c| c.name.clone()).collect();
        let empty = Vec::new();
        let a = columns.first().map(|c| &c.values).unwrap_or(&empty);
        let b = columns.get(1).map(|c| &c.values).unwrap_or(&empty);
        match scicanvas_core::stats::t_test(kind, a, b, level) {
            Ok(result) => {
                self.status = format!(
                    "{}：t = {}，p = {}",
                    kind.label(),
                    round_sig(result.t, 4),
                    round_sig(result.p, 4)
                );
                self.dialog = Some(Dialog::TTestReport { result, names });
            }
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn open_interpolate_dialog(&mut self) {
        let (from, to) = self
            .analysis_source()
            .and_then(|src| self.project.x_range(&src))
            .unwrap_or((0.0, 1.0));
        self.dialog = Some(Dialog::Interpolate {
            method: scicanvas_core::interpolate::InterpMethod::Spline,
            points: 100,
            custom: false,
            from,
            to,
        });
    }

    pub(super) fn interpolate(
        &mut self,
        method: scicanvas_core::interpolate::InterpMethod,
        points: usize,
        range: Option<(f64, f64)>,
    ) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "插值",
            |project| project.interpolate(&src, method, points, range).map(Some),
            |app, (gid, keys)| {
                app.show_item(gid);
                app.status = format!("已{}插值，结果在新列 {}", method.label(), keys.join(", "));
            },
        );
    }

    pub(crate) fn open_normalize_dialog(&mut self) {
        self.dialog = Some(Dialog::Normalize {
            method: NormalizeMethod::MinMax,
        });
    }

    pub(super) fn smooth(&mut self, method: SmoothMethod, window: usize) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "平滑",
            |project| project.smooth(&src, method, window).map(Some),
            |app, keys| app.status = format!("已平滑，结果在新列 {}", keys.join(", ")),
        );
    }

    pub(crate) fn differentiate(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.differentiate(&src) {
            Ok(keys) => self.status = format!("已求导数，结果在新列 {}", keys.join(", ")),
            Err(e) => self.status = e,
        }
    }

    pub(crate) fn integrate(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.integrate(&src) {
            Ok((keys, areas)) => {
                self.status = format!("已积分，累积积分在新列 {}", keys.join(", "));
                self.dialog = Some(Dialog::Areas(areas));
            }
            Err(e) => self.status = e,
        }
    }

    pub(super) fn normalize(&mut self, method: NormalizeMethod) {
        let Some(src) = self.analysis_source() else { return };
        self.edit_with(
            "归一化",
            |project| project.normalize(&src, method).map(Some),
            |app, keys| app.status = format!("已归一化，结果在新列 {}", keys.join(", ")),
        );
    }

    pub(crate) fn stats_on_columns(&mut self) {
        let Some(src) = self.analysis_source() else { return };
        match self.project.stats_on_columns(&src) {
            Ok(id) => {
                let name = self.project.name_of(id).unwrap_or("").to_string();
                self.show_item(id);
                self.status = format!("统计结果在 {}（标准差为样本标准差，分位数用线性插值）", name);
            }
            Err(e) => self.status = e,
        }
    }
}

impl SciApp {
    pub(crate) fn open_nlfit_reports(&mut self) {
        let reports = self.project.nlfit_reports();
        if reports.is_empty() {
            self.status = "项目中尚无保存的非线性拟合报告".into();
        } else {
            self.dialog = Some(Dialog::NlFitReport(reports));
        }
    }

    pub(super) fn edit_nlfit_report(&mut self, report: scicanvas_core::ops::NlFitReport) {
        use scicanvas_core::ops::OpKind;
        let op = match self.project.nlfit_operation(&report) {
            Ok(op) => op.clone(),
            Err(e) => {
                self.status = e;
                return;
            }
        };
        let OpKind::Nonlinear(recipe) = &op.kind else {
            unreachable!()
        };
        let target = Some(Box::new(dialogs::FitEditTarget {
            report: report.clone(),
            before: op.clone(),
        }));
        match &recipe.function {
            scicanvas_core::ops::FitMethod::Builtin(model) => {
                let mut limits = [(false, 0.0, false, 0.0, false, 1.0); scicanvas_core::nlfit::MAX_PARAMS];
                for (ui, limit) in limits.iter_mut().zip(&recipe.limits) {
                    *ui = (
                        limit.fixed.is_some(),
                        limit.fixed.unwrap_or(0.0),
                        limit.lower.is_some(),
                        limit.lower.unwrap_or(0.0),
                        limit.upper.is_some(),
                        limit.upper.unwrap_or(1.0),
                    );
                }
                self.dialog = Some(Dialog::NlFit {
                    model: *model,
                    initial: recipe
                        .initial
                        .as_ref()
                        .map(|v| v.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))
                        .unwrap_or_default(),
                    limits,
                    constraints: recipe.constraints.clone(),
                    target,
                });
            }
            scicanvas_core::ops::FitMethod::Custom {
                name,
                formula,
                parameters,
            } => {
                let definition = scicanvas_core::nlfit::CustomFitDefinition {
                    name: name.clone(),
                    formula: formula.clone(),
                    parameters: parameters.clone(),
                    initial: recipe.initial.clone().unwrap_or_default(),
                    constraints: recipe.constraints.clone(),
                };
                let mut edit = dialogs::CustomFitEditor::from_definition(&definition);
                edit.target = target;
                edit.limits = recipe.limits.clone();
                self.dialog = Some(Dialog::CustomNlFit(edit));
            }
        }
    }

    pub(super) fn refit_report(
        &mut self,
        target: dialogs::FitEditTarget,
        recipe: Result<scicanvas_core::ops::NonlinearRecipe, String>,
    ) -> bool {
        let recipe = match recipe {
            Ok(r) => r,
            Err(e) => {
                self.status = e;
                return false;
            }
        };
        let reference = target.report;
        self.edit_with(
            "修改拟合设置",
            |project| {
                if !project.nlfit_operation(&reference)?.same_settings(&target.before) {
                    return Err("拟合设置已改变，请从报告重新打开编辑器".into());
                }
                project.edit_nlfit(&reference, recipe).map(Some)
            },
            |app, report| {
                app.status = "已原位更新拟合曲线和报告".into();
                app.dialog = Some(Dialog::NlFitReport(vec![report]));
            },
        )
        .is_some()
    }
}
