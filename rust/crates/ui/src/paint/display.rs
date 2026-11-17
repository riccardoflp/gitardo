//! What a frame draws, kept as a list before it is drawn: each shape with the clip it is drawn in and the part of the
//! window it can change. Keeping the list is what lets overlays be drawn above everything whatever the order they were
//! declared in, and lets a frame draw again only the parts of the window whose shapes changed.

use std::hash::{Hash, Hasher};

use render::{
    Border, Canvas, Color, Command as PathCommand, FileIcon, FileIconLook, FillRule, Icon, Image, LineCap, Path, Point,
    Rect, Stroke,
};
use render::{TextRun, TextStyle};

use crate::id::StableHasher;

/// Something to draw, in logical pixels from the top left of the window.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Shape {
    /// A rectangle filled with a color.
    Rect {
        /// Where.
        rect: Rect,
        /// The color.
        color: Color,
    },
    /// A rectangle with rounded corners filled with a color.
    RoundedRect {
        /// Where.
        rect: Rect,
        /// The radius of the corners.
        radius: f32,
        /// The color.
        color: Color,
    },
    /// A border inside a rectangle.
    Border {
        /// Where.
        rect: Rect,
        /// Its width, color and corners.
        border: Border,
    },
    /// A path filled with a color.
    FillPath {
        /// The outline.
        path: Path,
        /// The color.
        color: Color,
        /// Which parts of the outline are inside.
        rule: FillRule,
    },
    /// A path stroked with a line.
    StrokePath {
        /// The path.
        path: Path,
        /// The line.
        stroke: Stroke,
    },
    /// A line of text.
    Text {
        /// The text.
        text: String,
        /// Its typeface and size.
        style: TextStyle,
        /// Its color.
        color: Color,
        /// The top left corner of the line.
        origin: Point,
        /// The widest it may be before it is cut with an ellipsis.
        max_width: Option<f32>,
    },
    /// An icon of Lucide's.
    Icon {
        /// Which.
        icon: Icon,
        /// The top left corner.
        origin: Point,
        /// The side.
        size: f32,
        /// The color.
        color: Color,
    },
    /// An icon of a file or a folder, of Symbols (`file-view.file-icons`).
    FileIcon {
        /// Which.
        icon: FileIcon,
        /// The top left corner.
        origin: Point,
        /// The side.
        size: f32,
        /// Its colors and how opaque it is.
        look: FileIconLook,
    },
    /// An image scaled into a rectangle.
    Image {
        /// The image.
        image: Image,
        /// Where.
        rect: Rect,
    },
}

impl Shape {
    /// The same shape at a part of its opacity, out of 255.
    pub(crate) fn faded(mut self, opacity: u8) -> Shape {
        match &mut self {
            Shape::Rect { color, .. }
            | Shape::RoundedRect { color, .. }
            | Shape::FillPath { color, .. }
            | Shape::Text { color, .. }
            | Shape::Icon { color, .. } => *color = color.faded(opacity),
            Shape::Border { border, .. } => border.color = border.color.faded(opacity),
            Shape::StrokePath { stroke, .. } => stroke.color = stroke.color.faded(opacity),
            Shape::FileIcon { look, .. } => {
                let faded = u16::from(look.opacity) * u16::from(opacity) / 255;
                look.opacity = u8::try_from(faded).unwrap_or(u8::MAX);
            }
            // An image keeps its own colors: what is faded around it is the rows and the text that step back.
            Shape::Image { .. } => {}
        }
        self
    }
}

/// A shape with the clip it is drawn in and the part of the window it can change.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Command {
    /// The shape.
    pub(crate) shape: Shape,
    /// The clip in force when it was drawn.
    pub(crate) clip: Rect,
    /// The pixels it can change: its bounds inside the clip, a pixel wider for anti-aliasing.
    pub(crate) bounds: Rect,
}

impl Command {
    /// The hash of everything that decides the pixels this command draws.
    pub(crate) fn hash_value(&self) -> u64 {
        let mut hasher = StableHasher::new();
        hash_rect(&mut hasher, self.clip);
        hash_shape(&mut hasher, &self.shape);
        hasher.finish()
    }

    /// Draws the command on a canvas, inside its clip and a region of the window.
    pub(crate) fn draw(&self, canvas: &mut Canvas<'_>, region: Rect) {
        canvas.save();
        canvas.clip(self.clip.intersection(region));
        draw_shape(canvas, &self.shape);
        canvas.restore();
    }
}

/// Draws a shape on a canvas whose origin is the top left of the window.
fn draw_shape(canvas: &mut Canvas<'_>, shape: &Shape) {
    match shape {
        Shape::Rect { rect, color } => canvas.fill_rect(*rect, *color),
        Shape::RoundedRect { rect, radius, color } => canvas.fill_rounded_rect(*rect, *radius, *color),
        Shape::Border { rect, border } => canvas.stroke_border(*rect, border),
        Shape::FillPath { path, color, rule } => canvas.fill_path(path, *color, *rule),
        Shape::StrokePath { path, stroke } => canvas.stroke_path(path, stroke),
        Shape::Text {
            text,
            style,
            color,
            origin,
            max_width,
        } => {
            let _ = canvas.draw_text(&TextRun {
                text,
                style: *style,
                color: *color,
                origin: *origin,
                max_width: *max_width,
            });
        }
        Shape::Icon {
            icon,
            origin,
            size,
            color,
        } => canvas.draw_icon(*icon, *origin, *size, *color),
        Shape::FileIcon {
            icon,
            origin,
            size,
            look,
        } => canvas.draw_file_icon(*icon, *origin, *size, look),
        Shape::Image { image, rect } => canvas.draw_image(image, *rect),
    }
}

/// Adds a shape to a hash.
fn hash_shape(hasher: &mut StableHasher, shape: &Shape) {
    match shape {
        Shape::Rect { rect, color } => {
            hasher.write_u8(0);
            hash_rect(hasher, *rect);
            hash_color(hasher, *color);
        }
        Shape::RoundedRect { rect, radius, color } => {
            hasher.write_u8(1);
            hash_rect(hasher, *rect);
            hasher.write_f32(*radius);
            hash_color(hasher, *color);
        }
        Shape::Border { rect, border } => {
            hasher.write_u8(2);
            hash_rect(hasher, *rect);
            hasher.write_f32(border.width);
            hasher.write_f32(border.radius);
            hash_color(hasher, border.color);
        }
        Shape::FillPath { path, color, rule } => {
            hasher.write_u8(3);
            hash_path(hasher, path);
            hash_color(hasher, *color);
            hasher.write_u8(match rule {
                FillRule::NonZero => 0,
                FillRule::EvenOdd => 1,
            });
        }
        Shape::StrokePath { path, stroke } => {
            hasher.write_u8(4);
            hash_path(hasher, path);
            hash_stroke(hasher, stroke);
        }
        Shape::Text { .. } | Shape::Icon { .. } => hash_glyphs(hasher, shape),
        Shape::FileIcon {
            icon,
            origin,
            size,
            look,
        } => {
            hasher.write_u8(8);
            icon.hash(hasher);
            hash_point(hasher, *origin);
            hasher.write_f32(*size);
            look.hash(hasher);
        }
        Shape::Image { image, rect } => {
            hasher.write_u8(7);
            hasher.write_u64(image.key());
            hash_rect(hasher, *rect);
        }
    }
}

/// Adds a line of text or an icon to a hash.
fn hash_glyphs(hasher: &mut StableHasher, shape: &Shape) {
    match shape {
        Shape::Text {
            text,
            style,
            color,
            origin,
            max_width,
        } => {
            hasher.write_u8(5);
            text.hash(hasher);
            style.typeface.hash(hasher);
            hasher.write_f32(style.size);
            hash_color(hasher, *color);
            hash_point(hasher, *origin);
            hasher.write_f32(max_width.unwrap_or(-1.0));
        }
        Shape::Icon {
            icon,
            origin,
            size,
            color,
        } => {
            hasher.write_u8(6);
            icon.hash(hasher);
            hash_point(hasher, *origin);
            hasher.write_f32(*size);
            hash_color(hasher, *color);
        }
        // The other shapes are hashed by `hash_shape`.
        _ => {}
    }
}

/// Adds a rectangle to a hash.
fn hash_rect(hasher: &mut StableHasher, rect: Rect) {
    for value in [rect.x, rect.y, rect.width, rect.height] {
        hasher.write_f32(value);
    }
}

/// Adds a point to a hash.
fn hash_point(hasher: &mut StableHasher, point: Point) {
    hasher.write_f32(point.x);
    hasher.write_f32(point.y);
}

/// Adds a color to a hash.
fn hash_color(hasher: &mut StableHasher, color: Color) {
    hasher.write(&[color.red, color.green, color.blue, color.alpha]);
}

/// Adds a line to a hash.
fn hash_stroke(hasher: &mut StableHasher, stroke: &Stroke) {
    hasher.write_f32(stroke.width);
    hash_color(hasher, stroke.color);
    hasher.write_u8(match stroke.cap {
        LineCap::Butt => 0,
        LineCap::Round => 1,
        LineCap::Square => 2,
    });
}

/// Adds the steps of a path to a hash.
fn hash_path(hasher: &mut StableHasher, path: &Path) {
    for command in path.commands() {
        match *command {
            PathCommand::MoveTo(point) => {
                hasher.write_u8(0);
                hash_point(hasher, point);
            }
            PathCommand::LineTo(point) => {
                hasher.write_u8(1);
                hash_point(hasher, point);
            }
            PathCommand::QuadraticTo(control, point) => {
                hasher.write_u8(2);
                hash_point(hasher, control);
                hash_point(hasher, point);
            }
            PathCommand::CubicTo(first, second, point) => {
                hasher.write_u8(3);
                hash_point(hasher, first);
                hash_point(hasher, second);
                hash_point(hasher, point);
            }
            PathCommand::Close => hasher.write_u8(4),
        }
    }
}
