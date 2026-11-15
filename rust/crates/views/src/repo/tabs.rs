//! The island of the tabs, at the top of the window (`repo.tabs`): a tab for each repository open, the one shown
//! marked, and **+**, which opens a folder or a recent workspace.

use ui::layout::{Area, Insets, inset};
use ui::{
    AccessRole, Button, ButtonKind, Description, Icon, Key, Label, MenuEntry, MenuItem, Modifiers, NamedKey, Rect,
    Sense, TextKind, Tone, Ui,
};

use crate::intent::Intent;
use crate::line::control_in;
use crate::text::header;
use crate::text::repo as words;

/// The widest a tab gets, in logical pixels.
pub const TAB_MOST: f32 = 200.0;

/// The key of the menu of **+**.
const OPEN_MENU: &str = "open menu";

/// What the island of the tabs shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabsState<'a> {
    /// The tabs, in their order.
    pub tabs: &'a [TabEntry],
    /// The recent workspaces, the latest first, each by its folders, for the menu of **+** (`repo.open-recent`).
    pub recent: &'a [String],
}

/// A tab, as the island draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabEntry {
    /// The name of its repository (`repo.name-repositories`).
    pub name: String,
    /// The folder of what it shows, for its tooltip.
    pub folder: String,
    /// Whether it is the tab shown.
    pub is_shown: bool,
}

/// What an entry of the menu of **+** does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenCommand {
    /// **Open folder…**.
    OpenFolder,
    /// Opens a recent workspace, by its place in the list.
    OpenRecent(usize),
}

/// The island of the tabs over the rectangle its caller clipped it to: each tab as wide as its name, up to
/// [`TAB_MOST`], a longer name cut at its start; the tab shown on the ground of a page with a border; **+** at the end.
/// `Ctrl+1` to `Ctrl+8` show the tab at that place and `Ctrl+9` the last, `Ctrl+PageDown` and `Ctrl+PageUp` the next
/// tab and the one before, going round.
///
/// Feature: repo.tabs
pub fn tabs_view(state: &TabsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.header_background);
    ui.region("tabs", (rect, AccessRole::Navigation, words::TABS), |ui| {
        let spacing = theme.sizes.spacing;
        let inner = inset(rect, Insets::symmetric(spacing.medium, 0.0));
        let mut area = Area::with_gap(inner, spacing.small);
        open_button(state.recent, ui, &mut area, &mut intents);
        for (index, tab) in state.tabs.iter().enumerate() {
            let style = ui.text_style(TextKind::Medium);
            let padding = 2.0 * spacing.large;
            let width = (ui.measure(&tab.name, style) + padding).min(TAB_MOST);
            if area.rest().width < width {
                break;
            }
            let cell = control_in(ui, area.left(width), width);
            if tab_button((index, tab), ui, (cell, padding)) {
                intents.push(Intent::SwitchTab(index));
            }
        }
    });
    intents.extend(keys(state.tabs, ui));
    intents
}

/// One tab, and whether it was clicked.
fn tab_button((index, tab): (usize, &TabEntry), ui: &mut Ui<'_>, (cell, padding): (Rect, f32)) -> bool {
    let theme = *ui.theme();
    let radius = theme.sizes.radius.medium;
    let id = ui.id(("tab", index, &tab.folder));
    let response = ui.interact(id, cell, Sense::CLICK);
    if tab.is_shown {
        ui.fill_rounded_rect(cell, radius, theme.colors.page_background);
        ui.border(
            cell,
            ui::Border {
                width: theme.sizes.border,
                radius,
                color: theme.colors.border,
            },
        );
    } else if response.hovered {
        ui.fill_rounded_rect(cell, radius, theme.colors.secondary_button_hover);
    }
    let room = cell.width - padding;
    let name = cut_at_start(ui, &tab.name, (room, TextKind::Medium));
    let tone = if tab.is_shown { Tone::Normal } else { Tone::Muted };
    let text = inset(cell, Insets::symmetric(padding / 2.0, 0.0));
    let _ = ui.label(text, &Label::new(&name).kind(TextKind::Medium).tone(tone));
    let description = Description::new(AccessRole::Button, &tab.name).selected(tab.is_shown);
    ui.describe(id, cell, &description);
    ui.tooltip(&response, &tab.folder);
    response.clicked
}

/// A name that does not fit cut at its start with `…`, so that its last part, the repository's own folder, stays.
#[must_use]
pub fn cut_at_start(ui: &Ui<'_>, name: &str, (room, kind): (f32, TextKind)) -> String {
    let style = ui.text_style(kind);
    if ui.measure(name, style) <= room {
        return name.to_owned();
    }
    let mut starts = name.char_indices().map(|(at, _)| at).skip(1);
    loop {
        let Some(at) = starts.next() else {
            return "\u{2026}".to_owned();
        };
        let cut = format!("\u{2026}{}", name.get(at..).unwrap_or_default());
        if ui.measure(&cut, style) <= room {
            return cut;
        }
    }
}

/// **+** at the end of the island, with its menu: **Open folder…** and the recent workspaces.
fn open_button(recent: &[String], ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    let button = Button::icon(Icon::Plus, words::OPEN_ANOTHER).kind(ButtonKind::Icon);
    let recent_entries: Vec<MenuEntry<'_, OpenCommand>> = recent
        .iter()
        .enumerate()
        .map(|(index, label)| MenuEntry::Item(MenuItem::new(label.as_str(), OpenCommand::OpenRecent(index))))
        .collect();
    let mut entries = vec![MenuEntry::Item(
        MenuItem::new(header::OPEN_FOLDER, OpenCommand::OpenFolder).with_icon(Icon::Folder),
    )];
    if !recent_entries.is_empty() {
        entries.push(MenuEntry::Submenu {
            label: header::OPEN_RECENT,
            icon: None,
            entries: &recent_entries,
        });
    }
    match ui.menu_button(OPEN_MENU, rect, &button, &entries) {
        Some(OpenCommand::OpenFolder) => intents.push(Intent::OpenFolder),
        Some(OpenCommand::OpenRecent(index)) => intents.push(Intent::OpenRecent(index)),
        None => {}
    }
}

/// The keys of the tabs, when nothing else took them.
fn keys(tabs: &[TabEntry], ui: &Ui<'_>) -> Vec<Intent> {
    let count = tabs.len();
    let shown = tabs.iter().position(|tab| tab.is_shown);
    if count < 2 {
        return Vec::new();
    }
    ui.unclaimed_keys()
        .into_iter()
        .filter(|key| key.modifiers == Modifiers::CONTROL)
        .filter_map(|key| match key.key {
            Key::Character(digit @ '1'..='8') => {
                let place = digit.to_digit(10).and_then(|place| usize::try_from(place).ok())?;
                (place <= count).then(|| Intent::SwitchTab(place - 1))
            }
            Key::Character('9') => Some(Intent::SwitchTab(count - 1)),
            Key::Named(NamedKey::PageDown) => Some(Intent::SwitchTab(shown.map_or(0, |shown| (shown + 1) % count))),
            Key::Named(NamedKey::PageUp) => {
                Some(Intent::SwitchTab(shown.map_or(0, |shown| (shown + count - 1) % count)))
            }
            _ => None,
        })
        .collect()
}
