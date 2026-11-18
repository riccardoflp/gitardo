//! The variables of `tasks.json` (`tasks.read`): those Gitardo resolves, with the worktree shown as the folder, and
//! those only VS Code can, which a task is listed with and cannot be run with.

use model::tasks::{TaskSystem, TasksContext};

/// The variables of VS Code that stand for something Gitardo does not have: the file of the editor, its selection,
/// the program of VS Code, the default build task.
const OF_VS_CODE: [&str; 15] = [
    "file",
    "fileWorkspaceFolder",
    "fileWorkspaceFolderBasename",
    "relativeFile",
    "relativeFileDirname",
    "fileBasename",
    "fileBasenameNoExtension",
    "fileExtname",
    "fileDirname",
    "fileDirnameBasename",
    "lineNumber",
    "columnNumber",
    "selectedText",
    "execPath",
    "defaultBuildTask",
];

/// The variables with an argument that only VS Code resolves, by what comes before their colon: an input asked of
/// the user, a command of VS Code, one of its settings, a folder of a workspace of several, an extension.
const WITH_ARGUMENT: [&str; 5] = [
    "input",
    "command",
    "config",
    "workspaceFolder",
    "extensionInstallFolder",
];

/// A text of `tasks.json` with its variables resolved: `${workspaceFolder}` and `${cwd}` are the worktree shown,
/// `${workspaceFolderBasename}` the name of its folder, `${userHome}` the home folder, `${pathSeparator}` and `${/}`
/// what separates folders on the system, `${env:NAME}` a variable of the environment, empty when it is not set. A
/// variable only VS Code resolves stays as written and is added to `unresolved`, once; anything else between `${` and
/// `}` is not a variable of VS Code, such as `${HOME}` for a shell, and stays as written.
pub(super) fn resolved(text: &str, context: &TasksContext, unresolved: &mut Vec<String>) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        let (before, variable) = rest.split_at(start);
        result.push_str(before);
        let Some(end) = variable.find('}') else {
            rest = variable;
            break;
        };
        let (written, after) = variable.split_at(end + 1);
        let name = written
            .strip_prefix("${")
            .and_then(|name| name.strip_suffix('}'))
            .unwrap_or_default();
        match value(name, context) {
            Variable::Resolved(value) => result.push_str(&value),
            Variable::OfVsCode => {
                if !unresolved.iter().any(|known| known == written) {
                    unresolved.push(written.to_owned());
                }
                result.push_str(written);
            }
            Variable::NotOne => result.push_str(written),
        }
        rest = after;
    }
    result.push_str(rest);
    result
}

/// What a name between `${` and `}` is.
enum Variable {
    /// A variable Gitardo resolves, with its value.
    Resolved(String),
    /// A variable only VS Code resolves.
    OfVsCode,
    /// Not a variable of `tasks.json`.
    NotOne,
}

/// The value of a variable, by its name without `${` and `}`.
fn value(name: &str, context: &TasksContext) -> Variable {
    let folder = || context.folder.to_string_lossy().into_owned();
    match name {
        // `workspaceRoot` is the name VS Code had for it before workspaces of several folders.
        "workspaceFolder" | "workspaceRoot" | "cwd" => Variable::Resolved(folder()),
        "workspaceFolderBasename" => {
            let name = context.folder.file_name().unwrap_or_default();
            Variable::Resolved(name.to_string_lossy().into_owned())
        }
        "userHome" => Variable::Resolved(context.home.to_string_lossy().into_owned()),
        "pathSeparator" | "/" => Variable::Resolved(context.system.path_separator().to_owned()),
        _ => match name.split_once(':') {
            Some(("env", variable)) => Variable::Resolved(environment(variable, context)),
            Some((kind, _)) if WITH_ARGUMENT.contains(&kind) => Variable::OfVsCode,
            None if OF_VS_CODE.contains(&name) => Variable::OfVsCode,
            // Anything else is the shell's own, or text.
            Some(_) | None => Variable::NotOne,
        },
    }
}

/// A variable of the environment, empty when it is not set; on Windows the case of its name does not count.
fn environment(name: &str, context: &TasksContext) -> String {
    let found = match context.system {
        TaskSystem::Windows => context
            .environment
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map(|(_, value)| value),
        TaskSystem::Linux | TaskSystem::MacOs => context.environment.get(name),
    };
    found.cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use model::tasks::{TaskFiles, TaskSystem, TasksContext};

    use super::resolved;

    fn context(system: TaskSystem) -> TasksContext {
        TasksContext {
            folder: PathBuf::from("work").join("app"),
            system,
            home: PathBuf::from("home"),
            environment: [
                (String::from("Path"), String::from("bin")),
                (String::from("CI"), String::from("1")),
            ]
            .into_iter()
            .collect(),
            files: TaskFiles::default(),
        }
    }

    fn resolve(text: &str, system: TaskSystem) -> (String, Vec<String>) {
        let mut unresolved = Vec::new();
        let text = resolved(text, &context(system), &mut unresolved);
        (text, unresolved)
    }

    #[test]
    fn resolves_the_folder_its_name_the_home_the_separator_and_the_environment() {
        let folder = PathBuf::from("work").join("app").display().to_string();
        let cases = [
            ("${workspaceFolder}/out", format!("{folder}/out")),
            ("${cwd}", folder.clone()),
            ("${workspaceRoot}", folder.clone()),
            ("${workspaceFolderBasename}.log", String::from("app.log")),
            ("${userHome}", String::from("home")),
            ("a${pathSeparator}b${/}c", String::from("a/b/c")),
            ("${env:CI}-${env:MISSING}-", String::from("1--")),
            ("no variables", String::from("no variables")),
            ("", String::new()),
        ];
        for (text, expected) in cases {
            assert_eq!(resolve(text, TaskSystem::Linux), (expected, Vec::new()), "{text}");
        }
        assert_eq!(resolve("a${pathSeparator}b", TaskSystem::Windows).0, "a\\b");
    }

    #[test]
    fn the_case_of_a_name_of_the_environment_counts_everywhere_but_on_windows() {
        assert_eq!(resolve("${env:PATH}", TaskSystem::Windows).0, "bin");
        assert_eq!(resolve("${env:PATH}", TaskSystem::Linux).0, "");
        assert_eq!(resolve("${env:Path}", TaskSystem::MacOs).0, "bin");
    }

    #[test]
    fn leaves_what_only_vs_code_resolves_as_written_and_names_each_once() {
        let (text, unresolved) = resolve(
            "${file} ${input:name} ${command:pick} ${config:a.b} ${input:name} ${workspaceFolder:api}",
            TaskSystem::Linux,
        );
        assert_eq!(
            text,
            "${file} ${input:name} ${command:pick} ${config:a.b} ${input:name} ${workspaceFolder:api}"
        );
        assert_eq!(
            unresolved,
            [
                "${file}",
                "${input:name}",
                "${command:pick}",
                "${config:a.b}",
                "${workspaceFolder:api}"
            ]
        );
    }

    #[test]
    fn leaves_what_is_the_shells_own_as_written() {
        for text in [
            "echo ${HOME}",
            "echo ${1:-default}",
            "${",
            "a ${unclosed",
            "$workspaceFolder",
            "${}",
        ] {
            assert_eq!(
                resolve(text, TaskSystem::Linux),
                (text.to_owned(), Vec::new()),
                "{text}"
            );
        }
        assert_eq!(resolve("${HOME} ${cwd} ${x", TaskSystem::Linux).0, {
            let folder = PathBuf::from("work").join("app").display().to_string();
            format!("${{HOME}} {folder} ${{x")
        });
    }
}
