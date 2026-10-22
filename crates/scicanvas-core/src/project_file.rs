//! 项目文件入口：版本迁移、完整性检查和同目录临时文件原子替换。
//!
//! `.scv` 是 ZIP 容器（类似 .docx）：`project.json` 是唯一的权威数据，
//! `tables/*.csv` 和 `graphs/*.svg` 是保存时生成的预览，解压即可查看，打开时忽略。
//! 旧版 `.scproj` 为纯 JSON，继续支持读写。

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use serde_json::Value;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::{Book, Project};

pub const CURRENT_VERSION: u32 = crate::model::PROJECT_VERSION;

/// `.scv` 容器版本，与 `project.json` 的项目版本分开演进。
pub const SCV_VERSION: u32 = 1;
pub const SCV_MIMETYPE: &str = "application/vnd.scicanvas.project+zip";
const PROJECT_ENTRY: &str = "project.json";
const MANIFEST_ENTRY: &str = "manifest.json";
/// 解压单个条目的上限，防止损坏或恶意压缩包耗尽内存。
const MAX_ENTRY_BYTES: u64 = 1 << 31;

/// v11 保存独立曲线映射与样式；属性控件的旧键仅留在内存编辑视图中。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCurve {
    mapping: crate::plotspec::CurveMapping,
    style: Option<crate::page::CurveStyle>,
    #[serde(default)]
    x_override: bool,
}

fn decode_plots(
    layer: &mut Value,
    defaults: &std::collections::BTreeMap<(u64, String), Option<String>>,
) -> Result<(), String> {
    let Some(plots) = layer.get("plots").cloned() else {
        return Ok(());
    };
    let plots: Vec<StoredCurve> = serde_json::from_value(plots).map_err(|e| format!("曲线映射无效：{e}"))?;
    let main = layer.get("book_id").and_then(Value::as_u64);
    let rows: Option<(usize, usize)> =
        serde_json::from_value(layer.get("rows").cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())?;
    let mut ys = Vec::new();
    let mut xs = serde_json::Map::new();
    let mut bases = serde_json::Map::new();
    let mut lines = Vec::new();
    let mut styles = serde_json::Map::new();
    let mut ranges = serde_json::Map::new();
    let mut seen = BTreeSet::new();
    for plot in plots {
        let m = plot.mapping;
        m.validate()?;
        if !seen.insert(m.y.clone()) {
            return Err("曲线行范围无效或来源重复".into());
        }
        let key = m.y.key(main);
        ys.push(key.clone());
        if let Some(x) = m.x {
            let default = if Some(m.y.book) == main {
                layer.get("x_key").and_then(Value::as_str).map(str::to_string)
            } else {
                defaults.get(&(m.y.book, m.y.col.clone())).cloned().flatten()
            };
            let inherited = x.book == m.y.book && default.as_deref() == Some(&x.col);
            let x = x.key(main);
            // 与图层默认 X 相同的本表映射无须重复存入编辑视图。
            if plot.x_override || !inherited {
                xs.insert(key.clone(), x.into());
            }
        }
        if let Some(base) = m.base {
            bases.insert(key.clone(), base.key(main).into());
        }
        let inherited = if Some(m.y.book) == main && !m.line_only {
            rows
        } else {
            None
        };
        if m.rows != inherited {
            ranges.insert(key.clone(), serde_json::to_value(m.rows).map_err(|e| e.to_string())?);
        }
        if m.line_only {
            lines.push(key.clone());
        }
        if let Some(style) = plot.style {
            styles.insert(key, serde_json::to_value(style).map_err(|e| e.to_string())?);
        }
    }
    layer["y_keys"] = serde_json::to_value(ys).map_err(|e| e.to_string())?;
    layer["x_by_y"] = xs.into();
    layer["base_by_y"] = bases.into();
    layer["line_only"] = serde_json::to_value(lines).map_err(|e| e.to_string())?;
    layer["curves"] = styles.into();
    layer["row_by_y"] = ranges.into();
    layer.as_object_mut().unwrap().remove("plots");
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
    let value = serde_json::from_str(text).map_err(|e| format!("JSON 格式错误：{e}"))?;
    let mut project: Project = serde_json::from_value(migrate(value)?).map_err(|e| format!("项目字段错误：{e}"))?;
    let max_id = project
        .books
        .iter()
        .map(|b| b.id)
        .chain(project.graphs.iter().map(|g| g.id))
        .chain(project.notes.iter().map(|n| n.id))
        .chain(project.folders.iter().map(|f| f.id))
        .max()
        .unwrap_or(0);
    project.next_id = project.next_id.max(max_id.checked_add(1).ok_or("项目对象编号已用尽")?);
    validate(&project)?;
    Ok(project)
}

pub fn to_json(project: &Project) -> Result<String, String> {
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
                .plots_in(&|id| project.book(id))
                .into_iter()
                .map(|plot| StoredCurve {
                    mapping: plot.mapping,
                    style: plot.style,
                    x_override: plot.x_override,
                })
                .collect();
            wire["plots"] = serde_json::to_value(plots).map_err(|e| e.to_string())?;
            for key in ["y_keys", "x_by_y", "base_by_y", "line_only", "curves", "row_by_y"] {
                wire.as_object_mut().unwrap().remove(key);
            }
        }
    }
    value["version"] = CURRENT_VERSION.into();
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
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
    {
        if id == 0 || id == u64::MAX || !ids.insert(id) {
            return Err(format!("对象编号 {id} 无效或重复"));
        }
    }
    project.validate_folders()?;
    if project.notes.iter().any(|n| n.name.trim().is_empty()) {
        return Err("Note 名称不能为空".into());
    }
    for book in &project.books {
        let mut keys = BTreeSet::new();
        for col in &book.columns {
            col.data.validate()?;
            if col.key.is_empty() || !keys.insert(&col.key) {
                return Err(format!("工作簿 {} 的列标识为空或重复", book.name));
            }
        }
    }
    for graph in &project.graphs {
        graph.page.validate()?;
        for layer in graph.page.layers() {
            for key in &layer.y_keys {
                if let Some(reference) = crate::plotspec::DataRef::from_key(layer.book_id, key) {
                    if let Some(mapping) = layer.curve_mapping(key, project.book(reference.book)) {
                        mapping.validate()?;
                    }
                }
            }
            if [
                layer.x_min,
                layer.x_max,
                layer.y_min,
                layer.y_max,
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
            if layer.rows.is_some_and(|(a, b)| a > b || b == usize::MAX)
                || layer
                    .row_by_y
                    .values()
                    .flatten()
                    .any(|(a, b)| a > b || *b == usize::MAX)
            {
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
    let name = path.file_name().ok_or("项目文件名为空")?.to_string_lossy();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = path.with_file_name(format!(".{name}.{}.{stamp}.tmp", std::process::id()));
    let mut file = std::fs::File::create_new(&temporary).map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(&content)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("保存项目失败：{e}"))
}
