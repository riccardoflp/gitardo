//! The toolkit of Gitardo Desktop: layout, widgets, lists, menus, dialogs, pickers, drag and drop, focus and find in page.
//!
//! The toolkit is immediate: a view is a function `fn(state: &State, ui: &mut Ui) -> Vec<Intent>` that declares its
//! widgets on a [`Ui`] every frame, each in a rectangle cut from its parent's (see [`layout`]), and reads what happened
//! to each from what it returns. The view keeps no widget objects and changes no state; what the toolkit itself must
//! remember between frames (the focus, scroll offsets, the text being typed, selections) is in a [`UiMemory`].
//!
//! A frame goes:
//!
//! 1. The application builds a [`Ui`] from a [`FrameInput`]: the frame buffer, the fonts, the memory, the
//!    [`InputEvent`]s since the last frame and the [`FrameSettings`] (theme, typefaces, scale, time).
//! 2. The input is sent to the widgets as the last frame laid them out: the pointer to the topmost one under it, the
//!    keys to the one with the focus.
//! 3. The view declares its widgets; each returns a [`Response`] or a richer answer, and the view turns those into
//!    intents.
//! 4. [`Ui::finish`] draws, layer by layer, only the parts of the window whose shapes changed since the last frame, and
//!    returns a [`FrameOutput`]: the cursor, text for the clipboard, when to draw again, where the caret is, and the
//!    regions drawn.
//!
//! ```
//! # use ui::{Ui, Button, layout::{Area, columns, Size}};
//! /// What the view wants, in the words of the domain.
//! enum Intent { Fetch, Search(String) }
//!
//! /// A toolbar: a button and a search box.
//! fn toolbar(search: &str, ui: &mut Ui<'_>) -> Vec<Intent> {
//!     let mut intents = Vec::new();
//!     let sizes = ui.theme().sizes;
//!     let fetch_button = Button::primary("Fetch");
//!     let (fetch_width, height) = ui.button_size(&fetch_button);
//!     let bar = Area::new(ui.window()).top(height + 2.0 * sizes.spacing.small);
//!     let [fetch, field] = columns(bar, sizes.spacing.medium, [Size::Fixed(fetch_width), Size::Fill(1.0)]);
//!     if ui.button("fetch", fetch, &fetch_button).clicked {
//!         intents.push(Intent::Fetch);
//!     }
//!     let typed = ui.input_box("search", field, &ui::InputBox::new(search, "Search"));
//!     if let Some(text) = typed.changed_text {
//!         intents.push(Intent::Search(text));
//!     }
//!     intents
//! }
//! ```

#![forbid(unsafe_code)]

mod access;
mod frame;
pub mod gallery;
mod id;
mod input;
mod interaction;
mod layer;
pub mod layout;
mod memory;
mod number;
mod paint;
mod widget;

/// The types of `render` that the toolkit's own interface takes and gives (rectangles, points, colors, icons, themes,
/// and the frame buffer and fonts a frame is drawn into), so that the views, which use the toolkit and not `render`
/// itself, can name them.
pub use render::{
    Appearance, Assets, Border, CodeKind, Color, FillRule, FontFamily, FontWeight, FrameBuffer, Icon, LineCap, Path,
    Point, Rect, Stroke, TextRun, Theme, ThemeName,
};

/// The text editor the editor of code edits: its documents, cursors and commands, which the views and the store name.
pub use editor;

pub use access::{AccessRole, Description};
pub use frame::{FrameInput, FrameOutput, FrameSettings, TextKind, Typefaces, Ui};
pub use id::WidgetId;
pub use input::{
    Cursor, InputEvent, Key, KeyEvent, Modifiers, NamedKey, PointerButton, PointerButtonEvent, PointerMove, WheelDelta,
    WheelEvent,
};
pub use interaction::{Response, Sense, WidgetEvent};
pub use layer::Layer;
pub use memory::UiMemory;
pub use widget::badge::{Badge, BadgeFill};
pub use widget::button::{Button, ButtonKind};
pub use widget::choice::{Choice, ChoiceKind};
pub use widget::code_editor::{
    CodeEditor, CodeEditorResponse, CodePlaces, CodeRows, CodeSpan, KeyCommands, LineWrap, code_columns, key_commands,
};
pub use widget::dialog::{
    ConfirmAnswer, ConfirmStyle, Confirmation, Dialog, DialogBody, DialogButton, DialogResponse, DialogRole,
};
pub use widget::drag::{DragSourceResponse, DropTargetResponse};
pub use widget::find::{FindResponse, ListSearch, MatchCase};
pub use widget::input_box::{EditableLine, Extent, InputBox, InputBoxResponse, Message, Movement, Selecting, Severity};
pub use widget::label::Label;
pub use widget::list::{List, ListResponse, Reveal, RevealPlace, Row, RowDrag, Selection, SelectionMode, Steering};
pub use widget::menu::{MenuAnchor, MenuEntry, MenuItem};
pub use widget::panel::Axis;
pub use widget::picker::fuzzy::{self, FuzzyMatch};
pub use widget::picker::{Picker, PickerAnswer, PickerItem, PickerMode};
pub use widget::popover::{Popover, PopoverResponse};
pub use widget::scroll::{ScrollAxis, ScrollResponse};
pub use widget::splitter::{Split, SplitResponse, Splitter};
pub use widget::{Availability, Tone};
