use super::*;

impl Dialog {
    pub(super) fn show_category_order(&mut self, ctx: &egui::Context, pal: &Palette, open: &mut bool) -> Action {
        let Self::CategoryOrder {
            book: _,
            key: _,
            levels,
            error,
        } = self
        else {
            unreachable!()
        };
        window(ctx, "类别顺序").centered().open(open).show(ctx, |ui| {
            let mut swap = None;
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                egui::Grid::new("category-order")
                    .num_columns(3)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        let n = levels.len();
                        for (i, name) in levels.iter().enumerate() {
                            ui.label(RichText::new(format!("{}", i + 1)).color(pal.weak));
                            ui.label(name);
                            ui.horizontal_wrapped(|ui| {
                                if ui.add_enabled(i > 0, egui::Button::new("↑").small()).clicked() {
                                    swap = Some((i, i - 1));
                                }
                                if ui.add_enabled(i + 1 < n, egui::Button::new("↓").small()).clicked() {
                                    swap = Some((i, i + 1));
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
            if let Some((a, b)) = swap {
                levels.swap(a, b);
            }
            if ui.add(plain("按名称排序")).clicked() {
                levels.sort_by(|a, b| natural_compare(a, b));
            }
            live_hint(ui);
            if let Some(error) = error {
                field_error(ui, error.as_str());
            }
        });
        Action::None
    }
}

fn natural_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let ca = a[i..].chars().next().unwrap();
        let cb = b[j..].chars().next().unwrap();
        let order = if ca.is_ascii_digit() && cb.is_ascii_digit() {
            let ia = i;
            let jb = j;
            while i < a.len() && a.as_bytes()[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b.as_bytes()[j].is_ascii_digit() {
                j += 1;
            }
            let x = a[ia..i].trim_start_matches('0');
            let y = b[jb..j].trim_start_matches('0');
            x.len().cmp(&y.len()).then_with(|| x.cmp(y))
        } else {
            i += ca.len_utf8();
            j += cb.len_utf8();
            ca.cmp(&cb)
        };
        if order != std::cmp::Ordering::Equal {
            return order;
        }
    }
    a[i..].len().cmp(&b[j..].len())
}
