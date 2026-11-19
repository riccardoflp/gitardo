//! `review.write`: the resolutions accepted on the review written in their files as one operation, and the files left
//! with no conflict staged.

use model::changes::pathspec;
use model::contract::{
    AnswerKey, Answers, CommandPreview, Context, Danger, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step,
    Target, Text, Undo, UndoRecord,
};
use model::review::{FileResolutions, Resolutions};

/// The key of the files read again before they are written.
pub const FILES: AnswerKey = AnswerKey("files");

/// The key of the confirmation asked first when `commandPreview` is `all`.
pub const WRITE: AnswerKey = AnswerKey("write");

/// What is said when nothing is accepted.
const NOTHING_ACCEPTED: &str = "No resolution is accepted.";

/// Writes the accepted resolutions in their files, each file only while it is still the text they were made from,
/// then stages with `git add` the files left with no conflict. The plan starts with what it does to each file, which
/// the log keeps with the `write` lines. Undo marks the conflicts of the files written again. With `commandPreview` set
/// to `all` the plan is asked first, as its own question, and the files are read again only once it is answered, so
/// that a file saved while the question was open is not written over; it is then not shown again.
///
/// Feature: review.write
#[must_use]
pub fn write(context: &Context, answers: &Answers) -> Outcome {
    let Target::Resolutions(resolutions) = &context.target else {
        return Outcome::Refuse(Refusal::saying(NOTHING_ACCEPTED));
    };
    let written: Vec<&FileResolutions> = resolutions
        .files
        .iter()
        .filter(|file| !file.accepted.is_empty())
        .collect();
    if written.is_empty() && resolutions.recorded.is_empty() {
        return Outcome::Refuse(Refusal::saying(NOTHING_ACCEPTED));
    }
    let mut texts = Vec::with_capacity(written.len());
    for file in &written {
        match file.resolved() {
            Ok(text) => texts.push(text),
            Err(_) => {
                return Outcome::Refuse(Refusal::saying(&format!(
                    "Two resolutions accepted in {} take the place of the same lines, so nothing was written.",
                    file.path
                )));
            }
        }
    }
    let asks_first = context.settings.command_preview == CommandPreview::All;
    let mut plan = plan(context, resolutions, (&written, texts));
    if asks_first && !answers.is_confirmed(WRITE) {
        return Outcome::Ask(confirmation(&plan));
    }
    let folder = &context.repository.folder;
    let Some(on_disk) = answers.files(FILES) else {
        let paths = written.iter().map(|file| folder.join(&file.path)).collect();
        return Outcome::Read(FILES, Query::FileTexts { paths });
    };
    let changed = written
        .iter()
        .zip(on_disk.iter().map(Option::as_deref).chain(std::iter::repeat(None)))
        .find(|(file, text)| *text != Some(file.text.as_bytes()));
    if let Some((file, _)) = changed {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{} has changed since it was read, so nothing was written. Try again.",
            file.path
        )));
    }
    if asks_first {
        plan.danger = Danger::Agreed;
    }
    Outcome::Run(plan)
}

/// The plan asked as a question, before the files are read again: its title, then its steps as the log writes them.
fn confirmation(plan: &Plan) -> Question {
    let lines: Vec<String> = plan.steps.iter().map(Step::log_line).collect();
    Question {
        key: WRITE,
        kind: QuestionKind::Confirm {
            accept: Text::from("Write"),
        },
        title: Text::from(format!("{}?", plan.title)),
        detail: Some(Text::from(lines.join("\n"))),
        default: None,
    }
}

/// The plan: what it does to each file, the files written, then those left with no conflict staged.
fn plan(context: &Context, resolutions: &Resolutions, (written, texts): (&[&FileResolutions], Vec<String>)) -> Plan {
    let folder = &context.repository.folder;
    let mut steps: Vec<Step> = detail(resolutions)
        .into_iter()
        .map(|words| Step::Warn { words })
        .collect();
    steps.extend(written.iter().zip(texts).map(|(file, text)| Step::Write {
        path: folder.join(&file.path),
        contents: text.into_bytes(),
    }));
    let whole = written.iter().filter(|file| file.is_whole()).map(|file| &file.path);
    let staged: Vec<&String> = whole.chain(&resolutions.recorded).collect();
    if !staged.is_empty() {
        let mut arguments = vec![std::ffi::OsString::from("add"), std::ffi::OsString::from("--")];
        arguments.extend(staged.iter().map(|path| pathspec(path)));
        steps.push(Step::git(folder.clone(), arguments));
    }
    let title = title(resolutions);
    let mut files: Vec<String> = written.iter().map(|file| file.path.clone()).collect();
    files.extend(resolutions.recorded.iter().cloned());
    Plan {
        title: Text::from(title),
        steps,
        danger: Danger::None,
        undo: Undo::Journal(UndoRecord::ConflictsResolved { files }),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}

/// The title of the plan: the resolutions written, or, with none, the files git resolved as before staged.
fn title(resolutions: &Resolutions) -> String {
    match (resolutions.accepted(), resolutions.recorded.len()) {
        (0, 1) => String::from("Stage the file git resolved as before"),
        (0, recorded) => format!("Stage the {recorded} files git resolved as before"),
        (1, _) => String::from("Write the resolution of 1 conflict"),
        (accepted, _) => format!("Write the resolutions of {accepted} conflicts"),
    }
}

/// What the write does, in sentences: for each file how many of its conflicts and by whom, how many are left, and a
/// warning when the AI wrote some.
#[must_use]
pub fn detail(resolutions: &Resolutions) -> Vec<String> {
    let mut lines: Vec<String> = resolutions
        .files
        .iter()
        .filter(|file| !file.accepted.is_empty())
        .map(|file| {
            let total = file.conflicts.max(file.accepted.len());
            let noun = if total == 1 { "conflict" } else { "conflicts" };
            format!(
                "{}: {} of {total} {noun} ({})",
                file.path,
                file.accepted.len(),
                by_whom(file)
            )
        })
        .collect();
    lines.extend(
        resolutions
            .recorded
            .iter()
            .map(|path| format!("{path}: staged as git resolved it before")),
    );
    lines.push(match resolutions.left() {
        0 => String::from("No conflict will be left."),
        1 => String::from("1 conflict will be left to resolve by hand, with the markers as they are."),
        left => format!("{left} conflicts will be left to resolve by hand, with the markers as they are."),
    });
    let (accepted, from_ai) = (resolutions.accepted(), resolutions.from_ai());
    if from_ai > 0 && from_ai == accepted {
        lines.push(String::from(
            "All of them were written by a language model: look at them before you commit.",
        ));
    } else if from_ai > 0 {
        lines.push(format!(
            "{from_ai} of them were written by a language model: look at them before you commit."
        ));
    }
    lines
}

/// Who proposed the resolutions accepted in a file: **by the rules**, **by the AI**, **chosen by you**, or how many
/// each when they are several.
fn by_whom(file: &FileResolutions) -> String {
    let (rules, ai, chosen) = file.sources();
    let parts: Vec<String> = [(rules, "by the rules"), (ai, "by the AI"), (chosen, "chosen by you")]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, words)| format!("{count} {words}"))
        .collect();
    if let [only] = parts.as_slice() {
        only.split_once(' ').map_or(only.clone(), |(_, words)| words.to_owned())
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;
