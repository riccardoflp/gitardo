//! Drag and drop inside the window: a drag source starts a drag once the pointer pressed on it has moved past a few
//! pixels, its label then follows the pointer above everything, a drop target lights up while the drag is over it and
//! is told what was dropped on it when the button is released there, and Escape cancels the drag.

use std::hash::Hash;

use render::{Border, Point, Rect, TextRun};

use crate::frame::{TextKind, Ui};
use crate::id::WidgetId;
use crate::input::{Cursor, Key, NamedKey, PointerButton};
use crate::interaction::{Response, Routed, Sense, WidgetEvent};
use crate::layer::Layer;
use crate::memory::UiMemory;

/// How much of the accent color lights a drop target a drag is over, out of 255: enough to show, little enough to
/// keep what the target shows readable under it.
const DROP_LIGHT: u8 = 40;

/// A drag that started on a drag source, which the toolkit remembers until the button is released.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DragAndDrop {
    /// The drag source.
    pub(crate) source: WidgetId,
    /// What follows the pointer while it is dragged.
    pub(crate) label: String,
    /// Where the pointer was pressed.
    pub(crate) start: Point,
    /// Whether the pointer moved far enough for this to be a drag rather than a click.
    pub(crate) is_dragging: bool,
    /// Where the button was released in this frame, which drops what is dragged there.
    pub(crate) dropped_at: Option<Point>,
}

/// What a drag source tells its view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragSourceResponse {
    /// What happened to the source as a widget.
    pub response: Response,
    /// Whether it is being dragged.
    pub is_dragging: bool,
}

/// What a drop target tells its view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropTargetResponse {
    /// Whether something is dragged over it now.
    pub is_hovered: bool,
    /// The drag source dropped on it in this frame.
    pub dropped: Option<WidgetId>,
}

/// Before the view of a frame runs: a release of the button that dragged notes where it was dropped, a release before
/// the drag started ends it, and Escape cancels the drag.
pub(crate) fn start_frame(memory: &mut UiMemory, routed: &mut Routed) {
    let Some(drag) = memory.drag_and_drop.as_mut() else {
        return;
    };
    let escape = routed
        .keys
        .iter()
        .zip(routed.key_routes.iter_mut())
        .find(|(key, route)| key.key == Key::Named(NamedKey::Escape) && !route.is_claimed);
    if let (true, Some((_, route))) = (drag.is_dragging, escape) {
        route.is_claimed = true;
        memory.drag_and_drop = None;
        return;
    }
    let released = routed.events.get(&drag.source).and_then(|events| {
        events.iter().find_map(|event| match event {
            WidgetEvent::Released {
                button: PointerButton::Primary,
                position,
                ..
            } => Some(*position),
            _ => None,
        })
    });
    match released {
        Some(position) if drag.is_dragging => drag.dropped_at = Some(position),
        Some(_) => memory.drag_and_drop = None,
        None => {}
    }
}

impl Ui<'_> {
    /// Declares something that can be dragged, with the words that follow the pointer while it is.
    pub fn drag_source(&mut self, key: impl Hash, rect: Rect, label: &str) -> DragSourceResponse {
        let id = self.id(key);
        let (response, events) = self.interact_with_events(id, rect, Sense::CLICK | Sense::DRAG);
        let threshold = self.theme().sizes.spacing.small;
        for event in events {
            match event {
                WidgetEvent::Pressed {
                    button: PointerButton::Primary,
                    position,
                    ..
                } => {
                    self.memory().drag_and_drop = Some(DragAndDrop {
                        source: id,
                        label: label.to_owned(),
                        start: position,
                        is_dragging: false,
                        dropped_at: None,
                    });
                }
                WidgetEvent::Dragged { position, .. } => {
                    if let Some(drag) = self.memory().drag_and_drop.as_mut().filter(|drag| drag.source == id) {
                        let moved = Point::new(position.x - drag.start.x, position.y - drag.start.y);
                        drag.is_dragging |= moved.length() > threshold;
                    }
                }
                _ => {}
            }
        }
        if response.hovered {
            self.set_cursor(Cursor::Grab);
        }
        DragSourceResponse {
            response,
            is_dragging: self.dragged() == Some(id),
        }
    }

    /// The drag source being dragged, if any.
    #[must_use]
    pub fn dragged(&self) -> Option<WidgetId> {
        self.memory_view()
            .drag_and_drop
            .as_ref()
            .filter(|drag| drag.is_dragging && drag.dropped_at.is_none())
            .map(|drag| drag.source)
    }

    /// Declares a place things can be dropped on: it lights up while a drag is over it, and says what was dropped on
    /// it in the frame the button is released over it.
    pub fn drop_target(&mut self, rect: Rect) -> DropTargetResponse {
        let visible = rect.intersection(self.current_clip());
        let drag = self.memory_view().drag_and_drop.clone().filter(|drag| drag.is_dragging);
        let Some(drag) = drag else {
            return DropTargetResponse {
                is_hovered: false,
                dropped: None,
            };
        };
        let dropped = drag.dropped_at.filter(|at| visible.contains(*at)).map(|_| drag.source);
        let is_hovered = drag.dropped_at.is_none() && self.pointer().is_some_and(|at| visible.contains(at));
        if is_hovered {
            let theme = *self.theme();
            self.fill_rect(rect, theme.colors.accent.with_alpha(DROP_LIGHT));
            let border = Border {
                width: theme.sizes.focus_ring * 2.0,
                radius: 0.0,
                color: theme.colors.focus_ring,
            };
            self.border(rect, border);
        }
        DropTargetResponse { is_hovered, dropped }
    }

    /// At the end of a frame: ends a drag that was dropped or whose press is gone, and draws the label of a drag in
    /// progress beside the pointer, above everything.
    pub(crate) fn finish_drag(&mut self) {
        let is_over = self.memory_view().drag_and_drop.as_ref().is_some_and(|drag| {
            drag.dropped_at.is_some()
                || self
                    .memory_view()
                    .interaction
                    .active
                    .is_none_or(|active| active.id != drag.source)
        });
        if is_over {
            self.memory().drag_and_drop = None;
            return;
        }
        let Some(label) = self
            .memory_view()
            .drag_and_drop
            .as_ref()
            .filter(|drag| drag.is_dragging)
            .map(|drag| drag.label.clone())
        else {
            return;
        };
        let Some(pointer) = self.pointer() else {
            return;
        };
        self.set_cursor(Cursor::Grab);
        self.drag_image(&label, pointer);
    }

    /// The words of what is dragged, in a box beside the pointer.
    fn drag_image(&mut self, label: &str, pointer: Point) {
        let theme = *self.theme();
        let style = self.text_style(TextKind::Body);
        let padding = theme.sizes.spacing.small + theme.sizes.spacing.tiny;
        let size = (
            self.measure(label, style) + 2.0 * padding,
            self.line_height(style) + 2.0 * padding,
        );
        let gap = theme.sizes.spacing.large;
        let rect = Rect::new(pointer.x + gap, pointer.y + gap, size.0, size.1);
        self.layer(Layer::Tooltip, |ui| {
            let radius = theme.sizes.radius.medium;
            ui.fill_rounded_rect(rect, radius, theme.colors.menu_selection);
            let border = Border {
                width: theme.sizes.border,
                radius,
                color: theme.colors.border,
            };
            ui.border(rect, border);
            let run = TextRun {
                text: label,
                style,
                color: theme.colors.menu_selection_foreground,
                origin: Point::new(rect.x + padding, rect.y + padding),
                max_width: None,
            };
            let _ = ui.text(&run);
        });
    }
}
