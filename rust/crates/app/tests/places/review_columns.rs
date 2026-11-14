//! The conflicts of the review in three columns, drawn from the store: a line too long for its column wraps under its
//! number, and the pointer on a line outlines it and the lines joined to it.

use app::store::State;
use platform::accesskit::{Role, TreeUpdate};
use render::{Color, Point, Rect, ThemeName};
use views::review::{Layout, ReviewIntent};

use super::pointer::moved;
use super::review::{NEXT_TO_EACH_OTHER, file, merged, review, reviewed};
use super::support::Window;

/// The size of the window the columns are drawn in.
const SIZE: (u32, u32) = (1480, 1000);

/// A line of the current side far longer than its column.
const LONG: &str = "B, the quick brown fox jumps over the lazy dog and runs on and on past the edge of its column";

/// Where the screen reader is told each text of a role is, from the left.
fn places(tree: &TreeUpdate, text: &str) -> Vec<Rect> {
    let mut found: Vec<Rect> = tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::Label && (node.label() == Some(text) || node.value() == Some(text)))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "a window is far smaller than an f32 counts"
            )]
            let rect = Rect::new(
                bounds.x0 as f32,
                bounds.y0 as f32,
                (bounds.x1 - bounds.x0) as f32,
                (bounds.y1 - bounds.y0) as f32,
            );
            rect
        })
        .collect();
    found.sort_by(|first, second| first.x.total_cmp(&second.x));
    found
}

/// The review in Columns of a repository, drawn once.
fn in_columns(state: &mut State) -> (Window, TreeUpdate) {
    review(state, ReviewIntent::Layout(Layout::Columns));
    let mut window = Window::new(ThemeName::Dark, SIZE);
    let (_, tree) = window.draw(state, &[moved(Point::new(1.0, 1.0))]);
    (window, tree)
}

/// The colors of the pixels of the window along a band, at a height, from one x to another.
fn colors_along(window: &Window, y: f32, (from, to): (f32, f32)) -> Vec<Color> {
    let mut colors: Vec<Color> = Vec::new();
    let mut x = from;
    while x < to {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "a window is far smaller than an i32 counts"
        )]
        let pixel = window.frame.pixel(x.round() as i32, y.round() as i32);
        if let Some(color) = pixel
            && !colors.contains(&color)
        {
            colors.push(color);
        }
        x += 1.0;
    }
    colors
}

/// The pixels of the edges of a line: along its top and down its left, where its outline is drawn.
fn edges(window: &Window, rect: Rect) -> Vec<Option<Color>> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a window is far smaller than an i32 counts"
    )]
    let (left, top) = (rect.x.floor() as i32, rect.y.floor() as i32);
    let mut pixels = Vec::new();
    for offset in 0..2 {
        pixels.extend((8..40).map(|x| window.frame.pixel(left + x, top + offset)));
        pixels.extend((2..10).map(|y| window.frame.pixel(left + offset, top + y)));
    }
    pixels
}

#[test]
fn review_columns__a_line_too_long_for_its_column_wraps_keeping_its_number_on_its_first_row() {
    let main = format!("a\n{LONG}\nc\nd\n");
    let repository = merged(&[file("one.txt", ("a\nb\nc\nd\n", &main, "a\nb\nC\nd\n"))]);
    let mut state = reviewed(&repository);
    let (window, tree) = in_columns(&mut state);
    let long = places(&tree, LONG);
    let [current, proposed] = long.as_slice() else {
        panic!("the long line is on the current side and proposed: {long:?}");
    };
    let row = places(&tree, "d").first().unwrap().height;
    assert!(
        current.height >= 2.0 * row && (current.height / row - (current.height / row).round()).abs() < 0.01,
        "the long line takes whole rows, two at least: {current:?} in rows of {row}"
    );
    assert_eq!(proposed.height, current.height);
    for line in [current, proposed] {
        let numbers = places(&tree, "2");
        let number = numbers
            .iter()
            .find(|number| (number.x - line.x).abs() < 0.5 && (number.y - line.y).abs() < 0.5);
        assert_eq!(
            number.map(|number| number.height),
            Some(row),
            "its number is on its first row"
        );
    }
    let incoming = places(&tree, "b");
    let incoming = incoming.last().unwrap();
    assert!(
        current.right() <= proposed.x && proposed.right() <= incoming.x,
        "the columns stand side by side"
    );
    assert!(
        incoming.right() <= f32::from(u16::try_from(SIZE.0).unwrap()),
        "no column goes past the page"
    );
    let second_row = proposed.y + row * 1.5;
    let text = colors_along(
        &window,
        second_row,
        (proposed.x + proposed.width * 0.4, proposed.right()),
    );
    assert!(text.len() > 1, "the second row of the line has its text");
}

#[test]
fn review_columns__the_pointer_on_a_proposed_line_outlines_it_and_the_line_of_the_side_it_is_joined_to() {
    let repository = merged(&[file("one.txt", NEXT_TO_EACH_OTHER)]);
    let mut state = reviewed(&repository);
    let (mut window, tree) = in_columns(&mut state);
    let [current_b, proposed_b] = places(&tree, "B")[..] else {
        panic!("B is on the current side and proposed");
    };
    let [proposed_c, incoming_c] = places(&tree, "C")[..] else {
        panic!("C is proposed and on the incoming side");
    };
    let incoming_b = *places(&tree, "b").last().unwrap();
    let lines = [current_b, proposed_b, proposed_c, incoming_c, incoming_b];
    let before: Vec<Vec<Option<Color>>> = lines.iter().map(|line| edges(&window, *line)).collect();
    let pointer = Point::new(
        proposed_b.x + proposed_b.width * 0.5,
        proposed_b.y + proposed_b.height * 0.5,
    );
    let _ = window.draw(&state, &[moved(pointer)]);
    let outlined: Vec<bool> = lines
        .iter()
        .zip(&before)
        .map(|(line, before)| edges(&window, *line) != *before)
        .collect();
    assert_eq!(
        outlined,
        [true, true, false, false, false],
        "B proposed and the current side's B are outlined, nothing else"
    );
}
