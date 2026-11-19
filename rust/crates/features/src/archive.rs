//! `archive.save`: the files of a commit, or of one folder of it, saved in a zip or a tar.gz file the user chooses.

use model::branch::repository_name;
use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FileFilter, FileQuestion, FollowUp, Outcome, Plan, Query,
    Question, QuestionKind, Refusal, Step, Target, Text, Undo, Validation,
};

/// What the archive holds: [`EVERY_FILE`] or [`ONE_FOLDER`].
pub const HOLDS: AnswerKey = AnswerKey("holds");
/// Every file of the commit.
pub const EVERY_FILE: &str = "every";
/// One folder of the commit, asked next.
pub const ONE_FOLDER: &str = "folder";
/// The folders of the commit, read for the search of [`FOLDER`].
pub const FOLDERS: AnswerKey = AnswerKey("folders");
/// The folder of the commit the archive holds.
pub const FOLDER: AnswerKey = AnswerKey("folder");
/// The format: [`ZIP`] or [`TAR_GZ`].
pub const FORMAT: AnswerKey = AnswerKey("format");
/// A zip.
pub const ZIP: &str = "zip";
/// A tar.gz.
pub const TAR_GZ: &str = "tar.gz";
/// The folder inside the archive, empty for none.
pub const INSIDE: AnswerKey = AnswerKey("inside");
/// Where the archive is saved.
pub const PATH: AnswerKey = AnswerKey("path");

/// What is archived: the revision git is given, and the name the questions and the file use.
struct Archived {
    revision: String,
    name: String,
}

/// Saves the files of the commit, branch, remote branch or tag invoked on as an archive: what it holds, its format,
/// the folder inside it and where to save it are asked in turn, then `git archive` writes it.
///
/// Feature: archive.save
#[must_use]
pub fn save(context: &Context, answers: &Answers) -> Outcome {
    let Some(archived) = archived(&context.target) else {
        return Outcome::Refuse(Refusal::saying(
            "Only a commit, a branch or a tag is saved as an archive.",
        ));
    };
    let folder = match answers.choice(HOLDS) {
        None => return Outcome::Ask(holds_question()),
        Some(ONE_FOLDER) => match (answers.names(FOLDERS), answers.choice(FOLDER)) {
            (None, _) => {
                let query = Query::CommitFolders {
                    revision: archived.revision.clone(),
                };
                return Outcome::Read(FOLDERS, query);
            }
            (Some(folders), None) => return Outcome::Ask(folder_question(folders)),
            (Some(_), Some(folder)) => Some(folder.to_owned()),
        },
        Some(_) => None,
    };
    let repository = repository_name(&context.repository);
    let full_name = format!("{repository}-{}", archived.name);
    let Some(format) = answers.choice(FORMAT) else {
        return Outcome::Ask(format_question());
    };
    let Some(inside) = answers.text(INSIDE) else {
        return Outcome::Ask(inside_question(&full_name));
    };
    let Some(path) = answers.path(PATH) else {
        return Outcome::Ask(path_question(context, (&archived.name, &full_name), format));
    };
    let mut arguments = vec![
        String::from("archive"),
        format!("--format={}", if format == TAR_GZ { TAR_GZ } else { ZIP }),
    ];
    let inside = inside.trim().trim_end_matches('/');
    if !inside.is_empty() {
        arguments.push(format!("--prefix={inside}/"));
    }
    arguments.extend([
        String::from("-o"),
        path.to_string_lossy().into_owned(),
        archived.revision,
    ]);
    if let Some(folder) = folder {
        arguments.extend([String::from("--"), folder]);
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Save {} as an archive", archived.name)),
        steps: vec![Step::git(context.repository.folder.clone(), arguments)],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "The file saved is outside what Undo takes back: delete it to take it back.",
        )),
        after: vec![FollowUp::Say(Text::from(format!("Saved {}", path.display())))],
        on_failure: Vec::new(),
    })
}

/// The revision git archives and the name of what is archived; none for what is not a commit, a branch or a tag. A
/// branch and a tag go by their full names, as they can share one.
fn archived(target: &Target) -> Option<Archived> {
    let dashed = |name: &str| name.replace('/', "-");
    match target {
        Target::Commit(commit) => Some(Archived {
            revision: commit.as_str().to_owned(),
            name: commit.short().to_owned(),
        }),
        Target::Branch(name) => Some(Archived {
            revision: format!("refs/heads/{name}"),
            name: dashed(name),
        }),
        Target::RemoteBranch(name) => Some(Archived {
            revision: format!("refs/remotes/{name}"),
            name: dashed(name),
        }),
        Target::Tag(name) => Some(Archived {
            revision: format!("refs/tags/{name}"),
            name: dashed(name),
        }),
        _ => None, // Nothing else is a commit to archive.
    }
}

/// A choice of a question.
fn choice(identifier: &str, label: &str) -> Choice {
    Choice {
        identifier: identifier.to_owned(),
        label: Text::from(label),
        description: None,
    }
}

/// **What the archive holds**: every file, or one folder.
fn holds_question() -> Question {
    Question {
        key: HOLDS,
        kind: QuestionKind::Pick {
            choices: vec![
                choice(EVERY_FILE, "Every file"),
                choice(ONE_FOLDER, "One folder\u{2026}"),
            ],
        },
        title: Text::from("What the archive holds"),
        detail: None,
        default: Some(Answer::Choice(EVERY_FILE.to_owned())),
    }
}

/// The folder, in a search among the folders of the commit.
fn folder_question(folders: &[String]) -> Question {
    Question {
        key: FOLDER,
        kind: QuestionKind::Search {
            choices: folders.iter().map(|folder| choice(folder, folder)).collect(),
        },
        title: Text::from("Folder the archive holds"),
        detail: None,
        default: None,
    }
}

/// **Format**: zip by default on Windows, tar.gz elsewhere (Q1217).
fn format_question() -> Question {
    let default = if cfg!(windows) { ZIP } else { TAR_GZ };
    Question {
        key: FORMAT,
        kind: QuestionKind::Pick {
            choices: vec![choice(ZIP, "Zip"), choice(TAR_GZ, "tar.gz")],
        },
        title: Text::from("Format"),
        detail: None,
        default: Some(Answer::Choice(default.to_owned())),
    }
}

/// **Folder inside the archive**, `<repository>-<name>/` by default; empty puts the files at its top.
fn inside_question(full_name: &str) -> Question {
    Question {
        key: INSIDE,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from(format!("{full_name}/"))),
            validation: Validation::Anything,
        },
        title: Text::from("Folder inside the archive"),
        detail: Some(Text::from("Empty puts the files at the top of the archive.")),
        default: Some(Answer::Text(format!("{full_name}/"))),
    }
}

/// Where to save it, in the save dialog, starting in the folder that holds the repository.
fn path_question(context: &Context, (name, full_name): (&str, &str), format: &str) -> Question {
    let extension = if format == TAR_GZ { TAR_GZ } else { ZIP };
    let folder = context
        .repository
        .folder
        .parent()
        .unwrap_or(&context.repository.folder)
        .to_path_buf();
    Question {
        key: PATH,
        kind: QuestionKind::SaveFile(FileQuestion {
            folder,
            file_name: Some(format!("{full_name}.{extension}")),
            filters: vec![FileFilter {
                name: Text::from(if format == TAR_GZ {
                    "tar.gz archives"
                } else {
                    "Zip archives"
                }),
                extensions: vec![if format == TAR_GZ {
                    String::from("gz")
                } else {
                    String::from(ZIP)
                }],
            }],
        }),
        title: Text::from(format!("Save {name} as an archive")),
        detail: None,
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
