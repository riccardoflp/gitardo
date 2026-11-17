//! Whether the pointer is within a part of the window that is not a widget of its own, such as a title with the button
//! it shows under the pointer beside it.

use render::Rect;

use super::Ui;

impl Ui<'_> {
    /// Whether the pointer is within a rectangle, as [`Response::contains_pointer`](crate::Response) says of a
    /// widget: in the part of it in sight, with the topmost widget under the pointer drawn inside the rectangle, or
    /// one the rectangle is drawn inside, such as the scroll area around it; not one of another layer, a menu over it,
    /// nor one that lies across it; and not while the main button is held on a widget elsewhere. For what is shown
    /// only under the pointer over a place with no widget of its own.
    #[must_use]
    pub fn is_pointer_within(&self, rect: Rect) -> bool {
        let interaction = &self.memory_view().interaction;
        let in_sight = rect.intersection(self.current_clip());
        let layer = self.current_layer();
        if !interaction.pointer.is_some_and(|point| in_sight.contains(point)) {
            return false;
        }
        match interaction.under {
            Some(under) => under.is_inside(layer, rect) || under.holds(layer, in_sight),
            // No widget is under the pointer: nothing covers the place, unless a dialog keeps the pointer from it.
            None => interaction.active.is_none() && layer >= interaction.hits.lowest_layer(),
        }
    }
}
