//! The bottom panel (`panel.bottom`, D53): an island under the timeline and the inspector, or the page shown, above the
//! status bar, with the terminals and the Command log each in a tab; the gap above it is dragged to change its height.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Button, ButtonKind, Icon, Key, Label, Modifiers, Rect, Sense, Split, Splitter, TextKind, Tone, Ui,
};

use crate::intent::Intent;
use crate::log::{LogState, log_view};
use crate::text::panel as words;

/// The height the panel starts at, and where a double click on its gap puts it back.
pub const HEIGHT_START: f32 = 240.0;
/// The least height of the panel.
pub const HEIGHT_LEAST: f32 = 100.0;
/// The most of the height of the window the panel takes.
pub const HEIGHT_MOST: f32 = 0.7;
/// How far an arrow moves the gap above the panel.
const STEP: f32 = 16.0;

/// A tab of the bottom panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PanelTab {
    /// The terminals of the repository tab shown (`terminal.view`).
    #[default]
    Terminal,
    /// The Command log (`log.show`).
    CommandLog,
}

/// Something asked of the bottom panel.
#[derive(Clone, Debug, PartialEq)]
pub enum PanelIntent {
    /// `` Ctrl+` ``, or **Show Terminal** of Quick search: the panel shown at the terminal, with the keyboard in it.
    Open,
    /// The panel shown at a tab: a tab clicked, **Show Command Log**.
    Show(PanelTab),
    /// **Hide the panel**, or `` Ctrl+` `` while the panel has the keyboard: it is hidden and nothing stops.
    Hide,
    /// **Terminal** of the bar of views: the panel shown at the tab it showed last, or hidden.
    Toggle,
    /// The height dragged; none when a double click puts it back where it starts.
    SetHeight(Option<f32>),
}

/// What the bottom panel shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BottomPanelState<'a> {
    /// The tab shown.
    pub tab: PanelTab,
    /// The height dragged; none while it is where it starts.
    pub height: Option<f32>,
    /// The Command log.
    pub log: LogState<'a>,
    /// A number that changes each time the keyboard is to go to the terminal.
    pub focus: u64,
}

/// The height of the panel inside a column of a height, in a window of a height: as dragged, never less than its least
/// nor more than its most of the window, and leaving the column its least too.
#[must_use]
pub fn panel_height(height: Option<f32>, column: f32, window: f32) -> f32 {
    let most = (window * HEIGHT_MOST).max(HEIGHT_LEAST);
    height
        .unwrap_or(HEIGHT_START)
        .clamp(HEIGHT_LEAST, most)
        .min((column - HEIGHT_LEAST).max(0.0))
}

/// A column cut for the panel: what is above it and the panel itself, with a gap between them.
#[must_use]
pub fn split_column(column: Rect, panel: f32, gap: f32) -> (Rect, Rect) {
    let above = Rect::new(column.x, column.y, column.width, (column.height - panel - gap).max(0.0));
    let below = Rect::new(column.x, column.bottom() - panel, column.width, panel);
    (above, below)
}

/// The gap above the panel, over the column the panel shares with what is above it: dragged, or moved with the
/// arrows, it says the new height; a double click puts the panel back at its start.
pub(crate) fn resize_panel_view(
    state: &BottomPanelState<'_>,
    (column, gap): (Rect, f32),
    ui: &mut Ui<'_>,
) -> Vec<Intent> {
    let window = ui.window().height;
    let height = panel_height(state.height, column.height, window);
    let most = (window * HEIGHT_MOST).max(HEIGHT_LEAST);
    let splitter = Splitter {
        split: Split::Rows,
        default_size: (column.height - gap - HEIGHT_START).max(0.0),
        least_first: (column.height - gap - most).max(0.0),
        least_second: HEIGHT_LEAST,
        size: Some((column.height - gap - height).max(0.0)),
        step: STEP,
    };
    let response = ui.splitter_across("bottom panel line", column, &splitter, gap);
    if response.is_reset {
        vec![Intent::Panel(PanelIntent::SetHeight(None))]
    } else if response.is_changed {
        let dragged = (column.height - gap - response.size).max(0.0);
        vec![Intent::Panel(PanelIntent::SetHeight(Some(dragged)))]
    } else {
        Vec::new()
    }
}

/// The bottom panel over the rectangle its caller clipped it to: its tabs on the left of its top, **Hide the panel** on
/// the right, and under them the terminal or the Command log.
///
/// Feature: panel.bottom
pub fn bottom_view(state: &BottomPanelState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    ui.region("bottom panel", (rect, AccessRole::Region, words::PANEL), |ui| {
        let mut area = Area::new(inset(rect, Insets::symmetric(spacing.medium, spacing.small)));
        let bar = area.top(theme.sizes.control_height);
        top_bar(state, ui, bar, &mut intents);
        let body = inset(area.rest(), Insets::symmetric(spacing.small, spacing.small));
        match state.tab {
            PanelTab::Terminal => terminal_tab(state, ui, body),
            PanelTab::CommandLog => intents.extend(ui.clip(body, |ui| log_view(&state.log, ui))),
        }
    });
    intents
}

/// The tabs, and the buttons on the right of the top of the panel.
fn top_bar(state: &BottomPanelState<'_>, ui: &mut Ui<'_>, bar: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let mut parts = Area::with_gap(bar, theme.sizes.spacing.small);
    let hide = Button::icon(Icon::ChevronDown, words::HIDE_PANEL).kind(ButtonKind::Icon);
    if ui.button("hide the panel", parts.right(bar.height), &hide).clicked {
        intents.push(Intent::Panel(PanelIntent::Hide));
    }
    for (tab, name) in [
        (PanelTab::Terminal, words::TERMINAL),
        (PanelTab::CommandLog, words::COMMAND_LOG),
    ] {
        let kind = if tab == state.tab {
            ButtonKind::Secondary
        } else {
            ButtonKind::Icon
        };
        let button = Button::secondary(name).kind(kind);
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("panel tab", name), cell, &button).clicked && tab != state.tab {
            intents.push(Intent::Panel(PanelIntent::Show(tab)));
        }
    }
}

/// The Terminal tab, until the terminals come (`terminal.view`): what it will hold, taking the keyboard when it is asked
/// to.
fn terminal_tab(state: &BottomPanelState<'_>, ui: &mut Ui<'_>, body: Rect) {
    let id = ui.id("terminal");
    if state.focus > 0 {
        ui.request_focus_once(id, state.focus);
    }
    let _ = ui.interact(id, body, Sense::CLICK | Sense::FOCUS);
    let height = ui.theme().sizes.row_height;
    let label = Label::new(words::NO_REPOSITORY)
        .kind(TextKind::Detail)
        .tone(Tone::Muted);
    let _ = ui.label(Rect::new(body.x, body.y, body.width, height), &label);
}

/// What `` Ctrl+` `` (`` Cmd+` `` on macOS) asks when it was pressed in this frame and no widget used it: the panel
/// hidden when it is shown, at a rectangle, and has the keyboard; otherwise shown at the terminal with the keyboard.
#[must_use]
pub(crate) fn panel_shortcut(ui: &Ui<'_>, shown: Option<Rect>) -> Option<Intent> {
    if !ui
        .unclaimed_keys()
        .iter()
        .any(|key| is_panel_key(key.key, key.modifiers))
    {
        return None;
    }
    Some(Intent::Panel(match shown {
        Some(rect) if ui.has_focus_within(rect) => PanelIntent::Hide,
        _ => PanelIntent::Open,
    }))
}

/// Whether a key is `` Ctrl+` ``, or `` Cmd+` `` on macOS.
#[must_use]
pub fn is_panel_key(key: Key, modifiers: Modifiers) -> bool {
    let command = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    key == Key::Character('`') && (modifiers == command || modifiers == Modifiers::CONTROL)
}
