//! The state of a repository, a plan and a question, as lines of text.

use std::io::{self, Write};

use model::contract::{Danger, Plan, Question, QuestionKind};
use model::state::{Branch, Operation, RepositoryState, WorkingTree};

/// Writes the state of a repository: where it is, what is checked out, its references and its changes.
pub fn state(out: &mut dyn Write, state: &RepositoryState) -> io::Result<()> {
    writeln!(out, "Repository  {}", state.folder.display())?;
    writeln!(out, "HEAD        {}", head(state))?;
    if let Some(operation) = state.operation {
        writeln!(out, "Operation   {} in progress", operation_name(operation))?;
    }
    if let Some(stop) = &state.rebase_stop {
        let why = match stop.kind {
            model::rebase::StopKind::Edit => "edit",
            model::rebase::StopKind::Split => "split",
        };
        writeln!(out, "Stopped     to {why} {} \"{}\"", stop.commit.short(), stop.subject)?;
    }
    writeln!(out, "Changes     {}", changes(&state.working_tree))?;
    writeln!(out, "Branches    {}", state.branches.len())?;
    for branch in &state.branches {
        writeln!(out, "  {}", branch_line(branch))?;
    }
    writeln!(out, "Remotes     {}", state.remotes.join(", "))?;
    writeln!(out, "Remote branches {}", state.remote_branches.len())?;
    for branch in &state.remote_branches {
        writeln!(out, "  {}  {}", branch.name, branch.commit.short())?;
    }
    writeln!(out, "Tags        {}", state.tags.len())?;
    for tag in &state.tags {
        writeln!(out, "  {}  {}", tag.name, tag.commit.short())?;
    }
    writeln!(out, "Stashes     {}", state.stashes.len())?;
    for stash in &state.stashes {
        writeln!(out, "  stash@{{{}}}  {}", stash.index, stash.message)?;
    }
    writeln!(out, "Worktrees   {}", state.worktrees.len())?;
    for worktree in &state.worktrees {
        let branch = worktree.branch.as_deref().unwrap_or("detached");
        writeln!(out, "  {}  {branch}", worktree.folder.display())?;
    }
    Ok(())
}

fn head(state: &RepositoryState) -> String {
    let commit = state.head.commit.as_ref().map(model::CommitId::short);
    match (state.checked_out_branch(), commit) {
        (Some(branch), Some(commit)) => format!("{branch} at {commit}"),
        (Some(branch), None) => format!("{branch}, with no commit yet"),
        (None, Some(commit)) => format!("detached at {commit}"),
        (None, None) => String::from("nothing checked out"),
    }
}

fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Merge => "Merge",
        Operation::Rebase => "Rebase",
        Operation::CherryPick => "Cherry-pick",
        Operation::Revert => "Revert",
        Operation::Patch => "Patch",
    }
}

fn changes(working_tree: &WorkingTree) -> String {
    if working_tree.is_clean() {
        return String::from("none");
    }
    format!(
        "{} staged, {} not staged, {} untracked, {} in conflict",
        working_tree.staged.len(),
        working_tree.unstaged.len(),
        working_tree.untracked.len(),
        working_tree.conflicted.len()
    )
}

fn branch_line(branch: &Branch) -> String {
    let mark = if branch.is_checked_out { '*' } else { ' ' };
    let line = format!("{mark} {}  {}", branch.name, branch.commit.short());
    match &branch.upstream {
        Some(upstream) if upstream.is_gone => format!("{line}  {} (gone)", upstream.reference),
        Some(upstream) => format!(
            "{line}  {} +{} -{}",
            upstream.reference, upstream.ahead, upstream.behind
        ),
        None => line,
    }
}

/// Writes a plan: its title, what it warns of, and the line of each step.
pub fn plan(out: &mut dyn Write, plan: &Plan) -> io::Result<()> {
    writeln!(out, "{}", plan.title)?;
    if let Danger::Confirm(warning) | Danger::Destructive(warning) = &plan.danger
        && warning != &plan.title
    {
        writeln!(out, "  {warning}")?;
    }
    for step in &plan.steps {
        writeln!(out, "  > {}", step.log_line())?;
    }
    Ok(())
}

/// Writes a question no argument answered, with the argument that would answer it.
pub fn question(out: &mut dyn Write, question: &Question) -> io::Result<()> {
    writeln!(out, "{}", question.title)?;
    if let Some(detail) = &question.detail {
        writeln!(out, "  {detail}")?;
    }
    let key = question.key.0;
    match &question.kind {
        QuestionKind::Text { placeholder, .. } => {
            let example = placeholder
                .as_ref()
                .map_or("<text>", |placeholder| placeholder.as_str());
            writeln!(out, "  Answer with {key}={example}")?;
        }
        QuestionKind::Pick { choices } | QuestionKind::PickMany { choices } => {
            writeln!(out, "  Answer with one of:")?;
            for choice in choices {
                writeln!(out, "    {key}={}  {}", choice.identifier, choice.label)?;
            }
        }
        QuestionKind::Confirm { accept } => writeln!(out, "  Answer {key}=yes to {accept}")?,
    }
    Ok(())
}
