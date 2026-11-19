//! The options of the next commit (`commit.options`): the button beside **Commit** or **Amend** and its menu, with
//! **Sign this commit**, **Add Signed-off-by**, **Co-authors…** and **Skip the hooks**; and the line under the commit
//! box that says the options in force while one differs from where it starts, which opens the same menu.

use model::commit::{CommitOptions, SigningConfig, option_arguments};
use ui::{Button, ButtonKind, MenuAnchor, MenuEntry, Rect, Ui};

use super::ChangesIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::changes as words;

/// The key of the menu, which the button and the line under the box open.
const MENU: &str = "commit options";

/// What the options of the next commit are drawn from.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CommitOptionsState {
    /// The options in force for the next commit.
    pub options: CommitOptions,
    /// Where each box starts: as git's configuration signs, and as `commit.signOff` says.
    pub starts: CommitOptions,
    /// How git's configuration signs commits (`commit.read-signing`).
    pub signing: SigningConfig,
    /// The line `--signoff` adds: `Signed-off-by: <name> <email>` of the identity.
    pub sign_off_line: String,
    /// Whether an operation runs: nothing can be changed.
    pub is_running: bool,
}

impl CommitOptionsState {
    /// Whether **Sign this commit** is ticked: as chosen, or as git's configuration signs.
    #[must_use]
    pub fn is_signed(&self) -> bool {
        self.options.sign.unwrap_or(self.signing.signs_every_commit) && self.signing.has_key()
    }

    /// Whether **Sign this commit** starts ticked.
    fn starts_signed(&self) -> bool {
        self.starts.sign.unwrap_or(self.signing.signs_every_commit) && self.signing.has_key()
    }

    /// What the line under the box says; nothing while every option is where it starts.
    #[must_use]
    pub fn said(&self) -> String {
        let names: Vec<&str> = self
            .options
            .co_authors
            .iter()
            .map(|person| person.name.as_str())
            .collect();
        let options = &self.options;
        words::options_said(&words::OptionsSaid {
            sign: (self.is_signed() != self.starts_signed()).then_some(self.is_signed()),
            sign_off: (options.sign_off != self.starts.sign_off).then_some(options.sign_off),
            co_authors: &names,
            skips_hooks: options.skip_hooks,
        })
    }

    /// The tooltip of the line: the arguments the options add to `git commit`.
    #[must_use]
    pub fn arguments_said(&self) -> String {
        let mut said = String::from("git commit");
        for argument in option_arguments(&self.options, &self.signing, "") {
            let argument = argument.to_string_lossy();
            if argument.contains(' ') {
                said.push_str(" \"");
                said.push_str(&argument);
                said.push('"');
            } else {
                said.push(' ');
                said.push_str(&argument);
            }
        }
        said
    }
}

/// Something asked of the options of the next commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionsIntent {
    /// **Sign this commit** ticked or unticked.
    Sign(bool),
    /// **Add Signed-off-by** ticked or unticked.
    SignOff(bool),
    /// **Skip the hooks** ticked or unticked.
    SkipHooks(bool),
    /// **Co-authors…**: open the picker (`commit.co-authors`).
    ChooseCoAuthors,
    /// A co-author taken away, by its place in the list.
    RemoveCoAuthor(usize),
}

/// What an entry of the menu stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Sign,
    SwitchIdentity,
    SignOff,
    CoAuthors,
    CoAuthor(usize),
    SkipHooks,
}

/// The button **Commit options** over the rectangle its caller clipped it to, with its menu, open from the button or
/// from the line under the box; gives what the entry chosen asks.
///
/// Feature: commit.options
pub fn commit_options_view(state: &CommitOptionsState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let people: Vec<String> = state
        .options
        .co_authors
        .iter()
        .map(|person| format!("{} <{}>", person.name, person.email))
        .collect();
    let signs_with = words::signs_with(&state.signing);
    let entries = entries(state, &people, &signs_with);
    let mut button = Button::icon(icons::CHOOSE, words::COMMIT_OPTIONS).kind(ButtonKind::Icon);
    if state.is_running {
        button = button.disabled();
    }
    let chosen = ui.menu_button(MENU, rect, &button, &entries);
    let intent = match chosen {
        Some(Entry::Sign) => Intent::Changes(ChangesIntent::Options(OptionsIntent::Sign(!state.is_signed()))),
        Some(Entry::SwitchIdentity) => Intent::Feature(crate::feature::FeatureIntent::Run(
            model::contract::FeatureId("identity.switch"),
            model::contract::Target::Head,
        )),
        Some(Entry::SignOff) => {
            Intent::Changes(ChangesIntent::Options(OptionsIntent::SignOff(!state.options.sign_off)))
        }
        Some(Entry::CoAuthors) => Intent::Changes(ChangesIntent::Options(OptionsIntent::ChooseCoAuthors)),
        Some(Entry::CoAuthor(index)) => Intent::Changes(ChangesIntent::Options(OptionsIntent::RemoveCoAuthor(index))),
        Some(Entry::SkipHooks) => Intent::Changes(ChangesIntent::Options(OptionsIntent::SkipHooks(
            !state.options.skip_hooks,
        ))),
        None => return Vec::new(),
    };
    vec![intent]
}

/// The entries of the menu, in its order.
fn entries<'a>(state: &'a CommitOptionsState, people: &'a [String], signs_with: &'a str) -> Vec<MenuEntry<'a, Entry>> {
    let mut entries = Vec::new();
    if state.signing.has_key() {
        let sign = icons::choice(words::SIGN_THIS_COMMIT, Entry::Sign, state.is_signed()).with_tooltip(signs_with);
        entries.push(MenuEntry::Item(sign));
    } else {
        let sign = icons::choice(words::SIGN_THIS_COMMIT, Entry::Sign, false)
            .with_tooltip(words::NO_SIGNING_KEY)
            .disabled();
        entries.push(MenuEntry::Item(sign));
        entries.push(MenuEntry::Item(icons::operation_item(
            words::SWITCH_IDENTITY,
            "identity.switch",
            Entry::SwitchIdentity,
        )));
    }
    let sign_off = icons::choice(words::ADD_SIGNED_OFF_BY, Entry::SignOff, state.options.sign_off);
    entries.push(MenuEntry::Item(sign_off.with_tooltip(&state.sign_off_line)));
    entries.push(MenuEntry::Item(icons::action(
        icons::CO_AUTHORS,
        words::CO_AUTHORS,
        Entry::CoAuthors,
    )));
    for (index, person) in people.iter().enumerate() {
        let item = icons::action(icons::CLOSE, person, Entry::CoAuthor(index));
        entries.push(MenuEntry::Item(item.with_tooltip(words::REMOVE_CO_AUTHOR)));
    }
    let skip = icons::choice(words::SKIP_THE_HOOKS, Entry::SkipHooks, state.options.skip_hooks);
    entries.push(MenuEntry::Item(skip.with_tooltip(words::SKIP_THE_HOOKS_TOOLTIP)));
    entries
}

/// The line under the commit box that says the options in force, muted, over a rectangle, while one differs from where
/// it starts; a click on it opens the menu of the options under it.
pub(super) fn options_line(state: &CommitOptionsState, ui: &mut Ui<'_>, rect: Rect) {
    let said = state.said();
    if said.is_empty() {
        return;
    }
    let tooltip = state.arguments_said();
    let link = Button::secondary(&said).kind(ButtonKind::Icon).with_tooltip(&tooltip);
    let (width, height) = ui.button_size(&link);
    let cell = ui::layout::place(
        rect,
        (width.min(rect.width), height),
        (ui::layout::Align::Start, ui::layout::Align::Center),
    );
    if ui.button("commit options line", cell, &link).clicked {
        ui.open_menu(MENU, MenuAnchor::Below(cell));
    }
}
