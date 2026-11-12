//! The left pane of the gallery: boxes of buttons, of choices, of input boxes, and of progress.

use render::{Icon, Rect, Theme};

use super::{Field, GalleryIntent, GalleryState, Switch};
use crate::frame::{TextKind, Ui};
use crate::layout::{Align, Area, Insets, Size, columns, inset, place};
use crate::widget::Tone;
use crate::widget::button::Button;
use crate::widget::choice::Choice;
use crate::widget::input_box::{InputBox, Message, Severity};
use crate::widget::label::Label;
use crate::widget::metrics;

/// The height of a box of the gallery with a title and rows of controls, and room for more below its last row.
fn panel_height(theme: &Theme, rows: u16, extra: f32) -> f32 {
    let sizes = theme.sizes;
    let control = metrics::control_height(theme);
    let rows = f32::from(rows);
    let title = sizes.row_height + sizes.spacing.medium + sizes.border;
    title
        + 2.0 * sizes.border
        + 2.0 * sizes.spacing.large
        + rows * control
        + (rows - 1.0) * sizes.spacing.medium
        + extra
}

/// The rows of controls inside a box, each as high as a control.
fn control_rows(theme: &Theme, inside: Rect) -> Area {
    Area::with_gap(inside, theme.sizes.spacing.medium)
}

/// Declares the boxes of the left pane.
pub(super) fn controls(state: &GalleryState, ui: &mut Ui<'_>, pane: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::with_gap(inset(pane, Insets::all(spacing.large)), spacing.large);
    let buttons = area.top(panel_height(&theme, 2, 0.0));
    buttons_panel(ui, buttons, intents);
    let choices = area.top(panel_height(&theme, 3, 0.0));
    choices_panel(state, ui, choices, intents);
    let message_room = theme.sizes.row_height + spacing.medium;
    let inputs = area.top(panel_height(&theme, 4, message_room));
    inputs_panel(state, ui, inputs, intents);
    let progress = area.top(panel_height(&theme, 1, 0.0));
    progress_panel(state, ui, progress);
}

/// Buttons of each kind.
fn buttons_panel(ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let inside = ui.panel(rect, Some("Buttons"));
    let mut rows = control_rows(&theme, inside);
    let control = metrics::control_height(&theme);
    let first = [
        Button::primary("Create branch"),
        Button::secondary("Cancel"),
        Button::secondary("Delete").disabled(),
    ];
    let second = [
        Button::secondary("Compare").with_icon(Icon::Branch),
        Button::icon(Icon::Plus, "Add a remote"),
        Button::icon(Icon::Search, "Search the commits"),
        Button::icon(Icon::Close, "Close the page"),
    ];
    for buttons in [&first[..], &second[..]] {
        let mut row = Area::with_gap(rows.top(control), theme.sizes.spacing.medium);
        for button in buttons {
            let (width, _) = ui.button_size(button);
            let key = button.tooltip.unwrap_or(button.label);
            if ui.button(key, row.left(width), button).clicked {
                intents.push(GalleryIntent::Clicked(key.to_owned()));
            }
        }
    }
}

/// Check boxes, radio buttons and toggle switches.
fn choices_panel(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let inside = ui.panel(rect, Some("Choices"));
    let mut rows = control_rows(&theme, inside);
    let control = metrics::control_height(&theme);
    let gap = theme.sizes.spacing.large;
    let [fetch, prune] = columns(rows.top(control), gap, [Size::Fill(1.0), Size::Fill(1.0)]);
    if ui
        .choice(
            "fetch first",
            fetch,
            &Choice::check_box("Fetch before pulling", state.fetch_first),
        )
        .clicked
    {
        intents.push(GalleryIntent::Toggled(Switch::FetchFirst));
    }
    if ui
        .choice("prune", prune, &Choice::check_box("Prune remote branches", state.prune))
        .clicked
    {
        intents.push(GalleryIntent::Toggled(Switch::Prune));
    }
    let radios = columns(
        rows.top(control),
        gap,
        [Size::Fill(1.0), Size::Fill(1.0), Size::Fill(1.2)],
    );
    for (index, (rect, label)) in radios.into_iter().zip(["Merge", "Rebase", "Fast-forward"]).enumerate() {
        if ui
            .choice(
                ("strategy", index),
                rect,
                &Choice::radio(label, state.strategy == index),
            )
            .clicked
        {
            intents.push(GalleryIntent::Chose(index));
        }
    }
    let [remotes, locked] = columns(rows.top(control), gap, [Size::Fill(1.0), Size::Fill(1.0)]);
    if ui
        .choice(
            "remotes",
            remotes,
            &Choice::toggle("Show remote branches", state.show_remotes),
        )
        .clicked
    {
        intents.push(GalleryIntent::Toggled(Switch::ShowRemotes));
    }
    let _ = ui.choice("signed", locked, &Choice::check_box("Sign commits", false).disabled());
}

/// Input boxes: one with the focus and a word selected, an empty one with a placeholder, and one with an error.
fn inputs_panel(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let inside = ui.panel(rect, Some("Input boxes"));
    let mut rows = control_rows(&theme, inside);
    let control = metrics::control_height(&theme);
    let label = rows.top(theme.sizes.row_height);
    let _ = ui.label(label, &Label::new("Name of the new branch").tone(Tone::Muted));
    let boxes = [
        (
            Field::BranchName,
            "branch name",
            state.branch_name.as_str(),
            "Branch name",
            None,
        ),
        (
            Field::Search,
            "search",
            state.search.as_str(),
            "Search the commits",
            None,
        ),
        (
            Field::Existing,
            "existing",
            state.existing.as_str(),
            "Branch name",
            Some(Message {
                text: "A branch named main already exists.",
                severity: Severity::Error,
            }),
        ),
    ];
    for (field, key, text, placeholder, message) in boxes {
        let input = InputBox {
            text,
            placeholder,
            message,
        };
        let answer = ui.input_box(key, rows.top(control), &input);
        if let Some(text) = answer.changed_text {
            intents.push(GalleryIntent::Typed(field, text));
        }
    }
}

/// A progress bar with its percentage, and a spinner with what it waits for.
fn progress_panel(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let inside = ui.panel(rect, Some("Progress"));
    let control = metrics::control_height(&theme);
    let row = place(inside, (inside.width, control), (Align::Start, Align::Start));
    let [bar, percent, spinner, waiting] = columns(
        row,
        theme.sizes.spacing.medium,
        [
            Size::Fill(1.0),
            Size::Fixed(theme.sizes.row_height * 2.0),
            Size::Fixed(theme.sizes.icon),
            Size::Fill(1.0),
        ],
    );
    ui.progress_bar(bar, state.progress);
    let shown = format!("{:.0} %", state.progress * 100.0);
    let _ = ui.label(percent, &Label::new(&shown).kind(TextKind::Small).tone(Tone::Muted));
    ui.spinner(spinner);
    let _ = ui.label(waiting, &Label::new("Fetching origin").tone(Tone::Muted));
}
