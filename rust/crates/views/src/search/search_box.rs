//! The search box at the top of the timeline (`search.box`): the list of its modes, the box, what it found at its end,
//! and the funnel; with its keys.

use model::search::{BoxMode, Direction};
use ui::layout::{Area, Insets, inset};
use ui::{
    Button, ButtonKind, InputBox, Key, KeyEvent, MenuEntry, MenuItem, Modifiers, NamedKey, Rect, Severity, Ui, WidgetId,
};

use super::SearchIntent;
use super::go_to_hash::{go_to_hash_view, go_to_hash_width};
use super::history::{HistorySearch, history_view, history_width};
use crate::icons;
use crate::intent::Intent;
use crate::line::control_in;
use crate::text::search as words;

/// The key of the box.
const BOX: &str = "search box";
/// The key of the list of the modes.
const MODES: &str = "search modes";
/// The widest the box and what is around it are.
const WIDEST: f32 = 640.0;

/// What the search box shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchBox<'a> {
    /// The mode chosen.
    pub mode: BoxMode,
    /// What is in the box.
    pub text: &'a str,
    /// What the search found, said at the end of the box.
    pub found: HistorySearch,
    /// Whether the text is a hash that no loaded commit has, which `Enter` goes to (`search.go-to-hash`).
    pub is_hash_offered: bool,
    /// Whether the funnel is pressed; none in the mode **Loaded**, where it is not there (`search.list-mode`).
    pub funnel: Option<bool>,
    /// What is said under the box: a search git refused, or a hash that is no commit.
    pub message: Option<&'a str>,
}

/// The entries of the list of the modes, in their order, the one chosen ticked.
#[must_use]
pub fn mode_entries(chosen: BoxMode) -> Vec<MenuEntry<'static, BoxMode>> {
    BoxMode::ALL
        .into_iter()
        .map(|mode| {
            let item = MenuItem::new(words::mode_name(mode), mode);
            MenuEntry::Item(if mode == chosen {
                item.with_icon(icons::CHOSEN)
            } else {
                item
            })
        })
        .collect()
}

/// The search box over the rectangle its caller clipped it to: the list of the modes, the box with its count or
/// `Enter ↵` and the buttons that step through the matches, and the funnel. `Ctrl+F` anywhere in the timeline puts the
/// keyboard in the box with its text selected; `F3` and `Shift+F3` step through the matches from anywhere.
///
/// Feature: search.box
pub fn box_view(state: &SearchBox<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let cell = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let width = cell.width.min(WIDEST);
    let row = Rect::new(cell.x, cell.y, width, cell.height);
    let mut area = Area::with_gap(row, spacing.medium);
    let mut intents = Vec::new();
    let input = ui.id(BOX);
    intents.extend(window_keys(ui, input));
    mode_list(state.mode, ui, &mut area, &mut intents);
    if let Some(is_pressed) = state.funnel {
        funnel(is_pressed, ui, &mut area, &mut intents);
    }
    let side = theme.sizes.control_height;
    let end_width = if state.is_hash_offered {
        go_to_hash_width(ui)
    } else {
        history_width(&state.found, ui)
    };
    let box_rect = control_in(ui, area.rest(), area.rest().width);
    let end = Rect::new(
        box_rect.right() - end_width - spacing.small,
        box_rect.y,
        end_width,
        box_rect.height,
    );
    let mut input_box = InputBox::new(state.text, words::placeholder(state.mode)).with_icon(icons::SEARCH);
    if let Some(message) = state.message {
        input_box = input_box.with_message(ui::Message {
            text: message,
            severity: Severity::Error,
        });
    }
    let typed = ui.input_box(BOX, box_rect, &input_box);
    if let Some(text) = typed.changed_text {
        intents.push(Intent::Search(SearchIntent::Type(text)));
    }
    intents.extend(box_keys(ui, input));
    let inner = inset(end, Insets::symmetric(0.0, (box_rect.height - side).max(0.0) * 0.5));
    let end_intents = if state.is_hash_offered {
        ui.clip(inner, |ui| go_to_hash_view(state, ui))
    } else {
        ui.clip(inner, |ui| history_view(&state.found, ui))
    };
    if !end_intents.is_empty() {
        ui.request_focus(input);
    }
    intents.extend(end_intents);
    intents
}

/// The list before the box, which says the mode chosen and what it runs, and chooses another.
fn mode_list(chosen: BoxMode, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let button = Button::secondary(words::mode_name(chosen))
        .with_trailing(icons::CHOOSE)
        .with_tooltip(words::mode_tooltip(chosen));
    let widest = BoxMode::ALL
        .into_iter()
        .map(|mode| {
            ui.button_size(&Button::secondary(words::mode_name(mode)).with_trailing(icons::CHOOSE))
                .0
        })
        .fold(0.0, f32::max);
    let rect = control_in(ui, area.left(widest), widest);
    if let Some(mode) = ui.menu_button(MODES, rect, &button, &mode_entries(chosen))
        && mode != chosen
    {
        intents.push(Intent::Search(SearchIntent::Mode(mode)));
    }
}

/// The funnel at the end, pressed while the search is shown as a list.
fn funnel(is_pressed: bool, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    let kind = if is_pressed {
        ButtonKind::Secondary
    } else {
        ButtonKind::Icon
    };
    let button = Button::icon(icons::FILTER, words::FUNNEL).kind(kind);
    if ui.button("search funnel", rect, &button).clicked {
        intents.push(Intent::Search(SearchIntent::ToggleList));
    }
}

/// The keys the box leaves to its view: `Enter` asks or steps, `Shift+Enter` and the arrows step, `Escape` empties it.
fn box_keys(ui: &Ui<'_>, input: WidgetId) -> Vec<Intent> {
    ui.unclaimed_keys_of(input)
        .into_iter()
        .filter_map(|key| {
            let intent = match (key.key, key.modifiers) {
                (Key::Named(NamedKey::Enter), Modifiers::NONE) => SearchIntent::Submit,
                (Key::Named(NamedKey::Enter), Modifiers::SHIFT) | (Key::Named(NamedKey::ArrowUp), Modifiers::NONE) => {
                    SearchIntent::Step(Direction::Previous)
                }
                (Key::Named(NamedKey::ArrowDown), Modifiers::NONE) => SearchIntent::Step(Direction::Next),
                (Key::Named(NamedKey::Escape), Modifiers::NONE) => SearchIntent::Empty,
                _ => return None,
            };
            Some(Intent::Search(intent))
        })
        .collect()
}

/// The keys of the search from anywhere in the timeline: `F3` and `Shift+F3` step; `Ctrl+F` (`Cmd+F` on macOS) puts
/// the keyboard in the box with its text selected.
fn window_keys(ui: &mut Ui<'_>, input: WidgetId) -> Vec<Intent> {
    let mut intents = Vec::new();
    for key in ui.unclaimed_keys() {
        match (key.key, key.modifiers) {
            (Key::Named(NamedKey::Function(3)), Modifiers::NONE) => {
                intents.push(Intent::Search(SearchIntent::Step(Direction::Next)));
            }
            (Key::Named(NamedKey::Function(3)), Modifiers::SHIFT) => {
                intents.push(Intent::Search(SearchIntent::Step(Direction::Previous)));
            }
            _ if is_find_key(key) => ui.focus_input_box(input),
            _ => {}
        }
    }
    intents
}

/// Whether a key is `Ctrl+F`, or `Cmd+F` on macOS.
fn is_find_key(key: KeyEvent) -> bool {
    matches!(key.key, Key::Character('f' | 'F')) && key.modifiers.command() && !key.modifiers.shift()
}
