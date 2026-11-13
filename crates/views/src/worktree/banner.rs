//! `worktree.banner`: says which worktree or submodule the views show while it is not the folder of the window, and
//! goes back.

use std::path::Path;

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Button, ButtonKind, Icon, Label, Rect, Sense, Tone, Ui};

use crate::intent::Intent;
use crate::text::worktree as words;

/// What the banner of a worktree or a submodule shown in this window shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerState<'a> {
    /// How the views name the worktree shown, or the folder of the submodule.
    pub label: String,
    /// Whether it is a submodule of the window's repository (`submodule.show-in-gitardo`) rather than one of its
    /// worktrees.
    pub is_submodule: bool,
    /// Its branch, by its short name; none when detached.
    pub branch: Option<String>,
    /// Its folder.
    pub folder: &'a Path,
    /// The name of the window's repository.
    pub window_name: String,
    /// The folder of the window.
    pub window_folder: &'a Path,
    /// Whether an operation runs: the button is disabled.
    pub is_running: bool,
}

/// The banner over the rectangle its caller clipped it to, above the commits and above the lists of the Changes page:
/// **Showing the worktree `<label>` on `<branch>`**, its folder as the tooltip, and **Back to `<name>`**, which points
/// the views at the window's repository again.
///
/// Feature: worktree.banner
pub fn banner(state: &BannerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    let says = words::showing(&state.label, state.branch.as_deref(), state.is_submodule);
    let mut intents = Vec::new();
    ui.region("worktree banner", (rect, AccessRole::Group, &says), |ui| {
        let mut parts = Area::with_gap(
            inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
            spacing.medium,
        );
        let icon = if state.is_submodule {
            Icon::Folder
        } else {
            Icon::Worktree
        };
        ui.icon_in(parts.left(theme.sizes.icon), icon, Tone::Accent);
        let back = words::back_to(&state.window_name);
        let button = Button::secondary(&back).kind(ButtonKind::Outline);
        let button = if state.is_running { button.disabled() } else { button };
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
        let response = ui.button("worktree back", cell, &button);
        ui.tooltip(&response, &state.window_folder.display().to_string());
        if response.clicked {
            intents.push(Intent::SelectRepository(state.window_folder.to_path_buf()));
        }
        let words_rect = parts.rest();
        let _ = ui.label(words_rect, &Label::new(&says));
        let id = ui.id("worktree banner words");
        let pointed = ui.interact(id, words_rect, Sense::HOVER);
        ui.tooltip(&pointed, &state.folder.display().to_string());
    });
    intents
}
