use super::*;
use crate::dialogs::{BatchExport, BatchFormat};
use std::io::Write;
use std::path::{Path, PathBuf};

fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let name = cleaned.trim().trim_end_matches('.');
    let name = if name.is_empty() { "图形" } else { name };
    let mut result = String::new();
    let mut units = 0;
    for c in name.chars() {
        if units + c.len_utf16() > 100 {
            break;
        }
        result.push(c);
        units += c.len_utf16();
    }
    let reserved = result.split('.').next().unwrap().to_ascii_uppercase();
    if matches!(
        reserved.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    ) {
        result.insert(0, '_');
    }
    result
}

fn write_unique(directory: &Path, name: &str, extension: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let name = safe_name(name);
    for index in 1_u32.. {
        let suffix = if index == 1 {
            String::new()
        } else {
            format!(" ({index})")
        };
        let path = directory.join(format!("{name}{suffix}.{extension}"));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes) {
                    drop(file);
                    let _ = std::fs::remove_file(&path);
                    return Err(error.to_string());
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    unreachable!()
}

pub(super) fn export_graphs_to_folder(
    project: &Project,
    graphs: &[u64],
    format: BatchFormat,
    dpi: u32,
    folder: &Path,
) -> (Vec<PathBuf>, Vec<String>) {
    let mut written = Vec::new();
    let mut errors = Vec::new();
    for id in graphs {
        let Some(graph) = project.graph(*id) else {
            errors.push(format!("图形 {id} 已不存在"));
            continue;
        };
        let svg = svg::render_page(graph, project);
        let data = match format {
            BatchFormat::Svg => Ok(svg.into_bytes()),
            BatchFormat::Pdf => crate::export::pdf_bytes(&svg),
            BatchFormat::Png => {
                crate::export::rasterize(&svg, dpi).and_then(|pixmap| crate::export::png_bytes(&pixmap, dpi))
            }
            BatchFormat::Tiff => {
                crate::export::rasterize(&svg, dpi).and_then(|pixmap| crate::export::tiff_bytes(&pixmap, dpi))
            }
        };
        match data.and_then(|bytes| write_unique(folder, &graph.name, format.extension(), &bytes)) {
            Ok(path) => written.push(path),
            Err(error) => errors.push(format!("{}：{error}", graph.name)),
        }
    }
    (written, errors)
}

impl SciApp {
    pub(crate) fn open_batch_export(&mut self) {
        if self.project.graphs.is_empty() {
            self.status_warn("项目没有图形".into());
            return;
        }
        self.dialog = Some(Dialog::BatchExport(Box::new(BatchExport {
            graphs: self
                .project
                .graphs
                .iter()
                .map(|g| (g.id, g.name.clone(), true))
                .collect(),
            format: BatchFormat::Png,
            dpi: 300,
        })));
    }

    pub(super) fn export_graphs(&mut self, graphs: Vec<u64>, format: BatchFormat, dpi: u32) -> bool {
        let Some(folder) = self.file_dialog().set_title("选择批量导出目录").pick_folder() else {
            self.status_info("已取消导出".into());
            return false;
        };
        self.settings.last_dir = Some(folder.clone());
        let (written, errors) = export_graphs_to_folder(&self.project, &graphs, format, dpi, &folder);
        {
            let status_kind = if errors.is_empty() {
                StatusKind::Success
            } else if written.is_empty() {
                StatusKind::Error
            } else {
                StatusKind::Warn
            };
            let text: String = if errors.is_empty() {
                format!("已导出 {} 张图到 {}", written.len(), folder.display())
            } else {
                format!(
                    "已导出 {} 张；{} 张失败：{}",
                    written.len(),
                    errors.len(),
                    errors.join("；")
                )
            };
            self.set_status(status_kind, text);
        };
        if !errors.is_empty() {
            self.dialog_error = Some(self.status.to_string());
        }
        errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_export_writes_distinct_files_without_overwriting() {
        let mut project = Project::sample();
        let id = project.graphs[0].id;
        project.graphs[0].name = "Alpha".into();
        let mut second = project.graphs[0].clone();
        second.id = project.alloc_id();
        let second_id = second.id;
        project.graphs.push(second);
        let folder = std::env::temp_dir().join(format!("scicanvas-batch-export-{}", std::process::id()));
        std::fs::create_dir(&folder).unwrap();
        let existing = folder.join("Alpha.svg");
        std::fs::write(&existing, b"existing").unwrap();
        let (files, errors) = export_graphs_to_folder(&project, &[id, second_id], BatchFormat::Svg, 300, &folder);
        assert!(errors.is_empty());
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].file_name().unwrap(), "Alpha (2).svg");
        assert_eq!(files[1].file_name().unwrap(), "Alpha (3).svg");
        assert_eq!(std::fs::read(&existing).unwrap(), b"existing");
        assert!(std::fs::read_to_string(&files[0]).unwrap().contains("<svg"));
        assert_eq!(safe_name("CON"), "_CON");
        assert_eq!(safe_name("A/B"), "A_B");
        for file in files {
            std::fs::remove_file(file).unwrap();
        }
        std::fs::remove_file(existing).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }

    #[test]
    fn batch_export_to_missing_directory_does_not_create_files() {
        let directory = std::env::temp_dir().join(format!(
            "scicanvas-batch-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(write_unique(&directory, "plot", "svg", b"data").is_err());
        assert!(!directory.exists());
    }

    #[test]
    fn batch_export_command_opens_graph_selection() {
        let mut app = SciApp::initial(theme::palette(egui::Theme::Light));
        app.run_command("graph.export_batch");
        assert!(matches!(app.dialog, Some(Dialog::BatchExport(_))));
    }
}
