//! Drawing the levels of a menu, each a floating card on the layer of popups, and the pointer over their items:
//! the item under the pointer is lit when the pointer moves, the submenu of an item opens when the pointer is over
//! it, and a click chooses. A level taller than its room scrolls, with a bar on its right and its edges faded where
//! there is more (`ui.menu-scroll`).

use render::{Icon, Point, Rect, TextRun};

use super::layout::{self, MenuMetrics, Placement};
use super::{MenuEntry, OpenMenu};
use crate::access::{AccessRole, Description};
use crate::frame::{TextKind, Ui};
use crate::id::WidgetId;
use crate::interaction::{Sense, WidgetEvent};
use crate::layer::Layer;
use crate::layout::{Align, place};
use crate::widget::Availability;
use crate::widget::panel::Axis;
use crate::widget::scroll::ScrollAxis;
use crate::widget::scroll::bar::BarSpec;

/// What an entry shows, whatever its kind.
struct EntryLook<'text> {
    /// Its words.
    label: &'text str,
    /// The icon before them.
    icon: Option<Icon>,
    /// The text of its shortcut.
    shortcut: Option<&'text str>,
    /// What it says under the pointer.
    tooltip: Option<&'text str>,
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
            tooltip: item.tooltip,
            is_submenu: false,
            is_enabled: item.availability == Availability::Enabled,
        }),
        MenuEntry::Submenu { label, icon, .. } => Some(EntryLook {
            label,
            icon: *icon,
            shortcut: None,
            tooltip: None,
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
    /// The height of all of its entries with their padding, taller than `rect` when it scrolls.
    content: f32,
    /// The sizes of the parts of a menu.
    metrics: MenuMetrics,
}

impl LevelPlace {
    /// The part of the level its entries are seen in: inside its border.
    fn inner(self, border: f32) -> Rect {
        Rect::new(
            self.rect.x,
            self.rect.y + border,
            self.rect.width,
            (self.rect.height - 2.0 * border).max(0.0),
        )
    }

    /// Whether the level is shorter than its entries, and scrolls.
    fn scrolls(self) -> bool {
        layout::MenuPlace {
            rect: self.rect,
            content: self.content,
        }
        .scrolls()
    }

    /// Where an entry is in the content of the level, from its top: its top and its bottom.
    fn span_of<T>(self, entries: &[MenuEntry<'_, T>], index: usize, border: f32) -> (f32, f32) {
        let before: f32 = entries
            .iter()
            .take(index)
            .map(|entry| entry_height(entry, &self.metrics))
            .sum();
        let height = entries
            .get(index)
            .map_or(0.0, |entry| entry_height(entry, &self.metrics));
        let top = border + self.metrics.padding + before;
        (top, top + height)
    }
}

impl Ui<'_> {
    /// Draws the levels open of a menu and returns the value of the item clicked, if any. `wheel` is how far the wheel
    /// turned over the menu itself, in logical pixels.
    pub(super) fn draw_menu<T: Copy>(
        &mut self,
        id: WidgetId,
        open: &mut OpenMenu,
        root: &[MenuEntry<'_, T>],
        wheel: f32,
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
                let placed = layout::place(size, placement, window, metrics.padding);
                rects.push(placed.rect);
                let level = LevelPlace {
                    id,
                    depth,
                    rect: placed.rect,
                    content: placed.content,
                    metrics,
                };
                let turned = if depth == 0 { wheel } else { 0.0 };
                let items = ui.draw_level(level, entries, open, (&mut chosen, turned));
                let lit = open.levels.get(depth).copied().flatten();
                match lit.and_then(|index| entries.get(index).zip(items.get(index))) {
                    Some((MenuEntry::Submenu { entries: inner, .. }, item)) => {
                        placement = Placement::Beside {
                            item: *item,
                            level: placed.rect,
                        };
                        entries = inner;
                    }
                    _ => open.levels.truncate(depth + 1),
                }
                depth += 1;
            }
        });
        open.rects = rects;
        open.reveal = false;
        open.scrolls.truncate(open.levels.len());
        open.shown.truncate(open.levels.len());
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

    /// Draws one level and its entries, scrolled as far as it is, lighting and opening as the pointer says, and
    /// returns where each entry is. `turned` is how far the wheel turned over the level, from the events its menu
    /// took.
    fn draw_level<T: Copy>(
        &mut self,
        level: LevelPlace,
        entries: &[MenuEntry<'_, T>],
        open: &mut OpenMenu,
        (chosen, turned): (&mut Option<T>, f32),
    ) -> Vec<Rect> {
        let theme = *self.theme();
        let (panel, sense) = if level.depth == 0 {
            (level.id, Sense::CLICK | Sense::FOCUS | Sense::SCROLL)
        } else {
            (level.id.with(("level", level.depth)), Sense::CLICK | Sense::SCROLL)
        };
        let (_, events) = self.interact_with_events(panel, level.rect, sense);
        let wheel: f32 = events
            .iter()
            .map(|event| match event {
                WidgetEvent::Wheel { delta } => delta.y,
                _ => 0.0,
            })
            .sum();
        self.floating_card(level.rect, theme.sizes.radius.large);
        self.access_node(panel, level.rect, &Description::new(AccessRole::Menu, ""));
        self.access_enter(panel);
        let offset = self.level_offset(level, entries, open, turned + wheel);
        let inner = level.inner(theme.sizes.border);
        let rects = self.clip(inner, |ui| ui.draw_entries(level, entries, open, (chosen, offset)));
        self.access_leave();
        if level.scrolls() {
            self.level_edges(level, offset);
            self.level_bar(panel, level, open);
        }
        rects
    }

    /// How far a level is scrolled in this frame, in logical pixels: the scroll it had, moved by the wheel, by entries,
    /// and by the keys to show the item they lit whole, inside its content.
    fn level_offset<T>(
        &mut self,
        level: LevelPlace,
        entries: &[MenuEntry<'_, T>],
        open: &mut OpenMenu,
        wheel: f32,
    ) -> f32 {
        let border = self.theme().sizes.border;
        let parent = level
            .depth
            .checked_sub(1)
            .and_then(|above| open.levels.get(above).copied().flatten());
        while open.scrolls.len() <= level.depth {
            open.scrolls.push((parent, ScrollAxis::default()));
        }
        let is_deepest = level.depth + 1 == open.levels.len();
        let lit = open.levels.get(level.depth).copied().flatten();
        let reveal = lit.filter(|_| open.reveal && is_deepest);
        let Some((opened_by, scroll)) = open.scrolls.get_mut(level.depth) else {
            return 0.0;
        };
        if *opened_by != parent {
            *opened_by = parent;
            *scroll = ScrollAxis::default();
        }
        if wheel != 0.0 {
            let item = level.metrics.item;
            let steps = (wheel / item).round();
            let steps = if steps == 0.0 { wheel.signum() } else { steps };
            scroll.jump(scroll.target() + f64::from(steps * item));
        }
        if let Some(index) = reveal {
            let (top, bottom) = level.span_of(entries, index, border);
            let view = f64::from(level.rect.height);
            scroll.reveal((f64::from(top - border), f64::from(bottom + border)), view);
        }
        scroll.clamp(f64::from(level.content - level.rect.height));
        let now = self.now();
        let is_gliding = scroll.advance(now);
        let offset = crate::number::narrow(scroll.offset());
        if is_gliding {
            self.request_frame_at(now + crate::widget::scroll::GLIDE_FRAME);
        }
        offset
    }

    /// Draws the entries of a level from the offset it is scrolled to, and returns where each is: those out of sight
    /// are neither drawn nor declared.
    fn draw_entries<T: Copy>(
        &mut self,
        level: LevelPlace,
        entries: &[MenuEntry<'_, T>],
        open: &mut OpenMenu,
        (chosen, offset): (&mut Option<T>, f32),
    ) -> Vec<Rect> {
        let theme = *self.theme();
        let border = theme.sizes.border;
        let inner = level.inner(border);
        let mut y = level.rect.y + level.metrics.padding + border - offset;
        let mut rects = Vec::with_capacity(entries.len());
        let mut whole = 0;
        for (index, entry) in entries.iter().enumerate() {
            let height = entry_height(entry, &level.metrics);
            let rect = Rect::new(level.rect.x + border, y, level.rect.width - 2.0 * border, height);
            rects.push(rect);
            y += height;
            if rect.bottom() <= inner.y || rect.y >= inner.bottom() {
                continue;
            }
            if rect.y >= inner.y - 0.5 && rect.bottom() <= inner.bottom() + 0.5 {
                whole += 1;
            }
            if let Some(look) = look(entry) {
                self.menu_item(level, (index, entry, rect), &look, (open, chosen));
            } else {
                let line = place(rect, (rect.width, border), (Align::Start, Align::Center));
                self.fill_rect(line, theme.colors.border);
            }
        }
        while open.shown.len() <= level.depth {
            open.shown.push(0);
        }
        if let Some(shown) = open.shown.get_mut(level.depth) {
            *shown = whole;
        }
        rects
    }

    /// Declares and draws an item of a level: lit and opened under the pointer, chosen by a click.
    fn menu_item<T: Copy>(
        &mut self,
        level: LevelPlace,
        (index, entry, rect): (usize, &MenuEntry<'_, T>, Rect),
        look: &EntryLook<'_>,
        (open, chosen): (&mut OpenMenu, &mut Option<T>),
    ) {
        let item = level.id.with(("item", level.depth, index));
        let sense = if look.is_enabled { Sense::CLICK } else { Sense::HOVER };
        let response = self.interact(item, rect, sense);
        if let Some(tooltip) = look.tooltip {
            self.tooltip(&response, tooltip);
        }
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
        self.menu_entry(rect, look, is_lit, &level.metrics);
        let mut description = Description::new(AccessRole::MenuItem, look.label).disabled(!look.is_enabled);
        if look.is_submenu {
            description = description.expanded(is_lit && open.levels.len() > level.depth + 1);
        }
        self.access_node(item, rect, &description);
    }

    /// Fades the first and the last entries shown of a level that scrolls into its border, where there is more above
    /// or below them.
    fn level_edges(&mut self, level: LevelPlace, offset: f32) {
        let theme = *self.theme();
        let inner = level.inner(theme.sizes.border);
        let largest = level.content - level.rect.height;
        let steps: u8 = 6;
        let step = level.metrics.item * 0.75 / f32::from(steps);
        for band in 0..steps {
            let color = theme.colors.menu_background.with_alpha(220 - band * 36);
            let from_edge = f32::from(band) * step;
            if offset > 0.5 {
                self.fill_rect(Rect::new(inner.x, inner.y + from_edge, inner.width, step), color);
            }
            if offset < largest - 0.5 {
                let y = inner.bottom() - from_edge - step;
                self.fill_rect(Rect::new(inner.x, y, inner.width, step), color);
            }
        }
    }

    /// The bar of a level that scrolls, inside its border on its right, which can be dragged.
    fn level_bar(&mut self, panel: WidgetId, level: LevelPlace, open: &mut OpenMenu) {
        let theme = *self.theme();
        let inner = level.inner(theme.sizes.border);
        let thickness = theme.sizes.scrollbar;
        let radius = theme.sizes.radius.large;
        let track = Rect::new(
            inner.right() - theme.sizes.border - thickness,
            inner.y + radius * 0.5,
            thickness,
            (inner.height - radius).max(0.0),
        );
        let spec = BarSpec {
            axis: Axis::Vertical,
            view: f64::from(level.rect.height),
            content: f64::from(level.content),
        };
        if let Some((_, scroll)) = open.scrolls.get_mut(level.depth) {
            let mut axis = *scroll;
            self.scroll_bar(panel.with("bar"), track, spec, &mut axis);
            *scroll = axis;
        }
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
