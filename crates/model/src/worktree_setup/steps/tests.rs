#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::{Path, PathBuf};

use super::{
    CommandStepsInput, LinkAndCopyInput, NoteStart, SetupSteps, Trust, command_steps, failed_setup_steps,
    link_and_copy_steps,
};
use crate::contract::Step;
use crate::worktree_setup::{InEffect, SettingSource, SetupPreview, SetupSettings};

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

fn settings(link: &[&str], copy: &[&str], commands: &[&str]) -> SetupSettings {
    let in_effect = |values: &[&str]| InEffect {
        values: strings(values),
        source: SettingSource::Repository,
    };
    SetupSettings {
        link_folders: in_effect(link),
        copy_files: in_effect(copy),
        setup_commands: in_effect(commands),
    }
}

fn steps_for(settings: &SetupSettings, take: &SetupPreview, worktree: &Path) -> SetupSteps {
    link_and_copy_steps(&LinkAndCopyInput {
        settings,
        source: Path::new("repository"),
        worktree,
        take,
    })
}

fn lines(steps: &SetupSteps) -> Vec<String> {
    steps.steps.iter().map(Step::log_line).collect()
}

#[test]
fn worktree_setup_link_and_copy_steps__links_then_copies_with_a_note_of_the_patterns_and_the_folder() {
    let take = SetupPreview {
        linked: strings(&["node_modules"]),
        copied: strings(&[".env"]),
        ..SetupPreview::default()
    };
    let worktree = PathBuf::from("trees").join("feature");
    let steps = steps_for(&settings(&["node_modules/"], &["/.env"], &[]), &take, &worktree);
    assert_eq!(lines(&steps), ["symlink node_modules", "copy .env"]);
    assert_eq!(
        steps.steps.first(),
        Some(&Step::Link {
            from: worktree.join("node_modules"),
            to: Path::new("repository").join("node_modules"),
        })
    );
    assert_eq!(
        steps.note.as_deref(),
        Some(
            "Then Gitardo will link the folders matching node_modules/, and copy what matches /.env, from repository, \
             where git ignores them."
        )
    );
    assert_eq!(
        steps_for(&settings(&[], &[], &["npm ci"]), &take, &worktree),
        SetupSteps::default()
    );
}

#[test]
fn worktree_setup_link_and_copy_steps__what_the_commit_has_is_not_copied_and_the_log_says_why() {
    let take = SetupPreview {
        already: strings(&[".env"]),
        ..SetupPreview::default()
    };
    let steps = steps_for(&settings(&[], &["/.env"], &[]), &take, Path::new("tree"));
    assert_eq!(lines(&steps), [".env is in the worktree already."]);
}

#[test]
fn worktree_setup_link_and_copy_steps__a_folder_that_holds_the_worktree_is_not_linked() {
    let take = SetupPreview {
        linked: strings(&[".worktrees", "node_modules"]),
        ..SetupPreview::default()
    };
    let worktree = Path::new("repository").join(".worktrees").join("feature");
    let steps = steps_for(&settings(&[".worktrees/", "node_modules/"], &[], &[]), &take, &worktree);
    assert_eq!(
        lines(&steps),
        [
            ".worktrees holds the worktree, so it is not linked.",
            "symlink node_modules"
        ]
    );
}

#[test]
fn worktree_setup_link_and_copy_steps__one_failure_or_more_is_said_once() {
    assert_eq!(failed_setup_steps(&[]), None);
    assert_eq!(
        failed_setup_steps(&strings(&["copy .env"])).as_deref(),
        Some("Gitardo: `copy .env` failed: the log says why.")
    );
    assert_eq!(
        failed_setup_steps(&strings(&["copy .env", "symlink node_modules"])).as_deref(),
        Some("2 folders or files failed: the log says why.")
    );
}

fn commands(commands: &[&str], trust: Trust, after: NoteStart) -> SetupSteps {
    let commands = strings(commands);
    command_steps(&CommandStepsInput {
        commands: &commands,
        worktree: Path::new("tree"),
        trust,
        after,
    })
}

#[test]
fn worktree_setup_command_steps__each_command_is_a_run_step_in_the_worktree_after_the_note() {
    let steps = commands(&["npm ci", "npm run build"], Trust::Trusted, NoteStart::AfterLinks);
    assert_eq!(lines(&steps), ["run npm ci", "run npm run build"]);
    assert_eq!(
        steps.note.as_deref(),
        Some("After that it will run npm ci, then npm run build, in Tasks, in the worktree.")
    );
    let alone = commands(&["npm ci"], Trust::Trusted, NoteStart::Alone);
    assert_eq!(
        alone.note.as_deref(),
        Some("Then Gitardo will run npm ci, in Tasks, in the worktree.")
    );
}

#[test]
fn worktree_setup_command_steps__in_a_folder_not_trusted_none_runs() {
    let steps = commands(&["npm ci"], Trust::NotTrusted, NoteStart::Alone);
    assert_eq!(
        lines(&steps),
        ["The commands of worktrees.setupCommands did not run: this folder is not trusted."]
    );
    assert_eq!(
        steps.said.as_deref(),
        Some("The commands for new worktrees did not run in tree: this folder is not trusted.")
    );
}
