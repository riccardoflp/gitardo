//! The canvas: what a widget draws on, in logical pixels, with a clip and an offset it can save and restore.

mod shapes;

pub use shapes::Border;

use crate::assets::Assets;
use crate::color::Color;
use crate::frame::FrameBuffer;
use crate::geometry::{PixelRect, Point, Rect, Transform};
use crate::mask::{Outline, rasterize};
use crate::number;
use crate::path::{FillRule, LineCap, Path};

/// How many pixels beyond the frame a shape is rasterized, so that the edge of what is rasterized changes none of the
/// pixels of the frame.
const RASTER_MARGIN: i32 = 4;

/// How a path is stroked: the width of the line, in logical pixels, how its ends look, and its color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    /// The width of the line, in logical pixels.
    pub width: f32,
    /// How the ends of open contours look.
    pub cap: LineCap,
    /// The color of the line.
    pub color: Color,
}

/// The clip and the offset a canvas draws with, saved and restored together.
#[derive(Clone, Copy, Debug, PartialEq)]
struct State {
    /// The pixels drawing may change, in physical pixels of the frame.
    clip: PixelRect,
    /// Where the origin of the logical coordinates is, in logical pixels from the top left of the frame.
    offset: Point,
}

/// A frame buffer to draw on, for one frame, with the assets drawing keeps between frames.
///
/// Widgets draw in logical pixels; the canvas multiplies by the scale of the screen (2 on a screen of 200 %) when it
/// draws, so that layout never sees physical pixels. Rectangles are snapped to whole physical pixels, so that their
/// sides are sharp; paths, circles, text and icons are anti-aliased. Drawing changes only the pixels inside the clip,
/// and every pixel it changes is marked in the frame's dirty regions.
///
/// A widget draws by saving the state, moving the origin to its top left and clipping to its bounds, drawing, and
/// restoring:
///
/// ```
/// # use render::{Assets, Canvas, Color, FrameBuffer, Rect};
/// let mut frame = FrameBuffer::new(200, 100);
/// let mut assets = Assets::new();
/// let mut canvas = Canvas::new(&mut frame, &mut assets, 2.0);
/// canvas.save();
/// canvas.translate(10.0, 10.0);
/// canvas.clip(Rect::new(0.0, 0.0, 50.0, 20.0));
/// canvas.fill_rect(Rect::new(0.0, 0.0, 80.0, 20.0), Color::WHITE);
/// canvas.restore();
/// assert_eq!(frame.pixel(119, 30), Some(Color::WHITE));
/// assert_eq!(frame.pixel(120, 30), Some(Color::BLACK));
/// ```
#[derive(Debug)]
pub struct Canvas<'frame> {
    /// The pixels drawn on.
    pub(crate) frame: &'frame mut FrameBuffer,
    /// The fonts, glyphs and icons.
    pub(crate) assets: &'frame mut Assets,
    /// Physical pixels for each logical pixel.
    scale: f32,
    /// The clip and offset in force.
    state: State,
    /// The states saved, the last saved last.
    saved: Vec<State>,
}

impl<'frame> Canvas<'frame> {
    /// A canvas over a whole frame, at a scale of the screen: physical pixels for each logical pixel, such as 1.5 on
    /// a screen of 150 %. A scale that is not a positive number counts as 1.
    pub fn new(frame: &'frame mut FrameBuffer, assets: &'frame mut Assets, scale: f32) -> Canvas<'frame> {
        let clip = frame.bounds();
        Canvas {
            frame,
            assets,
            scale: if scale.is_finite() && scale > 0.0 { scale } else { 1.0 },
            state: State {
                clip,
                offset: Point::ZERO,
            },
            saved: Vec::new(),
        }
    }

    /// Physical pixels for each logical pixel.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The fonts, glyphs and icons, to measure text while drawing.
    pub fn assets(&mut self) -> &mut Assets {
        self.assets
    }

    /// Saves the clip and the offset, to be restored by [`Canvas::restore`].
    pub fn save(&mut self) {
        self.saved.push(self.state);
    }

    /// Restores the clip and the offset last saved; does nothing when none is saved.
    pub fn restore(&mut self) {
        if let Some(state) = self.saved.pop() {
            self.state = state;
        }
    }

    /// Moves the origin of the logical coordinates by a vector.
    pub fn translate(&mut self, x: f32, y: f32) {
        self.state.offset = self.state.offset + Point::new(x, y);
    }

    /// Limits drawing to a rectangle, inside the clip already in force. Its sides are snapped to whole physical pixels.
    pub fn clip(&mut self, rect: Rect) {
        self.state.clip = self.state.clip.intersection(self.snap(rect));
    }

    /// The rectangle drawing is limited to, in logical pixels from the current origin.
    #[must_use]
    pub fn clip_bounds(&self) -> Rect {
        let clip = self.state.clip;
        let to_logical = |value: i32| number::float(value) / self.scale;
        Rect::new(
            to_logical(clip.left) - self.state.offset.x,
            to_logical(clip.top) - self.state.offset.y,
            to_logical(clip.right - clip.left),
            to_logical(clip.bottom - clip.top),
        )
    }

    /// Whether anything drawn in a rectangle could be seen: rows and widgets outside the clip need not be drawn.
    #[must_use]
    pub fn is_visible(&self, rect: Rect) -> bool {
        !PixelRect::enclosing(self.to_physical(rect))
            .intersection(self.state.clip)
            .is_empty()
    }

    /// Fills the whole clip with a color, laid over what is there when it is not opaque.
    pub fn clear(&mut self, color: Color) {
        self.frame.fill_rect(self.state.clip, color);
    }

    /// Fills a rectangle with a color, its sides snapped to whole physical pixels.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let snapped = self.snap(rect).intersection(self.state.clip);
        self.frame.fill_rect(snapped, color);
    }

    /// Fills a path with a color, anti-aliased, by a fill rule.
    pub fn fill_path(&mut self, path: &Path, color: Color, rule: FillRule) {
        let physical = path.transformed(self.transform());
        if let Some(outline) = Outline::fill(&physical, rule) {
            self.fill_outline_within(&outline, self.frame.bounds(), color);
        }
    }

    /// Strokes a path with a line of a width, anti-aliased.
    pub fn stroke_path(&mut self, path: &Path, stroke: &Stroke) {
        let physical = path.transformed(self.transform());
        if let Some(outline) = Outline::stroke(&physical, stroke.width * self.scale, stroke.cap) {
            self.fill_outline_within(&outline, self.frame.bounds(), stroke.color);
        }
    }

    /// A straight line from one point to another.
    pub fn line(&mut self, from: Point, to: Point, stroke: &Stroke) {
        let mut path = Path::new();
        let _ = path.move_to(from).line_to(to);
        self.stroke_path(&path, stroke);
    }

    /// Fills a circle.
    pub fn fill_circle(&mut self, center: Point, radius: f32, color: Color) {
        self.fill_path(&Path::circle(center, radius), color, FillRule::NonZero);
    }

    /// Strokes the outline of a circle, the line centered on the circle.
    pub fn stroke_circle(&mut self, center: Point, radius: f32, stroke: &Stroke) {
        self.stroke_path(&Path::circle(center, radius), stroke);
    }

    /// The transform from logical pixels from the current origin to physical pixels of the frame.
    pub(crate) fn transform(&self) -> Transform {
        Transform::scale(self.scale, self.scale).after(Transform::translation(self.state.offset.x, self.state.offset.y))
    }

    /// A point in logical pixels from the current origin, in physical pixels of the frame.
    pub(crate) fn to_physical_point(&self, point: Point) -> Point {
        (point + self.state.offset) * self.scale
    }

    /// A rectangle in logical pixels from the current origin, in physical pixels of the frame.
    pub(crate) fn to_physical(&self, rect: Rect) -> Rect {
        let origin = self.to_physical_point(Point::new(rect.x, rect.y));
        Rect::new(origin.x, origin.y, rect.width * self.scale, rect.height * self.scale)
    }

    /// A rectangle in logical pixels, in whole physical pixels: each side at the nearest pixel edge.
    pub(crate) fn snap(&self, rect: Rect) -> PixelRect {
        let physical = self.to_physical(rect);
        PixelRect::new(
            number::round(physical.x),
            number::round(physical.y),
            number::round(physical.right()),
            number::round(physical.bottom()),
        )
    }

    /// The clip in force, in physical pixels.
    pub(crate) fn physical_clip(&self) -> PixelRect {
        self.state.clip
    }

    /// Fills a shape in physical pixels with a color, only inside a rectangle of the frame and the clip.
    ///
    /// The shape is rasterized inside that rectangle whatever the clip, and a few pixels beyond the frame: the
    /// rasterizer treats the edge of what it rasterizes as a clip, which changes the coverage of the pixels near it,
    /// so rasterizing only what the clip shows would give a frame drawn in parts other pixels than the frame drawn at
    /// once.
    pub(crate) fn fill_outline_within(&mut self, outline: &Outline, within: PixelRect, color: Color) {
        if color.is_transparent() {
            return;
        }
        let rasterized = outline.bounds().intersection(within);
        let shown = rasterized.intersection(self.state.clip);
        if shown.is_empty() {
            return;
        }
        let frame = self.frame.bounds();
        let around_frame = PixelRect::new(
            frame.left - RASTER_MARGIN,
            frame.top - RASTER_MARGIN,
            frame.right + RASTER_MARGIN,
            frame.bottom + RASTER_MARGIN,
        );
        if let Some(mask) = rasterize(std::slice::from_ref(outline), rasterized.intersection(around_frame)) {
            self.frame.fill_mask(mask.view(), color, shown);
        }
    }
}
