//! Find and replace in the active worksheet.
use super::*;

fn matches_text(text: &str, needle: &str, match_case: bool, whole_cell: bool) -> bool {
    if match_case {
        if whole_cell {
            text == needle
        } else {
            text.contains(needle)
        }
    } else {
        let text = text.to_lowercase();
        let needle = needle.to_lowercase();
        if whole_cell {
            text == needle
        } else {
            text.contains(&needle)
        }
    }
}

fn replacement(text: &str, needle: &str, value: &str, match_case: bool, whole_cell: bool) -> String {
    if whole_cell {
        return if matches_text(text, needle, match_case, true) {
            value.into()
        } else {
            text.into()
        };
    }
    if match_case {
        return text.replace(needle, value);
    }
    let needle = needle.to_lowercase();
    let mut folded = String::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for (at, c) in text.char_indices() {
        let end = at + c.len_utf8();
        for lower in c.to_lowercase() {
            starts.extend(std::iter::repeat_n(at, lower.len_utf8()));
            ends.extend(std::iter::repeat_n(end, lower.len_utf8()));
            folded.push(lower);
        }
    }
    let mut output = String::new();
    let mut search = 0;
    let mut copied = 0;
    while let Some(found) = folded[search..].find(&needle) {
        let start = search + found;
        let end = start + needle.len();
        let a = starts[start];
        let b = ends[end - 1];
        if a >= copied {
            output.push_str(&text[copied..a]);
            output.push_str(value);
            copied = b;
        }
        search = end;
    }
    output.push_str(&text[copied..]);
    output
}

fn bounds(book: &scicanvas_core::Book, state: &dialogs::FindReplace) -> Option<(usize, usize, usize, usize)> {
    if book.columns.is_empty() || book.nrows() == 0 {
        return None;
    }
    let range = state.selected_only.then_some(state.selection).flatten();
    let (c0, c1, r0, r1) = range.map_or((0, book.columns.len() - 1, 0, book.nrows() - 1), |r| {
        (
            r.c0(),
            r.c1().min(book.columns.len() - 1),
            r.r0(),
            r.r1().min(book.nrows() - 1),
        )
    });
    (c0 <= c1 && r0 <= r1).then_some((c0, c1, r0, r1))
}

impl SciApp {
    pub(crate) fn open_find_replace(&mut self, replace_mode: bool) {
        let Some(book) = self.active_book() else { return };
        if !self.flush_sheet_edits() {
            return;
        }
        let selection = self.sheets.get(&book).and_then(|s| s.range);
        self.dialog = Some(Dialog::FindReplace(Box::new(dialogs::FindReplace::new(
            book,
            selection,
            replace_mode,
        ))));
    }

    pub(super) fn find_next(&mut self) {
        let Some(Dialog::FindReplace(state)) = &self.dialog else {
            return;
        };
        let state = (**state).clone();
        if state.needle.is_empty() {
            self.find_message("请输入查找内容".into(), None);
            return;
        }
        let Some(book) = self.project.book(state.book) else {
            return;
        };
        let Some((c0, c1, r0, r1)) = bounds(book, &state) else {
            self.find_message("范围内没有数据".into(), None);
            return;
        };
        let width = c1 - c0 + 1;
        let count = width * (r1 - r0 + 1);
        let start = state
            .cursor
            .filter(|c| c.c >= c0 && c.c <= c1 && c.r >= r0 && c.r <= r1)
            .map_or(0, |c| ((c.r - r0) * width + c.c - c0 + 1) % count);
        let found = (0..count).find_map(|step| {
            let index = (start + step) % count;
            let cell = Cell {
                c: c0 + index % width,
                r: r0 + index / width,
            };
            let column = &book.columns[cell.c];
            (cell.r < column.data.len()
                && matches_text(
                    &column.data.text_at(cell.r),
                    &state.needle,
                    state.match_case,
                    state.whole_cell,
                ))
            .then_some(cell)
        });
        if let Some(cell) = found {
            self.show_item(state.book);
            let sheet = self.sheets.entry(state.book).or_default();
            sheet.range = Some(Range::single(cell.c, cell.r));
            sheet.scroll_to = Some(cell.r);
            self.find_message(
                format!("已找到 {}{}", scicanvas_core::column_letter(cell.c), cell.r + 1),
                Some(cell),
            );
        } else {
            self.find_message("未找到匹配内容".into(), None);
        }
    }

    fn find_message(&mut self, message: String, cursor: Option<Cell>) {
        if let Some(Dialog::FindReplace(state)) = &mut self.dialog {
            state.message = message;
            state.cursor = cursor;
        }
    }

    pub(super) fn replace_matches(&mut self, all: bool) {
        let Some(Dialog::FindReplace(state)) = &self.dialog else {
            return;
        };
        let state = (**state).clone();
        if state.needle.is_empty() {
            self.find_message("请输入查找内容".into(), None);
            return;
        }
        if !all && state.cursor.is_none() {
            self.find_message("请先查找匹配单元格".into(), None);
            return;
        }
        let outcome = self.edit_with(
            "替换",
            |project| {
                let book = project.book_mut(state.book).ok_or("找不到工作簿")?;
                let Some((mut c0, mut c1, mut r0, mut r1)) = bounds(book, &state) else {
                    return Ok(Some((0, 0)));
                };
                if !all {
                    let cell = state.cursor.unwrap();
                    if cell.c < c0 || cell.c > c1 || cell.r < r0 || cell.r > r1 {
                        return Ok(Some((0, 0)));
                    }
                    (c0, c1, r0, r1) = (cell.c, cell.c, cell.r, cell.r);
                }
                let mut changed = 0;
                let mut rejected = 0;
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        let col = &mut book.columns[c];
                        if r >= col.data.len() {
                            continue;
                        }
                        let old = col.data.text_at(r).into_owned();
                        let new = replacement(
                            &old,
                            &state.needle,
                            &state.replacement,
                            state.match_case,
                            state.whole_cell,
                        );
                        if new == old {
                            continue;
                        }
                        if col.data.accepts(&new) {
                            col.set(r, new);
                            changed += 1;
                        } else {
                            rejected += 1;
                        }
                    }
                }
                Ok(Some((changed, rejected)))
            },
            |_, result| result,
        );
        if let Some((changed, rejected)) = outcome {
            self.find_message(
                format!("已替换 {changed} 个单元格，{rejected} 个因类型不符保留原值"),
                state.cursor,
            );
        }
    }
}
