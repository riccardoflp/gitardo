//! The images of a commit in its page (`commit-page.images`, D74): the version before and the version after of an
//! image file side by side under the header of its diff, in place of **Binary file.**, read only once the diff is shown
//! unfolded; and for an SVG, its pictures in place of its lines with **Show the images**.

use std::collections::{BTreeMap, BTreeSet};

use model::commit_page::FilePatch;
use ui::layout::{Insets, inset};
use ui::{AccessRole, Description, Rect, Sense, Ui};

use super::CommitPageId;
use crate::diff::{DiffImages, ImageBlock};
use crate::file_view::ImageShown;
use crate::intent::Intent;
use crate::text::details as words;

/// The highest a picture is drawn in the page.
pub const PICTURE_HEIGHT: f32 = 240.0;

/// The endings of the names of the files the page reads as images: PNG, JPEG, GIF, WebP, BMP, ICO and SVG (Q1301).
const IMAGE_ENDINGS: [&str; 8] = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".ico", ".svg"];

/// The two versions of an image file of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageImage {
    /// They are read.
    Reading,
    /// They were read; none when neither side is an image.
    Read(Option<Box<DiffImages>>),
}

/// The images of the files of a page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PageImages {
    /// The two versions of each file read, or being read, by the place of the file.
    pub files: BTreeMap<usize, PageImage>,
    /// The SVG files that show their pictures rather than their lines, by their place.
    pub svg_shown: BTreeSet<usize>,
}

impl PageImages {
    /// Whether the diff of a file shows its two pictures, read or being read, in place of its lines or its note: an
    /// SVG once **Show the images** is clicked, a binary file whose name is of an image unless it was read and is not
    /// one.
    #[must_use]
    pub fn shows(&self, file: usize, diff: &FilePatch, path: &str) -> bool {
        if is_svg_name(path) {
            return self.svg_shown.contains(&file);
        }
        diff.is_binary && is_image_name(path) && !matches!(self.files.get(&file), Some(PageImage::Read(None)))
    }
}

/// Whether the name of a file ends as an image of a format the program reads, in any case.
#[must_use]
pub fn is_image_name(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    IMAGE_ENDINGS.iter().any(|ending| lower.ends_with(ending))
}

/// Whether the name of a file ends in `.svg`, in any case.
#[must_use]
pub fn is_svg_name(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".svg")
}

/// How high the block of the two pictures of a file is: the line over them, the pictures at most 240 pixels high with
/// their margins, and the line under them. It is the same while they are read, so that the page does not jump.
pub(crate) fn block_height(ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    let line = theme.sizes.row_height + theme.sizes.spacing.medium;
    PICTURE_HEIGHT + 2.0 * line + 2.0 * theme.sizes.spacing.large
}

/// The two pictures of an image file of a page, as the page draws them.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagesState<'a> {
    /// The page.
    pub page: CommitPageId,
    /// The file, by its place.
    pub file: usize,
    /// Its path, which the screen reader says.
    pub path: &'a str,
    /// Its two versions; none before they are asked for.
    pub image: Option<&'a PageImage>,
    /// The rectangle of the block.
    pub rect: Rect,
    /// What opens the file in the Diff page, at that commit.
    pub open: Intent,
}

/// The two versions of an image file under the header of its diff, side by side, each at most 240 pixels high and half
/// the width of the page, never larger than it is, **Empty** where a side has none, its size and weight under each; a
/// click opens the file in the Diff page.
///
/// Feature: commit-page.images
pub fn images_view(state: &ImagesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let padding = ui.theme().sizes.spacing.large;
    let rect = inset(state.rect, Insets::symmetric(padding, 0.0));
    let images = match state.image {
        Some(PageImage::Read(Some(images))) => images,
        Some(PageImage::Read(None) | PageImage::Reading) | None => {
            crate::file_view::image::not_drawn(ImageShown::Reading, ui, inset(rect, Insets::all(padding)));
            return Vec::new();
        }
    };
    let id = ui.id(("commit page images", state.page, state.file));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::HOVER);
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::Button, &words::open_images(state.path)),
    );
    let half = rect.width / 2.0;
    let block = ImageBlock {
        images,
        rect,
        left: Rect::new(rect.x, rect.y, half, rect.height),
        right: Rect::new(rect.x + half, rect.y, half, rect.height),
    };
    let _ = crate::diff::image_view(&block, ui);
    if response.clicked {
        return vec![state.open.clone()];
    }
    Vec::new()
}
