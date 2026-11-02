use super::*;

pub(super) fn help(commands: &mut Vec<Command>) {
    commands.extend([
        // 帮助
        c(
            "help.show",
            "快速上手",
            "help",
            "F1",
            "帮助",
            "help",
            always,
            |a| a.show_help = true,
        ),
        c(
            "help.about",
            "关于 SciCanvas",
            "help",
            "",
            "帮助",
            "about version 版本",
            always,
            |a| a.show_about = true,
        ),
        c(
            "palette.open",
            "搜索命令",
            "search",
            "Ctrl+K",
            "帮助",
            "command palette search",
            always,
            |a| a.ui.open_palette(),
        ),
    ]);
}

pub(super) fn explorer(commands: &mut Vec<Command>) {
    commands.extend([
        // 视图
        c(
            "view.explorer",
            "项目管理器",
            "explorer",
            "",
            "视图",
            "project explorer",
            always,
            |a| a.show_explorer = !a.show_explorer,
        )
        .checked(|a| a.show_explorer),
    ]);
}

pub(super) fn appearance(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "theme.light",
            "浅色",
            "sun",
            "",
            "主题",
            "light theme",
            always,
            |a| a.set_theme(ThemePreference::Light),
        )
        .checked(|a| a.theme_pref == ThemePreference::Light),
        c(
            "theme.dark",
            "深色",
            "moon",
            "",
            "主题",
            "dark theme",
            always,
            |a| a.set_theme(ThemePreference::Dark),
        )
        .checked(|a| a.theme_pref == ThemePreference::Dark),
        c(
            "theme.system",
            "跟随系统",
            "system",
            "",
            "主题",
            "system theme",
            always,
            |a| a.set_theme(ThemePreference::System),
        )
        .checked(|a| a.theme_pref == ThemePreference::System),
        c(
            "ribbon.toggle",
            "折叠功能区",
            "chevron",
            "Ctrl+F1",
            "视图",
            "collapse ribbon",
            always,
            |a| a.ui.toggle_collapsed(),
        ),
    ]);
}
