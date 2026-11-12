//! Layout: rectangles cut from the rectangle of their parent, explicitly, with no solver. A view takes the rectangle it
//! is given, cuts a bar from its top, a column from its left, splits the rest in rows or columns of fixed, fractional
//! or filling sizes, and hands each piece to a widget.

use render::Rect;

/// The size of one piece of a row or a column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
    /// A number of logical pixels.
    Fixed(f32),
    /// A part of the whole length, from 0 to 1, gaps included.
    Fraction(f32),
    /// A share of what the fixed and fractional pieces leave: pieces of weight 2 get twice what pieces of weight 1
    /// get.
    Fill(f32),
}

/// The distances kept free inside the sides of a rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    /// Inside the left side.
    pub left: f32,
    /// Inside the top side.
    pub top: f32,
    /// Inside the right side.
    pub right: f32,
    /// Inside the bottom side.
    pub bottom: f32,
}

impl Insets {
    /// The same distance inside every side.
    #[must_use]
    pub fn all(distance: f32) -> Insets {
        Insets {
            left: distance,
            top: distance,
            right: distance,
            bottom: distance,
        }
    }

    /// One distance inside the left and right sides, another inside the top and bottom ones.
    #[must_use]
    pub fn symmetric(across: f32, down: f32) -> Insets {
        Insets {
            left: across,
            top: down,
            right: across,
            bottom: down,
        }
    }
}

/// Where something smaller is placed inside a rectangle, along one axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Against the left or top side.
    #[default]
    Start,
    /// In the middle.
    Center,
    /// Against the right or bottom side.
    End,
}

/// A rectangle to cut pieces from: each cut takes a piece from one side and leaves the rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    /// What is left.
    rest: Rect,
    /// The gap left after each cut.
    gap: f32,
}

impl Area {
    /// The whole of a rectangle, cut with no gap.
    #[must_use]
    pub fn new(rect: Rect) -> Area {
        Area { rest: rect, gap: 0.0 }
    }

    /// The whole of a rectangle, with a gap left between the pieces cut from it.
    #[must_use]
    pub fn with_gap(rect: Rect, gap: f32) -> Area {
        Area { rest: rect, gap }
    }

    /// Cuts a piece of a height from the top.
    pub fn top(&mut self, height: f32) -> Rect {
        let height = height.clamp(0.0, self.rest.height);
        let piece = Rect::new(self.rest.x, self.rest.y, self.rest.width, height);
        let used = (height + self.gap).min(self.rest.height);
        self.rest = Rect::new(
            self.rest.x,
            self.rest.y + used,
            self.rest.width,
            self.rest.height - used,
        );
        piece
    }

    /// Cuts a piece of a height from the bottom.
    pub fn bottom(&mut self, height: f32) -> Rect {
        let height = height.clamp(0.0, self.rest.height);
        let piece = Rect::new(self.rest.x, self.rest.bottom() - height, self.rest.width, height);
        let used = (height + self.gap).min(self.rest.height);
        self.rest = Rect::new(self.rest.x, self.rest.y, self.rest.width, self.rest.height - used);
        piece
    }

    /// Cuts a piece of a width from the left.
    pub fn left(&mut self, width: f32) -> Rect {
        let width = width.clamp(0.0, self.rest.width);
        let piece = Rect::new(self.rest.x, self.rest.y, width, self.rest.height);
        let used = (width + self.gap).min(self.rest.width);
        self.rest = Rect::new(
            self.rest.x + used,
            self.rest.y,
            self.rest.width - used,
            self.rest.height,
        );
        piece
    }

    /// Cuts a piece of a width from the right.
    pub fn right(&mut self, width: f32) -> Rect {
        let width = width.clamp(0.0, self.rest.width);
        let piece = Rect::new(self.rest.right() - width, self.rest.y, width, self.rest.height);
        let used = (width + self.gap).min(self.rest.width);
        self.rest = Rect::new(self.rest.x, self.rest.y, self.rest.width - used, self.rest.height);
        piece
    }

    /// What is left after the cuts.
    #[must_use]
    pub fn rest(self) -> Rect {
        self.rest
    }
}

/// A rectangle made smaller by insets; never smaller than nothing.
#[must_use]
pub fn inset(rect: Rect, insets: Insets) -> Rect {
    Rect::new(
        rect.x + insets.left,
        rect.y + insets.top,
        (rect.width - insets.left - insets.right).max(0.0),
        (rect.height - insets.top - insets.bottom).max(0.0),
    )
}

/// The lengths of pieces along a length, with gaps between them: fixed pieces first, then fractions of the whole,
/// then the fills share what is left. Pieces never go below zero.
#[must_use]
pub fn lengths(total: f32, gap: f32, sizes: &[Size]) -> Vec<f32> {
    let gaps = gap * count_as_float(sizes.len().saturating_sub(1));
    let available = (total - gaps).max(0.0);
    let mut used = 0.0;
    let mut weights = 0.0;
    for size in sizes {
        match *size {
            Size::Fixed(length) => used += length.max(0.0),
            Size::Fraction(part) => used += (part * total).max(0.0),
            Size::Fill(weight) => weights += weight.max(0.0),
        }
    }
    let left = (available - used).max(0.0);
    sizes
        .iter()
        .map(|size| match *size {
            Size::Fixed(length) => length.max(0.0),
            Size::Fraction(part) => (part * total).max(0.0),
            Size::Fill(weight) if weights > 0.0 => left * weight.max(0.0) / weights,
            Size::Fill(_) => 0.0,
        })
        .collect()
}

/// A rectangle split into columns, left to right, with gaps between them.
#[must_use]
pub fn columns<const N: usize>(rect: Rect, gap: f32, sizes: [Size; N]) -> [Rect; N] {
    let lengths = lengths(rect.width, gap, &sizes);
    let mut x = rect.x;
    std::array::from_fn(|index| {
        let width = lengths.get(index).copied().unwrap_or(0.0);
        let piece = Rect::new(x, rect.y, width, rect.height);
        x += width + gap;
        piece
    })
}

/// A rectangle split into rows, top to bottom, with gaps between them.
#[must_use]
pub fn rows<const N: usize>(rect: Rect, gap: f32, sizes: [Size; N]) -> [Rect; N] {
    let lengths = lengths(rect.height, gap, &sizes);
    let mut y = rect.y;
    std::array::from_fn(|index| {
        let height = lengths.get(index).copied().unwrap_or(0.0);
        let piece = Rect::new(rect.x, y, rect.width, height);
        y += height + gap;
        piece
    })
}

/// A rectangle of a size placed inside another, by an alignment across and one down. A size larger than the outer
/// rectangle is cut to it.
#[must_use]
pub fn place(outer: Rect, (width, height): (f32, f32), (across, down): (Align, Align)) -> Rect {
    let width = width.min(outer.width).max(0.0);
    let height = height.min(outer.height).max(0.0);
    let offset = |align: Align, free: f32| match align {
        Align::Start => 0.0,
        Align::Center => (free * 0.5).round(),
        Align::End => free,
    };
    Rect::new(
        outer.x + offset(across, outer.width - width),
        outer.y + offset(down, outer.height - height),
        width,
        height,
    )
}

/// A count as a float, for lengths: counts of pieces are far below where `f32` stops being exact.
fn count_as_float(count: usize) -> f32 {
    crate::number::float_of_count(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_and_fractional_pieces_come_first_and_fills_share_the_rest() {
        assert_eq!(
            lengths(130.0, 5.0, &[Size::Fixed(30.0), Size::Fill(1.0), Size::Fill(3.0)]),
            vec![30.0, 22.5, 67.5]
        );
        assert_eq!(
            lengths(100.0, 0.0, &[Size::Fraction(0.25), Size::Fill(1.0)]),
            vec![25.0, 75.0]
        );
        assert_eq!(
            lengths(10.0, 0.0, &[Size::Fixed(30.0), Size::Fill(1.0)]),
            vec![30.0, 0.0]
        );
    }

    #[test]
    fn columns_and_rows_follow_each_other_with_their_gaps() {
        let [left, right] = columns(
            Rect::new(10.0, 0.0, 100.0, 20.0),
            10.0,
            [Size::Fixed(30.0), Size::Fill(1.0)],
        );
        assert_eq!(left, Rect::new(10.0, 0.0, 30.0, 20.0));
        assert_eq!(right, Rect::new(50.0, 0.0, 60.0, 20.0));
        let [top, bottom] = rows(
            Rect::new(0.0, 0.0, 10.0, 100.0),
            0.0,
            [Size::Fill(1.0), Size::Fill(1.0)],
        );
        assert_eq!((top.y, bottom.y, bottom.height), (0.0, 50.0, 50.0));
    }

    #[test]
    fn an_area_is_cut_from_its_sides_and_never_below_nothing() {
        let mut area = Area::with_gap(Rect::new(0.0, 0.0, 100.0, 50.0), 4.0);
        assert_eq!(area.top(10.0), Rect::new(0.0, 0.0, 100.0, 10.0));
        assert_eq!(area.left(20.0), Rect::new(0.0, 14.0, 20.0, 36.0));
        assert_eq!(area.right(200.0), Rect::new(24.0, 14.0, 76.0, 36.0));
        assert!(area.rest().is_empty());
        let placed = place(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            (40.0, 10.0),
            (Align::Center, Align::End),
        );
        assert_eq!(placed, Rect::new(30.0, 10.0, 40.0, 10.0));
    }
}
