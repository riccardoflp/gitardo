//! A window of Gitardo Desktop: a title bar and a frame of pixels, with its input coming through its event loop.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use super::event_loop::EventLoop;
use super::handle::{OwnedWindow, WINDOW_STYLE, WindowHandle, WindowRequest, register_class};
use super::mailbox::{InputSender, Mailbox};
use super::present::{self, Frame};
use super::procedure::{WM_SYNTHETIC_INPUT, WindowState, window_procedure};
use super::{cursor, ime, wide};
use crate::convert::round_to_coordinate;
use crate::dispatch::WindowSink;
use crate::error::PlatformError;
use crate::event::{Cursor, WindowId};
use crate::geometry::{LogicalSize, PhysicalSize, PixelRect, Point, Scale, ScreenPosition};
use crate::input::InputEvent;
use crate::theme::Appearance;

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
    /// Whether its title bar is dark or light.
    pub title_bar: Appearance,
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
            title_bar: Appearance::Light,
        }
    }
}

/// A window, open until dropped. It belongs to the thread of its event loop.
pub struct Window {
    synthetic: Arc<Mailbox<InputEvent>>,
    native: OwnedWindow,
    // Shared, never borrowed mutably: the window procedure reaches it through a pointer while the window lives.
    state: Rc<WindowState>,
}

impl Window {
    /// Opens a window on an event loop, at the place the system picks and the size the options say on the screen it
    /// lands on. Its input comes through the loop as `Event::Input` with its id.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused to make or size the window.
    pub fn open<M: 'static>(event_loop: &EventLoop<M>, options: &WindowOptions) -> Result<Window, PlatformError> {
        let dispatcher = event_loop.dispatcher();
        let sink: Rc<dyn WindowSink> = Rc::clone(dispatcher) as Rc<dyn WindowSink>;
        let synthetic = Arc::new(Mailbox::new(WM_SYNTHETIC_INPUT));
        let state = Rc::new(WindowState {
            id: dispatcher.new_window_id(),
            sink,
            dpi: Cell::new(Scale::STANDARD_DPI),
            size: Cell::new(PhysicalSize::default()),
            minimum_size: options.minimum_size,
            cursor: Cell::new(Cursor::Arrow),
            frame: RefCell::default(),
            pointer: RefCell::default(),
            keyboard: RefCell::default(),
            synthetic: Arc::clone(&synthetic),
        });
        let class = wide("GitardoDesktopWindow");
        register_class(&class, window_procedure)?;
        let title = wide(&options.title);
        let window = OwnedWindow::create(&WindowRequest {
            class: &class,
            title: &title,
            style: WINDOW_STYLE,
            parent: 0,
            state: Rc::as_ptr(&state).cast(),
        })?;
        let opened = Window {
            synthetic,
            native: window,
            state,
        };
        opened.prepare(options)?;
        Ok(opened)
    }

    /// Sizes the window for its screen, colors its title bar and shows it, as the options say.
    fn prepare(&self, options: &WindowOptions) -> Result<(), PlatformError> {
        let handle = self.handle();
        self.state.dpi.set(Scale::from_dpi(handle.dpi()).dpi());
        handle.resize_inside(self.scale().to_physical(options.size))?;
        self.state.size.set(handle.client_size()?);
        // Before Windows 10 version 2004 the attribute had another number: there the title bar keeps the system's
        // colors, which is still a usable window.
        let _ = handle.set_title_bar(options.title_bar);
        self.synthetic.open(handle);
        if options.visibility == Visibility::Shown {
            handle.show();
        }
        Ok(())
    }

    /// Its name in its event loop, as events give it.
    #[must_use]
    pub fn id(&self) -> WindowId {
        self.state.id
    }

    /// Changes the text of its title bar.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused the text.
    pub fn set_title(&self, title: &str) -> Result<(), PlatformError> {
        self.handle().set_text(title)
    }

    /// The size of its inside, in physical pixels: the size of the frame to draw.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system could not say.
    pub fn size(&self) -> Result<PhysicalSize, PlatformError> {
        self.handle().client_size()
    }

    /// Resizes its inside, in logical pixels at the window's scale.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused the size.
    pub fn set_size(&self, size: LogicalSize) -> Result<(), PlatformError> {
        self.handle().resize_inside(self.scale().to_physical(size))
    }

    /// Where the top left corner of its frame is on the screen.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system could not say.
    pub fn position(&self) -> Result<ScreenPosition, PlatformError> {
        self.handle().position()
    }

    /// Moves its frame on the screen.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused the move.
    pub fn set_position(&self, position: ScreenPosition) -> Result<(), PlatformError> {
        self.handle().move_to(position)
    }

    /// The scale of the screen it is on.
    #[must_use]
    pub fn scale(&self) -> Scale {
        self.state.scale()
    }

    /// Shows it.
    pub fn show(&self) {
        self.handle().show();
    }

    /// Hides it.
    pub fn hide(&self) {
        self.handle().hide();
    }

    /// Makes its title bar dark or light, as the theme of the program is.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system has no dark title bars, before Windows 10 version 2004.
    pub fn set_title_bar(&self, appearance: Appearance) -> Result<(), PlatformError> {
        self.handle().set_title_bar(appearance)
    }

    /// The shape of the pointer over its inside, from now on.
    pub fn set_cursor(&self, cursor: Cursor) {
        let previous = self.state.cursor.replace(cursor);
        if previous != cursor {
            cursor::show(cursor);
        }
    }

    /// Where the caret is, in logical pixels: the input method shows the text being composed there.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the input method refused the place.
    pub fn set_text_input_area(&self, caret: Point) -> Result<(), PlatformError> {
        let factor = self.scale().factor();
        let (x, y) = (
            round_to_coordinate(caret.x * factor),
            round_to_coordinate(caret.y * factor),
        );
        ime::set_composition_point(self.handle(), x, y)
    }

    /// Asks for an `InputEvent::RedrawRequested`, handed over once no other event is waiting.
    pub fn request_redraw(&self) {
        self.state.sink.request_redraw(self.state.id);
    }

    /// Shows a frame: the rectangles that changed are copied into the window and painted at once; a frame of a new
    /// size is shown whole. The window keeps it, to paint again whatever the system asks.
    ///
    /// # Errors
    ///
    /// `PlatformError::FrameSize` when the pixels do not fill the frame's size; `PlatformError::System` when the
    /// system refused to paint.
    pub fn present(&self, frame: Frame<'_>, dirty: &[PixelRect]) -> Result<(), PlatformError> {
        present::present(&self.state, self.handle(), frame, dirty)
    }

    /// A sender of synthetic input for this window, which other threads can use.
    #[must_use]
    pub fn input_sender(&self) -> InputSender {
        InputSender::new(Arc::clone(&self.synthetic))
    }

    /// Feeds an event into this window as if it came from the system: it reaches the handler through the loop, after
    /// what the system already queued.
    ///
    /// # Errors
    ///
    /// `PlatformError::System` when the system refused to wake the loop.
    pub fn send_input(&self, event: InputEvent) -> Result<(), PlatformError> {
        self.synthetic.post(event)
    }

    /// Its handle.
    fn handle(&self) -> WindowHandle {
        self.native.handle()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.synthetic.close();
    }
}
