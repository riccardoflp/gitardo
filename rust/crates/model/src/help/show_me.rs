//! **Show me** (`help.show-me`): under an answer of Ask Gitardo, a button for each feature it named, which runs it as
//! Quick search does, opens its page, or shows its place and says what to choose there.

use super::guide::{Guide, GuideCard, Show};
use crate::contract::FeatureId;

/// The most buttons under an answer.
pub const MOST_BUTTONS: usize = 4;

/// A place of the window that **Show where** shows, named as the guide names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// The timeline.
    Timeline,
    /// The branch rail, with the branches, the remote branches and the tags.
    BranchRail,
    /// **Kept aside**, in the branch rail: the stashes, the worktrees, the tags and the submodules.
    KeptAside,
    /// The Diff and Merge pages, opened from a file.
    DiffPages,
    /// The Conflicts page.
    ConflictsPage,
    /// The header, Search and Quick search.
    Header,
    /// **Repositories**, in the side column.
    Repositories,
    /// The Files page.
    FilesPage,
    /// The Compare page.
    ComparePage,
    /// The draft and the Changes page.
    ChangesPage,
    /// The inspector and the Commit page.
    Inspector,
    /// The bottom panel.
    BottomPanel,
    /// The Settings page.
    SettingsPage,
}

impl Place {
    /// The place of the words of the guide, when the window has it.
    #[must_use]
    pub fn of(words: &str) -> Option<Place> {
        Some(match words {
            "the timeline" => Place::Timeline,
            "the branch rail" => Place::BranchRail,
            "Kept aside, in the branch rail" => Place::KeptAside,
            "the Diff and Merge pages" => Place::DiffPages,
            "the Conflicts page" => Place::ConflictsPage,
            "the header, Search and Quick search" | "the header and the bottom panel" => Place::Header,
            "Repositories, in the side column" => Place::Repositories,
            "the Files page" => Place::FilesPage,
            "the Compare page" => Place::ComparePage,
            "the draft and the Changes page" => Place::ChangesPage,
            "the inspector and the Commit page" => Place::Inspector,
            "the bottom panel" => Place::BottomPanel,
            "the Settings page" => Place::SettingsPage,
            _ => return None,
        })
    }
}

/// What the button of a feature does when it is pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Showing {
    /// Runs it as Quick search runs it: its questions, its preview and its confirmation.
    Run,
    /// Opens its page.
    Open,
    /// Shows its place, and says in the status bar what to choose there.
    Where(Place),
}

/// The button of a feature named by an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShowMe {
    /// The feature.
    pub feature: FeatureId,
    /// The title of its card.
    pub title: &'static str,
    /// The place of the feature in the window, as the guide names it.
    pub place: &'static str,
    /// What the button does.
    pub showing: Showing,
}

/// The button of a feature of the guide, from its place: an operation Quick search runs is run, a page is opened,
/// something done on a commit, a branch, a file or a stash has its place shown; anything else, or a place the window
/// does not know, has no button. Nothing runs until the button is pressed.
///
/// Feature: help.show-me
#[must_use]
pub fn show_me(card: &GuideCard) -> Option<ShowMe> {
    let showing = match card.show {
        Show::QuickSearch => Showing::Run,
        Show::Page => Showing::Open,
        Show::ShowWhere => Showing::Where(Place::of(card.place)?),
        Show::Nothing => return None,
    };
    Some(ShowMe {
        feature: card.id,
        title: card.title,
        place: card.place,
        showing,
    })
}

/// The buttons under an answer: one for each feature it named that has one, in the order named, at most
/// [`MOST_BUTTONS`].
#[must_use]
pub fn buttons(named: &[FeatureId], guide: &Guide) -> Vec<ShowMe> {
    named
        .iter()
        .filter_map(|feature| guide.card(feature.0))
        .filter_map(show_me)
        .take(MOST_BUTTONS)
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::*;
    use crate::help::{App, guide};

    #[test]
    fn help_show_me__stash_push_is_run_as_quick_search_runs_it() {
        let guide = guide(App::Desktop);
        let card = guide.card("stash.push").unwrap();
        assert_eq!(show_me(card).map(|button| button.showing), Some(Showing::Run));
    }

    #[test]
    fn help_show_me__file_search_page_is_opened() {
        let guide = guide(App::Desktop);
        let card = guide.card("file-search.page").unwrap();
        assert_eq!(show_me(card).map(|button| button.showing), Some(Showing::Open));
    }

    #[test]
    fn help_show_me__branch_rename_shows_the_branch_rail() {
        let guide = guide(App::Desktop);
        let card = guide.card("branch.rename").unwrap();
        assert_eq!(
            show_me(card).map(|button| button.showing),
            Some(Showing::Where(Place::BranchRail))
        );
    }

    #[test]
    fn help_show_me__six_features_named_have_four_buttons_the_first_four_named() {
        let guide = guide(App::Desktop);
        let named = [
            "stash.push",
            "file-search.page",
            "branch.rename",
            "branch.create",
            "sync.fetch",
            "sync.push",
        ]
        .map(FeatureId);
        let shown: Vec<&str> = buttons(&named, &guide).iter().map(|button| button.feature.0).collect();
        assert_eq!(
            shown,
            ["stash.push", "file-search.page", "branch.rename", "branch.create"]
        );
    }

    #[test]
    fn help_show_me__every_place_of_the_desktop_guide_is_known() {
        let guide = guide(App::Desktop);
        for card in &guide.cards {
            assert!(Place::of(card.place).is_some(), "{}: {}", card.id.0, card.place);
        }
    }
}
