//! The images of a Diff page as the store keeps them (`diff.image`): an image file shows its two pictures, and an SVG,
//! with **Text**, the diff of its lines in their place, its pictures kept aside until **Image** shows them again.

use std::collections::BTreeMap;

use views::diff::{DiffImages, FileDiff};

use super::{DiffPage, lines};

/// Whether the SVG files of a page show their pictures or the diff of their lines.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PageImages {
    /// Whether they show the diff of their lines, **Text**, rather than their pictures.
    pub shows_text: bool,
    /// The pictures of those that show the diff of their lines, by the place of the file.
    pub aside: BTreeMap<usize, Box<DiffImages>>,
}

/// What a page shows of the images read for a file: their pictures, or for an SVG while the page shows **Text**, the
/// diff of its lines, the pictures kept aside.
pub(super) fn file_diff(page: &mut DiffPage, file: usize, mut images: Box<DiffImages>) -> FileDiff {
    let _kept = page.images.aside.remove(&file);
    if page.images.shows_text
        && let Some(text) = images.text.take()
    {
        let _replaced = page.images.aside.insert(file, images);
        return FileDiff::Text(text);
    }
    FileDiff::Image(images)
}

/// **Text** or **Image**: each SVG of the page shows the diff of its lines, or its pictures.
pub(super) fn show_text(page: &mut DiffPage, shows_text: bool) {
    page.images.shows_text = shows_text;
    for file in 0..page.diffs.len() {
        let Some(diff) = page.diffs.get_mut(file) else {
            continue;
        };
        if shows_text {
            if let FileDiff::Image(images) = diff
                && images.text.is_some()
            {
                let mut images = std::mem::take(images);
                let text = images.text.take().unwrap_or_default();
                *diff = FileDiff::Text(text);
                let _replaced = page.images.aside.insert(file, images);
            }
        } else if let Some(mut images) = page.images.aside.remove(&file) {
            let Some(diff) = page.diffs.get_mut(file) else {
                continue;
            };
            if let FileDiff::Text(text) = std::mem::take(diff) {
                images.text = Some(text);
            }
            *diff = FileDiff::Image(images);
        }
    }
    page.caret = None;
    lines::rebuild(page);
}

/// Whether the page has an SVG, and whether it shows the diff of its lines.
pub(super) fn svg_text(page: &DiffPage) -> Option<bool> {
    let has_svg = !page.images.aside.is_empty()
        || page
            .diffs
            .iter()
            .any(|diff| matches!(diff, FileDiff::Image(images) if images.text.is_some()));
    has_svg.then_some(page.images.shows_text)
}
