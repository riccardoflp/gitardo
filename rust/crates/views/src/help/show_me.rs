//! The buttons of **Show me** under an answer (`help.show-me`): their words, the page each opens, and the row they are
//! drawn in.

use model::contract::FeatureId;
use model::help::{ShowMe, Showing};
use ui::layout::{Align, place};
use ui::{Button, Rect, Ui};

use super::HelpIntent;
use crate::intent::Intent;
use crate::page::Page;
use crate::search::{QuickChoice, QuickCommand, SearchIntent};
use crate::text::help as words;

/// A button under an answer, for a feature it named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowMeButton {
    /// Its words: **Stash Changes**, **Open Search**, **Show where**.
    pub label: String,
    /// The feature it does or shows.
    pub feature: FeatureId,
}

/// The words of the button of a feature: Quick search's own words for an operation it lists, else the title of its
/// card; **Open** and the name of a page; **Show where**, also for a page that only opens on something chosen.
#[must_use]
pub fn button_label(show: &ShowMe) -> String {
    match show.showing {
        Showing::Run => QuickCommand::of_feature(show.feature.0)
            .map_or(show.title, crate::text::search::command_name)
            .to_owned(),
        Showing::Open => {
            page_of(show.feature.0).map_or_else(|| words::SHOW_WHERE.to_owned(), |(name, _)| words::open_page(name))
        }
        Showing::Where(_) => words::SHOW_WHERE.to_owned(),
    }
}

/// The page a feature is, by its name, with what opens it; none for a page that opens on something chosen, as the
/// Diff page on a file.
#[must_use]
pub fn page_of(feature: &str) -> Option<(&'static str, Intent)> {
    let quick = |command| Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(command)));
    Some(match feature {
        "welcome.page" => (crate::text::pages::WELCOME, Intent::ShowPage(Page::Welcome)),
        "file-search.page" => (
            crate::text::file_search::PAGE,
            Intent::FileSearch(crate::file_search::FileSearchIntent::Show),
        ),
        "changes.pane" => (
            crate::text::changes::PAGE,
            Intent::Changes(crate::changes::ChangesIntent::Open),
        ),
        "settings.edit" | "settings.sections" => (crate::text::settings::PAGE, Intent::ShowPage(Page::Settings)),
        "worktree-setup.page" => (
            crate::text::worktree_setup::PAGE,
            Intent::WorktreeSetup(crate::worktree_setup::SetupIntent::Open),
        ),
        "review.page" => (crate::text::review::PAGE, quick(QuickCommand::ReviewConflicts)),
        "compare.page" => (
            crate::text::views::COMPARE,
            Intent::Feature(crate::feature::FeatureIntent::Run(
                FeatureId("compare.choose"),
                model::contract::Target::Head,
            )),
        ),
        _ => return None,
    })
}

/// The height of the row of the buttons, when there are some.
pub(super) fn row_height(buttons: &[ShowMeButton], ui: &Ui<'_>) -> f32 {
    if buttons.is_empty() {
        0.0
    } else {
        ui.theme().sizes.control_height
    }
}

/// The buttons of a question in a row, from the left of a rectangle, those that do not fit left out; what the one
/// pressed asks.
pub(super) fn buttons_view(buttons: &[ShowMeButton], key: usize, ui: &mut Ui<'_>, row: Rect) -> Vec<Intent> {
    let gap = ui.theme().sizes.spacing.small;
    let mut left = row.x;
    let mut intents = Vec::new();
    for button in buttons {
        let drawn = Button::secondary(&button.label);
        let (width, height) = ui.button_size(&drawn);
        if left + width > row.right() {
            break;
        }
        let cell = place(
            Rect::new(left, row.y, width, row.height),
            (width, height),
            (Align::Start, Align::Center),
        );
        left += width + gap;
        if ui.button(("help show me", key, button.feature.0), cell, &drawn).clicked {
            intents.push(Intent::Help(HelpIntent::ShowMe(button.feature)));
        }
    }
    intents
}
