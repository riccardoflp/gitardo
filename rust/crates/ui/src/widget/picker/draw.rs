//! Drawing a picker: its box, its input box with the button that takes the items checked, and the rows that match,
//! each with its check box, its label with the matched characters in bold, and its description on the right.

use render::{Border, Color, Point, Rect, TextRun};

use crate::access::{AccessRole, Description};

use super::fuzzy::FuzzyMatch;
use super::{Picker, PickerAnswer, PickerItem, PickerMode, PickerState, SHOWN_ROWS, ranked, toggle};
use crate::frame::{TextKind, Ui};
use crate::interaction::{Sense, WidgetEvent};
use crate::layout::{Align, Area, Insets, inset, place};
use crate::widget::Tone;
use crate::widget::button::Button;
use crate::widget::input_box::InputBox;
use crate::widget::label::Label;
use crate::widget::metrics;
use crate::widget::panel::Axis;
use crate::widget::scroll::ScrollAxis;
use crate::widget::scroll::bar::BarSpec;

/// A row of a picker being drawn.
struct PickerRow<'item, 'text> {
    /// Where it is.
    rect: Rect,
    /// The item.
    item: &'item PickerItem<'text>,
    /// How the item matched what is typed.
    found: &'item FuzzyMatch,
    /// Whether it is lit.
    is_lit: bool,
    /// Whether it is checked, in a picker of several.
    is_checked: Option<bool>,
}

impl Ui<'_> {
    /// Draws a picker in its rectangle, inside its scope of names, and returns what a click answered.
    pub(super) fn draw_picker(
        &mut self,
        rect: Rect,
        picker: &Picker<'_>,
        state: &mut PickerState,
    ) -> Option<PickerAnswer> {
        let theme = *self.theme();
        let spacing = theme.sizes.spacing;
        let box_id = self.id("box");
        let _ = self.interact(box_id, rect, Sense::CLICK);
        self.floating_card(rect, theme.sizes.radius.large + spacing.small);
        self.access_node(box_id, rect, &Description::new(AccessRole::Dialog, picker.placeholder));
        self.access_enter(box_id);
        let mut area = Area::with_gap(inset(rect, Insets::all(spacing.medium)), spacing.small);
        let mut top = Area::with_gap(area.top(metrics::control_height(&theme)), spacing.small);
        let mut answer = None;
        if picker.mode == PickerMode::Multiple {
            let ok = Button::primary("OK");
            let (width, _) = self.button_size(&ok);
            if self.button("ok", top.right(width), &ok).clicked {
                answer = Some(PickerAnswer::ChoseMany(state.checked.iter().copied().collect()));
            }
        }
        let query = state.query.clone();
        let input = InputBox::new(&query, picker.placeholder).with_icon(render::Icon::Search);
        let _ = self.input_box("query", top.rest(), &input);
        let rows = self.picker_rows(area.rest(), picker, state);
        self.access_leave();
        answer.or(rows)
    }

    /// The rows that match, the lit one lit, moved by the wheel and the pointer, and what a click on one answered.
    fn picker_rows(&mut self, rect: Rect, picker: &Picker<'_>, state: &mut PickerState) -> Option<PickerAnswer> {
        let row_height = self.theme().sizes.row_height;
        let matches = ranked(picker.items, &state.query, state.has_last);
        if matches.is_empty() {
            let label = Label::new("No matching items").tone(Tone::Muted);
            let _ = self.label(Rect::new(rect.x, rect.y, rect.width, row_height), &label);
            return None;
        }
        let (_, events) = self.interact_with_events(self.id("rows"), rect, Sense::SCROLL);
        for event in events {
            if let WidgetEvent::Wheel { delta } = event {
                let rows = (delta.y / row_height).round();
                let largest = matches.len().saturating_sub(SHOWN_ROWS);
                let step = if rows < 0.0 { -1 } else { 1 };
                let count = crate::number::index_at(f64::from(rows.abs()), 1.0).max(1);
                for _ in 0..count {
                    state.first_shown = state.first_shown.saturating_add_signed(step).min(largest);
                }
            }
        }
        let rect = self.picker_bar(rect, matches.len(), state);
        let mut answer = None;
        let shown = matches.iter().enumerate().skip(state.first_shown).take(SHOWN_ROWS);
        for (offset, (place, (item_index, found))) in shown.enumerate() {
            let Some(item) = picker.items.get(*item_index) else {
                continue;
            };
            let row_rect = Rect::new(
                rect.x,
                rect.y + crate::number::float_of_count(offset) * row_height,
                rect.width,
                row_height,
            );
            let response = self.interact(self.id(("row", *item_index)), row_rect, Sense::CLICK);
            if response.hovered && self.pointer_moved() {
                state.lit = place;
            }
            if response.clicked {
                match picker.mode {
                    PickerMode::Single => answer = Some(PickerAnswer::Chose(*item_index)),
                    PickerMode::Multiple => toggle(&mut state.checked, *item_index),
                }
            }
            let row = PickerRow {
                rect: row_rect,
                item,
                found,
                is_lit: state.lit == place,
                is_checked: (picker.mode == PickerMode::Multiple).then(|| state.checked.contains(item_index)),
            };
            self.picker_row(&row);
            let mut description = Description::new(AccessRole::ListItem, item.label).selected(row.is_lit);
            if let Some(is_checked) = row.is_checked {
                description = description.on(is_checked);
            }
            self.access_node(response.id, row_rect, &description);
        }
        answer
    }

    /// Draws the bar of a picker with more rows than it shows, on the right of its rows, and moves the first row shown
    /// as it is dragged or its track clicked. Returns where the rows go, left of the bar.
    fn picker_bar(&mut self, rect: Rect, count: usize, state: &mut PickerState) -> Rect {
        if count <= SHOWN_ROWS {
            return rect;
        }
        let row_height = f64::from(self.theme().sizes.row_height);
        let thickness = self.theme().sizes.scrollbar;
        let track = Rect::new(rect.right() - thickness, rect.y, thickness, rect.height);
        let spec = BarSpec {
            axis: Axis::Vertical,
            view: crate::number::wide_of_count(SHOWN_ROWS) * row_height,
            content: crate::number::wide_of_count(count) * row_height,
        };
        let mut scroll = ScrollAxis::default();
        scroll.jump(crate::number::wide_of_count(state.first_shown) * row_height);
        self.scroll_bar(self.id("bar"), track, spec, &mut scroll);
        let first = crate::number::index_at(scroll.target() + row_height * 0.5, row_height);
        state.first_shown = first.min(count - SHOWN_ROWS);
        Rect::new(rect.x, rect.y, rect.width - thickness, rect.height)
    }

    /// Draws a row of a picker.
    fn picker_row(&mut self, row: &PickerRow<'_, '_>) {
        let theme = *self.theme();
        let colors = theme.colors;
        let spacing = theme.sizes.spacing;
        if row.is_lit {
            self.fill_rounded_rect(row.rect, theme.sizes.radius.medium, colors.menu_selection);
        }
        let (color, matched, muted) = if row.is_lit {
            let lit = colors.menu_selection_foreground;
            (lit, lit, lit)
        } else {
            (colors.menu_foreground, colors.accent, colors.muted_foreground)
        };
        let mut area = Area::with_gap(inset(row.rect, Insets::symmetric(spacing.medium, 0.0)), spacing.medium);
        if let Some(is_checked) = row.is_checked {
            let side = metrics::mark_side(&theme);
            let mark = place(area.left(side), (side, side), (Align::Start, Align::Center));
            self.picker_check(mark, is_checked);
        }
        let style = self.text_style(TextKind::Body);
        if !row.item.description.is_empty() {
            let width = self.measure(row.item.description, style).min(row.rect.width * 0.4);
            let description = area.right(width);
            let line = place(
                description,
                (width, self.line_height(style)),
                (Align::End, Align::Center),
            );
            let run = TextRun {
                text: row.item.description,
                style,
                color: muted,
                origin: Point::new(line.x, line.y),
                max_width: Some(width),
            };
            let _ = self.text(&run);
        }
        let text = area.rest();
        self.clip(text, |ui| {
            ui.matched_label(text, row.item.label, &row.found.positions, (color, matched));
        });
    }

    /// The check box of a row, drawn without taking input: the row takes the click.
    fn picker_check(&mut self, mark: Rect, is_checked: bool) {
        let theme = *self.theme();
        let radius = theme.sizes.radius.small;
        if is_checked {
            self.fill_rounded_rect(mark, radius, theme.colors.button_background);
            self.icon(
                render::Icon::Check,
                Point::new(mark.x, mark.y),
                mark.height,
                theme.colors.button_foreground,
            );
        } else {
            let border = Border {
                width: theme.sizes.border,
                radius,
                color: theme.colors.input_border,
            };
            self.fill_rounded_rect(mark, radius, theme.colors.input_background);
            self.border(mark, border);
        }
    }

    /// Draws a label whose matched characters are in bold and in another color, piece by piece.
    fn matched_label(&mut self, rect: Rect, label: &str, positions: &[usize], (color, matched): (Color, Color)) {
        let plain = self.text_style(TextKind::Body);
        let strong = self.text_style(TextKind::Strong);
        let line = place(
            rect,
            (rect.width, self.line_height(plain)),
            (Align::Start, Align::Center),
        );
        let mut x = line.x;
        let mut start = 0;
        let mut characters = label.char_indices().peekable();
        while let Some((at, _)) = characters.next() {
            let is_matched = positions.contains(&at);
            let next = characters.peek().map_or(label.len(), |(next, _)| *next);
            let continues = characters
                .peek()
                .is_some_and(|(next, _)| positions.contains(next) == is_matched);
            if continues {
                continue;
            }
            let piece = label.get(start..next).unwrap_or_default();
            let (style, piece_color) = if is_matched { (strong, matched) } else { (plain, color) };
            let run = TextRun {
                text: piece,
                style,
                color: piece_color,
                origin: Point::new(x, line.y),
                max_width: None,
            };
            x += self.text(&run);
            start = next;
            if x > rect.right() {
                break;
            }
        }
    }
}
