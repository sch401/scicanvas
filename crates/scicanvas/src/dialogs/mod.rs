//! 对话框状态与分派；各窗口的控件和报告逻辑按类型分文件。

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

mod analysis_curves;
mod anova;
mod areas;
mod baseline;
mod batch_export;
mod batch_plot;
mod calibration;
mod category_order;
mod column_math;
mod column_props;
mod convert_text;
mod correlations;
mod docking;
mod excel_export;
mod fft;
mod fft_filter;
mod fill;
mod filter;
mod find_replace;
mod fit_compare;
mod fit_report;
mod frequency;
mod global_fit;
mod ifft;
mod import_wizard;
mod interpolate;
mod layer_contents;
mod modal;
mod nl_fit;
mod nl_fit_report;
mod normalize;
mod peak_fit;
mod peaks;
pub(crate) use peak_fit::{PeakFitEditor, PeakFitRequest};
mod plot_object;
mod png;
mod poly;
mod scientific_text;
mod set_values;
mod sg_diff;
mod sheet_transform;
mod smooth;
mod sort;
mod t_test;
mod t_test_report;
mod table;
mod tables;
mod two_way_anova;
mod widgets;
mod window;

pub use batch_export::{BatchExport, BatchFormat};
pub use batch_plot::BatchPlot;
pub use column_props::ColumnEdit;
pub(crate) use docking::{begin_docks, end_docks, has_docks, place_docks, reset_docks, DockSide};
pub use filter::{FilterDialog, FilterEditor};
pub(crate) use find_replace::FindReplace;
pub(crate) use global_fit::GlobalFitEditor;
pub(crate) use import_wizard::ImportWizard;
pub use layer_contents::LayerContents;
pub(crate) use modal::modal;
pub use nl_fit::{CustomFitAction, CustomFitEditor, FitEditTarget};
pub(crate) use plot_object::PlotObjectEditor;
pub(crate) use scientific_text::{scientific_text, scientific_text_multiline};
pub(crate) use tables::md_table;
use tables::{md_cell, md_pairs, table_grid};
use widgets::buttons;
pub(crate) use widgets::{actions, center_buttons, edit_group, plain, primary, ActionKind, Keys, BUTTON};
pub(crate) use window::{floating_bounds, window};

pub(crate) use sheet_transform::TransformKind;
pub use sheet_transform::{SheetTransform, TransformAction};

pub enum Dialog {
    BatchExport(Box<BatchExport>),
    BatchPlot(Box<BatchPlot>),
    Filter(Box<FilterDialog>),
    ExportXlsx {
        headers: bool,
    },
    SheetTransform(Box<SheetTransform>),
    FindReplace(Box<FindReplace>),
    ImportWizard(Box<ImportWizard>),
    LayerContents(Box<LayerContents>),
    AnalysisCurves {
        graph: u64,
        layer: usize,
        curves: Vec<(scicanvas_core::plotspec::DataRef, String, bool)>,
    },
    /// 列属性：编辑内容、第一行的数值（数值列或日期列的毫秒，用于格式预览）、转换失败等错误。
    ColumnProps {
        book: u64,
        before: ColumnEdit,
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
    /// 类别顺序：工作簿、列和当前顺序的类别。
    CategoryOrder {
        book: u64,
        key: String,
        levels: Vec<String>,
        error: Option<String>,
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
        error: Option<String>,
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
    FitCompare {
        reports: Vec<NlFitReport>,
        first: usize,
        second: usize,
    },
    Calibration {
        book: u64,
        columns: Vec<(String, String)>,
        keys: [String; 3],
        reports: Vec<NlFitReport>,
        model: Option<usize>,
        forward: bool,
    },
    Areas(Vec<AreaReport>),
    /// 插值：方法、点数，以及是否使用自定义 X 范围（默认是数据的 X 范围）。
    Interpolate {
        method: InterpMethod,
        points: usize,
        custom: bool,
        from: f64,
        to: f64,
    },
    TwoWayAnova {
        book: u64,
        columns: Vec<(String, String)>,
        keys: [String; 3],
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
    /// 非线性拟合：选择内置函数；参数设置长度跟随当前函数。
    NlFit {
        model: Model,
        initial: String,
        target: Option<Box<FitEditTarget>>,
        /// 每个参数：(固定?, 固定值, 有下限?, 下限, 有上限?, 上限)。
        limits: Vec<(bool, f64, bool, f64, bool, f64)>,
        /// 参数间线性约束，一行一条。
        constraints: String,
        /// 按与 Y 列关联的 Y 误差列加权（1/σ²）。
        weighted: bool,
    },
    GlobalFit(Box<GlobalFitEditor>),
    PeakFit(Box<PeakFitEditor>),
    CustomNlFit(CustomFitEditor),
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
        /// 工作表有筛选：提示会改写隐藏行。
        filtered: bool,
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
        /// 保存公式，数据变化时自动重算。
        keep: bool,
        columns: Vec<(String, String)>,
        error: Option<String>,
        /// 工作表有筛选：提示会改写隐藏行。
        filtered: bool,
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
    Tiff {
        dpi: u32,
        page_mm: (f64, f64),
    },
}

/// 对话框里用户做出的选择，由 app.rs 执行。
pub enum Action {
    BatchExport(Vec<u64>, BatchFormat, u32),
    BatchPlot(u64, Vec<u64>),
    SetFilter(u64, String, Option<scicanvas_core::filter::FilterRule>, bool),
    ExportXlsx(bool),
    SheetTransform(u64, TransformAction),
    FindNext,
    ReplaceCurrent,
    ReplaceAll,
    Import(std::path::PathBuf, scicanvas_core::csv::ImportOptions),
    LayerContents(Box<LayerContents>),
    AnalysisCurves(u64, usize, Vec<scicanvas_core::plotspec::DataRef>),
    SaveNote(String, String),
    ResidualPlot(Vec<FitReport>),
    ResidualQQ(Vec<FitReport>),
    NlResidualPlot(Vec<NlFitReport>, bool),
    CompareFits(Box<[NlFitReport; 2]>),
    EvaluateCalibration {
        book: u64,
        keys: [String; 3],
        report: Option<Box<NlFitReport>>,
        forward: bool,
    },
    FitBands(Vec<FitReport>),
    NlFitBands(Vec<NlFitReport>),
    EditNlFit(Box<NlFitReport>),
    Refit(Box<FitEditTarget>, Result<scicanvas_core::ops::NonlinearRecipe, String>),
    ColumnProps(u64, ColumnEdit),
    None,
    Close,
    ConvertText(u64, String, usize, String),
    Fit(usize),
    Smooth(SmoothMethod, usize),
    SgDiff(usize, usize, usize),
    CategoryOrder(u64, String, Vec<String>),
    Sort(u64, Vec<(String, bool)>, Option<Vec<String>>),
    Ifft(u64, String, Option<String>, scicanvas_core::fft::InverseInput),
    Normalize(NormalizeMethod),
    ExportPng(u32),
    ExportTiff(u32),
    Interpolate(InterpMethod, usize, Option<(f64, f64)>),
    TTest(TTestKind, f64),
    TwoWayAnova(u64, [String; 3]),
    Frequency(Option<usize>, Option<(f64, f64)>),
    Fft(FftWindow),
    /// 最小突出度（占 Y 范围的比例）。
    Peaks(f64),
    Baseline(BaselineMethod),
    FftFilter(FilterKind),
    Fill(Sequence, usize),
    SetValues(String, bool),
    /// 内置函数、参数限制、约束文字、是否按 Y 误差加权。
    NlFit(Model, Vec<scicanvas_core::nlfit::ParamLimits>, String, bool),
    OpenCustomNlFit,
    GlobalFit(Box<GlobalFitEditor>),
    PeakFit(Box<PeakFitRequest>),
    CustomFitLibrary(Box<CustomFitAction>),
    CustomNlFit {
        name: String,
        formula: String,
        parameters: String,
        initial: String,
        constraints: String,
    },
    /// 运算符，以及常数（为空时用列 B）。
    ColumnMath(ColumnOp, Option<f64>),
}

pub fn show(ctx: &egui::Context, dialog: &mut Dialog, pal: &Palette, project: &scicanvas_core::Project) -> Action {
    dialog.show(ctx, pal, project)
}

impl Dialog {
    fn show(&mut self, ctx: &egui::Context, pal: &Palette, project: &scicanvas_core::Project) -> Action {
        let mut open = true;
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape)) && ctx.memory(|m| m.focused().is_none());
        let mut action = match self {
            Self::LayerContents(contents) => contents.show(ctx, project, pal),
            Self::FindReplace(state) => state.show(ctx, &mut open),
            Self::ExportXlsx { .. } => self.show_export_xlsx(ctx, &mut open),
            Self::BatchExport(state) => state.show(ctx, &mut open),
            Self::BatchPlot(state) => state.show(ctx, &mut open),
            Self::Filter(state) => state.show(ctx, &mut open),
            Self::ImportWizard(wizard) => wizard.show(ctx, pal, &mut open),
            Self::AnalysisCurves { .. } => self.show_analysis_curves(ctx, pal, &mut open, esc),
            Self::ColumnProps { .. } => self.show_column_props(ctx, pal, &mut open, esc),
            Self::ConvertText { .. } => self.show_convert_text(ctx, pal, &mut open, esc),
            Self::Poly { .. } => self.show_poly(ctx, pal, &mut open, esc),
            Self::Smooth { .. } => self.show_smooth(ctx, pal, &mut open, esc),
            Self::CategoryOrder { .. } => self.show_category_order(ctx, pal, &mut open, esc),
            Self::Sort { .. } => self.show_sort(ctx, pal, &mut open, esc),
            Self::SheetTransform(state) => state.show(ctx, &mut open),
            Self::Ifft { .. } => self.show_ifft(ctx, pal, &mut open, esc),
            Self::SgDiff { .. } => self.show_sg_diff(ctx, pal, &mut open, esc),
            Self::Normalize { .. } => self.show_normalize(ctx, pal, &mut open, esc),
            Self::FitCompare { .. } => self.show_fit_compare(ctx, &mut open),
            Self::Calibration { .. } => self.show_calibration(ctx, &mut open),
            Self::FitReport(..) => self.show_fit_report(ctx, pal, &mut open, esc),
            Self::Areas(..) => self.show_areas(ctx, pal, &mut open, esc),
            Self::Interpolate { .. } => self.show_interpolate(ctx, pal, &mut open, esc),
            Self::TwoWayAnova { .. } => self.show_two_way_anova(ctx, &mut open),
            Self::Anova { .. } => self.show_anova(ctx, pal, &mut open, esc),
            Self::Correlations(..) => self.show_correlations(ctx, pal, &mut open, esc),
            Self::NlFit { .. } => self.show_nl_fit(ctx, pal, &mut open, esc),
            Self::GlobalFit(editor) => editor.show(ctx, pal, &mut open),
            Self::PeakFit(edit) => edit.show(ctx, pal, &mut open),
            Self::CustomNlFit(..) => self.show_custom_nl_fit(ctx, pal, &mut open, esc, project),
            Self::NlFitReport(..) => self.show_nl_fit_report(ctx, pal, &mut open, esc, project),
            Self::FftFilter { .. } => self.show_fft_filter(ctx, pal, &mut open, esc),
            Self::Fill { .. } => self.show_fill(ctx, pal, &mut open, esc),
            Self::Table { .. } => self.show_table(ctx, pal, &mut open, esc),
            Self::SetValues { .. } => self.show_set_values(ctx, pal, &mut open, esc),
            Self::Baseline { .. } => self.show_baseline(ctx, pal, &mut open, esc),
            Self::Peaks { .. } => self.show_peaks(ctx, pal, &mut open, esc),
            Self::Fft { .. } => self.show_fft(ctx, pal, &mut open, esc),
            Self::ColumnMath { .. } => self.show_column_math(ctx, pal, &mut open, esc),
            Self::Frequency { .. } => self.show_frequency(ctx, pal, &mut open, esc),
            Self::TTest { .. } => self.show_t_test(ctx, pal, &mut open, esc),
            Self::TTestReport { .. } => self.show_t_test_report(ctx, pal, &mut open, esc),
            Self::Png { .. } | Self::Tiff { .. } => self.show_png(ctx, pal, &mut open, esc),
        };
        if !open || (esc && matches!(action, Action::None)) {
            action = Action::Close;
        }
        action
    }
}

#[cfg(test)]
mod docking_tests;
#[cfg(test)]
mod markdown_tests;
#[cfg(test)]
mod scientific_text_tests;
