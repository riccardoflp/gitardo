//! The whole setup of a new worktree, for the operations that create one (`worktree.create`,
//! `worktree.create-from-branch`, `worktree.create-at-commit`, `worktree.open-in-worktree`): the links and copies of
//! what git ignores, then the commands, as steps to put after `git worktree add`.

use std::path::Path;

use model::contract::{Answer, AnswerKey, Answers, Context, Outcome, Query, Step};
use model::worktree_setup::{
    CommandStepsInput, LinkAndCopyInput, NoteStart, SetupPreview, SetupSteps, TakeRequest, command_steps,
    link_and_copy_steps,
};

use super::files::{patterns_of, read_files};

/// The key of what the patterns take for the new worktree.
pub const NEW_WORKTREE_TAKE: AnswerKey = AnswerKey("worktree-setup-new-worktree-take");

/// A worktree about to be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewWorktree<'a> {
    /// Its folder.
    pub folder: &'a Path,
    /// What it checks out, by any name git understands: the branch it starts from, or a commit.
    pub commit: &'a str,
    /// How the views name it, which names the tasks of its commands.
    pub name: &'a str,
}

/// The steps that set up a new worktree once `git worktree add` made it, from the folder the views show: first a
/// warning with the note of what follows (the patterns and where from, then the commands), then the links and the
/// copies (`worktree-setup.link-and-copy-steps`), then the commands, or the warning that they did not run in a folder
/// that is not trusted (`worktree-setup.command-steps`). `said` is what Gitardo says once the plan ran, for a
/// `FollowUp::Say`.
///
/// It reads the settings files, then what the patterns take for the worktree's commit, through the outcomes it gives
/// back to be returned by the operation that calls it; call it again with the same answers until it gives the steps.
///
/// # Errors
///
/// The outcome that reads what it needs, while it is not read yet.
pub fn new_worktree_steps(
    context: &Context,
    answers: &Answers,
    worktree: &NewWorktree<'_>,
) -> Result<SetupSteps, Box<Outcome>> {
    let settings = read_files(context, answers)?.in_effect();
    let patterns = patterns_of(&settings);
    let empty = SetupPreview::default();
    let take = if patterns.link.is_empty() && patterns.copy.is_empty() {
        &empty
    } else {
        let Some(Answer::SetupTakes(takes)) = answers.get(NEW_WORKTREE_TAKE) else {
            let request = TakeRequest {
                folder: context.repository.folder.clone(),
                patterns,
                commit: Some(worktree.commit.to_owned()),
            };
            return Err(Box::new(Outcome::Read(
                NEW_WORKTREE_TAKE,
                Query::SetupTakes(vec![request]),
            )));
        };
        takes.first().unwrap_or(&empty)
    };
    let links = link_and_copy_steps(&LinkAndCopyInput {
        settings: &settings,
        source: &context.repository.folder,
        worktree: worktree.folder,
        take,
    });
    let commands = command_steps(&CommandStepsInput {
        commands: &settings.setup_commands.values,
        worktree: worktree.folder,
        name: worktree.name,
        trust: context.settings.trust,
        after: if links.note.is_some() {
            NoteStart::AfterLinks
        } else {
            NoteStart::Alone
        },
    });
    let note: Vec<String> = links.note.into_iter().chain(commands.note).collect();
    let mut steps: Vec<Step> = Vec::new();
    if !note.is_empty() {
        steps.push(Step::Warn { words: note.join(" ") });
    }
    steps.extend(links.steps);
    steps.extend(commands.steps);
    Ok(SetupSteps {
        steps,
        note: (!note.is_empty()).then(|| note.join(" ")),
        said: commands.said,
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answer, Answers, Outcome, Query, Step, Target};
    use model::worktree_setup::{SetupPreview, Trust};

    use super::{NEW_WORKTREE_TAKE, NewWorktree, new_worktree_steps};
    use crate::testing::{context, state};
    use crate::worktree_setup::files::SETTINGS_FILES;

    const SETTINGS: &str = r#"{
        // What each new worktree gets.
        "worktrees.linkFolders": ["node_modules/"],
        "worktrees.copyFiles": ["/.env"],
        "worktrees.setupCommands": ["npm ci", "npm run build"]
    }"#;

    fn answers_with(settings: &str, take: SetupPreview) -> Answers {
        Answers::new()
            .with(
                SETTINGS_FILES,
                Answer::Files(vec![Some(settings.as_bytes().to_vec()), None]),
            )
            .with(NEW_WORKTREE_TAKE, Answer::SetupTakes(vec![take]))
    }

    fn lines(steps: &[Step]) -> Vec<String> {
        steps.iter().map(Step::log_line).collect()
    }

    #[test]
    fn worktree_setup_command_steps__the_commands_end_the_setup_after_the_links_and_copies() {
        let mut context = context(state(), Target::Head);
        context.settings.trust = Trust::Trusted;
        let worktree = PathBuf::from("trees").join("feature");
        let new = NewWorktree {
            folder: &worktree,
            commit: "main",
            name: "feature",
        };
        let first = new_worktree_steps(&context, &Answers::new(), &new).map_err(|outcome| *outcome);
        let Err(Outcome::Read(key, Query::FileTexts { paths })) = first else {
            panic!("expected a read of the settings files, got {first:?}");
        };
        assert_eq!((key, paths.len()), (SETTINGS_FILES, 2));
        let answers = Answers::new().with(
            SETTINGS_FILES,
            Answer::Files(vec![Some(SETTINGS.as_bytes().to_vec()), None]),
        );
        let second = new_worktree_steps(&context, &answers, &new).map_err(|outcome| *outcome);
        let Err(Outcome::Read(NEW_WORKTREE_TAKE, Query::SetupTakes(requests))) = second else {
            panic!("expected a read of what the patterns take, got {second:?}");
        };
        assert_eq!(
            requests.first().and_then(|request| request.commit.as_deref()),
            Some("main")
        );
        let take = SetupPreview {
            linked: vec![String::from("node_modules")],
            copied: vec![String::from(".env")],
            ..SetupPreview::default()
        };
        let steps = new_worktree_steps(&context, &answers_with(SETTINGS, take), &new).unwrap();
        assert_eq!(
            lines(&steps.steps),
            [
                "Then Gitardo will link the folders matching node_modules/, and copy what matches /.env, from \
                 repository, where git ignores them. After that it will run npm ci, then npm run build, in Tasks, in \
                 the worktree.",
                "symlink node_modules",
                "copy .env",
                "run npm ci",
                "run npm run build",
            ]
        );
        assert_eq!(
            steps.steps.last(),
            Some(&Step::Task {
                command: String::from("npm run build"),
                directory: worktree.clone(),
                name: String::from("feature"),
            })
        );
    }

    #[test]
    fn worktree_setup_command_steps__a_folder_not_trusted_still_gets_its_links_and_copies() {
        let context = context(state(), Target::Head);
        let worktree = PathBuf::from("tree");
        let new = NewWorktree {
            folder: &worktree,
            commit: "main",
            name: "tree",
        };
        let take = SetupPreview {
            linked: vec![String::from("node_modules")],
            ..SetupPreview::default()
        };
        let steps = new_worktree_steps(&context, &answers_with(SETTINGS, take), &new).unwrap();
        let lines = lines(&steps.steps);
        assert_eq!(
            lines.get(1..),
            Some(
                &[
                    String::from("symlink node_modules"),
                    String::from("The commands of worktrees.setupCommands did not run: this folder is not trusted."),
                ][..]
            )
        );
        assert_eq!(
            steps.said.as_deref(),
            Some("The commands for new worktrees did not run in tree: this folder is not trusted.")
        );
    }
}
