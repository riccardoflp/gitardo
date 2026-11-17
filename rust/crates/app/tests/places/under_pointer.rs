//! What a row shows only under the pointer stays there while the pointer is on it (M23): for each such control of the
//! window, the pointer goes from its row onto it, rests there ten frames, each drawn as the one before with the control
//! in it, and a click does what the control says.

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::{Point, Rect, ThemeName};
use ui::{InputEvent, Modifiers, PointerButton, PointerButtonEvent};
use views::Intent;

use super::pointer::{bounds, middle, moved, rect};
use super::support::Window;
use super::tabs::{WIDE, three_clones};

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
