//! Note 用的轻量 Markdown：标题、段落、列表、引用、代码块、分隔线和表格，
//! 行内支持 **粗体**、*斜体* 和 `代码`。表格右键可复制为制表符文本或 Markdown。
use eframe::egui::{self, text::LayoutJob, Color32, FontFamily, FontId, RichText, TextFormat};

use crate::theme::Palette;

#[derive(Debug, PartialEq)]
pub enum Block {
    Heading(usize, String),
    Paragraph(String),
    /// 列表项：标记（“•” 或 “1.”）、缩进层级、内容。
    List(Vec<(String, usize, String)>),
    Quote(String),
    Code(String),
    Rule,
    /// 首行为表头。
    Table(Vec<Vec<String>>),
}

fn heading(line: &str) -> Option<(usize, String)> {
    let level = line.chars().take_while(|&c| c == '#').count();
    let rest = &line[level..];
    ((1..=6).contains(&level) && (rest.is_empty() || rest.starts_with(' ')))
        .then(|| (level, rest.trim().trim_end_matches('#').trim_end().to_string()))
}

fn list_item(line: &str) -> Option<(String, usize, String)> {
    let indent = line.len() - line.trim_start().len();
    let t = line.trim_start();
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = t.strip_prefix(bullet) {
            return Some(("•".into(), indent / 2, rest.trim().to_string()));
        }
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    let rest = &t[digits..];
    (digits > 0 && (rest.starts_with(". ") || rest.starts_with(") ")))
        .then(|| (format!("{}.", &t[..digits]), indent / 2, rest[2..].trim().to_string()))
}

fn is_rule(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    t.len() >= 3 && ["-", "*", "_"].iter().any(|m| t.chars().all(|c| c.to_string() == *m))
}

/// 表格行拆成单元格：去掉首尾竖线，`\|` 是单元格里的竖线。
fn cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').filter(|s| !s.ends_with('\\')).unwrap_or(t);
    let mut out = vec![String::new()];
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => out.last_mut().unwrap().push(chars.next().unwrap()),
            '|' => out.push(String::new()),
            _ => out.last_mut().unwrap().push(c),
        }
    }
    out.iter().map(|c| c.trim().to_string()).collect()
}

fn is_separator(line: &str) -> bool {
    let c = cells(line);
    line.contains('-')
        && c.iter().all(|c| {
            let c = c.trim_matches(':');
            !c.is_empty() && c.chars().all(|ch| ch == '-')
        })
}

pub fn parse(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = vec![];
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();
        if t.is_empty() {
            i += 1;
        } else if t.starts_with("```") {
            let mut code = vec![];
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                code.push(lines[i]);
                i += 1;
            }
            i += 1;
            blocks.push(Block::Code(code.join("\n")));
        } else if let Some((level, title)) = heading(t) {
            blocks.push(Block::Heading(level, title));
            i += 1;
        } else if t.starts_with('|') && i + 1 < lines.len() && is_separator(lines[i + 1]) {
            let mut rows = vec![cells(t)];
            let width = rows[0].len();
            i += 2;
            while i < lines.len() && lines[i].trim().starts_with('|') {
                let mut row = cells(lines[i]);
                row.resize(width, String::new());
                rows.push(row);
                i += 1;
            }
            blocks.push(Block::Table(rows));
        } else if is_rule(t) {
            blocks.push(Block::Rule);
            i += 1;
        } else if list_item(line).is_some() {
            let mut items = vec![];
            while let Some(item) = lines.get(i).and_then(|l| list_item(l)) {
                items.push(item);
                i += 1;
            }
            blocks.push(Block::List(items));
        } else if t.starts_with('>') {
            let mut quote = vec![];
            while let Some(rest) = lines.get(i).and_then(|l| l.trim().strip_prefix('>')) {
                quote.push(rest.trim());
                i += 1;
            }
            blocks.push(Block::Quote(quote.join("\n")));
        } else {
            // 段落保留原来的换行，旧的纯文本 Note 显示不变
            let mut para = vec![line.trim_end()];
            i += 1;
            while let Some(l) = lines.get(i) {
                let t = l.trim();
                if t.is_empty()
                    || t.starts_with("```")
                    || t.starts_with('>')
                    || heading(t).is_some()
                    || is_rule(t)
                    || list_item(l).is_some()
                    || (t.starts_with('|') && lines.get(i + 1).is_some_and(|n| is_separator(n)))
                {
                    break;
                }
                para.push(l.trim_end());
                i += 1;
            }
            blocks.push(Block::Paragraph(para.join("\n")));
        }
    }
    blocks
}

/// 行内样式：`代码`、**粗体**、*斜体*。
fn inline(ui: &egui::Ui, text: &str, size: f32, bold: bool, color: Color32, pal: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let family = |b: bool| {
        let name = FontFamily::Name("plot-bold".into());
        if b && ui.fonts(|f| f.families().contains(&name)) {
            name
        } else {
            FontFamily::Proportional
        }
    };
    let (mut strong, mut italic) = (bold, false);
    let mut buf = String::new();
    let flush = |job: &mut LayoutJob, buf: &mut String, strong: bool, italic: bool| {
        if !buf.is_empty() {
            let format = TextFormat {
                font_id: FontId::new(size, family(strong)),
                color,
                italics: italic,
                ..Default::default()
            };
            job.append(buf, 0.0, format);
            buf.clear();
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        if c == '\\' && k + 1 < chars.len() && "\\`*_|#".contains(chars[k + 1]) {
            buf.push(chars[k + 1]);
            k += 2;
        } else if c == '`' {
            if let Some(end) = chars[k + 1..].iter().position(|&x| x == '`') {
                flush(&mut job, &mut buf, strong, italic);
                let code: String = chars[k + 1..k + 1 + end].iter().collect();
                job.append(
                    &code,
                    0.0,
                    TextFormat {
                        font_id: FontId::new(size, FontFamily::Monospace),
                        color,
                        background: pal.hover,
                        ..Default::default()
                    },
                );
                k += end + 2;
            } else {
                buf.push(c);
                k += 1;
            }
        } else if (c == '*' || c == '_') && chars.get(k + 1) == Some(&c) {
            flush(&mut job, &mut buf, strong, italic);
            strong = !strong;
            k += 2;
        } else if c == '*'
            && if italic {
                k > 0 && !chars[k - 1].is_whitespace()
            } else {
                chars.get(k + 1).is_some_and(|x| !x.is_whitespace())
            }
        {
            // “a * b” 里的乘号不当斜体标记
            flush(&mut job, &mut buf, strong, italic);
            italic = !italic;
            k += 1;
        } else {
            buf.push(c);
            k += 1;
        }
    }
    flush(&mut job, &mut buf, strong, italic);
    job.wrap.max_width = ui.available_width();
    job
}

/// 表格转成制表符分隔文本（粘贴到 Excel 会分列）。
pub fn tsv(rows: &[Vec<String>]) -> String {
    rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n")
}

/// 表格转回 Markdown。
pub fn table_markdown(rows: &[Vec<String>]) -> String {
    let line = |r: &Vec<String>| {
        let cells: Vec<String> = r.iter().map(|c| c.replace('|', "\\|")).collect();
        format!("| {} |", cells.join(" | "))
    };
    let mut out = vec![line(&rows[0]), format!("|{}", " --- |".repeat(rows[0].len()))];
    out.extend(rows[1..].iter().map(line));
    out.join("\n")
}

/// 正在“整格选中”的单元格：双击后临时换成只读文本框，整格全选（egui 双击选词会在小数点、负号处断开）。
#[derive(Clone, Copy)]
struct ActiveCell {
    id: egui::Id,
    /// 刚双击：下一帧设好全选并获得焦点。
    fresh: bool,
}

fn active_cell_key() -> egui::Id {
    egui::Id::new("md-active-cell")
}

fn table_menu(ui: &mut egui::Ui, rows: &[Vec<String>]) {
    if ui.button("复制表格").clicked() {
        ui.ctx().copy_text(tsv(rows));
        ui.close_menu();
    }
    if ui.button("复制为 Markdown").clicked() {
        ui.ctx().copy_text(table_markdown(rows));
        ui.close_menu();
    }
}

fn cell(ui: &mut egui::Ui, job: LayoutJob, id: egui::Id, rows: &[Vec<String>]) {
    let active = ui.ctx().data(|d| d.get_temp::<ActiveCell>(active_cell_key()));
    if let Some(mut state) = active.filter(|a| a.id == id) {
        // 只读文本框：&str 不能修改，但能选择和 Ctrl+C
        let mut text: &str = &job.text;
        let galley = ui.fonts(|f| f.layout_job(job.clone()));
        let mut layouter = |ui: &egui::Ui, _: &str, _: f32| ui.fonts(|f| f.layout_job(job.clone()));
        let mut output = egui::TextEdit::singleline(&mut text)
            .id(id)
            .frame(false)
            .margin(egui::Margin::ZERO)
            .desired_width(galley.size().x)
            .layouter(&mut layouter)
            .show(ui);
        if state.fresh {
            let all = egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(job.text.chars().count()),
            );
            output.state.cursor.set_char_range(Some(all));
            output.state.store(ui.ctx(), id);
            output.response.request_focus();
            state.fresh = false;
            ui.ctx().data_mut(|d| d.insert_temp(active_cell_key(), state));
            ui.ctx().request_repaint();
        } else if !output.response.has_focus() {
            ui.ctx().data_mut(|d| d.remove::<ActiveCell>(active_cell_key()));
            ui.ctx().request_repaint();
        }
        output.response.context_menu(|ui| table_menu(ui, rows));
        return;
    }
    let response = ui.add(egui::Label::new(job));
    if response.double_clicked() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(active_cell_key(), ActiveCell { id, fresh: true }));
        ui.ctx().request_repaint();
    }
    response.context_menu(|ui| table_menu(ui, rows));
}

fn table(ui: &mut egui::Ui, rows: &[Vec<String>], index: usize, pal: &Palette) {
    let size = 14.0;
    let base = ui.id().with(("md-table", index));
    egui::Frame::none()
        .stroke(egui::Stroke::new(1.0_f32, pal.line))
        .inner_margin(egui::Margin::symmetric(8.0, 6.0))
        .show(ui, |ui| {
            egui::Grid::new(base).striped(true).spacing([18.0, 5.0]).show(ui, |ui| {
                for (r, row) in rows.iter().enumerate() {
                    for (c, text) in row.iter().enumerate() {
                        let job = inline(ui, text, size, r == 0, pal.ink, pal);
                        cell(ui, job, base.with((r, c)), rows);
                    }
                    ui.end_row();
                }
            });
        });
}

pub fn show(ui: &mut egui::Ui, text: &str, pal: &Palette) {
    let body = 14.0;
    for (index, block) in parse(text).iter().enumerate() {
        match block {
            Block::Heading(level, title) => {
                let size = match level {
                    1 => 22.0,
                    2 => 19.0,
                    3 => 16.5,
                    _ => body,
                };
                ui.add_space(if index == 0 { 0.0 } else { 6.0 });
                ui.label(inline(ui, title, size, true, pal.ink, pal));
                if *level <= 2 {
                    ui.separator();
                }
            }
            Block::Paragraph(p) => {
                ui.label(inline(ui, p, body, false, pal.ink, pal));
            }
            Block::List(items) => {
                for (marker, depth, item) in items {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(8.0 + 18.0 * *depth as f32);
                        ui.label(RichText::new(marker).size(body).color(pal.weak));
                        ui.label(inline(ui, item, body, false, pal.ink, pal));
                    });
                }
            }
            Block::Quote(q) => {
                egui::Frame::none()
                    .inner_margin(egui::Margin {
                        left: 10.0,
                        ..Default::default()
                    })
                    .show(ui, |ui| {
                        let rect = ui.max_rect();
                        ui.label(inline(ui, q, body, false, pal.weak, pal));
                        ui.painter()
                            .vline(rect.left() - 8.0, rect.y_range(), egui::Stroke::new(3.0_f32, pal.line));
                    });
            }
            Block::Code(code) => {
                egui::Frame::none()
                    .fill(pal.hover)
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(code).monospace().color(pal.ink));
                    });
            }
            Block::Rule => {
                ui.separator();
            }
            Block::Table(rows) => table(ui, rows, index, pal),
        }
        ui.add_space(6.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headings_tables_lists_code_and_plain_text() {
        let text = "## 电压 A — 拟合\n\n`y = 1 + 2·x`\n\n| 参数 | 值 |\n| --- | ---: |\n| B0 | 1 |\n| a\\|b |\n\n- 一\n  - 二\n1. 三\n\n```\nx\ty\n```\n\n---\n> 引用\n旧文本\t1\n第二行";
        assert_eq!(
            parse(text),
            vec![
                Block::Heading(2, "电压 A — 拟合".into()),
                Block::Paragraph("`y = 1 + 2·x`".into()),
                Block::Table(vec![
                    vec!["参数".into(), "值".into()],
                    vec!["B0".into(), "1".into()],
                    vec!["a|b".into(), String::new()],
                ]),
                Block::List(vec![
                    ("•".into(), 0, "一".into()),
                    ("•".into(), 1, "二".into()),
                    ("1.".into(), 0, "三".into()),
                ]),
                Block::Code("x\ty".into()),
                Block::Rule,
                Block::Quote("引用".into()),
                Block::Paragraph("旧文本\t1\n第二行".into()),
            ]
        );
        // 没有分隔行的竖线文本不是表格
        assert_eq!(parse("| a |\nb"), vec![Block::Paragraph("| a |\nb".into())]);
    }

    #[test]
    fn tables_copy_as_tsv_and_markdown() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["1|2".to_string(), "3".to_string()],
        ];
        assert_eq!(tsv(&rows), "a\tb\n1|2\t3");
        assert_eq!(table_markdown(&rows), "| a | b |\n| --- | --- |\n| 1\\|2 | 3 |");
        let Block::Table(back) = &parse(&table_markdown(&rows))[0] else {
            panic!()
        };
        assert_eq!(back, &rows);
    }
}
