//! What a row shows only under the pointer stays there while the pointer is on it (M23): for each such control of the
//! window, the pointer goes from its row onto it, rests there ten frames, each drawn as the one before with the control
//! in it, and a click does what the control says.

use app::store::{Message, State};
use model::contract::{FeatureId, Target};
use model::filter::BranchesShown;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent};
use views::feature::FeatureIntent;
use views::files::FilesPageIntent;
use views::graph::GraphIntent;
use views::{FilterIntent, Intent, RailIntent, RailList};

use super::pointer::{bounds, middle, moved, rect, rows_drawn, said_at};
use super::support::{Window, has, run};
use super::tabs::{WIDE, three_clones};
use super::{repository_with_history, showing};

/// How many frames the pointer rests on a control before it is clicked.
const RESTING_FRAMES: usize = 10;

/// The node of a role and a name inside a rectangle, which the test fails without: the control of one row among those
/// of the same name on other rows.
pub(super) fn control_in(tree: &TreeUpdate, (role, name): (Role, &str), row: Rect) -> Rect {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.role() == role && node.label() == Some(name))
        .filter_map(|(_, node)| node.bounds().map(rect))
        .find(|found| row.contains(middle(*found)))
        .unwrap_or_else(|| panic!("no {role:?} named {name} on the row at {row:?}"))
}

/// Moves the pointer onto a point of a row, then onto the control the row shows under the pointer, which `control`
/// finds in what the screen reader is told; rests there ten frames, each of which has the control where it was and
/// draws what the frame before drew, the row lit as it was with the pointer beside the control; then presses and
/// releases the main button, each in a frame of its own, and gives the intents of those two frames.
pub(super) fn rest_and_click(
    window: &mut Window,
    state: &State,
    on_row: Point,
    control: impl Fn(&TreeUpdate) -> Rect,
) -> Vec<Intent> {
    let _ = window.draw(state, &[]);
    let (_, tree) = window.draw(state, &[moved(on_row)]);
    let (_, tree_again) = window.draw(state, &[]);
    let place = control(&tree);
    assert_eq!(
        control(&tree_again),
        place,
        "the control stays while the pointer is on its row"
    );
    let lit = pixel_at(window, on_row);
    let on_control = middle(place);
    let (_, tree) = window.draw(state, &[moved(on_control)]);
    assert_eq!(control(&tree), place, "the control is drawn with the pointer on it");
    let drawn = window.frame.pixels().to_vec();
    for frame in 1..RESTING_FRAMES {
        let (intents, tree) = window.draw(state, &[]);
        assert!(intents.is_empty(), "resting asks nothing, in frame {frame}");
        assert_eq!(control(&tree), place, "the control is drawn in frame {frame}");
        assert!(
            window.frame.pixels() == drawn.as_slice(),
            "frame {frame} draws what the frame before drew"
        );
    }
    assert_eq!(pixel_at(window, on_row), lit, "the row keeps its background");
    let button = PointerButtonEvent {
        button: PointerButton::Primary,
        position: on_control,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    let (mut intents, tree) = window.draw(state, &[InputEvent::PointerPressed(button)]);
    assert_eq!(control(&tree), place, "the control is drawn while it is pressed");
    intents.extend(window.draw(state, &[InputEvent::PointerReleased(button)]).0);
    intents
}

/// The color of the pixel under a point of the window.
fn pixel_at(window: &Window, point: Point) -> Option<render::Color> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an i32 counts"
    )]
    window.frame.pixel(point.x as i32, point.y as i32)
}

/// A point of a row near its left end, where no control of its right end is.
pub(super) fn left_of(row: Rect) -> Point {
    Point::new(row.x + row.height, row.y + row.height * 0.5)
}

#[test]
fn repo_tabs__the_close_button_of_a_row_not_shown_stays_under_the_pointer_and_the_row_stays_lit() {
    let (_kept, _clones, state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let row = bounds(&tree, Role::Button, "web");
    let close = (Role::Button, views::text::repo::CLOSE_TAB);
    assert!(
        tree.nodes
            .iter()
            .filter(|(_, node)| node.label() == Some(close.1))
            .count()
            == 1,
        "only the row shown has its close button while the pointer is elsewhere"
    );
    let unlit = pixel_at(&window, left_of(row));
    let _ = rest_and_click(&mut window, &state, left_of(row), |tree| control_in(tree, close, row));
    assert_ne!(
        pixel_at(&window, left_of(row)),
        unlit,
        "the row has the background of a row under the pointer"
    );
}

#[test]
fn repo_close_tab__a_click_on_the_close_button_of_a_row_not_shown_closes_that_tab() {
    let (_kept, _clones, state) = three_clones();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let row = bounds(&tree, Role::Button, "web");
    let close = (Role::Button, views::text::repo::CLOSE_TAB);
    let intents = rest_and_click(&mut window, &state, left_of(row), |tree| control_in(tree, close, row));
    assert_eq!(intents, [Intent::CloseTab(1)]);
}

/// Hands an intent to the store, running the jobs it starts.
fn ask(state: &mut State, intent: Intent) {
    let _effects = run(state, Message::Intent(intent));
}

/// The row of an entry of the branch rail, by its name, as it is drawn before the pointer comes.
fn rail_row(window: &mut Window, state: &State, name: &str) -> Rect {
    let (_, tree) = window.draw(state, &[]);
    super::rail::rail_entry(&tree, name).unwrap_or_else(|| panic!("the rail does not list {name}"))
}

#[test]
fn refs_tree__the_menu_button_of_the_title_of_a_list_stays_under_the_pointer_and_a_click_opens_the_menu() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let title = rail_row(&mut window, &state, views::text::rail::LOCAL);
    let more = (Role::Button, views::text::rail::LIST_MENU);
    let intents = rest_and_click(&mut window, &state, middle(title), |tree| control_in(tree, more, title));
    assert!(intents.is_empty(), "the menu is the toolkit's to open: {intents:?}");
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::MenuItem, views::text::rail::SORT_BY_NAME));
}

#[test]
fn filter_list_too__the_eye_of_a_dimmed_branch_stays_under_the_pointer_and_a_click_adds_its_history() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let main = BranchesShown::References(vec![String::from("refs/heads/main")]);
    ask(&mut state, Intent::Filter(FilterIntent::Show(Some(main))));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let row = rail_row(&mut window, &state, "feature");
    let eye = (Role::Button, views::text::filter::SHOW_TOO);
    let intents = rest_and_click(&mut window, &state, middle(row), |tree| control_in(tree, eye, row));
    let both = BranchesShown::References(vec![
        String::from("refs/heads/main"),
        String::from("refs/heads/feature"),
    ]);
    assert_eq!(intents, [Intent::Filter(FilterIntent::Show(Some(both)))]);
}

#[test]
fn stash_pane__the_button_of_the_title_stays_under_the_pointer_and_a_click_asks_to_stash_all_changes() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let title = rail_row(&mut window, &state, views::text::rail::STASHES);
    let stash_all = (Role::Button, views::text::stash::STASH_ALL);
    let intents = rest_and_click(&mut window, &state, middle(title), |tree| {
        control_in(tree, stash_all, title)
    });
    let push = FeatureIntent::Run(FeatureId("stash.push"), Target::Head);
    assert_eq!(intents, [Intent::Feature(push)]);
}

#[test]
fn stash_pane__the_buttons_of_a_row_stay_under_the_pointer_and_a_click_on_apply_asks_to_apply_the_stash() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    ask(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Stashes)));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let row = rail_row(&mut window, &state, "On main: half the tags");
    let apply = (Role::Button, views::text::stash::APPLY);
    let intents = rest_and_click(&mut window, &state, middle(row), |tree| control_in(tree, apply, row));
    let applied = FeatureIntent::Run(FeatureId("stash.apply"), Target::Stash(0));
    assert_eq!(intents, [Intent::Feature(applied)]);
}

#[test]
fn details_show__the_history_button_of_a_file_stays_under_the_pointer_and_a_click_asks_for_its_history() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    let _ = super::inspector::select(&mut state, "Show the branch rail");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let inspector = bounds(&tree, Role::Complementary, views::text::timeline::INSPECTOR);
    let name = said_at(&tree, "rail.txt", inspector);
    let history = views::text::details::FILE_HISTORY;
    assert!(!has(&tree, Role::Button, history), "no row is under the pointer yet");
    let intents = rest_and_click(&mut window, &state, middle(name), |tree| {
        bounds(tree, Role::Button, history)
    });
    let asked = FilesPageIntent::Run(FeatureId("file-history.file"), String::from("rail.txt"));
    assert_eq!(intents, [Intent::Files(asked)]);
}

#[test]
fn graph_fold_merge__the_arrow_of_a_merge_stays_under_the_pointer_and_a_click_folds_it() {
    let repository = super::folds::merged_feature();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let merge = state
        .timeline
        .rows
        .iter()
        .filter_map(model::graph::Row::commit)
        .find(|commit| commit.subject == "Merge branch 'feature'")
        .map(|commit| commit.id.clone())
        .unwrap();
    let shown = state.timeline.layout.shown_of.get(&merge).unwrap();
    let row = *rows_drawn(&tree).get(*shown).unwrap();
    let fold = views::text::timeline::FOLD_MERGE;
    assert!(
        !has(&tree, Role::Button, fold),
        "the merge is neither selected nor under the pointer yet"
    );
    let intents = rest_and_click(&mut window, &state, middle(row), |tree| {
        bounds(tree, Role::Button, fold)
    });
    assert_eq!(intents, [Intent::Graph(GraphIntent::Fold(merge))]);
}

#[test]
fn the_window_draws_the_same_twice_in_a_row_while_the_pointer_rests_anywhere_in_it() {
    let (repository, _remote) = repository_with_history();
    let mut state = showing(repository.path().to_path_buf());
    ask(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Stashes)));
    let main = BranchesShown::References(vec![String::from("refs/heads/main")]);
    ask(&mut state, Intent::Filter(FilterIntent::Show(Some(main))));
    let _ = super::inspector::select(&mut state, "Show the branch rail");
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let _ = window.draw(&state, &[]);
    let mut flickering = Vec::new();
    for row in 0..WIDE.1 / STEP {
        for column in 0..WIDE.0 / STEP {
            #[allow(clippy::cast_precision_loss, reason = "a window is far smaller than an f32 counts")]
            let point = Point::new((column * STEP + STEP / 2) as f32, (row * STEP + STEP / 2) as f32);
            let _ = window.draw(&state, &[moved(point)]);
            let _ = window.draw(&state, &[]);
            let _ = window.draw(&state, &[]);
            let settled = window.frame.pixels().to_vec();
            let _ = window.draw(&state, &[]);
            if window.frame.pixels() != settled.as_slice() {
                flickering.push((point.x, point.y));
            }
        }
    }
    assert!(
        flickering.is_empty(),
        "the window flickers with the pointer at {flickering:?}"
    );
}

/// How far apart, in logical pixels, the points the pointer rests on are.
const STEP: u32 = 16;
