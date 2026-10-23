use super::*;

pub(super) fn reset(commands: &mut Vec<Command>) {
    commands.extend([c(
        "win.reset",
        "重置窗口位置",
        "cascade",
        "Ctrl+Shift+R",
        "窗口",
        "reset windows 找回窗口 还原位置",
        always,
        |a| a.reset_windows(),
    )]);
}

pub(super) fn current(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "window.minimize",
            "最小化当前窗口",
            "book",
            "",
            "工作区",
            "minimize",
            need_window,
            |a| a.set_project_window_state(crate::mdi::WinState::Min),
        ),
        c(
            "window.maximize",
            "最大化当前窗口",
            "book",
            "",
            "工作区",
            "maximize",
            need_window,
            |a| a.set_project_window_state(crate::mdi::WinState::Max),
        ),
        c(
            "window.restore",
            "还原当前窗口",
            "book",
            "",
            "工作区",
            "restore",
            need_window,
            |a| a.set_project_window_state(crate::mdi::WinState::Normal),
        ),
    ]);
}

pub(super) fn arrange(commands: &mut Vec<Command>) {
    commands.extend([
        c(
            "win.showall",
            "显示全部窗口",
            "showall",
            "",
            "窗口",
            "show all windows",
            always,
            |a| a.show_all(),
        ),
        c(
            "win.cascade",
            "层叠",
            "cascade",
            "",
            "窗口",
            "cascade windows",
            always,
            |a| a.arrange_windows(false),
        ),
        c(
            "win.tile",
            "平铺",
            "tile",
            "",
            "窗口",
            "tile windows",
            always,
            |a| a.arrange_windows(true),
        ),
        c(
            "win.minall",
            "全部最小化",
            "minall",
            "",
            "窗口",
            "minimize all windows",
            always,
            |a| a.minimize_all(),
        ),
        c(
            "win.hideall",
            "隐藏全部窗口",
            "hideall",
            "",
            "窗口",
            "hide all windows",
            always,
            |a| a.hide_all(),
        ),
    ]);
}
