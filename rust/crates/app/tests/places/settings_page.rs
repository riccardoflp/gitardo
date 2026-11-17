//! The settings page in sections (`settings.sections`, `settings.edit`): the column of the sections, the list under
//! their names and the button of the section shown marked as the list scrolls, on a repository made for each test.

use app::store::State;
use platform::accesskit::Role;
use render::{Rect, ThemeName};
use views::Intent;
use views::page::Page;
use views::settings::{SettingsAnchor, SettingsIntent};
use views::text::settings::SettingsSection;

use super::pointer::{apply, bounds, middle, wheel};
use super::support::{Window, find, says};
use super::{repository_with_history, showing};

/// The size of the window of most tests.
const SIZE: (u32, u32) = (1280, 720);

/// A window showing a repository, with the settings page open.
fn settings_page() -> (State, testkit::Repository) {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    apply(&mut state, vec![Intent::ShowPage(Page::Settings)]);
    (state, repository)
}

/// The rectangle of the list of the settings: under the bar of the page, right of the column of the sections.
fn list_top(window: &mut Window, state: &State) -> f32 {
    let (_, tree) = window.draw(state, &[]);
    bounds(&tree, Role::Heading, "General").y.min(first_heading_top(&tree))
}

/// The top of the highest name of a section drawn.
fn first_heading_top(tree: &platform::accesskit::TreeUpdate) -> f32 {
    SettingsSection::ALL
        .iter()
        .filter_map(|section| {
            tree.nodes
                .iter()
                .find(|(_, node)| node.role() == Role::Heading && node.label() == Some(section.name()))
                .and_then(|(_, node)| node.bounds())
        })
        .map(|found| super::pointer::rect(found).y)
        .fold(f32::MAX, f32::min)
}

/// Whether the button of a section is marked; the buttons of the sections are the ones that say whether they are.
fn is_marked(tree: &platform::accesskit::TreeUpdate, section: SettingsSection) -> bool {
    let button = tree.nodes.iter().find(|(_, node)| {
        node.role() == Role::Button && node.label() == Some(section.name()) && node.is_selected().is_some()
    });
    button.unwrap().1.is_selected() == Some(true)
}

#[test]
fn settings_sections__the_page_opens_with_general_marked_and_its_first_setting_at_the_top_of_the_list() {
    let (state, _repository) = settings_page();
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(is_marked(&tree, SettingsSection::General));
    assert!(!is_marked(&tree, SettingsSection::Timeline));
    let general = bounds(&tree, Role::Heading, "General");
    let first = super::pointer::said_at(
        &tree,
        "Show commands before they run",
        Rect::new(0.0, 0.0, 4_000.0, 4_000.0),
    );
    assert!(
        first.y > general.y && first.y - general.bottom() < 30.0,
        "{general:?} {first:?}"
    );
    let column = bounds(&tree, Role::Button, "General");
    assert!(
        general.y - column.y < 12.0,
        "the list starts at the top of the column: {general:?} {column:?}"
    );
}

#[test]
fn settings_sections__a_click_on_worktrees_scrolls_the_list_until_worktrees_is_at_its_top_and_marks_it() {
    let (mut state, _repository) = settings_page();
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let top = list_top(&mut window, &state);
    let (_, tree) = window.draw(&state, &[]);
    // The button of the section, not the heading of the worktrees of the branch rail beside the page.
    let (_, button) = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button && node.label() == Some("Worktrees") && node.is_selected().is_some()
        })
        .unwrap();
    let intents = super::pointer::click_at(
        &mut window,
        &state,
        middle(super::pointer::rect(button.bounds().unwrap())),
        1,
    );
    assert_eq!(
        intents,
        vec![Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Section(
            SettingsSection::Worktrees
        )))]
    );
    apply(&mut state, intents);
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    let heading = bounds(&tree, Role::Heading, "Worktrees");
    assert!((heading.y - top).abs() < 1.0, "{heading:?} at {top}");
    assert!(is_marked(&tree, SettingsSection::Worktrees));
    assert!(!is_marked(&tree, SettingsSection::General));
}

#[test]
fn settings_sections__scrolling_the_list_from_timeline_into_branches_marks_branches() {
    let (mut state, _repository) = settings_page();
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let timeline = Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Section(
        SettingsSection::Timeline,
    )));
    apply(&mut state, vec![timeline]);
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(is_marked(&tree, SettingsSection::Timeline));
    let heading = bounds(&tree, Role::Heading, "Timeline");
    let over_list = middle(Rect::new(heading.x, heading.y, heading.width, 200.0));
    let mut turns = 0;
    loop {
        let _ = wheel(&mut window, &state, over_list, 1.0);
        let (_, tree) = window.draw(&state, &[]);
        turns += 1;
        if is_marked(&tree, SettingsSection::Branches) {
            let branches = bounds(&tree, Role::Heading, "Branches");
            assert!(
                branches.y <= heading.y + 0.5,
                "its name was scrolled past the top: {branches:?}"
            );
            assert!(!is_marked(&tree, SettingsSection::Timeline));
            break;
        }
        assert!(
            is_marked(&tree, SettingsSection::Timeline),
            "until then, Timeline is marked"
        );
        assert!(turns < 100, "the list reaches Branches");
    }
}

#[test]
fn settings_sections__in_a_page_600_pixels_wide_the_menu_at_the_top_reaches_every_section() {
    let (mut state, _repository) = settings_page();
    let mut window = Window::new(ThemeName::Dark, (680, 600));
    let (_, tree) = window.draw(&state, &[]);
    let page = bounds(&tree, Role::Region, "Settings");
    assert!(page.width < 640.0, "{page:?}");
    let intents = window.click(&state, Role::Button, "General");
    assert_eq!(intents, Vec::new());
    let (_, tree) = window.draw(&state, &[]);
    for section in SettingsSection::ALL {
        let _entry = find(&tree, Role::MenuItem, section.name());
    }
    let chosen = window.click(&state, Role::MenuItem, "Terminal and tools");
    let reveal = Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Section(
        SettingsSection::TerminalAndTools,
    )));
    assert_eq!(chosen, vec![reveal.clone()]);
    apply(&mut state, chosen);
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    let _named = find(&tree, Role::Button, "Terminal and tools");
}

#[test]
fn settings_sections__branches_follow_graph_filter_is_in_no_section() {
    let (state, _repository) = settings_page();
    let mut window = Window::new(ThemeName::Dark, (1280, 12_000));
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "branches.sortOrder"), "the whole list is drawn");
    assert!(!says(&tree, "branches.followGraphFilter"));
    let info = views::text::settings::settings().find(|info| info.key == "branches.followGraphFilter");
    assert_eq!(info.map(|info| info.section), Some(None));
}

#[test]
fn settings_edit__in_a_window_1600_pixels_wide_the_page_is_960_wide_and_as_far_from_either_side() {
    let (state, _repository) = settings_page();
    let mut window = Window::new(ThemeName::Dark, (1600, 900));
    let (_, tree) = window.draw(&state, &[]);
    let page = bounds(&tree, Role::Region, "Settings");
    assert!((page.width - 960.0).abs() < 0.5, "{page:?}");
    let content = bounds(&tree, Role::Navigation, "Trail");
    let left = page.x - content.x;
    let right = content.right() - page.right();
    assert!(
        (left - right).abs() < 1.0,
        "{left} on the left and {right} on the right of {content:?}"
    );
}

#[test]
fn settings_edit__each_row_shows_the_title_of_its_setting_its_description_whole_and_its_key() {
    let (mut state, _repository) = settings_page();
    apply(
        &mut state,
        vec![Intent::Settings(SettingsIntent::Reveal(SettingsAnchor::Setting(
            "worktrees.setupCommands",
        )))],
    );
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let _ = window.draw(&state, &[]);
    let (_, tree) = window.draw(&state, &[]);
    let info = views::text::settings::settings()
        .find(|info| info.key == "worktrees.setupCommands")
        .unwrap();
    assert!(says(&tree, info.title));
    assert!(says(&tree, info.description), "{}", info.description);
    assert!(says(&tree, info.key));
    let description = super::pointer::said_at(&tree, info.description, Rect::new(0.0, 0.0, 4_000.0, 4_000.0));
    assert!(
        description.height > 30.0,
        "a description longer than a line wraps: {description:?}"
    );
}
