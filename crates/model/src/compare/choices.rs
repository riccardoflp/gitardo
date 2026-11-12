//! What a side of a comparison can be chosen from, in Gitardo's own picker.

use super::Side;
use super::recent::RecentComparison;
use super::sides::{CompareSide, WORKING_TREE};
use crate::CommitId;
use crate::refs::References;

/// What an item of the picker chooses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideChoice {
    /// A comparison opened before, by its place in the list.
    Recent(usize),
    /// **Enter a hash or a revision…**: asks for the text.
    EnterRevision,
    /// A branch, a remote branch, a tag or what is checked out, by its name, at its commit.
    Named {
        /// The name.
        name: String,
        /// Its commit.
        commit: CommitId,
    },
    /// The working tree.
    WorkingTree,
}

/// An item of the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickItem {
    /// The group it is under, if any: **Opened before**, **Branches**, **Remote branches**, **Tags**.
    pub group: Option<&'static str>,
    /// Its name.
    pub label: String,
    /// What is said beside it.
    pub description: String,
    /// What it chooses.
    pub choice: SideChoice,
}

/// What [`side_choices`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideChoicesInput<'a> {
    /// The references, each list in the order of `refs.sort`.
    pub references: &'a References,
    /// The commit checked out; none before the first commit.
    pub head: Option<&'a CommitId>,
    /// The branch checked out; none when `HEAD` is detached.
    pub branch: Option<&'a str>,
    /// The side chosen already, for the second question.
    pub chosen: Option<&'a CompareSide>,
    /// Whether the working tree may be offered.
    pub may_offer_working_tree: bool,
    /// The comparisons opened before, the latest first, when they are offered.
    pub recent: Option<&'a [RecentComparison]>,
}

/// The items of the picker of a side: the comparisons opened before, **Enter a hash or a revision…**, what is checked
/// out, the working tree, then the branches, the remote branches and the tags; the side chosen already left out.
///
/// Feature: compare.side-choices
#[must_use]
pub fn side_choices(input: &SideChoicesInput<'_>) -> Vec<PickItem> {
    let mut items: Vec<PickItem> = recent_items(input.recent.unwrap_or_default());
    items.push(PickItem {
        group: None,
        label: String::from("Enter a hash or a revision…"),
        description: String::from("such as 1a2b3c4 or HEAD~3"),
        choice: SideChoice::EnterRevision,
    });
    if let Some(head) = input.head {
        let name = input.branch.unwrap_or("HEAD");
        items.push(named(None, name, head, &format!("{} · checked out", head.short())));
    }
    let chosen_is_working_tree = input.chosen.is_some_and(CompareSide::is_working_tree);
    if input.may_offer_working_tree && !chosen_is_working_tree {
        items.push(PickItem {
            group: None,
            label: String::from(WORKING_TREE),
            description: String::from("with the changes not committed"),
            choice: SideChoice::WorkingTree,
        });
    }
    let references = input.references;
    let branches = references
        .branches
        .iter()
        .filter(|branch| Some(branch.name.as_str()) != input.branch)
        .map(|branch| (&branch.name, &branch.commit));
    let groups: [(&'static str, Vec<(&String, &CommitId)>); 3] = [
        ("Branches", branches.collect()),
        (
            "Remote branches",
            references
                .remote_branches
                .iter()
                .map(|remote| (&remote.name, &remote.commit))
                .collect(),
        ),
        (
            "Tags",
            references.tags.iter().map(|tag| (&tag.name, &tag.commit)).collect(),
        ),
    ];
    for (group, references) in groups {
        items.extend(
            references
                .into_iter()
                .map(|(name, commit)| named(Some(group), name, commit, commit.short())),
        );
    }
    items.retain(|item| !is_chosen(item, input.chosen));
    items
}

/// The items of the comparisons opened before.
fn recent_items(recent: &[RecentComparison]) -> Vec<PickItem> {
    recent
        .iter()
        .enumerate()
        .map(|(index, known)| PickItem {
            group: Some("Opened before"),
            label: format!("{} ↔ {}", known.first.name, known.second.name),
            description: known.path.clone().unwrap_or_default(),
            choice: SideChoice::Recent(index),
        })
        .collect()
}

/// The item of a name at a commit.
fn named(group: Option<&'static str>, name: &str, commit: &CommitId, description: &str) -> PickItem {
    PickItem {
        group,
        label: name.to_owned(),
        description: description.to_owned(),
        choice: SideChoice::Named {
            name: name.to_owned(),
            commit: commit.clone(),
        },
    }
}

/// Whether an item is the side chosen already: the same name at the same commit.
fn is_chosen(item: &PickItem, chosen: Option<&CompareSide>) -> bool {
    let (Some(chosen), SideChoice::Named { name, commit }) = (chosen, &item.choice) else {
        return false;
    };
    *name == chosen.name && chosen.side == Side::Commit(commit.clone())
}

/// The items whose name or description has what is typed, ignoring case; **Enter a hash or a revision…** is always
/// kept.
#[must_use]
pub fn filter_choices<'a>(items: &'a [PickItem], typed: &str) -> Vec<&'a PickItem> {
    let typed = typed.trim().to_lowercase();
    items
        .iter()
        .filter(|item| {
            item.choice == SideChoice::EnterRevision
                || item.label.to_lowercase().contains(&typed)
                || item.description.to_lowercase().contains(&typed)
        })
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{SideChoice, SideChoicesInput, filter_choices, side_choices};
    use crate::compare::{CompareSide, RecentComparison, RecentSide, Side};
    use crate::refs::References;
    use crate::state::{Branch, Tag};
    use crate::{CommitId, Timestamp};

    fn id(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"))
    }

    fn references() -> References {
        let branch = |name: &str, digit: char| Branch {
            name: name.to_owned(),
            commit: id(digit),
            upstream: None,
            is_checked_out: name == "main",
            date: Timestamp(0),
        };
        References {
            branches: vec![branch("main", '1'), branch("feature", 'a')],
            remote_branches: Vec::new(),
            tags: vec![Tag {
                name: String::from("v1.0"),
                commit: id('3'),
                date: Timestamp(0),
            }],
        }
    }

    fn input<'a>(references: &'a References, head: Option<&'a CommitId>) -> SideChoicesInput<'a> {
        SideChoicesInput {
            references,
            head,
            branch: Some("main"),
            chosen: None,
            may_offer_working_tree: true,
            recent: None,
        }
    }

    fn labels(input: &SideChoicesInput<'_>) -> Vec<String> {
        side_choices(input).into_iter().map(|item| item.label).collect()
    }

    #[test]
    fn compare_side_choices__the_second_question_leaves_out_the_side_chosen_first() {
        let (references, head) = (references(), id('1'));
        assert_eq!(
            labels(&input(&references, Some(&head))),
            ["Enter a hash or a revision…", "main", "Working tree", "feature", "v1.0"]
        );
        let feature = CompareSide {
            name: String::from("feature"),
            side: Side::Commit(id('a')),
            is_followed: true,
            is_gone: false,
        };
        let second = SideChoicesInput {
            chosen: Some(&feature),
            ..input(&references, Some(&head))
        };
        assert!(!labels(&second).contains(&String::from("feature")));
    }

    #[test]
    fn compare_side_choices__the_working_tree_is_not_offered_against_itself() {
        let (references, head) = (references(), id('1'));
        let working_tree = CompareSide::working_tree();
        let second = SideChoicesInput {
            chosen: Some(&working_tree),
            ..input(&references, Some(&head))
        };
        assert!(!labels(&second).contains(&String::from("Working tree")));
    }

    #[test]
    fn compare_side_choices__typing_a_short_hash_finds_the_branch() {
        let (references, head) = (references(), id('1'));
        let items = side_choices(&input(&references, Some(&head)));
        let found: Vec<&str> = filter_choices(&items, "AAAAA")
            .into_iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(found, ["Enter a hash or a revision…", "feature"]);
    }

    #[test]
    fn compare_side_choices__without_a_commit_what_is_checked_out_is_not_there() {
        let references = References::default();
        assert_eq!(
            labels(&input(&references, None)),
            ["Enter a hash or a revision…", "Working tree"]
        );
    }

    #[test]
    fn compare_side_choices__opened_before_lists_the_latest_first() {
        let (references, head) = (references(), id('1'));
        let side = |name: &str| RecentSide {
            name: name.to_owned(),
            commit: Some(id('1')),
        };
        let recent = [
            RecentComparison {
                first: side("main"),
                second: side("feature"),
                path: Some(String::from("src")),
            },
            RecentComparison {
                first: side("main"),
                second: side("v1.0"),
                path: None,
            },
        ];
        let offered = SideChoicesInput {
            recent: Some(&recent),
            ..input(&references, Some(&head))
        };
        let items = side_choices(&offered);
        assert_eq!(
            (items[0].label.as_str(), items[0].description.as_str(), &items[0].choice),
            ("main ↔ feature", "src", &SideChoice::Recent(0))
        );
        assert_eq!(items[1].label, "main ↔ v1.0");
        assert_eq!(items[0].group, Some("Opened before"));
    }
}
