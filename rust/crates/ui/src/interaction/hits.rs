//! Where the widgets of a frame are and what they react to, to find the one under the pointer: the topmost, a higher
//! layer before a lower one, and in a layer the one declared last.

use std::ops::{BitOr, BitOrAssign};

use render::{Point, Rect};

use crate::id::WidgetId;
use crate::layer::Layer;

/// What a widget reacts to, as a set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sense {
    /// One bit for each kind of input.
    kinds: u8,
}

impl Sense {
    /// Nothing but the pointer being over it, for a tooltip.
    pub const HOVER: Sense = Sense { kinds: 1 };
    /// Clicks: a press and a release over it; Enter and Space when it has the keyboard.
    pub const CLICK: Sense = Sense { kinds: 2 };
    /// Drags: the moves of the pointer after a press on it, wherever the pointer goes, until the release.
    pub const DRAG: Sense = Sense { kinds: 4 };
    /// The keyboard: a click gives it the focus, and Tab goes through it.
    pub const FOCUS: Sense = Sense { kinds: 8 };
    /// The wheel, when it is the topmost widget under the pointer that scrolls.
    pub const SCROLL: Sense = Sense { kinds: 16 };

    /// Whether every kind of `other` is in this set.
    #[must_use]
    pub fn contains(self, other: Sense) -> bool {
        self.kinds & other.kinds == other.kinds
    }

    /// Whether some kind of `other` is in this set.
    #[must_use]
    pub fn intersects(self, other: Sense) -> bool {
        self.kinds & other.kinds != 0
    }
}

impl BitOr for Sense {
    type Output = Sense;

    fn bitor(self, other: Sense) -> Sense {
        Sense {
            kinds: self.kinds | other.kinds,
        }
    }
}

impl BitOrAssign for Sense {
    fn bitor_assign(&mut self, other: Sense) {
        self.kinds |= other.kinds;
    }
}

/// A widget of a frame, where it can be seen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    /// The widget.
    pub(crate) id: WidgetId,
    /// The part of it that can be seen: its rectangle inside the clip it was declared in.
    pub(crate) rect: Rect,
    /// Its layer.
    pub(crate) layer: Layer,
    /// What it reacts to.
    pub(crate) sense: Sense,
    /// Whether it stays above the other widgets of its layer whatever the order they were declared in, such as the
    /// grip of a splitter over the edges of its panes.
    pub(crate) is_raised: bool,
}

/// The widgets of a frame that take input, in the order they are stacked: the lowest layer first, and in each layer
/// the order they were declared in.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Hits {
    /// The widgets, stacked.
    entries: Vec<Hit>,
    /// Whether a modal layer was drawn, which keeps input from the layers below it.
    has_modal: bool,
}

impl Hits {
    /// Adds a widget declared in the frame.
    pub(crate) fn push(&mut self, hit: Hit) {
        self.has_modal |= hit.layer == Layer::Modal;
        self.entries.push(hit);
    }

    /// Notes that a modal layer was drawn, even with no widget on it yet.
    pub(crate) fn mark_modal(&mut self) {
        self.has_modal = true;
    }

    /// Puts the widgets in the order they are stacked, once the frame declared all of them.
    pub(crate) fn stack(&mut self) {
        self.entries.sort_by_key(|hit| (hit.layer, hit.is_raised));
    }

    /// The lowest layer that may take input: the modal layer while one is drawn.
    pub(crate) fn lowest_layer(&self) -> Layer {
        if self.has_modal { Layer::Modal } else { Layer::Base }
    }

    /// The topmost widget under a point that reacts to some kind of `sense`.
    pub(crate) fn topmost(&self, point: Point, sense: Sense) -> Option<&Hit> {
        let lowest = self.lowest_layer();
        self.entries
            .iter()
            .rev()
            .filter(|hit| hit.layer >= lowest && hit.layer.takes_input())
            .find(|hit| hit.rect.contains(point) && hit.sense.intersects(sense))
    }

    /// The topmost widget under a point that reacts to anything, the one that covers the others.
    pub(crate) fn covering(&self, point: Point) -> Option<&Hit> {
        self.topmost(
            point,
            Sense::HOVER | Sense::CLICK | Sense::DRAG | Sense::FOCUS | Sense::SCROLL,
        )
    }

    /// The widgets Tab goes through, in order.
    pub(crate) fn focus_order(&self) -> Vec<WidgetId> {
        let lowest = self.lowest_layer();
        self.entries
            .iter()
            .filter(|hit| hit.layer >= lowest && hit.sense.contains(Sense::FOCUS))
            .map(|hit| hit.id)
            .collect()
    }

    /// Whether a widget was declared and can take the focus.
    pub(crate) fn is_focusable(&self, id: WidgetId) -> bool {
        let lowest = self.lowest_layer();
        self.entries
            .iter()
            .any(|hit| hit.id == id && hit.layer >= lowest && hit.sense.contains(Sense::FOCUS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(key: &str, rect: Rect, layer: Layer, sense: Sense) -> Hit {
        Hit {
            id: WidgetId::ROOT.with(key),
            rect,
            layer,
            sense,
            is_raised: false,
        }
    }

    #[test]
    fn the_topmost_widget_takes_the_pointer_and_a_modal_layer_hides_the_ones_below() {
        let mut hits = Hits::default();
        let whole = Rect::new(0.0, 0.0, 100.0, 100.0);
        hits.push(hit("page", whole, Layer::Base, Sense::CLICK | Sense::FOCUS));
        hits.push(hit("menu", Rect::new(0.0, 0.0, 50.0, 50.0), Layer::Popup, Sense::CLICK));
        hits.push(hit(
            "button",
            Rect::new(0.0, 0.0, 20.0, 20.0),
            Layer::Base,
            Sense::CLICK,
        ));
        hits.stack();
        let at = |hits: &Hits, x: f32, y: f32| hits.covering(Point::new(x, y)).map(|hit| hit.id);
        assert_eq!(at(&hits, 10.0, 10.0), Some(WidgetId::ROOT.with("menu")));
        assert_eq!(at(&hits, 60.0, 60.0), Some(WidgetId::ROOT.with("page")));
        assert_eq!(hits.focus_order(), vec![WidgetId::ROOT.with("page")]);
        hits.push(hit(
            "dialog",
            Rect::new(40.0, 40.0, 10.0, 10.0),
            Layer::Modal,
            Sense::FOCUS,
        ));
        hits.stack();
        assert_eq!(at(&hits, 60.0, 60.0), None);
        assert_eq!(hits.focus_order(), vec![WidgetId::ROOT.with("dialog")]);
    }
}
