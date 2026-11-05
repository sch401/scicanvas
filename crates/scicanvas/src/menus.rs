use crate::{app::SciApp, commands, ribbon};
use eframe::egui;
use std::collections::HashMap;

type MenuEntry = (Result<(), &'static str>, Option<bool>);
pub(crate) struct MenuState(pub(crate) HashMap<&'static str, MenuEntry>);
impl MenuState {
    pub(crate) fn new(app: &SciApp) -> Self {
        let mut state = Self(
            commands::commands()
                .iter()
                .map(|c| (c.id, ((c.when)(app), c.checked.map(|f| f(app)))))
                .collect(),
        );
        if let Some(book) = app.active_book_id().and_then(|id| app.project.book(id)) {
            let selected = app.selected_keys(book.id);
            let columns: Vec<_> = selected.iter().filter_map(|k| book.column(k)).collect();
            for (id, role) in [
                ("col.setX", scicanvas_core::Role::X),
                ("col.setY", scicanvas_core::Role::Y),
                ("col.setZ", scicanvas_core::Role::Z),
                ("col.setLabel", scicanvas_core::Role::Label),
                ("col.setIgnore", scicanvas_core::Role::Ignore),
                ("col.setXError", scicanvas_core::Role::XError),
                ("col.setYError", scicanvas_core::Role::YError),
            ] {
                state.0.get_mut(id).unwrap().1 = Some(!columns.is_empty() && columns.iter().all(|c| c.role == role));
            }
            for (id, kind) in [
                ("col.type.num", scicanvas_core::DataKind::Num),
                ("col.type.text", scicanvas_core::DataKind::Text),
                ("col.type.datetime", scicanvas_core::DataKind::DateTime),
                ("col.type.category", scicanvas_core::DataKind::Category),
            ] {
                let current = !columns.is_empty() && columns.iter().all(|c| c.data.kind() == kind);
                let entry = state.0.get_mut(id).unwrap();
                entry.1 = Some(current);
                if current {
                    entry.0 = Err("当前数据类型");
                }
            }
        }
        state
    }
}
pub(crate) fn menu_command(ui: &mut egui::Ui, app: &MenuState, id: &str) -> bool {
    let c = commands::find(id).expect("registered menu command");
    let (enabled, checked) = app.0.get(c.id).copied().unwrap();
    let mut button = egui::Button::new(c.label).shortcut_text(c.keys);
    if let Some(checked) = checked {
        let image = crate::icons::icon("menu-check", ui.visuals().dark_mode).fit_to_exact_size(egui::vec2(12., 12.));
        let image = if checked {
            image
        } else {
            image.tint(egui::Color32::TRANSPARENT)
        };
        button = egui::Button::image_and_text(image, c.label)
            .shortcut_text(c.keys)
            .selected(checked);
    }
    let response = ui.add_enabled(enabled.is_ok(), button);
    let response = if let Err(reason) = enabled {
        response.on_disabled_hover_text(reason)
    } else {
        response
    };
    if response.clicked() {
        ui.close_menu();
        true
    } else {
        false
    }
}
pub(crate) fn plot_menu(ui: &mut egui::Ui, app: &MenuState) -> Option<&'static str> {
    let mut picked = None;
    ui.menu_button("绘图", |ui| {
        for (name, ids) in ribbon::PLOT_CATEGORIES {
            ui.menu_button(*name, |ui| {
                for id in *ids {
                    if menu_command(ui, app, id) {
                        picked = Some(*id);
                    }
                }
            });
        }
    });
    picked
}
