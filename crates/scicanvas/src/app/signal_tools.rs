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
                    swapped: false,
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
        let swapped = matches!(&action, dialogs::SignalAction::SignalSwapped(..));
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
            A::Signal(source, operation) | A::SignalSwapped(source, operation) => {
                self.edit_with(
                    "卷积与反卷积",
                    |p| signal_in_order(p, &source, operation, swapped).map(Some),
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

// The existing Source resolver preserves workbook/plot order. Temporarily reorder
// just these inputs for the core operation, then retain all original ordering and
// newly inserted output columns/plots at their source anchors.
fn restored_keys(original: &[String], current: Vec<String>) -> Vec<String> {
    let original_set: HashSet<&str> = original.iter().map(String::as_str).collect();
    let mut groups: HashMap<Option<String>, Vec<String>> = HashMap::new();
    let mut anchor = None;
    for key in current {
        if original_set.contains(key.as_str()) {
            anchor = Some(key);
        } else {
            groups.entry(anchor.clone()).or_default().push(key);
        }
    }
    let mut result = groups.remove(&None).unwrap_or_default();
    for key in original {
        result.push(key.clone());
        result.extend(groups.remove(&Some(key.clone())).unwrap_or_default());
    }
    result
}
fn signal_in_order(
    project: &mut Project,
    source: &Source,
    operation: scicanvas_core::signal::SignalOperation,
    swapped: bool,
) -> Result<(u64, Vec<String>), String> {
    if !swapped {
        return project.signal_operation(source, operation);
    }
    let resolved = project.resolve(source)?;
    if resolved.items.len() != 2 {
        return Err("请选择信号和卷积核两条 Y 列".into());
    }
    let mut base = source;
    while let Source::Visible(inner) = base {
        base = inner;
    }
    match base {
        Source::Book { id, .. } => {
            let book = project.book_mut(*id).unwrap();
            let original: Vec<_> = book.columns.iter().map(|c| c.key.clone()).collect();
            // Keep each Y next to its original X, so filtering finite rows does not
            // change when the two inputs use different X columns.
            let mut order = Vec::new();
            for item in resolved.items.iter().rev() {
                if let Some(x) = &item.x {
                    if !order.contains(x) {
                        order.push(x.clone());
                    }
                }
                order.push(item.y.clone());
            }
            let chosen: HashSet<_> = order.iter().cloned().collect();
            order.extend(original.iter().filter(|k| !chosen.contains(*k)).cloned());
            let mut columns: HashMap<_, _> = std::mem::take(&mut book.columns)
                .into_iter()
                .map(|c| (c.key.clone(), c))
                .collect();
            book.columns = order.into_iter().map(|k| columns.remove(&k).unwrap()).collect();
            let result = project.signal_operation(source, operation);
            let book = project.book_mut(*id).unwrap();
            let keys = restored_keys(&original, book.columns.iter().map(|c| c.key.clone()).collect());
            let mut columns: HashMap<_, _> = std::mem::take(&mut book.columns)
                .into_iter()
                .map(|c| (c.key.clone(), c))
                .collect();
            book.columns = keys.into_iter().map(|k| columns.remove(&k).unwrap()).collect();
            result
        }
        Source::Graph(graph_id) | Source::Layer { graph_id, .. } | Source::Curves { graph_id, .. } => {
            let index = resolved.layer_index;
            let layer = &mut project.graph_mut(*graph_id).unwrap().page.layers_mut()[index];
            let original = layer.plot_keys(true);
            let a = scicanvas_core::plotspec::DataRef::new(resolved.items[0].book_id, &resolved.items[0].y)
                .key(layer.book_id);
            let b = scicanvas_core::plotspec::DataRef::new(resolved.items[1].book_id, &resolved.items[1].y)
                .key(layer.book_id);
            layer.edit_plot_keys(|keys| {
                let i = keys.iter().position(|k| k == &a).unwrap();
                let j = keys.iter().position(|k| k == &b).unwrap();
                keys.swap(i, j);
            });
            let result = project.signal_operation(source, operation);
            let layer = &mut project.graph_mut(*graph_id).unwrap().page.layers_mut()[index];
            layer.edit_plot_keys(|keys| *keys = restored_keys(&original, std::mem::take(keys)));
            result
        }
        Source::Visible(_) => unreachable!(),
    }
}

#[cfg(test)]
mod ordered_tests {
    use super::*;
    #[test]
    fn h3_signal_swap_changes_values_without_reordering_sources() {
        use scicanvas_core::signal::{ConvolutionMode, SignalOperation};
        for graph in [false, true] {
            for separate_x in [false, true] {
                let mut p = Project::sample();
                p.books[0].columns[0].data = ["0", "1", "2", "3"].into_iter().map(String::from).collect();
                p.books[0].columns[1].data = ["1", "2", "3", "4"].into_iter().map(String::from).collect();
                p.books[0].columns[2].data = ["1", "1"].into_iter().map(String::from).collect();
                if separate_x {
                    let book = &mut p.books[0];
                    book.columns[0].data = ["0", "1", "NaN", "3"].into_iter().map(String::from).collect();
                    let key = book.push_column(Role::X);
                    let mut column = book.columns.pop().unwrap();
                    assert_eq!(column.key, key);
                    column.data = ["0", "NaN", "2", "3"].into_iter().map(String::from).collect();
                    book.columns.insert(2, column);
                }
                let source = if graph {
                    Source::Graph(2)
                } else {
                    Source::Book {
                        id: 1,
                        selected: vec!["B".into(), "C".into()],
                        rows: None,
                    }
                };
                let original = p.book(1).unwrap().columns.clone();
                let plots = p.graph(2).unwrap().first_layer().plot_keys(true);
                let items = p.resolve(&source).unwrap().items;
                let (_, first, _) = scicanvas_core::ops::get_xy(p.book(1).unwrap(), &items[0]);
                let (_, second, _) = scicanvas_core::ops::get_xy(p.book(1).unwrap(), &items[1]);
                let expected = scicanvas_core::signal::convolve(&second, &first, ConvolutionMode::Same);
                let (_, keys) =
                    signal_in_order(&mut p, &source, SignalOperation::Convolve(ConvolutionMode::Same), true).unwrap();
                assert_eq!(p.book(1).unwrap().column(&keys[1]).unwrap().values(), expected);
                let restored: Vec<_> = p
                    .book(1)
                    .unwrap()
                    .columns
                    .iter()
                    .filter(|c| original.iter().any(|o| o.key == c.key))
                    .cloned()
                    .collect();
                assert_eq!(restored, original);
                let restored: Vec<_> = p
                    .graph(2)
                    .unwrap()
                    .first_layer()
                    .plot_keys(true)
                    .into_iter()
                    .filter(|k| plots.contains(k))
                    .collect();
                assert_eq!(restored, plots);
            }
        }
    }
}
