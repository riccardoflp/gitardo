//! `worktree-setup.save`: the patterns and the commands of the page written to the settings of the repository or of
//! the user, and with the box ticked to VS Code's settings of the repository too.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Target, Text, Undo};
use model::worktree_setup::SettingsPlace;

use super::files::{
    COPY_FILES, LINK_FOLDERS, NO_USER_FILE, SETUP_COMMANDS, VS_CODE_COPY_FILES, VS_CODE_LINK_FOLDERS, edited,
    list_value, read_files, write_step,
};

/// What is said when what the page shows is what is saved there already.
const SAVED_ALREADY: &str = "New worktrees get this already, as it is saved there.";

/// Writes `worktrees.linkFolders`, `worktrees.copyFiles` and `worktrees.setupCommands` in the settings file of the
/// place chosen, keeping the rest of it, comments included; an empty list of commands only where one is set somewhere,
/// as it would change nothing else. With the box ticked, VS Code's `git.worktreeSymlinkFolders` and
/// `git.worktreeIncludeFiles` of the repository get the same patterns.
///
/// Feature: worktree-setup.save
#[must_use]
pub fn save(context: &Context, answers: &Answers) -> Outcome {
    let Target::SetupSettings(save) = &context.target else {
        return Outcome::Refuse(Refusal::saying(SAVED_ALREADY));
    };
    let files = match read_files(context, answers) {
        Ok(files) => files,
        Err(outcome) => return *outcome,
    };
    let Some(file) = files.of(save.place) else {
        return Outcome::Refuse(Refusal::saying(NO_USER_FILE));
    };
    let commands_set_somewhere =
        files.repository.has(SETUP_COMMANDS) || files.user.as_ref().is_some_and(|user| user.has(SETUP_COMMANDS));
    let mut changes = vec![
        (LINK_FOLDERS, Some(list_value(&save.patterns.link))),
        (COPY_FILES, Some(list_value(&save.patterns.copy))),
    ];
    if !save.commands.is_empty() || commands_set_somewhere {
        changes.push((SETUP_COMMANDS, Some(list_value(&save.commands))));
    }
    let text = match edited(file, &changes) {
        Ok(text) => text,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let mut steps: Vec<_> = write_step(file, text).into_iter().collect();
    if save.also_vs_code && save.place == SettingsPlace::Repository {
        let vs_code = [
            (VS_CODE_LINK_FOLDERS, Some(list_value(&save.patterns.link))),
            (VS_CODE_COPY_FILES, Some(list_value(&save.patterns.copy))),
        ];
        match edited(&files.vs_code, &vs_code) {
            Ok(text) => steps.extend(write_step(&files.vs_code, text)),
            Err(refusal) => return Outcome::Refuse(refusal),
        }
    }
    if steps.is_empty() {
        return Outcome::Refuse(Refusal::saying(SAVED_ALREADY));
    }
    Outcome::Run(Plan {
        title: Text::from("Save what new worktrees get"),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(
            "Saving is not taken back by Undo: change the patterns again, or remove them from where they are saved.",
        )),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[path = "save_tests.rs"]
mod tests;
