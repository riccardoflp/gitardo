//! The end of a frame: the tooltip drawn if it is due, the widgets kept for the input of the next frame, and the parts
//! of the window whose shapes changed drawn again.

use render::{Canvas, PixelRect, Rect};

use super::{FrameOutput, Ui};
use crate::interaction::update_hover;
use crate::memory::Surface;
use crate::widget::tooltip;

impl Ui<'_> {
    /// Ends the frame: draws what changed into the frame buffer and says what the application should do next.
    #[must_use]
    pub fn finish(mut self) -> FrameOutput {
        self.finish_drag();
        self.show_tooltip();
        self.keep_widgets();
        self.close_overlays();
        if self.memory.interaction.active.is_none() {
            self.memory.drag = None;
        }
        let regions = self.draw_changes();
        self.memory.last_frame = Some(self.settings.now);
        let scale = self.settings.scale;
        let bounds = self.frame.bounds();
        self.output.dirty = regions
            .iter()
            .map(|region| {
                let physical = Rect::new(
                    region.x * scale,
                    region.y * scale,
                    region.width * scale,
                    region.height * scale,
                );
                PixelRect::enclosing(physical).intersection(bounds)
            })
            .filter(|region| !region.is_empty())
            .collect();
        self.output
    }

    /// How many shapes the frame declared, for the tests of what is drawn.
    #[must_use]
    pub fn shape_count(&self) -> usize {
        self.display.len()
    }

    /// Draws the tooltip asked for, when the pointer has been over its widget long enough, or asks for a frame when it
    /// will have been.
    fn show_tooltip(&mut self) {
        let Some(request) = self.tooltip.take() else {
            return;
        };
        let interaction = &self.memory.interaction;
        let (Some((id, since)), Some(pointer)) = (interaction.hover_since, interaction.pointer) else {
            return;
        };
        if id != request.id || interaction.tooltip_hidden_on == Some(id) {
            return;
        }
        let due = since + tooltip::DELAY;
        if self.settings.now < due {
            self.request_frame_at(due);
            return;
        }
        tooltip::draw(self, &request.text, pointer);
    }

    /// Keeps the widgets of this frame for the input of the next one: the focus leaves a widget that is gone, and the
    /// widget under the pointer is found again, with a frame asked for if it changed.
    fn keep_widgets(&mut self) {
        let mut hits = std::mem::take(&mut self.hits);
        hits.stack();
        let now = self.settings.now;
        let interaction = &mut self.memory.interaction;
        if let Some(focus) = interaction.focus
            && !hits.is_focusable(focus)
        {
            interaction.focus = None;
        }
        interaction.hits = hits;
        update_hover(interaction, now);
        if interaction.hovered != self.hovered_at_start {
            self.request_frame_at(now);
        }
    }

    /// Draws the parts of the window whose shapes changed since the last frame, all of it when the size, the scale or
    /// the theme changed, and returns them, in logical pixels.
    fn draw_changes(&mut self) -> Vec<Rect> {
        let surface = Surface {
            width: self.frame.width(),
            height: self.frame.height(),
            scale: self.settings.scale,
            theme: self.settings.theme.name,
        };
        let before = if self.memory.surface == Some(surface) {
            self.memory.tiles.take()
        } else {
            None
        };
        let tiles = self.display.tiles(self.window.width, self.window.height);
        let regions = tiles.changed_since(before.as_ref());
        let mut canvas = Canvas::new(self.frame, self.assets, self.settings.scale);
        self.display.draw(&mut canvas, &regions);
        self.memory.tiles = Some(tiles);
        self.memory.surface = Some(surface);
        regions
    }
}
