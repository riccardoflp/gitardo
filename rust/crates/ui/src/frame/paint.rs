//! Drawing on a frame: the shapes a widget or a view draws, in logical pixels of the window, kept until the end of the
//! frame and drawn then, by layer, where the window changed.

use render::{Border, Color, FillRule, Icon, Path, Point, Rect, Stroke, TextRun};

use super::Ui;
use crate::paint::Shape;

impl Ui<'_> {
    /// Fills a rectangle with a color.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        if color.is_transparent() || rect.is_empty() {
            return;
        }
        self.record(Shape::Rect { rect, color }, rect);
    }

    /// Fills a rectangle with rounded corners.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        if color.is_transparent() || rect.is_empty() {
            return;
        }
        self.record(Shape::RoundedRect { rect, radius, color }, rect);
    }

    /// Draws a border inside a rectangle.
    pub fn border(&mut self, rect: Rect, border: Border) {
        if border.color.is_transparent() || rect.is_empty() {
            return;
        }
        self.record(Shape::Border { rect, border }, rect);
    }

    /// Fills a circle.
    pub fn fill_circle(&mut self, center: Point, radius: f32, color: Color) {
        self.fill_path(Path::circle(center, radius), color, FillRule::NonZero);
    }

    /// Strokes the outline of a circle.
    pub fn stroke_circle(&mut self, center: Point, radius: f32, stroke: Stroke) {
        self.stroke_path(Path::circle(center, radius), stroke);
    }

    /// Draws a straight line.
    pub fn line(&mut self, from: Point, to: Point, stroke: Stroke) {
        let mut path = Path::new();
        let _ = path.move_to(from).line_to(to);
        self.stroke_path(path, stroke);
    }

    /// Fills a path, by a fill rule.
    pub fn fill_path(&mut self, path: Path, color: Color, rule: FillRule) {
        let Some(bounds) = path.bounds() else {
            return;
        };
        if color.is_transparent() {
            return;
        }
        self.record(Shape::FillPath { path, color, rule }, bounds);
    }

    /// Strokes a path with a line.
    pub fn stroke_path(&mut self, path: Path, stroke: Stroke) {
        let Some(bounds) = path.bounds() else {
            return;
        };
        if stroke.color.is_transparent() {
            return;
        }
        self.record(Shape::StrokePath { path, stroke }, bounds.inset(-stroke.width));
    }

    /// Draws a line of text, cut with an ellipsis when the run allows less than its width, and returns the width it
    /// takes, in logical pixels.
    pub fn text(&mut self, run: &TextRun<'_>) -> f32 {
        let width = match run.max_width {
            Some(limit) => self.assets.fonts.truncate(run.text, run.style, limit).width,
            None => self.assets.fonts.measure(run.text, run.style),
        };
        let height = self.assets.fonts.line_metrics(run.style).height;
        // Accents and descenders may reach past the line: a quarter of the size around it holds them.
        let margin = run.style.size * 0.25;
        let bounds = Rect::new(
            run.origin.x - margin,
            run.origin.y - margin,
            width + 2.0 * margin,
            height + 2.0 * margin,
        );
        if !run.color.is_transparent() && !run.text.is_empty() {
            let shape = Shape::Text {
                text: run.text.to_owned(),
                style: run.style,
                color: run.color,
                origin: run.origin,
                max_width: run.max_width,
            };
            self.record(shape, bounds);
            if self.recording.is_on() && !bounds.intersection(self.current_clip()).is_empty() {
                let place = (self.current_clip(), self.layer);
                self.recording.record(run, place);
            }
        }
        width
    }

    /// Draws an icon of Gitardo's set, its top left corner at a point and its side `size` logical pixels.
    pub fn icon(&mut self, icon: Icon, origin: Point, size: f32, color: Color) {
        if color.is_transparent() {
            return;
        }
        let shape = Shape::Icon {
            icon,
            origin,
            size,
            color,
        };
        self.record(shape, Rect::new(origin.x, origin.y, size, size));
    }
}
