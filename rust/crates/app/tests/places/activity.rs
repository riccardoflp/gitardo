//! What runs, drawn: the activity indicator of the header (`activity.status-bar`), the Activity drawer
//! (`activity.what-is-running`), the Command log in it (`log.show`), a plan tracked while it runs (`activity.track`),
//! and the settings page (`settings.edit`), with their pictures.

use std::thread;
use std::time::{Duration, Instant};

use app::store::{Message, State, ViewData, reduce};
use engine::{Asker, CommandLog, Engine, FixedClock, LogLevel};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::contract::{
    Answer, Answers, Context, Danger, FeatureId, FeatureSettings, Outcome, Plan, Question, Step, Target, Text, Undo,
};
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use ui::{InputEvent, Key, KeyEvent, Modifiers, PointerButton, PointerButtonEvent, PointerMove};
use views::Intent;
use views::activity::{ActivityState, RunningTask, StatusBarState, status_bar_view, what_is_running_view};
use views::log::{LogLine, LogState, log_view};
use views::page::Page;

use super::support::{NOW, Window, find, has, run};
use super::{repository_with_history, showing};

/// The size of the window of the pictures.
const WIDE: (u32, u32) = (1000, 520);

/// Something running for some milliseconds.
fn task(title: &str, millis: u64, detail: Option<&str>) -> RunningTask {
    RunningTask {
        title: title.to_owned(),
        detail: detail.map(str::to_owned),
        elapsed: Duration::from_millis(millis),
    }
}

/// Whether the tree says some words, as the name of a widget or the words of a label.
fn says(tree: &TreeUpdate, words: &str) -> bool {
    tree.nodes
        .iter()
        .any(|(_, node)| node.label() == Some(words) || node.value() == Some(words))
}

/// The tree of a frame of the activity indicator, over the top right of the window.
fn status_bar(window: &mut Window, running: &[RunningTask]) -> TreeUpdate {
    let state = StatusBarState {
        running,
        identity: None,
        is_running: false,
        said: None,
    };
    window
        .frame(&[], |ui| {
            ui.clip(Rect::new(600.0, 10.0, 300.0, 28.0), |ui| status_bar_view(&state, ui))
        })
        .1
}

/// The tree and the intents of a frame of the drawer alone, over the whole window.
fn drawer(window: &mut Window, state: &ActivityState<'_>) -> (Vec<Intent>, TreeUpdate) {
    window.frame(&[], |ui| {
        let all = ui.window();
        ui.clip(all, |ui| what_is_running_view(state, ui))
    })
}

#[test]
fn activity_status_bar__a_read_of_0_3_s_shows_nothing_and_one_of_2_s_shows_from_0_5_s() {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = status_bar(&mut window, &[task("Reading app", 300, None)]);
    assert!(!has(&tree, Role::Button, "Reading app (0 s)"));
    let tree = status_bar(&mut window, &[task("Reading app", 600, None)]);
    let _shown = find(&tree, Role::Button, "Reading app (0 s)");
    let tree = status_bar(&mut window, &[task("Reading app", 2_000, None)]);
    let _shown = find(&tree, Role::Button, "Reading app (2 s)");
    let tree = status_bar(&mut window, &[]);
    assert!(!has(&tree, Role::Button, "Reading app (2 s)"));
}

#[test]
fn activity_status_bar__with_main_two_ahead_of_origin_the_header_names_main_without_how_far_and_sync_says_2() {
    let (mut repository, _remote) = repository_with_history();
    let _ahead = repository.commit("Show the drawer", &[("drawer.txt", "activity\n")]);
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let header: Vec<&str> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.bounds().is_some_and(|bounds| bounds.y1 <= 48.0))
        .filter_map(|(_, node)| node.label().or(node.value()))
        .collect();
    assert!(header.contains(&"main"), "{header:?}");
    assert!(
        !header
            .iter()
            .any(|said| said.contains("main \u{2191}") || said.contains("\u{2191}2 \u{2193}0")),
        "the branch of the header does not say how far it is: {header:?}"
    );
    let _ = find(&tree, Role::Button, "\u{2193}0  \u{2191}2   Sync");
}

#[test]
fn activity_what_is_running__opened_during_a_long_fetch_it_lists_it_and_its_time_grows() {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for (millis, said) in [(3_200, "3 s"), (4_100, "4 s")] {
        let running = [task("Fetching origin", millis, Some("Because Sync was clicked."))];
        let (_, tree) = drawer(
            &mut window,
            &ActivityState {
                running: &running,
                ..ActivityState::default()
            },
        );
        assert!(says(&tree, "Fetching origin"), "{}", "Fetching origin");
        assert!(says(&tree, said), "{}", said);
        assert!(
            says(&tree, "Because Sync was clicked."),
            "{}",
            "Because Sync was clicked."
        );
    }
}

#[test]
fn activity_what_is_running__when_the_fetch_ends_the_drawer_says_nothing_is_running_now() {
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let finished = [task("Fetching origin", 4_000, None)];
    let (_, tree) = drawer(
        &mut window,
        &ActivityState {
            running: &[],
            finished: &finished,
            tasks: None,
        },
    );
    assert!(says(&tree, "Nothing is running now"), "{}", "Nothing is running now");
    assert!(says(&tree, "Took 4 s"), "{}", "Took 4 s");
}

#[test]
fn activity_what_is_running__show_the_commands_gitardo_runs_shows_the_bottom_panel_at_the_command_log() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _opened = reduce(&mut state, Message::Intent(Intent::OpenActivity));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Show the commands Gitardo runs");
    assert_eq!(intents, [Intent::ShowCommandLog]);
    let _shown = reduce(&mut state, Message::Intent(Intent::ShowCommandLog));
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "No command has run yet"), "{}", "No command has run yet");
    let _panel = find(&tree, Role::Region, "Bottom panel");
}

#[test]
fn activity_what_is_running__a_running_and_a_finished_task_are_drawn_in_the_drawer() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _opened = reduce(&mut state, Message::Intent(Intent::OpenActivity));
    let running = vec![task(
        "Reading app",
        2_400,
        Some("Read 2 times in a row: something changed during each read. Because src/a.rs changed."),
    )];
    let finished = vec![task(
        "Looking for repositories in app",
        1_000,
        Some("Opening the workspace"),
    )];
    let data = ViewData::new(&state, running, NOW).with_finished(finished);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.frame(&[], |ui| views::view(&data.window_state(&state), ui));
    window.assert_snapshot("activity-drawer-dark");
}

/// Lines of the log, `14:02:0<n> git fetch --prune origin <n>`.
fn log_lines(count: usize) -> Vec<LogLine> {
    (0..count)
        .map(|index| LogLine {
            time: format!("14:02:0{}", index % 10),
            text: format!("git fetch --prune origin {index}"),
            is_failure: false,
        })
        .collect()
}

#[test]
fn log_show__the_lines_selected_are_copied_with_their_time() {
    let lines = log_lines(3);
    let state = LogState {
        lines: &lines,
        dropped: 0,
    };
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let area = Rect::new(20.0, 20.0, 600.0, 300.0);
    let (_, tree) = window.frame(&[], |ui| ui.clip(area, |ui| log_view(&state, ui)));
    let bounds = find(&tree, Role::ListBox, "Command log").bounds().unwrap();
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an f32 counts"
    )]
    let point = render::Point::new(bounds.x0 as f32 + 40.0, bounds.y0 as f32 + 6.0);
    let button = PointerButtonEvent {
        button: PointerButton::Primary,
        position: point,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let moved = InputEvent::PointerMoved(PointerMove {
        position: point,
        modifiers: Modifiers::NONE,
    });
    let clicks = [
        moved,
        InputEvent::PointerPressed(button),
        InputEvent::PointerReleased(button),
    ];
    let _ = window.frame(&clicks, |ui| ui.clip(area, |ui| log_view(&state, ui)));
    let copy = [InputEvent::KeyPressed(KeyEvent::press(
        Key::Character('c'),
        Modifiers::CONTROL,
    ))];
    let (_, _, output) = window.frame_output(&copy, |ui| ui.clip(area, |ui| log_view(&state, ui)));
    assert_eq!(output.clipboard.as_deref(), Some("14:02:00 git fetch --prune origin 0"));
}

#[test]
fn log_show__it_keeps_the_last_10_000_lines_and_says_how_many_were_dropped() {
    let mut log = CommandLog::new(LogLevel::Info);
    for index in 0..engine::log::MOST_LINES + 12 {
        log.record(
            Timestamp(0),
            &Step::Delete {
                path: std::path::PathBuf::from(index.to_string()),
            },
        );
    }
    assert_eq!(log.entries().count(), 10_000);
    let lines = log_lines(2);
    let state = LogState {
        lines: &lines,
        dropped: log.dropped(),
    };
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.frame(&[], |ui| {
        ui.clip(Rect::new(20.0, 20.0, 600.0, 300.0), |ui| log_view(&state, ui))
    });
    assert!(
        says(&tree, "12 older lines were dropped"),
        "{}",
        "12 older lines were dropped"
    );
}

/// An asker that answers nothing and confirms every plan.
struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// The push of `main` to `origin`.
fn push_main(context: &Context, _answers: &Answers) -> Outcome {
    Outcome::Run(Plan {
        title: Text::from("Pushing main to origin"),
        steps: vec![Step::git(context.repository.folder.clone(), ["push", "origin", "main"])],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from("A test plan.")),
        after: Vec::new(),
        on_failure: Vec::new(),
    })
}

#[test]
fn activity_track__a_plan_of_a_push_shows_in_the_activity_indicator_while_git_push_runs() {
    let (repository, _remote) = repository_with_history();
    let hook = repository.path().join(".git/hooks/pre-push");
    std::fs::write(&hook, "#!/bin/sh\nsleep 2\n").unwrap();
    // Git runs a hook on Linux only when it may be executed; on Windows it runs it as it is.
    #[cfg(unix)]
    std::fs::set_permissions(&hook, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let context = Context {
        repository: std::sync::Arc::new(
            read_state(
                &Git::new(repository.path()),
                &StateRequest {
                    folder: repository.path().to_path_buf(),
                },
            )
            .unwrap(),
        ),
        target: Target::Head,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let folder = repository.path().to_path_buf();
    let (sender, receiver) = std::sync::mpsc::channel();
    let pushing = thread::spawn(move || {
        let mut engine = Engine::new(
            Git::new(&folder),
            CommandLog::new(LogLevel::Info),
            Box::new(FixedClock(Timestamp(0))),
        );
        sender.send(engine.activities().clone()).unwrap();
        engine
            .run_feature(FeatureId("push"), push_main, &context, &mut RunsAll)
            .is_ok()
    });
    let activities = receiver.recv().unwrap();
    let started = Instant::now();
    let mut seen = None;
    while started.elapsed() < Duration::from_secs(10) && seen.is_none() {
        thread::sleep(Duration::from_millis(50));
        seen = activities
            .running()
            .into_iter()
            .find(|entry| entry.description.as_str() == "Pushing main to origin");
    }
    let entry = seen.expect("the push is among what runs");
    thread::sleep(Duration::from_millis(600));
    assert!(!activities.running().is_empty(), "git push still runs");
    let running = [RunningTask {
        title: entry.description.to_string(),
        detail: None,
        elapsed: Duration::from_millis(600),
    }];
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = status_bar(&mut window, &running);
    let _shown = find(&tree, Role::Button, "Pushing main to origin (0 s)");
    assert!(pushing.join().unwrap(), "the push ends well");
    assert_eq!(activities.running().len(), 0, "nothing runs once the push ended");
}

#[test]
fn settings_edit__a_setting_marked_experimental_says_so_and_the_page_is_drawn_in_each_theme() {
    let (repository, _remote) = repository_with_history();
    let mut state: State = showing(repository.path().to_path_buf());
    let _effects = run(&mut state, Message::Intent(Intent::ShowPage(Page::Settings)));
    for (theme, name) in [
        (ThemeName::Dark, "settings-dark"),
        (ThemeName::Light, "settings-light"),
        (ThemeName::HighContrastDark, "settings-high-contrast-dark"),
        (ThemeName::HighContrastLight, "settings-high-contrast-light"),
    ] {
        let mut window = Window::new(theme, WIDE);
        let (_, tree) = window.draw(&state, &[]);
        assert!(says(&tree, "commandPreview"), "{}", "commandPreview");
        let _edit = find(&tree, Role::Button, "Edit in settings.json");
        window.assert_snapshot(name);
    }
    let reveal = views::settings::SettingsIntent::Reveal(views::settings::SettingsAnchor::Setting("conflicts.ai"));
    let _effects = run(&mut state, Message::Intent(Intent::Settings(reveal)));
    let mut window = Window::new(ThemeName::Dark, (1000, 720));
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    let everywhere = ui::Rect::new(0.0, 0.0, 4_000.0, 4_000.0);
    let title = super::pointer::said_at(&tree, "Ask the AI about conflicts", everywhere);
    let badge = super::pointer::said_at(
        &tree,
        "Experimental",
        ui::Rect::new(0.0, title.y - 4.0, 4_000.0, title.height + 8.0),
    );
    assert!(badge.x > title.right(), "beside the title: {title:?} {badge:?}");
}

#[test]
fn settings_edit__changing_a_value_writes_only_that_key_and_keeps_the_comments() {
    let repository = testkit::Repository::new();
    let file = repository.path().parent().unwrap().join("edited-settings.json");
    let before =
        "{\n  // The look of the window.\n  \"theme\": \"dark\",\n  \"graph.firstParent\": false // for now\n}\n";
    std::fs::write(&file, before).unwrap();
    let writer = app::work::SettingWriter::new();
    writer.write(file.clone(), "graph.firstParent", json::Value::Bool(true));
    drop(writer);
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        after,
        before.replace("\"graph.firstParent\": false", "\"graph.firstParent\": true")
    );
}

#[test]
fn activity_status_bar__in_a_window_1000_pixels_wide_a_read_says_what_runs_and_the_identity_is_at_the_end() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let running = [task("Reading the history of app", 2_000, None)];
    let data = ViewData::new(&state, running.to_vec(), NOW);
    let mut window = Window::new(ThemeName::Dark, (1000, 520));
    let (_, tree) = window.frame(&[], |ui| views::view(&data.window_state(&state), ui));
    let indicator = find(&tree, Role::Button, "Reading the history of app (2 s)")
        .bounds()
        .unwrap();
    assert!(
        indicator.x1 - indicator.x0 > 100.0,
        "the indicator says what runs: {indicator:?}"
    );
    let identity = find(&tree, Role::Button, "Test Author <author@example.com>")
        .bounds()
        .unwrap();
    assert!(identity.x0 > indicator.x1, "the identity is after what runs");
    assert!(identity.y0 > 480.0, "both are in the status bar at the foot");
    window.assert_snapshot("header-running-dark");
    #[allow(clippy::cast_possible_truncation, reason = "the bounds of a window fit in an f32")]
    let center = ui::Point::new(
        f64::midpoint(identity.x0, identity.x1) as f32,
        f64::midpoint(identity.y0, identity.y1) as f32,
    );
    let moved = InputEvent::PointerMoved(PointerMove {
        position: center,
        modifiers: Modifiers::NONE,
    });
    let press = PointerButtonEvent {
        button: PointerButton::Primary,
        position: center,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let _ = window.frame(&[moved], |ui| views::view(&data.window_state(&state), ui));
    let (intents, _) = window.frame(
        &[InputEvent::PointerPressed(press), InputEvent::PointerReleased(press)],
        |ui| views::view(&data.window_state(&state), ui),
    );
    assert!(
        intents.contains(&Intent::Feature(views::feature::FeatureIntent::Run(
            FeatureId("identity.switch"),
            Target::Head
        ))),
        "{intents:?}"
    );
}
