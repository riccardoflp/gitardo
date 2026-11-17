//! Dialogs: a box in the middle of the window over a dimmed backdrop, with a title, a body of widgets the view draws,
//! and buttons along its bottom right, the main one last. While one is open nothing below it gets the pointer or the
//! keys, Tab goes round its widgets only, Enter presses its main button, unless that one cannot be undone, and Escape
//! cancels it.
//!
//! A dialog is open while the view declares it: the view keeps whether it is open, and stops declaring it once it is
//! answered.

use std::hash::Hash;

use render::{Border, Color, Rect, Theme};

use crate::access::{AccessRole, Description};

use crate::frame::{TextKind, Ui};
use crate::input::{Key, Modifiers, NamedKey};
use crate::interaction::Sense;
use crate::layer::Layer;
use crate::layout::{Align, Area, Insets, inset, place};
use crate::widget::button::Button;
use crate::widget::metrics;

/// What a button of a dialog does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DialogRole {
    /// The main button: what the dialog is for. Enter presses it.
    Primary,
    /// The main button of a dialog whose action cannot be undone, drawn in the color of errors. Enter does not press
    /// it: what cannot be undone runs on a click, never on a key pressed by habit.
    Danger,
    /// Another button.
    Secondary,
    /// The button that closes the dialog doing nothing, like Escape.
    Cancel,
    /// The main button while what it does cannot be done yet, such as **Compare** before both sides are chosen: drawn
    /// disabled, and Enter does not press it.
    Unavailable,
}

/// Where a dialog opens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogPlace {
    /// In the middle of the window, as wide as a sentence or two.
    Middle,
    /// At the top of the window, as wide as given or the window less its margins, as a picker opens.
    Top {
        /// Its width, in logical pixels.
        width: f32,
    },
}

/// A button of a dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogButton<'text> {
    /// Its words.
    pub label: &'text str,
    /// What it does.
    pub role: DialogRole,
}

/// A dialog.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dialog<'text> {
    /// The words of its title.
    pub title: &'text str,
    /// The height of the body the view draws, in logical pixels.
    pub body_height: f32,
    /// Its buttons, from left to right; the main one goes last.
    pub buttons: &'text [DialogButton<'text>],
}

/// The body of a dialog, for the view that draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialogBody {
    /// Where the body goes.
    pub rect: Rect,
    /// Whether the dialog opened in this frame, when its first input box should take the focus.
    pub is_opening: bool,
}

/// What a dialog tells its view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DialogResponse {
    /// The button pressed in this frame, by its place among the buttons: clicked, or the main one by Enter.
    pub pressed: Option<usize>,
    /// Whether the dialog was cancelled: by Escape, or by its button of the role [`DialogRole::Cancel`].
    pub is_cancelled: bool,
}

/// How opaque the backdrop of a dialog is, out of 255, in the themes whose shadows cannot be seen.
const BACKDROP: u8 = 170;

/// The color that dims the window behind a dialog: the shadow of the theme, or its background a little see-through
/// where the shadow cannot be seen, as in the high contrast themes.
fn backdrop(theme: &Theme) -> Color {
    if theme.colors.shadow.is_transparent() {
        theme.colors.window_background.with_alpha(BACKDROP)
    } else {
        theme.colors.shadow
    }
}

/// The width of a dialog: room for a sentence or two on a line, and never wider than the window.
pub(crate) fn dialog_width(theme: &Theme, window: Rect) -> f32 {
    (theme.sizes.row_height * 22.0).min(window.width - 2.0 * theme.sizes.spacing.huge)
}

impl Ui<'_> {
    /// The width of the body of a dialog in this window, for a view that lays out its body before declaring it.
    #[must_use]
    pub fn dialog_body_width(&self) -> f32 {
        let theme = self.theme();
        (dialog_width(theme, self.window()) - 2.0 * theme.sizes.spacing.huge).max(0.0)
    }

    /// Declares a dialog: draws it over the window, calls `body` to draw its body, and says which button was pressed.
    pub fn dialog(
        &mut self,
        key: impl Hash,
        dialog: &Dialog<'_>,
        body: impl FnOnce(&mut Ui<'_>, DialogBody),
    ) -> DialogResponse {
        self.dialog_at(key, (dialog, DialogPlace::Middle), body)
    }

    /// Declares a dialog as [`Ui::dialog`] does, in a place of the window.
    pub fn dialog_at(
        &mut self,
        key: impl Hash,
        (dialog, place_of): (&Dialog<'_>, DialogPlace),
        body: impl FnOnce(&mut Ui<'_>, DialogBody),
    ) -> DialogResponse {
        let id = self.id(key);
        let is_opening = self.declare_overlay(id, &[]);
        if is_opening {
            self.memory().set_focus(None);
        }
        let theme = *self.theme();
        let window = self.window();
        let spacing = theme.sizes.spacing;
        let control = metrics::control_height(&theme);
        let title_style = self.text_style(TextKind::Heading);
        let title_height = self.line_height(title_style);
        let height = 2.0 * spacing.huge + title_height + spacing.large + dialog.body_height + spacing.huge + control;
        let rect = match place_of {
            DialogPlace::Middle => place(
                window,
                (dialog_width(&theme, window), height.min(window.height)),
                (Align::Center, Align::Center),
            ),
            DialogPlace::Top { width } => {
                let width = width.min(window.width - 2.0 * spacing.huge).max(0.0);
                let height = height.min(window.height - 2.0 * spacing.huge).max(0.0);
                Rect::new(
                    window.x + (window.width - width) * 0.5,
                    window.y + spacing.huge,
                    width,
                    height,
                )
            }
        };
        let mut response = DialogResponse::default();
        self.layer(Layer::Modal, |ui| {
            ui.fill_rect(window, backdrop(&theme));
            let _ = ui.interact(id.with("backdrop"), window, Sense::CLICK);
            ui.floating_card(rect, theme.sizes.radius.large + spacing.small);
            ui.access_node(id, rect, &Description::new(AccessRole::Dialog, dialog.title));
            ui.access_enter(id);
            let mut area = Area::new(inset(rect, Insets::all(spacing.huge)));
            let _ = ui.heading(area.top(title_height), dialog.title);
            let _ = area.top(spacing.large);
            let buttons = area.bottom(control);
            let _ = area.bottom(spacing.huge);
            let body_rect = area.rest();
            ui.within(id, |ui| {
                let opened = DialogBody {
                    rect: body_rect,
                    is_opening,
                };
                ui.clip(body_rect, |ui| body(ui, opened));
                response = ui.dialog_buttons(buttons, dialog.buttons);
            });
            ui.access_leave();
        });
        self.dialog_keys(dialog, &mut response);
        response
    }

    /// The buttons of a dialog along the right of a row, in order, and which was pressed.
    fn dialog_buttons(&mut self, row: Rect, buttons: &[DialogButton<'_>]) -> DialogResponse {
        let mut response = DialogResponse::default();
        let gap = self.theme().sizes.spacing.medium;
        let mut area = Area::with_gap(row, gap);
        for (index, dialog_button) in buttons.iter().enumerate().rev() {
            let button = match dialog_button.role {
                DialogRole::Primary => Button::primary(dialog_button.label),
                DialogRole::Danger => Button::danger(dialog_button.label),
                DialogRole::Secondary | DialogRole::Cancel => Button::secondary(dialog_button.label),
                DialogRole::Unavailable => Button::primary(dialog_button.label).disabled(),
            };
            let (width, _) = self.button_size(&button);
            if self.button(("button", index), area.right(width), &button).clicked {
                response.pressed = Some(index);
                response.is_cancelled = dialog_button.role == DialogRole::Cancel;
            }
        }
        response
    }

    /// Enter presses the main button, unless it is one of danger, and Escape cancels, when no widget of the dialog used
    /// them.
    fn dialog_keys(&mut self, dialog: &Dialog<'_>, response: &mut DialogResponse) {
        let main = dialog
            .buttons
            .iter()
            .position(|button| button.role == DialogRole::Primary);
        for key in self.unclaimed_keys() {
            if key.modifiers != Modifiers::NONE {
                continue;
            }
            match key.key {
                Key::Named(NamedKey::Enter) if main.is_some() && response.pressed.is_none() => {
                    response.pressed = main;
                    self.claim_any_key(key);
                }
                Key::Named(NamedKey::Escape) => {
                    response.is_cancelled = true;
                    self.claim_any_key(key);
                }
                _ => {}
            }
        }
    }
}

/// How a confirmation looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ConfirmStyle {
    /// Its main button is drawn as any main button.
    #[default]
    Normal,
    /// What it confirms cannot be undone: its main button is drawn in the color of errors.
    Danger,
}

/// A dialog that asks to confirm what is about to run: a sentence, the commands as lines of code, and a button that
/// runs them beside one that cancels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Confirmation<'text> {
    /// The words of its title.
    pub title: &'text str,
    /// What will happen, in a sentence or two.
    pub message: &'text str,
    /// The commands that will run, one per line.
    pub commands: &'text [&'text str],
    /// The words of the button that runs them, such as **Run**.
    pub confirm: &'text str,
    /// How it looks.
    pub style: ConfirmStyle,
}

/// What the person answered to a confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConfirmAnswer {
    /// Run it.
    Confirmed,
    /// Do nothing.
    Cancelled,
}

/// The most lines of commands a confirmation shows before it scrolls them.
const COMMAND_LINES: u16 = 8;

impl Ui<'_> {
    /// Declares a confirmation and says what the person answered, in the frame they answered.
    pub fn confirm(&mut self, key: impl Hash, confirmation: &Confirmation<'_>) -> Option<ConfirmAnswer> {
        let theme = *self.theme();
        let spacing = theme.sizes.spacing;
        let width = self.dialog_body_width();
        let body_style = self.text_style(TextKind::Body);
        let code_style = self.text_style(TextKind::Code);
        let message_lines = self.fonts().wrap(confirmation.message, body_style, width).len();
        let message_height = crate::number::float_of_count(message_lines) * self.line_height(body_style);
        let code_line = self.line_height(code_style);
        let shown = u16::try_from(confirmation.commands.len())
            .unwrap_or(u16::MAX)
            .min(COMMAND_LINES);
        let commands_height = f32::from(shown) * code_line + 2.0 * spacing.medium;
        let role = match confirmation.style {
            ConfirmStyle::Normal => DialogRole::Primary,
            ConfirmStyle::Danger => DialogRole::Danger,
        };
        let buttons = [
            DialogButton {
                label: "Cancel",
                role: DialogRole::Cancel,
            },
            DialogButton {
                label: confirmation.confirm,
                role,
            },
        ];
        let dialog = Dialog {
            title: confirmation.title,
            body_height: message_height + spacing.large + commands_height,
            buttons: &buttons,
        };
        let response = self.dialog(key, &dialog, |ui, body| {
            let mut area = Area::with_gap(body.rect, spacing.large);
            let message = area.top(message_height);
            let _ = ui.paragraph(
                message,
                confirmation.message,
                (TextKind::Body, crate::widget::Tone::Normal),
            );
            ui.command_lines(area.top(commands_height), confirmation.commands);
        });
        match (response.pressed, response.is_cancelled) {
            (_, true) => Some(ConfirmAnswer::Cancelled),
            (Some(_), false) => Some(ConfirmAnswer::Confirmed),
            (None, false) => None,
        }
    }

    /// Commands as lines of code in a box, scrolled when there are many.
    fn command_lines(&mut self, rect: Rect, commands: &[&str]) {
        let theme = *self.theme();
        let radius = theme.sizes.radius.large;
        self.fill_rounded_rect(rect, radius, theme.colors.secondary_button_background);
        let border = Border {
            width: theme.sizes.border,
            radius,
            color: theme.colors.input_border,
        };
        self.border(rect, border);
        let inside = inset(rect, Insets::all(theme.sizes.spacing.medium));
        let style = self.text_style(TextKind::Code);
        let line = self.line_height(style);
        let widest = commands
            .iter()
            .map(|command| self.measure(command, style))
            .fold(0.0_f32, f32::max);
        let height = crate::number::float_of_count(commands.len()) * line;
        let _ = self.scroll_area("commands", inside, (widest.max(inside.width), height), |ui, content| {
            for (index, command) in commands.iter().enumerate() {
                let y = content.y + crate::number::float_of_count(index) * line;
                let run = render::TextRun {
                    text: command,
                    style,
                    color: theme.colors.input_foreground,
                    origin: render::Point::new(content.x, y),
                    max_width: None,
                };
                let _ = ui.text(&run);
            }
        });
    }
}
