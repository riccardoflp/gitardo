//! The header bar as the title bar of the window (D58): what each place of the window is to the system
//! (`window.title-bar`), and **Minimize**, **Maximize** or **Restore** and **Close** at its right end
//! (`window.caption-buttons`).

use ui::{AccessRole, Color, Description, LineCap, Path, Point, Rect, Sense, Stroke, Theme, Ui};

use crate::intent::Intent;
use crate::text::title_bar as words;

#[cfg(test)]
mod tests;

/// How far from the edges of the window, in logical pixels, the pointer resizes it.
pub const EDGE: f32 = 6.0;
/// How wide each button of the title bar is, in logical pixels: as wide as Windows draws its own.
pub const CAPTION_BUTTON_WIDTH: f32 = 46.0;
/// How wide the outlines of the buttons are, in logical pixels: as Windows draws them.
const GLYPH: f32 = 10.0;
/// How much room the system's three buttons take at the top left of a window on macOS, in logical pixels, with their
/// margins: the header bar starts after them.
pub const SYSTEM_BUTTONS_ROOM: f32 = 78.0;
/// How opaque the header bar is drawn while the window has not the focus, out of 255: dimmed, as the system dims the
/// title bar of a window behind another.
pub const UNFOCUSED_OPACITY: u8 = 115;

/// Who draws the title bar of the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TitleBar {
    /// The system: the header bar is under the system's title bar, which has the buttons.
    #[default]
    System,
    /// Gitardo: the header bar is the title bar, with **Minimize**, **Maximize** and **Close** at its right end
    /// (Windows, Linux).
    Drawn,
    /// The system's title bar, transparent over the header bar, with its three buttons at the top left (macOS).
    Transparent,
}

/// The title bar of the window, as the buttons show it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CaptionState {
    /// Who draws it.
    pub title_bar: TitleBar,
    /// Whether the window is maximized: **Maximize** is **Restore** then, and nothing resizes the window.
    pub is_maximized: bool,
}

/// What `hit` needs of the window, as the last frame drew it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameState {
    /// The title bar and whether the window is maximized.
    pub caption: CaptionState,
    /// How tall the header bar is, from the top of the window: the band that is the caption.
    pub caption_height: f32,
    /// The places of the header bar that take the pointer, buttons, fields and links, which are not the caption.
    pub controls: Vec<Rect>,
}

/// The size of the inside of the window, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowSize {
    /// Its width.
    pub width: f32,
    /// Its height.
    pub height: f32,
}

/// What a place of the window is, as the system asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hit {
    /// What Gitardo draws and takes the pointer on.
    Client,
    /// A free place of the header bar: it moves the window.
    Caption,
    /// **Minimize**.
    Minimize,
    /// **Maximize**, or **Restore**.
    Maximize,
    /// **Close**.
    Close,
    /// An edge, which resizes the window.
    Edge(Edge),
    /// A corner, which resizes the window both ways.
    Corner(Corner),
}

/// An edge of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
}

/// A corner of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Corner {
    /// The top left corner.
    TopLeft,
    /// The top right corner.
    TopRight,
    /// The bottom left corner.
    BottomLeft,
    /// The bottom right corner.
    BottomRight,
}

/// What a button of the title bar asks of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowIntent {
    /// **Minimize**.
    Minimize,
    /// **Maximize**.
    Maximize,
    /// **Restore**.
    Restore,
    /// **Close**: as the system's close request.
    Close,
}

/// How tall the header bar is, which is the title bar of the window.
#[must_use]
pub fn caption_height(theme: &Theme) -> f32 {
    theme.sizes.control_height + 2.0 * theme.sizes.spacing.medium
}

/// How opaque the header bar is drawn: whole while the window has the focus or the system draws the title bar,
/// dimmed otherwise.
#[must_use]
pub fn header_opacity(caption: CaptionState, ui: &Ui<'_>) -> u8 {
    if caption.title_bar == TitleBar::System || ui.is_window_focused() {
        u8::MAX
    } else {
        UNFOCUSED_OPACITY
    }
}

/// Where the three buttons of the title bar are, in a window of a size: at the right end of the header bar, as high
/// as it, reaching the top and the right edges of the window.
#[must_use]
pub fn caption_buttons_rect(size: WindowSize, caption_height: f32) -> Rect {
    let width = 3.0 * CAPTION_BUTTON_WIDTH;
    Rect::new(size.width - width, 0.0, width, caption_height)
}

/// What a place of the window is, as the system asks it: an edge or a corner while the window is not maximized, one
/// of the three buttons, the caption where the header bar is free, and the inside of the window elsewhere.
///
/// Feature: window.title-bar
#[must_use]
pub fn hit(place: Point, size: WindowSize, frame: &FrameState) -> Hit {
    let caption = frame.caption;
    match caption.title_bar {
        TitleBar::System => return Hit::Client,
        TitleBar::Drawn if !caption.is_maximized => {
            if let Some(edge) = edge_at(place, size) {
                return edge;
            }
        }
        // The system resizes a window with a transparent title bar itself, from outside it.
        TitleBar::Drawn | TitleBar::Transparent => {}
    }
    if caption.title_bar == TitleBar::Drawn
        && let Some(button) = button_at(place, size, frame.caption_height)
    {
        return button;
    }
    let is_in_band = place.y < frame.caption_height && place.x >= 0.0 && place.x < size.width;
    if is_in_band && !frame.controls.iter().any(|control| control.contains(place)) {
        Hit::Caption
    } else {
        Hit::Client
    }
}

/// The edge or the corner a place is near, if it is.
fn edge_at(place: Point, size: WindowSize) -> Option<Hit> {
    let is_left = place.x < EDGE;
    let is_right = place.x >= size.width - EDGE;
    let is_top = place.y < EDGE;
    let is_bottom = place.y >= size.height - EDGE;
    let hit = match (is_left, is_right, is_top, is_bottom) {
        (true, _, true, _) => Hit::Corner(Corner::TopLeft),
        (_, true, true, _) => Hit::Corner(Corner::TopRight),
        (true, _, _, true) => Hit::Corner(Corner::BottomLeft),
        (_, true, _, true) => Hit::Corner(Corner::BottomRight),
        (true, _, _, _) => Hit::Edge(Edge::Left),
        (_, true, _, _) => Hit::Edge(Edge::Right),
        (_, _, true, _) => Hit::Edge(Edge::Top),
        (_, _, _, true) => Hit::Edge(Edge::Bottom),
        (false, false, false, false) => return None,
    };
    Some(hit)
}

/// The button of the title bar a place is on, if it is.
fn button_at(place: Point, size: WindowSize, caption_height: f32) -> Option<Hit> {
    let buttons = caption_buttons_rect(size, caption_height);
    if !(place.y < buttons.bottom() && place.x >= buttons.x && place.x < size.width) {
        return None;
    }
    let from_left = place.x - buttons.x;
    Some(if from_left < CAPTION_BUTTON_WIDTH {
        Hit::Minimize
    } else if from_left < 2.0 * CAPTION_BUTTON_WIDTH {
        Hit::Maximize
    } else {
        Hit::Close
    })
}

/// One of the three buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Button {
    /// **Minimize**.
    Minimize,
    /// **Maximize**.
    Maximize,
    /// **Restore**.
    Restore,
    /// **Close**.
    Close,
}

/// **Minimize**, **Maximize** or **Restore**, and **Close**, side by side over the place the window clipped them to,
/// drawn as Windows draws its own: the outlines of Windows, **Close** red under the pointer, the other two lighter.
///
/// Feature: window.caption-buttons
pub fn caption_buttons_view(state: &CaptionState, ui: &mut Ui<'_>) -> Vec<Intent> {
    if state.title_bar != TitleBar::Drawn {
        return Vec::new();
    }
    let area = ui.current_clip();
    let middle = if state.is_maximized {
        Button::Restore
    } else {
        Button::Maximize
    };
    let mut intents = Vec::new();
    let mut left = area.right() - 3.0 * CAPTION_BUTTON_WIDTH;
    for button in [Button::Minimize, middle, Button::Close] {
        let rect = Rect::new(left, area.y, CAPTION_BUTTON_WIDTH, area.height);
        left += CAPTION_BUTTON_WIDTH;
        if caption_button(button, rect, header_opacity(*state, ui), ui) {
            intents.push(Intent::Window(match button {
                Button::Minimize => WindowIntent::Minimize,
                Button::Maximize => WindowIntent::Maximize,
                Button::Restore => WindowIntent::Restore,
                Button::Close => WindowIntent::Close,
            }));
        }
    }
    intents
}

/// One button, named for the screen reader and in its tooltip, its outline dimmed to an opacity unless the pointer is
/// on it; whether it was clicked.
fn caption_button(button: Button, rect: Rect, opacity: u8, ui: &mut Ui<'_>) -> bool {
    let name = match button {
        Button::Minimize => words::MINIMIZE,
        Button::Maximize => words::MAXIMIZE,
        Button::Restore => words::RESTORE,
        Button::Close => words::CLOSE,
    };
    let id = ui.id(("caption button", name));
    let response = ui.interact(id, rect, Sense::CLICK);
    ui.describe(id, rect, &Description::new(AccessRole::Button, name));
    ui.tooltip(&response, name);
    let colors = ui.theme().colors;
    let is_lit = response.hovered || response.pressed;
    let (background, foreground) = match (button, is_lit) {
        (Button::Close, true) => (Some(colors.close_hover), colors.close_hover_foreground),
        (_, true) => (Some(colors.list_hover), colors.foreground),
        (_, false) => (None, colors.foreground),
    };
    if let Some(background) = background {
        ui.fill_rect(rect, background);
    }
    let opacity = if is_lit { u8::MAX } else { opacity };
    ui.faded(opacity, |ui| draw_glyph(button, rect, foreground, ui));
    response.clicked
}

/// The outline of a button, ten logical pixels across in its middle, in lines one pixel wide.
fn draw_glyph(button: Button, rect: Rect, color: Color, ui: &mut Ui<'_>) {
    let stroke = Stroke {
        width: 1.0,
        cap: LineCap::Butt,
        color,
    };
    // The lines are laid on the middle of pixels, so that a line one pixel wide covers whole pixels.
    let left = (rect.x + (rect.width - GLYPH) / 2.0).round() + 0.5;
    let top = (rect.y + (rect.height - GLYPH) / 2.0).round() + 0.5;
    let size = GLYPH - 1.0;
    match button {
        Button::Minimize => {
            let y = top + (size / 2.0).round();
            ui.line(Point::new(left, y), Point::new(left + size, y), stroke);
        }
        Button::Maximize => ui.stroke_path(Path::rect(Rect::new(left, top, size, size)), stroke),
        Button::Restore => {
            let front = Rect::new(left, top + 2.0, size - 2.0, size - 2.0);
            ui.stroke_path(Path::rect(front), stroke);
            let mut back = Path::new();
            let _ = back
                .move_to(Point::new(left + 2.0, top + 2.0))
                .line_to(Point::new(left + 2.0, top))
                .line_to(Point::new(left + size, top))
                .line_to(Point::new(left + size, top + size - 2.0))
                .line_to(Point::new(left + size - 2.0, top + size - 2.0));
            ui.stroke_path(back, stroke);
        }
        Button::Close => {
            ui.line(Point::new(left, top), Point::new(left + size, top + size), stroke);
            ui.line(Point::new(left + size, top), Point::new(left, top + size), stroke);
        }
    }
}
