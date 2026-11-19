//! The picker of the co-authors of the next commit (`commit.co-authors`): a dialog of many, titled **Co-authors of
//! this commit**, with the authors of the history to tick, found by typing, and an author typed as `Name <email>` to
//! add.

use model::commit::{Author, CoAuthor};
use ui::layout::{Area, Insets, inset};
use ui::{Choice, Dialog, DialogButton, DialogRole, InputBox, Label, TextKind, Tone, Ui};

use super::ChangesIntent;
use crate::intent::Intent;
use crate::text::changes as words;

/// How many authors the picker shows at most; typing finds the others.
pub const AUTHORS_SHOWN: usize = 8;

/// What the picker is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoAuthorsState<'a> {
    /// The authors of the history, the one with the most commits first; none while they are read.
    pub authors: Option<&'a [Author]>,
    /// What is typed in the box at its top.
    pub typed: &'a str,
    /// The co-authors ticked, in the order ticked.
    pub ticked: &'a [CoAuthor],
    /// The email of the identity that commits, which the picker leaves out.
    pub identity_email: Option<&'a str>,
}

impl CoAuthorsState<'_> {
    /// Whether a person is ticked, by their email without regard to case.
    fn is_ticked(&self, email: &str) -> bool {
        self.ticked
            .iter()
            .any(|person| person.email.eq_ignore_ascii_case(email))
    }

    /// The authors the picker lists: those that match what is typed, without the identity that commits, the first
    /// [`AUTHORS_SHOWN`] of them.
    #[must_use]
    pub fn found(&self) -> Vec<&Author> {
        let identity = self.identity_email.unwrap_or_default();
        self.authors
            .unwrap_or_default()
            .iter()
            .filter(|author| identity.is_empty() || !author.has_email(identity))
            .filter(|author| author.matches(self.typed))
            .take(AUTHORS_SHOWN)
            .collect()
    }

    /// The person typed, offered to add: `Name <email>` that no author of the history and no co-author ticked has.
    #[must_use]
    pub fn offered(&self) -> Option<CoAuthor> {
        let typed = CoAuthor::parse(self.typed)?;
        let is_known = self
            .authors
            .unwrap_or_default()
            .iter()
            .any(|author| author.has_email(&typed.email));
        (!is_known && !self.is_ticked(&typed.email)).then_some(typed)
    }

    /// The co-authors ticked that are not among the authors of the history: those added by typing, listed first.
    fn added(&self) -> Vec<&CoAuthor> {
        let authors = self.authors.unwrap_or_default();
        self.ticked
            .iter()
            .filter(|person| !authors.iter().any(|author| author.has_email(&person.email)))
            .collect()
    }
}

/// Something asked of the picker of the co-authors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoAuthorsIntent {
    /// The box at the top was typed in.
    Typed(String),
    /// A person ticked or unticked; one typed and offered is added ticked.
    Tick(CoAuthor),
    /// `Enter` or **Set co-authors**: the ticks become the co-authors of the next commit.
    Keep,
    /// `Escape` or **Cancel**: the co-authors are left as they were.
    Cancel,
}

/// The picker of the co-authors over the window: the box to type in, the person typed offered to add, the co-authors
/// added by typing, and the authors of the history found, each ticked or not with its email and its commits under it.
///
/// Feature: commit.co-authors
pub fn co_authors_view(state: &CoAuthorsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let gap = theme.sizes.spacing.medium;
    let rows = 1 + 2 * AUTHORS_SHOWN;
    let body_height = theme.sizes.control_height + gap + crate::line::rows_of(rows) * theme.sizes.row_height;
    let buttons = [
        DialogButton {
            label: crate::text::feature::CANCEL,
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: words::SET_CO_AUTHORS,
            role: DialogRole::Primary,
        },
    ];
    let dialog = Dialog {
        title: words::CO_AUTHORS_TITLE,
        body_height,
        buttons: &buttons,
    };
    let mut intents = Vec::new();
    let response = ui.dialog("co-authors", &dialog, |ui, body| {
        let mut area = Area::with_gap(body.rect, gap);
        let field = area.top(theme.sizes.control_height);
        if body.is_opening {
            let id = ui.id("co-authors typed");
            ui.request_focus(id);
        }
        let input = InputBox::new(state.typed, words::TYPE_A_CO_AUTHOR);
        if let Some(text) = ui.input_box("co-authors typed", field, &input).changed_text {
            intents.push(asked(CoAuthorsIntent::Typed(text)));
        }
        intents.extend(rows_view(state, ui, &mut Area::new(area.rest())));
    });
    if response.is_cancelled {
        intents.push(asked(CoAuthorsIntent::Cancel));
    } else if response.pressed.is_some() {
        intents.push(asked(CoAuthorsIntent::Keep));
    }
    intents
}

/// The rows of the picker, from the top of an area.
fn rows_view(state: &CoAuthorsState<'_>, ui: &mut Ui<'_>, area: &mut Area) -> Vec<Intent> {
    let row_height = ui.theme().sizes.row_height;
    let mut intents = Vec::new();
    let Some(_) = state.authors else {
        let _ = ui.label(
            area.top(row_height),
            &Label::new(words::READING_AUTHORS).tone(Tone::Muted),
        );
        return intents;
    };
    if let Some(person) = state.offered() {
        let label = words::add_co_author(&person.name, &person.email);
        let choice = Choice::check_box(&label, false);
        if ui
            .choice(("co-author add", person.email.as_str()), area.top(row_height), &choice)
            .clicked
        {
            intents.push(asked(CoAuthorsIntent::Tick(person.clone())));
        }
    }
    for person in state.added().into_iter().take(AUTHORS_SHOWN) {
        let detail = person.email.clone();
        intents.extend(person_row(ui, area, (&person.name, &detail), (person, true)));
    }
    let added = state.added().len();
    for author in state.found().into_iter().take(AUTHORS_SHOWN.saturating_sub(added)) {
        let detail = words::author_detail(&author.email, author.commits);
        let person = CoAuthor {
            name: author.name.clone(),
            email: author.email.clone(),
        };
        let is_ticked = state.is_ticked(&author.email);
        intents.extend(person_row(ui, area, (&author.name, &detail), (&person, is_ticked)));
    }
    intents
}

/// A person ticked or not, by name, with a muted line under it.
fn person_row(
    ui: &mut Ui<'_>,
    area: &mut Area,
    (name, detail): (&str, &str),
    (person, is_ticked): (&CoAuthor, bool),
) -> Option<Intent> {
    let theme = *ui.theme();
    let row = area.top(theme.sizes.row_height);
    let clicked = ui
        .choice(
            ("co-author", person.email.as_str()),
            row,
            &Choice::check_box(name, is_ticked),
        )
        .clicked;
    let line = inset(
        area.top(theme.sizes.row_height),
        Insets::symmetric(theme.sizes.icon + theme.sizes.spacing.medium, 0.0),
    );
    let _ = ui.label(line, &Label::new(detail).kind(TextKind::Detail).tone(Tone::Muted));
    clicked.then(|| asked(CoAuthorsIntent::Tick(person.clone())))
}

/// The intent of something asked of the picker.
fn asked(intent: CoAuthorsIntent) -> Intent {
    Intent::Changes(ChangesIntent::CoAuthors(intent))
}
