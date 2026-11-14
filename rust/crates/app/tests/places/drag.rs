//! Dragging a commit or a branch onto the timeline (`drag.drop`): the rows that take what is dragged outlined, the
//! menu a drop opens at the pointer with what it offers, and the operation of the entry chosen.

use app::store::{Message, State};
use model::contract::QuestionKind;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use testkit::Repository;
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent};
use views::Intent;
use views::graph::GraphIntent;

use super::pointer::{bounds, moved, rows_drawn, said_at};
use super::showing;
use super::support::{OperationsDriver, Window};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 520);

/// A repository whose `main`, checked out, has `Fix the parser` over `Add the readme`, and whose `feature` has
/// `Show the branch rail` over `Add the readme`: the timeline lists `Fix the parser`, `Show the branch rail`, `Add the
/// readme`. Gives the repository and the hashes of the three commits, newest first.
fn two_branches() -> (Repository, [String; 3]) {
    let mut repository = Repository::new();
    let readme = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let rail = repository.commit("Show the branch rail", &[("rail.txt", "branches\n")]);
    repository.checkout("main");
    let parser = repository.commit("Fix the parser", &[("parser.txt", "parse\n")]);
    (repository, [parser, rail, readme])
}

/// The main button at a point.
fn button(position: Point) -> PointerButtonEvent {
    PointerButtonEvent {
        button: PointerButton::Primary,
        position,
        modifiers: Modifiers::NONE,
        click_count: 1,
    }
}

/// Presses the main button at a point and moves to another, holding it, and gives the intents of the frames and the
/// tree of the last.
fn drag_over(window: &mut Window, state: &State, from: Point, to: Point) -> (Vec<Intent>, TreeUpdate) {
    let mut intents = window.draw(state, &[moved(from)]).0;
    intents.extend(window.draw(state, &[InputEvent::PointerPressed(button(from))]).0);
    let halfway = Point::new(f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y));
    intents.extend(window.draw(state, &[moved(halfway)]).0);
    let (last, tree) = window.draw(state, &[moved(to)]);
    intents.extend(last);
    (intents, tree)
}

/// Lets the button go where the pointer is, and gives the intents of the frame and its tree.
fn release(window: &mut Window, state: &State, at: Point) -> (Vec<Intent>, TreeUpdate) {
    window.draw(state, &[InputEvent::PointerReleased(button(at))])
}

/// Drags from a point to another and lets go there, and gives the intents of the frames and the tree of the last.
fn drag_and_drop(window: &mut Window, state: &State, from: Point, to: Point) -> (Vec<Intent>, TreeUpdate) {
    let (mut intents, _) = drag_over(window, state, from, to);
    let (dropped, tree) = release(window, state, to);
    intents.extend(dropped);
    (intents, tree)
}

/// The entries of the menu open, from the top.
fn menu_entries(tree: &TreeUpdate) -> Vec<String> {
    let mut entries: Vec<(f64, String)> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::MenuItem)
        .filter_map(|(_, node)| Some((node.bounds()?.y0, node.label()?.to_owned())))
        .collect();
    entries.sort_by(|one, other| one.0.total_cmp(&other.0));
    entries.into_iter().map(|(_, label)| label).collect()
}

/// A point on the hash at the end of a row, where no label is.
fn on_hash(row: Rect) -> Point {
    Point::new(row.right() - 40.0, row.y + row.height * 0.5)
}

/// The middle of a rectangle.
fn middle(rect: Rect) -> Point {
    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

/// Hands the intents to the store.
fn apply(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
}

/// Whether a drop was said among the intents.
fn has_dropped(intents: &[Intent]) -> bool {
    intents
        .iter()
        .any(|intent| matches!(intent, Intent::Graph(GraphIntent::Dropped(_))))
}

#[test]
fn drag_drop__a_commit_dropped_on_the_commit_checked_out_opens_a_menu_with_one_entry_to_cherry_pick_it() {
    let (repository, [_, rail, _]) = two_branches();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rows = rows_drawn(&tree);
    let (intents, tree) = drag_and_drop(&mut window, &state, on_hash(rows[1]), on_hash(rows[0]));
    let short = &rail[..7];
    assert_eq!(menu_entries(&tree), [format!("Cherry-pick {short} onto main")]);
    apply(&driver, &mut state, intents);
    let (_, tree) = window.draw(&state, &[]);
    assert_eq!(
        menu_entries(&tree),
        [format!("Cherry-pick {short} onto main")],
        "the menu stays open"
    );
    let chosen = window.click(&state, Role::MenuItem, &format!("Cherry-pick {short} onto main"));
    assert!(
        chosen.contains(&Intent::Feature(views::feature::FeatureIntent::Run(
            model::contract::FeatureId("cherry-pick.commit"),
            model::contract::Target::Commit(model::CommitId::parse(&rail).unwrap()),
        ))),
        "{chosen:?}"
    );
}

#[test]
fn drag_drop__feature_dragged_from_the_rail_onto_the_commit_checked_out_outlines_it_and_offers_two_merges_and_a_rebase()
{
    let (repository, _) = two_branches();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rows = rows_drawn(&tree);
    let feature = bounds(&tree, Role::Button, "feature");
    let (_, _) = drag_over(&mut window, &state, middle(feature), on_hash(rows[0]));
    window.assert_snapshot("drag-feature-over-main-dark");
    let (intents, tree) = release(&mut window, &state, on_hash(rows[0]));
    assert!(has_dropped(&intents));
    assert_eq!(
        menu_entries(&tree),
        [
            "Merge feature into main",
            "Merge feature into main\u{2026}",
            "Rebase main onto feature"
        ]
    );
}

#[test]
fn drag_drop__the_current_branch_dropped_on_another_commit_then_reset_asks_soft_mixed_or_hard() {
    let (repository, [parser, _, readme]) = two_branches();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rows = rows_drawn(&tree);
    let main = said_at(&tree, "main", rows[0]);
    let (intents, tree) = drag_and_drop(&mut window, &state, middle(main), on_hash(rows[2]));
    let short = &readme[..7];
    let reset = format!("Reset main to {short}\u{2026}");
    assert_eq!(
        menu_entries(&tree),
        [format!("Rebase main onto {short}"), reset.clone()]
    );
    apply(&driver, &mut state, intents);
    let chosen = window.click(&state, Role::MenuItem, &reset);
    apply(&driver, &mut state, chosen);
    let Message::FeatureAsks(question) = driver.next(&mut state) else {
        panic!("the reset asks how");
    };
    let QuestionKind::Pick { choices } = &question.kind else {
        panic!("the reset asks with a pick: {question:?}");
    };
    let ways: Vec<&str> = choices.iter().map(|choice| choice.identifier.as_str()).collect();
    assert_eq!(ways, ["soft", "mixed", "hard"]);
    assert_eq!(repository.git(&["rev-parse", "HEAD"]).trim(), parser, "nothing ran yet");
}

#[test]
fn drag_drop__a_drop_on_a_row_that_does_not_take_it_opens_no_menu() {
    let (repository, _) = two_branches();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let rows = rows_drawn(&tree);
    let (intents, tree) = drag_and_drop(&mut window, &state, on_hash(rows[2]), on_hash(rows[1]));
    assert!(!has_dropped(&intents), "{intents:?}");
    assert_eq!(menu_entries(&tree), Vec::<String>::new());
}
