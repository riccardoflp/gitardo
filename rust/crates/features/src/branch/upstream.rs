//! `branch.set-upstream` and `branch.unset-upstream`: the remote branch a local branch follows, set, changed or taken
//! away.

use model::branch::{RefKind, RefNames, ref_argument};
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, Outcome, Plan, Question, QuestionKind, Step, Target, Text, Undo,
};
use model::state::{Branch, RemoteBranch};

use super::target::{local_branch, refuse};

/// The remote branch picked, by its short name: `origin/feature`.
pub const UPSTREAM: AnswerKey = AnswerKey("upstream");

/// The local branch an upstream operation was invoked on: the one named, or the one checked out from Quick search.
fn branch_of(context: &Context) -> Result<&Branch, Box<Outcome>> {
    if context.target == Target::Head {
        return context
            .repository
            .branches
            .iter()
            .find(|branch| branch.is_checked_out)
            .ok_or_else(|| Box::new(refuse("Check out a branch first.")));
    }
    local_branch(context)
}

/// The short name of the upstream of a branch: `origin/main`.
fn upstream_name(branch: &Branch) -> Option<&str> {
    branch.upstream.as_ref().map(|upstream| {
        upstream
            .reference
            .strip_prefix("refs/remotes/")
            .unwrap_or(&upstream.reference)
    })
}

/// Asks for the remote branch a local branch is to follow, and makes it follow that one with
/// `git branch --set-upstream-to`; the upstream it has already, picked again, runs nothing.
///
/// Feature: branch.set-upstream
#[must_use]
pub fn set_upstream(context: &Context, answers: &Answers) -> Outcome {
    let branch = match branch_of(context) {
        Ok(branch) => branch,
        Err(refusal) => return *refusal,
    };
    let Some(picked) = answers.choice(UPSTREAM) else {
        if context.repository.remote_branches.is_empty() {
            return refuse("This repository has no remote branch.");
        }
        return Outcome::Ask(upstream_question(context, branch));
    };
    if !context
        .repository
        .remote_branches
        .iter()
        .any(|remote| remote.name == picked)
    {
        return refuse(&format!("{picked} is no longer there: fetch, then pick again."));
    }
    let name = branch.name.as_str();
    if upstream_name(branch) == Some(picked) {
        return Outcome::Run(plan(format!("{name} follows {picked} already"), Vec::new()));
    }
    let remote = ref_argument(RefKind::RemoteBranch, picked, &RefNames::of(&context.repository));
    let step = Step::git(
        context.repository.folder.clone(),
        ["branch", &format!("--set-upstream-to={remote}"), name],
    );
    Outcome::Run(plan(format!("Make {name} follow {picked}"), vec![step]))
}

/// Makes a local branch follow nothing with `git branch --unset-upstream`; the remote branch stays.
///
/// Feature: branch.unset-upstream
#[must_use]
pub fn unset_upstream(context: &Context, _answers: &Answers) -> Outcome {
    let branch = match branch_of(context) {
        Ok(branch) => branch,
        Err(refusal) => return *refusal,
    };
    let name = branch.name.as_str();
    let Some(upstream) = upstream_name(branch) else {
        return refuse(&format!("{name} follows no upstream."));
    };
    let step = Step::git(context.repository.folder.clone(), ["branch", "--unset-upstream", name]);
    Outcome::Run(plan(format!("Stop tracking {upstream} from {name}"), vec![step]))
}

/// A plan that changes the configuration of a branch, which Undo does not take back.
fn plan(title: String, steps: Vec<Step>) -> Plan {
    Plan {
        title: Text::from(title),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: Change upstream\u{2026} sets another, and Stop tracking takes it away.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// The search of the remote branches: those of the name of the branch first, the one of `remote.default` before the
/// others, then by name; the upstream it has described **current upstream**.
fn upstream_question(context: &Context, branch: &Branch) -> Question {
    let default = model::remote::default(&context.repository.remotes).unwrap_or_default();
    let remotes = &context.repository.remotes;
    let mut found: Vec<&RemoteBranch> = context.repository.remote_branches.iter().collect();
    found.sort_by_key(|remote| {
        let short = model::remote::split_remote_branch(&remote.name, remotes).map_or("", |(_, name)| name);
        (short != branch.name, remote.remote != default, remote.name.clone())
    });
    let current = upstream_name(branch);
    let choices = found
        .into_iter()
        .map(|remote| Choice {
            identifier: remote.name.clone(),
            label: Text::from(remote.name.as_str()),
            description: (current == Some(remote.name.as_str())).then(|| Text::from("current upstream")),
        })
        .collect();
    let name = branch.name.as_str();
    Question {
        key: UPSTREAM,
        kind: QuestionKind::Search { choices },
        title: Text::from(format!("Upstream of `{name}`")),
        detail: Some(Text::from(format!("The remote branch {name} pulls from and pushes to"))),
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{UPSTREAM, set_upstream, unset_upstream};
    use crate::testing::{branch, context, lines, plan, question, state, tracking};
    use model::contract::{Answer, Answers, Outcome, QuestionKind, Target};
    use model::state::{RemoteBranch, RepositoryState};

    fn remote_branch(name: &str) -> RemoteBranch {
        let (remote, _) = name.split_once('/').unwrap_or((name, ""));
        RemoteBranch {
            name: name.to_owned(),
            remote: remote.to_owned(),
            commit: crate::testing::commit('c'),
            date: model::Timestamp::default(),
        }
    }

    fn repository(upstream: Option<(&str, &str)>) -> RepositoryState {
        let mut state = state();
        state.remotes = vec![String::from("origin"), String::from("upstream")];
        let mut feature = branch("feature", 'b', false);
        feature.upstream = upstream.map(|(remote, name)| tracking(remote, name));
        state.branches.push(feature);
        for name in ["origin/release", "upstream/feature", "origin/feature", "origin/main"] {
            state.remote_branches.push(remote_branch(name));
        }
        state
    }

    fn on_feature(state: RepositoryState) -> model::contract::Context {
        context(state, Target::Branch(String::from("feature")))
    }

    fn picked(name: &str) -> Answers {
        Answers::new().with(UPSTREAM, Answer::Choice(name.to_owned()))
    }

    fn listed(outcome: Outcome) -> Vec<(String, Option<String>)> {
        let QuestionKind::Search { choices } = question(outcome).kind else {
            panic!("the upstream is searched")
        };
        choices
            .into_iter()
            .map(|choice| (choice.identifier, choice.description.map(|text| text.to_string())))
            .collect()
    }

    #[test]
    fn branch_set_upstream__lists_the_namesakes_first_origin_first_and_sets_the_one_picked() {
        let context = on_feature(repository(None));
        let names: Vec<String> = listed(set_upstream(&context, &Answers::new()))
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(
            names,
            ["origin/feature", "upstream/feature", "origin/main", "origin/release"]
        );
        assert_eq!(
            lines(&plan(set_upstream(&context, &picked("origin/feature")))),
            ["git branch --set-upstream-to=origin/feature feature"]
        );
    }

    #[test]
    fn branch_set_upstream__changed_to_another_remote_sets_that_one() {
        let context = on_feature(repository(Some(("origin", "feature"))));
        assert_eq!(
            lines(&plan(set_upstream(&context, &picked("upstream/feature")))),
            ["git branch --set-upstream-to=upstream/feature feature"]
        );
    }

    #[test]
    fn branch_set_upstream__a_local_branch_of_the_same_short_name_names_the_remote_branch_in_full() {
        let mut state = repository(None);
        state.branches.push(branch("origin/feature", 'd', false));
        assert_eq!(
            lines(&plan(set_upstream(&on_feature(state), &picked("origin/feature")))),
            ["git branch --set-upstream-to=refs/remotes/origin/feature feature"]
        );
    }

    #[test]
    fn branch_set_upstream__the_current_upstream_is_described_so_and_picking_it_runs_nothing() {
        let context = on_feature(repository(Some(("origin", "feature"))));
        let listed = listed(set_upstream(&context, &Answers::new()));
        assert_eq!(
            listed[0],
            (String::from("origin/feature"), Some(String::from("current upstream")))
        );
        assert_eq!(
            lines(&plan(set_upstream(&context, &picked("origin/feature")))),
            Vec::<String>::new()
        );
    }

    #[test]
    fn branch_set_upstream__a_gone_upstream_can_be_changed() {
        let mut state = repository(Some(("origin", "old")));
        if let Some(upstream) = state.branches[1].upstream.as_mut() {
            upstream.is_gone = true;
        }
        assert_eq!(
            lines(&plan(set_upstream(&on_feature(state), &picked("origin/feature")))),
            ["git branch --set-upstream-to=origin/feature feature"]
        );
    }

    #[test]
    fn branch_set_upstream__a_repository_with_no_remote_branch_refuses() {
        let mut state = repository(None);
        state.remote_branches.clear();
        assert!(matches!(
            set_upstream(&on_feature(state), &Answers::new()),
            Outcome::Refuse(_)
        ));
    }

    #[test]
    fn branch_set_upstream__a_remote_branch_gone_since_the_pick_refuses() {
        let outcome = set_upstream(&on_feature(repository(None)), &picked("origin/gone"));
        let Outcome::Refuse(refusal) = outcome else {
            panic!("refused")
        };
        assert_eq!(
            refusal.message.as_str(),
            "origin/gone is no longer there: fetch, then pick again."
        );
    }

    #[test]
    fn branch_unset_upstream__runs_unset_upstream_on_the_branch() {
        let context = on_feature(repository(Some(("origin", "feature"))));
        assert_eq!(
            lines(&plan(unset_upstream(&context, &Answers::new()))),
            ["git branch --unset-upstream feature"]
        );
    }

    #[test]
    fn branch_unset_upstream__a_gone_upstream_is_taken_away_too() {
        let mut state = repository(Some(("origin", "old")));
        if let Some(upstream) = state.branches[1].upstream.as_mut() {
            upstream.is_gone = true;
        }
        assert_eq!(
            lines(&plan(unset_upstream(&on_feature(state), &Answers::new()))),
            ["git branch --unset-upstream feature"]
        );
    }

    #[test]
    fn branch_unset_upstream__a_branch_with_no_upstream_refuses() {
        let outcome = unset_upstream(&on_feature(repository(None)), &Answers::new());
        assert!(matches!(outcome, Outcome::Refuse(_)));
    }
}
