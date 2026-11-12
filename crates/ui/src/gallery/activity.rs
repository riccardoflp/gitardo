//! The Activity drawer: a card over the bottom of the timeline and the inspector with what is running and how far it is, the command
//! log, and the tasks that wait. Escape or its button closes it.

use render::{Icon, Point, Rect, TextRun};

use super::{GalleryIntent, GalleryState};
use crate::access::AccessRole;
use crate::frame::{TextKind, Ui};
use crate::interaction::Sense;
use crate::layer::Layer;
use crate::layout::{Area, Insets, Size, columns, inset};
use crate::widget::Tone;
use crate::widget::button::{Button, ButtonKind};
use crate::widget::label::Label;
use crate::widget::metrics;

/// The highest the drawer is.
const HEIGHT: f32 = 280.0;
/// How many lines the command log has.
const LOG_LINES: usize = 40;
/// The commands of the log, in turn.
const COMMANDS: [&str; 4] = [
    "git fetch --prune origin",
    "git switch --create feature/login-page origin/main",
    "git commit --file - --cleanup=strip",
    "git push --set-upstream origin feature/login-page --force-with-lease",
];

/// Declares the drawer over the bottom of what is right of the branch rail.
pub(super) fn drawer(state: &GalleryState, ui: &mut Ui<'_>, main: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let margin = spacing.large + spacing.small;
    let height = HEIGHT.min(main.height * 0.6);
    let rect = Rect::new(
        main.x + margin,
        main.bottom() - margin - height,
        (main.width - 2.0 * margin).max(0.0),
        height,
    );
    ui.layer(Layer::Popup, |ui| {
        ui.floating_card(rect, theme.sizes.radius.large + spacing.small);
        let blocker = ui.id("activity drawer");
        let _ = ui.interact(blocker, rect, Sense::CLICK);
        ui.region("activity region", (rect, AccessRole::Region, "Activity"), |ui| {
            let mut area = Area::with_gap(inset(rect, Insets::all(margin)), spacing.large);
            title_bar(ui, area.top(metrics::control_height(&theme)), intents);
            let [running, log, tasks] =
                columns(area.rest(), margin, [Size::Fill(1.0), Size::Fill(1.4), Size::Fill(1.0)]);
            what_is_running(state, ui, running);
            command_log(ui, log);
            waiting_tasks(ui, tasks, intents);
        });
    });
}

/// The title of the drawer and the button that closes it.
fn title_bar(ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let side = rect.height;
    let mut area = Area::new(rect);
    let close = Button::icon(Icon::Close, "Close the activity").kind(ButtonKind::Icon);
    if ui.button("close activity", area.right(side), &close).clicked {
        intents.push(GalleryIntent::CloseActivity);
    }
    let _ = ui.label(area.rest(), &Label::new("Activity").kind(TextKind::Strong));
}

/// The title of a column of the drawer.
fn column_title(ui: &mut Ui<'_>, area: &mut Area, text: &str) {
    let height = ui.theme().sizes.row_height;
    let words = Label::new(text).kind(TextKind::SmallStrong).tone(Tone::Muted);
    let _ = ui.label(area.top(height), &words);
}

/// What is running: a spinner with its words, a progress bar, and how far it is; or what was done last.
fn what_is_running(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::with_gap(rect, spacing.medium);
    column_title(ui, &mut area, "WHAT IS RUNNING");
    let mut line = Area::with_gap(area.top(theme.sizes.row_height), spacing.medium);
    let mark = line.left(theme.sizes.icon);
    if let Some(status) = &state.status {
        ui.icon_in(mark, Icon::Check, Tone::Success);
        let _ = ui.label(line.rest(), &Label::new(status));
        return;
    }
    ui.spinner(mark);
    let _ = ui.label(line.rest(), &Label::new("Fetching origin"));
    let progress = 0.62;
    ui.progress_bar(area.top(spacing.large), progress);
    let words = format!("Receiving objects, {:.0} %", progress * 100.0);
    let detail = Label::new(&words).kind(TextKind::Detail).tone(Tone::Muted);
    let _ = ui.label(area.top(theme.sizes.row_height), &detail);
}

/// The command log: the commands that ran, in a scroll area longer and wider than its box.
fn command_log(ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    column_title(ui, &mut area, "COMMAND LOG");
    let box_rect = area.rest();
    ui.fill_rounded_rect(
        box_rect,
        theme.sizes.radius.large,
        theme.colors.secondary_button_background,
    );
    let inside = inset(box_rect, Insets::all(theme.sizes.spacing.medium));
    let style = ui.text_style(TextKind::Code);
    let line = ui.line_height(style);
    let content_height = crate::number::float_of_count(LOG_LINES) * line;
    let content_width = inside.width * 1.6;
    let _ = ui.scroll_area("log", inside, (content_width, content_height), |ui, content| {
        for index in 0..LOG_LINES {
            let y = content.y + crate::number::float_of_count(index) * line;
            if y + line < inside.y || y > inside.bottom() {
                continue;
            }
            let command = COMMANDS.get(index % COMMANDS.len()).copied().unwrap_or_default();
            log_line(ui, Point::new(content.x, y), (index + 1, command));
        }
    });
}

/// A line of the command log: its number and its command.
fn log_line(ui: &mut Ui<'_>, origin: Point, (number, command): (usize, &str)) {
    let theme = *ui.theme();
    let style = ui.text_style(TextKind::Code);
    let shown = format!("{number:>3}");
    let digits = ui.text(&TextRun {
        text: &shown,
        style,
        color: theme.colors.muted_foreground,
        origin,
        max_width: None,
    });
    let _ = ui.text(&TextRun {
        text: command,
        style,
        color: theme.colors.foreground,
        origin: Point::new(origin.x + digits + theme.sizes.spacing.medium, origin.y),
        max_width: None,
    });
}

/// The tasks: one that waits for what is running, with a button that cannot be used yet, and one that is done.
fn waiting_tasks(ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::with_gap(rect, spacing.medium);
    column_title(ui, &mut area, "TASKS");
    let row = theme.sizes.row_height;
    let control = metrics::control_height(&theme);
    let mut first = Area::with_gap(area.top(row * 2.0), spacing.medium);
    let run = Button::secondary("Run").disabled();
    let (width, _) = ui.button_size(&run);
    let button = first.right(width);
    let button = Rect::new(button.x, button.y + (button.height - control) * 0.5, width, control);
    if ui.button("run task", button, &run).clicked {
        intents.push(GalleryIntent::Clicked(String::from("Run")));
    }
    let mut words = Area::new(first.rest());
    let _ = ui.label(words.top(row), &Label::new("Rebase spike/graph-colors onto main"));
    let waits = Label::new("Waits for the fetch")
        .kind(TextKind::Detail)
        .tone(Tone::Muted);
    let _ = ui.label(words.rest(), &waits);
    let mut second = Area::with_gap(area.top(row), spacing.medium);
    ui.icon_in(second.left(theme.sizes.icon), Icon::Check, Tone::Success);
    let _ = ui.label(second.rest(), &Label::new("Push feature/login-page"));
}
