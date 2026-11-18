//! How a task starts (`tasks.run`): the program and the arguments its pseudo-terminal is given, a shell with the
//! command line for a `shell` task and the command itself for a `process` task; and what the Tasks tab asks an
//! operation to run or to stop, with what the operation answers the window.

use std::path::{Path, PathBuf};

use super::{Quoting, RunOrder, Task, TaskGroup, TaskKind, TaskReference, Tasks, Word};

/// What the pseudo-terminal of a task starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStart {
    /// The program: the shell of a `shell` task, the command of a `process` task.
    pub program: String,
    /// Its arguments, each as the program gets it.
    pub arguments: Vec<String>,
    /// The folder it starts in.
    pub folder: PathBuf,
    /// `options.env`, set over the environment of Gitardo.
    pub environment: Vec<(String, String)>,
}

/// The family of a shell, which says how it is given a command line and how it quotes a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellFamily {
    /// `cmd.exe`.
    Cmd,
    /// Windows PowerShell and PowerShell 7.
    PowerShell,
    /// `sh`, `bash`, `zsh` and the others that take `-c`.
    Posix,
}

impl ShellFamily {
    /// The family of a shell, from the name of its program.
    fn of(program: &str) -> ShellFamily {
        let name = program.rsplit(['/', '\\']).next().unwrap_or(program).to_lowercase();
        let name = name.strip_suffix(".exe").unwrap_or(&name);
        match name {
            "cmd" => ShellFamily::Cmd,
            "powershell" | "pwsh" => ShellFamily::PowerShell,
            // Every other shell is taken as one that runs a command line given after `-c`.
            _ => ShellFamily::Posix,
        }
    }

    /// The arguments that have the shell run the command line that follows them, then end.
    fn run_arguments(self) -> Vec<String> {
        let arguments: &[&str] = match self {
            ShellFamily::Cmd => &["/d", "/c"],
            ShellFamily::PowerShell => &["-NoLogo", "-Command"],
            ShellFamily::Posix => &["-c"],
        };
        arguments.iter().map(|argument| (*argument).to_owned()).collect()
    }

    /// A word as the shell reads it back whole, quoted as `tasks.json` asks.
    fn quoted(self, word: &Word) -> String {
        let value = word.value.as_str();
        match (word.quoting, self) {
            (Quoting::Plain, _) if !value.contains(' ') => value.to_owned(),
            // `cmd.exe` has no character that escapes: a word to escape goes between quotes there.
            (Quoting::Plain | Quoting::Strong | Quoting::Escape, ShellFamily::Cmd) | (Quoting::Weak, _) => {
                format!("\"{value}\"")
            }
            (Quoting::Plain | Quoting::Strong, ShellFamily::PowerShell) => format!("'{}'", value.replace('\'', "''")),
            (Quoting::Plain | Quoting::Strong, ShellFamily::Posix) => format!("'{}'", value.replace('\'', "'\\''")),
            (Quoting::Escape, ShellFamily::PowerShell) => escaped(value, '`'),
            (Quoting::Escape, ShellFamily::Posix) => escaped(value, '\\'),
        }
    }
}

/// A word with its spaces and its quotes each after the character that escapes them.
fn escaped(value: &str, escape: char) -> String {
    let mut word = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, ' ' | '"' | '\'') {
            word.push(escape);
        }
        word.push(character);
    }
    word
}

/// The command line of a `shell` task as its shell reads it: the command as written, since a command written as one
/// string is a whole line (`npm run build`), unless `tasks.json` asks to quote it, then each argument, quoted when it
/// holds a space.
fn shell_line(task: &Task, family: ShellFamily) -> String {
    let command = task.command.iter().map(|command| match command.quoting {
        Quoting::Plain => command.value.clone(),
        Quoting::Escape | Quoting::Strong | Quoting::Weak => family.quoted(command),
    });
    let arguments = task.arguments.iter().map(|argument| family.quoted(argument));
    let words: Vec<String> = command.chain(arguments).collect();
    words.join(" ")
}

/// What the pseudo-terminal of a task starts: for a `shell` task the shell of `options.shell`, else the one given, the
/// shell of `terminal.shellPath` or the system's own, with the arguments that have it run the command line; for a
/// `process` task its command with its arguments as given, with no shell. None for a task that runs nothing itself:
/// one with no command, or of a kind only VS Code runs.
#[must_use]
pub fn task_start(task: &Task, shell: &str) -> Option<TaskStart> {
    let command = task.command.as_ref()?;
    let (program, arguments) = match task.kind {
        TaskKind::Other(_) => return None,
        TaskKind::Process => (
            command.value.clone(),
            task.arguments.iter().map(|argument| argument.value.clone()).collect(),
        ),
        TaskKind::Shell => {
            let asked = task.shell.as_ref();
            let program = asked.map_or(shell, |asked| asked.executable.as_str()).to_owned();
            let family = ShellFamily::of(&program);
            let mut arguments = asked
                .and_then(|asked| asked.arguments.clone())
                .unwrap_or_else(|| family.run_arguments());
            arguments.push(shell_line(task, family));
            (program, arguments)
        }
    };
    Some(TaskStart {
        program,
        arguments,
        folder: task.folder.clone(),
        environment: task.environment.clone(),
    })
}

/// What an operation of the tasks is asked to run or to stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskChoice {
    /// A task.
    Task(TaskReference),
    /// A group, with its tasks in their order.
    Group(TaskGroup),
    /// No task: the folder of the worktree shown, to trust so that its tasks can run.
    Trust,
    /// No task: `.vscode/tasks.json`, to make with no tasks in a worktree that has none.
    MakeFile,
}

/// What the Tasks tab gives the operations that run and stop tasks (`tasks.run`, `tasks.stop`, `tasks.run-group`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTarget {
    /// The tasks of the worktree shown, as last read.
    pub tasks: Tasks,
    /// The task or the group.
    pub choice: TaskChoice,
    /// The tasks running now.
    pub running: Vec<TaskReference>,
}

/// What an operation asks the window to run (`tasks.run`, `tasks.run-group`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TasksToRun {
    /// The runs to start, in order, each a task with those it depends on.
    pub orders: Vec<RunOrder>,
    /// The tasks running that are ended first, to run again.
    pub restart: Vec<TaskReference>,
    /// The task the Tasks tab selects and shows.
    pub show: Option<TaskReference>,
}

/// The folder a task of a worktree is said to run in when it is not the worktree itself: from the top of the
/// worktree with `/`, or whole when it is outside it.
#[must_use]
pub fn folder_said(folder: &Path, worktree: &Path) -> String {
    let said = folder.strip_prefix(worktree).unwrap_or(folder);
    let said = said.to_string_lossy().replace('\\', "/");
    if said.is_empty() { String::from(".") } else { said }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::super::testing::task;
    use super::super::{Quoting, TaskKind, TaskShell, Word};
    use super::{folder_said, task_start};

    #[test]
    fn tasks_run__a_shell_task_runs_its_command_line_with_the_shell_given_and_the_arguments_that_run_a_line() {
        let mut build = task("build", &[]);
        build.command = Some(Word::plain("npm run build"));
        build.arguments = vec![Word::plain("--"), Word::plain("two words")];
        let start = task_start(&build, "pwsh.exe").unwrap();
        assert_eq!(start.program, "pwsh.exe");
        assert_eq!(start.arguments, ["-NoLogo", "-Command", "npm run build -- 'two words'"]);
        assert_eq!(start.folder, PathBuf::from("repository"));
        let start = task_start(&build, "/bin/zsh").unwrap();
        assert_eq!(start.arguments, ["-c", "npm run build -- 'two words'"]);
        let start = task_start(&build, r"C:\Windows\System32\cmd.exe").unwrap();
        assert_eq!(start.arguments, ["/d", "/c", "npm run build -- \"two words\""]);
    }

    #[test]
    fn tasks_run__the_shell_of_the_task_is_used_in_place_of_the_one_given_with_its_own_arguments() {
        let mut build = task("build", &[]);
        build.shell = Some(TaskShell {
            executable: String::from("/bin/bash"),
            arguments: None,
        });
        build.environment = vec![(String::from("FOO"), String::from("1"))];
        let start = task_start(&build, "pwsh.exe").unwrap();
        assert_eq!((start.program.as_str(), start.arguments.len()), ("/bin/bash", 2));
        assert_eq!(start.arguments.first().map(String::as_str), Some("-c"));
        assert_eq!(start.environment, [(String::from("FOO"), String::from("1"))]);
        build.shell = Some(TaskShell {
            executable: String::from("/bin/bash"),
            arguments: Some(vec![String::from("-lc")]),
        });
        assert_eq!(task_start(&build, "pwsh.exe").unwrap().arguments, ["-lc", "run-build"]);
    }

    #[test]
    fn tasks_run__a_process_task_gets_its_arguments_as_given_with_no_shell() {
        let mut list = task("list", &[]);
        list.kind = TaskKind::Process;
        list.command = Some(Word::plain("git"));
        list.arguments = vec![Word::plain("log"), Word::plain("--format=%h %s")];
        let start = task_start(&list, "pwsh.exe").unwrap();
        assert_eq!(start.program, "git");
        assert_eq!(start.arguments, ["log", "--format=%h %s"]);
    }

    #[test]
    fn tasks_run__words_are_quoted_as_tasks_json_asks_for_each_shell() {
        let mut say = task("say", &[]);
        say.command = Some(Word::plain("echo"));
        say.arguments = vec![
            Word {
                value: String::from("it's here"),
                quoting: Quoting::Strong,
            },
            Word {
                value: String::from("$HOME dir"),
                quoting: Quoting::Weak,
            },
            Word {
                value: String::from("a b"),
                quoting: Quoting::Escape,
            },
        ];
        let line = |shell: &str| task_start(&say, shell).unwrap().arguments.pop().unwrap();
        assert_eq!(line("/bin/sh"), r#"echo 'it'\''s here' "$HOME dir" a\ b"#);
        assert_eq!(line("powershell.exe"), "echo 'it''s here' \"$HOME dir\" a` b");
        assert_eq!(line("cmd.exe"), "echo \"it's here\" \"$HOME dir\" \"a b\"");
    }

    #[test]
    fn tasks_run__a_task_with_no_command_or_of_another_kind_starts_nothing_itself() {
        let mut all = task("all", &["build"]);
        all.command = None;
        assert_eq!(task_start(&all, "sh"), None);
        let mut cargo = task("cargo", &[]);
        cargo.kind = TaskKind::Other(String::from("cargo"));
        assert_eq!(task_start(&cargo, "sh"), None);
    }

    #[test]
    fn tasks_view__the_folder_of_a_task_is_said_from_the_top_of_the_worktree() {
        let worktree = PathBuf::from("/work/app");
        assert_eq!(folder_said(&worktree.join("extension"), &worktree), "extension");
        assert_eq!(folder_said(&worktree, &worktree), ".");
        assert_eq!(folder_said(&PathBuf::from("/tmp/out"), &worktree), "/tmp/out");
    }
}
