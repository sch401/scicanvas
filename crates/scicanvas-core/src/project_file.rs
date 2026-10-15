//! 项目文件入口：版本迁移、完整性检查和同目录临时文件原子替换。

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use serde_json::Value;

use crate::Project;

pub const CURRENT_VERSION: u32 = 3;

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
        .max()
        .unwrap_or(0);
    project.next_id = project.next_id.max(max_id.checked_add(1).ok_or("项目对象编号已用尽")?);
    validate(&project)?;
    Ok(project)
}

pub fn to_json(project: &Project) -> Result<String, String> {
    validate(project)?;
    let mut value = serde_json::to_value(project).map_err(|e| e.to_string())?;
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
    {
        if id == 0 || id == u64::MAX || !ids.insert(id) {
            return Err(format!("对象编号 {id} 无效或重复"));
        }
    }
    for book in &project.books {
        let mut keys = BTreeSet::new();
        for col in &book.columns {
            if col.key.is_empty() || !keys.insert(&col.key) {
                return Err(format!("工作簿 {} 的列标识为空或重复", book.name));
            }
        }
    }
    for graph in &project.graphs {
        graph.page.validate()?;
        for layer in graph.page.layers() {
            if [
                layer.x_min,
                layer.x_max,
                layer.y_min,
                layer.y_max,
                f64::from(layer.line_width),
                f64::from(layer.symbol_size),
            ]
            .iter()
            .any(|x| !x.is_finite())
            {
                return Err(format!("图形 {} 含无效的数值属性", graph.name));
            }
            if layer.rows.is_some_and(|(a, b)| a > b || b == usize::MAX) {
                return Err("图层行范围无效".into());
            }
        }
    }
    Ok(())
}

pub fn save(path: &Path, project: &Project) -> Result<(), String> {
    let content = to_json(project)?;
    let name = path.file_name().ok_or("项目文件名为空")?.to_string_lossy();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temporary = path.with_file_name(format!(".{name}.{}.{stamp}.tmp", std::process::id()));
    let mut file = std::fs::File::create_new(&temporary).map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("保存项目失败：{e}"))
}
