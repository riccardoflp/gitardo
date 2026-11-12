//! Menus: a context menu opened at the pointer by its button or its key, or a menu opened under a button, with items
//! that have an icon, a label, the text of their shortcut and may be disabled, separators, and submenus that open
//! when the pointer is over their item or with the right arrow. The keys move through it (see [`keys`]), a click
//! outside closes it, and it stays inside the window, turning to the other side where there is no room.
//!
//! Whether a menu is open is the toolkit's to remember: the view opens it with [`Ui::open_menu`] and declares it in
//! every frame with [`Ui::menu`], which draws it while it is open and returns the value of the item chosen.

mod draw;
mod keys;
mod layout;

use std::hash::Hash;

use render::{Icon, Point, Rect};

use crate::frame::Ui;
use crate::id::WidgetId;
use crate::interaction::{Response, WidgetEvent};
use crate::widget::Availability;
use crate::widget::button::Button;
use keys::KeyAction;
pub(crate) use layout::Placement;

/// An entry of a menu, whose items carry a value of the view's, returned when the item is chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuEntry<'text, T> {
    /// An item that does something.
    Item(MenuItem<'text, T>),
    /// An item that opens a submenu.
    Submenu {
        /// Its words.
        label: &'text str,
        /// The icon before them.
        icon: Option<Icon>,
        /// The entries of the submenu.
        entries: &'text [MenuEntry<'text, T>],
    },
    /// A line between groups of items.
    Separator,
}

/// An item of a menu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuItem<'text, T> {
    /// Its words.
    pub label: &'text str,
    /// The icon before them.
    pub icon: Option<Icon>,
    /// The keys that do the same, as text, such as `Ctrl+C`.
    pub shortcut: Option<&'text str>,
    /// Whether it can be chosen now.
    pub availability: Availability,
    /// What the view gets when it is chosen.
    pub value: T,
}

impl<'text, T> MenuItem<'text, T> {
    /// An item with words and a value, with no icon and no shortcut.
    #[must_use]
    pub const fn new(label: &'text str, value: T) -> MenuItem<'text, T> {
        MenuItem {
            label,
            icon: None,
            shortcut: None,
            availability: Availability::Enabled,
            value,
        }
    }

    /// The same item with an icon.
    #[must_use]
    pub fn with_icon(self, icon: Icon) -> MenuItem<'text, T> {
        MenuItem {
            icon: Some(icon),
            ..self
        }
    }

    /// The same item with the text of its shortcut.
    #[must_use]
    pub fn with_shortcut(self, shortcut: &'text str) -> MenuItem<'text, T> {
        MenuItem {
            shortcut: Some(shortcut),
            ..self
        }
    }

    /// The same item, disabled.
    #[must_use]
    pub fn disabled(self) -> MenuItem<'text, T> {
        MenuItem {
            availability: Availability::Disabled,
            ..self
        }
    }
}

impl<T> MenuEntry<'_, T> {
    /// Whether the keys may light it: an enabled item or a submenu.
    fn is_usable(&self) -> bool {
        match self {
            MenuEntry::Item(item) => item.availability == Availability::Enabled,
            MenuEntry::Submenu { .. } => true,
            MenuEntry::Separator => false,
        }
    }

    /// Its words, none for a separator.
    fn label(&self) -> &str {
        match self {
            MenuEntry::Item(item) => item.label,
            MenuEntry::Submenu { label, .. } => label,
            MenuEntry::Separator => "",
        }
    }
}

/// Where a menu opens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAnchor {
    /// Where the pointer is, for a context menu.
    Pointer,
    /// At a point, in logical pixels of the window.
    At(Point),
    /// Under a rectangle, such as the button that opens it.
    Below(Rect),
}

/// The menu open, which the toolkit remembers between frames. One menu is open at a time.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OpenMenu {
    /// The menu.
    pub(crate) id: WidgetId,
    /// Where it opened.
    pub(crate) placement: Placement,
    /// The item lit in each level open, from the menu itself to the deepest submenu.
    pub(crate) levels: Vec<Option<usize>>,
    /// Where each level was drawn in the last frame.
    pub(crate) rects: Vec<Rect>,
}

/// The entries of the deepest level open: those of the submenus the lit items of the levels above open.
fn entries_of<'menu, 'text, T>(
    root: &'menu [MenuEntry<'text, T>],
    levels: &[Option<usize>],
) -> &'menu [MenuEntry<'text, T>] {
    let parents = levels.len().saturating_sub(1);
    let mut entries = root;
    for lit in levels.iter().take(parents) {
        match lit.and_then(|index| entries.get(index)) {
            Some(MenuEntry::Submenu { entries: inner, .. }) => entries = inner,
            _ => return &[],
        }
    }
    entries
}

impl Ui<'_> {
    /// Opens a menu, closing any other: from the next call of [`Ui::menu`] with the same key, it is drawn.
    pub fn open_menu(&mut self, key: impl Hash, anchor: MenuAnchor) {
        let id = self.id(key);
        let placement = match anchor {
            MenuAnchor::Pointer => Placement::At(self.pointer().unwrap_or_else(|| self.window().center())),
            MenuAnchor::At(point) => Placement::At(point),
            MenuAnchor::Below(rect) => Placement::Below(rect),
        };
        self.memory().menu = Some(OpenMenu {
            id,
            placement,
            levels: vec![None],
            rects: Vec::new(),
        });
    }

    /// Whether a menu is open.
    #[must_use]
    pub fn is_menu_open(&self, key: impl Hash) -> bool {
        let id = self.id(key);
        self.memory_view().menu.as_ref().is_some_and(|menu| menu.id == id)
    }

    /// Declares a menu: draws it while it is open, and returns the value of the item chosen in this frame, which
    /// closes it.
    pub fn menu<T: Copy>(&mut self, key: impl Hash, entries: &[MenuEntry<'_, T>]) -> Option<T> {
        let id = self.id(key);
        if self.memory_view().menu.as_ref().is_none_or(|menu| menu.id != id) {
            return None;
        }
        let mut open = self.memory().menu.take()?;
        let is_new = self.declare_overlay(id, &[]);
        for event in self.take_events(id) {
            let WidgetEvent::Key(key) = event else {
                continue;
            };
            self.claim_key(id, key);
            match keys::apply_key(&mut open.levels, entries, key) {
                KeyAction::Moved => {}
                KeyAction::Chose(value) => return Some(value),
                KeyAction::Closed => return None,
            }
        }
        let chosen = self.draw_menu(id, &mut open, entries);
        let mut inside = open.rects.clone();
        if let Placement::Below(anchor) = open.placement {
            inside.push(anchor);
        }
        if chosen.is_some() || (!is_new && self.pressed_outside_all(&inside)) {
            return chosen;
        }
        if self.memory_view().focus() != Some(id) {
            self.request_focus(id);
        }
        self.memory().menu = Some(open);
        None
    }

    /// Declares the context menu of a widget: opened at the pointer by the button or the key of context menus, and
    /// drawn while it is open. Returns the value of the item chosen.
    pub fn context_menu<T: Copy>(
        &mut self,
        key: impl Hash + Copy,
        response: &Response,
        entries: &[MenuEntry<'_, T>],
    ) -> Option<T> {
        if response.secondary_clicked {
            let at = self
                .pointer()
                .filter(|point| response.rect.contains(*point))
                .unwrap_or_else(|| Point::new(response.rect.x, response.rect.bottom()));
            self.open_menu(key, MenuAnchor::At(at));
        }
        self.menu(key, entries)
    }

    /// Draws a button that opens a menu under it, and closes it when clicked again. Returns the value of the item
    /// chosen.
    pub fn menu_button<T: Copy>(
        &mut self,
        key: impl Hash + Copy,
        rect: Rect,
        button: &Button<'_>,
        entries: &[MenuEntry<'_, T>],
    ) -> Option<T> {
        let response = self.scope(key, |ui| ui.button("button", rect, button));
        if response.clicked {
            if self.is_menu_open(key) {
                self.memory().menu = None;
            } else {
                self.open_menu(key, MenuAnchor::Below(rect));
            }
        }
        self.menu(key, entries)
    }
}
