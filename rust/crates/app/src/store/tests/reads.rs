//! The reads of the repository shown that changes ask for (`repo.watch-changes`, `repo.schedule-reads`), the window
//! opened with `readOnStartup` off (`repo.read-state`), the Command log of the drawer (`log.show`) and the settings
//! page (`settings.edit`, `settings.theme`).

use std::path::PathBuf;
use std::time::Duration;

use model::contract::{Answer, Answers, FeatureId, Target};
use model::repo::{Change, ReadCause};
use views::feature::FeatureIntent;
use views::settings::{SettingChange, SettingValue, SettingsTarget};
use views::{Content, Intent, TimelineState};

use super::{answer, folder, intent, open, reads, view_data, window};
use crate::store::{Effect, Message, State, reduce};

/// A window showing the repository `/work/app`, read.
fn showing() -> (State, PathBuf) {
    let mut state = window();
    let app = folder("app");
    let _opened = open(&mut state, &app, &[(&app, std::slice::from_ref(&app))]);
    assert!(state.shown_read().is_some());
    (state, app)
}

/// A change of a file of the working tree at a moment, in milliseconds since the window opened.
fn file_changed(state: &mut State, path: &str, at: u64) -> Vec<Effect> {
    reduce(
        state,
        Message::Changed {
            changes: vec![Change::WorkingTree(PathBuf::from(path))],
            at: Duration::from_millis(at),
        },
    )
}

/// The moment the effects ask to be woken at.
fn wake_at(effects: &[Effect]) -> Option<Duration> {
    effects.iter().find_map(|effect| match effect {
        Effect::WakeAt(at) => Some(*at),
        _ => None,
    })
}

#[test]
fn repo_schedule_reads__ten_files_written_within_half_a_second_lead_the_window_to_one_read_a_second_after() {
    let (mut state, app) = showing();
    let mut asked = Vec::new();
    for index in 0..10 {
        let effects = file_changed(&mut state, &format!("src/{index}.rs"), 1_000 + index * 50);
        assert_eq!(reads(&effects), [] as [PathBuf; 0]);
        asked.extend(wake_at(&effects));
    }
    assert!(asked.iter().all(|at| *at == Duration::from_millis(2_000)), "{asked:?}");
    let early = reduce(
        &mut state,
        Message::Wake {
            at: Duration::from_millis(1_900),
        },
    );
    assert_eq!(reads(&early), [] as [PathBuf; 0]);
    let effects = reduce(
        &mut state,
        Message::Wake {
            at: Duration::from_millis(2_000),
        },
    );
    assert_eq!(reads(&effects), [app]);
    let after = answer(&mut state, effects, &[]);
    assert_eq!(reads(&after).len(), 1, "one read, and none after it");
}

#[test]
fn repo_schedule_reads__a_change_during_a_read_is_followed_by_exactly_one_more_read() {
    let (mut state, app) = showing();
    let _waits = file_changed(&mut state, "src/a.rs", 0);
    let first = reduce(
        &mut state,
        Message::Wake {
            at: Duration::from_secs(1),
        },
    );
    assert_eq!(reads(&first), std::slice::from_ref(&app));
    let job = super::started(&first)[0].0;
    assert_eq!(
        state.reads.reason(job).map(|read| read.cause.clone()),
        Some(ReadCause::File(PathBuf::from("src/a.rs")))
    );
    let _during = file_changed(&mut state, "src/b.rs", 1_100);
    let _during = file_changed(&mut state, "src/c.rs", 1_200);
    let noted = reduce(
        &mut state,
        Message::Wake {
            at: Duration::from_millis(2_100),
        },
    );
    assert_eq!(reads(&noted), [] as [PathBuf; 0], "never two reads at once");
    let all = answer(&mut state, first, &[]);
    assert_eq!(reads(&all), [app.clone(), app], "the read, then one more");
    assert!(!state.reads.schedule.is_reading());
}

#[test]
fn repo_schedule_reads__refresh_during_a_read_of_the_window_leads_to_exactly_one_more_read_after_it() {
    let (mut state, app) = showing();
    let _waits = file_changed(&mut state, "src/a.rs", 0);
    let first = reduce(
        &mut state,
        Message::Wake {
            at: Duration::from_secs(1),
        },
    );
    assert_eq!(reads(&first), std::slice::from_ref(&app));
    let refreshed = intent(&mut state, Intent::Refresh);
    let again = intent(&mut state, Intent::Refresh);
    assert_eq!(super::started(&refreshed), [], "the read under way goes on");
    assert_eq!(super::started(&again), []);
    let all = answer(&mut state, first, &[]);
    assert_eq!(reads(&all), [app.clone(), app], "the read, then one more");
}

#[test]
fn repo_watch_changes__fsmonitor_cookies_and_locks_ask_the_window_for_nothing() {
    let (mut state, _app) = showing();
    let changes = vec![
        Change::GitFolder(PathBuf::from("worktrees/feature/fsmonitor--daemon/cookies/1-2")),
        Change::GitFolder(PathBuf::from("index.lock")),
        Change::WorkingTree(PathBuf::from(".git/index")),
    ];
    let effects = reduce(
        &mut state,
        Message::Changed {
            changes,
            at: Duration::from_secs(1),
        },
    );
    assert_eq!(effects, []);
}

#[test]
fn repo_watch_changes__the_repository_read_is_watched_and_one_with_none_shown_is_not() {
    let mut state = window();
    let app = folder("app");
    let effects = open(&mut state, &app, &[(&app, std::slice::from_ref(&app))]);
    assert!(effects.contains(&Effect::WatchRepository(Some(app.clone()))));
    let none = folder("none");
    let _asked = open(&mut state, &none, &[]);
    let effects = super::act(&mut state, Intent::AnswerOpenFolder(views::FolderAnswer::Replace), &[]);
    assert!(effects.contains(&Effect::WatchRepository(None)));
}

#[test]
fn repo_read_state__with_read_on_startup_off_nothing_runs_until_read_the_repository_is_clicked() {
    let mut state = window();
    state.settings.read_on_startup = false;
    let app = folder("app");
    let effects = reduce(
        &mut state,
        Message::Started {
            folders: vec![app.clone()],
        },
    );
    assert_eq!(super::started(&effects), []);
    let data = view_data(&state);
    assert!(matches!(
        data.window_state(&state).content,
        Content::Timeline(TimelineState::NoRepository(views::repo::NoRepositoryState::NotRead))
    ));
    let effects = intent(&mut state, Intent::ReadRepository);
    let all = answer(&mut state, effects, &[(&app, std::slice::from_ref(&app))]);
    assert_eq!(reads(&all), [app]);
    assert!(state.shown_read().is_some());
}

#[test]
fn log_show__the_reads_at_debug_are_in_the_command_log_with_the_time_they_started() {
    let (mut state, _app) = showing();
    state.log.set_level(engine::LogLevel::Debug);
    let _logged = reduce(
        &mut state,
        Message::ReadTook {
            line: String::from("read /work/app: the state and 1 pages of commits"),
            took: Duration::from_millis(42),
            time: model::Timestamp(14 * 3_600 + 2 * 60 + 9),
        },
    );
    let _shown = intent(&mut state, Intent::ShowCommandLog);
    let data = view_data(&state);
    let window = data.window_state(&state);
    let log = window.panel.map(|panel| panel.log).unwrap();
    assert_eq!(log.lines.len(), 1);
    assert_eq!(log.lines[0].time, "14:02:09");
    assert_eq!(
        log.lines[0].text,
        "read /work/app: the state and 1 pages of commits (42 ms)"
    );
}

#[test]
fn settings_edit__a_value_changed_is_written_alone_in_the_file_chosen() {
    let (mut state, app) = showing();
    state.user_settings_file = Some(PathBuf::from("/config/settings.json"));
    let change = |target| {
        Intent::SetSetting(SettingChange {
            key: "graph.firstParent",
            value: SettingValue::Toggle(true),
            target,
        })
    };
    let effects = intent(&mut state, change(SettingsTarget::User));
    assert_eq!(
        effects,
        [Effect::WriteSetting {
            file: PathBuf::from("/config/settings.json"),
            key: "graph.firstParent",
            value: json::Value::Bool(true),
        }]
    );
    let _chosen = intent(&mut state, Intent::ChooseSettingsFile(SettingsTarget::Repository));
    let target = state.settings_target;
    let effects = intent(&mut state, change(target));
    assert_eq!(
        effects,
        [Effect::WriteSetting {
            file: crate::settings::repository_settings_file(&app),
            key: "graph.firstParent",
            value: json::Value::Bool(true),
        }]
    );
}

#[test]
fn settings_theme__the_theme_picked_runs_settings_theme_and_the_picker_closes() {
    let (mut state, _app) = showing();
    state.user_settings_file = Some(PathBuf::from("/config/settings.json"));
    let _opened = intent(&mut state, Intent::PickTheme);
    assert_eq!(state.dialog, Some(crate::store::Dialog::ThemePicker));
    let data = view_data(&state);
    assert_eq!(
        data.window_state(&state).theme_picker.map(|picker| picker.current),
        Some("system")
    );
    let answers = Answers::new().with(features::settings::THEME, Answer::Choice(String::from("light")));
    let picked = FeatureIntent::RunWith(FeatureId("settings.theme"), Target::Head, answers);
    let effects = intent(&mut state, Intent::Feature(picked));
    assert!(
        matches!(
            &effects[..],
            [Effect::RunFeature {
                feature: FeatureId("settings.theme"),
                ..
            }]
        ),
        "{effects:?}"
    );
    let _closed = intent(&mut state, Intent::CloseThemePicker);
    assert_eq!(state.dialog, None);
}
