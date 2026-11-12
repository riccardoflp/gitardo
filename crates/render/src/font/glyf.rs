//! The outlines of glyphs, from `loca` and `glyf`: simple glyphs, contours of points on and off the curve joined by
//! quadratic Bézier curves, and composite glyphs, made of other glyphs placed by a transform.

use super::GlyphId;
use super::reader::Reader;
use super::tables::LocationFormat;
use crate::geometry::{Point, Transform};
use crate::path::Path;

/// The flag of a point that is on the curve; the others are control points.
const ON_CURVE: u8 = 0x01;
/// The flag of an x coordinate written in one byte.
const X_SHORT: u8 = 0x02;
/// The flag of a y coordinate written in one byte.
const Y_SHORT: u8 = 0x04;
/// The flag that says the next byte is how many times this flag repeats.
const REPEAT: u8 = 0x08;
/// With [`X_SHORT`], the byte is positive; without it, the x coordinate is the same as the point before.
const X_SAME_OR_POSITIVE: u8 = 0x10;
/// With [`Y_SHORT`], the byte is positive; without it, the y coordinate is the same as the point before.
const Y_SAME_OR_POSITIVE: u8 = 0x20;

/// A component's two arguments are 16-bit numbers, not bytes.
const ARGUMENTS_ARE_WORDS: u16 = 0x0001;
/// A component's arguments are an offset, not the numbers of two points to match.
const ARGUMENTS_ARE_OFFSETS: u16 = 0x0002;
/// A component has one scale for both axes.
const HAS_SCALE: u16 = 0x0008;
/// Another component follows this one.
const MORE_COMPONENTS: u16 = 0x0020;
/// A component has a scale for each axis.
const HAS_AXIS_SCALES: u16 = 0x0040;
/// A component has a 2 × 2 matrix.
const HAS_MATRIX: u16 = 0x0080;
/// A component's offset is scaled by its transform, as Apple's fonts do.
const SCALED_OFFSET: u16 = 0x0800;
/// A component's offset is not scaled by its transform, which is the default of Microsoft's fonts.
const UNSCALED_OFFSET: u16 = 0x1000;

/// How deep composite glyphs may nest, so that a damaged font whose glyphs refer to each other ends.
const DEEPEST_NESTING: u32 = 8;

/// Where the outlines of a font are.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Source<'data> {
    /// The `loca` table.
    pub(crate) locations: Reader<'data>,
    /// How the offsets of `loca` are written.
    pub(crate) format: LocationFormat,
    /// The `glyf` table.
    pub(crate) glyphs: Reader<'data>,
    /// The number of glyphs of the font.
    pub(crate) glyph_count: u16,
}

impl<'data> Source<'data> {
    /// The outline of a glyph in font units.
    pub(crate) fn outline(self, glyph: GlyphId) -> Option<Path> {
        let mut path = Path::new();
        self.add_glyph(&mut path, glyph, Transform::IDENTITY, 0)?;
        Some(path)
    }

    /// The data of a glyph in `glyf`, or nothing when it has no outline.
    fn data(self, glyph: GlyphId) -> Option<Reader<'data>> {
        if glyph.0 >= self.glyph_count {
            return None;
        }
        let index = usize::from(glyph.0);
        let (start, end) = match self.format {
            LocationFormat::Short => (
                self.locations.offset16(index * 2)? * 2,
                self.locations.offset16(index * 2 + 2)? * 2,
            ),
            LocationFormat::Long => (
                self.locations.offset32(index * 4)?,
                self.locations.offset32(index * 4 + 4)?,
            ),
        };
        if end <= start {
            return None;
        }
        self.glyphs.slice(start, end - start)
    }

    /// Adds the outline of a glyph, moved by a transform, to a path. A glyph with no outline adds nothing; one that
    /// is damaged adds what can be read of it and gives nothing.
    fn add_glyph(self, path: &mut Path, glyph: GlyphId, transform: Transform, depth: u32) -> Option<()> {
        let Some(data) = self.data(glyph) else {
            return Some(());
        };
        let contour_count = data.i16(0)?;
        if contour_count >= 0 {
            add_simple(path, data, usize::try_from(contour_count).ok()?, transform)
        } else if depth < DEEPEST_NESTING {
            self.add_composite(path, data, transform, depth)
        } else {
            None
        }
    }

    /// Adds every component of a composite glyph.
    fn add_composite(self, path: &mut Path, data: Reader<'_>, transform: Transform, depth: u32) -> Option<()> {
        let mut at = 10;
        loop {
            let flags = data.u16(at)?;
            let component = GlyphId(data.u16(at + 2)?);
            at += 4;
            let (offset, length) = component_offset(data, at, flags)?;
            at += length;
            let (matrix, length) = component_matrix(data, at, flags)?;
            at += length;
            let is_offset_scaled = flags & SCALED_OFFSET != 0 && flags & UNSCALED_OFFSET == 0;
            let translation = if is_offset_scaled { matrix.apply(offset) } else { offset };
            let placed = Transform { translation, ..matrix };
            self.add_glyph(path, component, transform.after(placed), depth + 1)?;
            if flags & MORE_COMPONENTS == 0 {
                return Some(());
            }
        }
    }
}

/// The offset of a component, and the bytes its arguments take. Arguments that name points to match, rather than an
/// offset, are read as no offset: such components are rare and the glyph is still drawn, a little out of place.
fn component_offset(data: Reader<'_>, at: usize, flags: u16) -> Option<(Point, usize)> {
    let (x, y, length) = if flags & ARGUMENTS_ARE_WORDS != 0 {
        (f32::from(data.i16(at)?), f32::from(data.i16(at + 2)?), 4)
    } else {
        (f32::from(data.i8(at)?), f32::from(data.i8(at + 1)?), 2)
    };
    let offset = if flags & ARGUMENTS_ARE_OFFSETS != 0 {
        Point::new(x, y)
    } else {
        Point::ZERO
    };
    Some((offset, length))
}

/// The transform of a component without its offset, and the bytes it takes: none, one scale, two scales, or a 2 × 2
/// matrix whose first column is where the x axis goes.
fn component_matrix(data: Reader<'_>, at: usize, flags: u16) -> Option<(Transform, usize)> {
    if flags & HAS_SCALE != 0 {
        let scale = data.f2dot14(at)?;
        Some((Transform::scale(scale, scale), 2))
    } else if flags & HAS_AXIS_SCALES != 0 {
        Some((Transform::scale(data.f2dot14(at)?, data.f2dot14(at + 2)?), 4))
    } else if flags & HAS_MATRIX != 0 {
        let transform = Transform {
            x_axis: Point::new(data.f2dot14(at)?, data.f2dot14(at + 2)?),
            y_axis: Point::new(data.f2dot14(at + 4)?, data.f2dot14(at + 6)?),
            translation: Point::ZERO,
        };
        Some((transform, 8))
    } else {
        Some((Transform::IDENTITY, 0))
    }
}

/// A point of a simple glyph: where it is, and whether it is on the curve.
#[derive(Clone, Copy, Debug, PartialEq)]
struct GlyphPoint {
    /// Where it is, in font units.
    position: Point,
    /// Whether the curve passes through it, rather than being pulled towards it.
    is_on_curve: bool,
}

/// Adds the contours of a simple glyph: the last point of each contour, its flags, then all the x coordinates and all
/// the y coordinates, each written as a change from the point before.
fn add_simple(path: &mut Path, data: Reader<'_>, contour_count: usize, transform: Transform) -> Option<()> {
    let mut ends = Vec::with_capacity(contour_count);
    for contour in 0..contour_count {
        ends.push(data.offset16(10 + contour * 2)?);
    }
    let point_count = ends.last().map_or(0, |last| last + 1);
    let instructions_at = 10 + contour_count * 2;
    let flags_at = instructions_at + 2 + data.offset16(instructions_at)?;
    let (flags, x_at) = read_flags(data, flags_at, point_count)?;
    let (xs, y_at) = read_coordinates(data, x_at, &flags, Axis::X)?;
    let (ys, _) = read_coordinates(data, y_at, &flags, Axis::Y)?;
    let mut start = 0;
    for end in ends {
        let points: Vec<GlyphPoint> = (start..=end)
            .filter_map(|index| {
                let position = Point::new(f32::from(*xs.get(index)?), f32::from(*ys.get(index)?));
                Some(GlyphPoint {
                    position: transform.apply(position),
                    is_on_curve: flags.get(index)? & ON_CURVE != 0,
                })
            })
            .collect();
        add_contour(path, &points);
        start = end + 1;
    }
    Some(())
}

/// The flags of every point, with their repeats spelt out, and where the x coordinates start.
fn read_flags(data: Reader<'_>, mut at: usize, point_count: usize) -> Option<(Vec<u8>, usize)> {
    let mut flags = Vec::with_capacity(point_count);
    while flags.len() < point_count {
        let flag = data.u8(at)?;
        at += 1;
        flags.push(flag);
        if flag & REPEAT != 0 {
            let repeats = data.u8(at)?;
            at += 1;
            for _ in 0..repeats {
                flags.push(flag);
            }
        }
    }
    flags.truncate(point_count);
    Some((flags, at))
}

/// One of the two axes of the coordinates of a glyph, each with its own two flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    /// The horizontal axis.
    X,
    /// The vertical axis.
    Y,
}

/// The coordinates of every point along one axis, and where the bytes after them start.
fn read_coordinates(data: Reader<'_>, mut at: usize, flags: &[u8], axis: Axis) -> Option<(Vec<i16>, usize)> {
    let (short, same_or_positive) = match axis {
        Axis::X => (X_SHORT, X_SAME_OR_POSITIVE),
        Axis::Y => (Y_SHORT, Y_SAME_OR_POSITIVE),
    };
    let mut value: i16 = 0;
    let mut coordinates = Vec::with_capacity(flags.len());
    for flag in flags {
        let change = if flag & short != 0 {
            let byte = i16::from(data.u8(at)?);
            at += 1;
            if flag & same_or_positive != 0 { byte } else { -byte }
        } else if flag & same_or_positive != 0 {
            0
        } else {
            let word = data.i16(at)?;
            at += 2;
            word
        };
        value = value.wrapping_add(change);
        coordinates.push(value);
    }
    Some((coordinates, at))
}

/// Adds one contour of points on and off the curve. Two control points in a row have an implied point on the curve
/// halfway between them; a contour that starts off the curve starts at its last point when that one is on the curve,
/// or halfway between its last and first points when it is not.
fn add_contour(path: &mut Path, points: &[GlyphPoint]) {
    let (Some(first), Some(last)) = (points.first().copied(), points.last().copied()) else {
        return;
    };
    let (start, rest): (Point, &[GlyphPoint]) = if first.is_on_curve {
        (first.position, points.get(1..).unwrap_or_default())
    } else if last.is_on_curve {
        (last.position, points.get(..points.len() - 1).unwrap_or_default())
    } else {
        (first.position.midpoint(last.position), points)
    };
    let _ = path.move_to(start);
    let mut control: Option<Point> = None;
    for point in rest {
        match (control, point.is_on_curve) {
            (Some(pending), true) => {
                let _ = path.quadratic_to(pending, point.position);
                control = None;
            }
            (None, true) => {
                let _ = path.line_to(point.position);
            }
            (Some(pending), false) => {
                let _ = path.quadratic_to(pending, pending.midpoint(point.position));
                control = Some(point.position);
            }
            (None, false) => control = Some(point.position),
        }
    }
    if let Some(pending) = control {
        let _ = path.quadratic_to(pending, start);
    }
    let _ = path.close();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Command;

    fn point(x: f32, y: f32, is_on_curve: bool) -> GlyphPoint {
        GlyphPoint {
            position: Point::new(x, y),
            is_on_curve,
        }
    }

    #[test]
    fn control_points_in_a_row_imply_a_point_between_them() {
        let mut path = Path::new();
        add_contour(
            &mut path,
            &[point(0.0, 0.0, true), point(10.0, 0.0, false), point(10.0, 10.0, false)],
        );
        assert_eq!(
            path.commands(),
            &[
                Command::MoveTo(Point::new(0.0, 0.0)),
                Command::QuadraticTo(Point::new(10.0, 0.0), Point::new(10.0, 5.0)),
                Command::QuadraticTo(Point::new(10.0, 10.0), Point::new(0.0, 0.0)),
                Command::Close,
            ]
        );
    }

    #[test]
    fn a_contour_of_control_points_only_starts_halfway_between_its_ends() {
        let mut path = Path::new();
        add_contour(&mut path, &[point(0.0, 0.0, false), point(4.0, 0.0, false)]);
        assert_eq!(path.commands().first(), Some(&Command::MoveTo(Point::new(2.0, 0.0))));
        assert_eq!(path.commands().len(), 4);
    }

    #[test]
    fn coordinates_are_changes_from_the_point_before() {
        let data = [5, 2, 0xFF, 0xFE];
        let flags = [X_SHORT | X_SAME_OR_POSITIVE, X_SAME_OR_POSITIVE, X_SHORT, 0];
        let (coordinates, end) = read_coordinates(Reader::new(&data), 0, &flags, Axis::X).unwrap();
        assert_eq!(coordinates, vec![5, 5, 3, 1]);
        assert_eq!(end, 4);
    }
}
