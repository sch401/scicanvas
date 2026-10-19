//! 分析用的对话框：参数设置（多项式阶数、平滑、归一化）和结果报告（拟合、积分）。

use eframe::egui::{self, Align2, RichText};
use scicanvas_core::analysis::{NormalizeMethod, SmoothMethod};
use scicanvas_core::baseline::BaselineMethod;
use scicanvas_core::fft::{FilterKind, Window as FftWindow};
use scicanvas_core::fmt_num;
use scicanvas_core::interpolate::InterpMethod;
use scicanvas_core::nlfit::Model;
use scicanvas_core::ops::{AreaReport, ColumnOp, FitReport, NlFitReport};
use scicanvas_core::stats::Sequence;
use scicanvas_core::stats::{Anova, Correlation, TTest, TTestKind, TukeyPair};

use crate::theme::Palette;

/// 列属性对话框里编辑的内容。
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnEdit {
    pub key: String,
    pub long_name: String,
    pub units: String,
    pub comment: String,
    pub role: scicanvas_core::Role,
    pub kind: scicanvas_core::DataKind,
    pub format: scicanvas_core::format::DisplayFormat,
}

pub enum Dialog {
    /// 列属性：编辑内容、第一行的数值（数值列或日期列的毫秒，用于格式预览）、转换失败等错误。
    ColumnProps {
        edit: ColumnEdit,
        sample: Option<f64>,
        error: Option<String>,
    },
    ConvertText {
        book: u64,
        key: String,
        row: usize,
        value: String,
    },
    Poly {
        order: usize,
    },
    Smooth {
        method: SmoothMethod,
        window: usize,
    },
    /// IFFT：工作簿、实部列、虚部列（可无）及显示名、频点数、是否双边谱、单边谱原信号点数是否为奇数。
    Ifft {
        book: u64,
        re: (String, String),
        im: Option<(String, String)>,
        rows: usize,
        two_sided: bool,
        odd: bool,
    },
    /// 自定义排序：可选列（标识、显示名）、最多三级关键字（列序号，None 为不用；是否升序）、
    /// 选中的列及是否只排选中列。
    Sort {
        book: u64,
        columns: Vec<(String, String)>,
        keys: [(Option<usize>, bool); 3],
        selected: Vec<String>,
        only_selected: bool,
    },
    /// Savitzky–Golay 平滑求导：窗口点数、多项式阶数、导数阶数。
    SgDiff {
        window: usize,
        order: usize,
        deriv: usize,
    },
    Normalize {
        method: NormalizeMethod,
    },
    FitReport(Vec<FitReport>),
    Areas(Vec<AreaReport>),
    /// 插值：方法、点数，以及是否使用自定义 X 范围（默认是数据的 X 范围）。
    Interpolate {
        method: InterpMethod,
        points: usize,
        custom: bool,
        from: f64,
        to: f64,
    },
    /// 单因素方差分析结果和各组（列）名称。
    Anova {
        result: Anova,
        names: Vec<String>,
        /// Tukey HSD 两两比较（95% 置信）。
        tukey: Vec<TukeyPair>,
    },
    /// 相关系数：每一对列（名称 A、名称 B）的三种相关系数。
    Correlations(Vec<(String, String, Vec<Correlation>)>),
    /// 非线性拟合：选择内置函数；每个参数可固定为指定值（最多 4 个参数）。
    NlFit {
        model: Model,
        /// 每个参数：(固定?, 固定值, 有下限?, 下限, 有上限?, 上限)。
        limits: [(bool, f64, bool, f64, bool, f64); scicanvas_core::nlfit::MAX_PARAMS],
        /// 参数间线性约束，一行一条。
        constraints: String,
    },
    /// 非线性拟合结果。
    NlFitReport(Vec<NlFitReport>),
    /// FFT 滤波：类型（0 低通、1 高通、2 带通、3 带阻）、截止频率与奈奎斯特频率（仅作提示）。
    FftFilter {
        kind: u8,
        lo: f64,
        hi: f64,
        nyquist: f64,
    },
    /// 填充序列：目标列名、类型（0 等差、1 均匀随机、2 正态随机）、两个参数与行数。
    Fill {
        target: String,
        kind: u8,
        a: f64,
        b: f64,
        rows: usize,
    },
    /// 通用结果表：标题、表头、各行和说明，可复制到 Excel。
    Table {
        title: String,
        rows: Vec<Vec<String>>,
        note: String,
    },
    /// 设置列值：目标列名、公式和可引用的列（短名、显示名）。
    SetValues {
        target: String,
        formula: String,
        columns: Vec<(String, String)>,
        error: Option<String>,
    },
    /// 扣除基线：是否用 ALS、log10 λ 与非对称系数 p。
    Baseline {
        als: bool,
        log_lambda: f64,
        p: f64,
    },
    /// 寻峰：最小突出度（占 Y 范围的百分比）。
    Peaks {
        percent: f64,
    },
    /// FFT：窗函数。
    Fft {
        window: FftWindow,
    },
    /// 列运算：A 与 B 的列名（只选一列时 B 为空，只能用常数）、运算符和常数。
    ColumnMath {
        a: String,
        b: Option<String>,
        op: ColumnOp,
        use_constant: bool,
        constant: f64,
    },
    /// 频数统计：区间数（自动时按 Sturges 规则）和统计范围（默认是数据范围）。
    Frequency {
        auto: bool,
        bins: usize,
        custom: bool,
        from: f64,
        to: f64,
    },
    /// t 检验设置：`columns` 是参与检验的列名（按点选顺序）。
    TTest {
        columns: Vec<String>,
        kind: usize,
        mu: f64,
        equal_var: bool,
        level: f64,
    },
    /// t 检验结果和参与检验的列名。
    TTestReport {
        result: TTest,
        names: Vec<String>,
    },
    /// 导出 PNG：分辨率和页面毫米尺寸（用来显示像素数）。
    Png {
        dpi: u32,
        page_mm: (f64, f64),
    },
}

/// 对话框里用户做出的选择，由 app.rs 执行。
pub enum Action {
    SaveNote(String, String),
    ResidualPlot(Vec<FitReport>),
    FitBands(Vec<FitReport>),
    NlFitBands(Vec<NlFitReport>),
    ColumnProps(ColumnEdit),
    None,
    Close,
    ConvertText(u64, String, usize, String),
    Fit(usize),
    Smooth(SmoothMethod, usize),
    SgDiff(usize, usize, usize),
    Sort(u64, Vec<(String, bool)>, Option<Vec<String>>),
    Ifft(u64, String, Option<String>, scicanvas_core::fft::InverseInput),
    Normalize(NormalizeMethod),
    ExportPng(u32),
    Interpolate(InterpMethod, usize, Option<(f64, f64)>),
    TTest(TTestKind, f64),
    Frequency(Option<usize>, Option<(f64, f64)>),
    Fft(FftWindow),
    /// 最小突出度（占 Y 范围的比例）。
    Peaks(f64),
    Baseline(BaselineMethod),
    FftFilter(FilterKind),
    Fill(Sequence, usize),
    SetValues(String),
    NlFit(Model, Vec<scicanvas_core::nlfit::ParamLimits>, String),
    /// 运算符，以及常数（为空时用列 B）。
    ColumnMath(ColumnOp, Option<f64>),
}

/// t 检验报告的行（标签、各组或总体的值），供窗口和复制共用。
fn ttest_rows(r: &TTest, names: &[String]) -> Vec<(String, String)> {
    let mut rows = vec![("检验".to_string(), r.kind.label().to_string())];
    let groups = r.n.len();
    for g in 0..groups {
        let name = names.get(g).cloned().unwrap_or_default();
        let tag = if groups > 1 {
            format!("{}（{}）", ["A", "B"][g], name)
        } else {
            name
        };
        rows.push((format!("{tag} N"), r.n[g].to_string()));
        rows.push((format!("{tag} 均值"), fmt_num(r.mean[g])));
        rows.push((format!("{tag} 标准差"), fmt_num(r.sd[g])));
    }
    let (estimate, null) = match r.kind {
        TTestKind::OneSample { mu } => ("均值", format!("均值 = {}", fmt_num(mu))),
        TTestKind::TwoSample { .. } => ("均值差 A − B", "两组均值相等".to_string()),
        TTestKind::Paired => ("差值均值 A − B", "差值均值为 0".to_string()),
    };
    rows.push(("原假设".into(), null));
    rows.push((estimate.into(), fmt_num(r.estimate)));
    rows.push(("标准误".into(), fmt_num(r.se)));
    rows.push(("t".into(), fmt_num(r.t)));
    rows.push(("自由度".into(), fmt_num(r.df)));
    rows.push(("p（双侧）".into(), fmt_num(r.p)));
    rows.push((
        format!("{}% 置信区间", fmt_num(r.level * 100.0)),
        format!("{} ~ {}", fmt_num(r.ci.0), fmt_num(r.ci.1)),
    ));
    rows
}

/// 对话框窗口：首次出现在屏幕中央，可以拖动标题栏移动（每种对话框各自记住位置）。
pub(crate) fn window(ctx: &egui::Context, title: &str) -> egui::Window<'static> {
    egui::Window::new(RichText::new(title).strong())
        .id(egui::Id::new(("scicanvas-dialog", title)))
        .collapsible(false)
        .resizable(false)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(ctx.screen_rect().center())
        .min_width(380.0)
        .frame(egui::Frame::window(&ctx.style()).inner_margin(egui::Margin::symmetric(18.0, 14.0)))
}

/// 对话框按钮的统一尺寸。
pub(crate) const BUTTON: egui::Vec2 = egui::vec2(84.0, 28.0);

/// 让接下来 `n` 个统一尺寸的按钮在这一行里居中。
pub(crate) fn center_buttons(ui: &mut egui::Ui, n: usize) {
    let gap = ui.spacing().item_spacing.x;
    let used = n as f32 * BUTTON.x + n.saturating_sub(1) as f32 * gap;
    ui.add_space(((ui.available_width() - used) / 2.0).max(0.0));
}

/// 主按钮：强调色底、白字。
pub(crate) fn primary(ui: &egui::Ui, text: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text).color(egui::Color32::WHITE).strong())
        .fill(ui.visuals().selection.stroke.color)
        .min_size(BUTTON)
}

/// 普通按钮。
pub(crate) fn plain(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text).min_size(BUTTON)
}

fn buttons(ui: &mut egui::Ui, ok: &str) -> (bool, bool) {
    let mut res = (false, false);
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        center_buttons(ui, 2);
        res.0 = ui.add(primary(ui, ok)).clicked();
        res.1 = ui.add(plain("取消")).clicked();
    });
    res
}

pub fn show(ctx: &egui::Context, dialog: &mut Dialog, pal: &Palette) -> Action {
    let mut action = Action::None;
    let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    match dialog {
        Dialog::ColumnProps { edit, sample, error } => {
            use scicanvas_core::format::{DateFormat, NumberFormat};
            use scicanvas_core::{DataKind, Role};
            let title = format!("列属性 — {}", edit.key);
            window(ctx, &title).show(ctx, |ui| {
                egui::Grid::new("column-props")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("短名");
                        ui.label(RichText::new(edit.key.as_str()).strong())
                            .on_hover_text("图形通过短名引用列，创建后不再改变");
                        ui.end_row();
                        for (label, field) in [
                            ("长名称", &mut edit.long_name),
                            ("单位", &mut edit.units),
                            ("注释", &mut edit.comment),
                        ] {
                            ui.label(label);
                            ui.add(egui::TextEdit::singleline(field).desired_width(260.0));
                            ui.end_row();
                        }
                        ui.label("列属性");
                        let roles = [
                            (Role::X, "X"),
                            (Role::Y, "Y"),
                            (Role::XError, "X 误差"),
                            (Role::YError, "Y 误差"),
                            (Role::Z, "Z"),
                            (Role::Label, "标签"),
                            (Role::Ignore, "忽略"),
                        ];
                        let current = roles.iter().find(|(r, _)| *r == edit.role).map_or("Y", |(_, n)| *n);
                        egui::ComboBox::from_id_salt("column-role")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (role, name) in roles {
                                    ui.selectable_value(&mut edit.role, role, name);
                                }
                            });
                        ui.end_row();
                        ui.label("数据类型");
                        egui::ComboBox::from_id_salt("column-kind")
                            .selected_text(edit.kind.label())
                            .show_ui(ui, |ui| {
                                for kind in DataKind::ALL {
                                    ui.selectable_value(&mut edit.kind, kind, kind.label());
                                }
                            });
                        ui.end_row();
                        ui.label("显示格式");
                        match edit.kind {
                            DataKind::Num => {
                                ui.horizontal(|ui| {
                                    let mut index = edit.format.number.index();
                                    let mut places = edit.format.number.places();
                                    egui::ComboBox::from_id_salt("column-number-format")
                                        .selected_text(NumberFormat::KINDS[index])
                                        .show_ui(ui, |ui| {
                                            for (i, name) in NumberFormat::KINDS.iter().enumerate() {
                                                ui.selectable_value(&mut index, i, *name);
                                            }
                                        });
                                    ui.add_enabled_ui(index != 0, |ui| {
                                        ui.label("小数位");
                                        ui.add(
                                            egui::DragValue::new(&mut places)
                                                .range(0..=scicanvas_core::format::MAX_PLACES)
                                                .speed(0.1),
                                        );
                                    });
                                    edit.format.number = NumberFormat::from_index(index, places);
                                });
                            }
                            DataKind::DateTime => {
                                egui::ComboBox::from_id_salt("column-date-format")
                                    .selected_text(edit.format.date.label())
                                    .show_ui(ui, |ui| {
                                        for f in DateFormat::ALL {
                                            ui.selectable_value(&mut edit.format.date, f, f.label());
                                        }
                                    });
                            }
                            _ => {
                                ui.label(RichText::new("文本和分类列按原样显示").color(pal.weak));
                            }
                        }
                        ui.end_row();
                        let preview = match (edit.kind, sample) {
                            (DataKind::Num, Some(v)) => {
                                Some(edit.format.number.apply(*v).unwrap_or_else(|| fmt_num(*v)))
                            }
                            (DataKind::DateTime, Some(ms)) => edit.format.date.apply(*ms as i64),
                            _ => None,
                        };
                        if let Some(text) = preview {
                            ui.label("第 1 行显示为");
                            ui.label(RichText::new(text).monospace());
                            ui.end_row();
                        }
                    });
                if let Some(e) = error {
                    ui.add_space(4.0);
                    ui.label(RichText::new(e.as_str()).color(ui.visuals().error_fg_color));
                }
                ui.label(
                    RichText::new("显示格式只改变工作表里的显示，存储的数值和原始写法不变；编辑、复制和导出仍用原值。")
                        .small()
                        .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "确定");
                if ok {
                    action = Action::ColumnProps(edit.clone());
                } else if cancel || esc {
                    action = Action::Close;
                }
            });
        }
        Dialog::ConvertText { book, key, row, value } => {
            window(ctx, "转换为文本列？").show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("列 {key} 第 {} 行的输入与当前列类型不符", *row + 1))
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new("转换后整列已有内容原样保留，再写入本次输入；可以撤销。").color(pal.weak));
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    // 主按钮文字较长，按实际宽度居中
                    let gap = ui.spacing().item_spacing.x;
                    let wide = egui::vec2(132.0, BUTTON.y);
                    ui.add_space(((ui.available_width() - wide.x - BUTTON.x - gap) / 2.0).max(0.0));
                    if ui.add(primary(ui, "转换为文本并写入").min_size(wide)).clicked() {
                        action = Action::ConvertText(*book, key.clone(), *row, value.clone());
                    }
                    if ui.add(plain("取消")).clicked() || esc {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Poly { order } => {
            window(ctx, "多项式拟合").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("阶数");
                    egui::ComboBox::from_id_salt("poly-order")
                        .selected_text(order.to_string())
                        .show_ui(ui, |ui| {
                            for k in 2..=6 {
                                ui.selectable_value(order, k, k.to_string());
                            }
                        });
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列拟合。\n阶数越高越容易过拟合，一般用 2 或 3 阶。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "拟合");
                if ok {
                    action = Action::Fit(*order);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Smooth { method, window: w } => {
            window(ctx, "平滑").show(ctx, |ui| {
                egui::Grid::new("smooth-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("方法");
                        egui::ComboBox::from_id_salt("smooth-method")
                            .selected_text(method.label())
                            .show_ui(ui, |ui| {
                                for m in SmoothMethod::ALL {
                                    ui.selectable_value(method, m, m.label());
                                }
                            });
                        ui.end_row();
                        ui.label("窗口点数");
                        ui.add(egui::DragValue::new(w).range(3..=101).speed(1.0));
                        ui.end_row();
                    });
                ui.label(
                    RichText::new(match method {
                        SmoothMethod::Lowess => "窗口点数即每点参与局部回归的近邻数。",
                        SmoothMethod::FftFilter => "滤掉高于 1/(窗口点数·ΔX) 的频率，需要等间距 X。",
                        _ => "窗口点数取奇数。Savitzky-Golay 能更好地保留峰形。",
                    })
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "平滑");
                if ok {
                    action = Action::Smooth(*method, (*w).max(3) | 1);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Sort {
            book,
            columns,
            keys,
            selected,
            only_selected,
        } => {
            window(ctx, "排序").show(ctx, |ui| {
                egui::Grid::new("sort-grid")
                    .num_columns(3)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        for (level, (col, asc)) in keys.iter_mut().enumerate() {
                            ui.label(["主关键字", "次关键字", "第三关键字"][level]);
                            let text = col.map_or("无".to_string(), |i| columns[i].1.clone());
                            egui::ComboBox::from_id_salt(("sort-key", level))
                                .selected_text(text)
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    if level > 0 {
                                        ui.selectable_value(col, None, "无");
                                    }
                                    for (i, (_, label)) in columns.iter().enumerate() {
                                        ui.selectable_value(col, Some(i), label);
                                    }
                                });
                            ui.horizontal(|ui| {
                                ui.radio_value(asc, true, "升序");
                                ui.radio_value(asc, false, "降序");
                            });
                            ui.end_row();
                        }
                    });
                if !selected.is_empty() {
                    ui.checkbox(
                        only_selected,
                        format!("只排选中列（{}），其余列不动", selected.join("、")),
                    );
                }
                let (ok, cancel) = buttons(ui, "排序");
                if ok {
                    let keys: Vec<(String, bool)> = keys
                        .iter()
                        .filter_map(|(c, asc)| c.map(|i| (columns[i].0.clone(), *asc)))
                        .collect();
                    let within = (*only_selected && !selected.is_empty()).then(|| selected.clone());
                    action = Action::Sort(*book, keys, within);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Ifft {
            book,
            re,
            im,
            rows,
            two_sided,
            odd,
        } => {
            window(ctx, "IFFT").show(ctx, |ui| {
                egui::Grid::new("ifft-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("实部");
                        ui.label(&re.1);
                        ui.end_row();
                        ui.label("虚部");
                        ui.label(im.as_ref().map_or("无（按 0）", |c| c.1.as_str()));
                        ui.end_row();
                        ui.label("输入");
                        ui.horizontal(|ui| {
                            ui.radio_value(two_sided, false, "单边谱");
                            ui.radio_value(two_sided, true, "双边谱");
                        });
                        ui.end_row();
                        if !*two_sided {
                            use scicanvas_core::fft::InverseInput;
                            ui.label("输出点数");
                            ui.horizontal(|ui| {
                                let even = InverseInput::OneSided { odd: false }.points(*rows);
                                let oddn = InverseInput::OneSided { odd: true }.points(*rows);
                                ui.radio_value(odd, false, even.to_string());
                                ui.radio_value(odd, true, oddn.to_string());
                            });
                            ui.end_row();
                        }
                    });
                let (ok, cancel) = buttons(ui, "变换");
                if ok {
                    let input = if *two_sided {
                        scicanvas_core::fft::InverseInput::TwoSided
                    } else {
                        scicanvas_core::fft::InverseInput::OneSided { odd: *odd }
                    };
                    action = Action::Ifft(*book, re.0.clone(), im.as_ref().map(|c| c.0.clone()), input);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::SgDiff {
            window: w,
            order,
            deriv,
        } => {
            window(ctx, "SG 平滑求导").show(ctx, |ui| {
                egui::Grid::new("sgdiff-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("导数阶数");
                        ui.horizontal(|ui| {
                            ui.radio_value(deriv, 1, "一阶");
                            ui.radio_value(deriv, 2, "二阶");
                        });
                        ui.end_row();
                        ui.label("窗口点数");
                        ui.add(egui::DragValue::new(w).range(3..=101).speed(1.0));
                        ui.end_row();
                        ui.label("多项式阶数");
                        ui.add(egui::DragValue::new(order).range(1..=6).speed(0.1));
                        ui.end_row();
                    });
                // 窗口取奇数；阶数不低于导数阶数，并小于窗口点数 − 1
                *w = (*w).max(3) | 1;
                *order = (*order).clamp((*deriv).max(1), *w - 2);
                let (ok, cancel) = buttons(ui, "求导");
                if ok {
                    action = Action::SgDiff(*w, *order, *deriv);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Normalize { method } => {
            window(ctx, "归一化").show(ctx, |ui| {
                for m in NormalizeMethod::ALL {
                    ui.radio_value(method, m, m.label());
                }
                let (ok, cancel) = buttons(ui, "归一化");
                if ok {
                    action = Action::Normalize(*method);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::FitReport(reports) => {
            window(ctx, "拟合结果").show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                    for (ri, r) in reports.iter().enumerate() {
                        ui.label(RichText::new(format!("{} · {}", r.name, r.kind)).strong());
                        table_grid(ui, ("fit-grid", ri), &fit_param_rows(r, false), pal);
                        ui.add_space(4.0);
                        let mut stats = vec![
                            ("R²".to_string(), fmt_num(r.fit.r2)),
                            ("调整后 R²".to_string(), fmt_num(r.fit.adj_r2)),
                        ];
                        if let Some(pr) = r.fit.pearson_r() {
                            stats.push(("Pearson r".into(), fmt_num(pr)));
                        }
                        stats.push(("残差平方和".into(), fmt_num(r.fit.rss)));
                        stats.push(("数据点数 N".into(), r.fit.n.to_string()));
                        let stats: Vec<Vec<String>> = stats.into_iter().map(|(k, v)| vec![k, v]).collect();
                        table_grid(
                            ui,
                            ("fit-stats", ri),
                            &[vec!["统计量".into(), "值".into()]]
                                .into_iter()
                                .chain(stats)
                                .collect::<Vec<_>>(),
                            pal,
                        );
                        ui.add_space(4.0);
                        ui.label(RichText::new("方差分析").color(pal.weak));
                        table_grid(ui, ("fit-anova", ri), &fit_anova_rows(r), pal);
                        ui.add_space(4.0);
                        ui.label(RichText::new(r.fit.equation()).monospace());
                        ui.add_space(10.0);
                    }
                });
                ui.label(
                    RichText::new("拟合曲线和逐点残差已写入工作簿，拟合曲线加到图形中。")
                        .small()
                        .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    center_buttons(ui, 5);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        ui.ctx().copy_text(fit_text(reports));
                    }
                    if ui.add(plain("残差图")).clicked() {
                        action = Action::ResidualPlot(reports.clone());
                    }
                    if ui.add(plain("置信/预测带")).on_hover_text("95%").clicked() {
                        action = Action::FitBands(reports.clone());
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        action = Action::SaveNote("拟合结果".into(), fit_markdown(reports));
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Areas(areas) => {
            window(ctx, "积分结果").show(ctx, |ui| {
                egui::Grid::new("area-grid")
                    .striped(true)
                    .num_columns(4)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
                        for h in ["数据", "面积", "绝对面积", "点数"] {
                            ui.label(RichText::new(h).color(pal.weak));
                        }
                        ui.end_row();
                        for a in areas.iter() {
                            ui.label(&a.name);
                            ui.label(RichText::new(fmt_num(a.area)).monospace());
                            ui.label(RichText::new(fmt_num(a.abs_area)).monospace());
                            ui.label(a.n.to_string());
                            ui.end_row();
                        }
                    });
                ui.label(
                    RichText::new("梯形法。绝对面积把 X 轴下方的部分也按正值计入。累积积分已写入新列。")
                        .small()
                        .color(pal.weak),
                );
                if ui.button("关闭").clicked() {
                    action = Action::Close;
                }
            });
        }
        Dialog::Interpolate {
            method,
            points,
            custom,
            from,
            to,
        } => {
            window(ctx, "插值").show(ctx, |ui| {
                egui::Grid::new("interp-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("方法");
                        ui.horizontal(|ui| {
                            for m in InterpMethod::ALL {
                                ui.radio_value(method, m, m.label());
                            }
                        });
                        ui.end_row();
                        ui.label("点数");
                        ui.add(egui::DragValue::new(points).range(2..=100_000).speed(1.0));
                        ui.end_row();
                        ui.label("X 范围");
                        ui.horizontal(|ui| {
                            ui.checkbox(custom, "自定义");
                            ui.add_enabled(*custom, egui::DragValue::new(from).speed(0.1));
                            ui.label("到");
                            ui.add_enabled(*custom, egui::DragValue::new(to).speed(0.1));
                        });
                        ui.end_row();
                    });
                ui.label(
                    RichText::new(
                        "在等间距的 X 上求值，结果写成新的 X、Y 列并画成线。超出数据范围时按端点那一段外推。\nAkima 在数据突变处比三次样条更少过冲。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let valid = !*custom || (from.is_finite() && to.is_finite() && from < to);
                if !valid {
                    ui.colored_label(egui::Color32::from_rgb(182, 67, 66), "起点必须小于终点");
                }
                let (ok, cancel) = buttons(ui, "插值");
                if ok && valid {
                    action = Action::Interpolate(*method, *points, custom.then_some((*from, *to)));
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Anova { result, names, tukey } => {
            window(ctx, "单因素方差分析").show(ctx, |ui| {
                let (table, groups) = anova_rows(result, names);
                let pairs = tukey_rows(tukey, names);
                for (gi, rows) in [table, groups, pairs].iter().enumerate() {
                    if gi == 2 {
                        ui.label(RichText::new("Tukey HSD 多重比较（95% 置信区间）").strong());
                    }
                    egui::Grid::new(("anova-grid", gi))
                        .striped(true)
                        .num_columns(rows[0].len())
                        .spacing([18.0, 4.0])
                        .show(ui, |ui| {
                            for (ri, row) in rows.iter().enumerate() {
                                for (ci, cell) in row.iter().enumerate() {
                                    if ri == 0 {
                                        ui.label(RichText::new(cell).color(pal.weak));
                                    } else if ci == 0 {
                                        ui.label(cell);
                                    } else {
                                        ui.label(RichText::new(cell).monospace());
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    ui.add_space(8.0);
                }
                let verdict = if result.p < 0.05 {
                    "α = 0.05 时各组均值有显著差异"
                } else {
                    "α = 0.05 时不能认为各组均值有差异"
                };
                ui.label(
                    RichText::new(format!(
                        "{verdict}。每个点选的列作为一组，缺失值已跳过。\nTukey HSD 的 p 值已按多重比较校正，组大小不等时为 Tukey–Kramer。"
                    ))
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let (table, groups) = anova_rows(result, names);
                        let pairs = tukey_rows(tukey, names);
                        let text: Vec<String> = table
                            .iter()
                            .chain([vec![]].iter())
                            .chain(groups.iter())
                            .chain([vec![]].iter())
                            .chain(pairs.iter())
                            .map(|r| r.join("\t"))
                            .collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let (table, groups) = anova_rows(result, names);
                        let pairs = tukey_rows(tukey, names);
                        let text = format!(
                            "## 单因素方差分析\n\n{}\n### 各组统计量\n\n{}\n### Tukey HSD 多重比较\n\n{}",
                            md_table(&table),
                            md_table(&groups),
                            md_table(&pairs)
                        );
                        action = Action::SaveNote("方差分析结果".into(), text);
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Correlations(pairs) => {
            window(ctx, "相关系数").show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    egui::Grid::new("corr-grid")
                        .striped(true)
                        .num_columns(9)
                        .spacing([14.0, 4.0])
                        .show(ui, |ui| {
                            for h in correlation_header() {
                                ui.label(RichText::new(h).color(pal.weak));
                            }
                            ui.end_row();
                            for row in correlation_rows(pairs) {
                                for (i, cell) in row.iter().enumerate() {
                                    if i < 2 {
                                        ui.label(cell);
                                    } else {
                                        ui.label(RichText::new(cell).monospace());
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
                ui.label(
                    RichText::new(
                        "p 值为双侧。Pearson 和 Spearman 的 p 值用 t 分布，Kendall τ-b 用含并列校正的正态近似；\n只用两列都有数值的行。",
                    )
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let mut lines = vec![correlation_header().join("\t")];
                        lines.extend(correlation_rows(pairs).into_iter().map(|r| r.join("\t")));
                        ui.ctx().copy_text(lines.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let mut rows = vec![correlation_header().map(String::from).to_vec()];
                        rows.extend(correlation_rows(pairs));
                        let text = format!("## 相关系数\n\n{}", md_table(&rows));
                        action = Action::SaveNote("相关系数结果".into(), text);
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::NlFit {
            model,
            limits,
            constraints,
        } => {
            window(ctx, "非线性拟合").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("类别");
                    let mut category = model.category();
                    egui::ComboBox::from_id_salt("nlfit-category")
                        .selected_text(category)
                        .show_ui(ui, |ui| {
                            for c in Model::CATEGORIES {
                                ui.selectable_value(&mut category, c, c);
                            }
                        });
                    if category != model.category() {
                        if let Some(m) = Model::ALL.into_iter().find(|m| m.category() == category) {
                            *model = m;
                        }
                    }
                });
                egui::Grid::new("nlfit-models")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        let category = model.category();
                        for m in Model::ALL.into_iter().filter(|m| m.category() == category) {
                            ui.radio_value(model, m, m.name());
                            ui.label(RichText::new(m.formula()).monospace());
                            ui.end_row();
                        }
                    });
                ui.label(
                    RichText::new(format!(
                        "参数：{}。初值由数据自动估计，Levenberg–Marquardt 迭代；\n对当前图形的数据曲线，或工作簿里选中的 Y 列拟合，拟合曲线加到工作簿和图中。",
                        model.params().join("、")
                    ))
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(4.0);
                egui::Grid::new("nlfit-limits")
                    .num_columns(4)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("");
                        for head in ["固定", "下限", "上限"] {
                            ui.label(RichText::new(head).color(pal.weak));
                        }
                        ui.end_row();
                        for (name, (fix, fv, has_lo, lo, has_hi, hi)) in model.params().iter().zip(limits.iter_mut()) {
                            ui.label(*name);
                            let free = !*fix;
                            for (on, v, enabled) in [(fix, fv, true), (has_lo, lo, free), (has_hi, hi, free)] {
                                ui.add_enabled_ui(enabled, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.checkbox(on, "");
                                        ui.add_enabled(*on, egui::DragValue::new(v).speed(0.01));
                                    });
                                });
                            }
                            ui.end_row();
                        }
                    });
                ui.add_space(4.0);
                ui.label(RichText::new("参数约束").color(pal.weak));
                ui.add(
                    egui::TextEdit::multiline(constraints)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("每行一条，如 A1 + A2 = 1、t2 >= t1"),
                );
                let (ok, cancel) = buttons(ui, "拟合");
                if ok {
                    let held = model
                        .params()
                        .iter()
                        .zip(limits.iter())
                        .map(|(_, &(fix, fv, has_lo, lo, has_hi, hi))| scicanvas_core::nlfit::ParamLimits {
                            fixed: fix.then_some(fv),
                            lower: (!fix && has_lo).then_some(lo),
                            upper: (!fix && has_hi).then_some(hi),
                        })
                        .collect();
                    action = Action::NlFit(*model, held, constraints.clone());
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::NlFitReport(reports) => {
            window(ctx, "非线性拟合结果").show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                    for (ri, r) in reports.iter().enumerate() {
                        let f = &r.fit;
                        ui.label(RichText::new(format!("{} · {}", r.name, f.model.name())).strong());
                        ui.label(RichText::new(f.model.formula()).monospace().small());
                        for c in &f.constraints {
                            ui.label(RichText::new(format!("约束：{c}")).small().color(pal.weak));
                        }
                        if !f.converged {
                            ui.colored_label(
                                egui::Color32::from_rgb(182, 67, 66),
                                "迭代 500 次仍未收敛，结果仅供参考；可换用其他函数或检查数据",
                            );
                        }
                        table_grid(ui, ("nlfit-grid", ri), &nlfit_param_rows(f), pal);
                        ui.add_space(4.0);
                        let stats: Vec<Vec<String>> = [vec!["统计量".to_string(), "值".to_string()]]
                            .into_iter()
                            .chain(nlfit_stats(f).into_iter().map(|(k, v)| vec![k.to_string(), v]))
                            .collect();
                        table_grid(ui, ("nlfit-stats", ri), &stats, pal);
                        ui.add_space(10.0);
                    }
                });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    center_buttons(ui, 4);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        ui.ctx().copy_text(nlfit_text(reports));
                    }
                    if ui.add(plain("置信/预测带")).on_hover_text("95%").clicked() {
                        action = Action::NlFitBands(reports.clone());
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        action = Action::SaveNote("非线性拟合结果".into(), nlfit_markdown(reports));
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::FftFilter { kind, lo, hi, nyquist } => {
            window(ctx, "FFT 滤波").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (i, name) in ["低通", "高通", "带通", "带阻"].into_iter().enumerate() {
                        ui.radio_value(kind, i as u8, name);
                    }
                });
                let band = *kind >= 2;
                egui::Grid::new("fft-filter-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(if band { "下限频率" } else { "截止频率" });
                    ui.add(egui::DragValue::new(lo).range(0.0..=f64::MAX).speed(*nyquist / 200.0));
                    ui.end_row();
                    if band {
                        ui.label("上限频率");
                        ui.add(egui::DragValue::new(hi).range(0.0..=f64::MAX).speed(*nyquist / 200.0));
                        ui.end_row();
                    }
                });
                ui.label(
                    RichText::new(format!(
                        "奈奎斯特频率 {}（频率单位为 1/X，X 以 s 为单位时为 Hz）。\n对当前图形的数据曲线，或工作簿里选中的 Y 列做理想滤波：变换后把不通过的频率分量置零再逆变换。\n结果写成新列；来源是图形时也画进图里。X 需要等间距。",
                        fmt_num(*nyquist)
                    ))
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "滤波");
                if ok {
                    let (a, b) = (lo.min(*hi), lo.max(*hi));
                    action = Action::FftFilter(match kind {
                        0 => FilterKind::LowPass(*lo),
                        1 => FilterKind::HighPass(*lo),
                        2 => FilterKind::BandPass(a, b),
                        _ => FilterKind::BandStop(a, b),
                    });
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Fill {
            target,
            kind,
            a,
            b,
            rows,
        } => {
            window(ctx, "填充数据序列").show(ctx, |ui| {
                ui.label(RichText::new(format!("目标列：{target}")).strong());
                ui.horizontal(|ui| {
                    for (i, name) in ["等差序列", "均匀分布随机数", "正态分布随机数"].into_iter().enumerate()
                    {
                        ui.radio_value(kind, i as u8, name);
                    }
                });
                let (la, lb) = match kind {
                    0 => ("首项", "公差"),
                    1 => ("最小值", "最大值"),
                    _ => ("均值", "标准差"),
                };
                egui::Grid::new("fill-grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(la);
                        ui.add(egui::DragValue::new(a).speed(0.1));
                        ui.end_row();
                        ui.label(lb);
                        ui.add(egui::DragValue::new(b).speed(0.1));
                        ui.end_row();
                        ui.label("行数");
                        ui.add(egui::DragValue::new(rows).range(1..=1_000_000).speed(1.0));
                        ui.end_row();
                    });
                ui.label(
                    RichText::new("结果覆盖目标列，可撤销。随机数每次生成都不同。")
                        .small()
                        .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "填充");
                if ok {
                    let seq = match kind {
                        0 => Sequence::Arithmetic { start: *a, step: *b },
                        1 => Sequence::Uniform {
                            min: a.min(*b),
                            max: a.max(*b),
                        },
                        _ => Sequence::Normal { mean: *a, sd: b.abs() },
                    };
                    action = Action::Fill(seq, *rows);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Table { title, rows, note } => {
            window(ctx, title).show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    egui::Grid::new("result-table")
                        .striped(true)
                        .num_columns(rows.first().map_or(1, Vec::len))
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            for (ri, row) in rows.iter().enumerate() {
                                for (ci, cell) in row.iter().enumerate() {
                                    if ri == 0 {
                                        ui.label(RichText::new(cell).color(pal.weak));
                                    } else if ci == 0 {
                                        ui.label(cell);
                                    } else {
                                        ui.label(RichText::new(cell).monospace());
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
                ui.label(RichText::new(note.as_str()).small().color(pal.weak));
                ui.add_space(6.0);
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let text: Vec<String> = rows.iter().map(|r| r.join("\t")).collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let text = format!("## {title}\n\n{}\n{note}", md_table(rows));
                        action = Action::SaveNote(title.clone(), text.trim_end().to_string());
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::SetValues {
            target,
            formula,
            columns,
            error,
        } => {
            window(ctx, "设置列值").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{target} =")).strong());
                    let edit = ui.add(
                        egui::TextEdit::singleline(formula)
                            .desired_width(340.0)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("例如 A * 2 + sin(B)"),
                    );
                    if edit.changed() {
                        *error = None;
                    }
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        action = Action::SetValues(formula.clone());
                    }
                });
                if let Some(e) = error {
                    ui.label(RichText::new(e.as_str()).color(ui.visuals().error_fg_color));
                }
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("插入").small().color(pal.weak));
                    for (key, name) in columns.iter() {
                        if ui.small_button(key.as_str()).on_hover_text(name.as_str()).clicked() {
                            formula.push_str(key);
                        }
                    }
                    // 函数目录：按类别列出，点击插入“函数名(”
                    ui.menu_button("函数", |ui| {
                        use scicanvas_core::formula::{CATEGORIES, FUNCTIONS};
                        for category in CATEGORIES {
                            ui.menu_button(category, |ui| {
                                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                                    for func in FUNCTIONS.iter().filter(|f| f.category == category) {
                                        if ui.button(RichText::new(func.usage).monospace()).clicked() {
                                            formula.push_str(func.name);
                                            formula.push('(');
                                            ui.close_menu();
                                        }
                                    }
                                });
                            });
                        }
                    });
                });
                ui.label(
                    RichText::new(
                        "列用短名（A、B 或 col(A)），i 为行号，常数 pi、e；比较 < > <= >= == != 结果为 1 或 0。\n结果覆盖目标列，无效结果留空，可撤销。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "计算");
                if ok {
                    action = Action::SetValues(formula.clone());
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Baseline { als, log_lambda, p } => {
            window(ctx, "扣除基线").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(als, false, "两端连线");
                    ui.radio_value(als, true, "自动基线（ALS）");
                });
                ui.add_enabled_ui(*als, |ui| {
                    egui::Grid::new("als-grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                        ui.label("平滑度 log₁₀ λ");
                        ui.add(egui::DragValue::new(log_lambda).range(0.0..=12.0).speed(0.05).fixed_decimals(1));
                        ui.end_row();
                        ui.label("非对称系数 p");
                        ui.add(egui::DragValue::new(p).range(0.0001..=0.5).speed(0.001).fixed_decimals(4));
                        ui.end_row();
                    });
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列求基线，在 Y 右边写入“基线”和“扣除基线”两列；\n来源是图形时基线也画进图里。ALS（非对称最小二乘）适合光谱等峰朝上的数据：\nλ 越大基线越平直（常用 10²–10⁹），p 越小基线越贴近谷底（常用 0.001–0.05）。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "扣除");
                if ok {
                    action = Action::Baseline(if *als {
                        BaselineMethod::Als {
                            lambda: 10f64.powf(*log_lambda),
                            p: *p,
                        }
                    } else {
                        BaselineMethod::Line
                    });
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Peaks { percent } => {
            window(ctx, "寻峰与峰积分").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("最小突出度");
                    ui.add(egui::DragValue::new(percent).range(0.1..=100.0).speed(0.2).suffix(" %"));
                    ui.label(RichText::new("（占 Y 范围）").color(pal.weak));
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列寻峰（算法同 SciPy find_peaks）。\n突出度是峰顶比两侧较高的那个谷底高出多少，调大可以滤掉噪声小峰。\n每条曲线生成一个峰表：峰位、峰高、突出度、半峰宽（半突出度处）、面积和质心；\n面积在两侧到相邻峰之间的最低点内、扣除两端连线（线性基线）后积分。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "寻峰");
                if ok {
                    action = Action::Peaks(*percent / 100.0);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Fft { window: w } => {
            window(ctx, "FFT").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("窗函数");
                    for m in FftWindow::ALL {
                        ui.radio_value(w, m, m.label());
                    }
                });
                ui.label(
                    RichText::new(
                        "对当前图形的数据曲线，或工作簿里选中的 Y 列做 FFT。X 需要等间距（X 以 s 为单位时频率单位为 Hz）。\n每条曲线生成一个频谱表（频率、实部、虚部、幅值、相位、功率）和一张幅值谱图；\n幅值按正弦振幅归一，加窗时按窗函数的相干增益补偿。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "变换");
                if ok {
                    action = Action::Fft(*w);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::ColumnMath {
            a,
            b,
            op,
            use_constant,
            constant,
        } => {
            window(ctx, "列运算").show(ctx, |ui| {
                if b.is_none() {
                    *use_constant = true;
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(a.as_str()).strong());
                    for o in ColumnOp::ALL {
                        ui.selectable_value(op, o, o.symbol());
                    }
                    if *use_constant {
                        ui.add(egui::DragValue::new(constant).speed(0.1));
                    } else if let Some(b) = b.as_ref() {
                        ui.label(RichText::new(b.as_str()).strong());
                    }
                });
                ui.add_enabled_ui(b.is_some(), |ui| {
                    ui.checkbox(use_constant, "用常数代替第二列");
                });
                ui.label(
                    RichText::new(
                        "结果写成新列放在第一列右边。两列对应不同的 X 列时（如原始曲线减拟合曲线），\n先把第二列按线性插值到第一列的 X 上；除以 0 或缺失值处留空。\n只选一列时与常数运算，选两列时按点选顺序。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "计算");
                if ok {
                    action = Action::ColumnMath(*op, use_constant.then_some(*constant));
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::Frequency {
            auto,
            bins,
            custom,
            from,
            to,
        } => {
            window(ctx, "频数统计").show(ctx, |ui| {
                egui::Grid::new("freq-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("区间数");
                        ui.horizontal(|ui| {
                            ui.checkbox(auto, "自动（Sturges）");
                            ui.add_enabled(!*auto, egui::DragValue::new(bins).range(1..=10_000));
                        });
                        ui.end_row();
                        ui.label("统计范围");
                        ui.horizontal(|ui| {
                            ui.checkbox(custom, "自定义");
                            ui.add_enabled(*custom, egui::DragValue::new(from).speed(0.1));
                            ui.label("到");
                            ui.add_enabled(*custom, egui::DragValue::new(to).speed(0.1));
                        });
                        ui.end_row();
                    });
                ui.label(
                    RichText::new(
                        "每个选中的列生成一个新表：区间下限、中点、上限、频数、累积频数、相对频率和累积相对频率。\n区间等宽，最后一个区间包含右端点（与 numpy.histogram 相同）；范围外的值不计。",
                    )
                    .small()
                    .color(pal.weak),
                );
                let valid = !*custom || (from.is_finite() && to.is_finite() && from < to);
                if !valid {
                    ui.colored_label(egui::Color32::from_rgb(182, 67, 66), "起点必须小于终点");
                }
                let (ok, cancel) = buttons(ui, "统计");
                if ok && valid {
                    action = Action::Frequency((!*auto).then_some(*bins), custom.then_some((*from, *to)));
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::TTest {
            columns,
            kind,
            mu,
            equal_var,
            level,
        } => {
            window(ctx, "t 检验").show(ctx, |ui| {
                const KINDS: [&str; 3] = ["单样本", "双样本（独立）", "配对"];
                ui.label(format!("参与检验的列：{}", columns.join("、")));
                ui.horizontal(|ui| {
                    for (i, label) in KINDS.iter().enumerate() {
                        ui.radio_value(kind, i, *label);
                    }
                });
                egui::Grid::new("ttest-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        if *kind == 0 {
                            ui.label("假设均值");
                            ui.add(egui::DragValue::new(mu).speed(0.1));
                            ui.end_row();
                        }
                        if *kind == 1 {
                            ui.label("方差");
                            ui.checkbox(equal_var, "假设两组方差相等（不勾选为 Welch 检验）");
                            ui.end_row();
                        }
                        ui.label("置信水平");
                        ui.horizontal(|ui| {
                            for l in [0.9, 0.95, 0.99] {
                                ui.selectable_value(level, l, format!("{}%", fmt_num(l * 100.0)));
                            }
                        });
                        ui.end_row();
                    });
                let need = if *kind == 0 { 1 } else { 2 };
                let ready = columns.len() >= need;
                let hint = match *kind {
                    0 => "使用第 1 列。".to_string(),
                    1 => "比较第 1 列（A）与第 2 列（B）两组独立样本。".to_string(),
                    _ => "按行配对第 1 列（A）与第 2 列（B），只用两列都有数值的行。".to_string(),
                };
                ui.label(RichText::new(hint).small().color(pal.weak));
                if !ready {
                    ui.colored_label(
                        egui::Color32::from_rgb(182, 67, 66),
                        "请先在工作表里按顺序单击选中两列（Ctrl+单击多选）",
                    );
                }
                let (ok, cancel) = buttons(ui, "计算");
                if ok && ready {
                    let k = match *kind {
                        0 => TTestKind::OneSample { mu: *mu },
                        1 => TTestKind::TwoSample { equal_var: *equal_var },
                        _ => TTestKind::Paired,
                    };
                    action = Action::TTest(k, *level);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
        Dialog::TTestReport { result, names } => {
            window(ctx, "t 检验结果").show(ctx, |ui| {
                let rows = ttest_rows(result, names);
                egui::Grid::new("ttest-report")
                    .striped(true)
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
                        for (k, v) in &rows {
                            ui.label(k);
                            ui.label(RichText::new(v).monospace());
                            ui.end_row();
                        }
                    });
                let verdict = if result.p < 1.0 - result.level {
                    "在该显著性水平下拒绝原假设"
                } else {
                    "在该显著性水平下不能拒绝原假设"
                };
                ui.label(
                    RichText::new(format!(
                        "显著性水平 α = {}：{verdict}。缺失值和非数值单元格已跳过。",
                        fmt_num(1.0 - result.level)
                    ))
                    .small()
                    .color(pal.weak),
                );
                ui.add_space(6.0);
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    center_buttons(ui, 3);
                    if ui
                        .add(plain("复制结果"))
                        .on_hover_text("复制为制表符分隔的文本，可粘贴到 Excel")
                        .clicked()
                    {
                        let text: Vec<String> = rows.iter().map(|(k, v)| format!("{k}\t{v}")).collect();
                        ui.ctx().copy_text(text.join("\n"));
                    }
                    if ui.add(plain("保存为 Note")).clicked() {
                        let text = format!("## t 检验\n\n{}", md_pairs(["项目", "值"], &rows));
                        action = Action::SaveNote("t 检验结果".into(), text);
                    }
                    if ui.add(primary(ui, "关闭")).clicked() {
                        action = Action::Close;
                    }
                });
            });
        }
        Dialog::Png { dpi, page_mm } => {
            window(ctx, "导出 PNG").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("分辨率");
                    for preset in [150, 300, 600, 1200] {
                        ui.selectable_value(dpi, preset, format!("{preset} dpi"));
                    }
                    ui.add(egui::DragValue::new(dpi).range(72..=2400).suffix(" dpi"));
                });
                let px = |mm: f64| (mm / 25.4 * f64::from(*dpi)).round();
                ui.label(format!(
                    "页面 {} × {} mm，输出约 {} × {} 像素",
                    fmt_num(page_mm.0),
                    fmt_num(page_mm.1),
                    px(page_mm.0),
                    px(page_mm.1)
                ));
                ui.label(
                    RichText::new("期刊投稿的线条图一般用 600 dpi；PNG 记录了分辨率，插入 Word 时按页面实际尺寸显示。")
                        .small()
                        .color(pal.weak),
                );
                let (ok, cancel) = buttons(ui, "导出…");
                if ok {
                    action = Action::ExportPng(*dpi);
                } else if cancel {
                    action = Action::Close;
                }
            });
        }
    }
    if esc && matches!(action, Action::None) {
        action = Action::Close;
    }
    action
}

/// Tukey HSD 两两比较表（首行为表头）。
fn tukey_rows(pairs: &[TukeyPair], names: &[String]) -> Vec<Vec<String>> {
    let mut rows = vec![["组 A", "组 B", "均值差 A−B", "下限", "上限", "p", "α = 0.05"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()];
    for t in pairs {
        rows.push(vec![
            names[t.a].clone(),
            names[t.b].clone(),
            fmt_num(t.diff),
            fmt_num(t.low),
            fmt_num(t.high),
            fmt_num(t.p),
            if t.p < 0.05 { "显著" } else { "不显著" }.into(),
        ]);
    }
    rows
}

/// ANOVA 表与各组统计量（首行为表头），供窗口和复制共用。
fn anova_rows(a: &Anova, names: &[String]) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let row = |cells: &[&str]| cells.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let table = vec![
        row(&["来源", "平方和", "自由度", "均方", "F", "p"]),
        vec![
            "组间".into(),
            fmt_num(a.ss_between),
            fmt_num(a.df_between),
            fmt_num(a.ms_between()),
            fmt_num(a.f),
            fmt_num(a.p),
        ],
        vec![
            "组内".into(),
            fmt_num(a.ss_within),
            fmt_num(a.df_within),
            fmt_num(a.ms_within()),
            String::new(),
            String::new(),
        ],
        vec![
            "总计".into(),
            fmt_num(a.ss_between + a.ss_within),
            fmt_num(a.df_between + a.df_within),
            String::new(),
            String::new(),
            String::new(),
        ],
    ];
    let mut groups = vec![row(&["组", "N", "均值", "标准差"])];
    for (i, name) in names.iter().enumerate() {
        groups.push(vec![
            name.clone(),
            a.n[i].to_string(),
            fmt_num(a.mean[i]),
            fmt_num(a.sd[i]),
        ]);
    }
    (table, groups)
}

fn correlation_header() -> [&'static str; 9] {
    [
        "列 A",
        "列 B",
        "N",
        "Pearson r",
        "p",
        "Spearman ρ",
        "p",
        "Kendall τ-b",
        "p",
    ]
}

fn correlation_rows(pairs: &[(String, String, Vec<Correlation>)]) -> Vec<Vec<String>> {
    pairs
        .iter()
        .map(|(a, b, cs)| {
            let mut row = vec![
                a.clone(),
                b.clone(),
                cs.first().map_or(String::new(), |c| c.n.to_string()),
            ];
            for c in cs {
                row.push(fmt_num(c.r));
                row.push(fmt_num(c.p));
            }
            row
        })
        .collect()
}

/// 标准误文字：固定参数写“固定”，停在上下限上的参数后面注明“（边界）”。
fn nlfit_se(f: &scicanvas_core::nlfit::NlFit, k: usize) -> String {
    if f.fixed[k] {
        "固定".into()
    } else if f.at_bound[k] {
        format!("{}（边界）", fmt_num(f.se[k]))
    } else {
        fmt_num(f.se[k])
    }
}

fn nlfit_stats(f: &scicanvas_core::nlfit::NlFit) -> Vec<(&'static str, String)> {
    vec![
        ("R²", fmt_num(f.r2)),
        ("调整后 R²", fmt_num(f.adj_r2)),
        ("残差平方和", fmt_num(f.rss)),
        ("约化 χ²", fmt_num(f.reduced_chi2)),
        ("数据点数 N", f.n.to_string()),
        ("迭代次数", f.iterations.to_string()),
    ]
}

/// Markdown 表格单元格：竖线转义，换行改成空格。
fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace(['\r', '\n'], " ")
}

/// Markdown 表格（首行为表头），末尾带换行；空表返回空串。
pub(crate) fn md_table(rows: &[Vec<String>]) -> String {
    let Some(head) = rows.first() else { return String::new() };
    let line = |r: &Vec<String>| {
        let cells: Vec<String> = (0..head.len())
            .map(|i| md_cell(r.get(i).map_or("", String::as_str)))
            .collect();
        format!("| {} |\n", cells.join(" | "))
    };
    let mut out = line(head);
    out.push_str(&format!("|{}\n", " --- |".repeat(head.len())));
    for r in &rows[1..] {
        out.push_str(&line(r));
    }
    out
}

/// 两列“名称 | 值”的 Markdown 表格。
fn md_pairs(head: [&str; 2], rows: &[(String, String)]) -> String {
    let mut table = vec![head.map(String::from).to_vec()];
    table.extend(rows.iter().map(|(k, v)| vec![k.clone(), v.clone()]));
    md_table(&table)
}

/// 结果表格：首行为表头（淡色），首列为名称，其余等宽数字。
fn table_grid(ui: &mut egui::Ui, id: impl std::hash::Hash, rows: &[Vec<String>], pal: &Palette) {
    egui::Grid::new(id)
        .striped(true)
        .num_columns(rows.first().map_or(1, Vec::len))
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            for (ri, row) in rows.iter().enumerate() {
                for (ci, cell) in row.iter().enumerate() {
                    if ri == 0 {
                        ui.label(RichText::new(cell).color(pal.weak));
                    } else if ci == 0 {
                        ui.label(cell);
                    } else {
                        ui.label(RichText::new(cell).monospace());
                    }
                }
                ui.end_row();
            }
        });
}

/// 线性/多项式参数表：参数、值、标准误、t、p。`exact` 为真时数值保留全部位数（复制到 Excel 用）。
fn fit_param_rows(r: &FitReport, exact: bool) -> Vec<Vec<String>> {
    let num = |v: f64| if exact { format!("{v}") } else { fmt_num(v) };
    let linear = r.fit.coef.len() == 2;
    let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
    for (k, ((c, se), (t, p))) in r.fit.coef.iter().zip(&r.fit.se).zip(r.fit.t_tests()).enumerate() {
        let name = match (linear, k) {
            (true, 0) => "截距 A".to_string(),
            (true, _) => "斜率 B".to_string(),
            (false, 0) => "B0（常数项）".to_string(),
            _ => format!("B{k}"),
        };
        rows.push(vec![name, num(*c), num(*se), num(t), num(p)]);
    }
    rows
}

/// 拟合方差分析表：来源、自由度、平方和、均方、F、p。
fn fit_anova_rows(r: &FitReport) -> Vec<Vec<String>> {
    let a = r.fit.anova();
    vec![
        ["来源", "自由度", "平方和", "均方", "F", "p"]
            .map(String::from)
            .to_vec(),
        vec![
            "回归".into(),
            fmt_num(a.df_reg),
            fmt_num(a.ss_reg),
            fmt_num(a.ms_reg()),
            fmt_num(a.f),
            fmt_num(a.p),
        ],
        vec![
            "残差".into(),
            fmt_num(a.df_err),
            fmt_num(a.ss_err),
            fmt_num(a.ms_err()),
            String::new(),
            String::new(),
        ],
        vec![
            "总计".into(),
            fmt_num(a.df_tot),
            fmt_num(a.ss_tot),
            String::new(),
            String::new(),
            String::new(),
        ],
    ]
}

/// 非线性参数表：参数、值、标准误、t、p（固定参数只写值）。
fn nlfit_param_rows(f: &scicanvas_core::nlfit::NlFit) -> Vec<Vec<String>> {
    let mut rows = vec![["参数", "值", "标准误", "t", "p"].map(String::from).to_vec()];
    for (k, (name, (t, p))) in f.model.params().iter().zip(f.t_tests()).enumerate() {
        let (t, p) = if f.fixed[k] {
            (String::new(), String::new())
        } else {
            (fmt_num(t), fmt_num(p))
        };
        rows.push(vec![name.to_string(), fmt_num(f.params[k]), nlfit_se(f, k), t, p]);
    }
    rows
}

fn nlfit_markdown(reports: &[NlFitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let f = &r.fit;
            let stats: Vec<(String, String)> = nlfit_stats(f).into_iter().map(|(k, v)| (k.to_string(), v)).collect();
            let constraints: String = f.constraints.iter().map(|c| format!("约束：`{c}`\n\n")).collect();
            format!(
                "## {} — {}\n\n`{}`\n\n{}{}\n{}",
                md_cell(&r.name),
                f.model.name(),
                f.model.formula(),
                constraints,
                md_table(&nlfit_param_rows(f)),
                md_pairs(["统计量", "值"], &stats)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn fit_markdown(reports: &[FitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let stats = [
                ("R²", fmt_num(r.fit.r2)),
                ("调整后 R²", fmt_num(r.fit.adj_r2)),
                ("残差平方和", fmt_num(r.fit.rss)),
                ("N", r.fit.n.to_string()),
            ]
            .map(|(k, v)| (k.to_string(), v));
            format!(
                "## {} — {}\n\n`{}`\n\n{}\n{}\n### 方差分析\n\n{}",
                md_cell(&r.name),
                r.kind,
                r.fit.equation(),
                md_table(&fit_param_rows(r, false)),
                md_pairs(["统计量", "值"], &stats),
                md_table(&fit_anova_rows(r))
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn nlfit_text(reports: &[NlFitReport]) -> String {
    let mut lines = vec![];
    for r in reports {
        let f = &r.fit;
        lines.push(format!("{}\t{}\t{}", r.name, f.model.name(), f.model.formula()));
        lines.extend(f.constraints.iter().map(|c| format!("约束\t{c}")));
        lines.extend(nlfit_param_rows(f).iter().map(|row| row.join("\t")));
        for (k, v) in nlfit_stats(f) {
            lines.push(format!("{k}\t{v}"));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn fit_text(reports: &[FitReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let mut lines = vec![format!("{}\t{}", r.name, r.kind), r.fit.equation()];
            lines.extend(fit_param_rows(r, true).iter().map(|row| row.join("\t")));
            lines.push(format!("R²\t{}", r.fit.r2));
            lines.push(format!("调整后 R²\t{}", r.fit.adj_r2));
            lines.push(format!("残差平方和\t{}", r.fit.rss));
            lines.push(format!("N\t{}", r.fit.n));
            lines.push(String::new());
            lines.extend(fit_anova_rows(r).iter().map(|row| row.join("\t")));
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod markdown_tests {
    use super::*;

    #[test]
    fn markdown_tables_escape_cells_and_pad_short_rows() {
        let rows = vec![
            vec!["列 A".to_string(), "p".to_string()],
            vec!["a|b".to_string(), "0.01".to_string()],
            vec!["两\n行".to_string()],
        ];
        assert_eq!(
            md_table(&rows),
            "| 列 A | p |\n| --- | --- |\n| a\\|b | 0.01 |\n| 两 行 |  |\n"
        );
        assert_eq!(md_table(&[]), "");
        assert_eq!(
            md_pairs(["项目", "值"], &[("t".into(), "2".into())]),
            "| 项目 | 值 |\n| --- | --- |\n| t | 2 |\n"
        );
    }
}
