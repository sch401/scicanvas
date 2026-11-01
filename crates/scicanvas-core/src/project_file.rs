//! 项目文件入口：版本迁移、完整性检查和同目录临时文件原子替换。
//!
//! `.scv` 是 ZIP 容器（类似 .docx）：`project.json` 是唯一的权威数据，
//! `tables/*.csv` 和 `graphs/*.svg` 是保存时生成的预览，解压即可查看，打开时忽略。
//! 旧版 `.scproj` 为纯 JSON，继续支持读写。

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::path::Path;

use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::{Book, Graph, GraphTemplate, Project};

mod workbook_template;
pub use workbook_template::*;

pub const CURRENT_VERSION: u32 = crate::model::PROJECT_VERSION;

/// `.scv` 容器版本，与 `project.json` 的项目版本分开演进。
pub const SCV_VERSION: u32 = 1;
pub const SCV_MIMETYPE: &str = "application/vnd.scicanvas.project+zip";
const PROJECT_ENTRY: &str = "project.json";
const MANIFEST_ENTRY: &str = "manifest.json";
/// 解压单个条目的上限，防止损坏或恶意压缩包耗尽内存。
const MAX_ENTRY_BYTES: u64 = 1 << 31;

use crate::page::plots::StoredCurve;

/// 原有文件的显式/继承 X 规则在读取边界归一化，不再展开为平行表。
fn decode_plots(
    layer: &mut Value,
    defaults: &std::collections::BTreeMap<(u64, String), Option<String>>,
) -> Result<(), String> {
    let Some(plots) = layer.get("plots").cloned() else {
        return Ok(());
    };
    let mut plots: Vec<StoredCurve> = serde_json::from_value(plots).map_err(|e| format!("曲线映射无效：{e}"))?;
    let main = layer.get("book_id").and_then(Value::as_u64);
    let mut seen = BTreeSet::new();
    for plot in &mut plots {
        let m = &plot.mapping;
        m.validate()?;
        if plot.base_missing && m.base.is_some() {
            return Err("曲线下端不能同时为缺失和已指定".into());
        }
        if !seen.insert(m.y.clone()) {
            return Err("曲线行范围无效或来源重复".into());
        }
        if let Some(x) = &m.x {
            let default = if Some(m.y.book) == main {
                layer.get("x_key").and_then(Value::as_str).map(str::to_string)
            } else {
                defaults.get(&(m.y.book, m.y.col.clone())).cloned().flatten()
            };
            let inherited = x.book == m.y.book && default.as_deref() == Some(&x.col);
            if !inherited {
                plot.x_override = true;
            }
            // 继承的其他表 X 仍在使用时解析，不固化旧的默认列。
            if inherited && !plot.x_override && Some(m.y.book) != main {
                plot.mapping.x = None;
            }
        }
    }
    layer["plots"] = serde_json::to_value(plots).map_err(|e| e.to_string())?;
    Ok(())
}

/// 无版本号为 v0；显式未知版本拒绝打开，保留源文件。
pub fn migrate(mut value: Value) -> Result<Value, String> {
    let object = value.as_object_mut().ok_or("项目文件必须是 JSON 对象")?;
    let version = match object.get("version") {
        None => 0,
        Some(v) => v.as_u64().ok_or("项目版本号必须是非负整数")?,
    };
    if version > u64::from(CURRENT_VERSION) {
        return Err(format!(
            "项目版本 {version} 高于当前支持的 {CURRENT_VERSION}，请更新 SciCanvas"
        ));
    }
    if version >= 2 {
        if let Some(graphs) = object.get("graphs").and_then(Value::as_array) {
            for graph in graphs {
                if graph
                    .get("page")
                    .and_then(|p| p.get("layers"))
                    .and_then(Value::as_array)
                    .is_none()
                {
                    return Err("图形缺少页面或图层数据".into());
                }
            }
        }
    }
    // v0 → v1：补充格式版本；v1 → v2：单层图形转换为页面和图层。
    if version < 2 {
        if let Some(graphs) = object.get_mut("graphs").and_then(Value::as_array_mut) {
            for graph in graphs {
                let old = graph.as_object_mut().ok_or("图形必须是 JSON 对象")?;
                let id = old.remove("id").ok_or("图形缺少编号")?;
                let name = old.remove("name").ok_or("图形缺少名称")?;
                let layer = Value::Object(std::mem::take(old));
                *graph = serde_json::json!({"id": id, "name": name, "page": {"layers": [layer]}});
            }
        }
    }
    let mut defaults = std::collections::BTreeMap::new();
    if let Some(books) = object.get("books").and_then(Value::as_array) {
        for book in books {
            let (Some(id), Some(columns)) = (
                book.get("id").and_then(Value::as_u64),
                book.get("columns").and_then(Value::as_array),
            ) else {
                continue;
            };
            let first_x = columns
                .iter()
                .find(|c| c.get("role").and_then(Value::as_str) == Some("X"))
                .and_then(|c| c.get("key"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let mut last_x = None;
            for col in columns {
                if let Some(key) = col.get("key").and_then(Value::as_str) {
                    defaults.insert((id, key.to_string()), last_x.clone().or_else(|| first_x.clone()));
                    if col.get("role").and_then(Value::as_str) == Some("X") {
                        last_x = Some(key.to_string());
                    }
                }
            }
        }
    }
    // 默认页宽从 89 mm 改为 A4 宽之前，早期文件可能没有记录页面尺寸：补上当时的默认值，打开后版面不变。
    if let Some(graphs) = object.get_mut("graphs").and_then(Value::as_array_mut) {
        for graph in graphs {
            if let Some(page) = graph.get_mut("page").and_then(Value::as_object_mut) {
                if let Some(layers) = page.get_mut("layers").and_then(Value::as_array_mut) {
                    for layer in layers {
                        decode_plots(layer, &defaults)?;
                        // v0-v11 titles and column names always keep their literal meaning.
                        if version < 12 {
                            layer["rich_text"] = false.into();
                        }
                    }
                }
                let (w, h) = crate::page::LEGACY_PAGE_MM;
                page.entry("width_mm").or_insert(w.into());
                page.entry("height_mm").or_insert(h.into());
            }
        }
    }
    // v8 → v9: infer numeric/text storage while preserving every legacy cell spelling.
    if version < 9 {
        if let Some(books) = object.get_mut("books").and_then(Value::as_array_mut) {
            for book in books {
                if let Some(columns) = book.get_mut("columns").and_then(Value::as_array_mut) {
                    for column in columns {
                        if column.get("data").is_some_and(Value::is_array) {
                            let cells: Vec<String> = serde_json::from_value(column["data"].clone())
                                .map_err(|_| "旧版列数据必须为字符串数组")?;
                            column["data"] =
                                serde_json::to_value(crate::ColumnData::from(cells)).map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }
    }
    object.insert("version".into(), CURRENT_VERSION.into());
    Ok(value)
}

pub fn from_json(text: &str) -> Result<Project, String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| format!("JSON 格式错误：{e}"))?;
    let old_version = value.get("version").and_then(Value::as_u64).unwrap_or(0);
    if old_version < 21 {
        for column in value
            .get_mut("books")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
            .filter_map(|b| b.get_mut("columns").and_then(Value::as_array_mut))
            .flatten()
        {
            if let Some(recipe) = column.pointer_mut("/op/kind/Nonlinear").and_then(Value::as_object_mut) {
                for (old, new) in [("input_hash", "input_digest"), ("attempted_hash", "attempted_digest")] {
                    if let Some(v) = recipe.remove(old) {
                        recipe.insert(new.into(), v);
                    }
                }
            }
        }
    }
    let mut project: Project = serde_json::from_value(migrate(value)?).map_err(|e| format!("项目字段错误：{e}"))?;
    if old_version < 21 {
        migrate_fit_state(&mut project);
    }
    // v21 -> v22: old reports only stored the convergence boolean.
    if old_version < 22 {
        for report in project
            .books
            .iter_mut()
            .flat_map(|b| &mut b.columns)
            .filter_map(|c| c.fit_report.as_deref_mut())
        {
            if !report.fit.converged {
                report.fit.status = crate::nlfit::FitStatus::Failed;
            }
        }
    }
    let max_id = project
        .books
        .iter()
        .map(|b| b.id)
        .chain(project.graphs.iter().map(|g| g.id))
        .chain(project.notes.iter().map(|n| n.id))
        .chain(project.folders.iter().map(|f| f.id))
        .chain(project.fit_functions.iter().map(|f| f.id))
        .chain(
            project
                .books
                .iter()
                .flat_map(|b| &b.columns)
                .filter_map(|c| c.fit_report.as_ref().map(|r| r.id)),
        )
        .max()
        .unwrap_or(0);
    project.next_id = project.next_id.max(max_id.checked_add(1).ok_or("项目对象编号已用尽")?);
    validate(&project)?;
    Ok(project)
}

pub fn to_json(project: &Project) -> Result<String, String> {
    serde_json::to_string_pretty(&project_value(project)?).map_err(|e| e.to_string())
}

fn project_value(project: &Project) -> Result<Value, String> {
    validate(project)?;
    let mut value = serde_json::to_value(project).map_err(|e| e.to_string())?;
    for (graph, wire) in project.graphs.iter().zip(value["graphs"].as_array_mut().unwrap()) {
        for (layer, wire) in graph
            .page
            .layers()
            .iter()
            .zip(wire["page"]["layers"].as_array_mut().unwrap())
        {
            // 保留失去主工作簿的旧空图编辑设置；其余图层用明确的曲线对象写盘。
            if layer.book_id.is_none() {
                continue;
            }
            let plots: Vec<StoredCurve> = layer
                .plots(&|id| project.book(id))
                .into_iter()
                .map(|plot| StoredCurve {
                    base_missing: layer.plot_base_key(&plot.key).as_deref() == Some(""),
                    mapping: plot.mapping,
                    style: plot.style,
                    x_override: plot.x_override,
                })
                .collect();
            wire["plots"] = serde_json::to_value(plots).map_err(|e| e.to_string())?;
        }
    }
    value["version"] = CURRENT_VERSION.into();
    Ok(value)
}

fn validate(project: &Project) -> Result<(), String> {
    if project.version > CURRENT_VERSION {
        return Err("项目来自更新的文件格式，请更新 SciCanvas 后保存".into());
    }
    if project.next_id == u64::MAX {
        return Err("项目对象编号已用尽".into());
    }
    let mut ids = BTreeSet::new();
    for id in project
        .books
        .iter()
        .map(|b| b.id)
        .chain(project.graphs.iter().map(|g| g.id))
        .chain(project.notes.iter().map(|n| n.id))
        .chain(project.folders.iter().map(|f| f.id))
        .chain(project.fit_functions.iter().map(|f| f.id))
        .chain(
            project
                .books
                .iter()
                .flat_map(|b| &b.columns)
                .filter_map(|c| c.fit_report.as_ref().map(|r| r.id)),
        )
    {
        if id == 0 || id == u64::MAX || !ids.insert(id) {
            return Err(format!("对象编号 {id} 无效或重复"));
        }
    }
    let mut function_names = BTreeSet::new();
    for saved in &project.fit_functions {
        saved.definition.resolve()?;
        if !function_names.insert(&saved.definition.name) {
            return Err("项目函数库中有重复名称".into());
        }
    }
    project.validate_folders()?;
    if project.notes.iter().any(|n| n.name.trim().is_empty()) {
        return Err("Note 名称不能为空".into());
    }
    for book in &project.books {
        if let Some(filter) = &book.filter { filter.validate(book)?; }
        let mut keys = BTreeSet::new();
        let mut fit_outputs = BTreeSet::new();
        for col in &book.columns {
            col.data.validate()?;
            if let Some(report) = &col.fit_report {
                report.validate()?;
                if report.book_id != book.id || report.curve.1 != col.key {
                    return Err(format!(
                        "工作簿 {} 列 {}：拟合报告与结果列关联不一致",
                        book.name, col.key
                    ));
                }
                if let Some(crate::ops::Operation {
                    kind: crate::ops::OpKind::Nonlinear(recipe),
                    ..
                }) = &col.op
                {
                    if recipe.output_x != report.curve.0
                        || recipe.function != crate::ops::FitMethod::from_function(&report.fit.model)
                    {
                        return Err(format!("工作簿 {} 列 {}：拟合报告与操作定义不一致", book.name, col.key));
                    }
                }
            }
            if let Some(op) = &col.op {
                if let crate::ops::OpKind::Nonlinear(recipe) = &op.kind {
                    recipe.validate()?;
                    if col.formula.is_some() {
                        return Err(format!("工作簿 {} 列 {}：拟合与列公式同时占用结果", book.name, col.key));
                    }
                    let mut outputs = BTreeSet::from([col.key.as_str()]);
                    for key in std::iter::once(&recipe.output_x).chain(recipe.bands.iter().flat_map(|b| &b.columns)) {
                        if !outputs.insert(key.as_str())
                            || book.column(key).is_none()
                            || book.column(key).is_some_and(|c| c.formula.is_some() || c.op.is_some())
                        {
                            return Err(format!(
                                "工作簿 {} 列 {}：非线性输出列 {key} 缺失、重复或有其他计算定义",
                                book.name, col.key
                            ));
                        }
                    }
                    if outputs.iter().any(|key| !fit_outputs.insert(*key)) {
                        return Err(format!(
                            "工作簿 {} 列 {}：多次非线性拟合重复占用同一输出列",
                            book.name, col.key
                        ));
                    }
                    if outputs.contains(op.y.as_str())
                        || op.x.as_ref().is_some_and(|x| outputs.contains(x.as_str()))
                        || book.column(&op.y).is_none()
                        || op.x.as_ref().is_some_and(|x| book.column(x).is_none())
                    {
                        return Err(format!(
                            "工作簿 {} 列 {}：非线性重算来源缺失或引用自身输出",
                            book.name, col.key
                        ));
                    }
                }
            }
            if col.key.is_empty() || !keys.insert(&col.key) {
                return Err(format!("工作簿 {} 的列标识为空或重复", book.name));
            }
        }
    }
    for graph in &project.graphs {
        graph.page.validate()?;
        for layer in graph.page.layers() {
            for plot in layer.plots(&|id| project.book(id)) {
                plot.mapping.validate()?;
            }
            if [
                layer.x_min,
                layer.x_max,
                layer.y_min,
                layer.y_max,
                layer.y2_min,
                layer.y2_max,
                f64::from(layer.line_width),
                f64::from(layer.symbol_size),
                f64::from(layer.error_bars.cap),
                f64::from(layer.error_bars.width),
            ]
            .iter()
            .any(|x| !x.is_finite())
            {
                return Err(format!("图形 {} 含无效的数值属性", graph.name));
            }
            if layer.error_bars.cap < 0.0 || layer.error_bars.width <= 0.0 {
                return Err("误差棒端帽长度应非负，线宽应为正数".into());
            }
            if !layer.area_opacity.is_finite() || !(0.0..=1.0).contains(&layer.area_opacity) {
                return Err("面积填充透明度应在 0 与 1 之间".into());
            }
            if !layer.labels.size.is_finite()
                || layer.labels.size <= 0.0
                || !layer.labels.offset.is_finite()
                || layer.labels.offset < 0.0
            {
                return Err("数据标签字号应为有限正数，偏移应为有限非负数".into());
            }
            if !layer.valid_plot_rows() {
                return Err("图层行范围无效".into());
            }
        }
    }
    Ok(())
}

/// 工作簿导出为 CSV：UTF-8 BOM（Excel 直接识别中文），表头为长名称（无则用列标识），
/// 单位写成 `名称 (单位)`，与导入时的拆分规则一致。
pub fn book_csv(book: &Book) -> String {
    fn field(out: &mut String, text: &str) {
        if text.contains([',', '"', '\r', '\n']) || text.trim() != text {
            out.push('"');
            out.push_str(&text.replace('"', "\"\""));
            out.push('"');
        } else {
            out.push_str(text);
        }
    }
    let mut out = String::from('\u{feff}');
    for (i, col) in book.columns.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let name = if col.long_name.is_empty() {
            &col.key
        } else {
            &col.long_name
        };
        if col.units.is_empty() {
            field(&mut out, name);
        } else {
            field(&mut out, &format!("{name} ({})", col.units));
        }
    }
    out.push_str("\r\n");
    for row in 0..book.nrows() {
        for (i, col) in book.columns.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            field(&mut out, &col.data.text_at(row));
        }
        out.push_str("\r\n");
    }
    out
}

/// 压缩包内的文件名：编号前缀保证唯一，去掉 Windows 不允许的字符。
fn entry_name(dir: &str, id: u64, name: &str, ext: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(80)
        .collect();
    let safe = safe.trim().trim_end_matches('.');
    format!("{dir}/{id}-{}.{ext}", if safe.is_empty() { "未命名" } else { safe })
}

pub fn to_scv(project: &Project) -> Result<Vec<u8>, String> {
    let json = to_json(project)?;
    let folder = |id: u64| {
        project
            .folder_of(id)
            .and_then(|f| project.folder_path(f))
            .unwrap_or_default()
    };
    let tables: Vec<_> = project
        .books
        .iter()
        .map(|b| (entry_name("tables", b.id, &b.name, "csv"), b))
        .collect();
    let graphs: Vec<_> = project
        .graphs
        .iter()
        .map(|g| (entry_name("graphs", g.id, &g.name, "svg"), g))
        .collect();
    let manifest = serde_json::json!({
        "format": "SciCanvas Project",
        "scv_version": SCV_VERSION,
        "project_version": CURRENT_VERSION,
        "generator": concat!("SciCanvas ", env!("CARGO_PKG_VERSION")),
        "project": PROJECT_ENTRY,
        "tables": tables.iter().map(|(path, b)| serde_json::json!({
            "id": b.id, "name": b.name, "folder": folder(b.id), "path": path,
            "rows": b.nrows(), "columns": b.columns.len(),
        })).collect::<Vec<_>>(),
        "graphs": graphs.iter().map(|(path, g)| serde_json::json!({
            "id": g.id, "name": g.name, "folder": folder(g.id), "path": path,
        })).collect::<Vec<_>>(),
        "notes": project.notes.iter().map(|n| serde_json::json!({
            "id": n.id, "name": n.name, "folder": folder(n.id),
            "path": entry_name("notes", n.id, &n.name, "txt"),
        })).collect::<Vec<_>>(),
    });
    let manifest = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;

    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut add = |name: &str, method: CompressionMethod, bytes: &[u8]| -> zip::result::ZipResult<()> {
        let options = SimpleFileOptions::default()
            .compression_method(method)
            .large_file(bytes.len() as u64 >= u64::from(u32::MAX));
        zip.start_file(name, options)?;
        zip.write_all(bytes)?;
        Ok(())
    };
    let result = (|| {
        // 与 ODF/EPUB 相同：第一个条目不压缩，便于按文件头识别类型。
        add("mimetype", CompressionMethod::Stored, SCV_MIMETYPE.as_bytes())?;
        add(MANIFEST_ENTRY, CompressionMethod::Deflated, manifest.as_bytes())?;
        add(PROJECT_ENTRY, CompressionMethod::Deflated, json.as_bytes())?;
        for (path, book) in &tables {
            add(path, CompressionMethod::Deflated, book_csv(book).as_bytes())?;
        }
        for note in &project.notes {
            add(
                &entry_name("notes", note.id, &note.name, "txt"),
                CompressionMethod::Deflated,
                note.text.as_bytes(),
            )?;
        }
        for (path, graph) in &graphs {
            add(
                path,
                CompressionMethod::Deflated,
                crate::svg::render_page(graph, project).as_bytes(),
            )?;
        }
        Ok(())
    })();
    result
        .and_then(|()| zip.finish())
        .map(Cursor::into_inner)
        .map_err(|e| format!("生成 .scv 失败：{e}"))
}

const MAX_GRAPH_TEMPLATE_BYTES: usize = 16 * 1024 * 1024;

fn template_column(key: &str) -> String {
    crate::plotspec::split_key(key).0.to_owned()
}

fn template_key_map<T: PartialEq>(
    entries: impl IntoIterator<Item = (String, T)>,
) -> Result<BTreeMap<String, T>, String> {
    let mut out = BTreeMap::new();
    for (key, value) in entries {
        let key = template_column(&key);
        if out.get(&key).is_some_and(|existing| existing != &value) {
            return Err(format!("模板中同名列有冲突的映射：{key}"));
        }
        out.entry(key).or_insert(value);
    }
    Ok(out)
}

/// 提取页面样式与列标识；不把源工作簿编号或数据复制进模板。
pub fn graph_template_from_graph(graph: &Graph, project: &Project) -> Result<GraphTemplate, String> {
    let mut curve_count = 0;
    for layer in graph.page.layers() {
        for book_id in layer.book_id.into_iter().chain(layer.foreign_books()).chain(
            layer
                .styles()
                .keys()
                .filter_map(|key| crate::plotspec::split_key(key).1),
        ) {
            if project.book(book_id).is_none() {
                return Err("图形引用的工作簿已不存在，无法保存模板".into());
            }
        }
        let spec = layer.draft();
        if !spec.y_keys.is_empty() {
            let book_id = layer.book_id.ok_or("图形曲线没有绑定源工作簿")?;
            let book = project.book(book_id).ok_or("找不到图形源工作簿")?;
            layer.validate_in(book, &|id| project.book(id))?;
            curve_count += spec.y_keys.len();
        }
    }
    if curve_count == 0 {
        return Err("图形没有可保存的曲线".into());
    }

    let mut page = graph.page.clone();
    for layer in page.layers_mut() {
        let mut draft = layer.draft();
        draft.book_id = None;
        draft.x_key = draft.x_key.as_deref().map(template_column);
        let mut seen = BTreeSet::new();
        draft.y_keys = draft
            .y_keys
            .iter()
            .map(|key| template_column(key))
            .map(|key| {
                if seen.insert(key.clone()) {
                    Ok(key)
                } else {
                    Err(format!("同一图层有重复列标识 {key}，无法保存为模板"))
                }
            })
            .collect::<Result<_, _>>()?;
        draft.x_by_y = template_key_map(
            draft
                .x_by_y
                .into_iter()
                .map(|(key, value)| (key, template_column(&value))),
        )?;
        draft.base_by_y = template_key_map(
            draft
                .base_by_y
                .into_iter()
                .map(|(key, value)| (key, template_column(&value))),
        )?;
        draft.row_by_y = template_key_map(draft.row_by_y)?;
        let mut seen_lines = BTreeSet::new();
        draft.line_only = draft
            .line_only
            .iter()
            .map(|key| template_column(key))
            .filter(|key| seen_lines.insert(key.clone()))
            .collect();

        let styles = template_key_map(layer.styles())?;
        layer.spec = crate::page::LayerSpec::from(draft);
        layer.spec.edit_styles(|stored| *stored = styles);
        layer.book_id = None;
    }
    page.validate()?;
    Ok(GraphTemplate {
        version: crate::model::GRAPH_TEMPLATE_VERSION,
        name: graph.name.clone(),
        page,
    })
}

fn validate_graph_template(template: &GraphTemplate) -> Result<(), String> {
    if template.version != crate::model::GRAPH_TEMPLATE_VERSION {
        return Err(format!("不支持的图形模板版本 {}", template.version));
    }
    if template.name.trim().is_empty() {
        return Err("图形模板名称不能为空".into());
    }
    template.page.validate()?;
    let mut curves = 0;
    for layer in template.page.layers() {
        if layer.book_id.is_some()
            || !layer.foreign_books().is_empty()
            || layer
                .styles()
                .keys()
                .any(|key| crate::plotspec::split_key(key).1.is_some())
        {
            return Err("图形模板不能包含源工作簿编号".into());
        }
        curves += layer.draft().y_keys.len();
    }
    if curves == 0 {
        return Err("图形模板没有可应用的曲线".into());
    }
    Ok(())
}

pub fn graph_template_to_json(template: &GraphTemplate) -> Result<String, String> {
    validate_graph_template(template)?;
    serde_json::to_string_pretty(template).map_err(|e| format!("序列化图形模板失败：{e}"))
}

pub fn graph_template_from_json(text: &str) -> Result<GraphTemplate, String> {
    if text.len() > MAX_GRAPH_TEMPLATE_BYTES {
        return Err("图形模板文件过大".into());
    }
    let template: GraphTemplate =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("图形模板格式错误：{e}"))?;
    validate_graph_template(&template)?;
    Ok(template)
}

#[cfg(feature = "xlsx")]
pub fn save_xlsx_export(path: &Path, project: &Project, headers: bool) -> Result<(), String> {
    let data = crate::xlsx_export::write_project(project, headers)?;
    write_atomic(path, &data, "文件名为空", "导出 Excel 失败：")
}

pub fn save_graph_template(path: &Path, template: &GraphTemplate) -> Result<(), String> {
    let content = graph_template_to_json(template)?.into_bytes();
    write_atomic(path, &content, "模板文件名为空", "保存图形模板失败：")
}

pub fn load_graph_template(path: &Path) -> Result<GraphTemplate, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_GRAPH_TEMPLATE_BYTES as u64 {
        return Err("图形模板文件过大".into());
    }
    let mut content = Vec::new();
    file.take(MAX_GRAPH_TEMPLATE_BYTES as u64 + 1)
        .read_to_end(&mut content)
        .map_err(|e| e.to_string())?;
    if content.len() > MAX_GRAPH_TEMPLATE_BYTES {
        return Err("图形模板文件过大".into());
    }
    let text = String::from_utf8(content).map_err(|_| "图形模板不是 UTF-8 JSON")?;
    graph_template_from_json(&text)
}

const MAX_FIT_FUNCTION_BYTES: usize = 64 * 1024;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FitFunctionFile {
    version: u32,
    function: crate::nlfit::CustomFitDefinition,
}

pub fn fit_function_to_json(function: &crate::nlfit::CustomFitDefinition) -> Result<String, String> {
    function.resolve()?;
    let content = serde_json::to_string_pretty(&FitFunctionFile {
        version: 1,
        function: function.clone(),
    })
    .map_err(|error| format!("序列化拟合函数失败：{error}"))?;
    if content.len() > MAX_FIT_FUNCTION_BYTES {
        return Err("拟合函数文件过大".into());
    }
    Ok(content)
}

pub fn fit_function_from_json(text: &str) -> Result<crate::nlfit::CustomFitDefinition, String> {
    if text.len() > MAX_FIT_FUNCTION_BYTES {
        return Err("拟合函数文件过大".into());
    }
    let file: FitFunctionFile = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("拟合函数文件格式错误：{error}"))?;
    if file.version != 1 {
        return Err(format!("不支持的拟合函数文件版本 {}", file.version));
    }
    file.function.resolve()?;
    Ok(file.function)
}

pub fn save_fit_function(path: &Path, function: &crate::nlfit::CustomFitDefinition) -> Result<(), String> {
    let content = fit_function_to_json(function)?;
    write_atomic(path, content.as_bytes(), "函数文件名为空", "保存拟合函数失败：")
}

pub fn load_fit_function(path: &Path) -> Result<crate::nlfit::CustomFitDefinition, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > MAX_FIT_FUNCTION_BYTES as u64 {
        return Err("拟合函数文件过大".into());
    }
    let mut content = String::new();
    file.take(MAX_FIT_FUNCTION_BYTES as u64 + 1)
        .read_to_string(&mut content)
        .map_err(|error| format!("读取拟合函数失败：{error}"))?;
    fit_function_from_json(&content)
}

fn scv_text(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Option<String>, String> {
    let entry = match zip.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(format!("不是有效的 .scv 文件：{e}")),
    };
    let too_large = || format!("{name} 过大，无法打开");
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(too_large());
    }
    let mut data = Vec::new();
    entry
        .take(MAX_ENTRY_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|e| format!("读取 {name} 失败：{e}"))?;
    if data.len() as u64 > MAX_ENTRY_BYTES {
        return Err(too_large());
    }
    String::from_utf8(data)
        .map(Some)
        .map_err(|_| format!("{name} 不是 UTF-8 文本"))
}

/// 只读取 `project.json`；`tables/`、`graphs/` 下的预览文件即使被修改或删除也不影响打开。
pub fn from_scv(bytes: &[u8]) -> Result<Project, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("不是有效的 .scv 文件：{e}"))?;
    if let Some(mimetype) = scv_text(&mut zip, "mimetype")? {
        if mimetype.trim() != SCV_MIMETYPE {
            return Err("压缩包不是 SciCanvas 项目".into());
        }
    }
    if let Some(manifest) = scv_text(&mut zip, MANIFEST_ENTRY)? {
        let manifest: Value = serde_json::from_str(&manifest).map_err(|e| format!("manifest.json 格式错误：{e}"))?;
        let version = manifest
            .get("scv_version")
            .and_then(Value::as_u64)
            .ok_or("manifest.json 缺少 scv_version")?;
        if version > u64::from(SCV_VERSION) {
            return Err(format!(
                ".scv 容器版本 {version} 高于当前支持的 {SCV_VERSION}，请更新 SciCanvas"
            ));
        }
    }
    let json = scv_text(&mut zip, PROJECT_ENTRY)?.ok_or("压缩包缺少 project.json")?;
    from_json(&json)
}

/// 按内容识别格式：ZIP 文件头为 `.scv`，否则按旧版 JSON 读取。
pub fn from_bytes(bytes: &[u8]) -> Result<Project, String> {
    if bytes.starts_with(b"PK") {
        return from_scv(bytes);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "项目文件既不是 .scv 压缩包，也不是 UTF-8 JSON")?;
    from_json(text.trim_start_matches('\u{feff}'))
}

pub fn load(path: &Path) -> Result<Project, String> {
    from_bytes(&std::fs::read(path).map_err(|e| e.to_string())?)
}

/// 扩展名为 `.scproj` 时保存旧版 JSON，其余（含 `.scv`）保存为压缩包。
pub fn is_legacy_path(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("scproj"))
}

pub fn save(path: &Path, project: &Project) -> Result<(), String> {
    let content = if is_legacy_path(path) {
        to_json(project)?.into_bytes()
    } else {
        to_scv(project)?
    };
    write_atomic(path, &content, "项目文件名为空", "保存项目失败：")
}

fn write_atomic(path: &Path, content: &[u8], missing_name: &str, failure_prefix: &str) -> Result<(), String> {
    write_atomic_with(path, missing_name, failure_prefix, |file| {
        file.write_all(content).map_err(|e| e.to_string())
    })
}
fn write_atomic_with(
    path: &Path,
    missing_name: &str,
    failure_prefix: &str,
    write: impl FnOnce(&mut std::fs::File) -> Result<(), String>,
) -> Result<(), String> {
    let name = path.file_name().ok_or(missing_name)?.to_string_lossy();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = path.with_file_name(format!(".{name}.{}.{stamp}.tmp", std::process::id()));
    let mut file = std::fs::File::create_new(&temporary).map_err(|e| e.to_string())?;
    let result = write(&mut file).and_then(|_| file.sync_all().map_err(|e| e.to_string()));
    drop(file);
    let result = result.and_then(|_| std::fs::rename(&temporary, path).map_err(|e| e.to_string()));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("{failure_prefix}{e}"))
}

fn migrate_fit_state(project: &mut Project) {
    use crate::ops::{OpKind, Operation};
    for book in &mut project.books {
        let updates: Vec<_> = book
            .columns
            .iter()
            .filter_map(|col| {
                let op = col.op.as_ref()?;
                let OpKind::Nonlinear(r) = &op.kind else { return None };
                let legacy = r.legacy_fingerprint(op, book);
                let digest = r.fingerprint(op, book);
                Some((
                    col.key.clone(),
                    r.input_digest.filter(|v| *v == legacy || *v == digest).map(|_| digest),
                    r.attempted_digest
                        .filter(|v| *v == legacy || *v == digest)
                        .map(|_| digest),
                ))
            })
            .collect();
        for (key, input, attempted) in updates {
            let col = book.columns.iter_mut().find(|c| c.key == key).unwrap();
            if let Some(Operation {
                kind: OpKind::Nonlinear(r),
                ..
            }) = &mut col.op
            {
                r.input_digest = input;
                r.attempted_digest = attempted;
            }
            if input.is_none() {
                if let Some(report) = &mut col.fit_report {
                    report.current = false;
                }
            }
        }
    }
    // 旧文件中已有明确的拟合操作或独立结果 X 时恢复行域。
    let grids: Vec<_> = project
        .graphs
        .iter()
        .flat_map(|g| g.page.layers())
        .flat_map(|l| l.plots(&|id| project.book(id)))
        .filter_map(|p| {
            let x = p.mapping.x?;
            let col = project.book(x.book)?.column(&x.col)?;
            col.derived_from.as_ref()?;
            Some((x, p.mapping.y))
        })
        .collect();
    for (x, y) in grids {
        for reference in [x, y] {
            if let Some(book) = project.book_mut(reference.book) {
                if let Some(c) = book.columns.iter_mut().find(|c| c.key == reference.col) {
                    c.independent_rows = true;
                }
            }
        }
    }
    for book in &mut project.books {
        let keys: Vec<String> = book
            .columns
            .iter()
            .flat_map(|col| match &col.op {
                Some(Operation {
                    kind: OpKind::Nonlinear(r),
                    ..
                }) => std::iter::once(col.key.clone())
                    .chain(std::iter::once(r.output_x.clone()))
                    .chain(r.bands.iter().flat_map(|b| b.columns.clone()))
                    .collect(),
                Some(Operation {
                    kind: OpKind::PolyFitX { .. } | OpKind::PolyFitY { .. },
                    ..
                }) => vec![col.key.clone()],
                _ => vec![],
            })
            .collect();
        for c in &mut book.columns {
            if keys.contains(&c.key) {
                c.independent_rows = true;
            }
        }
    }
}

/// 桌面偏好与恢复文件。恢复使用项目 JSON，避免生成图形预览。
pub mod session {
    use super::*;
    #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    pub struct Settings {
        pub window: Option<[f32; 4]>,
        pub maximized: bool,
        pub recent: Vec<std::path::PathBuf>,
        pub last_dir: Option<std::path::PathBuf>,
        pub theme: String,
    }
    impl Settings {
        pub fn remember(&mut self, path: &Path) {
            let path = normalize_path(path);
            self.last_dir = path.parent().map(Path::to_path_buf);
            self.recent.retain(|p| !same_path(p, &path));
            self.recent.insert(0, path);
            self.recent.truncate(10);
        }
    }
    pub fn settings_path() -> Option<std::path::PathBuf> {
        std::env::var_os("APPDATA").map(|p| std::path::PathBuf::from(p).join("SciCanvas/settings.json"))
    }
    pub fn load_settings() -> Settings {
        let mut settings: Settings = settings_path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default();
        settings.normalize();
        settings
    }
    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        let path = settings_path().ok_or("没有可用的偏好保存目录")?;
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        write_atomic(
            &path,
            &serde_json::to_vec(settings).map_err(|e| e.to_string())?,
            "偏好文件名为空",
            "保存偏好失败：",
        )
    }
    pub fn recovery_dir() -> Option<std::path::PathBuf> {
        std::env::var_os("LOCALAPPDATA").map(|p| std::path::PathBuf::from(p).join("SciCanvas/recovery"))
    }
    pub fn lock_recovery(path: &Path) -> std::io::Result<std::fs::File> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        options.open(path.with_extension("lock"))
    }
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    #[serde(default)]
    struct RecoveryMeta {
        source: Option<std::path::PathBuf>,
        retired: bool,
    }
    pub const RECOVERY_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(30 * 24 * 60 * 60);
    fn recovery_meta(path: &Path) -> RecoveryMeta {
        std::fs::read(path.with_extension("source"))
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default()
    }
    fn write_meta(path: &Path, meta: &RecoveryMeta) -> Result<(), String> {
        write_atomic(
            &path.with_extension("source"),
            &serde_json::to_vec(meta).map_err(|e| e.to_string())?,
            "恢复信息文件名为空",
            "保存恢复信息失败：",
        )
    }
    pub fn retire_recovery(path: &Path) -> Result<(), String> {
        let meta = RecoveryMeta {
            source: recovery_source(path),
            retired: true,
        };
        write_meta(path, &meta)
    }
    fn remove_locked(path: &Path, lock: std::fs::File) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
        drop(lock);
        let _ = std::fs::remove_file(path.with_extension("source"));
        let _ = std::fs::remove_file(path.with_extension("lock"));
        Ok(())
    }
    pub fn discard_recovery(path: &Path) -> Result<(), String> {
        let lock = lock_recovery(path).map_err(|e| format!("恢复文件正在其他会话使用：{e}"))?;
        retire_recovery(path)?;
        remove_locked(path, lock)
    }
    pub fn save_recovery(path: &Path, project: &Project, source: Option<&Path>) -> Result<(), String> {
        let value = project_value(project)?;
        write_meta(
            path,
            &RecoveryMeta {
                source: source.map(normalize_path),
                retired: false,
            },
        )?;
        write_atomic_with(path, "恢复文件名为空", "自动备份失败：", |file| {
            let mut writer = std::io::BufWriter::with_capacity(64 * 1024, file);
            serde_json::to_writer(&mut writer, &value).map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())
        })
    }
    pub fn recovery_source(path: &Path) -> Option<std::path::PathBuf> {
        if let Ok(bytes) = std::fs::read(path.with_extension("source")) {
            if let Ok(meta) = serde_json::from_slice::<RecoveryMeta>(&bytes) {
                return meta.source.map(|p| normalize_path(&p));
            }
        }
        #[derive(serde::Deserialize)]
        struct Legacy {
            recovery_source: Option<std::path::PathBuf>,
        }
        let file = std::fs::File::open(path).ok()?;
        serde_json::from_reader::<_, Legacy>(std::io::BufReader::new(file))
            .ok()?
            .recovery_source
            .map(|p| normalize_path(&p))
    }
    pub fn recovery_notice(path: &Path) -> Option<&'static str> {
        let backup = path.metadata().and_then(|m| m.modified()).ok()?;
        if backup.elapsed().ok().is_some_and(|age| age > RECOVERY_MAX_AGE) {
            return Some("这份备份已超过 30 天，恢复或确认放弃后清理");
        }
        if recovery_source(path)
            .and_then(|p| p.metadata().ok())
            .and_then(|m| m.modified().ok())
            .is_some_and(|saved| saved >= backup)
        {
            return Some("源文件在备份后有更新，请选择需要保留的版本");
        }
        None
    }
    pub fn recoveries(directory: &Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "source" || e == "lock") {
                let project = path.with_extension("scproj");
                if !project.exists() {
                    // 活跃备份先写 source 再写项目；两种附属文件都先取得会话锁。
                    if let Ok(lock) = lock_recovery(&project) {
                        if !project.exists() {
                            let _ = std::fs::remove_file(project.with_extension("source"));
                            drop(lock);
                            let _ = std::fs::remove_file(project.with_extension("lock"));
                        }
                    }
                }
                continue;
            }
            if !path.extension().is_some_and(|e| e == "scproj") {
                continue;
            }
            let Ok(lock) = lock_recovery(&path) else { continue };
            // 只自动清理已保存/明确放弃的会话，时间较旧的唯一未保存副本仍可恢复。
            if recovery_meta(&path).retired {
                let _ = remove_locked(&path, lock);
            } else {
                drop(lock);
                files.push(path);
            }
        }
        files.sort_by_key(|p| std::cmp::Reverse(p.metadata().and_then(|m| m.modified()).ok()));
        files
    }
    pub fn normalize_path(path: &Path) -> std::path::PathBuf {
        let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        #[cfg(windows)]
        if let Some(rest) = path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
            let normalized = if let Some(unc) = rest.strip_prefix(r"UNC\") {
                Some(format!(r"\\{unc}"))
            } else if rest.as_bytes().get(1) == Some(&b':') {
                Some(rest.to_owned())
            } else {
                None
            };
            if let Some(normalized) = normalized.filter(|p| p.encode_utf16().count() <= 259) {
                return normalized.into();
            }
        }
        path
    }
    fn same_path(a: &Path, b: &Path) -> bool {
        #[cfg(windows)]
        {
            a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
        }
        #[cfg(not(windows))]
        {
            a == b
        }
    }
    impl Settings {
        pub fn normalize(&mut self) {
            let mut recent: Vec<std::path::PathBuf> = Vec::new();
            for path in std::mem::take(&mut self.recent) {
                let path = normalize_path(&path);
                if !recent.iter().any(|p| same_path(p, &path)) {
                    recent.push(path);
                }
                if recent.len() == 10 {
                    break;
                }
            }
            self.recent = recent;
            self.last_dir = self.last_dir.as_deref().map(normalize_path);
        }
    }
    /// 只保留最近一次启动环境，不向异常日志追加正常启动记录。
    pub fn write_startup_log(message: &str) {
        if let Some(directory) = recovery_dir() {
            write_startup_log_in(&directory, message);
        }
    }
    fn write_startup_log_in(directory: &Path, message: &str) {
        let _ = std::fs::create_dir_all(directory);
        let _ = std::fs::write(directory.join("startup.log"), message);
    }
    pub fn append_log(message: &str) {
        if let Some(directory) = recovery_dir() {
            let _ = std::fs::create_dir_all(&directory);
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join("crash.log"))
            {
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let _ = writeln!(file, "{stamp} PID {}：{message}", std::process::id());
            }
        }
    }
    pub fn install_panic_log() {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            append_log(&info.to_string());
            previous(info);
        }));
    }
    #[cfg(test)]
    mod startup_tests {
        use super::*;
        #[test]
        fn startup_log_overwrites_and_survives_recovery_scanning() {
            let dir = std::env::temp_dir().join(format!(
                "scicanvas-startup-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            write_startup_log_in(&dir, "first");
            write_startup_log_in(&dir, "second");
            assert!(recoveries(&dir).is_empty());
            assert_eq!(std::fs::read_to_string(dir.join("startup.log")).unwrap(), "second");
            std::fs::remove_file(dir.join("startup.log")).unwrap();
            std::fs::remove_dir(dir).unwrap();
        }
    }
}

#[cfg(test)]
mod review_session_tests {
    use super::*;
    #[test]
    fn recovery_is_a_readable_project_and_settings_bound_recent_files() {
        let directory = std::env::temp_dir().join(format!(
            "scicanvas-recovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("session.scproj");
        let p = Project::sample();
        session::save_recovery(&path, &p, None).unwrap();
        assert_eq!(load(&path).unwrap(), p);
        let mut settings = session::Settings::default();
        for i in 0..12 {
            settings.remember(&directory.join(format!("{i}.scv")));
        }
        assert_eq!(settings.recent.len(), 10);
        let first = settings.recent[3].clone();
        settings.remember(&first);
        assert_eq!(settings.recent[0], first);
        assert_eq!(settings.recent.len(), 10);
        let lock = session::lock_recovery(&path).unwrap();
        #[cfg(windows)]
        assert!(session::recoveries(&directory).is_empty());
        #[cfg(windows)]
        {
            assert!(session::discard_recovery(&path).is_err());
            assert!(path.exists());
        }
        drop(lock);
        assert_eq!(session::recoveries(&directory), vec![path.clone()]);
        session::discard_recovery(&path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}

#[cfg(test)]
mod review_legacy_versions {
    use super::*;
    #[test]
    fn all_versions_zero_through_twenty_one_load_into_current() {
        for version in 0..=21 {
            let mut p = Project::default();
            p.new_book();
            p.books[0].columns[0].data = vec!["0".into(), "1".into()].into();
            p.books[0].columns[1].data = vec!["2".into(), "3".into()].into();
            let mut value = serde_json::to_value(&p).unwrap();
            value["version"] = version.into();
            if version < 9 {
                for (index, col) in value["books"][0]["columns"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .enumerate()
                {
                    col["data"] = serde_json::to_value(
                        p.books[0].columns[index]
                            .data
                            .iter()
                            .map(|s| s.into_owned())
                            .collect::<Vec<_>>(),
                    )
                    .unwrap();
                }
            }
            let loaded = from_json(&value.to_string()).unwrap();
            assert_eq!(loaded.version, CURRENT_VERSION);
            assert_eq!(loaded.books, p.books, "v{version}");
            assert_eq!(from_json(&to_json(&loaded).unwrap()).unwrap(), loaded);
        }
    }
}

#[cfg(test)]
mod round_two_tests {
    use super::*;
    #[test]
    #[cfg(windows)]
    fn paths_normalize_old_prefixes_and_deduplicate() {
        use session::*;
        assert_eq!(
            normalize_path(Path::new(r"\\?\D:\a\b.scv")),
            std::path::PathBuf::from(r"D:\a\b.scv")
        );
        assert_eq!(
            normalize_path(Path::new(r"\\?\UNC\server\share\x.scv")),
            std::path::PathBuf::from(r"\\server\share\x.scv")
        );
        for prefix in [r"\\?\D:\", r"\\?\UNC\server\share\"] {
            let long = format!("{prefix}{}.scv", "字".repeat(260));
            assert_eq!(normalize_path(Path::new(&long)), std::path::PathBuf::from(&long));
        }
        let mut settings = Settings {
            recent: vec![r"\\?\D:\a\b.scv".into(), r"d:\A\B.SCV".into()],
            last_dir: Some(r"\\?\D:\a".into()),
            ..Default::default()
        };
        settings.normalize();
        assert_eq!(settings.recent.len(), 1);
        assert_eq!(settings.last_dir, Some(r"D:\a".into()));
        settings.remember(Path::new(r"d:\A\B.SCV"));
        assert_eq!(settings.recent.len(), 1);
    }
    #[test]
    fn recovery_cleanup_respects_ownership_and_preserves_unique_old_work() {
        use session::*;
        let dir = std::env::temp_dir().join(format!(
            "scicanvas-round2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let p = Project::sample();
        let source = dir.join("saved.scv");
        save(&source, &p).unwrap();
        let backup = dir.join("active.scproj");
        save_recovery(&backup, &p, Some(&source)).unwrap();
        assert_eq!(load(&backup).unwrap(), p);
        assert_eq!(recovery_source(&backup), Some(source.clone()));
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(31 * 24 * 3600);
        std::fs::File::options()
            .write(true)
            .open(&backup)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(old))
            .unwrap();
        assert!(recovery_notice(&backup).unwrap().contains("30"));
        assert!(recoveries(&dir).contains(&backup));
        let retired = dir.join("retired.scproj");
        save_recovery(&retired, &p, None).unwrap();
        retire_recovery(&retired).unwrap();
        let lock = lock_recovery(&retired).unwrap();
        #[cfg(windows)]
        {
            assert!(!recoveries(&dir).contains(&retired));
            assert!(retired.exists());
        }
        drop(lock);
        assert!(!recoveries(&dir).contains(&retired));
        assert!(!retired.exists());
        assert!(!retired.with_extension("source").exists());
        for name in ["orphan-source", "orphan-lock"] {
            let orphan = dir.join(format!("{name}.scproj"));
            std::fs::write(orphan.with_extension("source"), b"{}").unwrap();
            drop(lock_recovery(&orphan).unwrap());
            assert!(recoveries(&dir).contains(&backup));
            assert!(!orphan.with_extension("source").exists());
            assert!(!orphan.with_extension("lock").exists());
        }
        let writing = dir.join("writing.scproj");
        let writer = lock_recovery(&writing).unwrap();
        std::fs::write(writing.with_extension("source"), b"{}").unwrap();
        #[cfg(windows)]
        {
            recoveries(&dir);
            assert!(writing.with_extension("source").exists());
            assert!(writing.with_extension("lock").exists());
        }
        drop(writer);
        recoveries(&dir);
        assert!(!writing.with_extension("source").exists());
        assert!(!writing.with_extension("lock").exists());
        discard_recovery(&backup).unwrap();
        std::fs::remove_file(source).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
    #[test]
    fn long_legacy_custom_function_reopens_resolves_and_refits() {
        use crate::{
            nlfit::CustomFitDefinition,
            ops::{OpKind, Source},
        };
        let mut p = Project::sample();
        let formula = std::iter::repeat_n("a*x", 150).collect::<Vec<_>>().join("+");
        let definition =
            CustomFitDefinition::from_text_with("legacy", &formula, "a", "1", "", &crate::formula::FILE_LIMITS)
                .unwrap();
        p.save_fit_function(None, definition.clone()).unwrap();
        let mut wire: Value = serde_json::from_str(&to_json(&p).unwrap()).unwrap();
        wire["version"] = 21.into();
        let mut p = from_json(&wire.to_string()).unwrap();
        assert_eq!(p.fit_functions[0].definition, definition);
        let (f, cs) = p.fit_functions[0].definition.resolve().unwrap();
        let (_, reports) = p
            .nlfit_custom(
                &Source::Book {
                    id: 1,
                    selected: vec!["C".into()],
                    rows: None,
                },
                f,
                &[1.],
                &[],
                &cs,
            )
            .unwrap();
        assert_eq!(reports.len(), 1);
        let reopened = from_json(&to_json(&p).unwrap()).unwrap();
        let r = &reopened.nlfit_reports()[0];
        assert!(matches!(
            reopened.nlfit_operation(r).unwrap().kind,
            OpKind::Nonlinear(_)
        ));
        let mut v22 = reopened.clone();
        let report = v22
            .books
            .iter_mut()
            .flat_map(|b| &mut b.columns)
            .find_map(|c| c.fit_report.as_deref_mut())
            .unwrap();
        report.fit.status = crate::nlfit::FitStatus::PrecisionLimit;
        assert!(report.fit.converged);
        for restored in [
            from_json(&to_json(&v22).unwrap()).unwrap(),
            from_scv(&to_scv(&v22).unwrap()).unwrap(),
        ] {
            assert_eq!(
                restored.nlfit_reports()[0].fit.status,
                crate::nlfit::FitStatus::PrecisionLimit
            );
        }
        let mut wire = serde_json::to_value(&reopened).unwrap();
        wire["version"] = 21.into();
        let report = wire["books"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .flat_map(|b| b["columns"].as_array_mut().unwrap().iter_mut())
            .filter_map(|c| c.get_mut("fit_report"))
            .find(|r| r.is_object())
            .unwrap();
        report["current"] = false.into();
        report["fit"]["converged"] = false.into();
        report["fit"].as_object_mut().unwrap().remove("status");
        let migrated = from_json(&wire.to_string()).unwrap();
        assert_eq!(migrated.nlfit_reports()[0].fit.status, crate::nlfit::FitStatus::Failed);
        assert_eq!(
            from_json(&to_json(&migrated).unwrap()).unwrap().nlfit_reports()[0]
                .fit
                .status,
            crate::nlfit::FitStatus::Failed
        );
    }
}

#[cfg(test)]
mod round_two_column_formula_tests {
    use super::*;
    #[test]
    fn unchanged_legacy_column_formula_can_be_applied_again() {
        let mut p = Project::sample();
        let formula = std::iter::repeat_n("A", 150).collect::<Vec<_>>().join("+");
        p.books[0].columns[1].formula = Some(formula.clone());
        let mut p = from_json(&to_json(&p).unwrap()).unwrap();
        p.set_column_formula(1, "B", &formula, true).unwrap();
        assert_eq!(p.books[0].columns[1].values()[2], 150.);
        assert!(p.set_column_formula(1, "B", &format!("{formula}+A"), true).is_err());
    }
}
