//! An image file in the File page, drawn as a picture in place of its bytes (`file-view.image`, D74): fit to the page,
//! on a ground of squares that shows what is transparent, with its size, format and weight under it; zoomed with
//! `Ctrl` and the wheel or the keys, about the pointer, and moved by dragging and by the wheel.

use ui::layout::{Area, Insets, inset};
use ui::{
    AccessRole, Description, ImageFormat, ImageRead, Key, Label, Modifiers, NotRead, Point, Rect, Sense, TextKind,
    Tone, Ui, WidgetEvent,
};

use super::{FilePageId, FilePageIntent, FileViewIntent};
use crate::intent::Intent;
use crate::text::file_view as words;

/// The zooms `Ctrl+=` and `Ctrl+-` go through, as how many pixels of the page a pixel of the image takes.
pub const ZOOM_STEPS: [f32; 6] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0];

/// The side of a square of the ground behind a picture, in logical pixels.
const SQUARE: f32 = 8.0;

/// Where a picture is drawn in its page: its zoom, and the point of it at the middle of the page.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImagePlace {
    /// How many logical pixels of the page a pixel of the image takes; none for the fit.
    pub zoom: Option<f32>,
    /// The point of the image, in its pixels, at the middle of the page; none for its middle.
    pub center: Option<(f32, f32)>,
    /// While it is dragged: where the pointer was pressed, and the point at the middle then.
    pub held: Option<((f32, f32), (f32, f32))>,
}

// A place is made from whole sizes and the positions of the pointer, never from a division by zero: none is NaN.
impl Eq for ImagePlace {}

/// What the page of an image shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageShown<'a> {
    /// The image is being read.
    Reading,
    /// The image read.
    Drawn(&'a ImageRead),
    /// Why it is not drawn.
    NotRead(&'a NotRead),
}

/// What the picture of a File page is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageState<'a> {
    /// The page.
    pub id: FilePageId,
    /// The name of the file, for the screen reader.
    pub name: &'a str,
    /// What it shows.
    pub shown: ImageShown<'a>,
    /// Where the picture is.
    pub place: ImagePlace,
}

/// Something asked of the picture of a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageIntent {
    /// **Preview** of an SVG: its picture.
    ShowPreview,
    /// **Source** of an SVG: its text in the editor.
    ShowSource,
    /// The picture zoomed, moved, or brought back to the fit.
    Place(ImagePlace),
}

/// What a page says in place of a picture it cannot draw.
#[must_use]
pub fn not_read_words(not_read: &NotRead) -> String {
    match not_read {
        NotRead::TooLarge => words::IMAGE_TOO_LARGE.to_owned(),
        NotRead::SvgError(said) => words::svg_error(said),
        NotRead::Unreadable | NotRead::NotImage => words::IMAGE_UNREADABLE.to_owned(),
    }
}

/// The name of a format, as the line under a picture says it.
#[must_use]
pub fn format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "PNG",
        ImageFormat::Svg => "SVG",
    }
}

/// The line under a picture: its size in pixels, its format, its weight, and the zoom when it is not the fit.
#[must_use]
pub fn caption(read: &ImageRead, zoom: Option<f32>) -> String {
    let mut said = words::image_size(read.width, read.height);
    said.push_str(" \u{b7} ");
    said.push_str(format_name(read.format));
    said.push_str(" \u{b7} ");
    said.push_str(&crate::text::worktree_setup::bytes(read.weight));
    if let Some(zoom) = zoom {
        said.push_str(" \u{b7} ");
        said.push_str(&percent(zoom).to_string());
        said.push('%');
    }
    said
}

/// A zoom in whole percents.
fn percent(zoom: f32) -> u32 {
    let percent = (zoom * 100.0).round().clamp(0.0, 1_000_000.0);
    // Within the range of a u32, and whole, after the clamp and the round.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped and rounded"
    )]
    let whole = percent as u32;
    whole
}

/// The picture of an image file, centered in the page and fit to it until it is zoomed, with the line under it.
///
/// Feature: file-view.image
pub fn image_view(state: &ImageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let ImageShown::Drawn(read) = state.shown else {
        not_drawn(state.shown, ui, inset(rect, Insets::all(theme.sizes.spacing.huge)));
        return Vec::new();
    };
    let mut area = Area::new(rect);
    let caption_rect = area.bottom(theme.sizes.row_height + theme.sizes.spacing.medium);
    let picture_area = inset(area.rest(), Insets::all(theme.sizes.spacing.large));
    let size = (to_float(read.width), to_float(read.height));
    let fit = fit_zoom(size, picture_area);
    let zoom = state.place.zoom.unwrap_or(fit);
    let center = clamped_center(
        state.place.center.unwrap_or((size.0 / 2.0, size.1 / 2.0)),
        size,
        (zoom, picture_area),
    );
    let drawn = Rect::new(
        picture_area.center().x - center.0 * zoom,
        picture_area.center().y - center.1 * zoom,
        size.0 * zoom,
        size.1 * zoom,
    );
    ui.clip(picture_area, |ui| picture(ui, drawn, &read.image));
    let id = ui.id(("image picture", state.id));
    let (_, events) = ui.interact_with_events(id, picture_area, Sense::DRAG | Sense::SCROLL | Sense::FOCUS);
    let label = format!("{}, {}, {}", words::IMAGE, state.name, caption(read, None));
    ui.describe(id, picture_area, &Description::new(AccessRole::Label, &label));
    let shown_zoom = state.place.zoom.map(|_| zoom);
    let said = caption(read, shown_zoom);
    let _ = ui.label(
        inset(caption_rect, Insets::symmetric(theme.sizes.spacing.large, 0.0)),
        &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let view = View {
        area: picture_area,
        zoom,
        center,
        fit,
    };
    let mut place = state.place;
    let mut is_changed = moved(&events, &view, (ui.modifiers(), ui.pointer()), &mut place);
    is_changed |= zoom_keys(ui, &view, &mut place);
    if is_changed && place != state.place {
        return vec![Intent::FileView(FileViewIntent::Page(
            state.id,
            FilePageIntent::Image(ImageIntent::Place(place)),
        ))];
    }
    Vec::new()
}

/// What is said in place of a picture not drawn: that it is read, or why it could not be.
pub(crate) fn not_drawn(shown: ImageShown<'_>, ui: &mut Ui<'_>, rect: Rect) {
    let said = match shown {
        ImageShown::NotRead(not_read) => not_read_words(not_read),
        ImageShown::Reading | ImageShown::Drawn(_) => words::READING.to_owned(),
    };
    let _ = ui.label(rect, &Label::new(&said).tone(Tone::Muted));
}

/// A picture over its ground of squares, scaled into a rectangle; only what is inside the clip is seen.
pub(crate) fn picture(ui: &mut Ui<'_>, rect: Rect, image: &ui::Image) {
    squares(ui, rect.intersection(ui.current_clip()));
    ui.image(rect, image);
}

/// How the picture is drawn in this frame.
#[derive(Clone, Copy, Debug)]
struct View {
    /// Where pictures are drawn.
    area: Rect,
    /// Its zoom.
    zoom: f32,
    /// Its point at the middle of the area.
    center: (f32, f32),
    /// The zoom that fits it in the area.
    fit: f32,
}

/// The zoom at which an image fits a rectangle: as large as it is, smaller to fit, never larger.
#[must_use]
pub fn fit_zoom((width, height): (f32, f32), area: Rect) -> f32 {
    if width <= 0.0 || height <= 0.0 {
        return 1.0;
    }
    (area.width / width)
        .min(area.height / height)
        .clamp(f32::MIN_POSITIVE, 1.0)
}

/// The point at the middle of the area, kept so that a picture smaller than the area stays centered across that side,
/// and one larger does not leave the area empty at an edge.
fn clamped_center(center: (f32, f32), size: (f32, f32), (zoom, area): (f32, Rect)) -> (f32, f32) {
    let along = |point: f32, length: f32, room: f32| {
        let half = room / 2.0 / zoom;
        if length * zoom <= room {
            length / 2.0
        } else {
            point.clamp(half, length - half)
        }
    };
    (
        along(center.0, size.0, area.width),
        along(center.1, size.1, area.height),
    )
}

/// The ground of squares behind a picture, in two greys close to the ground of the page.
fn squares(ui: &mut Ui<'_>, rect: Rect) {
    if rect.is_empty() {
        return;
    }
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.page_background);
    let columns = (rect.width / SQUARE).ceil();
    let rows = (rect.height / SQUARE).ceil();
    let mut row = 0.0;
    while row < rows {
        let mut column = (row % 2.0).abs();
        while column < columns {
            let square = Rect::new(rect.x + column * SQUARE, rect.y + row * SQUARE, SQUARE, SQUARE).intersection(rect);
            ui.fill_rect(square, theme.colors.card_background);
            column += 2.0;
        }
        row += 1.0;
    }
}

/// Zooms with `Ctrl` and the wheel about the pointer, moves the picture with the wheel and by dragging: whether the
/// place changed.
fn moved(
    events: &[WidgetEvent],
    view: &View,
    (modifiers, pointer): (Modifiers, Option<Point>),
    place: &mut ImagePlace,
) -> bool {
    let mut is_changed = false;
    for event in events {
        match event {
            WidgetEvent::Wheel { delta } if modifiers.command() => {
                let zoom = if delta.y < 0.0 {
                    step_in(place.zoom.unwrap_or(view.fit))
                } else {
                    step_out(place.zoom.unwrap_or(view.fit))
                };
                zoom_about(view, place, zoom, pointer);
                is_changed = true;
            }
            WidgetEvent::Wheel { delta } => {
                let center = place.center.unwrap_or(view.center);
                place.center = Some((center.0 + delta.x / view.zoom, center.1 + delta.y / view.zoom));
                is_changed = true;
            }
            WidgetEvent::Pressed { position, .. } => {
                place.held = Some(((position.x, position.y), view.center));
                is_changed = true;
            }
            WidgetEvent::Dragged { position, .. } => {
                if let Some(((pressed_x, pressed_y), (center_x, center_y))) = place.held {
                    place.center = Some((
                        center_x - (position.x - pressed_x) / view.zoom,
                        center_y - (position.y - pressed_y) / view.zoom,
                    ));
                    is_changed = true;
                }
            }
            WidgetEvent::Released { .. } => {
                place.held = None;
                is_changed = true;
            }
            _ => {}
        }
    }
    is_changed
}

/// `Ctrl+=` and `Ctrl+-` zoom in steps about the middle, `Ctrl+0` goes back to the fit (`Cmd` on macOS): whether the
/// place changed.
fn zoom_keys(ui: &Ui<'_>, view: &View, place: &mut ImagePlace) -> bool {
    let mut is_changed = false;
    for key in ui.unclaimed_keys() {
        if !key.modifiers.command() || key.modifiers.contains(Modifiers::ALT) {
            continue;
        }
        let current = place.zoom.unwrap_or(view.fit);
        match key.key {
            Key::Character('=' | '+') => zoom_about(view, place, step_in(current), None),
            Key::Character('-') => zoom_about(view, place, step_out(current), None),
            Key::Character('0') => *place = ImagePlace::default(),
            _ => continue,
        }
        is_changed = true;
    }
    is_changed
}

/// Zooms to a zoom keeping the point of the image under the pointer where it is, or the point at the middle without
/// a pointer.
fn zoom_about(view: &View, place: &mut ImagePlace, zoom: f32, pointer: Option<Point>) {
    let middle = view.area.center();
    let (offset_x, offset_y) = pointer.map_or((0.0, 0.0), |point| (point.x - middle.x, point.y - middle.y));
    let under = (
        view.center.0 + offset_x / view.zoom,
        view.center.1 + offset_y / view.zoom,
    );
    place.zoom = Some(zoom);
    place.center = Some((under.0 - offset_x / zoom, under.1 - offset_y / zoom));
}

/// The next step of zoom above a zoom.
#[must_use]
pub fn step_in(zoom: f32) -> f32 {
    ZOOM_STEPS
        .iter()
        .copied()
        .find(|step| *step > zoom * 1.001)
        .unwrap_or(8.0)
}

/// The next step of zoom below a zoom.
#[must_use]
pub fn step_out(zoom: f32) -> f32 {
    ZOOM_STEPS
        .iter()
        .rev()
        .copied()
        .find(|step| *step < zoom * 0.999)
        .unwrap_or(0.25)
}

/// A size in pixels, as logical pixels.
fn to_float(value: u32) -> f32 {
    // An image has at most 64 million pixels, so a side is far below what an f32 holds exactly.
    #[allow(clippy::cast_precision_loss, reason = "a side of an image is small")]
    let float = value as f32;
    float
}
