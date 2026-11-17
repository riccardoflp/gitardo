//! A window whose title bar the program draws (decision D58): what each place of it is to the system, so that the
//! system moves it, resizes it, maximizes and restores it and opens its menu as it does for a title bar of its own.
//!
//! The program says what the places are with a function of the place and of the size of the window, which the window
//! keeps. On Windows it answers what the system asks of the place under the pointer; on Linux and macOS the window asks
//! the system to move or resize it when the pointer is pressed on a caption or an edge.

use crate::geometry::{LogicalSize, Point};

/// Whether the system draws the title bar of a window, or the program.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Decorations {
    /// The system's title bar and frame.
    #[default]
    System,
    /// The program draws its title bar: on Windows the window keeps the rest of its frame for the system (its shadow,
    /// its rounded corners, Snap Layouts and Aero Snap); on macOS the system's title bar becomes transparent over the
    /// window and keeps its three buttons; on Linux the window has no decorations.
    Drawn,
}

/// What a place of a window drawing its own title bar is, as the system sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FramePart {
    /// The inside of the window: the program takes the pointer there.
    Client,
    /// The title bar: dragged it moves the window, a double click maximizes or restores it, and a right click opens
    /// the system's menu of the window.
    Caption,
    /// The button that minimizes the window, which the program draws and takes the pointer on.
    Minimize,
    /// The button that maximizes or restores the window, which the program draws: on Windows 11 the pointer resting on
    /// it shows Snap Layouts.
    Maximize,
    /// The button that closes the window, which the program draws and takes the pointer on.
    Close,
    /// An edge or a corner, which resizes the window.
    Edge(ResizeEdge),
}

/// An edge or a corner of a window, which resizes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResizeEdge {
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The top left corner.
    TopLeft,
    /// The top right corner.
    TopRight,
    /// The bottom left corner.
    BottomLeft,
    /// The bottom right corner.
    BottomRight,
}

/// What each place of a window is: a function of the place and of the size of the window, both in logical pixels,
/// which the window calls when the system asks.
pub(crate) type FrameParts = Box<dyn Fn(Point, LogicalSize) -> FramePart>;

/// What a window keeps of a title bar the program draws.
pub(crate) struct DrawnFrame {
    /// The procedure that answers what Windows asks of the places of the window, when the program draws its title
    /// bar.
    #[cfg(windows)]
    procedure: Option<crate::windows::FrameProcedure>,
    /// What each place of the window is, once the program said it.
    #[cfg(not(windows))]
    parts: std::cell::RefCell<Option<FrameParts>>,
}

impl DrawnFrame {
    /// What a new window keeps, as its decorations say.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when Windows refused the procedure of the window.
    #[cfg(windows)]
    pub(crate) fn new(
        window: &winit::window::Window,
        decorations: Decorations,
        inbox: &std::sync::Arc<crate::mailbox::Mailbox<crate::input::InputEvent>>,
    ) -> Result<DrawnFrame, crate::error::PlatformError> {
        let procedure = match decorations {
            Decorations::System => None,
            Decorations::Drawn => Some(crate::windows::FrameProcedure::install(
                window,
                std::sync::Arc::clone(inbox),
            )?),
        };
        Ok(DrawnFrame { procedure })
    }

    /// What a new window keeps.
    ///
    /// # Errors
    ///
    /// None on this system.
    #[cfg(not(windows))]
    pub(crate) fn new(
        _window: &winit::window::Window,
        _decorations: Decorations,
        _inbox: &std::sync::Arc<crate::mailbox::Mailbox<crate::input::InputEvent>>,
    ) -> Result<DrawnFrame, crate::error::PlatformError> {
        Ok(DrawnFrame {
            parts: std::cell::RefCell::new(None),
        })
    }

    /// Says what each place of the window is, from now on.
    pub(crate) fn set_parts(&self, parts: FrameParts) {
        #[cfg(windows)]
        if let Some(procedure) = &self.procedure {
            procedure.set_parts(parts);
        }
        #[cfg(not(windows))]
        if let Ok(mut kept) = self.parts.try_borrow_mut() {
            *kept = Some(parts);
        }
    }
}
