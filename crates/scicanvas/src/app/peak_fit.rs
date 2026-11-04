use super::*;
use scicanvas_core::{
    nlfit::{FitFunction, PeakSum},
    ops::{FitWeight, Item, OpKind},
};
impl SciApp {
    pub(crate) fn open_peak_fit(&mut self) {
        let Some(source) = self.analysis_source() else { return };
        match self.project.resolve(&source) {
            Ok(resolved) if resolved.items.len() == 1 => {
                let item = &resolved.items[0];
                let (x, y, _) = scicanvas_core::ops::get_xy(self.project.book(item.book_id).unwrap(), item);
                self.dialog = Some(Dialog::PeakFit(Box::new(dialogs::PeakFitEditor::new(source, x, y))));
            }
            Ok(_) => self.status = "多峰拟合请选一条 Y 曲线".into(),
            Err(error) => self.status = error,
        }
    }
    pub(super) fn run_peak_fit(&mut self, request: dialogs::PeakFitRequest) {
        let function = FitFunction::PeakSum(request.peak);
        let constraints = match scicanvas_core::nlfit::parse_constraints_for_function(&function, &request.constraints) {
            Ok(c) => c,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        self.edit_with(
            "多峰拟合",
            |project| {
                project
                    .nlfit_weighted(
                        &request.source,
                        function,
                        Some(&request.initial),
                        &request.limits,
                        &constraints,
                        FitWeight::from_y_error(request.weighted),
                    )
                    .map(Some)
            },
            |app, (graph, reports)| {
                app.show_item(graph);
                app.status = "已生成总拟合与分量，支持自动重算".into();
                app.dialog = Some(Dialog::NlFitReport(reports));
            },
        );
    }
    pub(super) fn edit_peak_fit(
        &mut self,
        report: scicanvas_core::ops::NlFitReport,
        op: scicanvas_core::ops::Operation,
        peak: PeakSum,
    ) {
        let OpKind::Nonlinear(recipe) = &op.kind else {
            unreachable!()
        };
        let item = Item {
            book_id: report.book_id,
            y: op.y.clone(),
            x: op.x.clone(),
            rows: op.rows,
            visible_only: op.visible_only,
        };
        let (x, y, _) = scicanvas_core::ops::get_xy(self.project.book(report.book_id).unwrap(), &item);
        let source = Source::Book {
            id: report.book_id,
            selected: vec![op.y.clone()],
            rows: op.rows,
        };
        let mut edit = dialogs::PeakFitEditor::new(source, x, y);
        edit.shape = peak.shape;
        edit.count = peak.count;
        edit.baseline = peak.baseline;
        edit.initial = recipe
            .initial
            .as_ref()
            .unwrap_or(&report.fit.params)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        edit.limits = recipe.limits.clone();
        edit.constraints = recipe.constraints.clone();
        edit.weighted = recipe.weight == FitWeight::YError;
        edit.target = Some(Box::new(dialogs::FitEditTarget { report, before: op }));
        edit.preview();
        self.dialog = Some(Dialog::PeakFit(Box::new(edit)));
    }
}
