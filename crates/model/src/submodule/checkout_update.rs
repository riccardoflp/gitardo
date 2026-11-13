//! The update of the submodules that ends a checkout (`submodule.update-on-checkout`).

use std::path::Path;

use super::{Submodule, commits};
use crate::contract::{Danger, FollowUp, Plan, Step, Text};

/// What the note of a checkout followed by the update says first.
pub const THEN_UPDATED: &str = "Then the submodules go to the commits recorded for them.";

/// A checkout, and what decides whether the submodules are updated after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckoutUpdate<'a> {
    /// The plan of the checkout.
    pub checkout: &'a Plan,
    /// `submodules.updateOnCheckout`.
    pub is_on: bool,
    /// Whether the repository has submodules before or after the checkout.
    pub has_submodules: bool,
    /// The top folder of the repository.
    pub folder: &'a Path,
    /// The submodules as last read, for what the update leaves behind in each.
    pub submodules: &'a [Submodule],
}

/// The plan of a checkout, ended with `git submodule update --init --recursive` when the setting is on and the
/// repository has submodules on either side: its note says the submodules go to their recorded commits and what that
/// leaves behind in each, and it is asked first when a submodule has commits on a detached `HEAD` that no branch will
/// have. A refusal to clone from a folder on disk is offered again as `submodule.allow-file-protocol` says. Otherwise
/// the checkout alone.
///
/// Feature: submodule.update-on-checkout
#[must_use]
pub fn update_on_checkout(input: &CheckoutUpdate<'_>) -> Plan {
    let mut plan = input.checkout.clone();
    if !(input.is_on && input.has_submodules) {
        return plan;
    }
    plan.steps.push(Step::Warn {
        words: String::from(THEN_UPDATED),
    });
    let mut left_on_no_branch = Vec::new();
    for submodule in input.submodules {
        let lost = lost_commits(submodule).map(|count| detached_words(&submodule.folder, count));
        if let Some(words) = &lost {
            left_on_no_branch.push(words.clone());
        }
        let kept = has_changes(submodule).then(|| uncommitted_words(&submodule.folder));
        for words in lost.into_iter().chain(kept) {
            plan.steps.push(Step::Warn { words });
        }
    }
    plan.steps.push(Step::git(
        input.folder.to_path_buf(),
        ["submodule", "update", "--init", "--recursive"],
    ));
    if !left_on_no_branch.is_empty() {
        plan.danger = with_reason(&plan.danger, &left_on_no_branch.join("\n"));
    }
    plan.on_failure.push(FollowUp::AllowFileProtocol);
    plan
}

/// How many commits after the recorded one a submodule has on a detached `HEAD`, which no branch will have once it goes
/// to the recorded commit; none when it has none.
#[must_use]
pub fn lost_commits(submodule: &Submodule) -> Option<u32> {
    let checkout = submodule.checkout.as_ref()?;
    let ahead = checkout.distance?.ahead;
    (checkout.branch.is_none() && ahead > 0).then_some(ahead)
}

/// Whether a submodule has files that are not committed.
#[must_use]
pub fn has_changes(submodule: &Submodule) -> bool {
    submodule
        .checkout
        .as_ref()
        .is_some_and(|checkout| checkout.uncommitted > 0)
}

/// **`<folder>` has `<n>` commits after the recorded one on a detached HEAD: …**
#[must_use]
pub fn detached_words(folder: &str, count: u32) -> String {
    format!(
        "{folder} has {} after the recorded one on a detached HEAD: no branch will have them. Create a branch in it \
         first to keep them.",
        commits(count)
    )
}

/// **`<folder>` has changes that are not committed: …**
#[must_use]
pub fn uncommitted_words(folder: &str) -> String {
    format!("{folder} has changes that are not committed: git keeps them, or stops if they are in the way.")
}

/// A danger that asks first, with a reason added to what it said.
fn with_reason(danger: &Danger, reason: &str) -> Danger {
    match danger {
        Danger::None => Danger::Confirm(Text::from(reason)),
        Danger::Confirm(note) => Danger::Confirm(Text::from(format!("{note}\n\n{reason}"))),
        Danger::Destructive(note) => Danger::Destructive(Text::from(format!("{note}\n\n{reason}"))),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{CheckoutUpdate, THEN_UPDATED, update_on_checkout};
    use crate::CommitId;
    use crate::contract::{Danger, FollowUp, Plan, Step, Text, Undo};
    use crate::submodule::{Checkout, Distance, Submodule};

    fn checkout_plan() -> Plan {
        Plan {
            title: Text::from("Check out feature"),
            steps: vec![Step::git(PathBuf::from("repository"), ["checkout", "feature"])],
            danger: Danger::None,
            undo: Undo::ByReflog,
            after: vec![FollowUp::SelectHead],
            on_failure: Vec::new(),
        }
    }

    /// `lib`, so many commits ahead of its recorded one, on `main` or detached, with files not committed.
    fn lib(ahead: u32, branch: Option<&str>, uncommitted: usize) -> Submodule {
        let commit = CommitId::parse(&"a".repeat(40)).unwrap();
        Submodule {
            name: String::from("lib"),
            folder: String::from("lib"),
            parent: None,
            url: None,
            branch: None,
            recorded: Some(commit.clone()),
            staged: None,
            checkout: Some(Checkout {
                commit: Some(commit),
                branch: branch.map(str::to_owned),
                uncommitted,
                distance: Some(Distance { ahead, behind: 0 }),
            }),
        }
    }

    fn update(is_on: bool, has_submodules: bool, submodules: &[Submodule]) -> Plan {
        update_on_checkout(&CheckoutUpdate {
            checkout: &checkout_plan(),
            is_on,
            has_submodules,
            folder: Path::new("repository"),
            submodules,
        })
    }

    fn lines(plan: &Plan) -> Vec<String> {
        plan.steps.iter().map(Step::log_line).collect()
    }

    #[test]
    fn submodule_update_on_checkout__the_checkout_ends_with_the_update_and_says_so() {
        let plan = update(true, true, &[lib(0, Some("main"), 0)]);
        assert_eq!(
            lines(&plan),
            [
                "git checkout feature",
                THEN_UPDATED,
                "git submodule update --init --recursive"
            ]
        );
        assert_eq!(plan.danger, Danger::None);
        assert_eq!(plan.on_failure, [FollowUp::AllowFileProtocol]);
    }

    #[test]
    fn submodule_update_on_checkout__commits_on_a_detached_head_ask_first_and_name_the_submodule() {
        let plan = update(true, true, &[lib(2, None, 1)]);
        let Danger::Confirm(reason) = &plan.danger else {
            panic!("the checkout asks first: {:?}", plan.danger);
        };
        assert!(
            reason
                .as_str()
                .starts_with("lib has 2 commits after the recorded one on a detached HEAD")
        );
        assert!(
            lines(&plan)
                .iter()
                .any(|line| line.starts_with("lib has changes that are not committed"))
        );
    }

    #[test]
    fn submodule_update_on_checkout__files_not_committed_alone_do_not_ask() {
        let plan = update(true, true, &[lib(2, Some("main"), 3)]);
        assert_eq!(plan.danger, Danger::None);
    }

    #[test]
    fn submodule_update_on_checkout__with_the_setting_off_or_no_submodules_the_checkout_runs_alone() {
        assert_eq!(update(false, true, &[lib(0, None, 0)]), checkout_plan());
        assert_eq!(update(true, false, &[]), checkout_plan());
    }
}
