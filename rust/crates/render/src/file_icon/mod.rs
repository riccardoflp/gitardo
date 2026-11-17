//! The icons of files and folders: those of the icon theme of [Symbols](https://github.com/miguelsolorio/vscode-symbols),
//! kept in `icons/symbols/` with its licence (MIT), each chosen by the name of the file or the folder as the theme
//! says, and drawn by `resvg` in the colors of the window's theme (`file-view.file-icons`). An icon is read and drawn
//! into pixels once for each size and colors it is asked in, and kept.

mod colors;
mod languages;
mod lookup;
mod sources;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use resvg::usvg;

pub use colors::{ColorFamily, FileIconColors};

use crate::canvas::Canvas;
use crate::geometry::{PixelRect, Point};
use crate::number;

/// An icon of Symbols, for a file or a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileIcon {
    /// Whether it is an icon of folders.
    set: Set,
    /// Its place in the icons of its set, which are in the order of their names.
    index: u16,
}

/// The two sets of icons of Symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Set {
    /// The icons of files.
    Files,
    /// The icons of folders.
    Folders,
}

impl FileIcon {
    /// The first icon of files, given when the theme names none, which a test shows never happens.
    const FIRST_FILE: FileIcon = FileIcon {
        set: Set::Files,
        index: 0,
    };

    /// The first icon of folders, given when the theme names none, which a test shows never happens.
    const FIRST_FOLDER: FileIcon = FileIcon {
        set: Set::Folders,
        index: 0,
    };

    /// The icon of files of a name of Symbols, such as `rust`.
    fn of_file_named(name: &str) -> Option<FileIcon> {
        let index = sources::FILES.binary_search_by(|(each, _)| (*each).cmp(name)).ok()?;
        Some(FileIcon {
            set: Set::Files,
            index: u16::try_from(index).ok()?,
        })
    }

    /// The icon of folders of a name of Symbols, such as `folder-src`.
    fn of_folder_named(name: &str) -> Option<FileIcon> {
        let index = sources::FOLDERS.binary_search_by(|(each, _)| (*each).cmp(name)).ok()?;
        Some(FileIcon {
            set: Set::Folders,
            index: u16::try_from(index).ok()?,
        })
    }

    /// Its name in Symbols, the name of its file: `rust`, `folder`.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.entry().0
    }

    /// Whether it is the icon of a folder.
    #[must_use]
    pub fn is_folder(self) -> bool {
        self.set == Set::Folders
    }

    /// Its name and its SVG.
    fn entry(self) -> (&'static str, &'static str) {
        let entries: &[(&str, &str)] = match self.set {
            Set::Files => &sources::FILES,
            Set::Folders => &sources::FOLDERS,
        };
        entries.get(usize::from(self.index)).copied().unwrap_or(("", ""))
    }
}

/// What an icon is asked for: a file or a folder by its name, without the folders above it, or the folder of the
/// repository itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileOrFolder<'name> {
    /// A file, such as `main.rs`.
    File(&'name str),
    /// A folder, such as `src`.
    Folder(&'name str),
    /// The folder of the repository, at the top of a tree of its files.
    Top,
}

/// How an icon of files is drawn: in which colors, and how opaque, out of 255.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FileIconLook {
    /// The color of each family of hues.
    pub colors: FileIconColors,
    /// How opaque it is drawn, out of 255: less in a row that steps back.
    pub opacity: u8,
}

/// An icon at a side in physical pixels and in colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    /// The icon.
    icon: FileIcon,
    /// Its side, in physical pixels.
    side: u32,
    /// Its colors.
    colors: FileIconColors,
}

/// The theme of Symbols, read the first time it is asked, and the icons drawn so far, each for its side and colors.
#[derive(Clone, Debug, Default)]
pub struct FileIcons {
    /// The theme, once read.
    lookup: Option<lookup::Lookup>,
    /// The pixels of each icon drawn: red, green, blue and opacity, row after row, not multiplied by the opacity.
    drawn: HashMap<Key, Vec<u8>>,
}

impl FileIcons {
    /// The icon the theme of Symbols gives a file or a folder by its name.
    pub fn icon_of(&mut self, asked: FileOrFolder<'_>) -> FileIcon {
        self.lookup.get_or_insert_with(lookup::Lookup::bundled).icon_of(asked)
    }

    /// The pixels of an icon at a side in physical pixels and in colors, drawn the first time they are asked for.
    fn pixels(&mut self, key: Key) -> &[u8] {
        self.drawn
            .entry(key)
            .or_insert_with(|| rasterize(key.icon.entry().1, &key.colors, key.side))
    }
}

/// The extensions of a name, from the longest: `d.ts` then `ts` for `index.d.ts`, `gitignore` for `.gitignore`.
fn extensions(name: &str) -> impl Iterator<Item = &str> {
    name.match_indices('.')
        .filter_map(|(at, _)| name.get(at + 1..))
        .filter(|extension| !extension.is_empty())
}

/// An SVG drawn in colors into a square of a side in pixels, its longer side filling it, centered: four bytes for each
/// pixel, not multiplied by the opacity. An SVG that cannot be read draws nothing: they are part of the program, and a
/// test reads every one.
fn rasterize(source: &str, colors: &FileIconColors, side: u32) -> Vec<u8> {
    let length = number::index_of_size(side) * number::index_of_size(side) * 4;
    let mut pixels = vec![0; length];
    let options = usvg::Options {
        // An image an icon names is never read, as for the diagrams of Mermaid.
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        resources_dir: None,
        ..usvg::Options::default()
    };
    let Ok(tree) = usvg::Tree::from_str(&colors::recolored(source, colors), &options) else {
        return pixels;
    };
    let Some(mut pixmap) = tiny_skia::Pixmap::new(side.max(1), side.max(1)) else {
        return pixels;
    };
    let drawing = tree.size();
    let square = number::float_of_size(side);
    let scale = square / drawing.width().max(drawing.height()).max(f32::MIN_POSITIVE);
    let transform = tiny_skia::Transform::from_row(
        scale,
        0.0,
        0.0,
        scale,
        (square - drawing.width() * scale) / 2.0,
        (square - drawing.height() * scale) / 2.0,
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    for (pixel, color) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(pixmap.pixels()) {
        let color = color.demultiply();
        pixel.copy_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    pixels
}

impl Canvas<'_> {
    /// Draws an icon of files in its colors, its top left corner at a point snapped to a whole physical pixel, its
    /// side `size` logical pixels, drawn into pixels once at that side and kept.
    ///
    /// Feature: file-view.file-icons
    pub fn draw_file_icon(&mut self, icon: FileIcon, origin: Point, size: f32, look: &FileIconLook) {
        let corner = self.to_physical_point(origin);
        let (column, row) = (number::round(corner.x), number::round(corner.y));
        let square = number::size(number::ceil(size * self.scale()));
        let pixels = number::coordinate(square);
        let rect = PixelRect::new(column, row, column.saturating_add(pixels), row.saturating_add(pixels));
        let clip = self.physical_clip();
        if square == 0 || rect.intersection(clip).is_empty() || look.opacity == 0 {
            return;
        }
        let colors = self.assets.file_icons.pixels(Key {
            icon,
            side: square,
            colors: look.colors,
        });
        if look.opacity == u8::MAX {
            self.frame.fill_colors(rect, colors, clip);
            return;
        }
        let faded: Vec<u8> = colors
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[red, green, blue, opacity]| {
                let opacity = u16::from(opacity) * u16::from(look.opacity) / 255;
                [red, green, blue, u8::try_from(opacity).unwrap_or(u8::MAX)]
            })
            .collect();
        self.frame.fill_colors(rect, &faded, clip);
    }
}
