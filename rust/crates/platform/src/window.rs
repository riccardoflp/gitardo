//! A window of Gitardo Desktop: a title bar and a frame of pixels, with its input coming through its event loop.

use std::rc::Rc;

use accesskit::TreeUpdate;
use winit::dpi::{LogicalPosition as LibraryLogicalPosition, LogicalSize as LibraryLogicalSize, PhysicalPosition};
use winit::window::{Window as LibraryWindow, WindowAttributes};

use crate::error::PlatformError;
use crate::event::{Cursor, WindowId};
use crate::event_loop::{Context, LoopState};
use crate::frame::{Decorations, FramePart};
use crate::geometry::{LogicalSize, PhysicalSize, PixelRect, Point, Scale, ScreenPosition};
use crate::input::InputEvent;
use crate::mailbox::InputSender;
use crate::placement::{Screen, WindowPlacement, lands_on_a_screen};
use crate::present::Frame;
use crate::theme::Appearance;
use crate::translate;
use crate::window_state::WindowShared;

/// Whether a window shows when it opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// It shows at once.
    #[default]
    Shown,
    /// It stays hidden until `show`: for tests, or to show it once its first frame is drawn.
    Hidden,
}

/// What a new window is like.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowOptions {
    /// The text of its title bar.
    pub title: String,
    /// The size of its inside, in logical pixels.
    pub size: LogicalSize,
    /// The smallest size of its inside the person can resize it to, in logical pixels.
    pub minimum_size: LogicalSize,
    /// Whether it shows when it opens.
    pub visibility: Visibility,
    /// Whether its title bar is dark or light; none follows the system.
    pub title_bar: Option<Appearance>,
    /// Whether the system draws its title bar, or the program.
    pub decorations: Decorations,
    /// Where the top left corner of its frame goes on the screen; none, or a place on no screen, lets the system
    /// pick.
    pub position: Option<ScreenPosition>,
    /// Whether it is maximized when it shows, its size and its place being those it is restored to.
    pub is_maximized: bool,
}

impl Default for WindowOptions {
    fn default() -> WindowOptions {
        WindowOptions {
            title: String::new(),
            size: LogicalSize {
                width: 1024.0,
                height: 720.0,
            },
            minimum_size: LogicalSize {
                width: 480.0,
                height: 320.0,
            },
            visibility: Visibility::Shown,
            title_bar: None,
            decorations: Decorations::System,
            position: None,
            is_maximized: false,
        }
    }
}

/// Where text is being typed, in logical pixels: the input method shows what it composes and its choices there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextInputArea {
    /// The top left corner of the caret.
    pub position: Point,
    /// The size of the caret, or of the text around it.
    pub size: LogicalSize,
}

/// A window, open until dropped. It belongs to the thread of its event loop.
pub struct Window {
    shared: Rc<WindowShared>,
    owner: Rc<LoopState>,
}

impl Window {
    /// Opens a window, at the place the system picks and the size the options say on the screen it lands on. Its
    /// input comes through the loop as `Event::Input` with its id, starting with a request for its first frame.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused to make the window or what draws in it.
    pub fn open(context: &Context<'_>, options: &WindowOptions) -> Result<Window, PlatformError> {
        let attributes = LibraryWindow::default_attributes()
            .with_title(options.title.clone())
            .with_inner_size(library_size(options.size))
            .with_min_inner_size(library_size(options.minimum_size))
            .with_theme(options.title_bar.map(translate::library_theme))
            // Hidden until the screen reader adapter is made, as it asks.
            .with_visible(false);
        let attributes = decorated(attributes, options.decorations);
        let attributes = match options
            .position
            .filter(|position| lands_on_a_screen(*position, &screens(context)))
        {
            Some(position) => attributes.with_position(PhysicalPosition::new(position.x, position.y)),
            None => attributes,
        };
        let window = context
            .active
            .create_window(attributes)
            .map_err(|error| PlatformError::system("winit, to open a window", &error))?;
        let shared = Rc::new(WindowShared::new(
            context.active,
            Rc::new(window),
            (context.state.proxy.clone(), options.decorations),
        )?);
        shared.keep_placement();
        shared.is_maximized_on_show.set(options.is_maximized);
        context.state.register(&shared);
        let opened = Window {
            shared,
            owner: Rc::clone(context.state),
        };
        opened.request_redraw();
        if options.visibility == Visibility::Shown {
            opened.show();
        }
        Ok(opened)
    }

    /// Its name, as events give it.
    #[must_use]
    pub fn id(&self) -> WindowId {
        self.shared.id
    }

    /// Changes the text of its title bar.
    pub fn set_title(&self, title: &str) {
        self.library().set_title(title);
    }

    /// The size of its inside, in physical pixels: the size of the frame to draw.
    #[must_use]
    pub fn size(&self) -> PhysicalSize {
        let size = self.library().inner_size();
        PhysicalSize {
            width: size.width,
            height: size.height,
        }
    }

    /// Resizes its inside, in logical pixels at the window's scale. The system may refuse, or take its time:
    /// `InputEvent::Resized` says what it did.
    pub fn set_size(&self, size: LogicalSize) {
        // The size applied at once, when it is, comes again as `Resized`; there is nothing else to do with it here.
        let _ = self.library().request_inner_size(library_size(size));
    }

    /// Where the top left corner of its frame is on the screen.
    ///
    /// # Errors
    ///
    /// `PlatformError::Unsupported` on a system that does not say, such as Wayland.
    pub fn position(&self) -> Result<ScreenPosition, PlatformError> {
        let position = self
            .library()
            .outer_position()
            .map_err(|_| PlatformError::Unsupported {
                what: "say where a window is",
            })?;
        Ok(ScreenPosition {
            x: position.x,
            y: position.y,
        })
    }

    /// Moves its frame on the screen; a system that does not let programs place their windows, such as Wayland,
    /// leaves it where it is.
    pub fn set_position(&self, position: ScreenPosition) {
        self.library()
            .set_outer_position(PhysicalPosition::new(position.x, position.y));
    }

    /// The scale of the screen it is on.
    #[must_use]
    pub fn scale(&self) -> Scale {
        self.shared.scale()
    }

    /// Shows it, maximized if it opened so.
    pub fn show(&self) {
        self.shared.is_visible.set(true);
        self.library().set_visible(true);
        if self.shared.is_maximized_on_show.replace(false) {
            self.maximize();
        }
    }

    /// Hides it.
    pub fn hide(&self) {
        self.shared.is_visible.set(false);
        self.library().set_visible(false);
    }

    /// Says what each place of the window is, when the program draws its title bar: where it is moved, resized,
    /// minimized, maximized and closed from. The function is called with a place and the size of the inside of the
    /// window, both in logical pixels, whenever the system asks.
    pub fn set_frame_parts(&self, parts: impl Fn(Point, LogicalSize) -> FramePart + 'static) {
        self.shared.frame.set_parts(Box::new(parts));
    }

    /// Minimizes it.
    pub fn minimize(&self) {
        self.library().set_minimized(true);
    }

    /// Maximizes it.
    pub fn maximize(&self) {
        self.library().set_maximized(true);
    }

    /// Restores it from maximized.
    pub fn restore(&self) {
        self.library().set_maximized(false);
    }

    /// Whether it is maximized.
    #[must_use]
    pub fn is_maximized(&self) -> bool {
        self.library().is_maximized()
    }

    /// Where it is and how large as it is restored from maximized or minimized, and whether it is maximized: what is
    /// kept for the next start.
    #[must_use]
    pub fn placement(&self) -> WindowPlacement {
        self.shared.placement()
    }

    /// The text of its title bar, which the taskbar, the switcher of windows and the screen reader say, even when the
    /// program draws the title bar.
    #[must_use]
    pub fn title(&self) -> String {
        self.library().title()
    }

    /// The size of the window with its frame, in physical pixels.
    #[must_use]
    pub fn outer_size(&self) -> PhysicalSize {
        let size = self.library().outer_size();
        PhysicalSize {
            width: size.width,
            height: size.height,
        }
    }

    /// Makes its title bar dark or light, as the theme of the program is; none follows the system.
    pub fn set_title_bar(&self, appearance: Option<Appearance>) {
        self.library().set_theme(appearance.map(translate::library_theme));
    }

    /// The shape of the pointer over its inside, from now on.
    pub fn set_cursor(&self, cursor: Cursor) {
        if self.shared.cursor.replace(cursor) != cursor {
            self.library().set_cursor(translate::cursor_icon(cursor));
        }
    }

    /// Where text is being typed: the input method is on there, and shows what it composes beside it. None turns
    /// the input method off, so that keys reach the program as they are.
    pub fn set_text_input_area(&self, area: Option<TextInputArea>) {
        let is_allowed = area.is_some();
        if self.shared.is_composing_allowed.replace(is_allowed) != is_allowed {
            self.library().set_ime_allowed(is_allowed);
        }
        if let Some(area) = area {
            let position = LibraryLogicalPosition::new(area.position.x, area.position.y);
            self.library().set_ime_cursor_area(position, library_size(area.size));
        }
    }

    /// Asks for an `InputEvent::RedrawRequested`, handed over once no other event is waiting.
    pub fn request_redraw(&self) {
        self.shared.is_redraw_asked.set(true);
        if self.shared.is_visible.get() {
            self.library().request_redraw();
        } else {
            // The system does not paint a hidden window: the loop hands the request over itself.
            self.owner.ask_redraw(self.shared.id);
        }
    }

    /// Shows a frame: the rectangles that changed are copied into the window and shown at once; a frame of a new
    /// size, or after the system lost what the window showed, is shown whole.
    ///
    /// # Errors
    ///
    /// `PlatformError::FrameSize` when the pixels do not fill the frame's size; `PlatformError::System` when the
    /// system refused to show it.
    pub fn present(&self, frame: Frame<'_>, dirty: &[PixelRect]) -> Result<(), PlatformError> {
        self.shared.presenter.borrow_mut().present(frame, dirty)
    }

    /// Gives the screen reader the tree of what is on screen, or its changes since the last; nothing happens while
    /// no screen reader listens. The first after `AccessibilityRequest::InitialTree` is the whole tree.
    pub fn update_accessibility(&self, update: TreeUpdate) {
        self.shared.adapter.borrow_mut().update_if_active(|| update);
    }

    /// A sender of synthetic input for this window, which other threads can use.
    #[must_use]
    pub fn input_sender(&self) -> InputSender {
        InputSender::new(&self.shared.inbox)
    }

    /// Feeds an event into this window as if it came from the system: it reaches the handler through the loop.
    ///
    /// # Errors
    ///
    /// `PlatformError::Closed` when the loop is gone.
    pub fn send_input(&self, event: InputEvent) -> Result<(), PlatformError> {
        self.shared.inbox.post(event)
    }

    /// The window of `winit`, for the dialogs that belong to it.
    pub(crate) fn library(&self) -> &LibraryWindow {
        &self.shared.window
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.shared.inbox.close();
        self.owner.forget(self.shared.id);
    }
}

/// The attributes of a window with the decorations asked for: without the system's title bar on Windows, keeping
/// the shadow of its frame (D58).
#[cfg(windows)]
fn decorated(attributes: WindowAttributes, decorations: Decorations) -> WindowAttributes {
    use winit::platform::windows::WindowAttributesExtWindows;
    match decorations {
        Decorations::System => attributes,
        Decorations::Drawn => attributes.with_decorations(false).with_undecorated_shadow(true),
    }
}

/// The attributes of a window with the decorations asked for: the systems other than Windows keep theirs for now.
#[cfg(not(windows))]
fn decorated(attributes: WindowAttributes, _decorations: Decorations) -> WindowAttributes {
    attributes
}

/// A logical size as `winit` takes it.
fn library_size(size: LogicalSize) -> LibraryLogicalSize<f32> {
    LibraryLogicalSize::new(size.width, size.height)
}

/// The screens, where each is and how large.
fn screens(context: &Context<'_>) -> Vec<Screen> {
    context
        .active
        .available_monitors()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            Screen {
                position: ScreenPosition {
                    x: position.x,
                    y: position.y,
                },
                size: PhysicalSize {
                    width: size.width,
                    height: size.height,
                },
            }
        })
        .collect()
}
