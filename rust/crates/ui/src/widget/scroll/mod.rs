//! Scroll areas: a view onto content larger than it, moved by the wheel, by dragging its scroll bars, and by the arrows,
//! Page Up, Page Down, Home and End while it has the focus. The scroll bars are drawn over the edge of the content, so
//! that showing them never changes its layout.

mod axis;
pub(crate) mod bar;

use std::hash::Hash;

pub(crate) use axis::GLIDE_FRAME;
pub use axis::ScrollAxis;

use render::Rect;

use crate::frame::Ui;
use crate::input::{Key, Modifiers, NamedKey};
use crate::interaction::{Sense, WidgetEvent};
use crate::memory::ScrollState;
use crate::widget::panel::Axis;
use bar::BarSpec;

/// What a scroll area tells its view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollResponse {
    /// The content, where it is drawn now: its top left corner moved up and left by the offsets.
    pub content: Rect,
    /// The offsets the content is drawn at in this frame, across and down, in logical pixels.
    pub offset: (f64, f64),
}

/// A movement of the keys over something that scrolls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum KeyScroll {
    /// By a distance.
    By(f64),
    /// To the start.
    Start,
    /// To the end.
    End,
}

/// What a key does to a view that scrolls by lines and pages, if anything.
pub(crate) fn key_scroll(key: Key, modifiers: Modifiers, (line, page): (f64, f64)) -> Option<KeyScroll> {
    if !modifiers.is_empty() {
        return None;
    }
    match key {
        Key::Named(NamedKey::ArrowUp) => Some(KeyScroll::By(-line)),
        Key::Named(NamedKey::ArrowDown) => Some(KeyScroll::By(line)),
        Key::Named(NamedKey::PageUp) => Some(KeyScroll::By(-page)),
        Key::Named(NamedKey::PageDown) => Some(KeyScroll::By(page)),
        Key::Named(NamedKey::Home) => Some(KeyScroll::Start),
        Key::Named(NamedKey::End) => Some(KeyScroll::End),
        _ => None,
    }
}

impl Ui<'_> {
    /// Declares a scroll area over a rectangle for content of a size, across and down; `inside` declares the content,
    /// laid out in the rectangle it is given, which starts above and left of the area by the offsets. Only what is
    /// inside the area is seen.
    pub fn scroll_area<R>(
        &mut self,
        key: impl Hash,
        rect: Rect,
        content_size: (f32, f32),
        inside: impl FnOnce(&mut Ui<'_>, Rect) -> R,
    ) -> (ScrollResponse, R) {
        let id = self.id(key);
        let (_, events) = self.interact_with_events(id, rect, Sense::SCROLL | Sense::FOCUS);
        let mut state = self.memory().scrolls.remove(&id).unwrap_or_default();
        let theme = *self.theme();
        let line = f64::from(theme.sizes.row_height);
        let view = (f64::from(rect.width), f64::from(rect.height));
        let content = (f64::from(content_size.0), f64::from(content_size.1));
        let now = self.now();
        for event in events {
            match event {
                WidgetEvent::Wheel { delta } => {
                    state.x.scroll_by(f64::from(delta.x), now);
                    state.y.scroll_by(f64::from(delta.y), now);
                }
                WidgetEvent::Key(key) => {
                    let page = (view.1 - line).max(line);
                    let scroll = key_scroll(key.key, key.modifiers, (line, page));
                    if scroll.is_some() {
                        self.claim_key(id, key);
                    }
                    match scroll {
                        Some(KeyScroll::By(distance)) => state.y.scroll_by(distance, now),
                        Some(KeyScroll::Start) => state.y.glide_to(0.0, now),
                        Some(KeyScroll::End) => state.y.glide_to(content.1, now),
                        None => {}
                    }
                }
                _ => {}
            }
        }
        self.settle(&mut state, (content.0 - view.0, content.1 - view.1));
        let shown = Rect::new(
            rect.x - crate::number::narrow(state.x.offset()),
            rect.y - crate::number::narrow(state.y.offset()),
            content_size.0,
            content_size.1,
        );
        let response = ScrollResponse {
            content: shown,
            offset: (state.x.offset(), state.y.offset()),
        };
        let result = self.clip(rect, |ui| inside(ui, shown));
        self.scroll_bars(id, rect, content, &mut state);
        let _ = self.memory().scrolls.insert(id, state);
        (response, result)
    }

    /// Keeps the offsets inside the content and moves them along their glides, asking for the next frame of a glide.
    fn settle(&mut self, state: &mut ScrollState, largest: (f64, f64)) {
        let now = self.now();
        state.x.clamp(largest.0);
        state.y.clamp(largest.1);
        let across = state.x.advance(now);
        let down = state.y.advance(now);
        if across || down {
            self.request_frame_at(now + GLIDE_FRAME);
        }
    }

    /// Draws the scroll bars a scroll area needs, down its right side and along its bottom.
    fn scroll_bars(&mut self, id: crate::id::WidgetId, rect: Rect, content: (f64, f64), state: &mut ScrollState) {
        let thickness = self.theme().sizes.scrollbar;
        let needs_across = content.0 > f64::from(rect.width);
        let needs_down = content.1 > f64::from(rect.height);
        let corner = |needed: bool| if needed { thickness } else { 0.0 };
        let down_track = Rect::new(
            rect.right() - thickness,
            rect.y,
            thickness,
            rect.height - corner(needs_across),
        );
        let across_track = Rect::new(
            rect.x,
            rect.bottom() - thickness,
            rect.width - corner(needs_down),
            thickness,
        );
        let down = BarSpec {
            axis: Axis::Vertical,
            view: f64::from(rect.height),
            content: content.1,
        };
        let across = BarSpec {
            axis: Axis::Horizontal,
            view: f64::from(rect.width),
            content: content.0,
        };
        self.scroll_bar(id.with("down"), down_track, down, &mut state.y);
        self.scroll_bar(id.with("across"), across_track, across, &mut state.x);
    }
}
