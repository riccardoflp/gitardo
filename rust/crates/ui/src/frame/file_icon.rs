//! The icons beside the names of files and folders (`file-view.file-icons`): those of Symbols in the colors of the
//! theme, the same in the color of the text, or the plain icons of a file and a folder, as the setting `fileIcons`
//! chooses. They are decoration: the screen reader hears the name beside them.

use render::{Color, FileIconColors, FileIconLook, FileOrFolder, Icon, Point, Rect};

use super::Ui;
use crate::layout::{Align, place};
use crate::paint::Shape;
use crate::widget::Tone;

/// How the icons of files and folders are drawn, as the setting `fileIcons` says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FileIconStyle {
    /// The icons of Symbols, each family of their hues in its color of the theme.
    #[default]
    Symbols,
    /// The icons of Symbols, all in the color of the text, as the other icons of the window.
    Monochrome,
    /// The plain icons of a file and a folder of Lucide, and none beside the folders of a tree.
    Simple,
}

impl Ui<'_> {
    /// How the icons of files and folders are drawn.
    #[must_use]
    pub fn file_icons(&self) -> FileIconStyle {
        self.settings.file_icons
    }

    /// Draws the icon of a file or a folder, its top left corner at a point and its side `size` logical pixels; in
    /// the color given when it is drawn in one color.
    pub fn file_icon(&mut self, asked: FileOrFolder<'_>, origin: Point, size: f32, color: Color) {
        let colors = match self.settings.file_icons {
            FileIconStyle::Symbols => FileIconColors::of_theme(&self.settings.theme),
            FileIconStyle::Monochrome => FileIconColors::single(color),
            FileIconStyle::Simple => {
                let icon = match asked {
                    FileOrFolder::File(_) => Icon::File,
                    FileOrFolder::Folder(_) | FileOrFolder::Top => Icon::Folder,
                };
                self.icon(icon, origin, size, color);
                return;
            }
        };
        let icon = self.assets.file_icons.icon_of(asked);
        let look = FileIconLook {
            colors,
            opacity: u8::MAX,
        };
        let shape = Shape::FileIcon {
            icon,
            origin,
            size,
            look,
        };
        self.record(shape, Rect::new(origin.x, origin.y, size, size));
    }

    /// Draws the icon of a file or a folder of the theme's size in the middle of a rectangle, in the color of a tone
    /// when it is drawn in one color.
    pub fn file_icon_in(&mut self, rect: Rect, asked: FileOrFolder<'_>, tone: Tone) {
        let side = self.theme().sizes.icon;
        let placed = place(rect, (side, side), (Align::Center, Align::Center));
        let color = tone.color(self.theme());
        self.file_icon(asked, Point::new(placed.x, placed.y), side, color);
    }
}
