//! The comparison of two commits in the inspector, in place of the card of a commit.

use model::compare::{CompareSide, FileList, InPlaceSides, NamedCommit, Side};
use model::state::FileChange;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Icon, Label, MenuEntry, MenuItem, Rect, TextKind, Tone, Ui};

use super::CompareIntent;
use crate::graph::GraphIntent;
use crate::intent::Intent;
use crate::text::compare as words;

/// How far the files of the comparison in the inspector are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InPlaceRead<'a> {
    /// Not yet: **Loading…**.
    Loading,
    /// The files that differ from the first to the second.
    Read(&'a [FileChange]),
    /// Git could not compare the two.
    Failed,
}

/// What the comparison in the inspector shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InPlaceState<'a> {
    /// The two commits, from and to.
    pub sides: &'a InPlaceSides,
    /// Its files, as far as they are read.
    pub read: InPlaceRead<'a>,
    /// The file whose diff is shown beside the inspector, by its path, which it marks (`diff.beside-inspector`).
    pub marked: Option<&'a str>,
}

/// The comparison in the inspector: **Comparison** and the button that closes it, **From** and **To** with the name
/// and the short hash of each, the hash going to that commit in the timeline, **Swap**, **Open as a page**, the count
/// of the files with the button that opens them all in one Diff page, and the files, a click opening one in the Diff
/// page.
///
/// Feature: compare.in-place
pub fn in_place(state: &InPlaceState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = theme.sizes.row_height;
    let control = theme.sizes.control_height;
    let files = match state.read {
        InPlaceRead::Read(files) => files,
        InPlaceRead::Loading | InPlaceRead::Failed => &[],
    };
    let count = f32::from(u16::try_from(files.len()).unwrap_or(u16::MAX));
    let height = 2.0 * spacing.large + 3.0 * control + 4.0 * line + count * line + 4.0 * spacing.medium;
    ui.region(
        "comparison in the inspector",
        (rect, AccessRole::Region, words::PAGE),
        |ui| {
            let _ = ui.scroll_area(
                "comparison in the inspector",
                rect,
                (rect.width, height),
                |ui, content| {
                    let mut column = Area::with_gap(inset(content, Insets::all(spacing.large)), spacing.medium);
                    let mut heading = Area::with_gap(column.top(control), spacing.medium);
                    let close = Button::icon(Icon::Close, words::CLOSE);
                    let (width, button_height) = ui.button_size(&close);
                    let cell = place(
                        heading.right(width),
                        (width, button_height),
                        (Align::Start, Align::Center),
                    );
                    if ui.button("close comparison", cell, &close).clicked {
                        intents.push(Intent::Compare(CompareIntent::CloseInPlace));
                    }
                    let _ = ui.label(heading.rest(), &Label::new(words::PAGE).kind(TextKind::Strong));
                    side_row(words::FROM, &state.sides.from, ui, column.top(line), &mut intents);
                    side_row(words::TO, &state.sides.to, ui, column.top(line), &mut intents);
                    actions(ui, column.top(control), &mut intents);
                    match state.read {
                        InPlaceRead::Loading => {
                            let _ = ui.label(column.top(line), &Label::new(words::LOADING).tone(Tone::Muted));
                        }
                        InPlaceRead::Failed => {
                            let _ = ui.label(column.top(line), &Label::new(words::FAILED).tone(Tone::Muted));
                        }
                        InPlaceRead::Read(files) => {
                            file_rows((files, state.sides, state.marked), ui, &mut column, &mut intents);
                        }
                    }
                },
            );
        },
    );
    intents
}

/// **From** or **To**: the name of the commit when it is not its short hash, and its short hash, which shows that
/// commit in the timeline.
fn side_row(heading: &str, side: &NamedCommit, ui: &mut Ui<'_>, row: Rect, intents: &mut Vec<Intent>) {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(row, spacing.medium);
    let width = ui.measure(heading, ui.text_style(TextKind::Body)).max(40.0);
    let _ = ui.label(parts.left(width), &Label::new(heading).tone(Tone::Muted));
    let short = side.commit.short();
    if side.name != short {
        let width = ui.measure(&side.name, ui.text_style(TextKind::Body));
        let _ = ui.label(parts.left(width), &Label::new(&side.name));
    }
    let hash = Button::secondary(short)
        .kind(ButtonKind::Icon)
        .with_tooltip(words::SHOW_THIS_COMMIT);
    let (width, height) = ui.button_size(&hash);
    let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
    if ui.button(("in place side", heading), cell, &hash).clicked {
        intents.push(Intent::Graph(GraphIntent::GoTo(side.commit.clone())));
    }
}

/// **Swap** and **Open as a page**.
fn actions(ui: &mut Ui<'_>, row: Rect, intents: &mut Vec<Intent>) {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(row, spacing.medium);
    let swap = Button::secondary(words::IN_PLACE_SWAP)
        .kind(ButtonKind::Outline)
        .with_tooltip(words::SWAP_TOOLTIP);
    let page = Button::secondary(words::OPEN_AS_PAGE)
        .kind(ButtonKind::Outline)
        .with_tooltip(words::OPEN_AS_PAGE_TOOLTIP);
    for (key, button, intent) in [
        ("in place swap", swap, CompareIntent::SwapInPlace),
        ("in place page", page, CompareIntent::InPlaceAsPage),
    ] {
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(key, cell, &button).clicked {
            intents.push(Intent::Compare(intent));
        }
    }
}

/// **N changed files**, the button that opens them all when there are more than one, and the files, with **Compare
/// with…** in the context menu of each (`details.compare-file-with`).
fn file_rows(
    (files, sides, marked): (&[FileChange], &InPlaceSides, Option<&str>),
    ui: &mut Ui<'_>,
    column: &mut Area,
    intents: &mut Vec<Intent>,
) {
    let line = ui.theme().sizes.row_height;
    let spacing = ui.theme().sizes.spacing;
    let mut heading = Area::with_gap(column.top(line), spacing.medium);
    if files.len() > 1 {
        let all = Button::icon(Icon::Eye, words::OPEN_ALL);
        let (width, height) = ui.button_size(&all);
        let cell = place(heading.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button("in place all", cell, &all).clicked {
            intents.push(Intent::Compare(CompareIntent::InPlaceAll));
        }
    }
    let count = words::changed_files(files.len());
    let _ = ui.label(heading.rest(), &Label::new(&count).kind(TextKind::Strong));
    for (index, file) in files.iter().enumerate() {
        let row = column.top(line);
        if marked == Some(file.path.as_str()) {
            let radius = ui.theme().sizes.radius.medium;
            ui.fill_rounded_rect(row, radius, ui.theme().colors.list_selection);
        }
        let label = match &file.original_path {
            Some(original) => format!("{original} \u{2192} {}", file.path),
            None => file.path.clone(),
        };
        let open = Button::secondary(&label).kind(ButtonKind::Icon).with_icon(Icon::File);
        let (width, height) = ui.button_size(&open);
        let cell = place(row, (width.min(row.width), height), (Align::Start, Align::Center));
        let response = ui.button(("in place file", index), cell, &open);
        if response.clicked {
            intents.push(Intent::Compare(CompareIntent::InPlaceFile(index)));
        }
        let entries = [MenuEntry::Item(MenuItem::new(crate::text::details::COMPARE_WITH, ()))];
        if ui
            .context_menu(("in place file menu", index), &response, &entries)
            .is_some()
        {
            let list = FileList::Comparison {
                first: fixed(&sides.from),
                second: fixed(&sides.to),
            };
            intents.push(crate::details::compare_file(file, list));
        }
    }
}

/// A side of the comparison in the inspector, at its commit, named as it shows it.
fn fixed(side: &NamedCommit) -> CompareSide {
    CompareSide {
        name: side.name.clone(),
        side: Side::Commit(side.commit.clone()),
        is_followed: false,
        is_gone: false,
    }
}
