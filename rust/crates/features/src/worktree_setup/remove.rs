//! `worktree-setup.remove-from`: the three settings of new worktrees taken out of a place, so that what is set
//! elsewhere applies again.

use model::contract::{Answers, Context, Danger, Outcome, Plan, Refusal, Target, Text, Undo};

use super::files::{COPY_FILES, LINK_FOLDERS, NO_USER_FILE, SETUP_COMMANDS, edited, read_files, write_step};

/// What is said when the place sets none of the three.
const NOT_SET: &str = "None of the settings of new worktrees is set there.";

/// Writes the settings file of the place chosen on the page without `worktrees.linkFolders`, `worktrees.copyFiles`
/// and `worktrees.setupCommands`, keeping the rest of it; VS Code's settings are not changed.
///
/// Feature: worktree-setup.remove-from
#[must_use]
pub fn remove_from(context: &Context, answers: &Answers) -> Outcome {
    let Target::SetupSettings(save) = &context.target else {
        return Outcome::Refuse(Refusal::saying(NOT_SET));
    };
    let files = match read_files(context, answers) {
        Ok(files) => files,
        Err(outcome) => return *outcome,
    };
    let Some(file) = files.of(save.place) else {
        return Outcome::Refuse(Refusal::saying(NO_USER_FILE));
    };
    let text = match edited(
        file,
        &[(LINK_FOLDERS, None), (COPY_FILES, None), (SETUP_COMMANDS, None)],
    ) {
        Ok(text) => text,
        Err(refusal) => return Outcome::Refuse(refusal),
    };
    let Some(step) = write_step(file, text) else {
        return Outcome::Refuse(Refusal::saying(NOT_SET));
    };
    Outcome::Run(Plan {
        title: Text::from("Remove what new worktrees get from the settings"),
        steps: vec![step],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Removing is not taken back by Undo: save them again.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}
