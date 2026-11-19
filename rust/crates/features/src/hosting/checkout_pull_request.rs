//! `hosting.checkout-pull-request`: checks out the commits of a pull request in a local branch, one from a branch of the
//! repository as its remote branch, one from a fork by the reference the service keeps it under.

use model::branch::other_worktree;
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, FollowUp, Outcome, Query, Question, QuestionKind, Refusal, Step,
    Target, Text, Validation,
};
use model::hosting::{HostingRepo, HostingService, PullList, PullListEntry, detect_remote, typed_number};

use crate::branch::checkout::checkout_plan;
use crate::branch::submodules::update_after_checkout;

/// The remotes, read to find the project of the main remote on its service.
pub const REMOTES: AnswerKey = AnswerKey("remotes");

/// The open pull requests, read for the pick.
pub const LIST: AnswerKey = AnswerKey("pull-requests");

/// The pull request picked, by its number, or [`TYPE_NUMBER`].
pub const PICK: AnswerKey = AnswerKey("pick");

/// The number typed: `12`, `#12` or `!12`. The entry of the menu of a remote branch gives it.
pub const NUMBER: AnswerKey = AnswerKey("number");

/// The name of the local branch the pull request is checked out in.
pub const NAME: AnswerKey = AnswerKey("name");

/// The choice of the pick that asks for a number instead.
pub const TYPE_NUMBER: &str = "type-number";

/// The pull request to check out: its number, and what the list says of it, when it was read.
struct Chosen {
    number: u64,
    entry: Option<PullListEntry>,
}

/// Checks out a pull request picked from those open, or whose number is typed: one from a branch of the repository with
/// `git fetch` and the checkout of its remote branch, so that a push goes to it; one from a fork, or whose branch is not
/// known, fetched by the reference of the service into a local branch whose name is asked, then checked out.
///
/// Feature: hosting.checkout-pull-request
#[must_use]
pub fn checkout_pull_request(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = crate::history::refuse_under_way(context) {
        return refusal;
    }
    if context.repository.bisect.is_some() {
        return Outcome::Refuse(Refusal::saying(crate::bisect::END_FIRST));
    }
    let Some(remotes) = answers.remotes(REMOTES) else {
        return Outcome::Read(REMOTES, Query::Remotes);
    };
    let Some(project) = detect_remote(remotes, &context.settings.hosting)
        .filter(|project| project.service.pull_request_reference(1).is_some())
    else {
        return Outcome::Refuse(Refusal::saying(
            "The main remote of this repository is not on GitHub, GitLab, Gitea or Forgejo.",
        ));
    };
    match chosen(answers, &project) {
        Ok(chosen) => checkout(context, answers, &project, &chosen),
        Err(outcome) => *outcome,
    }
}

/// The pull request chosen: the number given or typed, the one picked, or the question or read that comes first.
fn chosen(answers: &Answers, project: &HostingRepo) -> Result<Chosen, Box<Outcome>> {
    let listed = match answers.get(LIST) {
        Some(Answer::PullList(PullList::Listed(entries))) => Some(entries.as_slice()),
        _ => None,
    };
    let entry_of = |number: u64| {
        listed
            .and_then(|entries| entries.iter().find(|entry| entry.number == number))
            .cloned()
    };
    if let Some(number) = answers.text(NUMBER).and_then(typed_number) {
        return Ok(Chosen {
            number,
            entry: entry_of(number),
        });
    }
    match answers.choice(PICK) {
        Some(TYPE_NUMBER) => Err(Box::new(Outcome::Ask(number_question(project.service, None)))),
        Some(picked) => match typed_number(picked) {
            Some(number) => Ok(Chosen {
                number,
                entry: entry_of(number),
            }),
            None => Err(Box::new(Outcome::Ask(number_question(project.service, None)))),
        },
        None => Err(Box::new(match answers.get(LIST) {
            None => Outcome::Read(
                LIST,
                Query::PullRequestList {
                    repository: Box::new(project.clone()),
                },
            ),
            Some(Answer::PullList(PullList::Listed(entries))) if !entries.is_empty() => {
                Outcome::Ask(pick_question(project.service, entries))
            }
            Some(Answer::PullList(PullList::Unread(why))) => Outcome::Ask(number_question(project.service, Some(why))),
            Some(_) => Outcome::Ask(number_question(project.service, None)),
        })),
    }
}

/// The branch of the repository itself a pull request comes from: the one the list says, or the remote branch of the
/// main remote the menu was opened on; none for one from a fork, or not known.
fn own_branch<'a>(context: &'a Context, project: &HostingRepo, chosen: &'a Chosen) -> Option<&'a str> {
    match &chosen.entry {
        Some(entry) => entry.fork.is_none().then_some(entry.branch.as_str()),
        None => match &context.target {
            Target::RemoteBranch(name) => name
                .strip_prefix(project.remote.as_str())
                .and_then(|rest| rest.strip_prefix('/')),
            _ => None,
        },
    }
}

/// The plan of the checkout of the pull request chosen, or the name it is checked out in asked first.
fn checkout(context: &Context, answers: &Answers, project: &HostingRepo, chosen: &Chosen) -> Outcome {
    let state = &context.repository;
    let number = project.service.number(chosen.number);
    let words = project.service.pull_request_words();
    let title = format!("Check out {words} {number}");
    let own = own_branch(context, project, chosen);
    if let Some(branch) = own
        && !state.branches.iter().any(|local| local.name == branch)
    {
        let remote_branch = format!("{}/{branch}", project.remote);
        let steps = vec![
            Step::git(state.folder.clone(), ["fetch", project.remote.as_str(), branch]),
            Step::git(state.folder.clone(), ["checkout", "--track", remote_branch.as_str()]),
        ];
        return update_after_checkout(context, answers, &remote_branch, checkout_plan(title, steps));
    }
    let name = match (own, answers.text(NAME)) {
        (Some(branch), _) => branch.to_owned(),
        (None, Some(typed)) => typed.trim().to_owned(),
        (None, None) => return Outcome::Ask(name_question(context, project, chosen)),
    };
    if let Some(worktree) = other_worktree(state, &name) {
        let folder = worktree.folder.display();
        return Outcome::Refuse(Refusal::saying(&format!(
            "{name} is checked out in another worktree, {folder}: check out the {words} there."
        )));
    }
    let Some(reference) = project.service.pull_request_reference(chosen.number) else {
        return Outcome::Refuse(Refusal::saying(
            "This hosting service keeps no reference of its pull requests.",
        ));
    };
    let mut plan = fetched_plan(context, (project, &reference, &name), title);
    if own.is_none() {
        plan.after.push(FollowUp::Say(Text::from(format!(
            "Checked out {number} as {name}. It has no upstream: a push would make a branch {name} on {}.",
            project.remote
        ))));
    }
    let is_checked_out = state
        .branches
        .iter()
        .any(|local| local.name == name && local.is_checked_out);
    if is_checked_out {
        return Outcome::Run(plan);
    }
    update_after_checkout(context, answers, &name, plan)
}

/// The plan that fetches the reference of the pull request into the local branch and checks it out, or, when the
/// branch is checked out, pulls the reference into it.
fn fetched_plan(
    context: &Context,
    (project, reference, name): (&HostingRepo, &str, &str),
    title: String,
) -> model::contract::Plan {
    let state = &context.repository;
    let folder = state.folder.clone();
    let remote = project.remote.as_str();
    let is_checked_out = state
        .branches
        .iter()
        .any(|local| local.name == name && local.is_checked_out);
    let steps = if is_checked_out {
        vec![Step::git(
            folder,
            ["pull", "--ff-only", "--no-rebase", remote, reference],
        )]
    } else {
        let refspec = format!("{reference}:refs/heads/{name}");
        vec![
            Step::git(folder.clone(), ["fetch", remote, refspec.as_str()]),
            Step::git(folder, ["checkout", name]),
        ]
    };
    checkout_plan(title, steps)
}

/// The pick of the open pull requests, newest first, with a last choice that asks for a number.
fn pick_question(service: HostingService, entries: &[PullListEntry]) -> Question {
    let mut choices: Vec<Choice> = entries
        .iter()
        .map(|entry| {
            let branch = match entry.fork_owner() {
                Some(owner) => format!("{owner}:{}", entry.branch),
                None => entry.branch.clone(),
            };
            Choice {
                identifier: entry.number.to_string(),
                label: Text::from(format!("{} {}", service.number(entry.number), entry.title)),
                description: Some(Text::from(format!("{} \u{b7} {branch}", entry.author))),
            }
        })
        .collect();
    choices.push(Choice {
        identifier: TYPE_NUMBER.to_owned(),
        label: Text::from("Type its number\u{2026}"),
        description: None,
    });
    Question {
        key: PICK,
        kind: QuestionKind::Search { choices },
        title: Text::from(format!("Check out a {}", service.pull_request_words())),
        detail: Some(Text::from("Pick one, or type its number")),
        default: None,
    }
}

/// The box for the number of a pull request, with why none is listed when they could not be read.
fn number_question(service: HostingService, why: Option<&String>) -> Question {
    Question {
        key: NUMBER,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from(service.number(12))),
            validation: Validation::PullRequestNumber,
        },
        title: Text::from(format!("Check out a {}", service.pull_request_words())),
        detail: why.map(|why| Text::from(why.as_str())),
        default: None,
    }
}

/// The box for the name of the local branch, starting from `<owner of the fork>/<branch>`, or `pull/<n>`
/// (`merge-request/<n>` on GitLab) when the branch is not known.
fn name_question(context: &Context, project: &HostingRepo, chosen: &Chosen) -> Question {
    let proposed = match chosen
        .entry
        .as_ref()
        .and_then(|entry| Some((entry.fork_owner()?, &entry.branch)))
    {
        Some((owner, branch)) => format!("{owner}/{branch}"),
        None if project.service == HostingService::GitLab => format!("merge-request/{}", chosen.number),
        None => format!("pull/{}", chosen.number),
    };
    Question {
        key: NAME,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::PullRequestBranch {
                branches: context
                    .repository
                    .branches
                    .iter()
                    .map(|branch| branch.name.clone())
                    .collect(),
            },
        },
        title: Text::from(format!("Check out {} as", project.service.number(chosen.number))),
        detail: None,
        default: Some(Answer::Text(proposed)),
    }
}

#[cfg(test)]
#[path = "checkout_pull_request_tests.rs"]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
