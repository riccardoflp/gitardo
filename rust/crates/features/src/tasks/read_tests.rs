#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::path::PathBuf;

use model::tasks::{NotRunnable, PackageManager, TaskKind, TaskReference, TaskSystem, Tasks, Word};

use super::read_tasks;
use crate::tasks::testing::{EXTENSION_PACKAGE, TWO_SHELL_TASKS, context, folder, names, one, read, with_packages};

#[test]
fn tasks_read__a_tasks_json_with_a_comment_and_two_shell_tasks_lists_both_in_its_order_under_workspace() {
    let tasks = read(TaskSystem::Linux, TWO_SHELL_TASKS);
    assert_eq!(tasks.problems, Vec::new());
    assert_eq!(tasks.packages, Vec::new());
    assert_eq!(names(&tasks.workspace), ["build", "check"]);
    let [build, check] = tasks.workspace.as_slice() else {
        panic!("two tasks");
    };
    assert_eq!(build.reference, TaskReference::workspace("build"));
    assert_eq!(build.kind, TaskKind::Shell);
    assert_eq!(build.command_line(), "cargo build");
    assert_eq!(build.folder, folder());
    assert_eq!(build.not_runnable(), None);
    assert_eq!(build.line, Some(6), "the line of its label");
    assert_eq!(check.command, Some(Word::plain("cargo")));
    assert_eq!(check.arguments, [Word::plain("xtask"), Word::plain("check")]);
    assert_eq!(check.command_line(), "cargo xtask check");
    assert_eq!(check.line, Some(11));
}

#[test]
fn tasks_read__a_task_with_windows_command_runs_it_on_windows_and_its_own_command_elsewhere() {
    let text = r#"{ "tasks": [{
        "label": "open", "type": "shell", "command": "xdg-open", "args": ["out"],
        "windows": { "command": "start" },
        "osx": { "command": "open", "args": ["-a", "Preview", "out"] }
    }] }"#;
    let line = |system| read(system, text).workspace.first().unwrap().command_line();
    assert_eq!(
        line(TaskSystem::Windows),
        "start out",
        "its arguments stay where the system gives none"
    );
    assert_eq!(line(TaskSystem::Linux), "xdg-open out");
    assert_eq!(line(TaskSystem::MacOs), "open -a Preview out");
}

#[test]
fn tasks_read__a_system_overrides_the_options_too_and_adds_to_the_environment() {
    let text = r#"{ "tasks": [{
        "label": "serve", "type": "shell", "command": "serve",
        "options": { "cwd": "web", "env": { "PORT": "8080", "MODE": "dev" } },
        "linux": { "options": { "cwd": "web/linux", "env": { "PORT": "9090", "DISPLAY": ":0" } } }
    }] }"#;
    let on = |system| read(system, text).workspace.into_iter().next().unwrap();
    let pairs = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    };
    let windows = on(TaskSystem::Windows);
    assert_eq!(windows.folder, folder().join("web"));
    assert_eq!(windows.environment, pairs(&[("PORT", "8080"), ("MODE", "dev")]));
    let linux = on(TaskSystem::Linux);
    assert_eq!(linux.folder, folder().join("web/linux"));
    assert_eq!(
        linux.environment,
        pairs(&[("PORT", "9090"), ("MODE", "dev"), ("DISPLAY", ":0")])
    );
}

#[test]
fn tasks_read__workspace_folder_in_options_cwd_is_the_folder_of_the_worktree_shown_in_a_linked_worktree_too() {
    let text = r#"{ "tasks": [{
        "label": "build", "type": "shell", "command": "make",
        "args": ["-C", "${workspaceFolder}", "NAME=${workspaceFolderBasename}"],
        "options": { "cwd": "${workspaceFolder}/rust" }
    }] }"#;
    let main = read(TaskSystem::Linux, text).workspace.into_iter().next().unwrap();
    assert_eq!(main.folder, PathBuf::from(format!("{}/rust", folder().display())));
    let linked = PathBuf::from("/work/app.worktrees/feature");
    let mut context = context(TaskSystem::Linux, Some(text));
    context.folder.clone_from(&linked);
    let task = read_tasks(&context).workspace.into_iter().next().unwrap();
    assert_eq!(task.folder, PathBuf::from(format!("{}/rust", linked.display())));
    assert_eq!(
        task.command_line(),
        format!("make -C {} NAME=feature", linked.display()),
        "the folder of the linked worktree, not of the main one"
    );
    assert_eq!(task.unresolved, Vec::<String>::new());
}

#[test]
fn tasks_read__the_home_the_separator_and_the_environment_are_resolved_in_what_a_task_runs() {
    let task = one(
        r#"{ "label": "serve", "type": "process", "command": "${userHome}${pathSeparator}bin${/}serve",
            "args": ["--port", "${env:PORT}", "--from", "${cwd}"],
            "options": { "env": { "HOME_BIN": "${userHome}/bin" }, "shell": { "executable": "${env:PORT}sh" } } }"#,
    );
    assert_eq!(task.kind, TaskKind::Process);
    assert_eq!(task.command, Some(Word::plain("home/bin/serve")));
    assert_eq!(
        task.arguments.iter().map(|word| word.value.clone()).collect::<Vec<_>>(),
        ["--port", "3000", "--from", &folder().display().to_string()]
    );
    assert_eq!(task.environment, [(String::from("HOME_BIN"), String::from("home/bin"))]);
    assert_eq!(task.shell.map(|shell| shell.executable), Some(String::from("3000sh")));
}

#[test]
fn tasks_read__a_task_with_an_input_is_listed_and_says_it_cannot_be_run_because_of_it() {
    let task = one(
        r#"{ "label": "deploy", "type": "shell", "command": "deploy ${input:name}",
            "args": ["--file", "${file}", "${input:name}"] }"#,
    );
    assert_eq!(task.reference, TaskReference::workspace("deploy"));
    assert_eq!(task.command_line(), "deploy ${input:name} --file ${file} ${input:name}");
    assert_eq!(task.unresolved, ["${input:name}", "${file}"]);
    let why = task.not_runnable().unwrap();
    assert_eq!(
        why,
        NotRunnable::Unresolved(vec![String::from("${input:name}"), String::from("${file}")])
    );
    assert_eq!(
        why.said(),
        "It cannot be run with `${input:name}`, `${file}`, which only VS Code resolves."
    );
}

#[test]
fn tasks_read__a_task_of_type_cargo_is_listed_dimmed_saying_it_runs_in_vs_code_only() {
    let text = r#"{ "tasks": [
        { "type": "cargo", "command": "build", "label": "rust: cargo build" },
        { "type": "npm", "script": "watch" },
        { "type": "typescript", "tsconfig": "tsconfig.json" }
    ] }"#;
    let tasks = read(TaskSystem::Linux, text);
    assert_eq!(
        names(&tasks.workspace),
        ["rust: cargo build", "npm: watch", "typescript"]
    );
    let cargo = tasks.workspace.first().unwrap();
    assert_eq!(cargo.kind, TaskKind::Other(String::from("cargo")));
    let why = cargo.not_runnable().unwrap();
    assert_eq!(why, NotRunnable::OtherKind(String::from("cargo")));
    assert_eq!(why.said(), "This kind of task runs in VS Code only.");
    assert!(tasks.workspace.iter().all(|task| task.not_runnable().is_some()));
}

#[test]
fn tasks_read__the_scripts_of_extension_package_json_are_under_npm_in_a_group_named_extension() {
    let files = [
        ("extension/package.json", EXTENSION_PACKAGE),
        ("extension/node_modules/vitest/package.json", EXTENSION_PACKAGE),
        ("node_modules/left-pad/package.json", EXTENSION_PACKAGE),
    ];
    let tasks = read_tasks(&with_packages(context(TaskSystem::Linux, None), &files, &[]));
    assert_eq!(tasks.problems, Vec::new());
    assert_eq!(tasks.workspace, Vec::new());
    let [package] = tasks.packages.as_slice() else {
        panic!(
            "a `package.json` under `node_modules` lists nothing: {:?}",
            tasks.packages
        );
    };
    assert_eq!(
        (package.name.as_str(), package.folder.as_str()),
        ("extension", "extension")
    );
    assert_eq!(names(&package.scripts), ["build", "test"]);
    let build = package.scripts.first().unwrap();
    assert_eq!(build.reference, TaskReference::npm("extension", "build"));
    assert_eq!(build.reference.to_string(), "npm: build - extension");
    assert_eq!(build.kind, TaskKind::Shell);
    assert_eq!(build.command_line(), "npm run build");
    assert_eq!(
        build.script.as_deref(),
        Some("node esbuild.mjs"),
        "its command under its name"
    );
    assert_eq!(build.folder, folder().join("extension"));
    assert_eq!(build.line, None);
}

#[test]
fn tasks_read__with_pnpm_lock_yaml_beside_the_package_json_build_runs_pnpm_run_build() {
    let files = [
        ("extension/package.json", EXTENSION_PACKAGE),
        ("package.json", EXTENSION_PACKAGE),
    ];
    let context = with_packages(
        context(TaskSystem::Linux, None),
        &files,
        &["extension/pnpm-lock.yaml", "yarn.lock"],
    );
    let tasks = read_tasks(&context);
    let [top, extension] = tasks.packages.as_slice() else {
        panic!("two files, the one at the top first: {:?}", tasks.packages);
    };
    assert_eq!(extension.manager, PackageManager::Pnpm);
    assert_eq!(extension.scripts.first().unwrap().command_line(), "pnpm run build");
    assert_eq!(
        (top.folder.as_str(), top.name.as_str()),
        ("", "app"),
        "named by the folder of the worktree"
    );
    assert_eq!(top.manager, PackageManager::Yarn);
    assert_eq!(top.scripts.last().unwrap().command_line(), "yarn run test");
    assert_eq!(top.scripts.last().unwrap().reference.to_string(), "npm: test");
    let without = read_tasks(&with_packages(context.clone(), &files[..1], &[]));
    assert_eq!(
        without
            .packages
            .first()
            .unwrap()
            .scripts
            .first()
            .unwrap()
            .command_line(),
        "npm run build"
    );
    let above = read_tasks(&with_packages(context, &files[..1], &["bun.lockb"]));
    assert_eq!(
        above.packages.first().unwrap().scripts.first().unwrap().command_line(),
        "bun run build",
        "the nearest lock file above it"
    );
}

#[test]
fn tasks_read__a_tasks_json_with_a_missing_comma_lists_the_npm_scripts_and_says_it_could_not_be_read() {
    let broken = "{\n  \"version\": \"2.0.0\"\n  \"tasks\": []\n}\n";
    let context = with_packages(
        context(TaskSystem::Linux, Some(broken)),
        &[("extension/package.json", EXTENSION_PACKAGE)],
        &[],
    );
    let tasks = read_tasks(&context);
    assert_eq!(tasks.workspace, Vec::new());
    assert_eq!(names(&tasks.packages.first().unwrap().scripts), ["build", "test"]);
    let [problem] = tasks.problems.as_slice() else {
        panic!("one problem: {:?}", tasks.problems);
    };
    assert_eq!(problem.file, ".vscode/tasks.json");
    assert_eq!(
        problem.said(),
        "`.vscode/tasks.json` could not be read: at line 3, column 3, a comma or a closing brace was expected."
    );
}

#[test]
fn tasks_read__a_package_json_that_is_not_json_lists_nothing_of_it_and_says_so() {
    let files = [
        ("package.json", "{ \"scripts\": "),
        ("extension/package.json", EXTENSION_PACKAGE),
    ];
    let tasks = read_tasks(&with_packages(
        context(TaskSystem::Linux, Some(TWO_SHELL_TASKS)),
        &files,
        &[],
    ));
    assert_eq!(names(&tasks.workspace), ["build", "check"]);
    assert_eq!(tasks.packages.len(), 1);
    let said: Vec<String> = tasks.problems.iter().map(model::tasks::TaskFileProblem::said).collect();
    assert_eq!(
        said,
        ["`package.json` could not be read: at line 1, column 14, the text ends before the value is complete."]
    );
}

#[test]
fn tasks_read__a_worktree_with_no_file_of_tasks_or_an_empty_one_lists_nothing_and_says_nothing() {
    assert_eq!(read_tasks(&context(TaskSystem::Linux, None)), Tasks::default());
    assert_eq!(read(TaskSystem::Linux, "  \n"), Tasks::default());
    assert_eq!(read(TaskSystem::Linux, "{ \"version\": \"2.0.0\" }"), Tasks::default());
    let without_scripts = with_packages(context(TaskSystem::Linux, None), &[("package.json", "{}")], &[]);
    assert_eq!(read_tasks(&without_scripts), Tasks::default());
}
