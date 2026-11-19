//! `undo.last`: takes back the last commit, amend, checkout, merge, pull, rebase or reset, also one made from a
//! terminal, by reading the reflog.
//!
//! The operations Gitardo makes that the reflog does not see (a branch or a stash deleted, conflicts resolved, a series
//! of cherry-picks) are taken back from the engine's journal, filled by the plans of the cards that make them.

use std::ffi::OsString;

use model::CommitId;
use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, JournalEntry, MovedBranch, Outcome, Plan, Query, ReflogEntry,
    Refusal, Step, Text, Undo, UndoRecord, moved_branches,
};

/// The newest entry of the engine's journal that still holds.
pub const JOURNAL: AnswerKey = AnswerKey("journal");

/// The last two entries of the reflog of `HEAD`.
pub const HEAD_REFLOG: AnswerKey = AnswerKey("head-reflog");

/// The last two entries of the reflog of the branch checked out: a rebase moves `HEAD` many times but the branch once.
pub const BRANCH_REFLOG: AnswerKey = AnswerKey("branch-reflog");

/// Whether the files of the working tree are as the previous position has them.
pub const FILES_MATCH: AnswerKey = AnswerKey("files-match");

/// Takes back the newest operation of Gitardo, as the engine's journal keeps it: what the reflog does not see from the
/// journal, a move of `HEAD` by the reflog; or, with nothing in the journal, the last move of `HEAD` or of the branch
/// checked out, as their reflogs say, also one made from a terminal. It refuses, rather than take back the operation
/// before it, when the newest one cannot be taken back, and while a merge, a rebase, a cherry-pick, a revert or a patch
/// is under way, but for a squash merge or a resolution of conflicts, which are taken back from where they stopped.
/// Showing what it is about to do first. With nothing in the journal, undoing twice redoes.
///
/// Feature: undo.last
#[must_use]
pub fn last(context: &Context, answers: &Answers) -> Outcome {
    if context.repository.bisect.is_some() {
        // Undo would move `HEAD` under the bisect, which then tests the wrong commit (`bisect.start`).
        return Outcome::Refuse(Refusal::saying(crate::bisect::END_FIRST));
    }
    let Some(newest) = answers.journal(JOURNAL) else {
        return Outcome::Read(JOURNAL, Query::Journal);
    };
    let mid_operation = matches!(
        newest.map(|entry| &entry.record),
        Some(UndoRecord::SquashMerge | UndoRecord::ConflictsResolved { .. })
    );
    if !mid_operation && let Some(refusal) = crate::history::refuse_under_way(context) {
        return refusal;
    }
    match newest {
        Some(JournalEntry {
            record: UndoRecord::NotUndoable { reason },
            title,
            ..
        }) => Outcome::Refuse(Refusal::saying(&format!("\"{title}\" cannot be undone. {reason}"))),
        Some(JournalEntry {
            record: UndoRecord::Reflog,
            number,
            ..
        }) => match by_reflog(context, answers) {
            Outcome::Run(plan) => Outcome::Run(Plan {
                undo: Undo::TakesBack(*number),
                ..plan
            }),
            other => other,
        },
        Some(JournalEntry {
            record: UndoRecord::Rewrite { moved },
            number,
            ..
        }) => match by_reflog(context, answers) {
            Outcome::Run(plan) => Outcome::Run(with_moved_back(
                context,
                Plan {
                    undo: Undo::TakesBack(*number),
                    ..plan
                },
                moved,
            )),
            other => other,
        },
        Some(entry) => Outcome::Run(super::journal::plan(context, entry)),
        None => by_reflog(context, answers),
    }
}

/// Takes back the last move of `HEAD` or of the branch checked out, as their reflogs say.
fn by_reflog(context: &Context, answers: &Answers) -> Outcome {
    let Some(head) = answers.reflog(HEAD_REFLOG) else {
        return Outcome::Read(
            HEAD_REFLOG,
            Query::Reflog {
                reference: String::from("HEAD"),
                count: 2,
            },
        );
    };
    let Some(newest) = head.first() else {
        return nothing_to_undo();
    };
    if let Some(checkout) = Checkout::parse(&newest.subject) {
        // Back on what was checked out before, the submodules go to the commits it records, as after any checkout.
        return crate::branch::submodules::update_after_checkout(
            context,
            answers,
            checkout.from,
            checkout.plan(context),
        );
    }
    let entries = match context.repository.checked_out_branch() {
        Some(branch) => {
            let Some(entries) = answers.reflog(BRANCH_REFLOG) else {
                let reference = format!("refs/heads/{branch}");
                return Outcome::Read(BRANCH_REFLOG, Query::Reflog { reference, count: 2 });
            };
            entries
        }
        None => head,
    };
    let (Some(undone), Some(previous)) = (entries.first(), entries.get(1)) else {
        return nothing_to_undo();
    };
    match reset_mode(context, answers, undone, &previous.commit) {
        Ok(mode) => Outcome::Run(reset_plan(context, undone, &previous.commit, mode)),
        Err(query) => Outcome::Read(FILES_MATCH, query),
    }
}

/// The Undo of a rewrite by the reflog, then the other branches it moved put back in one
/// `git update-ref -m "undo: back to before the rebase" --stdin`, which git makes whole or not at all; a branch moved
/// since stays where it is, and the dialog says so (`undo.moved-branches`).
fn with_moved_back(context: &Context, mut plan: Plan, moved: &[MovedBranch]) -> Plan {
    let undo = moved_branches(moved, &context.repository);
    let mut said = Vec::new();
    if !undo.back.is_empty() {
        plan.steps.push(Step::Git {
            arguments: ["update-ref", "-m", "undo: back to before the rebase", "--stdin"]
                .map(OsString::from)
                .to_vec(),
            directory: context.repository.folder.clone(),
            input: Some(undo.lines.into_bytes()),
        });
        said.push(format!(
            "Undo the rebase of `{}`, and put {} back where {} were.",
            crate::rebase::current_name(context),
            and_list(&undo.back),
            if undo.back.len() == 1 { "it" } else { "they" }
        ));
    }
    said.extend(
        undo.moved_since
            .iter()
            .map(|name| format!("`{name}` moved since the rebase and stays where it is.")),
    );
    if !said.is_empty() {
        plan.danger = Danger::Destructive(Text::from(said.join(" ")));
    }
    plan
}

/// `` `a` ``, `` `a` and `b` ``, `` `a`, `b` and `c` ``.
fn and_list(names: &[String]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match quoted.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn nothing_to_undo() -> Outcome {
    Outcome::Refuse(Refusal::saying("Nothing to undo"))
}

/// A checkout, as the reflog of `HEAD` writes it: `checkout: moving from <from> to <to>`.
struct Checkout<'a> {
    from: &'a str,
    to: &'a str,
}

impl<'a> Checkout<'a> {
    fn parse(subject: &'a str) -> Option<Self> {
        let (from, to) = subject.strip_prefix("checkout: moving from ")?.rsplit_once(" to ")?;
        Some(Checkout { from, to })
    }

    /// `git checkout <from>`, or `git checkout --detach <hash>` when what was checked out before was a commit.
    fn plan(&self, context: &Context) -> Plan {
        let arguments = if CommitId::parse(self.from).is_some() {
            vec!["checkout", "--detach", self.from]
        } else {
            vec!["checkout", self.from]
        };
        let what = format!("the checkout of {}", self.to);
        undo_plan(
            &what,
            vec![Step::git(context.repository.folder.clone(), arguments)],
            Vec::new(),
        )
    }
}

/// How `git reset` takes a move back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResetMode {
    /// A commit or an amend: its changes stay staged.
    Soft,
    /// The files already are as the previous position has them: only `HEAD` and the index go back.
    Mixed,
    /// Anything else: git refuses when it would overwrite changes not committed.
    Keep,
}

/// The mode of the reset, or the query that must be read, under [`FILES_MATCH`], before it can be told.
fn reset_mode(
    context: &Context,
    answers: &Answers,
    undone: &ReflogEntry,
    previous: &CommitId,
) -> Result<ResetMode, Query> {
    let subject = undone.subject.as_str();
    if subject.starts_with("commit: ") || subject.starts_with("commit (amend): ") {
        return Ok(ResetMode::Soft);
    }
    let working_tree = &context.repository.working_tree;
    let has_uncommitted =
        !(working_tree.staged.is_empty() && working_tree.unstaged.is_empty() && working_tree.conflicted.is_empty());
    if !has_uncommitted {
        return Ok(ResetMode::Keep);
    }
    match answers.flag(FILES_MATCH) {
        Some(true) => Ok(ResetMode::Mixed),
        Some(false) => Ok(ResetMode::Keep),
        None => Err(Query::FilesMatch {
            commit: previous.clone(),
        }),
    }
}

fn reset_plan(context: &Context, undone: &ReflogEntry, previous: &CommitId, mode: ResetMode) -> Plan {
    let flag = match mode {
        ResetMode::Soft => "--soft",
        ResetMode::Mixed => "--mixed",
        ResetMode::Keep => "--keep",
    };
    let step = Step::git(context.repository.folder.clone(), ["reset", flag, previous.as_str()]);
    let on_failure = if mode == ResetMode::Keep && !context.repository.working_tree.is_clean() {
        vec![FollowUp::Say(Text::from(
            "Undo would overwrite changes that are not committed: commit or stash them first",
        ))]
    } else {
        Vec::new()
    };
    undo_plan(&describe(&undone.subject), vec![step], on_failure)
}

/// What a reflog entry did, in words: `the commit "Fix the parser"`.
fn describe(subject: &str) -> String {
    if let Some(message) = subject.strip_prefix("commit: ") {
        format!("the commit \"{message}\"")
    } else if let Some(message) = subject.strip_prefix("commit (amend): ") {
        format!("the amend of \"{message}\"")
    } else {
        format!("\"{subject}\"")
    }
}

fn undo_plan(what: &str, steps: Vec<Step>, on_failure: Vec<FollowUp>) -> Plan {
    let title = Text::from(format!("Undo {what}"));
    Plan {
        title: title.clone(),
        steps,
        danger: Danger::Destructive(title),
        undo: Undo::ByReflog,
        after: vec![
            FollowUp::SelectHead,
            FollowUp::Say(Text::from(format!("Undid {what}."))),
        ],
        on_failure,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
