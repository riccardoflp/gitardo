//! `repo.clone`: copies a repository from its URL, or from its folder on disk, into a new folder, and opens the copy in
//! a tab.

use std::ffi::OsString;
use std::path::Path;

use model::contract::{
    Answer, AnswerKey, Answers, Choice, Context, Danger, FileQuestion, FollowUp, Outcome, Plan, Query, Question,
    QuestionKind, Refusal, Step, Text, Undo, Validation, count_of,
};
use model::repo::{CloneFolderRule, clone_folder, folder_from_url};

/// The folder the dialog of the folder that holds the clone starts in: the one the last clone went in, else the user's
/// home folder. The window gives it.
pub const START_FOLDER: AnswerKey = AnswerKey("start-folder");
/// The text of the clipboard, which the URL box starts with when it looks like the URL of a repository.
const CLIPBOARD: AnswerKey = AnswerKey("clipboard");
/// The URL of the repository, or its path on disk.
pub const URL: AnswerKey = AnswerKey("url");
/// The folder that holds the clone.
pub const PARENT: AnswerKey = AnswerKey("parent");
/// The names of what is in the folder that holds the clone and is not an empty folder.
const TAKEN: AnswerKey = AnswerKey("taken");
/// The name of the folder of the clone.
pub const NAME: AnswerKey = AnswerKey("name");
/// What is ticked of the options that are not every day.
pub const OPTIONS: AnswerKey = AnswerKey("options");
/// The branch to check out, when another than the remote's default one is ticked.
pub const BRANCH: AnswerKey = AnswerKey("branch");
/// How many commits a shallow clone fetches.
pub const DEPTH: AnswerKey = AnswerKey("depth");
/// Whether the folder of the clone has something in it by the time it runs.
const HAS_FILES: AnswerKey = AnswerKey("has-files");

/// The option that checks out another branch than the remote's default one.
pub const OTHER_BRANCH: &str = "branch";
/// The option of a shallow clone.
pub const SHALLOW: &str = "shallow";
/// The option that clones the submodules too, and theirs.
pub const SUBMODULES: &str = "submodules";

/// Asks the URL, the folder that holds the clone, the name of its folder and what is not every day, then runs
/// `git clone --progress [--branch <branch>] [--depth <n>] [--recurse-submodules] -- <url> <name>` in the folder that
/// holds it; once it ran, the folder is remembered and the clone opened in a tab.
///
/// Feature: repo.clone
#[must_use]
pub fn clone(context: &Context, answers: &Answers) -> Outcome {
    let Some(clipboard) = answers.get(CLIPBOARD) else {
        return Outcome::Read(CLIPBOARD, Query::Clipboard);
    };
    let Some(url) = answers.text(URL).map(str::trim) else {
        return Outcome::Ask(url_question(clipboard));
    };
    let Some(Answer::Path(parent)) = answers.get(PARENT) else {
        return Outcome::Ask(parent_question(context, answers));
    };
    let Some(taken) = answers.names(TAKEN) else {
        return Outcome::Read(TAKEN, Query::TakenNames { folder: parent.clone() });
    };
    let Some(name) = answers.text(NAME).map(str::trim) else {
        return Outcome::Ask(name_question(url, parent, taken));
    };
    let Some(options) = answers.choices(OPTIONS) else {
        return Outcome::Ask(options_question(url, name));
    };
    let is_ticked = |option: &str| options.iter().any(|ticked| ticked == option);
    let branch = match answers.text(BRANCH) {
        None if is_ticked(OTHER_BRANCH) => return Outcome::Ask(branch_question()),
        branch => branch.map(str::trim),
    };
    let shallow = is_ticked(SHALLOW) && !is_on_disk(url);
    let depth = match answers.text(DEPTH) {
        None if shallow => return Outcome::Ask(depth_question()),
        depth => depth.and_then(count_of).filter(|_| shallow),
    };
    let folder = clone_folder(parent, name);
    match answers.flag(HAS_FILES) {
        None => return Outcome::Read(HAS_FILES, Query::HasFiles { path: folder }),
        Some(true) => return Outcome::Refuse(Refusal::saying(&format!("{} is not empty.", folder.display()))),
        Some(false) => {}
    }
    let mut arguments: Vec<OsString> = ["clone", "--progress"].map(OsString::from).to_vec();
    if let Some(branch) = branch {
        arguments.extend([OsString::from("--branch"), OsString::from(branch)]);
    }
    if let Some(depth) = depth {
        arguments.extend([OsString::from("--depth"), OsString::from(depth.to_string())]);
    }
    if is_ticked(SUBMODULES) {
        arguments.push(OsString::from("--recurse-submodules"));
    }
    arguments.extend(["--", url, name].map(OsString::from));
    plan(url, (parent, &folder), arguments)
}

/// The plan of the clone, run in the folder that holds it.
fn plan(url: &str, (parent, folder): (&Path, &Path), arguments: Vec<OsString>) -> Outcome {
    let name = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Outcome::Run(Plan {
        title: Text::from(format!("Clone {url} into {name}")),
        steps: vec![Step::Git {
            arguments,
            directory: parent.to_path_buf(),
            input: None,
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Not taken back by Undo: the folder of the clone is yours to delete.",
        )),
        after: vec![
            FollowUp::RememberCloneFolder(parent.to_path_buf()),
            FollowUp::OpenRepository(folder.to_path_buf()),
            FollowUp::Say(Text::from(format!("Cloned into {}", folder.display()))),
        ],
        on_failure: Vec::new(),
    })
}

/// Whether what was typed is a path on disk rather than a URL: it has no `://`, and no `host:` before its first slash
/// as the URLs of SSH have, a letter of a drive of Windows not counting as a host. Git ignores the depth of a clone
/// of a path on disk.
#[must_use]
pub fn is_on_disk(url: &str) -> bool {
    if url.contains("://") {
        return false;
    }
    let Some((host, _)) = url.split_once(':') else {
        return true;
    };
    host.len() <= 1 || host.contains(['/', '\\'])
}

/// Whether the text of the clipboard looks like the URL of a repository: one line that starts with `https://`,
/// `ssh://`, `git@` or `file://`.
fn looks_like_url(text: &str) -> bool {
    let text = text.trim();
    !text.contains('\n')
        && ["https://", "ssh://", "git@", "file://"]
            .iter()
            .any(|start| text.starts_with(start))
}

/// The URL of the repository, or its path on disk, starting with the text of the clipboard when it looks like one.
fn url_question(clipboard: &Answer) -> Question {
    let default = match clipboard {
        Answer::Clipboard(Some(text)) if looks_like_url(text) => Some(Answer::Text(text.trim().to_owned())),
        _ => None,
    };
    Question {
        key: URL,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("https://github.com/owner/repo.git")),
            validation: Validation::Url,
        },
        title: Text::from("URL of the repository, or its path on disk"),
        detail: None,
        default,
    }
}

/// The folder that holds the clone, in the system's dialog, starting where the last clone went.
fn parent_question(context: &Context, answers: &Answers) -> Question {
    let home = &context.settings.worktrees.home;
    let start = match answers.get(START_FOLDER) {
        Some(Answer::Path(folder)) => folder.clone(),
        _ if !home.as_os_str().is_empty() => home.clone(),
        _ => context.repository.folder.clone(),
    };
    Question {
        key: PARENT,
        kind: QuestionKind::OpenFolder(FileQuestion {
            folder: start,
            file_name: None,
            filters: Vec::new(),
        }),
        title: Text::from("Folder to clone into"),
        detail: None,
        default: None,
    }
}

/// The name of the folder of the clone, the name of the repository by default.
fn name_question(url: &str, parent: &Path, taken: &[String]) -> Question {
    let rule = CloneFolderRule {
        parent: parent.to_path_buf(),
        taken: taken.to_vec(),
    };
    let proposed = folder_from_url(url);
    Question {
        key: NAME,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::CloneFolder(Box::new(rule)),
        },
        title: Text::from("Name of the folder of the clone"),
        detail: None,
        default: proposed.map(Answer::Text),
    }
}

/// What is not every day, ticked as most clones want it: the submodules.
fn options_question(url: &str, name: &str) -> Question {
    let mut choices = vec![Choice {
        identifier: String::from(OTHER_BRANCH),
        label: Text::from("Check out another branch than the remote's default one"),
        description: None,
    }];
    if !is_on_disk(url) {
        choices.push(Choice {
            identifier: String::from(SHALLOW),
            label: Text::from("Only the last commits, a shallow clone"),
            description: None,
        });
    }
    choices.push(Choice {
        identifier: String::from(SUBMODULES),
        label: Text::from("The submodules too, and theirs"),
        description: None,
    });
    Question {
        key: OPTIONS,
        kind: QuestionKind::PickMany { choices },
        title: Text::from(format!("How to clone {name}")),
        detail: None,
        default: Some(Answer::Choices(vec![String::from(SUBMODULES)])),
    }
}

/// The branch to check out.
fn branch_question() -> Question {
    Question {
        key: BRANCH,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::ReferenceName {
                taken: Vec::new(),
                taken_message: Text::from(""),
            },
        },
        title: Text::from("Branch to check out"),
        detail: None,
        default: None,
    }
}

/// How many commits a shallow clone fetches.
fn depth_question() -> Question {
    Question {
        key: DEPTH,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::Count,
        },
        title: Text::from("How many commits to fetch"),
        detail: Some(Text::from(
            "Only this branch is fetched, as git does for a shallow clone.",
        )),
        default: Some(Answer::Text(String::from("1"))),
    }
}

#[cfg(test)]
#[path = "clone_tests.rs"]
mod tests;
