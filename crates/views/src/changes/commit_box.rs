//! The commit box at the top of the Changes page: the message, the sparkle button that drafts it, **Commit** or
//! **Amend**, the box **Amend last commit**, and under them the identity commits will be made with.

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Choice, Icon, InputBox, Key, NamedKey, Rect, Ui};

use super::{ChangesIntent, ChangesRepository};
use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::changes as words;

/// The key of the message box.
const MESSAGE_BOX: &str = "commit message";

/// What the commit box holds, which the draft of the timeline shows too.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CommitBox<'a> {
    /// The message typed.
    pub message: &'a str,
    /// The branch checked out, for the placeholder; none when `HEAD` is detached.
    pub branch: Option<&'a str>,
    /// What **Commit** makes: a new commit, or the last one again while **Amend last commit** is ticked.
    pub kind: CommitKind,
    /// Whether there is a commit to amend.
    pub has_commit: bool,
    /// Whether something is changed, staged or not.
    pub has_changes: bool,
    /// Whether an operation runs.
    pub is_running: bool,
}

/// What **Commit** makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CommitKind {
    /// A new commit (`commit.create`).
    #[default]
    New,
    /// The last commit again (`commit.amend`), while **Amend last commit** is ticked.
    Amend,
}

impl CommitBox<'_> {
    /// Whether **Amend last commit** is ticked.
    #[must_use]
    pub fn is_amend(&self) -> bool {
        self.kind == CommitKind::Amend
    }

    /// Whether **Commit** can be clicked: to amend, once there is a commit; to commit, with a message that is not only
    /// spaces and something changed; never while an operation runs.
    #[must_use]
    pub fn can_commit(&self) -> bool {
        if self.is_running {
            return false;
        }
        if self.is_amend() {
            self.has_commit
        } else {
            !self.message.trim().is_empty() && self.has_changes
        }
    }

    /// The words of the button: **Amend** while **Amend last commit** is ticked, **Commit** otherwise.
    #[must_use]
    pub fn button_words(&self) -> &'static str {
        if self.is_amend() { words::AMEND } else { words::COMMIT }
    }
}

/// The height the commit box and the identity take.
pub(super) fn height(ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    theme.sizes.control_height * 3.0 + theme.sizes.spacing.medium * 2.0
}

/// The commit box over a rectangle, and what was asked of it.
pub(super) fn view(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let control = theme.sizes.control_height;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    let mut message_row = Area::with_gap(area.top(control), theme.sizes.spacing.small);
    let sparkle = Button::icon(Icon::Star, words::DRAFT_MESSAGE)
        .kind(ButtonKind::Icon)
        .disabled();
    let _ = ui.button("draft message", message_row.right(control), &sparkle);
    intents.extend(message_box(state, ui, message_row.rest()));
    let mut button_row = Area::with_gap(area.top(control), theme.sizes.spacing.large);
    let commit = &state.commit;
    let mut button = Button::primary(commit.button_words());
    if !commit.can_commit() {
        button = button.disabled();
    }
    let width = ui.button_size(&button).0;
    let cell = place(button_row.left(width), (width, control), (Align::Start, Align::Center));
    if ui.button("commit", cell, &button).clicked {
        intents.push(Intent::Changes(ChangesIntent::Commit));
    }
    let mut amend = Choice::check_box(words::AMEND_LAST_COMMIT, commit.is_amend());
    if !commit.has_commit || state.is_running {
        amend = amend.disabled();
    }
    let response = ui.choice("amend", button_row.rest(), &amend);
    ui.tooltip(&response, words::AMEND_TOOLTIP);
    if response.clicked {
        intents.push(Intent::Changes(ChangesIntent::Amend(!commit.is_amend())));
    }
    intents.extend(identity_line(state, ui, area.rest()));
    intents
}

/// The message box: what is typed goes to the store, and `Ctrl+Enter` (`Cmd+Enter` on macOS) commits as the button
/// does, and does nothing while the button is disabled.
fn message_box(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let id = ui.id(MESSAGE_BOX);
    if state.wants_focus {
        ui.focus_input_box(id);
        intents.push(Intent::Changes(ChangesIntent::Focused));
    }
    let placeholder = words::message_placeholder(state.commit.branch);
    let typed = ui.input_box(MESSAGE_BOX, rect, &InputBox::new(state.commit.message, &placeholder));
    if let Some(text) = typed.changed_text {
        intents.push(Intent::Changes(ChangesIntent::Type(text)));
    }
    let is_commit_key = ui
        .unclaimed_keys_of(id)
        .iter()
        .any(|key| key.key == Key::Named(NamedKey::Enter) && key.modifiers.command());
    if is_commit_key && state.commit.can_commit() {
        intents.push(Intent::Changes(ChangesIntent::Commit));
    }
    intents
}

/// The identity commits will be made with, `Name <email>` or **No identity configured**: a click on it switches it
/// (`identity.switch`).
fn identity_line(state: &ChangesRepository<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let said = match (&state.identity.name, &state.identity.email) {
        (Some(name), Some(email)) => words::identity(name, email),
        (Some(name), None) => name.clone(),
        (None, Some(email)) => email.clone(),
        (None, None) => words::NO_IDENTITY.to_owned(),
    };
    let mut link = Button::secondary(&said).kind(ButtonKind::Icon);
    if state.is_running {
        link = link.disabled();
    }
    let (width, height) = ui.button_size(&link);
    let cell = place(rect, (width.min(rect.width), height), (Align::Start, Align::Center));
    if ui.button("identity", cell, &link).clicked {
        return vec![Intent::Feature(FeatureIntent::Run(
            FeatureId("identity.switch"),
            Target::Head,
        ))];
    }
    Vec::new()
}
