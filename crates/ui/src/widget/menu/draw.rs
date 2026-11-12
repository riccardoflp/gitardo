//! Drawing the levels of a menu, each a floating card on the layer of popups, and the pointer over their items:
//! the item under the pointer is lit when the pointer moves, the submenu of an item opens when the pointer is over
//! it, and a click chooses.

use render::{Icon, Point, Rect, TextRun};

use super::layout::{self, MenuMetrics, Placement};
use super::{MenuEntry, OpenMenu};
use crate::access::{AccessRole, Description};
use crate::frame::{TextKind, Ui};
use crate::id::WidgetId;
use crate::interaction::Sense;
use crate::layer::Layer;
use crate::layout::{Align, place};
use crate::widget::Availability;

/// What an entry shows, whatever its kind.
struct EntryLook<'text> {
    /// Its words.
    label: &'text str,
    /// The icon before them.
    icon: Option<Icon>,
    /// The text of its shortcut.
    shortcut: Option<&'text str>,
    /// Whether it opens a submenu.
    is_submenu: bool,
    /// Whether it can be used.
    is_enabled: bool,
}

/// What an entry shows, or nothing for a separator.
fn look<'text, T>(entry: &MenuEntry<'text, T>) -> Option<EntryLook<'text>> {
    match entry {
        MenuEntry::Item(item) => Some(EntryLook {
            label: item.label,
            icon: item.icon,
            shortcut: item.shortcut,
            is_submenu: false,
            is_enabled: item.availability == Availability::Enabled,
        }),
        MenuEntry::Submenu { label, icon, .. } => Some(EntryLook {
            label,
            icon: *icon,
            shortcut: None,
            is_submenu: true,
            is_enabled: true,
        }),
        MenuEntry::Separator => None,
    }
}

/// A level being drawn: where it is, how deep, and the sizes of its parts.
#[derive(Clone, Copy)]
struct LevelPlace {
    /// The menu.
    id: WidgetId,
    /// How deep the level is, the menu itself being 0.
    depth: usize,
    /// Where the level is.
    rect: Rect,
    /// The sizes of the parts of a menu.
    metrics: MenuMetrics,
}

impl Ui<'_> {
    /// Draws the levels open of a menu and returns the value of the item clicked, if any.
    pub(super) fn draw_menu<T: Copy>(
        &mut self,
        id: WidgetId,
        open: &mut OpenMenu,
        root: &[MenuEntry<'_, T>],
    ) -> Option<T> {
        let metrics = MenuMetrics::new(self.theme());
        let window = self.window();
        let mut placement = open.placement;
        let mut entries = root;
        let mut rects = Vec::new();
        let mut chosen = None;
        self.layer(Layer::Popup, |ui| {
            let mut depth = 0;
            while depth < open.levels.len() && chosen.is_none() {
                let size = ui.level_size(entries, &metrics);
                let rect = layout::place(size, placement, window, metrics.padding);
                rects.push(rect);
                let level = LevelPlace {
                    id,
                    depth,
                    rect,
                    metrics,
                };
                let items = ui.draw_level(level, entries, open, &mut chosen);
                let lit = open.levels.get(depth).copied().flatten();
                match lit.and_then(|index| entries.get(index).zip(items.get(index))) {
                    Some((MenuEntry::Submenu { entries: inner, .. }, item)) => {
                        placement = Placement::Beside {
                            item: *item,
                            level: rect,
                        };
                        entries = inner;
                    }
                    _ => open.levels.truncate(depth + 1),
                }
                depth += 1;
            }
        });
        open.rects = rects;
        chosen
    }

    /// The size of a level: as wide as its widest entry and as high as its entries.
    fn level_size<T>(&self, entries: &[MenuEntry<'_, T>], metrics: &MenuMetrics) -> (f32, f32) {
        let style = self.text_style(TextKind::Body);
        let has_submenu = entries.iter().any(|entry| matches!(entry, MenuEntry::Submenu { .. }));
        let chevron = if has_submenu {
            self.theme().sizes.icon + metrics.side
        } else {
            0.0
        };
        let widest = entries
            .iter()
            .filter_map(look)
            .map(|look| {
                let shortcut = look
                    .shortcut
                    .map_or(0.0, |text| metrics.shortcut_gap + self.measure(text, style));
                self.measure(look.label, style) + shortcut
            })
            .fold(0.0_f32, f32::max);
        let height: f32 = entries.iter().map(|entry| entry_height(entry, metrics)).sum();
        let border = self.theme().sizes.border;
        let width = 2.0 * (metrics.side + border) + metrics.icon_column + widest + chevron;
        (
            width.max(metrics.least_width),
            height + 2.0 * (metrics.padding + border),
        )
    }

    /// Draws one level and its entries, lighting and opening as the pointer says, and returns where each entry is.
    fn draw_level<T: Copy>(
        &mut self,
        level: LevelPlace,
        entries: &[MenuEntry<'_, T>],
        open: &mut OpenMenu,
        chosen: &mut Option<T>,
    ) -> Vec<Rect> {
        let theme = *self.theme();
        let sense = if level.depth == 0 {
            Sense::CLICK | Sense::FOCUS
        } else {
            Sense::CLICK
        };
        let panel = if level.depth == 0 {
            level.id
        } else {
            level.id.with(("level", level.depth))
        };
        let _ = self.interact(panel, level.rect, sense);
        self.floating_card(level.rect, theme.sizes.radius.large);
        self.access_node(panel, level.rect, &Description::new(AccessRole::Menu, ""));
        self.access_enter(panel);
        let border = theme.sizes.border;
        let mut y = level.rect.y + level.metrics.padding + border;
        let mut rects = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let height = entry_height(entry, &level.metrics);
            let rect = Rect::new(level.rect.x + border, y, level.rect.width - 2.0 * border, height);
            rects.push(rect);
            y += height;
            let Some(look) = look(entry) else {
                let line = place(rect, (rect.width, border), (Align::Start, Align::Center));
                self.fill_rect(line, theme.colors.border);
                continue;
            };
            let item = level.id.with(("item", level.depth, index));
            let sense = if look.is_enabled { Sense::CLICK } else { Sense::HOVER };
            let response = self.interact(item, rect, sense);
            if response.hovered && self.pointer_moved() {
                open.levels.truncate(level.depth + 1);
                if let Some(lit) = open.levels.last_mut() {
                    *lit = look.is_enabled.then_some(index);
                }
                if look.is_submenu {
                    open.levels.push(None);
                }
            }
            if response.clicked {
                match entry {
                    MenuEntry::Item(item) => *chosen = Some(item.value),
                    MenuEntry::Submenu { .. } if open.levels.len() == level.depth + 1 => open.levels.push(None),
                    MenuEntry::Submenu { .. } | MenuEntry::Separator => {}
                }
            }
            let is_lit = open.levels.get(level.depth).copied().flatten() == Some(index);
            self.menu_entry(rect, &look, is_lit, &level.metrics);
            let mut description = Description::new(AccessRole::MenuItem, look.label).disabled(!look.is_enabled);
            if look.is_submenu {
                description = description.expanded(is_lit && open.levels.len() > level.depth + 1);
            }
            self.access_node(item, rect, &description);
        }
        self.access_leave();
        rects
    }

    /// Draws an entry: lit under the pointer or the keys, its icon, its words, its shortcut, and the arrow of a
    /// submenu.
    fn menu_entry(&mut self, rect: Rect, look: &EntryLook<'_>, is_lit: bool, metrics: &MenuMetrics) {
        let theme = *self.theme();
        let colors = theme.colors;
        if is_lit {
            let inset = theme.sizes.spacing.small;
            let inner = Rect::new(
                rect.x + inset,
                rect.y + 1.0,
                rect.width - 2.0 * inset,
                rect.height - 2.0,
            );
            self.fill_rounded_rect(inner, theme.sizes.radius.medium, colors.menu_selection);
        }
        let (color, shortcut_color) = entry_colors(&colors, look.is_enabled, is_lit);
        let side = theme.sizes.icon;
        let mut x = rect.x + metrics.side;
        if let Some(icon) = look.icon {
            self.entry_icon(icon, Rect::new(x, rect.y, side, rect.height), color);
        }
        x += metrics.icon_column;
        let style = self.text_style(TextKind::Body);
        let line = place(
            rect,
            (rect.width, self.line_height(style)),
            (Align::Start, Align::Center),
        );
        let mut right = rect.right() - metrics.side;
        if look.is_submenu {
            right -= side;
            self.entry_icon(Icon::ChevronRight, Rect::new(right, rect.y, side, rect.height), color);
        }
        if let Some(shortcut) = look.shortcut {
            let width = self.measure(shortcut, style);
            right -= width;
            let run = TextRun {
                text: shortcut,
                style,
                color: shortcut_color,
                origin: Point::new(right, line.y),
                max_width: None,
            };
            let _ = self.text(&run);
            right -= metrics.shortcut_gap;
        }
        let run = TextRun {
            text: look.label,
            style,
            color,
            origin: Point::new(x, line.y),
            max_width: Some((right - x).max(0.0)),
        };
        let _ = self.text(&run);
    }

    /// Draws an icon of an entry in the middle of its column, down.
    fn entry_icon(&mut self, icon: Icon, column: Rect, color: render::Color) {
        let side = self.theme().sizes.icon;
        let at = place(column, (side, side), (Align::Start, Align::Center));
        self.icon(icon, Point::new(at.x, at.y), side, color);
    }
}

/// The colors of the words and of the shortcut of an entry, lit or not, enabled or not.
fn entry_colors(colors: &render::Colors, is_enabled: bool, is_lit: bool) -> (render::Color, render::Color) {
    match (is_enabled, is_lit) {
        (false, _) => (colors.disabled_foreground, colors.disabled_foreground),
        (true, true) => (colors.menu_selection_foreground, colors.menu_selection_foreground),
        (true, false) => (colors.menu_foreground, colors.muted_foreground),
    }
}

/// The height of an entry.
fn entry_height<T>(entry: &MenuEntry<'_, T>, metrics: &MenuMetrics) -> f32 {
    match entry {
        MenuEntry::Separator => metrics.separator,
        MenuEntry::Item(_) | MenuEntry::Submenu { .. } => metrics.item,
    }
}
