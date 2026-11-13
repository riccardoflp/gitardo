//! `patch.save`: the patch of `patch.copy` saved to a file the user chooses.

use model::contract::{
    AnswerKey, Answers, Context, Danger, FileFilter, FileQuestion, Outcome, Plan, Question, QuestionKind, Step, Text,
    Undo,
};

use super::{Made, made};

/// The key of the path the patch is saved to.
pub const PATH: AnswerKey = AnswerKey("path");

/// Saves the patch `git format-patch` makes of the commits chosen to a file chosen in the save dialog, which starts in
/// the top folder of the repository with the name `<short hash>.patch`, or `<oldest>-<newest>.patch` for several.
///
/// Feature: patch.save
#[must_use]
pub fn save(context: &Context, answers: &Answers) -> Outcome {
    let made = match made(context, answers) {
        Ok(made) => made,
        Err(outcome) => return *outcome,
    };
    let title = format!("Save {} as a patch", made.named());
    let Some(path) = answers.path(PATH) else {
        return Outcome::Ask(question(context, &made, &title));
    };
    Outcome::Run(Plan {
        title: Text::from(title),
        steps: vec![Step::Write {
            path: path.to_path_buf(),
            contents: made.patch.as_bytes().to_vec(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "The file saved is outside what Undo takes back: delete it to take it back.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// Where to save the patch, in the save dialog.
fn question(context: &Context, made: &Made<'_>, title: &str) -> Question {
    let name = match (made.commits.first(), made.commits.last()) {
        (Some(oldest), Some(newest)) if oldest != newest => format!("{}-{}.patch", oldest.short(), newest.short()),
        (Some(commit), _) => format!("{}.patch", commit.short()),
        (None, _) => String::from("commits.patch"),
    };
    Question {
        key: PATH,
        kind: QuestionKind::SaveFile(FileQuestion {
            folder: context.repository.folder.clone(),
            file_name: Some(name),
            filters: vec![FileFilter {
                name: Text::from("Patches"),
                extensions: vec![String::from("patch")],
            }],
        }),
        title: Text::from(title),
        detail: None,
        default: None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answer, Answers, QuestionKind, Step, Target};

    use super::{PATH, save};
    use crate::patch::PATCH;
    use crate::patch::testing::format_patch;
    use crate::testing::{commit, context, lines, plan, question, state};

    #[test]
    fn patch_save__proposes_the_short_hash_in_the_top_folder_and_writes_the_patch_there() {
        let context = context(state(), Target::Commit(commit('3')));
        let patch = format_patch(&[&commit('3')]);
        let answers = Answers::new().with(PATCH, Answer::Patch(patch.clone()));
        let asked = question(save(&context, &answers));
        let short = commit('3').short().to_owned();
        assert_eq!(asked.title.as_str(), format!("Save {short} as a patch"));
        let QuestionKind::SaveFile(file) = asked.kind else {
            panic!("asks where to save it")
        };
        assert_eq!(file.folder, context.repository.folder);
        assert_eq!(file.file_name, Some(format!("{short}.patch")));
        let chosen = context.repository.folder.join(format!("{short}.patch"));
        let plan = plan(save(&context, &answers.with(PATH, Answer::Path(chosen.clone()))));
        assert_eq!(
            plan.steps,
            [Step::Write {
                path: chosen,
                contents: patch.into_bytes()
            }]
        );
    }

    #[test]
    fn patch_save__two_commits_propose_the_oldest_then_the_newest() {
        let context = context(state(), Target::Commits(vec![commit('7'), commit('2')]));
        let patch = format_patch(&[&commit('2'), &commit('7')]);
        let asked = question(save(&context, &Answers::new().with(PATCH, Answer::Patch(patch))));
        let QuestionKind::SaveFile(file) = asked.kind else {
            panic!("asks where to save it")
        };
        let name = format!("{}-{}.patch", commit('2').short(), commit('7').short());
        assert_eq!(file.file_name, Some(name));
        assert_eq!(asked.title.as_str(), "Save 2 commits as a patch");
    }

    #[test]
    fn patch_save__the_log_has_a_write_line_for_the_file() {
        let context = context(state(), Target::Commit(commit('3')));
        let answers = Answers::new()
            .with(PATCH, Answer::Patch(format_patch(&[&commit('3')])))
            .with(PATH, Answer::Path(PathBuf::from("out.patch")));
        assert_eq!(lines(&plan(save(&context, &answers))), ["write out.patch"]);
    }
}
