//! `rebase.interactive`: runs `git rebase -i` with the todo list prepared on the interactive rebase page.

use model::CommitId;
use model::contract::{AnswerKey, Answers, Context, Outcome, Question, QuestionKind, Text, Validation};
use model::rebase::{LineCommit, LineToHead, Reword, TodoAction, TodoRow, TodoStep, build_todo_list, read_todo_rows};

use super::{Rewrite, current_name, files_folder, line_holding, refuse, refuse_under_way, rewrite_plan, stack};

/// The rows of the interactive rebase page, oldest first, as `model::rebase::write_todo_rows` writes them.
pub const TODO: AnswerKey = AnswerKey("todo");

/// Runs an interactive rebase of the rows answered under [`TODO`], which must name each commit from the oldest of them
/// up to `HEAD` once: the new messages and the todo list are written to the folder of the files of the rebase, and git's
/// editor of the todo list copies the prepared one over its own. It asks first, showing the todo list.
///
/// Feature: rebase.interactive
#[must_use]
pub fn interactive(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = refuse_under_way(context) {
        return refusal;
    }
    let Some(text) = answers.text(TODO) else {
        return Outcome::Ask(Question {
            key: TODO,
            kind: QuestionKind::Text {
                placeholder: None,
                validation: Validation::Required(Text::from("Enter the todo list")),
            },
            title: Text::from("Todo list of the interactive rebase"),
            detail: Some(Text::from(
                "One line a commit, oldest first: the action, then the full hash.",
            )),
            default: None,
        });
    };
    let Some(rows) = read_todo_rows(text).filter(|rows| !rows.is_empty()) else {
        return refuse("The todo list cannot be read: write one line a commit, the action, then the full hash.");
    };
    let named: Vec<CommitId> = rows.iter().map(|row| row.commit.clone()).collect();
    match line_holding(answers, &named) {
        Ok(line) => rebase_rows(context, answers, &rows, line),
        Err(read) => *read,
    }
}

/// The plan of the rows, once the line from the oldest of their commits up to `HEAD` is read.
fn rebase_rows(context: &Context, answers: &Answers, rows: &[TodoRow], line: &LineToHead) -> Outcome {
    let (Some(commits), Some(oldest)) = (&line.commits, line.commits.as_ref().and_then(|commits| commits.first()))
    else {
        return refuse(
            "The commits of the todo list are not on the line of commits before HEAD without merges: they cannot be \
             rebased from here.",
        );
    };
    let mut named: Vec<&str> = rows.iter().map(|row| row.commit.as_str()).collect();
    let mut expected: Vec<&str> = commits.iter().map(|commit| commit.id.as_str()).collect();
    named.sort_unstable();
    expected.sort_unstable();
    if named != expected {
        return refuse(&format!(
            "The todo list must name each commit from {} up to HEAD once.",
            oldest.id.short()
        ));
    }
    if rows
        .iter()
        .any(|row| row.action == TodoAction::Reword && row.new_subject.as_deref().is_none_or(str::is_empty))
    {
        return refuse("Enter a message for each reworded commit.");
    }
    let steps: Vec<TodoStep<'_>> = rows.iter().map(|row| step(row, commits)).collect();
    let moving = match stack::decide_todo(context, answers, line.base()) {
        Ok((_, moving)) => moving,
        Err(read) => return *read,
    };
    let written = files_folder(context).to_string_lossy().replace('\\', "/");
    let todo = match build_todo_list(&steps, &written, &moving) {
        Ok(list) => list.text,
        Err(refusal) => return refuse(refusal.as_str()),
    };
    let after = line
        .base()
        .map_or_else(|| String::from("the root"), |base| base.short().to_owned());
    let rewrite = Rewrite {
        steps: &steps,
        base: line.base(),
        sets_aside: false,
        title: format!("Interactive rebase of {}", current_name(context)),
        note: format!("This rewrites the commits after {after} with this todo list:\n\n{todo}"),
        after: Vec::new(),
    };
    match rewrite_plan(context, answers, rewrite) {
        Ok(plan) => Outcome::Run(plan),
        Err(outcome) => *outcome,
    }
}

/// The step of the todo list of a row; a reword keeps the body of the commit's message under its new subject.
fn step<'a>(row: &'a TodoRow, commits: &'a [LineCommit]) -> TodoStep<'a> {
    let original = commits
        .iter()
        .find(|commit| commit.id == row.commit)
        .map_or("", |commit| commit.message.as_str());
    TodoStep {
        action: row.action,
        commit: row.commit.as_str(),
        reword: row.new_subject.as_deref().map(|subject| Reword {
            message: subject,
            is_subject_only: true,
            original,
        }),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
