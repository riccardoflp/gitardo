//! `submodule.open-in-new-window`, `submodule.open-in-terminal` and `submodule.reveal`: a submodule opened as a
//! repository of its own in another window, in a terminal, or in the file manager.

use model::contract::{Answers, Context, Danger, FollowUp, Outcome, Plan, Step, Text};

use super::{initialized, nothing_to_undo, on_disk};

/// The operating system the commands are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and the others.
    Other,
}

impl System {
    /// The system Gitardo runs on.
    pub const CURRENT: System = if cfg!(windows) {
        System::Windows
    } else if cfg!(target_os = "macos") {
        System::MacOs
    } else {
        System::Other
    };
}

/// The commands that open a folder outside Gitardo, through the system's shell, on a system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenCommands(pub System);

impl OpenCommands {
    /// The command that shows a folder in the file manager. On Windows, `start` opens it in File Explorer and ends
    /// well, where `explorer` ends with 1 even when it opened it.
    #[must_use]
    pub fn reveal(self, folder: &str) -> String {
        let folder = quoted(folder);
        match self.0 {
            System::Windows => format!("start \"\" {folder}"),
            System::MacOs => format!("open {folder}"),
            System::Other => format!("xdg-open {folder}"),
        }
    }

    /// The command that opens the terminal of the system in the folder it runs in.
    #[must_use]
    pub fn terminal(self) -> &'static str {
        match self.0 {
            System::Windows => "start \"\" cmd",
            System::MacOs => "open -a Terminal .",
            System::Other => "x-terminal-emulator",
        }
    }
}

/// Opens the folder of an initialized submodule in a new window of Gitardo Desktop, as a repository of its own: no
/// steps, the window opens it once the plan ran.
///
/// Feature: submodule.open-in-new-window
#[must_use]
pub fn open_in_new_window(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match initialized(context, ": update it first.") {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Open {} in a new window", submodule.folder)),
        steps: Vec::new(),
        danger: Danger::None,
        undo: nothing_to_undo(),
        after: vec![FollowUp::OpenWindow(on_disk(context, submodule))],
        on_failure: Vec::new(),
    })
}

/// Opens the terminal of the system with the folder of an initialized submodule as its folder: `run <command>` in that
/// folder.
///
/// Feature: submodule.open-in-terminal
#[must_use]
pub fn open_in_terminal(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match initialized(context, ": update it first.") {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    Outcome::Run(Plan {
        title: Text::from(format!("Open a terminal in {}", submodule.folder)),
        steps: vec![Step::Run {
            command: OpenCommands(System::CURRENT).terminal().to_owned(),
            directory: on_disk(context, submodule),
        }],
        danger: Danger::None,
        undo: nothing_to_undo(),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// Shows the folder of an initialized submodule in the file manager of the system: `run <command>`.
///
/// Feature: submodule.reveal
#[must_use]
pub fn reveal(context: &Context, _answers: &Answers) -> Outcome {
    let submodule = match initialized(context, ": update it first.") {
        Ok(submodule) => submodule,
        Err(refusal) => return *refusal,
    };
    let folder = on_disk(context, submodule);
    Outcome::Run(Plan {
        title: Text::from(format!("Reveal {}", submodule.folder)),
        steps: vec![Step::Run {
            command: OpenCommands(System::CURRENT).reveal(&folder.display().to_string()),
            directory: folder,
        }],
        danger: Danger::None,
        undo: nothing_to_undo(),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

/// A path as the shell takes it as one word: in double quotes when it has a space or a character the shell reads.
fn quoted(path: &str) -> String {
    let is_plain = path
        .chars()
        .all(|character| character.is_alphanumeric() || r"/\:._-+~".contains(character));
    if is_plain {
        path.to_owned()
    } else {
        format!("\"{path}\"")
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use model::contract::{Answers, FollowUp, Outcome, Step};

    use super::{OpenCommands, System, open_in_new_window, open_in_terminal, reveal};
    use crate::submodule::testing::{lib, on};
    use crate::testing::plan;

    #[test]
    fn submodule_open_in_new_window__opens_the_folder_of_lib_in_a_window_of_its_own() {
        let plan = plan(open_in_new_window(&on(lib()), &Answers::new()));
        assert_eq!(plan.steps, []);
        assert_eq!(
            plan.after,
            [FollowUp::OpenWindow(PathBuf::from("repository").join("lib"))]
        );
        let mut empty = lib();
        empty.checkout = None;
        let Outcome::Refuse(refusal) = open_in_new_window(&on(empty), &Answers::new()) else {
            panic!("a submodule that is not initialized is refused");
        };
        assert_eq!(refusal.message.as_str(), "lib is not initialized: update it first.");
    }

    #[test]
    fn submodule_open_in_terminal__the_terminal_opens_with_the_folder_of_the_submodule_as_its_folder() {
        let plan = plan(open_in_terminal(&on(lib()), &Answers::new()));
        let [Step::Run { command, directory }] = plan.steps.as_slice() else {
            panic!("one command: {:?}", plan.steps);
        };
        assert_eq!(directory, &PathBuf::from("repository").join("lib"));
        assert_eq!(command, OpenCommands(System::CURRENT).terminal());
        assert_eq!(OpenCommands(System::Windows).terminal(), "start \"\" cmd");
    }

    #[test]
    fn submodule_reveal__on_each_system_the_file_manager_opens_on_the_folder_of_the_submodule() {
        let plan = plan(reveal(&on(lib()), &Answers::new()));
        let [Step::Run { directory, .. }] = plan.steps.as_slice() else {
            panic!("one command: {:?}", plan.steps);
        };
        assert_eq!(directory, &PathBuf::from("repository").join("lib"));
        let cases = [
            (System::Windows, r#"start "" "C:\work\my repo\lib""#),
            (System::MacOs, r#"open "C:\work\my repo\lib""#),
            (System::Other, r#"xdg-open "C:\work\my repo\lib""#),
        ];
        for (system, expected) in cases {
            assert_eq!(OpenCommands(system).reveal(r"C:\work\my repo\lib"), expected);
        }
    }
}
