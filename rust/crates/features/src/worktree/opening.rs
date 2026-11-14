//! The plans that open a worktree: in a new window, in the external editor, in a terminal, in the file manager, or
//! where `worktrees.openIn` says.

use std::path::Path;

use model::contract::{Context, Danger, FollowUp, Plan, Step, Text, Undo, WorktreeOpenIn};

use super::{NOTHING_CHANGES, no_steps, system_path};
use crate::file_view::{quoted, system_opener};

/// Said when `worktrees.openIn` is `editor` but `editor.external` is not set.
pub(super) const NO_EDITOR: &str =
    "No external editor is set in editor.external, so the worktree opens in a new window.";

/// A new window of Gitardo Desktop on the folder.
pub(super) fn in_new_window(folder: &Path, label: &str) -> Plan {
    no_steps(
        format!("Open the worktree {label} in a new window"),
        vec![FollowUp::OpenWindow(folder.to_path_buf())],
    )
}

/// The command of `editor.external` started on the folder: with `${folder}` in its place when the command has it,
/// after the program of the command otherwise, since `${file}` and `${line}` name a file; none when it is not set.
pub(super) fn editor_command(context: &Context, folder: &Path) -> Option<String> {
    let template = context.settings.external_editor.trim();
    if template.is_empty() {
        return None;
    }
    let path = quoted(&system_path(folder));
    if template.contains("${folder}") {
        return Some(template.replace("${folder}", &path));
    }
    let program = program_of(template);
    Some(format!("{program} {path}"))
}

/// The program of a command: its first word, or the words in quotes it starts with.
fn program_of(command: &str) -> &str {
    if let Some(rest) = command.strip_prefix('"') {
        let end = rest.find('"').map_or(command.len(), |end| end + 2);
        return command.get(..end).unwrap_or(command);
    }
    command.split_whitespace().next().unwrap_or(command)
}

/// The external editor started on the folder, in the folder: `run <command>`.
pub(super) fn in_editor(command: String, folder: &Path, label: &str) -> Plan {
    run_plan(
        format!("Open the worktree {label} in the external editor"),
        command,
        folder,
    )
}

/// The command that opens a terminal in the folder: `terminal.external` with `${folder}` filled, or the terminal of
/// the system, started in the folder.
pub(super) fn terminal_command(context: &Context, folder: &Path) -> String {
    let template = context.settings.worktrees.terminal.trim();
    if !template.is_empty() {
        return template.replace("${folder}", &quoted(&system_path(folder)));
    }
    if cfg!(windows) {
        String::from("start \"\" cmd")
    } else if cfg!(target_os = "macos") {
        String::from("open -a Terminal .")
    } else {
        String::from("x-terminal-emulator")
    }
}

/// A terminal opened in the folder: `run <command>`, run in the folder.
pub(super) fn in_terminal(context: &Context, folder: &Path, label: &str) -> Plan {
    run_plan(
        format!("Open a terminal in the worktree {label}"),
        terminal_command(context, folder),
        folder,
    )
}

/// The folder shown in the file manager of the system: `run <command>`.
pub(super) fn in_file_manager(folder: &Path, label: &str) -> Plan {
    run_plan(
        format!("Reveal the worktree {label}"),
        system_opener(&quoted(&system_path(folder))),
        folder,
    )
}

/// The worktree opened where `worktrees.openIn` says; with `editor` and no `editor.external`, or an editor that does
/// not start, in a new window, saying why.
pub(super) fn where_set(context: &Context, folder: &Path, label: &str) -> Plan {
    match context.settings.worktrees.open_in {
        WorktreeOpenIn::Window => in_new_window(folder, label),
        WorktreeOpenIn::Terminal => in_terminal(context, folder, label),
        WorktreeOpenIn::Editor => {
            if let Some(command) = editor_command(context, folder) {
                return Plan {
                    on_failure: vec![FollowUp::OpenWindow(folder.to_path_buf())],
                    ..in_editor(command, folder, label)
                };
            }
            let mut plan = in_new_window(folder, label);
            plan.after.push(FollowUp::Say(Text::from(NO_EDITOR)));
            plan
        }
    }
}

/// A plan that runs one command of the user's in the folder, and changes nothing in the repository.
fn run_plan(title: String, command: String, folder: &Path) -> Plan {
    Plan {
        title: Text::from(title),
        steps: vec![Step::Run {
            command,
            directory: folder.to_path_buf(),
        }],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOTHING_CHANGES)),
        after: Vec::new(),
        on_failure: Vec::new(),
    }
}
