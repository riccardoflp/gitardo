//! `submodule.add`: clones a repository into a folder of this one as a submodule, with its own submodules, staged with
//! `.gitmodules`.

use std::ffi::OsString;

use model::contract::{
    Answer, AnswerKey, Answers, Context, Danger, FollowUp, Outcome, Plan, Query, Question, QuestionKind, Refusal, Step,
    Text, Undo, Validation,
};
use model::submodule::{folder_of_url, typed_folder};

/// The URL of the repository, or its path on disk.
pub const URL: AnswerKey = AnswerKey("url");

/// The folder to clone it in, from the root of the repository.
pub const FOLDER: AnswerKey = AnswerKey("folder");

/// The branch it follows, for updates from its remote; empty for the default branch of the remote.
pub const BRANCH: AnswerKey = AnswerKey("branch");

/// Whether something is in the folder already.
const HAS_FILES: AnswerKey = AnswerKey("has-files");

/// What is said once it ran.
pub const COMMIT_THEM: &str = "Git clones it and stages it along with .gitmodules: commit them to record it in the \
repository.";

/// Asks for the URL or path of a repository, the folder to clone it in and the branch it follows, then runs
/// `git submodule add [-b <branch>] -- <url> <folder>` and `git submodule update --init --recursive -- <folder>` for
/// its own submodules. A folder with something in it is refused.
///
/// Feature: submodule.add
#[must_use]
pub fn add(context: &Context, answers: &Answers) -> Outcome {
    let Some(url) = answers.text(URL).map(str::trim) else {
        return Outcome::Ask(url_question());
    };
    let Some(folder) = answers.text(FOLDER).map(typed_folder) else {
        return Outcome::Ask(folder_question(context, url));
    };
    let Some(branch) = answers.text(BRANCH).map(str::trim) else {
        return Outcome::Ask(branch_question());
    };
    let path = folder
        .split('/')
        .fold(context.repository.folder.clone(), |path, part| path.join(part));
    match answers.flag(HAS_FILES) {
        None => return Outcome::Read(HAS_FILES, Query::HasFiles { path }),
        Some(true) => {
            return Outcome::Refuse(Refusal::saying(&format!("{} is not empty.", path.display())));
        }
        Some(false) => {}
    }
    let mut adding: Vec<OsString> = vec![OsString::from("submodule"), OsString::from("add")];
    if !branch.is_empty() {
        adding.extend([OsString::from("-b"), OsString::from(branch)]);
    }
    adding.extend(["--", url, folder.as_str()].map(OsString::from));
    let top = context.repository.folder.clone();
    Outcome::Run(Plan {
        title: Text::from(format!("Add submodule {folder}")),
        steps: vec![
            Step::Git {
                arguments: adding,
                directory: top.clone(),
                input: None,
            },
            Step::git(
                top,
                [
                    "submodule",
                    "update",
                    "--init",
                    "--recursive",
                    "--",
                    super::literal(&folder).as_str(),
                ],
            ),
        ],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Not taken back by Undo: Remove takes it away.")),
        after: vec![FollowUp::Say(Text::from(COMMIT_THEM))],
        on_failure: vec![FollowUp::AllowFileProtocol],
    })
}

/// The URL of the repository, or its path on disk.
fn url_question() -> Question {
    Question {
        key: URL,
        kind: QuestionKind::Text {
            placeholder: Some(Text::from("https://github.com/owner/repo.git")),
            validation: Validation::Url,
        },
        title: Text::from("URL of the repository, or its path on disk"),
        detail: None,
        default: None,
    }
}

/// The folder to clone it in, the name of the repository by default.
fn folder_question(context: &Context, url: &str) -> Question {
    let proposed = folder_of_url(url);
    let top = &context.repository.folder;
    Question {
        key: FOLDER,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::SubmoduleFolder {
                submodules: context
                    .repository
                    .submodules
                    .iter()
                    .map(|submodule| submodule.folder.clone())
                    .collect(),
            },
        },
        title: Text::from("Folder to clone it in, from the root of the repository"),
        detail: Some(Text::from(format!("In {}", top.join(&proposed).display()))),
        default: Some(Answer::Text(proposed)),
    }
}

/// The branch it follows, empty for the default branch of the remote.
fn branch_question() -> Question {
    Question {
        key: BRANCH,
        kind: QuestionKind::Text {
            placeholder: None,
            validation: Validation::OptionalReferenceName,
        },
        title: Text::from(
            "Branch it follows, for updates from its remote. Leave empty for the default branch of the remote.",
        ),
        detail: None,
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use model::contract::{Answer, Answers, FollowUp, Outcome, QuestionKind, Target};

    use super::{BRANCH, FOLDER, HAS_FILES, URL, add};
    use crate::testing::{context, lines, plan, question, state};

    const URL_TYPED: &str = "https://example.com/acme/lib.git";

    fn answered(folder: &str, branch: &str) -> Answers {
        Answers::new()
            .with(URL, Answer::Text(String::from(URL_TYPED)))
            .with(FOLDER, Answer::Text(folder.to_owned()))
            .with(BRANCH, Answer::Text(branch.to_owned()))
            .with(HAS_FILES, Answer::Flag(false))
    }

    #[test]
    fn submodule_add__proposes_lib_and_adds_then_updates_it() {
        let context = context(state(), Target::Head);
        let asked = question(add(
            &context,
            &Answers::new().with(URL, Answer::Text(String::from(URL_TYPED))),
        ));
        assert_eq!(asked.key, FOLDER);
        assert_eq!(asked.default, Some(Answer::Text(String::from("lib"))));
        let plan = plan(add(&context, &answered("lib", "")));
        assert_eq!(
            lines(&plan),
            [
                "git submodule add -- https://example.com/acme/lib.git lib",
                "git submodule update --init --recursive -- lib",
            ]
        );
        assert_eq!(plan.on_failure, [FollowUp::AllowFileProtocol]);
    }

    #[test]
    fn submodule_add__with_the_branch_stable_the_first_command_has_b_stable() {
        let plan = plan(add(&context(state(), Target::Head), &answered("lib", "stable")));
        assert_eq!(
            lines(&plan)[0],
            "git submodule add -b stable -- https://example.com/acme/lib.git lib"
        );
    }

    #[test]
    fn submodule_add__folders_outside_the_repository_are_refused_while_typed_and_a_folder_with_files_when_chosen() {
        let context = context(state(), Target::Head);
        let asked = question(add(
            &context,
            &Answers::new().with(URL, Answer::Text(String::from(URL_TYPED))),
        ));
        let QuestionKind::Text { validation, .. } = asked.kind else {
            panic!("the folder is typed");
        };
        for folder in ["../lib", "/tmp/lib", "C:/lib"] {
            assert!(validation.problem(folder).is_some(), "{folder}");
        }
        assert_eq!(validation.problem("lib"), None);
        let full = answered("lib", "").with(HAS_FILES, Answer::Flag(true));
        let Outcome::Refuse(refusal) = add(&context, &full) else {
            panic!("a folder with files is refused");
        };
        assert!(refusal.message.as_str().ends_with("is not empty."));
    }

    #[test]
    fn submodule_add__a_second_command_initializes_its_own_submodules() {
        let plan = plan(add(&context(state(), Target::Head), &answered("vendor/lib", "")));
        assert_eq!(lines(&plan)[1], "git submodule update --init --recursive -- vendor/lib");
    }

    #[test]
    fn submodule_add__a_folder_git_could_read_as_a_pattern_is_updated_by_its_literal_name() {
        let plan = plan(add(&context(state(), Target::Head), &answered("lib[1]", "")));
        assert_eq!(
            lines(&plan)[1],
            "git submodule update --init --recursive -- :(literal)lib[1]"
        );
    }

    #[test]
    fn submodule_add__backing_out_of_any_question_runs_nothing() {
        let context = context(state(), Target::Head);
        let mut answers = Answers::new();
        for key in [URL, FOLDER, BRANCH] {
            let asked = question(add(&context, &answers));
            assert_eq!(asked.key, key, "each question comes before anything runs");
            answers.insert(key, Answer::Text(String::from(if key == URL { URL_TYPED } else { "" })));
        }
    }
}
