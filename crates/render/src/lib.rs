//! The pixels of Gitardo Desktop: the frame buffer, shapes, anti-aliasing, fonts and glyphs, icons and themes.
//!
//! Every frame is drawn by the program into a [`FrameBuffer`], which the platform shows in the window. A widget draws
//! through a [`Canvas`] over the frame, in logical pixels: the canvas applies the scale of the screen, keeps a clip
//! and an origin that can be saved and restored, and marks what it draws as dirty so that only that is shown again.
//! It fills rectangles (snapped to whole pixels, the fast path), rounded rectangles, borders, circles and any
//! [`Path`] (anti-aliased by `tiny-skia`), strokes lines and curves, draws text with the [`Fonts`] of its [`Assets`]
//! (shaped and rasterized by `cosmic-text`), and draws the [`Icon`]s of Gitardo's own set. The colors and sizes come
//! from the tokens of a [`Theme`].
//!
//! The fonts that come with the program are part of it ([`Fonts::bundled`]); the only files read are those of the
//! fonts of the system, once, by [`Fonts::load_system_fonts`]. Drawing never reads a file. No type of the two
//! libraries appears in what this crate offers.

#![forbid(unsafe_code)]

mod assets;
mod canvas;
mod color;
mod dirty;
mod error;
mod frame;
mod geometry;
mod icon;
mod mask;
mod number;
mod path;
mod text;
mod theme;

pub use assets::Assets;
pub use canvas::{Border, Canvas, Stroke};
pub use color::Color;
pub use error::Error;
pub use frame::FrameBuffer;
pub use geometry::{PixelRect, Point, Rect, Transform};
pub use icon::{GRID as ICON_GRID, Icon, Icons};
pub use path::{Command, FillRule, LineCap, Path};
pub use text::{FontFamily, FontWeight, Fonts, LineMetrics, TextRun, TextStyle, Truncation, TypefaceId};
pub use theme::{Appearance, CodeColors, CodeKind, Colors, FontSizes, Radii, Sizes, Spacing, Theme, ThemeName};
