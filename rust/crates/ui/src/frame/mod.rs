//! The frame cycle: a [`Ui`] is made for one frame from the frame buffer, the input since the last frame, the time and
//! the [`UiMemory`]; the view declares its widgets on it, which react to the input and say what happened; and
//! [`Ui::finish`] draws what changed and says what the application should do next, in a [`FrameOutput`].

mod file_icon;
mod finish;
mod keys;
mod overlay;
mod paint;
mod raised;

pub use file_icon::FileIconStyle;
pub(crate) use overlay::OverlayState;

use std::time::Duration;

use render::{Assets, FrameBuffer, PixelRect, Rect, TextStyle, Theme, TypefaceId};

use crate::id::WidgetId;
use crate::input::{Cursor, InputEvent};
use crate::interaction::{Hit, Hits, Response, RouteSettings, Routed, Sense, WidgetEvent, route};
use crate::layer::Layer;
use crate::memory::UiMemory;
use crate::paint::{DisplayList, Shape};

/// The typefaces the toolkit draws with, made by the application from the fonts it loaded: Inter at three weights
/// and `JetBrains Mono`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Typefaces {
    /// The text of the interface: Inter, regular.
    pub sans: TypefaceId,
    /// Names that stand out a little, such as the subject of a commit or a branch: Inter, medium (500).
    pub medium: TypefaceId,
    /// Headings, the main button and what stands out: Inter, semibold (600).
    pub bold: TypefaceId,
    /// Code, hashes and the command log.
    pub mono: TypefaceId,
}

/// The kinds of text of the interface, each with its typeface and its size from the theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextKind {
    /// The text of the interface.
    Body,
    /// Names that stand out a little: the subject of a commit, a branch, a person.
    Medium,
    /// The text of the interface that stands out.
    Strong,
    /// The second line of a row and what is said beside a control: an author and a date, a count, what runs.
    Detail,
    /// Small print.
    Small,
    /// Small print that stands out: the words of pills and the titles of the sections of the branch rail.
    SmallStrong,
    /// The titles of pages, cards and dialogs.
    Heading,
    /// Code and the command log.
    Code,
    /// Hashes beside the words of the interface, at the size of the second line of a row.
    Hash,
}

/// What a frame is drawn with, besides the input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSettings {
    /// The colors and sizes.
    pub theme: Theme,
    /// The typefaces.
    pub typefaces: Typefaces,
    /// Physical pixels for each logical pixel.
    pub scale: f32,
    /// The time of the frame, from a moment the application chose, such as its start: animations, the blinking of
    /// the caret and the delay of tooltips are measured with it, so that tests can give any time they like.
    pub now: Duration,
    /// How the icons of files and folders are drawn (`fileIcons`).
    pub file_icons: FileIconStyle,
}

/// Everything a frame is made from.
#[derive(Debug)]
pub struct FrameInput<'frame> {
    /// The pixels of the window, which keep what the last frame drew.
    pub frame: &'frame mut FrameBuffer,
    /// The fonts and the glyphs and icons drawn so far.
    pub assets: &'frame mut Assets,
    /// What the toolkit keeps between frames.
    pub memory: &'frame mut UiMemory,
    /// What happened since the last frame, in order.
    pub events: &'frame [InputEvent],
    /// The theme, the typefaces, the scale and the time.
    pub settings: FrameSettings,
}

/// What a frame asks of the application once it is drawn.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameOutput {
    /// The shape of the pointer to show.
    pub cursor: Cursor,
    /// Text to put on the clipboard, copied or cut in this frame.
    pub clipboard: Option<String>,
    /// When to draw a frame again even if no input comes, for the caret, a tooltip, a scroll or a spinner; a time at
    /// or before the frame's means at once. Nothing when nothing moves.
    pub next_frame: Option<Duration>,
    /// Where the caret of the input box with the focus is, in logical pixels, for the window of the input method.
    pub caret: Option<Rect>,
    /// The parts of the frame buffer drawn again, in physical pixels: what to show in the window.
    pub dirty: Vec<PixelRect>,
}

/// A tooltip asked for in a frame, drawn at its end if the pointer stayed long enough.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TooltipRequest {
    /// The widget it explains.
    pub(crate) id: WidgetId,
    /// What it says.
    pub(crate) text: String,
}

/// One frame of the toolkit: the view declares its widgets on it, in logical pixels, and each says what happened to
/// it.
#[derive(Debug)]
pub struct Ui<'frame> {
    /// The pixels of the window.
    frame: &'frame mut FrameBuffer,
    /// The fonts, glyphs and icons.
    assets: &'frame mut Assets,
    /// What is kept between frames.
    memory: &'frame mut UiMemory,
    /// The theme, the typefaces, the scale and the time.
    settings: FrameSettings,
    /// The whole window, in logical pixels.
    window: Rect,
    /// The scopes of names opened, the innermost last.
    scopes: Vec<WidgetId>,
    /// The clips opened, the innermost last, each inside the one before.
    clips: Vec<Rect>,
    /// The layer drawn into.
    layer: Layer,
    /// How opaque what is drawn now is, out of 255 (`Ui::faded`).
    opacity: u8,
    /// The shapes declared.
    display: DisplayList,
    /// The widgets declared that take input.
    hits: Hits,
    /// The input of the frame, by widget.
    routed: Routed,
    /// The widget under the pointer when the frame started.
    hovered_at_start: Option<WidgetId>,
    /// What the frame asks of the application.
    output: FrameOutput,
    /// The tooltip asked for.
    tooltip: Option<TooltipRequest>,
    /// The overlays declared in this frame, such as dialogs and pickers, to give the focus back when one closes.
    overlays: Vec<WidgetId>,
    /// The text drawn in this frame, kept while find in page is open.
    pub(crate) recording: crate::widget::find::Recording,
    /// The widgets described for the screen reader in this frame.
    pub(crate) access: crate::access::AccessRecorder,
}

impl<'frame> Ui<'frame> {
    /// Starts a frame: sorts its input by widget, as the last frame laid them out, and fills the window with its
    /// background.
    #[must_use]
    pub fn new(input: FrameInput<'frame>) -> Ui<'frame> {
        let FrameInput {
            frame,
            assets,
            memory,
            events,
            settings,
        } = input;
        let route_settings = RouteSettings {
            line_height: settings.theme.sizes.row_height,
            now: settings.now,
        };
        let mut routed = route(&mut memory.interaction, events, route_settings);
        for (id, event) in memory.access.pending.drain(..) {
            routed.events.entry(id).or_default().push(event);
        }
        crate::widget::drag::start_frame(memory, &mut routed);
        let recording = crate::widget::find::Recording::new(memory.find.is_searching());
        let scale = if settings.scale.is_finite() && settings.scale > 0.0 {
            settings.scale
        } else {
            1.0
        };
        let window = Rect::new(
            0.0,
            0.0,
            float_of_pixels(frame.width()) / scale,
            float_of_pixels(frame.height()) / scale,
        );
        let mut ui = Ui {
            frame,
            assets,
            hovered_at_start: memory.interaction.hovered,
            memory,
            settings: FrameSettings { scale, ..settings },
            window,
            scopes: Vec::new(),
            clips: Vec::new(),
            layer: Layer::Base,
            opacity: u8::MAX,
            display: DisplayList::default(),
            hits: Hits::default(),
            routed,
            output: FrameOutput::default(),
            tooltip: None,
            overlays: Vec::new(),
            recording,
            access: crate::access::AccessRecorder::default(),
        };
        ui.fill_rect(window, ui.settings.theme.colors.window_background);
        ui
    }

    /// The whole window, in logical pixels.
    #[must_use]
    pub fn window(&self) -> Rect {
        self.window
    }

    /// The colors and sizes.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.settings.theme
    }

    /// The time of the frame.
    #[must_use]
    pub fn now(&self) -> Duration {
        self.settings.now
    }

    /// The style of a kind of text: its typeface and its size from the theme.
    #[must_use]
    pub fn text_style(&self, kind: TextKind) -> TextStyle {
        let typefaces = self.settings.typefaces;
        let sizes = self.settings.theme.sizes.font;
        let (typeface, size) = match kind {
            TextKind::Body => (typefaces.sans, sizes.body),
            TextKind::Medium => (typefaces.medium, sizes.body),
            TextKind::Strong => (typefaces.bold, sizes.body),
            TextKind::Detail => (typefaces.sans, sizes.detail),
            TextKind::Small => (typefaces.sans, sizes.small),
            TextKind::SmallStrong => (typefaces.bold, sizes.small),
            TextKind::Heading => (typefaces.bold, sizes.heading),
            TextKind::Code => (typefaces.mono, sizes.code),
            TextKind::Hash => (typefaces.mono, sizes.detail),
        };
        TextStyle { typeface, size }
    }

    /// The width of a line of text, in logical pixels.
    #[must_use]
    pub fn measure(&self, text: &str, style: TextStyle) -> f32 {
        self.assets.fonts.measure(text, style)
    }

    /// The height of a line of text, in logical pixels.
    #[must_use]
    pub fn line_height(&self, style: TextStyle) -> f32 {
        self.assets.fonts.line_metrics(style).height
    }

    /// The fonts, to measure, cut or wrap text.
    #[must_use]
    pub fn fonts(&self) -> &render::Fonts {
        &self.assets.fonts
    }

    /// The name of a widget by its key, in the scope open.
    #[must_use]
    pub fn id(&self, key: impl std::hash::Hash) -> WidgetId {
        self.scope_id().with(key)
    }

    /// Declares widgets inside a scope of names, so that the same keys can be used again in another scope, such as
    /// the rows of a list.
    pub fn scope<R>(&mut self, key: impl std::hash::Hash, inside: impl FnOnce(&mut Ui<'frame>) -> R) -> R {
        let id = self.id(key);
        self.scopes.push(id);
        let result = inside(self);
        let _ = self.scopes.pop();
        result
    }

    /// Declares widgets inside the scope of names of a widget already named, so that their names are made from its.
    pub(crate) fn within<R>(&mut self, id: WidgetId, inside: impl FnOnce(&mut Ui<'frame>) -> R) -> R {
        self.scopes.push(id);
        let result = inside(self);
        let _ = self.scopes.pop();
        result
    }

    /// Declares widgets on a layer: shapes drawn above those of the layers below, and widgets that take the pointer
    /// before them.
    pub fn layer<R>(&mut self, layer: Layer, inside: impl FnOnce(&mut Ui<'frame>) -> R) -> R {
        let before = std::mem::replace(&mut self.layer, layer);
        // What opens above, such as a menu or a dialog, is not faded with what opened it.
        let opacity = std::mem::replace(&mut self.opacity, u8::MAX);
        if layer == Layer::Modal {
            self.hits.mark_modal();
        }
        let clips = std::mem::take(&mut self.clips);
        let result = inside(self);
        self.clips = clips;
        self.layer = before;
        self.opacity = opacity;
        result
    }

    /// Declares widgets that are seen only inside a rectangle, within the clip already open.
    pub fn clip<R>(&mut self, rect: Rect, inside: impl FnOnce(&mut Ui<'frame>) -> R) -> R {
        let clip = self.current_clip().intersection(rect);
        self.clips.push(clip);
        let result = inside(self);
        let _ = self.clips.pop();
        result
    }

    /// The rectangle widgets are seen in now, in logical pixels.
    #[must_use]
    pub fn current_clip(&self) -> Rect {
        self.clips.last().copied().unwrap_or(self.window)
    }

    /// Declares a widget that takes input, where it is and what it reacts to, and says what happened to it. The keys
    /// it reacts to, Enter and Space for a click and the key of context menus, are no longer the view's shortcuts.
    pub fn interact(&mut self, id: WidgetId, rect: Rect, sense: Sense) -> Response {
        let response = self.respond(id, rect, sense);
        self.claim_response_keys(id, sense);
        response
    }

    /// Declares a widget that takes input and takes the events it was given, one by one, for widgets that read them
    /// themselves, such as input boxes, lists and the graph. Such a widget says which keys it used with
    /// [`Ui::claim_key`].
    pub fn interact_with_events(&mut self, id: WidgetId, rect: Rect, sense: Sense) -> (Response, Vec<WidgetEvent>) {
        let response = self.respond(id, rect, sense);
        let events = self.routed.events.remove(&id).unwrap_or_default();
        (response, events)
    }

    /// Declares a widget and says what happened to it, leaving its keys to whoever reads them.
    fn respond(&mut self, id: WidgetId, rect: Rect, sense: Sense) -> Response {
        self.register(id, rect, (sense, false));
        let events = self.routed.events.get(&id).map_or(&[][..], Vec::as_slice);
        Response::new(id, rect, sense, events, &self.memory.interaction)
    }

    /// Where the pointer is, when it is over the window.
    #[must_use]
    pub fn pointer(&self) -> Option<render::Point> {
        self.memory.interaction.pointer
    }

    /// The modifier keys held, as the last input said.
    #[must_use]
    pub fn modifiers(&self) -> crate::input::Modifiers {
        self.memory.interaction.modifiers
    }

    /// Since when, in the time of the frames, the pointer has rested on a widget; none while it is not over it.
    #[must_use]
    pub fn hovered_since(&self, response: &Response) -> Option<Duration> {
        self.memory
            .interaction
            .hover_since
            .filter(|(id, _)| *id == response.id && response.hovered)
            .map(|(_, since)| since)
    }

    /// Whether the window has the keyboard.
    #[must_use]
    pub fn is_window_focused(&self) -> bool {
        self.memory.interaction.window_focused
    }

    /// Gives the keyboard focus to a widget, from this point of the frame on: for a dialog that opens on its input box.
    pub fn request_focus(&mut self, id: WidgetId) {
        self.memory.interaction.focus = Some(id);
    }

    /// Whether the pointer is about a widget: under it, or, while the main button is held, where it was pressed, even
    /// on a widget that takes no press, such as a label of a row a drag starts from.
    #[must_use]
    pub fn is_pointed_at(&self, response: &Response) -> bool {
        let interaction = &self.memory.interaction;
        match interaction.active {
            Some(_) => interaction.pressed_on == Some(response.id),
            None => response.hovered,
        }
    }

    /// Asks for the pointer to have a shape while it is where this frame says.
    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.output.cursor = cursor;
    }

    /// Puts text on the clipboard at the end of the frame.
    pub fn copy_to_clipboard(&mut self, text: String) {
        self.output.clipboard = Some(text);
    }

    /// Says where the caret of the input box with the focus is, for the window of the input method.
    pub fn set_caret(&mut self, rect: Rect) {
        self.output.caret = Some(rect);
    }

    /// Asks for a frame to be drawn again at a time even if no input comes.
    pub fn request_frame_at(&mut self, time: Duration) {
        self.output.next_frame = Some(self.output.next_frame.map_or(time, |earlier| earlier.min(time)));
    }

    /// Asks for a tooltip over a widget, shown when the pointer has stayed over it long enough.
    pub fn tooltip(&mut self, response: &Response, text: &str) {
        if response.hovered && !response.pressed {
            self.tooltip = Some(TooltipRequest {
                id: response.id,
                text: text.to_owned(),
            });
        }
    }

    /// Physical pixels for each logical pixel.
    pub(crate) fn scale(&self) -> f32 {
        self.settings.scale
    }

    /// The layer drawn into now.
    pub(crate) fn current_layer(&self) -> Layer {
        self.layer
    }

    /// The widgets declared so far that take input.
    pub(crate) fn hits(&self) -> &Hits {
        &self.hits
    }

    /// What is kept between frames.
    pub(crate) fn memory(&mut self) -> &mut UiMemory {
        self.memory
    }

    /// What is kept between frames, to read.
    pub(crate) fn memory_view(&self) -> &UiMemory {
        self.memory
    }

    /// The scope names are made in now.
    fn scope_id(&self) -> WidgetId {
        self.scopes.last().copied().unwrap_or(WidgetId::ROOT)
    }

    /// Notes where a widget is and what it reacts to, for the next frame's input and for Tab.
    fn register(&mut self, id: WidgetId, rect: Rect, (sense, is_raised): (Sense, bool)) {
        let visible = rect.intersection(self.current_clip());
        if !visible.is_empty() && self.layer.takes_input() {
            self.hits.push(Hit {
                id,
                rect: visible,
                layer: self.layer,
                sense,
                is_raised,
            });
        }
    }

    /// Adds a shape to the layer drawn into, when any of it can be seen.
    fn record(&mut self, shape: Shape, bounds: Rect) {
        let clip = self.current_clip();
        let visible = bounds.inset(-1.0).intersection(clip);
        if visible.is_empty() {
            return;
        }
        let command = crate::paint::Command {
            shape: if self.opacity == u8::MAX {
                shape
            } else {
                shape.faded(self.opacity)
            },
            clip,
            bounds: visible,
        };
        self.display.push(self.layer, command);
    }
}

/// A size in pixels as a float: frames are far smaller than where `f32` stops being exact.
fn float_of_pixels(pixels: u32) -> f32 {
    crate::number::float_of_count(usize::try_from(pixels).unwrap_or(usize::MAX))
}
