//! The window of D21 drawn from the store: the islands on the ground (`graph.islands`), the bar of views
//! (`graph.view-bar`), **Search** in the header (`graph.search-button`) and the header with no box of text
//! (`graph.toolbar`).

use app::store::Message;
use platform::accesskit::{Node, Role, TreeUpdate};
use render::{Rect, ThemeName};
use ui::Modifiers;
use views::Intent;
use views::page::Page;
use views::search::SearchIntent;

use super::support::{Window, find, has, run, shortcut};
use super::{repository_with_history, showing, window};

/// The size of a wide window.
const WIDE: (u32, u32) = (1000, 520);
/// The size of a narrow window, where the gaps narrow and the header keeps its icons.
const NARROW: (u32, u32) = (820, 480);

/// The rectangle of a node, which the test fails without.
fn bounds(node: &Node) -> Rect {
    let bounds = node.bounds().unwrap();
    #[allow(clippy::cast_possible_truncation, reason = "the bounds of a window fit in an f32")]
    Rect::new(
        bounds.x0 as f32,
        bounds.y0 as f32,
        (bounds.x1 - bounds.x0) as f32,
        (bounds.y1 - bounds.y0) as f32,
    )
}

/// The color of a pixel of the window, as red, green and blue.
fn pixel(window: &Window, (x, y): (f32, f32)) -> [u8; 3] {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the points are inside the window"
    )]
    let (x, y) = (x as usize, y as usize);
    let width = window.frame.width() as usize;
    let pixels = window.frame.to_rgba();
    let at = (y * width + x) * 4;
    let [red, green, blue, _] = pixels.get(at..at + 4).unwrap() else {
        panic!("the point is inside the window")
    };
    [*red, *green, *blue]
}

/// How light a color is, as the sum of its channels.
fn lightness(color: [u8; 3]) -> u32 {
    color.iter().map(|&channel| u32::from(channel)).sum()
}

/// The ground of the dark theme.
const DARK_GROUND: [u8; 3] = [0x0E, 0x0F, 0x12];

#[test]
fn graph_islands__in_the_dark_theme_the_ground_between_the_rail_and_the_timeline_is_darker_than_both() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rail = bounds(find(&tree, Role::Navigation, "Branches"));
    let timeline = bounds(find(&tree, Role::Main, "Timeline"));
    let middle = rail.bottom() - 20.0;
    let ground = pixel(&window, (f32::midpoint(rail.right(), timeline.x), middle));
    assert_eq!(ground, DARK_GROUND);
    assert!(lightness(pixel(&window, (rail.x + 10.0, middle))) > lightness(ground));
    assert!(lightness(pixel(&window, (timeline.x + 10.0, middle))) > lightness(ground));
}

#[test]
fn graph_islands__the_corners_of_the_inspector_are_the_ground_of_the_window() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let inspector = bounds(find(&tree, Role::Complementary, "Selected commit"));
    assert_eq!(pixel(&window, (inspector.x, inspector.y)), DARK_GROUND);
    assert_eq!(
        pixel(&window, (inspector.right() - 1.0, inspector.bottom() - 1.0)),
        DARK_GROUND
    );
    let inside = pixel(&window, (inspector.x + 20.0, inspector.bottom() - 20.0));
    assert!(lightness(inside) > lightness(DARK_GROUND));
}

#[test]
fn graph_islands__in_a_window_narrower_than_900_pixels_the_gap_is_4_pixels() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let gap = |size| {
        let mut window = Window::new(ThemeName::Dark, size);
        let (_, tree) = window.draw(&state, &[]);
        let timeline = bounds(find(&tree, Role::Main, "Timeline"));
        let inspector = bounds(find(&tree, Role::Complementary, "Selected commit"));
        inspector.x - timeline.right()
    };
    assert!((gap(WIDE) - 8.0).abs() < 0.5, "{}", gap(WIDE));
    assert!((gap(NARROW) - 4.0).abs() < 0.5, "{}", gap(NARROW));
}

/// The words of the badges drawn over a rectangle.
fn badges_over(tree: &TreeUpdate, over: Rect) -> Vec<String> {
    tree.nodes
        .iter()
        .filter(|(_, node)| !matches!(node.role(), Role::Button | Role::Navigation) && node.bounds().is_some())
        .filter_map(|(_, node)| {
            let rect = bounds(node);
            let meets =
                rect.x < over.right() && rect.right() > over.x && rect.y < over.bottom() && rect.bottom() > over.y;
            meets
                .then(|| node.label().or_else(|| node.value()).map(str::to_owned))
                .flatten()
        })
        .collect()
}

#[test]
fn graph_view_bar__with_three_changed_files_changes_shows_3_and_with_none_no_number() {
    let (repository, _remote) = repository_with_history();
    let clean = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&clean, &[]);
    let changes = bounds(find(&tree, Role::Button, "Changes"));
    assert_eq!(badges_over(&tree, changes), [] as [String; 0]);
    for name in ["a.txt", "b.txt", "c.txt"] {
        std::fs::write(repository.path().join(name), "new\n").unwrap();
    }
    let with_changes = showing(repository.path().to_path_buf());
    let (_, tree) = window.draw(&with_changes, &[]);
    let changes = bounds(find(&tree, Role::Button, "Changes"));
    assert_eq!(badges_over(&tree, changes), ["3"]);
}

#[test]
fn graph_view_bar__a_click_on_changes_opens_the_changes_page_and_a_click_on_timeline_comes_back() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Changes");
    assert_eq!(intents, [Intent::ShowPage(Page::Changes)]);
    for intent in intents {
        let _effects = run(&mut state, Message::Intent(intent));
    }
    assert_eq!(state.page_shown(), Some(Page::Changes));
    let data = app::store::ViewData::new(&state, Vec::new(), super::support::NOW);
    assert_eq!(data.window_state(&state).view_bar.shown, views::ViewShown::Changes);
    let intents = window.click(&state, Role::Button, "Timeline");
    assert_eq!(intents, [Intent::ShowTimeline]);
}

#[test]
fn graph_view_bar__a_click_on_activity_shows_the_bottom_panel_at_activity_and_another_hides_it() {
    use views::panel::{PanelIntent, PanelTab};
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for is_shown in [true, false] {
        let intents = window.click(&state, Role::Button, "Activity");
        assert_eq!(intents, [Intent::Panel(PanelIntent::ToggleActivity)]);
        let _effects = run(&mut state, Message::Intent(Intent::Panel(PanelIntent::ToggleActivity)));
        let look = app::store::panel::look(&state);
        assert_eq!((look.is_shown, look.tab), (is_shown, PanelTab::Activity));
        let (_, tree) = window.draw(&state, &[]);
        assert_eq!(has(&tree, Role::Region, "What is running"), is_shown);
    }
}

#[test]
fn graph_view_bar__with_the_panel_at_activity_a_click_on_terminal_shows_its_terminals() {
    use views::panel::{PanelIntent, PanelTab};
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _shown = run(&mut state, Message::Intent(Intent::Panel(PanelIntent::ToggleActivity)));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Terminal");
    assert_eq!(intents, [Intent::Panel(PanelIntent::Toggle)]);
    let _effects = run(&mut state, Message::Intent(Intent::Panel(PanelIntent::Toggle)));
    let look = app::store::panel::look(&state);
    assert_eq!((look.is_shown, look.tab), (true, PanelTab::Terminal));
}

#[test]
fn graph_view_bar__with_no_repository_read_timeline_changes_files_search_compare_and_terminal_cannot_be_clicked() {
    let state = window();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    for name in ["Timeline", "Changes", "Files", "Search in files", "Compare", "Terminal"] {
        assert!(find(&tree, Role::Button, name).is_disabled(), "{name} can be clicked");
    }
    for name in ["Activity", "Settings"] {
        assert!(
            !find(&tree, Role::Button, name).is_disabled(),
            "{name} cannot be clicked"
        );
    }
}

#[test]
fn graph_view_bar__a_click_on_search_in_files_opens_the_search_page_and_marks_it() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Search in files");
    assert_eq!(
        intents,
        [Intent::FileSearch(views::file_search::FileSearchIntent::Show)]
    );
    for intent in intents {
        let _effects = run(&mut state, Message::Intent(intent));
    }
    assert_eq!(state.page_shown(), Some(Page::Search));
    let data = app::store::ViewData::new(&state, Vec::new(), super::support::NOW);
    assert_eq!(data.window_state(&state).view_bar.shown, views::ViewShown::Search);
}

#[test]
fn graph_search_button__a_click_on_search_opens_quick_search_with_nothing_typed() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Search");
    assert_eq!(intents, [Intent::Search(SearchIntent::OpenQuick(String::new()))]);
}

#[test]
fn graph_search_button__ctrl_k_opens_quick_search_from_a_page_too() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _effects = run(&mut state, Message::Intent(Intent::ShowPage(Page::Settings)));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (intents, _) = window.draw(&state, &[shortcut('k', Modifiers::CONTROL)]);
    assert!(
        intents.contains(&Intent::Search(SearchIntent::OpenQuick(String::new()))),
        "{intents:?}"
    );
}

#[test]
fn graph_search_button__in_a_window_narrower_than_900_pixels_the_button_shows_its_icon_alone() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, NARROW);
    let (_, tree) = window.draw(&state, &[]);
    let button = bounds(find(&tree, Role::Button, "Search (Ctrl+K)"));
    assert!(button.width < 40.0, "{button:?}");
    assert!(!has(&tree, Role::Button, "Search"));
}

#[test]
fn graph_toolbar__the_header_has_no_box_of_text_the_filter_and_the_search_box_are_at_the_top_of_the_timeline() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let header = bounds(
        tree.nodes
            .iter()
            .map(|(_, node)| node)
            .find(|node| node.role() == Role::Banner)
            .unwrap(),
    );
    let boxes: Vec<Rect> = tree
        .nodes
        .iter()
        .map(|(_, node)| node)
        .filter(|node| matches!(node.role(), Role::TextInput | Role::SearchInput))
        .map(bounds)
        .collect();
    assert!(!boxes.is_empty(), "the timeline has its search box");
    assert!(
        boxes.iter().all(|rect| rect.y >= header.bottom()),
        "{boxes:?} under {header:?}"
    );
    let timeline = bounds(find(&tree, Role::Main, "Timeline"));
    let filter = tree
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| {
            node.label()
                .is_some_and(|label| label.starts_with("Branches shown in the timeline"))
        })
        .map(bounds)
        .unwrap();
    assert!(filter.x >= timeline.x && filter.y >= header.bottom(), "{filter:?}");
}
