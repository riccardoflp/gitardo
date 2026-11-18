//! The groups of the Tasks tab (`tasks.edit-groups`, `tasks.run-group`): the operations the tab runs on the groups of
//! the repository shown, and the tasks read again once one of them changed the groups.

use std::sync::Arc;

use model::contract::{Answer, Answers, FeatureId, Target};
use model::tasks::{GroupsAction, GroupsTarget, TaskChoice};
use views::tasks::TasksIntent;

use super::super::message::Effect;
use super::super::state::State;
use super::{TabTasks, ask, shown, shown_mut, view};

/// The operations that write the groups of the repository.
const WRITERS: [&str; 2] = ["tasks.edit-groups", "tasks.import-groups"];

/// The group the heading of a key is the heading of, among those read.
pub(super) fn named_by(tab: &TabTasks, key: &str) -> Option<String> {
    let groups = tab.read.as_ref()?.groups.iter();
    let mut names = groups.map(|group| &group.name);
    names.find(|name| view::group_key(name) == key).cloned()
}

/// Runs an operation on the groups of the repository shown.
fn run(state: &mut State, feature: &'static str, target: GroupsTarget) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let repository = (read.folder.clone(), Arc::clone(&read.state));
    let asked = (FeatureId(feature), Target::TaskGroups(Box::new(target)));
    super::super::feature::run_in(state, repository, asked, Answers::new())
}

/// A change of the groups asked by the tab: the operation that makes it and writes their file.
fn edit(state: &mut State, action: GroupsAction) -> Vec<Effect> {
    run(state, "tasks.edit-groups", GroupsTarget::Edit(action))
}

/// What the Tasks tab asked of the groups: a change of them, or the tasks of one run or stopped.
pub(super) fn reduce(state: &mut State, intent: TasksIntent) -> Vec<Effect> {
    let group_named = |state: &State, name: &str| {
        let mut groups = shown(state)?.read.as_ref()?.groups.iter();
        groups.find(|group| group.name == name).cloned()
    };
    let is_stopping = matches!(intent, TasksIntent::StopGroup(_));
    match intent {
        TasksIntent::Groups(action) => edit(state, action),
        TasksIntent::RunGroup(name) | TasksIntent::StopGroup(name) => {
            let Some(group) = group_named(state, &name) else {
                return Vec::new();
            };
            let mut preset = Answers::new();
            if is_stopping {
                preset = preset.with(features::tasks::run::STOP, Answer::Confirmed);
            }
            ask(state, "tasks.run-group", TaskChoice::Group(group), preset)
        }
        // The other intents are not about the groups.
        _ => Vec::new(),
    }
}

/// An operation ended: the tasks of the repository tab shown are read again when it may have written the groups,
/// without waiting for the watcher to see their file change.
pub fn operation_ended(state: &mut State, feature: Option<FeatureId>) {
    if feature.is_some_and(|feature| WRITERS.contains(&feature.0))
        && let Some(tab) = shown_mut(state)
    {
        tab.is_stale = true;
    }
}
