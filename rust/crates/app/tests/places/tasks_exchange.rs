//! The groups of tasks imported and exported from the window (`tasks.import-groups`, `tasks.export-groups`): **Import
//! groups…** and **Export groups…** in the menu of **Groups** in the Tasks tab and in Quick search, the file chosen
//! as the dialog of the system would, a file written as Run My Tasks 0.8.0 writes it, and one of another kind.

use std::path::PathBuf;

use app::store::State;
use model::contract::Answer;
use model::tasks::TaskGroup;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::panel::PanelTab;
use views::search::{QuickChoice, QuickCommand, SearchIntent};

use super::support::{Window, has};
use super::tasks_groups::{group, listed, write_groups};
use super::tasks_tab::{Runner, WIDE, shell_task, with_tasks};

/// A file as **Export Groups...** of Run My Tasks 0.8.0 writes it: two groups, the second with a task these
/// repositories do not have and a launch configuration.
const OF_RUN_MY_TASKS: &str = r#"{
  "version": 1,
  "groups": [
    {
      "name": "Servers",
      "tasks": [
        "serve the docs",
        "watch"
      ],
      "launches": []
    },
    {
      "name": "Checks",
      "tasks": [
        "lint",
        "test"
      ],
      "launches": [
        "Debug the tests"
      ]
    }
  ]
}
"#;

/// A repository whose `tasks.json` has `serve the docs`, `watch` and `lint`, which print their names.
fn with_three_tasks() -> (Repository, State) {
    let task = |label: &str| shell_task(label, ("echo task", "echo task"), "");
    with_tasks(&[task("serve the docs"), task("watch"), task("lint")])
}

/// A file beside the git folder's own, where no test reads the working tree: its path, with some text written.
fn file(repository: &Repository, name: &str, text: Option<&str>) -> PathBuf {
    let path = repository.path().join(".git").join(name);
    if let Some(text) = text {
        std::fs::write(&path, text).unwrap();
    }
    path
}

/// Chooses a command of Quick search.
fn quick(runner: &Runner, state: &mut State, command: QuickCommand) {
    let chosen = Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(command)));
    runner.intents(state, vec![chosen]);
}

/// Chooses groups by their names in the question that lists them.
fn choose_groups(runner: &Runner, state: &mut State, names: &[&str]) {
    let names = names.iter().map(|name| (*name).to_owned()).collect();
    runner.answer(state, Answer::Choices(names));
}

/// What the operation that ended said.
fn said(runner: &Runner, state: &mut State) -> String {
    runner.until(state, "what is said", |state| state.operations.said.is_some());
    state.operations.said.clone().unwrap().text
}

/// The two groups of the file of Run My Tasks, as Gitardo keeps them.
fn both() -> [TaskGroup; 2] {
    [
        group("Servers", &["serve the docs", "watch"]),
        group("Checks", &["lint", "test"]),
    ]
}

#[test]
fn tasks_import_groups__import_groups_of_the_menu_of_groups_imports_the_two_groups_of_a_file_of_run_my_tasks() {
    let (repository, mut state) = with_three_tasks();
    let exported = file(&repository, "exported.json", Some(OF_RUN_MY_TASKS));
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let _ = runner.draw(&mut state, &mut window, &[]);
    runner.click(&mut state, &mut window, Role::Button, "More of the groups");
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        has(&tree, Role::MenuItem, "Import groups…") && has(&tree, Role::MenuItem, "Export groups…"),
        "the menu of Groups has both"
    );
    runner.click(&mut state, &mut window, Role::MenuItem, "Import groups…");
    runner.choose_file(&mut state, "Import groups", &exported);
    choose_groups(&runner, &mut state, &["Servers", "Checks"]);
    runner.until(&mut state, "the groups imported", |state| listed(state) == both());
    let tree = runner.draw(&mut state, &mut window, &[]);
    assert!(
        has(&tree, Role::Button, "Servers") && has(&tree, Role::Button, "Checks"),
        "the groups imported are listed under Groups"
    );
}

#[test]
fn tasks_import_groups__merge_from_quick_search_adds_the_tasks_a_group_of_the_repository_lacks_and_shows_the_tab() {
    let (repository, mut state) = with_three_tasks();
    write_groups(&repository, &[group("Checks", &["test", "format"])]);
    let exported = file(&repository, "exported.json", Some(OF_RUN_MY_TASKS));
    let runner = Runner::new();
    // The Tasks tab was never shown: the import shows it and waits for the tasks to be read.
    quick(&runner, &mut state, QuickCommand::ImportGroups);
    assert!(state.panel.look.is_shown && state.panel.look.tab == PanelTab::Tasks);
    runner.choose_file(&mut state, "Import groups", &exported);
    choose_groups(&runner, &mut state, &["Checks"]);
    runner.choose(&mut state, "merge");
    let merged = [group("Checks", &["test", "format", "lint"])];
    runner.until(&mut state, "the group merged", |state| listed(state) == merged);
}

#[test]
fn tasks_import_groups__a_group_with_a_launch_configuration_is_imported_without_it_and_one_is_said_left_out() {
    let (repository, mut state) = with_three_tasks();
    let exported = file(&repository, "exported.json", Some(OF_RUN_MY_TASKS));
    let runner = Runner::new();
    runner.open(&mut state);
    quick(&runner, &mut state, QuickCommand::ImportGroups);
    runner.choose_file(&mut state, "Import groups", &exported);
    choose_groups(&runner, &mut state, &["Checks"]);
    assert_eq!(
        said(&runner, &mut state),
        "1 group was imported. This worktree does not have `test`. 1 launch configuration was left out, since \
         Gitardo has no debugger."
    );
    runner.until(&mut state, "the group imported", |state| {
        listed(state) == [group("Checks", &["lint", "test"])]
    });
}

#[test]
fn tasks_import_groups__a_json_file_of_another_kind_imports_nothing_and_says_it_is_not_a_file_of_groups() {
    let (repository, mut state) = with_three_tasks();
    write_groups(&repository, &[group("Docs", &[])]);
    let package = file(
        &repository,
        "package.json",
        Some("{ \"name\": \"app\", \"scripts\": {} }"),
    );
    let runner = Runner::new();
    runner.open(&mut state);
    quick(&runner, &mut state, QuickCommand::ImportGroups);
    runner.choose_file(&mut state, "Import groups", &package);
    assert_eq!(
        said(&runner, &mut state),
        "`package.json` is not a file of groups: nothing was imported."
    );
    runner.ended(&mut state);
    assert_eq!(listed(&state), [group("Docs", &[])]);
}

#[test]
fn tasks_export_groups__two_groups_exported_and_imported_in_another_repository_give_the_same_groups_in_their_order() {
    let (repository, mut state) = with_three_tasks();
    write_groups(&repository, &both());
    let saved = file(&repository, "task-groups.json", None);
    let runner = Runner::new();
    runner.open(&mut state);
    quick(&runner, &mut state, QuickCommand::ExportGroups);
    choose_groups(&runner, &mut state, &["Servers", "Checks"]);
    runner.choose_file(&mut state, "Export groups", &saved);
    assert_eq!(
        said(&runner, &mut state),
        format!("2 groups were saved in `{}`.", saved.display())
    );
    let (_other, mut another) = with_three_tasks();
    let runner = Runner::new();
    runner.open(&mut another);
    assert!(listed(&another).is_empty(), "the other repository has no groups yet");
    quick(&runner, &mut another, QuickCommand::ImportGroups);
    runner.choose_file(&mut another, "Import groups", &saved);
    choose_groups(&runner, &mut another, &["Servers", "Checks"]);
    runner.until(&mut another, "the groups imported", |state| listed(state) == both());
}

#[test]
fn tasks_export_groups__the_file_exported_from_the_menu_of_groups_is_the_one_run_my_tasks_writes_for_its_groups() {
    let (repository, mut state) = with_three_tasks();
    write_groups(&repository, &both());
    let saved = file(&repository, "task-groups.json", None);
    let runner = Runner::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    runner.open(&mut state);
    let _ = runner.draw(&mut state, &mut window, &[]);
    runner.click(&mut state, &mut window, Role::Button, "More of the groups");
    runner.click(&mut state, &mut window, Role::MenuItem, "Export groups…");
    choose_groups(&runner, &mut state, &["Servers", "Checks"]);
    runner.choose_file(&mut state, "Export groups", &saved);
    runner.ended(&mut state);
    // What Run My Tasks 0.8.0 writes for the same groups, without the launch configuration Gitardo does not keep,
    // whatever each line is indented by.
    let lines = |text: &str| -> Vec<String> { text.lines().map(|line| line.trim_start().to_owned()).collect() };
    let of_run_my_tasks = OF_RUN_MY_TASKS.replace(
        "[
        \"Debug the tests\"
      ]",
        "[]",
    );
    assert_eq!(lines(&std::fs::read_to_string(saved).unwrap()), lines(&of_run_my_tasks));
}
