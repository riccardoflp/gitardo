//! Sizes, points and rectangles: in logical pixels for layout, in physical pixels for the screen and the frame.

use crate::convert::{round_to_pixels, to_float};

/// A size in logical pixels: what layout works in, the same on every screen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalSize {
    /// The width.
    pub width: f32,
    /// The height.
    pub height: f32,
}

/// A size in physical pixels: the pixels of the screen and of a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhysicalSize {
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
}

/// A point in logical pixels, from the top left corner of the inside of a window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// The distance from the left edge.
    pub x: f32,
    /// The distance from the top edge.
    pub y: f32,
}

/// A position on the screen in physical pixels, from the top left corner of the main screen: where a window is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ScreenPosition {
    /// The distance from the left edge of the main screen; negative on a screen to its left.
    pub x: i32,
    /// The distance from the top edge of the main screen; negative on a screen above it.
    pub y: i32,
}

/// A rectangle of physical pixels in a frame, such as the part of it that changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PixelRect {
    /// The column of its left edge.
    pub x: u32,
    /// The row of its top edge.
    pub y: u32,
    /// How many columns it covers.
    pub width: u32,
    /// How many rows it covers.
    pub height: u32,
}

impl PixelRect {
    /// The rectangle that covers a whole frame of this size.
    #[must_use]
    pub fn covering(size: PhysicalSize) -> PixelRect {
        PixelRect {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        }
    }

    /// Whether it covers no pixel.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// The column just past its right edge.
    #[must_use]
    pub fn right(self) -> u32 {
        self.x.saturating_add(self.width)
    }

    /// The row just past its bottom edge.
    #[must_use]
    pub fn bottom(self) -> u32 {
        self.y.saturating_add(self.height)
    }

    /// The pixels it shares with another rectangle, if any.
    #[must_use]
    pub fn intersection(self, other: PixelRect) -> Option<PixelRect> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        let shared = PixelRect {
            x: left,
            y: top,
            width: right.saturating_sub(left),
            height: bottom.saturating_sub(top),
        };
        (!shared.is_empty()).then_some(shared)
    }

    /// The part of it inside a frame of this size, if any.
    #[must_use]
    pub fn clipped(self, size: PhysicalSize) -> Option<PixelRect> {
        self.intersection(PixelRect::covering(size))
    }
}

/// How many physical pixels a logical pixel is, kept as the dots per inch of the screen: 96 is a scale of 1, 192 a
/// scale of 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scale {
    dpi: u32,
}

impl Default for Scale {
    fn default() -> Scale {
        Scale {
            dpi: Scale::STANDARD_DPI,
        }
    }
}

impl Scale {
    /// The dots per inch of a scale of 1.
    pub const STANDARD_DPI: u32 = 96;

    /// The scale of a screen of so many dots per inch; a screen that says 0 is taken as standard.
    #[must_use]
    pub fn from_dpi(dpi: u32) -> Scale {
        Scale {
            dpi: if dpi == 0 { Scale::STANDARD_DPI } else { dpi },
        }
    }

    /// The dots per inch of the screen.
    #[must_use]
    pub fn dpi(self) -> u32 {
        self.dpi
    }

    /// How many physical pixels a logical pixel is: 1.0, 1.25, 1.5, 2.0…
    #[must_use]
    pub fn factor(self) -> f32 {
        to_float(self.dpi) / to_float(Scale::STANDARD_DPI)
    }

    /// A logical size in physical pixels, rounded to the nearest pixel.
    #[must_use]
    pub fn to_physical(self, size: LogicalSize) -> PhysicalSize {
        PhysicalSize {
            width: round_to_pixels(size.width * self.factor()),
            height: round_to_pixels(size.height * self.factor()),
        }
    }

    /// A physical size in logical pixels.
    #[must_use]
    pub fn to_logical(self, size: PhysicalSize) -> LogicalSize {
        LogicalSize {
            width: to_float(size.width) / self.factor(),
            height: to_float(size.height) / self.factor(),
        }
    }

    /// The scale of a screen whose logical pixel is `factor` physical pixels, as `winit` gives it.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the dots per inch are rounded and clamped to the range of u32 first"
    )]
    pub(crate) fn from_factor(factor: f64) -> Scale {
        let dpi = (factor * f64::from(Scale::STANDARD_DPI)).round();
        if dpi.is_nan() || dpi < 1.0 {
            Scale::default()
        } else {
            Scale::from_dpi(dpi.min(f64::from(u32::MAX)) as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LogicalSize, PhysicalSize, PixelRect, Scale};

    fn rect(x: u32, y: u32, width: u32, height: u32) -> PixelRect {
        PixelRect { x, y, width, height }
    }

    #[test]
    fn clips_rectangles_to_the_frame() {
        let frame = PhysicalSize { width: 100, height: 50 };
        let cases = [
            (rect(0, 0, 10, 10), Some(rect(0, 0, 10, 10))),
            (rect(95, 45, 10, 10), Some(rect(95, 45, 5, 5))),
            (rect(100, 0, 10, 10), None),
            (rect(0, 50, 10, 10), None),
            (rect(10, 10, 0, 5), None),
            (rect(u32::MAX - 1, 0, 10, 10), None),
            (rect(0, 0, 1000, 1000), Some(rect(0, 0, 100, 50))),
        ];
        for (given, expected) in cases {
            assert_eq!(given.clipped(frame), expected, "{given:?}");
        }
    }

    #[test]
    fn intersects_rectangles() {
        assert_eq!(
            rect(0, 0, 10, 10).intersection(rect(5, 5, 10, 10)),
            Some(rect(5, 5, 5, 5))
        );
        assert_eq!(rect(0, 0, 10, 10).intersection(rect(10, 0, 10, 10)), None);
        assert_eq!(rect(2, 3, 4, 5).right(), 6);
        assert_eq!(rect(2, 3, 4, 5).bottom(), 8);
    }

    #[test]
    fn converts_between_logical_and_physical_pixels() {
        let cases = [
            (96, 1.0, 800, 600),
            (120, 1.25, 1000, 750),
            (144, 1.5, 1200, 900),
            (192, 2.0, 1600, 1200),
        ];
        for (dpi, factor, width, height) in cases {
            let scale = Scale::from_dpi(dpi);
            assert!((scale.factor() - factor).abs() < f32::EPSILON, "{dpi}");
            let physical = scale.to_physical(LogicalSize {
                width: 800.0,
                height: 600.0,
            });
            assert_eq!(physical, PhysicalSize { width, height });
            assert_eq!(
                scale.to_logical(physical),
                LogicalSize {
                    width: 800.0,
                    height: 600.0
                }
            );
        }
    }

    #[test]
    fn rounds_odd_scales_to_whole_pixels() {
        let scale = Scale::from_dpi(120);
        assert_eq!(
            scale.to_physical(LogicalSize {
                width: 3.0,
                height: 1.0
            }),
            PhysicalSize { width: 4, height: 1 }
        );
        assert_eq!(Scale::from_dpi(0), Scale::default());
        assert_eq!(Scale::from_factor(1.25), scale);
        assert_eq!(Scale::from_factor(2.0), Scale::from_dpi(192));
        assert_eq!(Scale::from_factor(f64::NAN), Scale::default());
        assert_eq!(Scale::from_factor(-1.0), Scale::default());
    }
}
