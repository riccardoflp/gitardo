//! The title bar the window draws itself (D58): who draws it on each system, and what each place of the window is,
//! kept from the last frame for the platform, which asks between frames.

use std::cell::RefCell;
use std::rc::Rc;

use platform::{Decorations, FramePart, ResizeEdge, Window};
use ui::{Rect, Theme, UiMemory};
use views::window::{CaptionState, Corner, Edge, FrameState, Hit, TitleBar, WindowSize};

/// Who draws the title bar of the window on this system: Gitardo on Windows.
#[must_use]
pub fn system_title_bar() -> TitleBar {
    if cfg!(windows) {
        TitleBar::Drawn
    } else {
        TitleBar::System
    }
}

/// The decorations the platform opens a window with, for a title bar.
#[must_use]
pub fn decorations(title_bar: TitleBar) -> Decorations {
    match title_bar {
        TitleBar::System => Decorations::System,
        TitleBar::Drawn | TitleBar::Transparent => Decorations::Drawn,
    }
}

/// What each place of the window is, as the last frame drew it, shared with the platform.
#[derive(Clone, Debug, Default)]
pub struct FrameParts {
    /// What `hit` needs, from the last frame.
    shared: Rc<RefCell<FrameState>>,
}

impl FrameParts {
    /// Gives the platform the function that says what each place of a window is.
    pub fn attach(&self, window: &Window) {
        let shared = Rc::clone(&self.shared);
        window.set_frame_parts(move |place, size| {
            // A frame being kept as the system asks, which cannot happen on the one thread of the window, reads as
            // the inside of the window.
            shared.try_borrow().map_or(FramePart::Client, |frame| {
                let place = ui::Point::new(place.x, place.y);
                let size = WindowSize {
                    width: size.width,
                    height: size.height,
                };
                frame_part(views::window::hit(place, size, &frame))
            })
        });
    }

    /// Keeps what a frame just drawn says of the title bar: its height and the places of it that take the pointer.
    pub fn keep(&self, caption: CaptionState, (memory, theme, width): (&UiMemory, &Theme, f32)) {
        let caption_height = views::window::caption_height(theme);
        let band = Rect::new(0.0, 0.0, width, caption_height);
        let frame = FrameState {
            caption,
            caption_height,
            controls: memory.pointer_targets(band),
        };
        if let Ok(mut kept) = self.shared.try_borrow_mut() {
            *kept = frame;
        }
    }
}

/// What the platform is told a place is.
#[must_use]
pub fn frame_part(hit: Hit) -> FramePart {
    match hit {
        Hit::Client => FramePart::Client,
        Hit::Caption => FramePart::Caption,
        Hit::Minimize => FramePart::Minimize,
        Hit::Maximize => FramePart::Maximize,
        Hit::Close => FramePart::Close,
        Hit::Edge(edge) => FramePart::Edge(match edge {
            Edge::Left => ResizeEdge::Left,
            Edge::Right => ResizeEdge::Right,
            Edge::Top => ResizeEdge::Top,
            Edge::Bottom => ResizeEdge::Bottom,
        }),
        Hit::Corner(corner) => FramePart::Edge(match corner {
            Corner::TopLeft => ResizeEdge::TopLeft,
            Corner::TopRight => ResizeEdge::TopRight,
            Corner::BottomLeft => ResizeEdge::BottomLeft,
            Corner::BottomRight => ResizeEdge::BottomRight,
        }),
    }
}
