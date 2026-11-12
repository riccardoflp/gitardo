//! A window of the platform with what the toolkit needs to draw in it: its frame buffer, its fonts, the memory of the
//! toolkit, its theme, the input waiting for the next frame and the timer of the frame the last one asked for.

use std::time::{Duration, Instant};

use platform::{
    Clipboard, Event, EventLoop, Frame, InputEvent, LogicalSize, PhysicalSize, SystemTheme, TimerId, Visibility,
    Window, WindowOptions,
};
use render::{Assets, FrameBuffer, Theme, ThemeName};
use ui::{FrameInput, FrameOutput, FrameSettings, Typefaces, Ui, UiMemory};

use super::fonts::load_system_fonts;
use super::input::{is_paste, system_cursor, toolkit_event};
use super::theme::{resolve_theme, title_bar};
use crate::error::AppError;

/// What a new window is like.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceOptions {
    /// The text of its title bar.
    pub title: String,
    /// The size of its inside, in logical pixels.
    pub size: LogicalSize,
    /// Whether it shows when it opens.
    pub visibility: Visibility,
    /// The theme chosen, which may be to follow the system.
    pub theme: ThemeName,
    /// The timer it uses to draw a frame when the last one asked for it, unique in its event loop.
    pub frame_timer: TimerId,
}

/// What an event of the loop asks of the program that owns the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The event is not for this window.
    Ignored,
    /// The window took it, and asked for a frame if it needs one.
    Handled,
    /// A frame is to be drawn now, with [`Surface::draw`].
    Draw,
    /// The person asked to close the window.
    Close,
}

/// A window that shows a view of the toolkit.
pub struct Surface {
    /// The window.
    window: Window,
    /// The pixels of the window.
    frame: FrameBuffer,
    /// The fonts and the glyphs drawn so far.
    assets: Assets,
    /// The typefaces of the interface.
    typefaces: Typefaces,
    /// What the toolkit keeps between frames.
    memory: UiMemory,
    /// The input since the last frame.
    pending: Vec<ui::InputEvent>,
    /// The clipboard, once copying or pasting needed it.
    clipboard: Option<Clipboard>,
    /// When the window opened: the time of a frame is counted from it.
    opened: Instant,
    /// The theme chosen.
    choice: ThemeName,
    /// The theme of the system.
    system: SystemTheme,
    /// The theme drawn.
    theme: Theme,
    /// The timer of the next frame.
    frame_timer: TimerId,
    /// Where the caret was in the last frame, given to the input method.
    caret: Option<render::Rect>,
    /// How many frames were drawn.
    frames_drawn: u64,
}

impl Surface {
    /// Opens a window and loads the fonts of the system for it. It asks for its first frame.
    ///
    /// # Errors
    ///
    /// `AppError::Platform` when the window could not open; the errors of [`load_system_fonts`] when the fonts
    /// could not be loaded.
    pub fn open<M: 'static>(event_loop: &EventLoop<M>, options: SurfaceOptions) -> Result<Surface, AppError> {
        let system = platform::system_theme().unwrap_or_default();
        let theme = resolve_theme(options.theme, system);
        let window_options = WindowOptions {
            title: options.title,
            size: options.size,
            visibility: options.visibility,
            title_bar: title_bar(&theme),
            ..WindowOptions::default()
        };
        let window = Window::open(event_loop, &window_options)?;
        let mut assets = Assets::new();
        let typefaces = load_system_fonts(&mut assets)?;
        let size = window.size()?;
        window.request_redraw();
        Ok(Surface {
            window,
            frame: FrameBuffer::new(size.width, size.height),
            assets,
            typefaces,
            memory: UiMemory::new(),
            pending: Vec::new(),
            clipboard: None,
            opened: Instant::now(),
            choice: options.theme,
            system,
            theme,
            frame_timer: options.frame_timer,
            caret: None,
            frames_drawn: 0,
        })
    }

    /// The window.
    #[must_use]
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// What the toolkit keeps between frames.
    #[must_use]
    pub fn memory(&self) -> &UiMemory {
        &self.memory
    }

    /// What the toolkit keeps between frames, for a view to set up before its first frame.
    pub fn memory_mut(&mut self) -> &mut UiMemory {
        &mut self.memory
    }

    /// The theme drawn.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// How many frames were drawn since the window opened.
    #[must_use]
    pub fn frames_drawn(&self) -> u64 {
        self.frames_drawn
    }

    /// Changes the theme chosen: the title bar follows, and a frame is asked for.
    pub fn set_theme(&mut self, choice: ThemeName) {
        self.choice = choice;
        self.apply_theme();
    }

    /// Takes an event of the loop: input for this window, the timer of its next frame, or a change of the theme of
    /// the system. Says what the program is to do next.
    pub fn handle<M>(&mut self, event: &Event<M>) -> Step {
        match event {
            Event::Input { window, event } if *window == self.window.id() => self.handle_input(event),
            Event::Timer(timer) if *timer == self.frame_timer => {
                self.window.request_redraw();
                Step::Handled
            }
            Event::ThemeChanged => {
                self.system = platform::system_theme().unwrap_or(self.system);
                self.apply_theme();
                Step::Handled
            }
            Event::Input { .. } | Event::Timer(_) | Event::Message(_) => Step::Ignored,
        }
    }

    /// Draws a frame of a view with the input since the last one, shows what changed of it, and does what it asks:
    /// the shape of the pointer, the place of the caret, text to copy, and when to draw again.
    ///
    /// # Errors
    ///
    /// `AppError::Platform` when the system refused to show the frame.
    pub fn draw<M: 'static, R>(
        &mut self,
        event_loop: &EventLoop<M>,
        view: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> Result<R, AppError> {
        let size = self.window.size()?;
        if size.width != self.frame.width() || size.height != self.frame.height() {
            self.frame.resize(size.width, size.height);
        }
        let now = self.opened.elapsed();
        let events = std::mem::take(&mut self.pending);
        let settings = FrameSettings {
            theme: self.theme,
            typefaces: self.typefaces,
            scale: self.window.scale().factor(),
            now,
        };
        let mut ui = Ui::new(FrameInput {
            frame: &mut self.frame,
            assets: &mut self.assets,
            memory: &mut self.memory,
            events: &events,
            settings,
        });
        let answer = view(&mut ui);
        let output = ui.finish();
        self.frames_drawn += 1;
        self.present(&output)?;
        self.follow(&output);
        self.schedule(event_loop, output.next_frame, now)?;
        Ok(answer)
    }

    /// Takes input for this window.
    fn handle_input(&mut self, event: &InputEvent) -> Step {
        match event {
            InputEvent::RedrawRequested => return Step::Draw,
            InputEvent::CloseRequested => return Step::Close,
            InputEvent::KeyPressed(key) if is_paste(key) => {
                self.pending.extend(toolkit_event(event));
                if let Some(text) = self
                    .clipboard()
                    .and_then(|clipboard| clipboard.read_text().ok().flatten())
                {
                    self.pending.push(ui::InputEvent::Paste(text));
                }
            }
            InputEvent::Resized(_) | InputEvent::ScaleChanged(_) => {}
            _ => self.pending.extend(toolkit_event(event)),
        }
        self.window.request_redraw();
        Step::Handled
    }

    /// Draws in the theme the choice and the system give, with the title bar to go with it.
    fn apply_theme(&mut self) {
        let theme = resolve_theme(self.choice, self.system);
        if theme.name != self.theme.name {
            self.theme = theme;
            // A system without dark title bars keeps a light one: nothing else to do about it.
            let _ = self.window.set_title_bar(title_bar(&theme));
            self.window.request_redraw();
        }
    }

    /// Shows the parts of the frame that changed.
    fn present(&self, output: &FrameOutput) -> Result<(), AppError> {
        let dirty: Vec<platform::PixelRect> = output.dirty.iter().filter_map(|rect| system_rect(*rect)).collect();
        if dirty.is_empty() {
            return Ok(());
        }
        let frame = Frame {
            pixels: self.frame.pixels(),
            size: PhysicalSize {
                width: self.frame.width(),
                height: self.frame.height(),
            },
        };
        self.window.present(frame, &dirty)?;
        Ok(())
    }

    /// Does what a frame asks of the window: the shape of the pointer, the place of the caret and text to copy.
    fn follow(&mut self, output: &FrameOutput) {
        self.window.set_cursor(system_cursor(output.cursor));
        if output.caret != self.caret {
            self.caret = output.caret;
            if let Some(caret) = output.caret {
                // The input method keeps its last place when it refuses this one, which is the best left to do.
                let _ = self
                    .window
                    .set_text_input_area(platform::Point { x: caret.x, y: caret.y });
            }
        }
        if let Some(text) = &output.clipboard {
            // Text that could not be copied stays where it was: the person sees nothing on the clipboard and copies
            // again.
            let _ = self.clipboard().map(|clipboard| clipboard.write_text(text));
        }
    }

    /// Asks for the next frame when the last one wants it: at once, or when the timer goes off.
    fn schedule<M: 'static>(
        &self,
        event_loop: &EventLoop<M>,
        next: Option<Duration>,
        now: Duration,
    ) -> Result<(), AppError> {
        match next {
            None => event_loop.stop_timer(self.frame_timer),
            Some(at) if at <= now => {
                event_loop.stop_timer(self.frame_timer);
                self.window.request_redraw();
            }
            Some(at) => event_loop.start_timer(self.frame_timer, at.saturating_sub(now))?,
        }
        Ok(())
    }

    /// The clipboard, opened the first time it is needed.
    fn clipboard(&mut self) -> Option<&Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = Clipboard::new().ok();
        }
        self.clipboard.as_ref()
    }
}

/// A rectangle of the frame buffer as the platform takes it, when it covers any pixel.
fn system_rect(rect: render::PixelRect) -> Option<platform::PixelRect> {
    let left = u32::try_from(rect.left.max(0)).ok()?;
    let top = u32::try_from(rect.top.max(0)).ok()?;
    let right = u32::try_from(rect.right.max(0)).ok()?;
    let bottom = u32::try_from(rect.bottom.max(0)).ok()?;
    let system = platform::PixelRect {
        x: left,
        y: top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    };
    (!system.is_empty()).then_some(system)
}
