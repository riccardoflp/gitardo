//! The way back in the title row of a page (`ui.page-trail`): the names of the pages it was reached through, each
//! going back to its page as it was left, at most four of them, and `Alt+Left`.

use app::store::State;
use model::CommitId;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use ui::{InputEvent, Key, KeyEvent, Modifiers, NamedKey};
use views::file_view::{FileTarget, FileViewIntent};
use views::page::Page;
use views::text::pages as words;
use views::{Content, Intent, TimelineState};

use super::beside::{click_words, three_files};
use super::inspector::{ask, select};
use super::pointer::{apply, bounds, rect};
use super::showing;
use super::support::Window;

/// A window wide enough for the inspector beside the timeline.
const WIDE: (u32, u32) = (1200, 700);

/// The names the title row says, from its left: its buttons and its labels but the button of the side column, the
/// `›` between them and the button that closes the page.
fn names(tree: &TreeUpdate) -> Vec<String> {
    let trail = bounds(tree, Role::Navigation, words::TRAIL);
    let mut found: Vec<(f64, String)> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() != Role::Navigation)
        .filter_map(|(_, node)| {
            let label = node.label().or(node.value())?;
            let place = rect(node.bounds()?);
            let is_across = place.x >= trail.x && place.right() <= trail.right() + 1.0;
            let is_inside = is_across && place.y >= trail.y && place.bottom() <= trail.bottom() + 1.0;
            is_inside.then(|| (f64::from(place.x), label.to_owned()))
        })
        .filter(|(_, label)| ![words::TRAIL_SEPARATOR, words::CLOSE_PAGE].contains(&label.as_str()))
        .filter(|(_, label)| !label.ends_with("the repositories and the branches"))
        .collect();
    found.sort_by(|first, second| first.0.total_cmp(&second.0));
    found.dedup_by(|second, first| first.1 == second.1);
    found.into_iter().map(|(_, label)| label).collect()
}

/// The Diff page of `src/a.rs` of the last commit beside the inspector, then the File page `F4` asks for from it: the
/// file at that commit.
fn file_from_diff() -> (testkit::Repository, State, Window, CommitId) {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    let commit = select(&mut state, "Change three files");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    click_words(&mut window, &mut state, "a.rs");
    let target = FileTarget {
        path: "src/a.rs".to_owned(),
        revision: Some(commit.clone()),
        line: Some(1),
    };
    ask(&mut state, Intent::FileView(FileViewIntent::Open(target)));
    (repository, state, window, commit)
}

/// The name of the Diff page shown beside the inspector, as its own title row says it.
fn diff_name(state: &State) -> String {
    let Some(Page::Diff(id)) = state.pages.way.first().copied() else {
        panic!("the way starts at the Diff page: {:?}", state.pages.way);
    };
    state
        .diff_pages
        .pages
        .iter()
        .find(|page| page.id == id)
        .unwrap()
        .title
        .clone()
}

/// Whether the Diff page beside the inspector is shown, with the file it shows.
fn beside_file(window: &mut Window, state: &State) -> Option<String> {
    let data = app::store::ViewData::new(state, Vec::new(), super::support::NOW);
    let shown = match data.window_state(state).content {
        Content::Timeline(TimelineState::Repository {
            beside: Some(beside), ..
        }) => Some(beside.host.page.title()),
        _ => None,
    };
    let _ = window.draw(state, &[]);
    shown.map(str::to_owned)
}

#[test]
fn ui_page_trail__a_file_page_opened_from_the_diff_page_of_a_file_of_a_commit_names_it_and_its_name_goes_back_to_it() {
    let (_repository, mut state, mut window, commit) = file_from_diff();
    let diff = diff_name(&state);
    let (_, tree) = window.draw(&state, &[]);
    let file = views::text::file_view::at_revision("a.rs", commit.short());
    assert_eq!(names(&tree), [words::TIMELINE, diff.as_str(), file.as_str()]);
    let intents = window.click(&state, Role::Button, &diff);
    assert!(
        matches!(intents.as_slice(), [Intent::BackTo(Page::Diff(_))]),
        "{intents:?}"
    );
    apply(&mut state, intents);
    assert_eq!(
        beside_file(&mut window, &state).as_deref(),
        Some(diff.as_str()),
        "as it was left"
    );
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        names(&tree),
        [words::TIMELINE, diff.as_str()],
        "what came after it is dropped"
    );
}

#[test]
fn ui_page_trail__alt_left_goes_back_one_name() {
    let (_repository, mut state, mut window, _commit) = file_from_diff();
    let diff = diff_name(&state);
    let alt_left = InputEvent::KeyPressed(KeyEvent::press(Key::Named(NamedKey::ArrowLeft), Modifiers::ALT));
    let (intents, _) = window.draw(&state, std::slice::from_ref(&alt_left));
    assert!(
        matches!(intents.as_slice(), [Intent::BackTo(Page::Diff(_))]),
        "{intents:?}"
    );
    apply(&mut state, intents);
    assert_eq!(beside_file(&mut window, &state).as_deref(), Some(diff.as_str()));
    let (intents, _) = window.draw(&state, &[alt_left]);
    assert_eq!(intents, [Intent::ShowTimeline]);
}

#[test]
fn ui_page_trail__a_page_of_the_bar_of_views_or_of_a_key_starts_the_way_again_from_the_timeline() {
    let (_repository, mut state, mut window, _commit) = file_from_diff();
    ask(&mut state, Intent::ShowPage(Page::Settings));
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(names(&tree), [words::TIMELINE, views::text::settings::PAGE]);
    // `Ctrl+Tab` from the last page shows the timeline, then the first page open: the Diff page, alone on its way.
    ask(&mut state, Intent::NextPage);
    assert_eq!(state.page_shown(), None);
    ask(&mut state, Intent::NextPage);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(names(&tree), [words::TIMELINE, diff_name(&state).as_str()]);
}

#[test]
fn ui_page_trail__with_five_names_those_in_between_are_folded_into_an_ellipsis_whose_menu_lists_them() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    for page in [Page::Settings, Page::Changes, Page::Files, Page::Welcome] {
        ask(&mut state, Intent::ShowPage(page));
    }
    state.pages.way = vec![Page::Settings, Page::Changes, Page::Files, Page::Welcome];
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let files = views::text::files::PAGE;
    assert_eq!(
        names(&tree),
        [
            words::TIMELINE,
            words::FOLDED,
            views::text::changes::PAGE,
            files,
            words::WELCOME
        ]
    );
    let _ = window.click(&state, Role::Button, words::FOLDED);
    let (_, tree) = window.draw(&state, &[]);
    assert!(super::support::has(&tree, Role::MenuItem, views::text::settings::PAGE));
    let point = super::pointer::middle(bounds(&tree, Role::MenuItem, views::text::settings::PAGE));
    let intents = super::pointer::click_at(&mut window, &state, point, 1);
    assert_eq!(intents, [Intent::BackTo(Page::Settings)]);
    apply(&mut state, intents);
    assert_eq!(state.page_shown(), Some(Page::Settings));
    assert_eq!(state.pages.way, [Page::Settings]);
}

#[test]
fn the_title_row_of_a_page_reached_through_others_is_drawn_in_each_theme() {
    let repository = three_files();
    let mut state = showing(repository.path().to_path_buf());
    for page in [Page::Welcome, Page::Changes, Page::Files, Page::Settings] {
        ask(&mut state, Intent::ShowPage(page));
    }
    state.pages.way = vec![Page::Welcome, Page::Changes, Page::Files, Page::Settings];
    for (theme, name) in [
        (ThemeName::Dark, "page-trail-dark"),
        (ThemeName::Light, "page-trail-light"),
        (ThemeName::HighContrastDark, "page-trail-high-contrast-dark"),
        (ThemeName::HighContrastLight, "page-trail-high-contrast-light"),
    ] {
        let mut window = Window::new(theme, (1000, 360));
        let _ = window.draw(&state, &[]);
        window.assert_snapshot(name);
    }
}
