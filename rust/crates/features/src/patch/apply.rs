//! `patch.apply`: a patch from a file or from the clipboard, applied as commits or to the working tree only.

use std::path::{Path, PathBuf};

use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FileFilter, FileQuestion, FollowUp, Outcome, Plan, Query, Question,
    QuestionKind, Refusal, Step, Text, Undo, UndoRecord,
};
use model::patch::{PatchCheck, PatchInput};

/// The key of where the patch comes from: `file` or `clipboard`.
pub const FROM: AnswerKey = AnswerKey("from");
/// The key of the file chosen.
pub const FILE: AnswerKey = AnswerKey("file");
/// The key of the bytes of the file chosen.
pub const CONTENTS: AnswerKey = AnswerKey("contents");
/// The key of the text of the clipboard.
pub const CLIPBOARD: AnswerKey = AnswerKey("clipboard");
/// The key of how a patch of commits is applied: `commits` or `working-tree`.
pub const WAY: AnswerKey = AnswerKey("as");
/// The key of what `git apply --check --3way` says of the patch.
const CHECK: AnswerKey = AnswerKey("check");
/// The key of the temporary file the text of the clipboard is written to.
pub const TEMPORARY: AnswerKey = AnswerKey("temporary");

/// How many files or reasons are named before **…and K more**.
const NAMED: usize = 15;

/// The patch to apply, once read: its name, its text, and the file git is given.
struct Source<'a> {
    /// The file's name, or **the patch in the clipboard**.
    name: String,
    /// The text of the patch.
    text: &'a [u8],
    /// The file chosen; none for the clipboard, whose text is written to a temporary file.
    file: Option<&'a Path>,
}

/// Applies a patch from a file or from the clipboard. A patch made by `git format-patch` is applied as commits with
/// `git am --3way`, or to the working tree only with `git apply --3way`, as chosen; any other patch to the working
/// tree only. `patch.check` runs first: a patch that does not apply is refused with git's reasons, and one that will
/// stop on conflicts names the files and asks first. The text of the clipboard goes through a temporary file, deleted
/// after git ran whether it succeeded or not.
///
/// Feature: patch.apply
#[must_use]
pub fn apply(context: &Context, answers: &Answers) -> Outcome {
    let source = match source(context, answers) {
        Ok(source) => source,
        Err(outcome) => return *outcome,
    };
    let text = String::from_utf8_lossy(source.text);
    if !has_patch(&text) {
        let message = match source.file {
            Some(_) => format!("{} has no patch in it.", source.name),
            None => String::from("There is no patch in the clipboard."),
        };
        return Outcome::Refuse(Refusal::saying(&message));
    }
    let commits = text::patch::count_commits(&text);
    let as_commits = match (commits, answers.choice(WAY)) {
        (0, _) | (_, Some("working-tree")) => false,
        (_, Some("commits")) => true,
        _ => return Outcome::Ask(way(context, &source.name, commits)),
    };
    if as_commits {
        if let Some(refusal) = crate::bisect::refuse_while_operation(context) {
            return refusal;
        }
        if !context.repository.working_tree.staged.is_empty() {
            return Outcome::Refuse(Refusal::saying(
                "git am applies patches only while nothing is staged: commit or unstage the staged changes first.",
            ));
        }
    }
    let Some(check) = answers.patch_check(CHECK) else {
        let patch = match source.file {
            Some(file) => PatchInput::File(file.to_path_buf()),
            None => PatchInput::Text(source.text.to_vec()),
        };
        return Outcome::Read(CHECK, Query::PatchCheck { patch });
    };
    let conflicts = match check {
        PatchCheck::Applies { conflicts } => conflicts,
        PatchCheck::Refused { reasons } => return refused(&source.name, reasons),
    };
    let file = match source.file {
        Some(file) => file.to_path_buf(),
        None => match answers.path(TEMPORARY) {
            Some(temporary) => temporary.to_path_buf(),
            None => {
                return Outcome::Read(
                    TEMPORARY,
                    Query::TemporaryFile {
                        extension: String::from("patch"),
                    },
                );
            }
        },
    };
    Outcome::Run(plan(context, &source, (as_commits, commits), (&file, conflicts)))
}

/// Where the patch comes from, and its text once read: the file chosen, or the clipboard.
fn source<'a>(context: &Context, answers: &'a Answers) -> Result<Source<'a>, Box<Outcome>> {
    match answers.choice(FROM) {
        Some("file") => {
            let Some(file) = answers.path(FILE) else {
                return Err(Box::new(Outcome::Ask(open_file(context))));
            };
            let Some(text) = answers.contents(CONTENTS) else {
                let path = file.to_path_buf();
                return Err(Box::new(Outcome::Read(CONTENTS, Query::FileContents { path })));
            };
            let name = file.file_name().map_or_else(
                || file.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            Ok(Source {
                name,
                text,
                file: Some(file),
            })
        }
        Some("clipboard") => match answers.clipboard(CLIPBOARD) {
            None => Err(Box::new(Outcome::Read(CLIPBOARD, Query::Clipboard))),
            Some(text) => Ok(Source {
                name: String::from("the patch in the clipboard"),
                text: text.unwrap_or_default().as_bytes(),
                file: None,
            }),
        },
        _ => Err(Box::new(Outcome::Ask(from()))),
    }
}

/// Whether a text holds a patch: a line that starts with `diff --git `, or with `--- ` followed by a name.
fn has_patch(text: &str) -> bool {
    text.lines().any(|line| {
        line.starts_with("diff --git ") || line.strip_prefix("--- ").is_some_and(|name| !name.trim().is_empty())
    })
}

/// **Apply patch**: **From a file…** or **From the clipboard**.
fn from() -> Question {
    let choice = |identifier: &str, label: &str| Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: None,
    };
    Question {
        key: FROM,
        kind: QuestionKind::Pick {
            choices: vec![
                choice("file", "From a file\u{2026}"),
                choice("clipboard", "From the clipboard"),
            ],
        },
        title: Text::from("Apply patch"),
        detail: None,
        default: None,
    }
}

/// The open dialog of **Apply patch**, in the top folder of the repository.
fn open_file(context: &Context) -> Question {
    let filter = |name: &str, extensions: &[&str]| FileFilter {
        name: Text::from(name),
        extensions: extensions.iter().map(|&extension| extension.to_owned()).collect(),
    };
    Question {
        key: FILE,
        kind: QuestionKind::OpenFile(FileQuestion {
            folder: context.repository.folder.clone(),
            file_name: None,
            filters: vec![
                filter("Patches", &["patch", "diff", "mbox", "eml"]),
                filter("All files", &["*"]),
            ],
        }),
        title: Text::from("Apply patch"),
        detail: None,
        default: None,
    }
}

/// **Apply `<name>`**: **As commits** or **To the working tree only**.
fn way(context: &Context, name: &str, commits: usize) -> Question {
    let branch = branch(context);
    let made = if commits == 1 {
        format!("Makes its commit again on top of {branch}, with the authors and messages")
    } else {
        format!("Makes its {commits} commits again on top of {branch}, with the authors and messages")
    };
    Question {
        key: WAY,
        kind: QuestionKind::Pick {
            choices: vec![
                Choice {
                    identifier: String::from("commits"),
                    label: Text::from("As commits"),
                    description: Some(Text::from(format!("git am --3way: {made}"))),
                },
                Choice {
                    identifier: String::from("working-tree"),
                    label: Text::from("To the working tree only"),
                    description: Some(Text::from(
                        "git apply --3way: Applies the changes to the files and stages them, without committing",
                    )),
                },
            ],
        },
        title: Text::from(format!("Apply {name}")),
        detail: None,
        default: Some(model::contract::Answer::Choice(String::from("commits"))),
    }
}

/// The branch checked out, or `HEAD` when none is.
fn branch(context: &Context) -> String {
    context.repository.checked_out_branch().unwrap_or("HEAD").to_owned()
}

/// The refusal of a patch that does not apply, with git's reasons.
fn refused(name: &str, reasons: &[String]) -> Outcome {
    let mut name = name.to_owned();
    if let Some(first) = name.get(..1) {
        name = first.to_uppercase() + name.get(1..).unwrap_or_default();
    }
    let mut message = format!("{name} does not apply, so nothing was changed.");
    for line in named(reasons) {
        message.push('\n');
        message.push_str(&line);
    }
    Outcome::Refuse(Refusal::saying(&message))
}

/// The first fifteen of some lines, then **…and K more**.
fn named(lines: &[String]) -> Vec<String> {
    let mut named: Vec<String> = lines.iter().take(NAMED).cloned().collect();
    if lines.len() > NAMED {
        named.push(format!("\u{2026}and {} more", lines.len() - NAMED));
    }
    named
}

/// Asks first when the patch will stop on conflicts, naming the files.
fn danger(note: &str, conflicts: &[String]) -> Danger {
    if conflicts.is_empty() {
        return Danger::None;
    }
    let files = if conflicts.len() == 1 {
        String::from("1 file")
    } else {
        format!("{} files", conflicts.len())
    };
    let mut said = format!("{note} This will stop with conflicts in {files}:");
    for line in named(conflicts) {
        said.push('\n');
        said.push_str(&line);
    }
    Danger::Confirm(Text::from(said))
}

/// The plan: the git command on the file, after the text of the clipboard is written to it, and its deletion after.
fn plan(
    context: &Context,
    source: &Source<'_>,
    (as_commits, commits): (bool, usize),
    (file, conflicts): (&PathBuf, &[String]),
) -> Plan {
    let folder = context.repository.folder.clone();
    let mut steps = Vec::new();
    let mut after = Vec::new();
    if source.file.is_none() {
        steps.push(Step::Write {
            path: file.clone(),
            contents: source.text.to_vec(),
        });
        after.push(FollowUp::Cleanup(Step::Delete { path: file.clone() }));
    }
    let command = if as_commits { "am" } else { "apply" };
    steps.push(Step::git(
        folder,
        [command.into(), "--3way".into(), file.as_os_str().to_owned()],
    ));
    let name = &source.name;
    let (title, note) = if as_commits {
        let (counted, made) = if commits == 1 {
            (String::from("1 commit"), String::from("commit"))
        } else {
            (format!("{commits} commits"), format!("{commits} commits"))
        };
        (
            format!("Apply {name} as {counted}"),
            format!(
                "The {made} will be made on top of {}, with the authors and messages the patch has.",
                branch(context)
            ),
        )
    } else {
        (
            format!("Apply {name} to the working tree"),
            String::from("The changes will be applied to the files and staged: nothing is committed."),
        )
    };
    let danger = danger(&note, conflicts);
    let undo = match (as_commits, &context.repository.head.commit) {
        (true, Some(head)) => Undo::Journal(UndoRecord::Series {
            head_before: head.clone(),
        }),
        (true, None) => Undo::ByReflog,
        (false, _) => Undo::Impossible(Text::from(
            "Applying a patch to the working tree moves no reference: Undo does not take it back.",
        )),
    };
    Plan {
        title: Text::from(title),
        steps,
        danger,
        undo,
        after,
        on_failure: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
