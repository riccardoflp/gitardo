//! What a press on the title bar the program draws asks of the system, on the systems that do not ask the program what
//! each place of a window is (Linux and macOS): the window asks the system to move it, to resize it, to maximize or
//! restore it or to open its menu, as the system's own title bar would.

use winit::window::ResizeDirection;

use super::{FramePart, ResizeEdge};
use crate::input::PointerButton;

/// What the window asks of the system for a press.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FrameAction {
    /// The system moves the window while the button is held.
    Move,
    /// The window is maximized, or restored when it is maximized.
    ToggleMaximized,
    /// The system opens its menu of the window.
    WindowMenu,
    /// The system resizes the window from an edge or a corner while the button is held.
    Resize(ResizeEdge),
}

impl FrameAction {
    /// Whether the window starts to be moved or resized: what opened over its inside closes then.
    pub(crate) fn starts_moving(self) -> bool {
        matches!(self, FrameAction::Move | FrameAction::Resize(_))
    }

    /// Asks the system for it, at a place of the window in physical pixels. A system that cannot do it, such as one
    /// whose compositor has no menu of the window for `winit` to open, leaves the window as it is.
    #[cfg(not(windows))]
    pub(crate) fn ask(self, window: &winit::window::Window, place: winit::dpi::PhysicalPosition<f64>) {
        match self {
            FrameAction::Move => {
                let _ = window.drag_window();
            }
            FrameAction::ToggleMaximized => window.set_maximized(!window.is_maximized()),
            FrameAction::WindowMenu => window.show_window_menu(place),
            FrameAction::Resize(edge) => {
                let _ = window.drag_resize_window(resize_direction(edge));
            }
        }
    }
}

/// What a press of a button on a place of the window asks: on the caption the main button moves the window, twice it
/// maximizes or restores it, the other button opens the menu of the window; on an edge or a corner the main button
/// resizes it. Anywhere else the press is the program's.
pub(crate) fn press_action(part: FramePart, button: PointerButton, click_count: u32) -> Option<FrameAction> {
    match (part, button) {
        (FramePart::Caption, PointerButton::Primary) if click_count == 2 => Some(FrameAction::ToggleMaximized),
        (FramePart::Caption, PointerButton::Primary) => Some(FrameAction::Move),
        (FramePart::Caption, PointerButton::Secondary) => Some(FrameAction::WindowMenu),
        (FramePart::Edge(edge), PointerButton::Primary) => Some(FrameAction::Resize(edge)),
        _ => None,
    }
}

/// The direction `winit` resizes a window in from an edge or a corner, which also gives the shape of the pointer there.
pub(crate) fn resize_direction(edge: ResizeEdge) -> ResizeDirection {
    match edge {
        ResizeEdge::Left => ResizeDirection::West,
        ResizeEdge::Right => ResizeDirection::East,
        ResizeEdge::Top => ResizeDirection::North,
        ResizeEdge::Bottom => ResizeDirection::South,
        ResizeEdge::TopLeft => ResizeDirection::NorthWest,
        ResizeEdge::TopRight => ResizeDirection::NorthEast,
        ResizeEdge::BottomLeft => ResizeDirection::SouthWest,
        ResizeEdge::BottomRight => ResizeDirection::SouthEast,
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

    use super::*;

    #[test]
    fn window_title_bar__on_linux_and_macos_a_press_on_the_caption_or_an_edge_asks_the_system() {
        let primary = PointerButton::Primary;
        assert_eq!(press_action(FramePart::Caption, primary, 1), Some(FrameAction::Move));
        assert_eq!(
            press_action(FramePart::Caption, primary, 2),
            Some(FrameAction::ToggleMaximized)
        );
        assert_eq!(
            press_action(FramePart::Caption, PointerButton::Secondary, 1),
            Some(FrameAction::WindowMenu)
        );
        let corner = FramePart::Edge(ResizeEdge::BottomRight);
        assert_eq!(
            press_action(corner, primary, 1),
            Some(FrameAction::Resize(ResizeEdge::BottomRight))
        );
        assert!(FrameAction::Move.starts_moving() && FrameAction::Resize(ResizeEdge::Left).starts_moving());
        assert!(!FrameAction::ToggleMaximized.starts_moving());
        // The buttons the program draws and the inside of the window are the program's.
        for part in [
            FramePart::Client,
            FramePart::Minimize,
            FramePart::Maximize,
            FramePart::Close,
        ] {
            assert_eq!(press_action(part, primary, 1), None, "{part:?}");
        }
        assert_eq!(press_action(corner, PointerButton::Secondary, 1), None);
        assert_eq!(resize_direction(ResizeEdge::TopLeft), ResizeDirection::NorthWest);
    }
}
