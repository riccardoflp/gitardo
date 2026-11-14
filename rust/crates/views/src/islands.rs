//! The islands of the window (`graph.islands`, D21): the header, the branch rail, the timeline, the inspector and the
//! page shown, each a rounded card on the ground of the window, with a gap between them.

use ui::{Border, FillRule, Path, Point, Rect, Ui};

use crate::line::is_narrow;

/// The gap between two islands, and between the islands and the edges of the window.
pub(crate) const GAP: f32 = 8.0;
/// The gap in a narrow window.
pub(crate) const NARROW_GAP: f32 = 4.0;
/// How far from the end of a quarter circle the control points of the curve that draws it are, as a share of its
/// radius.
const QUARTER_CIRCLE: f32 = 0.552_284_8;

/// The gap between islands in this window: smaller when the window is narrow.
pub(crate) fn gap(ui: &Ui<'_>) -> f32 {
    if is_narrow(ui) { NARROW_GAP } else { GAP }
}

/// An island over a rectangle: its rounded ground, what `inside` draws clipped to it, then the corners cut back to the
/// ground of the window over whatever was drawn there, and its border. Gives what `inside` gives.
///
/// Feature: graph.islands
pub fn island<R>(ui: &mut Ui<'_>, rect: Rect, inside: impl FnOnce(&mut Ui<'_>) -> R) -> R {
    let theme = *ui.theme();
    let radius = theme.sizes.radius.large.min(rect.width * 0.5).min(rect.height * 0.5);
    ui.fill_rounded_rect(rect, radius, theme.colors.page_background);
    let given = ui.clip(rect, inside);
    if radius > 0.0 {
        ui.fill_path(corners(rect, radius), theme.colors.ground_background, FillRule::NonZero);
    }
    ui.border(
        rect,
        Border {
            width: theme.sizes.border,
            radius,
            color: theme.colors.border,
        },
    );
    given
}

/// The four corners of a rectangle outside the quarter circles of a radius: what is cut back to the ground.
fn corners(rect: Rect, radius: f32) -> Path {
    let mut path = Path::new();
    let near = radius * (1.0 - QUARTER_CIRCLE);
    // Each corner as its point and the directions towards the inside of the rectangle along x and y.
    let corners = [
        (Point::new(rect.x, rect.y), 1.0, 1.0),
        (Point::new(rect.right(), rect.y), -1.0, 1.0),
        (Point::new(rect.right(), rect.bottom()), -1.0, -1.0),
        (Point::new(rect.x, rect.bottom()), 1.0, -1.0),
    ];
    for (corner, along_x, along_y) in corners {
        let on_x = Point::new(corner.x + along_x * radius, corner.y);
        let on_y = Point::new(corner.x, corner.y + along_y * radius);
        let _ = path
            .move_to(corner)
            .line_to(on_x)
            .cubic_to(
                Point::new(corner.x + along_x * near, corner.y),
                Point::new(corner.x, corner.y + along_y * near),
                on_y,
            )
            .close();
    }
    path
}
