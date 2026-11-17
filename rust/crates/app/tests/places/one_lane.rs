//! All commits on one line (`graph.one-lane`), turned on from the store, on a repository made for the test.

use app::settings::Settings;
use app::store::{Effect, Message};
use render::ThemeName;
use views::Intent;
use views::graph::{GraphIntent, TimelineSwitch};

use super::folds::merged_feature;
use super::support::{Window, run};
use super::timeline::{showing_with, written};

/// Whether effects read the repository, or stop a read of it.
fn reads(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::Start(_) | Effect::Cancel(_)))
}

#[test]
fn graph_one_lane__turning_it_on_writes_graph_one_lane_true_in_the_users_settings_and_asks_no_read() {
    let repository = merged_feature();
    let mut state = showing_with(repository.path().to_path_buf(), Settings::default());
    assert!(state.timeline.layout.lane_count > 1);
    let rows = state.timeline.rows.len();
    let switch = GraphIntent::Switch(TimelineSwitch::OneLane, true);
    let effects = run(&mut state, Message::Intent(Intent::Graph(switch)));
    assert_eq!(written(&effects), Some(("graph.oneLane", json::Value::Bool(true))));
    assert!(!reads(&effects), "{effects:?}");
    assert_eq!(state.timeline.rows.len(), rows);
    assert_eq!(state.timeline.layout.lane_count, 1);
    let file = state.user_settings_file.clone().unwrap();
    std::fs::write(&file, "{\n  \"graph.oneLane\": true\n}\n").unwrap();
    let read = app::settings::read(&file, None);
    let effects = run(&mut state, Message::SettingsRead(Box::new(read)));
    assert!(!reads(&effects), "the file read again asks no read either: {effects:?}");
    assert_eq!(state.timeline.layout.lane_count, 1);
    let mut window = Window::new(ThemeName::Dark, (1000, 520));
    let _ = window.draw(&state, &[]);
    window.assert_snapshot("timeline-one-lane-dark");
}
