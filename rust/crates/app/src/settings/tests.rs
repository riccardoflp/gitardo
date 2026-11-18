#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use model::graph::PageSize;
use render::ThemeName;

use super::{ProblemReason, SettingProblem, Settings, SettingsFile, parse, read, repository_settings_file};

#[test]
fn settings_read__a_setting_the_repository_sets_comes_before_the_users_for_the_repository_settings() {
    let user = r#"{ "graph.pageSize": 300, "readOnStartup": true, "theme": "light" }"#;
    let repository = r#"{
        // Shared with everyone who works on the repository.
        "graph.pageSize": 200,
        "readOnStartup": false,
    }"#;
    let settings = parse(Some(user), Some(repository));
    assert_eq!(settings.page_size, PageSize::from_setting(Some(200.0)));
    assert!(!settings.read_on_startup);
    assert_eq!(settings.theme, ThemeName::Light);
    assert!(settings.problems.is_empty(), "{:?}", settings.problems);
}

#[test]
fn settings_read__editor_word_wrap_on_wraps_the_lines_of_the_file_page_and_off_cuts_them() {
    let wrap = |user: &str| parse(Some(user), None).word_wrap;
    assert_eq!(wrap(r#"{ "editor.wordWrap": "on" }"#), ui::LineWrap::Words);
    assert_eq!(wrap(r#"{ "editor.wordWrap": "off" }"#), ui::LineWrap::Cut);
    assert_eq!(wrap("{}"), ui::LineWrap::Cut);
}

#[test]
fn changes_open__a_click_shows_the_diff_unless_changes_open_on_click_says_file() {
    use model::changes::OpenOnClick;
    let on_click = |user: &str| parse(Some(user), None).open_on_click;
    assert_eq!(on_click("{}"), OpenOnClick::Diff);
    assert_eq!(Settings::default().open_on_click, OpenOnClick::Diff);
    assert_eq!(on_click(r#"{ "changes.openOnClick": "diff" }"#), OpenOnClick::Diff);
    assert_eq!(on_click(r#"{ "changes.openOnClick": "file" }"#), OpenOnClick::File);
    let unknown = parse(Some(r#"{ "changes.openOnClick": "editor" }"#), None);
    assert_eq!(unknown.open_on_click, OpenOnClick::Diff);
    assert!(
        unknown
            .problems
            .iter()
            .any(|problem| problem.key == "changes.openOnClick"),
        "{:?}",
        unknown.problems
    );
}

#[test]
fn file_view_file_icons__the_setting_file_icons_chooses_symbols_monochrome_or_simple() {
    let style = |user: &str| parse(Some(user), None).file_icons;
    assert_eq!(style("{}"), ui::FileIconStyle::Symbols);
    assert_eq!(style(r#"{ "fileIcons": "monochrome" }"#), ui::FileIconStyle::Monochrome);
    assert_eq!(style(r#"{ "fileIcons": "simple" }"#), ui::FileIconStyle::Simple);
    let unknown = parse(Some(r#"{ "fileIcons": "seti" }"#), None);
    assert_eq!(unknown.file_icons, ui::FileIconStyle::Symbols);
    assert!(
        unknown.problems.iter().any(|problem| problem.key == "fileIcons"),
        "{:?}",
        unknown.problems
    );
}

#[test]
fn settings_read__a_user_setting_the_repository_sets_is_not_taken_and_is_said() {
    let settings = parse(
        Some(r#"{ "theme": "dark" }"#),
        Some(r#"{ "theme": "light", "font.size": 20 }"#),
    );
    assert_eq!(settings.theme, ThemeName::Dark);
    assert!((settings.fonts.size - Settings::default().fonts.size).abs() < f32::EPSILON);
    let keys: Vec<(&str, &ProblemReason)> = settings
        .problems
        .iter()
        .map(|problem| (problem.key.as_str(), &problem.reason))
        .collect();
    assert_eq!(
        keys,
        [
            ("theme", &ProblemReason::UserOnly),
            ("font.size", &ProblemReason::UserOnly)
        ]
    );
}

#[test]
fn settings_read__a_value_of_the_wrong_type_is_ignored_and_the_problem_names_the_key_and_why() {
    let settings = parse(
        Some(r#"{ "graph.pageSize": "many", "theme": "purple", "font.size": 90, "font.ui": 12 }"#),
        None,
    );
    let defaults = Settings::default();
    assert_eq!(settings.page_size, defaults.page_size);
    assert_eq!(settings.theme, defaults.theme);
    assert_eq!(settings.fonts, defaults.fonts);
    let problem = |key: &str| settings.problems.iter().find(|problem| problem.key == key).cloned();
    assert_eq!(
        problem("graph.pageSize"),
        Some(SettingProblem {
            key: String::from("graph.pageSize"),
            file: SettingsFile::User,
            reason: ProblemReason::WrongType { expected: "a number" },
        })
    );
    assert!(matches!(
        problem("theme").map(|found| found.reason),
        Some(ProblemReason::NotAChoice { .. })
    ));
    assert!(matches!(
        problem("font.size").map(|found| found.reason),
        Some(ProblemReason::OutOfRange { .. })
    ));
    assert!(matches!(
        problem("font.ui").map(|found| found.reason),
        Some(ProblemReason::WrongType { .. })
    ));
}

#[test]
fn settings_read__a_file_that_is_not_json_sets_nothing_and_is_said() {
    let settings = parse(Some("{ theme: "), Some(r#"{ "readOnStartup": false }"#));
    assert!(!settings.read_on_startup);
    assert_eq!(settings.theme, ThemeName::System);
    assert!(matches!(
        settings.problems.as_slice(),
        [SettingProblem {
            file: SettingsFile::User,
            reason: ProblemReason::Unreadable(_),
            ..
        }]
    ));
}

#[test]
fn settings_read__files_that_are_not_there_give_the_defaults() {
    let folder = testkit::Repository::new();
    let settings = read(
        &folder.path().join("settings.json"),
        Some(&repository_settings_file(folder.path())),
    );
    assert_eq!(settings, Settings::default());
}

#[test]
fn settings_read__the_files_on_disk_are_read_the_repositorys_first() {
    let folder = testkit::Repository::new();
    let user = folder.path().join("user-settings.json");
    std::fs::write(&user, r#"{ "font.size": 15, "graph.pageSize": 400 }"#).unwrap();
    let repository = repository_settings_file(folder.path());
    std::fs::create_dir_all(repository.parent().unwrap()).unwrap();
    std::fs::write(&repository, r#"{ "graph.pageSize": 250 }"#).unwrap();
    let settings = read(&user, Some(&repository));
    assert!((settings.fonts.size - 15.0).abs() < f32::EPSILON);
    assert_eq!(settings.page_size, PageSize::from_setting(Some(250.0)));
}

#[test]
fn settings_read__the_settings_page_says_which_key_and_why_and_shows_the_default() {
    let settings = parse(Some(r#"{ "graph.pageSize": "many", "graph.firstParent": true }"#), None);
    let rows = super::setting_rows(&settings, views::settings::SettingsTarget::User);
    let row = |key: &str| rows.iter().find(|row| row.info.key == key).unwrap();
    let page_size = row("graph.pageSize");
    assert_eq!(page_size.value, "1000");
    assert_eq!(page_size.source, views::settings::SettingSource::Default);
    assert_eq!(
        page_size.problem.as_deref(),
        Some("User settings: not a number. The default is used.")
    );
    let first_parent = row("graph.firstParent");
    assert_eq!(first_parent.value, "true");
    assert_eq!(first_parent.source, views::settings::SettingSource::User);
    assert_eq!(first_parent.problem, None);
}
