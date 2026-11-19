//! A file stored with Git LFS whose content is not on this computer, in place of its text (`lfs.show-in-pages`):
//! **Stored with Git LFS, not downloaded** in the File page, **Stored with Git LFS** with each side in the Diff page,
//! its sizes and the first digits of its ids, and **Download**; never the lines of its pointer.

use model::contract::{FeatureId, Target};
use model::lfs::{SideOfPage, StoredWithLfs};
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Button, ButtonKind, Label, Rect, Sense, TextKind, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::lfs as words;

/// What the message of a file stored with Git LFS shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredWithLfsState<'a> {
    /// The file and its sides that are pointers.
    pub stored: &'a StoredWithLfs,
    /// Whether **Download** is offered: Git LFS is installed (`lfs.read-status`).
    pub can_download: bool,
}

/// The height the message takes: its heading, a line for each side and **Download**, with room around them.
#[must_use]
pub fn stored_height(ui: &Ui<'_>, state: &StoredWithLfsState<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    let lines = crate::number::float_of_count(state.stored.sides.len() + 2);
    lines * (sizes.control_height + sizes.spacing.small) + 2.0 * sizes.spacing.huge
}

/// The intent of **Download** for a file stored with Git LFS.
#[must_use]
pub fn download_intent(stored: &StoredWithLfs) -> Intent {
    Intent::Feature(FeatureIntent::Run(
        FeatureId("lfs.download"),
        Target::LfsFiles(Box::new(stored.download())),
    ))
}

/// The message over the rectangle its caller clipped it to: the heading, each side with its size, the first digits of
/// its id and whether it is downloaded, and **Download** while Git LFS is installed.
///
/// Feature: lfs.show-in-pages
pub fn stored_with_lfs(state: &StoredWithLfsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = theme.sizes.control_height;
    let mut intents = Vec::new();
    let is_file_page = matches!(state.stored.sides.as_slice(), [side] if side.side == SideOfPage::Only);
    let heading = if is_file_page {
        words::NOT_DOWNLOADED_HEADING
    } else {
        words::STORED
    };
    ui.region(
        ("stored with lfs", state.stored.path.as_str()),
        (rect, AccessRole::Group, heading),
        |ui| {
            let mut area = Area::with_gap(inset(rect, Insets::all(spacing.huge)), spacing.small);
            let _ = ui.label(area.top(line), &Label::new(heading).kind(TextKind::Heading));
            for side in &state.stored.sides {
                let said = words::side(side.size, &side.oid, side.is_downloaded);
                let said = match side.side {
                    SideOfPage::Only => said,
                    SideOfPage::Before => format!("{}: {said}", words::BEFORE),
                    SideOfPage::After => format!("{}: {said}", words::AFTER),
                };
                let _ = ui.label(area.top(line), &Label::new(&said).tone(Tone::Muted));
            }
            if state.can_download {
                let button = Button::secondary(words::DOWNLOAD).kind(ButtonKind::Outline);
                let (width, height) = ui.button_size(&button);
                let row = area.top(line);
                let cell = Rect::new(row.x, row.y, width, height);
                let response = ui.button(("lfs download", state.stored.path.as_str()), cell, &button);
                ui.tooltip(&response, words::DOWNLOAD_TOOLTIP);
                // In the Diff page the rows lie under the sides, which take the pointer to place the caret: the
                // button takes its clicks above them.
                let above = ui.id(("lfs download above", state.stored.path.as_str()));
                let raised = ui.interact_above(above, cell, Sense::CLICK);
                if response.clicked || raised.clicked {
                    intents.push(download_intent(state.stored));
                }
            }
        },
    );
    intents
}
