//! A window of the platform with what the toolkit needs to draw in it: its frame buffer, its fonts, the memory of the
//! toolkit, its theme, the input waiting for the next frame and the timer of the frame the last one asked for.

use std::time::{Duration, Instant};

use platform::accesskit::{Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate};
use platform::{
    AccessibilityRequest, Clipboard, Context, Event, Frame, InputEvent, LogicalSize, PhysicalSize, SystemTheme,
    TextInputArea, TimerId, Visibility, Window, WindowOptions,
};
use render::{Assets, FrameBuffer, Theme, ThemeName};
use ui::{FrameInput, FrameOutput, FrameSettings, Typefaces, Ui, UiMemory};

use super::fonts::{chosen_typefaces, load_system_fonts};
use super::frame::{FrameParts, decorations};
use super::input::{is_paste, system_cursor, toolkit_event};
use super::theme::{resolve_theme, scale_text, title_bar};
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
    /// Who draws its title bar (`window.title-bar`).
    pub title_bar: views::window::TitleBar,
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

/// The node of the window itself in the tree given to the screen reader.
const WINDOW_NODE: NodeId = NodeId(0);

/// A window that shows a view of the toolkit.
pub struct Surface {
    /// The window.
    window: Window,
    /// The text of its title bar, which the screen reader says.
    title: String,
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
    /// The theme drawn, its text scaled.
    theme: Theme,
    /// How much larger the text is drawn than the theme says: `font.size` over its default.
    text_scale: f32,
    /// The timer of the next frame.
    frame_timer: TimerId,
    /// Where the caret was in the last frame, given to the input method.
    caret: Option<render::Rect>,
    /// How many frames were drawn.
    frames_drawn: u64,
    /// Whether a screen reader listens, from its first question until it goes away.
    is_reader_listening: bool,
    /// The tree the screen reader was given last, so that a frame that changed nothing of it sends nothing.
    told_reader: Option<TreeUpdate>,
    /// What each place of the window is, as the last frame drew it, for a title bar the window draws.
    frame_parts: FrameParts,
}

impl Surface {
    /// Opens a window and loads the fonts for it, once the loop runs. It asks for its first frame.
    ///
    /// # Errors
    ///
    /// `AppError::Platform` when the window could not open; the errors of [`load_system_fonts`] when the fonts
    /// could not be loaded.
    pub fn open(context: &Context<'_>, options: SurfaceOptions) -> Result<Surface, AppError> {
        let system = context.system_theme();
        let theme = resolve_theme(options.theme, system);
        let window_options = WindowOptions {
            title: options.title.clone(),
            size: options.size,
            visibility: options.visibility,
            title_bar: title_bar(options.theme, &theme),
            decorations: decorations(options.title_bar),
            ..WindowOptions::default()
        };
        let window = Window::open(context, &window_options)?;
        let frame_parts = FrameParts::default();
        frame_parts.attach(&window);
        let mut assets = Assets::new();
        let typefaces = load_system_fonts(&mut assets)?;
        let size = window.size();
        Ok(Surface {
            window,
            title: options.title,
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
            text_scale: 1.0,
            frame_timer: options.frame_timer,
            caret: None,
            frames_drawn: 0,
            is_reader_listening: false,
            told_reader: None,
            frame_parts,
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

    /// Keeps what the frame just drawn says of the title bar, for the system's questions until the next one
    /// (`window.title-bar`).
    pub fn keep_frame_parts(&self, caption: views::window::CaptionState) {
        let width = self.window.scale().to_logical(self.window.size()).width;
        self.frame_parts.keep(caption, (&self.memory, &self.theme, width));
    }

    /// Changes the theme chosen: the title bar follows, and a frame is asked for.
    pub fn set_theme(&mut self, choice: ThemeName) {
        self.choice = choice;
        self.window
            .set_title_bar(title_bar(choice, &resolve_theme(choice, self.system)));
        self.apply_theme();
    }

    /// Changes the text of the title bar, which the screen reader says too.
    pub fn set_title(&mut self, title: &str) {
        if self.title != title {
            title.clone_into(&mut self.title);
            self.window.set_title(title);
        }
    }

    /// Changes the fonts and the size of the text (`font.ui`, `font.code`, `font.size`): a family the system does not
    /// have gives the program's own. A frame is asked for.
    ///
    /// # Errors
    ///
    /// `AppError::Font` when even the program's fonts are missing.
    pub fn set_text(&mut self, interface: &str, code: &str, scale: f32) -> Result<(), AppError> {
        self.typefaces = chosen_typefaces(&mut self.assets.fonts, interface, code)?;
        self.text_scale = scale;
        let theme = scale_text(resolve_theme(self.choice, self.system), scale);
        self.theme = theme;
        self.window.request_redraw();
        Ok(())
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
            Event::ThemeChanged(system) => {
                self.system = *system;
                self.apply_theme();
                Step::Handled
            }
            Event::Started | Event::Input { .. } | Event::Timer(_) | Event::Message(_) => Step::Ignored,
        }
    }

    /// Draws a frame of a view with the input since the last one, shows what changed of it, and does what it asks:
    /// the shape of the pointer, the place of the caret, text to copy, and when to draw again.
    ///
    /// # Errors
    ///
    /// `AppError::Platform` when the system refused to show the frame.
    pub fn draw<R>(&mut self, context: &Context<'_>, view: impl FnOnce(&mut Ui<'_>) -> R) -> Result<R, AppError> {
        let size = self.window.size();
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
        self.tell_screen_reader();
        self.present(&output)?;
        self.follow(&output);
        self.schedule(context, output.next_frame, now);
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
            InputEvent::Accessibility(request) => return self.answer_screen_reader(request),
            InputEvent::Resized(_) | InputEvent::ScaleChanged(_) => {}
            _ => self.pending.extend(toolkit_event(event)),
        }
        self.window.request_redraw();
        Step::Handled
    }

    /// Draws in the theme the choice and the system give.
    fn apply_theme(&mut self) {
        let theme = scale_text(resolve_theme(self.choice, self.system), self.text_scale);
        if theme != self.theme {
            self.theme = theme;
            self.window.request_redraw();
        }
    }

    /// Answers what a screen reader asks: the tree of the last frame when it starts listening, and a click or the
    /// focus it asks for, which the next frame acts on as it would on the pointer's or the keyboard's.
    fn answer_screen_reader(&mut self, request: &AccessibilityRequest) -> Step {
        match request {
            AccessibilityRequest::InitialTree => {
                self.is_reader_listening = true;
                let tree = self.reader_tree();
                self.told_reader = Some(tree.clone());
                self.window.update_accessibility(tree);
            }
            AccessibilityRequest::Action(action) => {
                if self.memory.accessibility_action(action) {
                    self.window.request_redraw();
                }
            }
            AccessibilityRequest::Deactivated => {
                self.is_reader_listening = false;
                self.told_reader = None;
            }
        }
        Step::Handled
    }

    /// Gives the screen reader the tree of the frame just drawn, when one listens and the tree changed.
    fn tell_screen_reader(&mut self) {
        if !self.is_reader_listening {
            return;
        }
        let tree = self.reader_tree();
        if self.told_reader.as_ref() != Some(&tree) {
            self.told_reader = Some(tree.clone());
            self.window.update_accessibility(tree);
        }
    }

    /// The tree of the last frame for the screen reader, its window named by the title; the window alone with its
    /// title before the first frame.
    fn reader_tree(&self) -> TreeUpdate {
        if self.frames_drawn == 0 {
            return window_tree(&self.title);
        }
        let mut tree = self.memory.accessibility_update();
        let root = tree.tree.as_ref().map(|info| info.root);
        if let Some((_, window)) = tree.nodes.iter_mut().find(|(id, _)| Some(*id) == root) {
            window.set_label(self.title.as_str());
        }
        tree
    }

    /// Shows the parts of the frame that changed; the platform shows it whole when the window lost what it showed,
    /// so it is asked even when nothing changed.
    fn present(&self, output: &FrameOutput) -> Result<(), AppError> {
        let dirty: Vec<platform::PixelRect> = output.dirty.iter().filter_map(|rect| system_rect(*rect)).collect();
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
            self.window.set_text_input_area(output.caret.map(|caret| TextInputArea {
                position: platform::Point { x: caret.x, y: caret.y },
                size: LogicalSize {
                    width: caret.width,
                    height: caret.height,
                },
            }));
        }
        if let Some(text) = &output.clipboard {
            // Text that could not be copied stays where it was: the person sees nothing on the clipboard and copies
            // again.
            let _ = self.clipboard().map(|clipboard| clipboard.write_text(text));
        }
    }

    /// Asks for the next frame when the last one wants it: at once, or when the timer goes off.
    fn schedule(&self, context: &Context<'_>, next: Option<Duration>, now: Duration) {
        match next {
            None => context.stop_timer(self.frame_timer),
            Some(at) if at <= now => {
                context.stop_timer(self.frame_timer);
                self.window.request_redraw();
            }
            Some(at) => context.start_timer(self.frame_timer, at.saturating_sub(now)),
        }
    }

    /// Puts text on the clipboard; nothing happens when the system has none the program can reach.
    pub fn copy(&mut self, text: &str) {
        let _ = self.clipboard().map(|clipboard| clipboard.write_text(text));
    }

    /// The clipboard, opened the first time it is needed.
    fn clipboard(&mut self) -> Option<&mut Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = Clipboard::new().ok();
        }
        self.clipboard.as_mut()
    }
}

/// The tree of a window with nothing in it but its title, for the screen reader.
fn window_tree(title: &str) -> TreeUpdate {
    let mut window = Node::new(Role::Window);
    window.set_label(title);
    TreeUpdate {
        nodes: vec![(WINDOW_NODE, window)],
        tree: Some(TreeInfo::new(WINDOW_NODE)),
        tree_id: TreeId::ROOT,
        focus: WINDOW_NODE,
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
