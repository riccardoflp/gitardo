//! `conflicts.resolve-automatically`: writes the resolutions of the rules in the files at once, after saying what they
//! are, with a way to look at each one first.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::PathBuf;

use model::changes::pathspec;
use model::conflicts::ConflictTexts;
use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FeatureId, FollowUp, Outcome, Plan, Query, Question, QuestionKind,
    Refusal, Step, Text, Undo, UndoRecord,
};

use super::rules::{FileResolution, case_words, resolve_files};
use super::{noun, refuse};

/// The files in conflict, read before the question.
pub const TEXTS: AnswerKey = AnswerKey("texts");

/// The answer of the question: `resolve` or `review`.
pub const RESOLVE: AnswerKey = AnswerKey("resolve");

/// The files in conflict, read again once the question is answered, to write nothing in a file changed meanwhile.
pub const AGAIN: AnswerKey = AnswerKey("again");

/// Resolves the conflicts that need no choice in every file in conflict (`conflicts.resolve-by-rules`): it says first
/// how many each file has and why, then writes the files and stages those left with no conflict. **Review** opens the
/// review of the same files and writes nothing; a file changed since it was read is not written, nor any other. The
/// question is the confirmation: neither answer is shown again before it runs, whatever `commandPreview` says.
///
/// Feature: conflicts.resolve-automatically
#[must_use]
pub fn resolve_automatically(context: &Context, answers: &Answers) -> Outcome {
    if !context.settings.conflicts_auto_resolve {
        return Outcome::Refuse(Refusal {
            message: Text::from(
                "Resolving conflicts automatically is experimental, and it is switched off in the settings.",
            ),
            offer: Some((FeatureId("settings.edit"), Text::from("Open settings"))),
        });
    }
    let Some(texts) = answers.conflict_texts(TEXTS) else {
        return Outcome::Read(TEXTS, Query::ConflictTexts);
    };
    let files = resolve_files(texts);
    let total: usize = files.iter().map(|file| file.conflicts).sum();
    let resolvable: usize = files.iter().map(|file| file.resolved).sum();
    if total == 0 {
        return refuse("There are no conflicts marked in the files to resolve.");
    }
    if resolvable == 0 {
        return refuse(&match total {
            1 => String::from("The conflict cannot be resolved without choosing a side."),
            total => format!("None of the {total} conflicts can be resolved without choosing a side."),
        });
    }
    let written: Vec<&FileResolution> = files.iter().filter(|file| file.text.is_some()).collect();
    let staged: Vec<&str> = written
        .iter()
        .filter(|file| file.is_resolved_in_full())
        .map(|file| file.path.as_str())
        .collect();
    let title = title(resolvable, total);
    match answers.choice(RESOLVE) {
        None => {
            let detail = detail(&written, &staged, total - resolvable);
            return Outcome::Ask(question(&title, detail));
        }
        Some("review") => return Outcome::Run(review_plan(&files)),
        Some(_) => {}
    }
    let Some(again) = answers.conflict_texts(AGAIN) else {
        return Outcome::Read(AGAIN, Query::ConflictTexts);
    };
    if let Some(changed) = written.iter().find(|file| has_changed(texts, again, &file.path)) {
        return refuse(&format!(
            "{} has changed since it was read, so nothing was written. Try again.",
            changed.path
        ));
    }
    Outcome::Run(write_plan(context, &title, (&written, &staged)))
}

/// The title of the question: **Resolve all 3 conflicts automatically?**.
fn title(resolvable: usize, total: usize) -> String {
    match (resolvable, total) {
        (1, 1) => String::from("Resolve the conflict automatically?"),
        (resolvable, total) if resolvable == total => format!("Resolve all {total} conflicts automatically?"),
        (resolvable, total) => format!("Resolve {resolvable} of {total} conflicts automatically?"),
    }
}

/// What the question says: for each file, how many of its conflicts are resolved and why; that every line is one of
/// the sides; what is left; and what is written and run.
fn detail(written: &[&FileResolution], staged: &[&str], left: usize) -> String {
    let mut lines: Vec<String> = written
        .iter()
        .map(|file| {
            let how_many = match (file.resolved, file.conflicts) {
                (1, 1) => String::from("the conflict"),
                (resolved, conflicts) if resolved == conflicts => format!("all {conflicts} conflicts"),
                (resolved, conflicts) => format!("{resolved} of {conflicts} conflicts"),
            };
            let cases: Vec<&str> = file.cases.iter().map(|case| case_words(*case)).collect();
            format!("{}: {how_many} ({})", file.path, cases.join(", "))
        })
        .collect();
    lines.push(String::from("Every line written is a line of one of the two sides."));
    lines.push(match left {
        0 => String::from("No conflict will be left."),
        1 => String::from("1 conflict will be left to resolve by hand, with the markers as they are."),
        left => format!("{left} conflicts will be left to resolve by hand, with the markers as they are."),
    });
    let paths: Vec<&str> = written.iter().map(|file| file.path.as_str()).collect();
    lines.push(if staged.is_empty() {
        format!("Writes {}.", paths.join(", "))
    } else {
        format!("Writes {}, then runs {}.", paths.join(", "), add_line(staged))
    });
    lines.join("\n")
}

/// The command that stages the files left with no conflict, as the log writes it.
fn add_line(staged: &[&str]) -> String {
    let mut line = String::from("git add --");
    for path in staged {
        line.push(' ');
        line.push_str(&pathspec(path).to_string_lossy());
    }
    line
}

/// The question: **Resolve** writes, **Review** opens the review.
fn question(title: &str, detail: String) -> Question {
    let choice = |identifier: &str, label: &str| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: None,
    };
    Question {
        key: RESOLVE,
        kind: QuestionKind::Pick {
            choices: vec![choice("resolve", "Resolve"), choice("review", "Review")],
        },
        title: Text::from(title),
        detail: Some(Text::from(detail)),
        default: Some(model::contract::Answer::Choice(String::from("resolve"))),
    }
}

/// **Review**: the review of the files with conflicts marked opens, and nothing is written.
fn review_plan(files: &[FileResolution]) -> Plan {
    Plan {
        title: Text::from("Review the resolutions"),
        steps: Vec::new(),
        danger: Danger::Agreed,
        undo: Undo::Impossible(Text::from("Nothing was written.")),
        after: vec![FollowUp::OpenReview(
            files
                .iter()
                .filter(|file| file.conflicts > 0)
                .map(|file| file.path.clone())
                .collect(),
        )],
        on_failure: Vec::new(),
    }
}

/// Whether a file is not on disk as it was read before the question.
fn has_changed(before: &[ConflictTexts], after: &[ConflictTexts], path: &str) -> bool {
    let file = |texts: &[ConflictTexts]| {
        texts
            .iter()
            .find(|text| text.path == path)
            .map(|text| text.file.clone())
    };
    file(before) != file(after)
}

/// The files written, then those left with no conflict staged.
fn write_plan(context: &Context, title: &str, (written, staged): (&[&FileResolution], &[&str])) -> Plan {
    let folder = &context.repository.folder;
    let mut steps: Vec<Step> = written
        .iter()
        .filter_map(|file| {
            Some(Step::Write {
                path: file
                    .path
                    .split('/')
                    .fold(PathBuf::from(folder), |path, name| path.join(name)),
                contents: file.text.clone()?,
            })
        })
        .collect();
    if !staged.is_empty() {
        let mut arguments = vec![OsString::from("add"), OsString::from("--")];
        arguments.extend(staged.iter().map(|path| pathspec(path)));
        steps.push(Step::git(folder.clone(), arguments));
    }
    let still = context
        .repository
        .working_tree
        .conflicted
        .len()
        .saturating_sub(staged.len());
    let files = |count: usize| match count {
        1 => String::from("1 file"),
        count => format!("{count} files"),
    };
    let mut said = format!("Gitardo resolved conflicts in {}.", files(written.len()));
    let mut after = Vec::new();
    match (still, context.repository.operation) {
        (0, Some(operation)) => {
            let _ = write!(
                said,
                " No file is left in conflict: the {} can continue.",
                noun(operation)
            );
            after.push(FollowUp::Offer(
                FeatureId("conflicts.continue-operation"),
                Text::from("Continue"),
            ));
        }
        (0, None) => said.push_str(" No file is left in conflict."),
        (1, _) => said.push_str(" 1 file still has conflicts to resolve by hand."),
        (still, _) => {
            let _ = write!(said, " {still} files still have conflicts to resolve by hand.");
        }
    }
    after.insert(0, FollowUp::Say(Text::from(said)));
    Plan {
        title: Text::from(title.trim_end_matches('?')),
        steps,
        // The question said what is written and run: it was the confirmation, whatever `commandPreview` says.
        danger: Danger::Agreed,
        undo: Undo::Journal(UndoRecord::ConflictsResolved {
            files: written.iter().map(|file| file.path.clone()).collect(),
        }),
        after,
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
