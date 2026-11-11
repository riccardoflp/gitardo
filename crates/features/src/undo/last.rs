//! `undo.last`: takes back the last commit, amend, checkout, merge, pull, rebase or reset, also one made from a
//! terminal, by reading the reflog.
//!
//! The operations Gitardo makes that the reflog does not see (a branch or a stash deleted, conflicts resolved, a series
//! of cherry-picks) are taken back from the engine's journal, which comes with the cards that fill it.

use model::CommitId;
use model::contract::{
    AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, ReflogEntry, Refusal, Step, Text, Undo,
};

/// The last two entries of the reflog of `HEAD`.
pub const HEAD_REFLOG: AnswerKey = AnswerKey("head-reflog");

/// The last two entries of the reflog of the branch checked out: a rebase moves `HEAD` many times but the branch once.
pub const BRANCH_REFLOG: AnswerKey = AnswerKey("branch-reflog");

/// Whether the files of the working tree are as the previous position has them.
pub const FILES_MATCH: AnswerKey = AnswerKey("files-match");

/// Takes back the last move of `HEAD` or of the branch checked out, as their reflogs say, showing what it is about to
/// do first. Undoing twice redoes.
///
/// Feature: undo.last
#[must_use]
pub fn last(context: &Context, answers: &Answers) -> Outcome {
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
        return Outcome::Run(checkout.plan(context));
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
