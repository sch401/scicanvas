use super::*;
use scicanvas_core::{interpolate::InterpMethod, ops::MergeDuplicates, signal::ConvolutionMode};
impl SciApp {
    pub(crate) fn open_signal_dialog(&mut self) {
        let Some(source) = self.analysis_source() else { return };
        match self.project.resolve(&source) {
            Ok(resolved) if resolved.items.len() == 2 => {
                let labels = resolved
                    .items
                    .iter()
                    .map(|item| format!("{}:{}", self.project.name_of(item.book_id).unwrap_or(""), item.y))
                    .collect();
                self.dialog = Some(Dialog::Signal(Box::new(dialogs::SignalEditor {
                    source,
                    labels,
                    inverse: false,
                    mode: ConvolutionMode::Full,
                    epsilon: 1e-3,
                })));
            }
            Ok(_) => self.status_warn("请依次选择信号和卷积核两个 Y 列".into()),
            Err(error) => self.status_err(error),
        }
    }
    pub(crate) fn open_average_dialog(&mut self) {
        self.dialog = Some(Dialog::Average {
            merge: false,
            mean: true,
        });
    }
    pub(super) fn run_curve_merge(&mut self, duplicate: Option<MergeDuplicates>) {
        let Some(source) = self.analysis_source() else { return };
        self.edit_with(
            if duplicate.is_some() {
                "曲线合并"
            } else {
                "曲线平均"
            },
            |project| {
                match duplicate {
                    Some(duplicate) => project.merge_curves(&source, duplicate),
                    None => project.average_curves(&source),
                }
                .map(Some)
            },
            |app, (book, graph)| {
                app.show_item(graph);
                app.dialog = None;
                app.status_ok(format!("结果已写入 {}", app.project.name_of(book).unwrap_or("")));
            },
        );
    }
    pub(super) fn apply_signal_action(&mut self, action: dialogs::SignalAction) {
        use dialogs::SignalAction as A;
        match action {
            A::Baseline(method, anchors) => {
                let Some(source) = self.analysis_source() else { return };
                self.edit_with(
                    "扣除基线",
                    |p| p.subtract_baseline_with_anchors(&source, method, &anchors).map(Some),
                    |app, keys| {
                        app.status_ok(format!("已扣除基线，生成 {} 组结果", keys.len()));
                        app.dialog = None;
                    },
                );
            }
            A::Peaks(window, threshold) => {
                self.finish_analysis_input(|app| app.find_peaks_method(threshold, Some(window)))
            }
            A::Interpolate(method, target) => {
                let Some(source) = self.analysis_source() else { return };
                self.edit_with(
                    "目标 X 插值",
                    |p| p.interpolate_at(&source, method, &target).map(Some),
                    |app, keys| {
                        app.status_ok(format!("结果在目标 X 右侧 {}", keys.join("、")));
                        app.dialog = None;
                    },
                );
            }
            A::Query(method, x) => self.query_interpolation(method, x),
            A::Merge(None) => self.average_curves(),
            A::Merge(duplicate) => self.run_curve_merge(duplicate),
            A::Signal(source, operation) => {
                self.edit_with(
                    "卷积与反卷积",
                    |p| p.signal_operation(&source, operation).map(Some),
                    |app, (graph, keys)| {
                        app.show_item(graph);
                        app.status_ok(format!("结果在新列 {}", keys.join("、")));
                        app.dialog = None;
                    },
                );
            }
        }
    }
    fn query_interpolation(&mut self, method: InterpMethod, x: f64) {
        let Some(source) = self.analysis_source() else { return };
        match self.project.interpolate_value(&source, method, x) {
            Ok(values) => {
                if let Some(Dialog::Interpolate { result, .. }) = &mut self.dialog {
                    *result = values;
                }
                self.status_ok("已查询单点插值".into());
            }
            Err(error) => self.status_err(error),
        }
    }
}
