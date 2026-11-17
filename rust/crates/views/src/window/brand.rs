//! The logo and the name of Gitardo (`window.brand`, D58): the logo in the square above the bar of views, the name at
//! the start of the header bar, before where one is. Neither is a button: both are part of the caption of the title
//! bar (`window.title-bar`).

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Description, Label, Rect, TextKind, Tone, Ui};

use crate::icons;
use crate::intent::Intent;
use crate::line::{control_in, is_narrow};
use crate::text::header as words;
use crate::view_bar::VIEW_BAR_WIDTH;

use super::frame::{CaptionState, TitleBar};

/// What the top left of the window shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrandState {
    /// Whether the name is drawn after the logo: not in a narrow window, whose header bar needs the room.
    pub is_name_shown: bool,
}

/// The width the name takes at the start of the header bar, with the room around it; nothing when it is not shown.
pub(crate) fn name_width(state: BrandState, ui: &Ui<'_>) -> f32 {
    if !state.is_name_shown {
        return 0.0;
    }
    let spacing = ui.theme().sizes.spacing;
    spacing.small + ui.measure(words::NAME, ui.text_style(TextKind::Strong)) + spacing.medium
}

/// The logo and the name over the rectangle their caller clipped them to, whose left part is the square above the bar
/// of views: the logo in its middle, in the accent of the theme, at the size of the icons of the bar of views; then,
/// when it is shown, the name **Gitardo** in the weight of a title. Nothing is asked when either is clicked.
///
/// Feature: window.brand
#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "a view takes its state by reference, as rule R2.4 says"
)]
pub fn brand_view(state: &BrandState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let square = ui::Rect::new(rect.x, rect.y, VIEW_BAR_WIDTH.min(rect.width), rect.height);
    ui.icon_in(square, icons::LOGO, Tone::Accent);
    let side = theme.sizes.icon;
    let id = ui.id("logo");
    let logo = place(square, (side, side), (Align::Center, Align::Center));
    ui.describe(id, logo, &Description::new(AccessRole::Label, words::LOGO));
    if state.is_name_shown {
        let room = inset(
            rect,
            Insets {
                left: square.width + theme.sizes.spacing.small,
                ..Insets::all(0.0)
            },
        );
        let _ = ui.label(
            control_in(ui, room, room.width),
            &Label::new(words::NAME).kind(TextKind::Strong),
        );
    }
    Vec::new()
}

/// Draws the logo in the square above the bar of views and the name at the start of the header bar, or both at the
/// start of the header bar on macOS, whose buttons take that square; gives what is left of the header bar for the
/// header (`window.brand`).
pub(super) fn with_brand(
    caption: CaptionState,
    (corner, header): (Rect, Rect),
    ui: &mut Ui<'_>,
    intents: &mut Vec<Intent>,
) -> Rect {
    let brand = BrandState {
        is_name_shown: !is_narrow(ui),
    };
    let name_width = name_width(brand, ui);
    let mut header = Area::new(header);
    let top_left = if caption.title_bar == TitleBar::Transparent {
        header.left(VIEW_BAR_WIDTH + name_width)
    } else {
        let name = header.left(name_width);
        Rect::new(corner.x, corner.y, corner.width + name.width, corner.height)
    };
    intents.extend(ui.clip(top_left, |ui| brand_view(&brand, ui)));
    header.rest()
}
