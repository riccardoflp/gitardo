//! `worktree.switch`: lists the worktrees with their branches and how they stand, and opens the one picked.

use model::contract::{Answer, AnswerKey, Answers, Context, Outcome, Question, QuestionKind, Text};
use model::state::Worktree;

use super::names::choice;
use super::{create, label, no_steps, opening};

/// The worktree picked, by its folder, or [`CREATE`].
pub const PICKED: AnswerKey = AnswerKey("worktree");
/// The last choice, which starts `worktree.create`.
pub const CREATE: &str = "\u{0}create";

/// Lists the worktrees that are not missing, the one the views show last as **this window**, then **Create
/// worktree…**; the worktree picked opens where `worktrees.openIn` says, **Create worktree…** goes on as
/// `worktree.create`, and the worktree of this window does nothing.
///
/// Feature: worktree.switch
#[must_use]
pub fn switch(context: &Context, answers: &Answers) -> Outcome {
    let Some(picked) = answers.choice(PICKED) else {
        return Outcome::Ask(question(context));
    };
    if picked == CREATE {
        return create(context, answers);
    }
    let worktree = context
        .repository
        .worktrees
        .iter()
        .find(|worktree| worktree.folder.to_string_lossy() == picked);
    match worktree {
        Some(worktree) if !worktree.is_current => Outcome::Run(opening::where_set(
            context,
            &worktree.folder,
            &label(context, &worktree.folder),
        )),
        _ => Outcome::Run(no_steps(String::from("Stay in this worktree"), Vec::new())),
    }
}

/// **Switch Worktree**: the worktrees that are not missing, this window's last, then **Create worktree…**.
fn question(context: &Context) -> Question {
    let mut worktrees: Vec<&Worktree> = context
        .repository
        .worktrees
        .iter()
        .filter(|worktree| !worktree.is_missing)
        .collect();
    worktrees.sort_by_key(|worktree| worktree.is_current);
    let mut choices: Vec<_> = worktrees
        .iter()
        .map(|worktree| {
            let folder = worktree.folder.to_string_lossy();
            choice(
                &folder,
                &label(context, &worktree.folder),
                Some(format!("{}  {folder}", standing(context, worktree))),
            )
        })
        .collect();
    choices.push(choice(CREATE, "Create worktree\u{2026}", None));
    let default = choices.first().map(|first| Answer::Choice(first.identifier.clone()));
    Question {
        key: PICKED,
        kind: QuestionKind::Pick { choices },
        title: Text::from("Switch Worktree"),
        detail: Some(Text::from("The worktree to open")),
        default,
    }
}

/// How a worktree stands: its branch or **detached at `<short>`**, `↑n ↓m` from the branch's upstream, how many of
/// its files are not committed, and **this window** for the one the views show.
fn standing(context: &Context, worktree: &Worktree) -> String {
    let mut parts = Vec::new();
    match worktree
        .branch
        .as_deref()
        .map(|full| full.strip_prefix("refs/heads/").unwrap_or(full))
    {
        Some(name) => {
            parts.push(name.to_owned());
            let upstream = context
                .repository
                .branches
                .iter()
                .find(|branch| branch.name == name)
                .and_then(|branch| branch.upstream.as_ref());
            if let Some(upstream) = upstream.filter(|upstream| upstream.ahead > 0 || upstream.behind > 0) {
                parts.push(format!("\u{2191}{} \u{2193}{}", upstream.ahead, upstream.behind));
            }
        }
        None => parts.push(format!(
            "detached at {}",
            worktree.commit.as_ref().map_or("", |commit| commit.short())
        )),
    }
    if let Some(count) = worktree.uncommitted.filter(|count| *count > 0) {
        // The fonts of the window have no pencil: the count says what it is in words.
        parts.push(format!("{count} not committed"));
    }
    if worktree.is_current {
        parts.push(String::from("this window"));
    }
    parts.join("  ")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome, QuestionKind, Target};

    use super::{CREATE, PICKED, switch};
    use crate::testing::{plan, question};
    use crate::worktree::testing::{LOGIN, MAIN, with_worktrees};

    #[test]
    fn worktree_switch__the_worktree_of_this_window_is_listed_last_marked_this_window() {
        let asked = question(switch(&with_worktrees(Target::Head), &Answers::new()));
        let QuestionKind::Pick { choices } = &asked.kind else {
            panic!()
        };
        let identifiers: Vec<&str> = choices.iter().map(|choice| choice.identifier.as_str()).collect();
        assert_eq!(identifiers, [LOGIN, MAIN, CREATE], "the missing worktree is not listed");
        let this_window = choices.get(1).and_then(|choice| choice.description.as_ref()).unwrap();
        assert!(this_window.as_str().contains("this window"), "{this_window:?}");
        assert!(this_window.as_str().contains(MAIN), "its folder is under it");
    }

    #[test]
    fn worktree_switch__picking_another_worktree_opens_it_and_the_last_entry_starts_creating_one() {
        let context = with_worktrees(Target::Head);
        let opened = plan(switch(
            &context,
            &Answers::new().with(PICKED, Answer::Choice(LOGIN.to_owned())),
        ));
        assert_eq!(opened.after, [FollowUp::OpenWindow(LOGIN.into())]);
        let creating = switch(
            &context,
            &Answers::new().with(PICKED, Answer::Choice(CREATE.to_owned())),
        );
        assert!(matches!(creating, Outcome::Ask(asked) if asked.key == crate::worktree::START));
        let here = plan(switch(
            &context,
            &Answers::new().with(PICKED, Answer::Choice(MAIN.to_owned())),
        ));
        assert_eq!((here.steps.len(), here.after.len()), (0, 0));
    }
}
