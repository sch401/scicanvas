//! Text import options, preview, and reusable templates.
use super::*;
use scicanvas_core::csv::{self, ImportOptions};
use std::path::PathBuf;

pub(crate) struct ImportWizard {
    path: PathBuf,
    bytes: Vec<u8>,
    text: String,
    encoding: u8,
    options: ImportOptions,
    pub(crate) error: Option<String>,
}

impl ImportWizard {
    pub(crate) fn new(path: PathBuf, bytes: Vec<u8>) -> Self {
        let mut options = ImportOptions::default();
        let text = csv::decode(&bytes);
        options.header_rows = csv::guess_import_header(&text, &options);
        Self {
            path,
            bytes,
            text,
            encoding: options.encoding,
            options,
            error: None,
        }
    }

    pub(super) fn show(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let mut action = Action::None;
        window(ctx, "导入向导").open(open).default_width(760.0).show(ctx, |ui| {
            ui.label(self.path.display().to_string());
            ui.horizontal_wrapped(|ui| {
                ui.label("编码");
                egui::ComboBox::from_id_salt("import-encoding")
                    .selected_text(
                        ["自动", "UTF-8", "GBK", "UTF-16 LE", "UTF-16 BE"]
                            .get(self.options.encoding as usize)
                            .copied()
                            .unwrap_or("自动"),
                    )
                    .show_ui(ui, |ui| {
                        for (i, name) in ["自动", "UTF-8", "GBK", "UTF-16 LE", "UTF-16 BE"].iter().enumerate() {
                            ui.selectable_value(&mut self.options.encoding, i as u8, *name);
                        }
                    });
                ui.label("分隔符");
                egui::ComboBox::from_id_salt("import-delimiter")
                    .selected_text(
                        ["自动", "逗号", "制表符", "分号", "空白"]
                            .get(self.options.delimiter as usize)
                            .copied()
                            .unwrap_or("自动"),
                    )
                    .show_ui(ui, |ui| {
                        for (i, name) in ["自动", "逗号", "制表符", "分号", "空白"].iter().enumerate() {
                            ui.selectable_value(&mut self.options.delimiter, i as u8, *name);
                        }
                    });
                ui.label("跳过行");
                ui.add(egui::DragValue::new(&mut self.options.skip_rows).range(0..=1_000_000));
                ui.label("表头行（名称、单位、注释）");
                ui.add(egui::DragValue::new(&mut self.options.header_rows).range(0..=3));
            });
            if self.encoding != self.options.encoding {
                match csv::decode_import(&self.bytes, &self.options) {
                    Ok(text) => {
                        self.text = text;
                        self.error = None;
                    }
                    Err(e) => {
                        self.text.clear();
                        self.error = Some(e);
                    }
                }
                self.encoding = self.options.encoding;
            }
            let preview = csv::preview_import(&self.text, &self.options, 50);
            let columns = preview.iter().map(Vec::len).max().unwrap_or(0);
            self.options.column_types.resize(columns, 0);
            ui.label("预览前 50 行，可为每列选择类型");
            egui::ScrollArea::both().max_height(360.0).show(ui, |ui| {
                egui::Grid::new("import-preview").striped(true).show(ui, |ui| {
                    ui.label("列类型");
                    for (i, kind) in self.options.column_types.iter_mut().enumerate() {
                        let labels = ["自动", "数值", "文本", "日期时间", "分类"];
                        egui::ComboBox::from_id_salt(("import-type", i))
                            .selected_text(labels.get(*kind as usize).copied().unwrap_or("自动"))
                            .show_ui(ui, |ui| {
                                for (n, label) in labels.iter().enumerate() {
                                    ui.selectable_value(kind, n as u8, *label);
                                }
                            });
                    }
                    ui.end_row();
                    for (i, row) in preview.iter().enumerate() {
                        ui.label((i + self.options.skip_rows + 1).to_string());
                        for c in 0..columns {
                            ui.label(row.get(c).map(String::as_str).unwrap_or(""));
                        }
                        ui.end_row();
                    }
                });
            });
            if let Some(error) = &self.error {
                ui.colored_label(pal.danger, error);
            }
            ui.horizontal(|ui| {
                if ui.button("保存模板…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("导入模板", &["scit"]).save_file() {
                        self.error = csv::import_template_json(&self.options)
                            .and_then(|s| std::fs::write(path, s).map_err(|e| e.to_string()))
                            .err();
                    }
                }
                if ui.button("加载模板…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("导入模板", &["scit"]).pick_file() {
                        match std::fs::read_to_string(path)
                            .map_err(|e| e.to_string())
                            .and_then(|s| csv::import_template_from_json(&s))
                        {
                            Ok(options) => {
                                self.options = options;
                                self.error = None;
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                }
            });
            let (ok, cancel) = buttons(ui, "导入");
            if ok {
                action = Action::Import(self.path.clone(), self.options.clone());
            }
            if cancel {
                action = Action::Close;
            }
        });
        action
    }
}
