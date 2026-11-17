//! The settings page opened at a section or a setting from Quick search (`settings.reveal`), and its tabs of the files
//! (`settings.edit`), on a repository made for each test.

use std::time::Duration;

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::{Rect, ThemeName};
use views::Intent;
use views::page::Page;
use views::search::SearchIntent;
use views::text::settings::{SettingsSection, settings};

use super::pointer::{apply, bounds, said_at};
use super::support::{Window, has, says_in, typed};
use super::{repository_with_history, showing};

/// The size of the window.
const SIZE: (u32, u32) = (1280, 720);

/// Everywhere in the window, for the text said anywhere.
const ANYWHERE: Rect = Rect::new(0.0, 0.0, 4_000.0, 4_000.0);

/// Draws the window twice, so that the page is laid out and scrolled where it was asked, and gives its tree.
fn settled(window: &mut Window, state: &State) -> TreeUpdate {
    let _ = window.draw(state, &[]);
    window.draw(state, &[]).1
}

/// Types in Quick search and chooses the entry of a name, as a click on it does.
fn choose_in_quick_search(window: &mut Window, state: &mut State, name: &str) {
    apply(state, vec![Intent::Search(SearchIntent::OpenQuick(String::new()))]);
    let _ = settled(window, state);
    let _ = window.draw(state, &[typed(name)]);
    let _ = settled(window, state);
    let chosen = window.click(state, Role::ListBoxOption, name);
    assert!(!chosen.is_empty(), "{name} is an entry of Quick search");
    apply(state, chosen);
}

/// The top of the list of the settings, which the name of the first section shown is at when the page opens.
fn list_top(window: &mut Window, state: &State) -> f32 {
    let tree = settled(window, state);
    bounds(&tree, Role::Heading, "General").y
}

/// Whether the row of a setting is marked, as it says to the screen reader.
fn is_row_marked(tree: &TreeUpdate, title: &str) -> bool {
    tree.nodes
        .iter()
        .any(|(_, node)| node.role() == Role::Group && node.label() == Some(title) && node.is_selected() == Some(true))
}

/// How many pages are open.
fn settings_pages(state: &State) -> usize {
    state.pages.open.iter().filter(|page| **page == Page::Settings).count()
}

#[test]
fn settings_reveal__open_settings_worktrees_opens_the_page_with_worktrees_at_the_top_of_the_list_and_marked() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    apply(&mut state, vec![Intent::ShowPage(Page::Settings)]);
    let top = list_top(&mut window, &state);
    apply(&mut state, vec![Intent::ShowTimeline]);
    choose_in_quick_search(&mut window, &mut state, "Open Settings: Worktrees");
    assert_eq!(
        state.pages.shown.and_then(|index| state.pages.open.get(index)),
        Some(&Page::Settings)
    );
    let tree = settled(&mut window, &state);
    let heading = bounds(&tree, Role::Heading, "Worktrees");
    assert!((heading.y - top).abs() < 1.0, "{heading:?} at {top}");
    let marked = tree.nodes.iter().find(|(_, node)| {
        node.role() == Role::Button && node.label() == Some("Worktrees") && node.is_selected().is_some()
    });
    assert_eq!(marked.unwrap().1.is_selected(), Some(true));
}

#[test]
fn settings_reveal__open_settings_commits_read_at_a_time_opens_the_page_with_it_at_the_top_marked_for_a_second() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    apply(&mut state, vec![Intent::ShowPage(Page::Settings)]);
    let top = list_top(&mut window, &state);
    apply(&mut state, vec![Intent::ShowTimeline]);
    choose_in_quick_search(&mut window, &mut state, "Open Settings: Commits read at a time");
    let tree = settled(&mut window, &state);
    let row = bounds(&tree, Role::Group, "Commits read at a time");
    assert!((row.y - top).abs() < 1.0, "{row:?} at {top}");
    assert!(says_in(&tree, "graph.pageSize"));
    assert!(is_row_marked(&tree, "Commits read at a time"));
    window.now += Duration::from_millis(1_100);
    let (_, tree) = window.draw(&state, &[]);
    assert!(
        !is_row_marked(&tree, "Commits read at a time"),
        "a second later, it is no longer marked"
    );
}

#[test]
fn settings_reveal__with_the_page_already_open_a_way_in_scrolls_it_there_and_no_second_page_opens() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    apply(&mut state, vec![Intent::ShowPage(Page::Settings)]);
    let top = list_top(&mut window, &state);
    choose_in_quick_search(&mut window, &mut state, "Open Settings: Branches");
    assert_eq!(settings_pages(&state), 1);
    let tree = settled(&mut window, &state);
    let heading = bounds(&tree, Role::Heading, "Branches");
    assert!((heading.y - top).abs() < 1.0, "{heading:?} at {top}");
}

#[test]
fn settings_reveal__the_last_setting_of_the_last_section_is_shown_whole_at_the_bottom_of_the_list() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let last_section = SettingsSection::ALL.last().copied();
    let last = settings().filter(|info| info.section == last_section).last().unwrap();
    choose_in_quick_search(&mut window, &mut state, &format!("Open Settings: {}", last.title));
    let tree = settled(&mut window, &state);
    let page = bounds(&tree, Role::Region, "Settings");
    let row = bounds(&tree, Role::Group, last.title);
    assert!(row.bottom() <= page.bottom() + 0.5, "{row:?} in {page:?}");
    let key = said_at(&tree, last.key, ANYWHERE);
    assert!(key.bottom() <= page.bottom(), "its key is shown: {key:?} in {page:?}");
    let title = said_at(&tree, last.title, ANYWHERE);
    assert!(title.y >= page.y, "its title is shown: {title:?}");
}

#[test]
fn settings_edit__with_no_repository_shown_repository_is_not_there() {
    let folder = testkit::Repository::new();
    let empty = folder.path().parent().unwrap().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let mut state = showing(empty);
    apply(&mut state, vec![Intent::ShowPage(Page::Settings)]);
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let tree = settled(&mut window, &state);
    assert!(has(&tree, Role::Tab, "User"));
    assert!(!has(&tree, Role::Tab, "Repository"));
    let (repository, _remote) = repository_with_history();
    let shown = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let mut with_repository = shown;
    apply(&mut with_repository, vec![Intent::ShowPage(Page::Settings)]);
    let tree = settled(&mut window, &with_repository);
    assert!(has(&tree, Role::Tab, "Repository"));
    let chosen = window.click(&with_repository, Role::Tab, "Repository");
    assert_eq!(
        chosen,
        vec![Intent::ChooseSettingsFile(views::settings::SettingsTarget::Repository)]
    );
    apply(&mut with_repository, chosen);
    let tree = settled(&mut window, &with_repository);
    assert!(
        !says_in(&tree, "graph.showStashes"),
        "Repository lists only the settings its file may set"
    );
    let _ = said_at(&tree, "readOnStartup", ANYWHERE);
}
