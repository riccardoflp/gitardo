//! The design of each icon, on a grid of 16 by 16. Straight lines along the axes sit on the middle of a pixel (at
//! `x.5`), so that a line one pixel wide covers whole pixels at 100 % and at 200 %.

use super::{Design, Icon};
use crate::geometry::Point;
use crate::path::Path;

/// The cosine of an eighth of a turn, for the teeth of the gear.
const DIAGONAL: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// The paths of an icon.
pub(crate) fn design(icon: Icon) -> Design {
    match icon {
        Icon::ChevronRight => lines(&[&[(6.0, 3.5), (10.5, 8.0), (6.0, 12.5)]]),
        Icon::ChevronDown => lines(&[&[(3.5, 6.0), (8.0, 10.5), (12.5, 6.0)]]),
        Icon::Close => lines(&[&[(3.5, 3.5), (12.5, 12.5)], &[(12.5, 3.5), (3.5, 12.5)]]),
        Icon::Check => lines(&[&[(2.5, 8.5), (6.0, 12.0), (13.5, 4.0)]]),
        Icon::Plus => lines(&[&[(7.5, 2.5), (7.5, 12.5)], &[(2.5, 7.5), (12.5, 7.5)]]),
        Icon::Minus => lines(&[&[(2.5, 7.5), (12.5, 7.5)]]),
        Icon::Search => search(),
        Icon::Branch => branch(),
        Icon::Tag => tag(),
        Icon::Cloud => cloud(),
        Icon::Commit => commit(),
        Icon::Folder => closed(&[
            (1.5, 3.5),
            (6.0, 3.5),
            (7.5, 5.0),
            (14.5, 5.0),
            (14.5, 12.5),
            (1.5, 12.5),
        ]),
        Icon::File => file(),
        Icon::Refresh => refresh(),
        Icon::Gear => gear(),
        Icon::More => dots(&[(3.0, 8.0), (8.0, 8.0), (13.0, 8.0)], 1.25),
        Icon::ArrowUp => lines(&[&[(7.5, 13.5), (7.5, 2.5)], &[(3.0, 7.0), (7.5, 2.5), (12.0, 7.0)]]),
        Icon::ArrowDown => lines(&[&[(7.5, 2.5), (7.5, 13.5)], &[(3.0, 9.0), (7.5, 13.5), (12.0, 9.0)]]),
        Icon::Warning => warning(),
        Icon::Error => with_ring(lines(&[&[(5.5, 5.5), (10.5, 10.5)], &[(10.5, 5.5), (5.5, 10.5)]])),
        Icon::Info => {
            let mut design = with_ring(lines(&[&[(8.0, 7.0), (8.0, 11.5)]]));
            let _ = design.fills.extend(&Path::circle(Point::new(8.0, 4.75), 0.85));
            design
        }
        Icon::Worktree => worktree(),
        Icon::Stash => stash(),
        Icon::Star => star(),
        Icon::Clock => with_ring(lines(&[&[(8.0, 4.5), (8.0, 8.0), (10.5, 9.5)]])),
        Icon::Eye => eye(),
        Icon::Filter => lines(&[
            &[(2.5, 4.5), (13.5, 4.5)],
            &[(4.5, 8.0), (11.5, 8.0)],
            &[(6.5, 11.5), (9.5, 11.5)],
        ]),
        Icon::Pencil => {
            let mut design = closed(&[(2.5, 13.5), (3.0, 10.5), (10.5, 3.0), (13.0, 5.5), (5.5, 13.0)]);
            add_polyline(&mut design.strokes, &[(9.0, 4.5), (11.5, 7.0)]);
            design
        }
        Icon::Wand => lines(&[
            &[(2.5, 13.5), (9.5, 6.5)],
            &[(12.0, 1.5), (12.0, 6.5)],
            &[(9.5, 4.0), (14.5, 4.0)],
        ]),
    }
}

/// Open lines through points, each a list of points.
fn lines(polylines: &[&[(f32, f32)]]) -> Design {
    let mut strokes = Path::new();
    for points in polylines {
        add_polyline(&mut strokes, points);
    }
    Design {
        strokes,
        fills: Path::new(),
    }
}

/// A closed outline through points.
fn closed(points: &[(f32, f32)]) -> Design {
    let mut strokes = Path::new();
    add_polyline(&mut strokes, points);
    let _ = strokes.close();
    Design {
        strokes,
        fills: Path::new(),
    }
}

/// Adds an open line through points to a path.
fn add_polyline(path: &mut Path, points: &[(f32, f32)]) {
    for (index, (x, y)) in points.iter().enumerate() {
        let point = Point::new(*x, *y);
        let _ = if index == 0 {
            path.move_to(point)
        } else {
            path.line_to(point)
        };
    }
}

/// Filled dots of a radius.
fn dots(centers: &[(f32, f32)], radius: f32) -> Design {
    let mut fills = Path::new();
    for (x, y) in centers {
        let _ = fills.extend(&Path::circle(Point::new(*x, *y), radius));
    }
    Design {
        strokes: Path::new(),
        fills,
    }
}

/// A design inside a ring that fills the grid.
fn with_ring(mut design: Design) -> Design {
    let _ = design.strokes.extend(&Path::circle(Point::new(8.0, 8.0), 6.0));
    design
}

/// A magnifying glass: a ring and its handle.
fn search() -> Design {
    let mut design = lines(&[&[(10.0, 10.0), (13.5, 13.5)]]);
    let _ = design.strokes.extend(&Path::circle(Point::new(6.5, 6.5), 4.5));
    design
}

/// A branch: a straight line of commits, and a second line curving away from it to a commit of its own.
fn branch() -> Design {
    let mut design = lines(&[&[(4.5, 5.0), (4.5, 11.0)]]);
    let strokes = &mut design.strokes;
    let _ = strokes.move_to(Point::new(11.5, 6.0)).cubic_to(
        Point::new(11.5, 9.0),
        Point::new(4.5, 8.0),
        Point::new(4.5, 11.0),
    );
    for (x, y) in [(4.5, 3.5), (4.5, 12.5), (11.5, 4.5)] {
        let _ = strokes.extend(&Path::circle(Point::new(x, y), 1.5));
    }
    design
}

/// A tag: a label with a pointed end and a hole.
fn tag() -> Design {
    let mut design = closed(&[(2.5, 2.5), (8.0, 2.5), (13.5, 8.0), (8.0, 13.5), (2.5, 8.0)]);
    let _ = design.fills.extend(&Path::circle(Point::new(5.5, 5.5), 1.1));
    design
}

/// A cloud: a flat bottom and three bumps.
fn cloud() -> Design {
    let mut strokes = Path::new();
    let _ = strokes
        .move_to(Point::new(4.5, 12.5))
        .line_to(Point::new(12.0, 12.5))
        .cubic_to(Point::new(15.0, 12.5), Point::new(15.0, 8.0), Point::new(12.0, 8.0))
        .cubic_to(Point::new(11.5, 4.0), Point::new(6.5, 3.5), Point::new(5.5, 7.5))
        .cubic_to(Point::new(2.0, 7.5), Point::new(1.5, 12.5), Point::new(4.5, 12.5))
        .close();
    Design {
        strokes,
        fills: Path::new(),
    }
}

/// A commit: a ring on a line.
fn commit() -> Design {
    let mut design = lines(&[&[(0.5, 7.5), (4.5, 7.5)], &[(10.5, 7.5), (14.5, 7.5)]]);
    let _ = design.strokes.extend(&Path::circle(Point::new(7.5, 7.5), 3.0));
    design
}

/// A file: a page with its top right corner folded.
fn file() -> Design {
    let mut design = closed(&[(3.5, 1.5), (9.5, 1.5), (12.5, 4.5), (12.5, 14.5), (3.5, 14.5)]);
    add_polyline(&mut design.strokes, &[(9.5, 1.5), (9.5, 4.5), (12.5, 4.5)]);
    design
}

/// A circular arrow: three quarters of a circle, clockwise from the right, ending at the top in a head.
fn refresh() -> Design {
    let (center, radius) = (Point::new(8.0, 8.5), 5.0);
    let handle = radius * 0.552_284_8;
    let at = |x: f32, y: f32| Point::new(center.x + x, center.y + y);
    let mut strokes = Path::new();
    let _ = strokes
        .move_to(at(radius, 0.0))
        .cubic_to(at(radius, handle), at(handle, radius), at(0.0, radius))
        .cubic_to(at(-handle, radius), at(-radius, handle), at(-radius, 0.0))
        .cubic_to(at(-radius, -handle), at(-handle, -radius), at(0.0, -radius));
    add_polyline(&mut strokes, &[(6.0, 1.0), (8.5, 3.5), (6.0, 6.0)]);
    Design {
        strokes,
        fills: Path::new(),
    }
}

/// A gear: a ring with a hole, and eight teeth around it.
fn gear() -> Design {
    let center = Point::new(8.0, 8.0);
    let mut strokes = Path::new();
    let _ = strokes
        .extend(&Path::circle(center, 4.0))
        .extend(&Path::circle(center, 1.75));
    let directions = [(1.0, 0.0), (DIAGONAL, DIAGONAL), (0.0, 1.0), (-DIAGONAL, DIAGONAL)];
    for (x, y) in directions {
        let direction = Point::new(x, y);
        for sign in [1.0, -1.0] {
            let _ = strokes
                .move_to(center + direction * (4.0 * sign))
                .line_to(center + direction * (6.5 * sign));
        }
    }
    Design {
        strokes,
        fills: Path::new(),
    }
}

/// A warning: a triangle with an exclamation mark.
fn warning() -> Design {
    let mut design = closed(&[(8.0, 1.5), (14.5, 13.5), (1.5, 13.5)]);
    add_polyline(&mut design.strokes, &[(8.0, 5.5), (8.0, 9.0)]);
    let _ = design.fills.extend(&Path::circle(Point::new(8.0, 11.25), 0.85));
    design
}

/// A worktree: a trunk with two lines growing from it, each to a ring of its own.
fn worktree() -> Design {
    let mut design = lines(&[&[(8.0, 4.5), (8.0, 14.5)]]);
    let strokes = &mut design.strokes;
    let _ = strokes
        .move_to(Point::new(3.5, 8.0))
        .cubic_to(Point::new(3.5, 10.5), Point::new(8.0, 10.0), Point::new(8.0, 12.5))
        .move_to(Point::new(12.5, 8.0))
        .cubic_to(Point::new(12.5, 10.5), Point::new(8.0, 10.0), Point::new(8.0, 12.5));
    for (x, y) in [(8.0, 3.0), (3.5, 6.5), (12.5, 6.5)] {
        let _ = strokes.extend(&Path::circle(Point::new(x, y), 1.5));
    }
    design
}

/// A stash: a tray with two layers above it.
fn stash() -> Design {
    let mut design = closed(&[
        (1.5, 9.5),
        (5.0, 9.5),
        (6.0, 11.0),
        (10.0, 11.0),
        (11.0, 9.5),
        (14.5, 9.5),
        (14.5, 14.0),
        (1.5, 14.0),
    ]);
    add_polyline(&mut design.strokes, &[(3.0, 6.5), (13.0, 6.5)]);
    add_polyline(&mut design.strokes, &[(4.5, 3.5), (11.5, 3.5)]);
    design
}

/// A star with five points, its outline through the points and the corners between them.
fn star() -> Design {
    let center = Point::new(8.0, 8.5);
    let points: Vec<(f32, f32)> = (0..10_u8)
        .map(|index| {
            let radius = if index % 2 == 0 { 6.5 } else { 2.75 };
            let angle = std::f32::consts::PI * (f32::from(index) / 5.0 - 0.5);
            (center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect();
    closed(&points)
}

/// An eye: two curves that meet at its corners, and a ring for its middle.
fn eye() -> Design {
    let mut strokes = Path::new();
    let _ = strokes
        .move_to(Point::new(1.5, 8.0))
        .cubic_to(Point::new(4.5, 3.0), Point::new(11.5, 3.0), Point::new(14.5, 8.0))
        .cubic_to(Point::new(11.5, 13.0), Point::new(4.5, 13.0), Point::new(1.5, 8.0))
        .close();
    let _ = strokes.extend(&Path::circle(Point::new(8.0, 8.0), 2.0));
    Design {
        strokes,
        fills: Path::new(),
    }
}
