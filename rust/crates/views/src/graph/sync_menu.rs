//! The menu of **Sync** in the header (`graph.toolbar`): **Fetch**; **Pull**, which pulls as the settings say, then the
//! four ways to pull, the one **Pull** takes marked **default** (`sync.pull-with`); **Push**, then the two forced pushes.
//! Each entry says in its tooltip what it runs, and none can be chosen until its operation is built or while an
//! operation runs.

use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use ui::MenuEntry;

use super::toolbar::{SyncCounts, SyncMenu};
use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::header as words;

/// The operations of **Sync** and of its menu, in the order of `SyncMenu::built`.
pub const SYNC_FEATURES: [&str; 7] = [
    "sync.sync",
    "sync.fetch",
    "sync.pull",
    "sync.pull-with",
    "sync.push",
    "sync.push-force-with-lease",
    "sync.push-force",
];

/// The ways to pull, as `sync.pull-with` names its choices: the choice, the words of the entry, and the command.
pub const PULL_WAYS: [(&str, &str, &str); 4] = [
    ("plain", words::PULL_PLAIN, "git pull"),
    ("rebase", words::PULL_REBASE, "git pull --rebase"),
    ("autostash", words::PULL_AUTOSTASH, "git pull --autostash"),
    (
        "rebase-autostash",
        words::PULL_REBASE_AUTOSTASH,
        "git pull --rebase --autostash",
    ),
];

/// The question of `sync.pull-with` that an entry of a way answers.
const WAY: AnswerKey = AnswerKey("way");

/// What an entry of the menu runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SyncCommand {
    /// An operation, at `HEAD`.
    Run(&'static str),
    /// `sync.pull-with` in the way of this choice.
    Pull(&'static str),
    /// `range-diff.compare` of the branch checked out with its upstream.
    CompareWithUpstream,
}

/// The words of the menu that depend on the repository: the counts beside **Pull** and **Push**, and what **Pull** runs.
pub(super) struct SyncWords {
    /// **Pull**, with the number of commits to pull when there are some.
    pull: String,
    /// **Push**, with the number of commits to push when there are some.
    push: String,
    /// What **Pull** runs: `Pull: git pull --rebase --autostash` with both settings on.
    pull_tooltip: String,
}

impl SyncWords {
    /// The words for the counts of the branch checked out and the way **Pull** takes.
    pub(super) fn new(counts: Option<SyncCounts>, menu: &SyncMenu) -> SyncWords {
        let (behind, ahead) = counts.map_or((0, 0), |counts| (counts.behind, counts.ahead));
        let command = PULL_WAYS
            .get(menu.pull_way)
            .map_or("git pull", |(_, _, command)| command);
        SyncWords {
            pull: words::with_count(words::PULL, '\u{2193}', behind),
            push: words::with_count(words::PUSH, '\u{2191}', ahead),
            pull_tooltip: format!("{}: {command}", words::PULL),
        }
    }
}

/// The entries of the menu, each disabled until its operation is built and while an operation runs.
pub(super) fn sync_entries<'t>(menu: &SyncMenu, said: &'t SyncWords) -> Vec<MenuEntry<'t, SyncCommand>> {
    let [_, fetch, pull, pull_with, push, lease, force] = menu.built.map(|is_built| is_built && !menu.is_running);
    let item = |label: &'t str, command, tooltip: &'t str, can_run: bool| {
        let feature = match command {
            SyncCommand::Run(feature) => feature,
            SyncCommand::Pull(_) => "sync.pull-with",
            SyncCommand::CompareWithUpstream => "range-diff.compare",
        };
        let item = icons::operation_item(label, feature, command).with_tooltip(tooltip);
        if can_run { item } else { item.disabled() }
    };
    let mut entries = vec![
        MenuEntry::Item(item(
            words::FETCH,
            SyncCommand::Run("sync.fetch"),
            words::FETCH_TOOLTIP,
            fetch,
        )),
        MenuEntry::Separator,
        MenuEntry::Item(item(
            &said.pull,
            SyncCommand::Run("sync.pull"),
            &said.pull_tooltip,
            pull,
        )),
    ];
    entries.extend(PULL_WAYS.iter().enumerate().map(|(index, (way, label, command))| {
        let entry = item(label, SyncCommand::Pull(way), command, pull_with);
        MenuEntry::Item(if index == menu.pull_way {
            entry.with_shortcut(words::DEFAULT_WAY)
        } else {
            entry
        })
    }));
    entries.extend([
        MenuEntry::Separator,
        MenuEntry::Item(item(
            &said.push,
            SyncCommand::Run("sync.push"),
            words::PUSH_TOOLTIP,
            push,
        )),
        MenuEntry::Item(item(
            words::PUSH_FORCE_WITH_LEASE,
            SyncCommand::Run("sync.push-force-with-lease"),
            words::PUSH_FORCE_WITH_LEASE_TOOLTIP,
            lease,
        )),
        MenuEntry::Item(item(
            words::PUSH_FORCE,
            SyncCommand::Run("sync.push-force"),
            words::PUSH_FORCE_TOOLTIP,
            force,
        )),
    ]);
    entries.extend(compare_with_upstream(menu, force));
    entries
}

/// **Compare with the upstream commit by commit** among the other ways to push, while the branch checked out and its
/// upstream have diverged: what a force push would replace (`range-diff.compare`).
fn compare_with_upstream<'t>(menu: &SyncMenu, can_run: bool) -> Option<MenuEntry<'t, SyncCommand>> {
    let item = icons::operation_item(
        words::COMPARE_WITH_UPSTREAM,
        "range-diff.compare",
        SyncCommand::CompareWithUpstream,
    )
    .with_tooltip(words::COMPARE_WITH_UPSTREAM_TOOLTIP);
    menu.has_diverged
        .then(|| MenuEntry::Item(if can_run { item } else { item.disabled() }))
}

/// The intent of an entry chosen.
pub(super) fn sync_intent(command: SyncCommand) -> Intent {
    let intent = match command {
        SyncCommand::Run(feature) => FeatureIntent::Run(FeatureId(feature), Target::Head),
        SyncCommand::Pull(way) => FeatureIntent::RunWith(
            FeatureId("sync.pull-with"),
            Target::Head,
            Answers::new().with(WAY, Answer::Choice(String::from(way))),
        ),
        SyncCommand::CompareWithUpstream => FeatureIntent::RunWith(
            FeatureId("range-diff.compare"),
            Target::Head,
            Answers::new().with(AnswerKey("range-diff.with"), Answer::Choice(String::from("upstream"))),
        ),
    };
    Intent::Feature(intent)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use ui::MenuEntry;

    use super::{SyncCommand, SyncMenu, SyncWords, sync_entries};
    use crate::text::header as words;

    /// The entries of the menu as (words, tooltip, shortcut, whether it can be chosen).
    fn entries(menu: &SyncMenu) -> Vec<(String, String, Option<String>, bool)> {
        let said = SyncWords::new(None, menu);
        sync_entries(menu, &said)
            .into_iter()
            .filter_map(|entry| match entry {
                MenuEntry::Item(item) => Some((
                    item.label.to_owned(),
                    item.tooltip.unwrap_or_default().to_owned(),
                    item.shortcut.map(str::to_owned),
                    item.availability == ui::Availability::Enabled,
                )),
                _ => None,
            })
            .collect()
    }

    fn built(pull_way: usize) -> SyncMenu {
        SyncMenu {
            is_running: false,
            built: [true; 7],
            pull_way,
            has_diverged: false,
        }
    }

    #[test]
    fn sync_pull__the_tooltip_of_pull_says_git_pull_rebase_autostash_with_both_settings_on() {
        let listed = entries(&built(3));
        let pull = listed.iter().find(|(label, ..)| label == words::PULL);
        assert_eq!(
            pull.map(|(_, tooltip, ..)| tooltip.as_str()),
            Some("Pull: git pull --rebase --autostash")
        );
    }

    #[test]
    fn sync_pull_with__the_menu_marks_the_way_pull_takes_as_the_default() {
        let marked: Vec<String> = entries(&built(2))
            .into_iter()
            .filter(|(.., shortcut, _)| shortcut.as_deref() == Some(words::DEFAULT_WAY))
            .map(|(label, ..)| label)
            .collect();
        assert_eq!(marked, [words::PULL_AUTOSTASH]);
    }

    #[test]
    fn graph_toolbar__while_an_operation_runs_no_entry_of_the_menu_of_sync_can_be_chosen() {
        let running = SyncMenu {
            is_running: true,
            ..built(3)
        };
        assert!(entries(&running).iter().all(|(.., can_choose)| !can_choose));
        assert!(entries(&built(3)).iter().all(|(.., can_choose)| *can_choose));
    }

    #[test]
    fn a_way_chosen_pulls_with_its_answer_given() {
        let intent = super::sync_intent(SyncCommand::Pull("rebase"));
        let crate::intent::Intent::Feature(crate::feature::FeatureIntent::RunWith(feature, _, answers)) = intent else {
            panic!("a way runs sync.pull-with with its answer")
        };
        assert_eq!(feature.0, "sync.pull-with");
        assert_eq!(answers.choice(model::contract::AnswerKey("way")), Some("rebase"));
    }
}
