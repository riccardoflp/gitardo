//! `worktree-setup.remove-from`: the three settings of new worktrees taken out of a place, so that what is set
//! elsewhere applies again.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Refusal, Step, Target, Text, Undo};

use super::files::{
    COPY_FILES, LINK_FOLDERS, NO_USER_FILE, SETUP_COMMANDS, edited, host_target, host_writes, read_files,
    setting_written, write_step,
};

/// What is said when the place sets none of the three.
const NOT_SET: &str = "None of the settings of new worktrees is set there.";

/// Writes the settings file of the place chosen on the page without `worktrees.linkFolders`, `worktrees.copyFiles`
/// and `worktrees.setupCommands`, keeping the rest of it; VS Code's settings are not changed. Inside VS Code, the
/// plan has no step: its follow-ups have VS Code remove the three from the place of its settings the place chosen
/// stands for, which the page offers only where one of them is set.
///
/// Feature: worktree-setup.remove-from
#[must_use]
pub fn remove_from(context: &Context, answers: &Answers) -> Outcome {
    let Target::SetupSettings(save) = &context.target else {
        return Outcome::Refuse(Refusal::saying(NOT_SET));
    };
    if host_writes(context) {
        let target = host_target(context, save.place);
        let removed = [LINK_FOLDERS, COPY_FILES, SETUP_COMMANDS].map(|key| setting_written(key, None, target));
        return Outcome::Run(removing_plan(Vec::new(), removed.into()));
    }
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
    Outcome::Run(removing_plan(vec![step], Vec::new()))
}

/// The plan that removes, with its step or the settings the host removes.
fn removing_plan(steps: Vec<Step>, after: Vec<FollowUp>) -> Plan {
    Plan {
        title: Text::from("Remove what new worktrees get from the settings"),
        steps,
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("Removing is not taken back by Undo: save them again.")),
        after,
        on_failure: Vec::new(),
    }
}
