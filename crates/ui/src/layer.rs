//! Layers: what is drawn above the rest of the window and takes the pointer first, such as menus, pickers, dialogs and
//! tooltips.
//!
//! A view draws into the layer of the moment, the base layer unless it said otherwise with
//! [`Ui::layer`](crate::Ui::layer). At the end of the frame the layers are drawn from the lowest up, so an overlay may
//! be declared in the middle of the view that opens it and still covers what is declared after it. The pointer goes to
//! the topmost widget under it, a higher layer before a lower one; and while a modal layer is open, nothing below it
//! gets the pointer or the keyboard, and Tab goes round its widgets only.

/// A layer of the window, from the lowest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// The panes, pages and bars of the window.
    #[default]
    Base,
    /// What opens over the base and closes when the pointer is pressed elsewhere: menus, pickers, the message under
    /// an input box.
    Popup,
    /// Dialogs: while one is drawn, the layers below it get no input.
    Modal,
    /// Tooltips, above everything, which never take input.
    Tooltip,
}

impl Layer {
    /// How many layers there are.
    pub(crate) const COUNT: usize = 4;

    /// The place of the layer from the lowest, from zero.
    pub(crate) fn index(self) -> usize {
        match self {
            Layer::Base => 0,
            Layer::Popup => 1,
            Layer::Modal => 2,
            Layer::Tooltip => 3,
        }
    }

    /// Whether widgets on this layer may get the pointer and the keyboard.
    pub(crate) fn takes_input(self) -> bool {
        self != Layer::Tooltip
    }
}
