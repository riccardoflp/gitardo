#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::tasks::{
    Background, DependsOrder, Quoting, Task, TaskCategory, TaskKind, TaskReference, TaskShell, TaskSystem,
};

use crate::tasks::read_tasks;
use crate::tasks::testing::{EXTENSION_PACKAGE, TWO_SHELL_TASKS, context, folder, names, one, read, with_packages};

#[test]
fn tasks_read__a_task_with_hide_is_read_and_marked_so_that_it_runs_as_one_another_depends_on() {
    let task = one(r#"{ "label": "prepare", "type": "shell", "command": "make prepare", "hide": true }"#);
    assert!(task.is_hidden);
    assert!(!one(r#"{ "label": "build", "command": "make" }"#).is_hidden);
}

#[test]
fn tasks_read__the_group_the_tasks_depended_on_and_their_order_are_read() {
    let text = r#"{ "tasks": [
        { "label": "all", "dependsOn": ["build", "npm: test - extension", { "type": "npm", "script": "build", "path": "extension/" }, "gone"],
          "dependsOrder": "sequence", "group": { "kind": "build", "isDefault": true } },
        { "label": "build", "type": "shell", "command": "make", "dependsOn": "npm: build", "group": "build" },
        { "label": "npm: build", "type": "shell", "command": "echo not the script" }
    ] }"#;
    let files = [
        ("extension/package.json", EXTENSION_PACKAGE),
        ("package.json", EXTENSION_PACKAGE),
    ];
    let tasks = read_tasks(&with_packages(context(TaskSystem::Linux, Some(text)), &files, &[]));
    let [all, build, _] = tasks.workspace.as_slice() else {
        panic!("three tasks");
    };
    assert_eq!(all.command, None, "a task that only runs those it depends on");
    assert_eq!(all.depends_order, DependsOrder::Sequence);
    assert_eq!(
        all.depends_on,
        [
            TaskReference::workspace("build"),
            TaskReference::npm("extension", "test"),
            TaskReference::npm("extension", "build"),
            TaskReference::workspace("gone"),
        ]
    );
    let category = |kind: &str, is_default| {
        Some(TaskCategory {
            kind: kind.to_owned(),
            is_default,
        })
    };
    assert_eq!(all.category, category("build", true));
    assert_eq!(build.category, category("build", false));
    assert_eq!(build.depends_order, DependsOrder::Parallel);
    assert_eq!(
        build.depends_on,
        [TaskReference::workspace("npm: build")],
        "a label of the file counts before the script VS Code names so"
    );
    let script_at_the_top = one(r#"{ "label": "x", "dependsOn": ["npm: build"] }"#);
    assert_eq!(
        script_at_the_top.depends_on,
        [TaskReference::workspace("npm: build")],
        "no such script here"
    );
}

#[test]
fn tasks_read__a_background_task_has_the_patterns_of_its_problem_matcher() {
    let watch = one(
        r#"{ "label": "watch", "type": "shell", "command": "tsc -w", "isBackground": true,
            "problemMatcher": ["$tsc", { "owner": "custom", "pattern": { "regexp": "^x$" },
                "background": { "activeOnStart": true, "beginsPattern": "^Starting", "endsPattern": { "regexp": "Watching for file changes" } } }] }"#,
    );
    assert_eq!(
        watch.background,
        Some(Background {
            begins_pattern: Some(String::from("^Starting")),
            ends_pattern: Some(String::from("Watching for file changes")),
        })
    );
    let named = one(r#"{ "label": "watch", "command": "tsc", "isBackground": true, "problemMatcher": "$tsc-watch" }"#);
    assert_eq!(
        named.background,
        Some(Background::default()),
        "a matcher of VS Code's own has no patterns here"
    );
    let not_background =
        one(r#"{ "label": "build", "command": "tsc", "problemMatcher": { "background": { "endsPattern": "done" } } }"#);
    assert_eq!(not_background.background, None);
}

#[test]
fn tasks_read__the_words_of_a_command_keep_their_quoting_and_the_shell_asked_for() {
    let task = one(r#"{ "label": "say", "type": "shell",
            "command": { "value": "echo", "quoting": "escape" },
            "args": ["plain words", { "value": "strong words", "quoting": "strong" }, { "value": "$HOME", "quoting": "weak" }],
            "options": { "shell": { "executable": "bash", "args": ["-lc"] } } }"#);
    let quoting: Vec<Quoting> = task
        .command
        .iter()
        .chain(&task.arguments)
        .map(|word| word.quoting)
        .collect();
    assert_eq!(
        quoting,
        [Quoting::Escape, Quoting::Plain, Quoting::Strong, Quoting::Weak]
    );
    assert_eq!(task.command_line(), "echo plain words strong words $HOME");
    assert_eq!(
        task.shell,
        Some(TaskShell {
            executable: String::from("bash"),
            arguments: Some(vec![String::from("-lc")]),
        })
    );
}

#[test]
fn tasks_read__the_options_of_the_file_hold_for_every_task_that_does_not_say_otherwise() {
    let text = r#"{
        "options": { "cwd": "rust", "env": { "RUSTFLAGS": "-D warnings", "MODE": "all" } },
        "windows": { "options": { "shell": { "executable": "pwsh.exe" } } },
        "tasks": [
            { "label": "build", "type": "shell", "command": "cargo build" },
            { "label": "docs", "type": "shell", "command": "mdbook build", "options": { "cwd": "docs", "env": { "MODE": "docs" } } }
        ]
    }"#;
    let tasks = read(TaskSystem::Windows, text);
    let [build, docs] = tasks.workspace.as_slice() else {
        panic!("two tasks");
    };
    assert_eq!(build.folder, folder().join("rust"));
    assert_eq!(
        build.shell.as_ref().map(|shell| shell.executable.as_str()),
        Some("pwsh.exe")
    );
    assert_eq!(docs.folder, folder().join("docs"));
    assert_eq!(
        docs.environment,
        [
            (String::from("RUSTFLAGS"), String::from("-D warnings")),
            (String::from("MODE"), String::from("docs"))
        ]
    );
    let on_linux = read(TaskSystem::Linux, text);
    assert_eq!(on_linux.workspace.first().unwrap().shell, None);
}

#[test]
fn tasks_read__a_task_without_a_label_is_named_by_its_older_name_or_its_command() {
    let text = r#"{ "tasks": [
        { "taskName": "old", "command": "make" },
        { "type": "shell", "command": "make test" },
        { "type": "shell" },
        { "label": "", "command": "named by its command" }
    ] }"#;
    let tasks = read(TaskSystem::Linux, text);
    assert_eq!(names(&tasks.workspace), ["old", "make test", "named by its command"]);
    assert_eq!(
        tasks.workspace.first().unwrap().kind,
        TaskKind::Process,
        "as VS Code takes a task with no type"
    );
}

#[test]
fn tasks_read__a_task_is_found_by_how_a_group_names_it_and_by_the_name_vs_code_gives_it() {
    let files = [
        ("extension/package.json", EXTENSION_PACKAGE),
        ("package.json", EXTENSION_PACKAGE),
    ];
    let tasks = read_tasks(&with_packages(
        context(TaskSystem::Linux, Some(TWO_SHELL_TASKS)),
        &files,
        &[],
    ));
    assert_eq!(tasks.all().count(), 6);
    let found = |reference: TaskReference| tasks.find(&reference).map(Task::command_line);
    assert_eq!(found(TaskReference::workspace("build")).as_deref(), Some("cargo build"));
    assert_eq!(found(TaskReference::npm("", "build")).as_deref(), Some("npm run build"));
    assert_eq!(found(TaskReference::npm("docs", "build")), None);
    let named = |name: &str| tasks.named_in_vs_code(name).map(|task| task.reference.clone());
    assert_eq!(
        named("build"),
        Some(TaskReference::workspace("build")),
        "the one of tasks.json first"
    );
    assert_eq!(named("test"), Some(TaskReference::npm("", "test")));
    assert_eq!(named("test - extension"), Some(TaskReference::npm("extension", "test")));
    assert_eq!(named("lint"), None);
}
