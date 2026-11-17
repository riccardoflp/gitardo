//! Popovers: a floating card that opens under a control and holds widgets of its own, such as the list of the branch
//! filter, on the layer of popups. A popover is open while the view declares it, as a dialog is; it says when it
//! should close: when the pointer is pressed outside it and its control, or when the window loses the focus.

use std::hash::Hash;

use render::Rect;

use crate::access::AccessRole;
use crate::frame::Ui;
use crate::interaction::Sense;
use crate::layer::Layer;

/// A popover under a control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Popover<'text> {
    /// The control it opens under, whose presses do not close it.
    pub anchor: Rect,
    /// How wide it is, and how tall it would be with room enough; it is never taller than the room under the control.
    pub size: (f32, f32),
    /// Its name, for the screen reader.
    pub name: &'text str,
    /// Where it opens against its control.
    pub side: PopoverSide,
}

/// Where a popover opens against its control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PopoverSide {
    /// Under it, as tall as the room there is.
    #[default]
    Below,
    /// Under it, or above it when it does not fit under it and there is more room above: never over the control.
    BelowOrAbove,
}

/// What a popover tells its view.
#[derive(Clone, Debug, PartialEq)]
pub struct PopoverResponse<R> {
    /// What the widgets inside it returned.
    pub inner: R,
    /// Where it is.
    pub rect: Rect,
    /// Whether it opened in this frame.
    pub is_new: bool,
    /// Whether it should close: the pointer was pressed outside it and its control, or the window lost the focus.
    pub is_dismissed: bool,
}

impl Ui<'_> {
    /// Declares a popover under its control, and the widgets `inside` declares in it, over the rectangle given and told
    /// whether the popover opened in this frame, to put the keyboard in it.
    pub fn popover<R>(
        &mut self,
        key: impl Hash,
        popover: &Popover<'_>,
        inside: impl FnOnce(&mut Ui<'_>, Rect, bool) -> R,
    ) -> PopoverResponse<R> {
        let id = self.id(key);
        let is_new = self.declare_overlay(id, &[]);
        let rect = self.popover_rect(popover);
        let radius = self.theme().sizes.radius.large;
        let inner = self.layer(Layer::Popup, |ui| {
            let _ = ui.interact(id, rect, Sense::CLICK);
            ui.floating_card(rect, radius);
            ui.within(id, |ui| {
                ui.region("popover", (rect, AccessRole::Dialog, popover.name), |ui| {
                    ui.clip(rect, |ui| inside(ui, rect, is_new))
                })
            })
        });
        let is_dismissed = !is_new && (self.pressed_outside_all(&[rect, popover.anchor]) || !self.is_window_focused());
        PopoverResponse {
            inner,
            rect,
            is_new,
            is_dismissed,
        }
    }

    /// Where a popover goes: under its control, from its left edge, inside the window, as tall as the room there is;
    /// above it instead when its side allows it, it does not fit under it and there is more room above.
    fn popover_rect(&self, popover: &Popover<'_>) -> Rect {
        let window = self.window();
        let spacing = self.theme().sizes.spacing;
        let (width, height) = popover.size;
        let width = width.min(window.width - 2.0 * spacing.medium).max(0.0);
        let x = popover
            .anchor
            .x
            .min(window.right() - spacing.medium - width)
            .max(window.x + spacing.medium);
        let y = popover.anchor.bottom() + spacing.tiny;
        let room = (window.bottom() - spacing.medium - y).max(0.0);
        let above = (popover.anchor.y - spacing.tiny - (window.y + spacing.medium)).max(0.0);
        if popover.side == PopoverSide::BelowOrAbove && height > room && above > room {
            let height = height.min(above);
            return Rect::new(x, popover.anchor.y - spacing.tiny - height, width, height);
        }
        Rect::new(x, y, width, height.min(room))
    }
}
