use super::*;
use scicanvas_core::{Book, Graph, GraphTemplate};
use std::collections::{BTreeMap, BTreeSet};

fn match_column(project: &Project, source_book: u64, key: &str, target: &Book) -> Result<String, String> {
    let (column_key, foreign_book) = scicanvas_core::plotspec::split_key(key);
    let source = project
        .book(foreign_book.unwrap_or(source_book))
        .ok_or("找不到模板图形的源工作簿")?;
    let column = source
        .column(column_key)
        .ok_or_else(|| format!("找不到模板源列 {column_key}"))?;
    if column.long_name.trim().is_empty() {
        return target
            .column(column_key)
            .map(|c| c.key.clone())
            .ok_or_else(|| format!("工作簿 {} 缺少列 {column_key}", target.name));
    }
    let name = column.long_name.trim();
    let matches: Vec<_> = target.columns.iter().filter(|c| c.long_name.trim() == name).collect();
    match matches.as_slice() {
        [column] => Ok(column.key.clone()),
        [] => Err(format!("工作簿 {} 缺少列名 {name}", target.name)),
        _ => Err(format!("工作簿 {} 的列名 {name} 不唯一", target.name)),
    }
}

fn remap_template(project: &Project, graph: &Graph, target: &Book, template: &mut GraphTemplate) -> Result<(), String> {
    for (source, layer) in graph.page.layers().iter().zip(template.page.layers_mut()) {
        let Some(source_book) = source.book_id else { continue };
        let map = |key: &str| match_column(project, source_book, key, target);
        let mut draft = source.draft();
        draft.book_id = None;
        draft.x_key = draft.x_key.as_deref().map(map).transpose()?;
        draft.y_keys = draft.y_keys.iter().map(|key| map(key)).collect::<Result<_, _>>()?;
        if draft.y_keys.iter().collect::<BTreeSet<_>>().len() != draft.y_keys.len() {
            return Err(format!("工作簿 {} 的目标列重复", target.name));
        }
        draft.x_by_y = draft
            .x_by_y
            .into_iter()
            .map(|(key, value)| Ok((map(&key)?, map(&value)?)))
            .collect::<Result<_, String>>()?;
        draft.base_by_y = draft
            .base_by_y
            .into_iter()
            .map(|(key, value)| Ok((map(&key)?, map(&value)?)))
            .collect::<Result<_, String>>()?;
        draft.row_by_y = draft
            .row_by_y
            .into_iter()
            .map(|(key, value)| Ok((map(&key)?, value)))
            .collect::<Result<_, String>>()?;
        draft.line_only = draft.line_only.iter().map(|key| map(key)).collect::<Result<_, _>>()?;
        let mut styles = BTreeMap::new();
        for (key, mut style) in source.styles() {
            let key = map(&key)?;
            style.color_column = style.color_column.as_deref().map(map).transpose()?;
            style.size_column = style.size_column.as_deref().map(map).transpose()?;
            style.symbol_column = style.symbol_column.as_deref().map(map).transpose()?;
            if styles.insert(key, style).is_some() {
                return Err(format!("工作簿 {} 的目标列重复", target.name));
            }
        }
        layer.labels.column = source.labels.column.as_deref().map(map).transpose()?;
        layer.spec = scicanvas_core::page::LayerSpec::from(draft);
        layer.spec.edit_styles(|current| *current = styles);
        layer.book_id = None;
    }
    Ok(())
}

fn plot_for_books(project: &mut Project, graph_id: u64, books: &[u64]) -> Result<Vec<u64>, String> {
    if books.is_empty() {
        return Err("请选择至少一个工作簿".into());
    }
    let graph = project.graph(graph_id).cloned().ok_or("找不到模板图形")?;
    let original = scicanvas_core::project_file::graph_template_from_graph(&graph, project)?;
    let mut results = Vec::new();
    for id in books {
        let target = project.book(*id).ok_or("找不到目标工作簿")?;
        let mut template = original.clone();
        remap_template(project, &graph, target, &mut template)?;
        results.push(project.apply_graph_template(&template, *id)?);
    }
    Ok(results)
}

impl SciApp {
    pub(crate) fn open_batch_plot(&mut self) {
        if self.project.graphs.is_empty() || self.project.books.is_empty() {
            self.status = "需要至少一个图形和工作簿".into();
            return;
        }
        let selected_graph = self
            .active
            .and_then(|id| self.project.graphs.iter().position(|g| g.id == id))
            .unwrap_or(0);
        self.dialog = Some(Dialog::BatchPlot(Box::new(dialogs::BatchPlot {
            graphs: self.project.graphs.iter().map(|g| (g.id, g.name.clone())).collect(),
            selected_graph,
            books: self
                .project
                .books
                .iter()
                .map(|b| (b.id, b.name.clone(), false))
                .collect(),
        })));
    }

    pub(crate) fn create_batch_plots(&mut self, graph: u64, books: Vec<u64>) -> Option<Vec<u64>> {
        self.edit_with(
            "批量绘图",
            |project| plot_for_books(project, graph, &books).map(Some),
            |app, ids| {
                if let Some(id) = ids.last() {
                    app.show_item(*id);
                }
                app.status = format!("已生成 {} 张图", ids.len());
                app.dialog = None;
                ids
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(app: &mut SciApp, name: &str) -> u64 {
        let id = app.project.alloc_id();
        let mut book = app.project.book(1).unwrap().clone();
        book.id = id;
        book.name = name.into();
        book.columns.swap(0, 1);
        book.columns[0].key = "A".into();
        book.columns[1].key = "B".into();
        app.project.books.push(book);
        id
    }

    #[test]
    fn batch_plot_matches_column_names_and_is_atomic_and_undoable() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        let style = scicanvas_core::page::CurveStyle {
            color: Some([19, 45, 80]),
            ..Default::default()
        };
        app.project
            .graph_mut(2)
            .unwrap()
            .first_layer_mut()
            .spec
            .edit_styles(|styles| {
                styles.insert("B".into(), style);
            });
        let book = target(&mut app, "Target");
        let before = app.project.clone();
        let ids = app.create_batch_plots(2, vec![book]).unwrap();
        let result = app.project.graph(ids[0]).unwrap().first_layer();
        let draft = result.draft();
        assert_eq!(result.book_id, Some(book));
        assert_eq!(draft.x_key.as_deref(), Some("B"));
        assert_eq!(draft.y_keys, ["A", "C"]);
        assert_eq!(result.styles()["A"].color, Some([19, 45, 80]));
        let after = app.project.clone();
        app.run_command("edit.undo");
        assert_eq!(app.project, before);
        app.run_command("edit.redo");
        assert_eq!(app.project, after);
        app.run_command("edit.undo");
        let bad = target(&mut app, "Missing");
        app.project.book_mut(bad).unwrap().columns[2].long_name = "different".into();
        let before = app.project.clone();
        assert!(app.create_batch_plots(2, vec![book, bad]).is_none());
        assert_eq!(app.project, before, "a failed target rolls back the entire batch");
        assert!(app.status.contains("缺少列名"));
    }

    #[test]
    fn batch_plot_command_opens_target_selection() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.run_command("graph.template.batch");
        assert!(matches!(app.dialog, Some(Dialog::BatchPlot(_))));
    }
}
