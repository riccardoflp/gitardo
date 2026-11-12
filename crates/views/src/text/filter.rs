//! The words of the branches the timeline shows: the branch filter (`filter.picker`, `filter.preset`,
//! `filter.list-too`), the entries of the menus (`filter.show-only`, `filter.show-too`, `filter.hide`,
//! `filter.show-all`, `filter.kinds`) and the bar above the commits (`filter.bar`).

use model::filter::{BranchesShown, short_name};

/// The list of the branch filter, for the screen reader.
pub const PICKER: &str = "Branches shown";
/// **All**.
pub const ALL: &str = "All";
/// The tooltip of **All**.
pub const ALL_TOOLTIP: &str = "Every branch, remote branch and tag";
/// **Current**.
pub const CURRENT: &str = "Current";
/// The tooltip of **Current**.
pub const CURRENT_TOOLTIP: &str = "Only what is checked out, whichever branch that is as you switch";
/// **Upstream**.
pub const UPSTREAM: &str = "Upstream";
/// The tooltip of **Upstream**.
pub const UPSTREAM_TOOLTIP: &str = "What is checked out and its upstream, whichever branch that is as you switch: what \
                                    is ahead of it and what is behind";
/// **Local**.
pub const LOCAL: &str = "Local";
/// The tooltip of **Local**.
pub const LOCAL_TOOLTIP: &str = "Only the local branches, and what is checked out";
/// **Remotes**.
pub const REMOTES: &str = "Remotes";
/// The tooltip of **Remotes**.
pub const REMOTES_TOOLTIP: &str = "Only the remote branches, and what is checked out";
/// The box of the branch filter.
pub const BOX: &str = "Filter branches and tags";
/// The heading of the tags in the list.
pub const TAGS: &str = "Tags";
/// The list with no name left.
pub const NO_NAME_MATCHES: &str = "No branch or tag has that in its name";
/// The list in a repository with no branch or tag.
pub const NO_REFERENCES: &str = "No branches or tags yet";
/// **Filter the list of branches too**.
pub const LIST_TOO: &str = "Filter the list of branches too";
/// Its tooltip.
pub const LIST_TOO_TOOLTIP: &str = "In the branch rail: only the branches the timeline shows, or all of them with the \
                                    others dimmed";
/// **Show only this history** in the menu of a reference.
pub const SHOW_ONLY: &str = "Show only this history";
/// **Show this history too**.
pub const SHOW_TOO: &str = "Show this history too";
/// **Hide this history**.
pub const HIDE: &str = "Hide this history";
/// **Show all branches**, in the menu of a reference and on the bar above the commits.
pub const SHOW_ALL_BRANCHES: &str = "Show all branches";
/// **Show local branches** in **…**.
pub const SHOW_LOCAL: &str = "Show local branches";
/// **Show remote branches** in **…**.
pub const SHOW_REMOTE: &str = "Show remote branches";
/// **Show tags** in **…**.
pub const SHOW_TAGS: &str = "Show tags";
/// The hint of the kinds while the filter chose otherwise.
pub const CHOSEN_IN_FILTER: &str = "Chosen in the filter";
/// The bar above the commits, for the screen reader.
pub const BAR: &str = "What the timeline is narrowed to";

/// The tooltip of the button of the branch filter: `Branches shown in the timeline: main +2`.
#[must_use]
pub fn button_tooltip(what: &str) -> String {
    format!("Branches shown in the timeline: {what}")
}

/// What the bar says the timeline is narrowed to: `Only the history of main, feature, fix and 2 more`, then how many
/// commits are listed, with `+` while there are more: `· 1000+ commits`.
#[must_use]
pub fn bar_words(shown: &BranchesShown, (listed, has_more): (usize, bool)) -> String {
    let more = if has_more { "+" } else { "" };
    let commits = if listed == 1 && !has_more {
        String::from("1 commit")
    } else {
        format!("{listed}{more} commits")
    };
    format!("Only the history of {} \u{b7} {commits}", narrowed_to(shown))
}

/// What the timeline is narrowed to, in the words of the bar.
fn narrowed_to(shown: &BranchesShown) -> String {
    match shown {
        BranchesShown::Current => String::from("the current branch"),
        BranchesShown::Upstream => String::from("the current branch and its upstream"),
        BranchesShown::Kinds(kinds) => {
            let named: Vec<&str> = [
                (kinds.local, "the local branches"),
                (kinds.remote, "the remote branches"),
                (kinds.tags, "the tags"),
            ]
            .into_iter()
            .filter(|(is_shown, _)| *is_shown)
            .map(|(_, words)| words)
            .collect();
            named.join(" and ")
        }
        BranchesShown::References(names) => names_words(names),
    }
}

/// Up to three names, `main, feature and 2 more`.
fn names_words(names: &[String]) -> String {
    let short: Vec<&str> = names.iter().map(|name| short_name(name)).collect();
    match short.as_slice() {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [first, second] => format!("{first} and {second}"),
        [first, second, third] => format!("{first}, {second} and {third}"),
        [first, second, third, rest @ ..] => format!("{first}, {second}, {third} and {} more", rest.len()),
    }
}
