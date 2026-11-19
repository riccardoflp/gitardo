//! The two versions of an image side by side in the Diff page (`diff.image`, D74): each in its column at one scale for
//! both, so that a picture that grew looks larger, **Empty** where a side has none, what changed over them, and the size,
//! format and weight of each under it.

use ui::layout::{Align, Insets, inset, place};
use ui::{ImageRead, Label, NotRead, Rect, TextKind, Tone, Ui};

use crate::file_view::{ImageShown, caption};
use crate::intent::Intent;
use crate::text::diff as words;
use crate::text::file_view as file_words;

/// How many rows of a page an image takes at least, and at most: its pictures fit the rows between.
pub const IMAGE_ROWS: (usize, usize) = (8, 26);

/// The two versions of an image a file of a page compares, and for an SVG the diff of its lines.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DiffImages {
    /// The version on the left, none when the side has no file.
    pub left: Option<Result<ImageRead, NotRead>>,
    /// The version on the right.
    pub right: Option<Result<ImageRead, NotRead>>,
    /// The diff of the lines of an SVG, shown with **Text**; none for an image that is not text.
    pub text: Option<Box<super::DiffText>>,
    /// Whether the two sides have the same bytes: there is no change.
    pub is_same_bytes: bool,
}

impl DiffImages {
    /// Whether both sides are read and their pixels are the same.
    #[must_use]
    pub fn is_same_pixels(&self) -> bool {
        match (&self.left, &self.right) {
            (Some(Ok(left)), Some(Ok(right))) => {
                left.image == right.image && (left.width, left.height) == (right.width, right.height)
            }
            _ => false,
        }
    }

    /// How many rows of a page the pictures take, from the taller of the two, with a row over them and one under.
    #[must_use]
    pub fn rows(&self, row_height: f32) -> usize {
        let tallest = [&self.left, &self.right]
            .into_iter()
            .filter_map(|side| side.as_ref().and_then(|read| read.as_ref().ok()))
            .map(|read| read.height)
            .max()
            .unwrap_or(0);
        let picture = crate::number::float_of_count(usize::try_from(tallest).unwrap_or(0));
        let rows = (picture / row_height.max(1.0)).ceil();
        // A count of rows, small and whole after the ceil.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small whole count"
        )]
        let rows = rows.clamp(0.0, 1000.0) as usize;
        (rows + 2).clamp(IMAGE_ROWS.0, IMAGE_ROWS.1)
    }
}

/// What the line over two pictures says: their sizes and weights, before and after, or that only the bytes changed.
#[must_use]
pub fn what_changed(images: &DiffImages) -> String {
    if images.is_same_bytes {
        return words::NO_CHANGE.to_owned();
    }
    if images.is_same_pixels() {
        return words::SAME_PIXELS.to_owned();
    }
    let read = |side: &Option<Result<ImageRead, NotRead>>| side.as_ref().and_then(|read| read.as_ref().ok()).cloned();
    match (read(&images.left), read(&images.right)) {
        (Some(left), Some(right)) => words::image_change(
            (
                &file_words::image_size(left.width, left.height),
                &file_words::image_size(right.width, right.height),
            ),
            (
                &crate::text::worktree_setup::bytes(left.weight),
                &crate::text::worktree_setup::bytes(right.weight),
            ),
        ),
        (Some(only), None) | (None, Some(only)) => caption(&only, None),
        (None, None) => String::new(),
    }
}

/// Where the two pictures of an image file are drawn in a page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageBlock<'a> {
    /// The two versions.
    pub images: &'a DiffImages,
    /// The rectangle of the whole block: the line over the pictures, the pictures, and the lines under them.
    pub rect: Rect,
    /// The left column, across.
    pub left: Rect,
    /// The right column, across.
    pub right: Rect,
}

/// The two pictures of an image file, side by side at one scale, over the rectangle of their block; what is outside the
/// clip its caller gave is not seen.
///
/// Feature: diff.image
pub fn image_view(state: &ImageBlock<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let row = state
        .rect
        .height
        .min(theme.sizes.row_height + theme.sizes.spacing.medium);
    let padding = theme.sizes.spacing.large;
    let over = Rect::new(state.rect.x, state.rect.y, state.rect.width, row);
    let _ = ui.label(
        inset(over, Insets::symmetric(padding, 0.0)),
        &Label::new(&what_changed(state.images))
            .kind(TextKind::Detail)
            .tone(Tone::Muted),
    );
    let pictures_top = state.rect.y + row;
    let pictures_height = (state.rect.height - 2.0 * row).max(0.0);
    let sides = [(state.left, &state.images.left), (state.right, &state.images.right)];
    let room = sides.map(|(column, _)| {
        inset(
            Rect::new(column.x, pictures_top, column.width, pictures_height),
            Insets::all(padding),
        )
    });
    let scale = one_scale(state.images, room);
    for ((column, side), area) in sides.into_iter().zip(room) {
        let under = Rect::new(
            column.x + padding,
            pictures_top + pictures_height,
            column.width - 2.0 * padding,
            row,
        );
        match side {
            None => {
                let _ = ui.label(area, &Label::new(words::EMPTY).tone(Tone::Muted));
            }
            Some(Err(not_read)) => crate::file_view::image::not_drawn(ImageShown::NotRead(not_read), ui, area),
            Some(Ok(read)) => {
                let size = (to_float(read.width) * scale, to_float(read.height) * scale);
                let drawn = place(area, size, (Align::Center, Align::Center));
                ui.clip(area, |ui| crate::file_view::image::picture(ui, drawn, &read.image));
                let said = caption(read, None);
                let _ = ui.label(under, &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted));
            }
        }
    }
    Vec::new()
}

/// The one scale both pictures are drawn at: the largest at which both fit their columns, never larger than they are.
fn one_scale(images: &DiffImages, room: [Rect; 2]) -> f32 {
    let mut scale: f32 = 1.0;
    for (side, area) in [&images.left, &images.right].into_iter().zip(room) {
        if let Some(Ok(read)) = side {
            let fit = crate::file_view::fit_zoom((to_float(read.width), to_float(read.height)), area);
            scale = scale.min(fit);
        }
    }
    scale
}

/// A size in pixels, as logical pixels.
fn to_float(value: u32) -> f32 {
    crate::number::float_of_count(usize::try_from(value).unwrap_or(0))
}
