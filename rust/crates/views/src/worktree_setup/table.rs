//! The head of the page and the table of what git ignores: the sections, each line with its size and the three
//! buttons that choose what it gets, and the places of a name in several places.

use model::worktree_setup::{
    ChooseInput, ChosenLine, Gets, IgnoredEntry, IgnoredKind, Section, SettingSource, choose, use_proposed,
};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Label, Rect, Sense, TextKind, Tone, Ui};

use super::lines::{SetupLine, bytes_of, is_folder, proposed};
use super::rows::{Drawn, Row, SECTION_CAP};
use super::{SetupBox, SetupIntent, SetupRead};
use crate::icons;
use crate::intent::Intent;
use crate::text::worktree_setup as words;

/// The width of the column of the sizes.
const SIZE_WIDTH: f32 = 200.0;
/// What a copy takes before the page says it is large to copy into each worktree: 100 MB.
const LARGE_COPY: u64 = 100 * 1024 * 1024;

/// An intent of the page.
fn page(intent: SetupIntent) -> Intent {
    Intent::WorktreeSetup(intent)
}

/// A row of the head of the page: the header with what applies now, the bar of what git ignores, the note under it, and
/// the titles of the columns.
pub(super) fn head_row(drawn: &Drawn<'_, '_>, row: Row, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let state = drawn.state;
    match row {
        Row::Header => {
            let _ = ui.label(rect, &Label::new(words::HEADER).kind(TextKind::Heading));
        }
        Row::HeaderDetail => {
            let _ = ui.label(rect, &Label::new(words::HEADER_DETAIL).tone(Tone::Muted));
        }
        Row::Now(which) => {
            let (what, setting, vs_code) = match which {
                SetupBox::Link => (
                    words::LINKED_NOW,
                    &state.in_effect.link_folders,
                    "git.worktreeSymlinkFolders",
                ),
                SetupBox::Copy => (
                    words::COPIED_NOW,
                    &state.in_effect.copy_files,
                    "git.worktreeIncludeFiles",
                ),
                SetupBox::Command => (words::RUN_NOW, &state.in_effect.setup_commands, ""),
            };
            let (said, tone) = if setting.source == SettingSource::Nothing {
                (format!("{what}: {}", words::GET_NOTHING), Tone::Muted)
            } else {
                let from = words::source(setting.source, state.repository, vs_code);
                (words::now(what, &setting.values, &from), Tone::Normal)
            };
            let _ = ui.label(rect, &Label::new(&said).tone(tone));
        }
        Row::IgnoredBar => return ignored_bar(drawn, ui, rect),
        Row::Note => note(drawn, ui, rect),
        Row::Columns => {
            let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.large);
            let gets_width = gets_width(ui);
            let _ = ui.label(
                parts.right(gets_width),
                &Label::new(words::GETS).kind(TextKind::SmallStrong),
            );
            let _ = ui.label(
                parts.right(SIZE_WIDTH),
                &Label::new(words::SIZE).kind(TextKind::SmallStrong),
            );
            let _ = ui.label(parts.rest(), &Label::new(words::PATH).kind(TextKind::SmallStrong));
        }
        _ => {}
    }
    Vec::new()
}

/// **What git ignores here** with how many, **Use what is proposed** and **Look again** (**Looking…** while it reads).
fn ignored_bar(drawn: &Drawn<'_, '_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let state = drawn.state;
    let mut intents = Vec::new();
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let again = if state.is_looking {
        Button::secondary(words::LOOKING).disabled()
    } else {
        Button::secondary(words::LOOK_AGAIN)
    }
    .with_icon(icons::READ_AGAIN);
    let (width, height) = ui.button_size(&again);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    if ui.button("look again", cell, &again).clicked {
        intents.push(page(SetupIntent::LookAgain));
    }
    let mut proposed_button = Button::secondary(words::USE_PROPOSED).with_icon(icons::DRAFTED);
    if drawn.lines.is_empty() {
        proposed_button = proposed_button.disabled();
    }
    let (width, height) = ui.button_size(&proposed_button);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    if ui.button("use what is proposed", cell, &proposed_button).clicked {
        let paths: Vec<Vec<String>> = drawn.lines.iter().map(SetupLine::paths).collect();
        let patterns = use_proposed(state.chosen.patterns, &proposed(drawn.lines, &paths));
        intents.push(page(SetupIntent::SetPatterns(patterns)));
    }
    let count = match state.read {
        SetupRead::Read(entries) => entries.len(),
        SetupRead::Looking | SetupRead::Failed(_) => 0,
    };
    let title = words::ignored_here(count);
    let _ = ui.label(parts.rest(), &Label::new(&title).kind(TextKind::Strong));
    intents
}

/// What the page says under the bar: while it first looks, when nothing is ignored, when the patterns changed since
/// git was asked, or what git could not say.
fn note(drawn: &Drawn<'_, '_>, ui: &mut Ui<'_>, rect: Rect) {
    let state = drawn.state;
    let (said, tone) = match state.read {
        SetupRead::Looking => (words::LOOKING_FIRST.to_owned(), Tone::Muted),
        SetupRead::Failed(error) => (format!("{} {error}", words::COULD_NOT_LIST), Tone::Error),
        SetupRead::Read([]) => (words::NOTHING_IGNORED.to_owned(), Tone::Muted),
        SetupRead::Read(_) => match state.look_failed {
            Some(error) => (format!("{} {error}", words::COULD_NOT_SAY), Tone::Error),
            None => (words::CHANGED_SINCE.to_owned(), Tone::Warning),
        },
    };
    let _ = ui.label(rect, &Label::new(&said).kind(TextKind::Detail).tone(tone));
}

/// The width of the three buttons of a line.
fn gets_width(ui: &Ui<'_>) -> f32 {
    let spacing = ui.theme().sizes.spacing.small;
    [words::LINK, words::COPY, words::NOTHING]
        .iter()
        .map(|label| ui.button_size(&Button::secondary(label)).0 + spacing)
        .sum()
}

/// The line of a section: its arrow, title, how many, why, size and what its lines get in short; a click opens or
/// closes it.
pub(super) fn section_row(drawn: &Drawn<'_, '_>, section: Section, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let state = drawn.state;
    let theme = *ui.theme();
    let of_section: Vec<&SetupLine<'_>> = drawn.lines.iter().filter(|line| line.section == section).collect();
    let gets: Vec<Option<Gets>> = of_section.iter().map(|line| state.chosen.gets(line)).collect();
    let has_something = of_section.iter().any(|line| {
        line.places
            .iter()
            .any(|place| state.chosen.place_gets(place) != Gets::Nothing)
    });
    let is_open = super::lines::is_open(section, state.toggled, has_something);
    let response = ui.interact(ui.id(("section", section)), rect, Sense::CLICK | Sense::HOVER);
    if response.hovered {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.large);
    let arrow = if is_open { icons::UNFOLDED } else { icons::FOLDED };
    ui.icon_in(parts.left(theme.sizes.icon), arrow, Tone::Muted);
    let summary = summary(&gets);
    let _ = ui.label(parts.right(gets_width(ui)), &Label::new(&summary).tone(Tone::Muted));
    let bytes: u64 = of_section.iter().map(|line| line.bytes()).sum();
    let size = words::bytes(bytes);
    let _ = ui.label(parts.right(SIZE_WIDTH), &Label::new(&size).tone(Tone::Muted));
    let title = format!(
        "{} \u{b7} {} \u{b7} {}",
        words::section_title(section),
        of_section.len(),
        words::section_why(section)
    );
    let _ = ui.label(parts.rest(), &Label::new(&title).kind(TextKind::Strong));
    if response.clicked {
        return vec![page(SetupIntent::ToggleSection(section, !is_open))];
    }
    Vec::new()
}

/// What the lines of a section get, in short: **all linked**, **all copied**, **nothing**, or how many of each.
fn summary(gets: &[Option<Gets>]) -> String {
    let linked = gets.iter().filter(|gets| **gets == Some(Gets::Link)).count();
    let copied = gets.iter().filter(|gets| **gets == Some(Gets::Copy)).count();
    match (linked, copied) {
        (0, 0) => words::GET_NOTHING.to_owned(),
        (all, 0) if all == gets.len() => words::ALL_LINKED.to_owned(),
        (0, all) if all == gets.len() => words::ALL_COPIED.to_owned(),
        (0, copied) => words::copied(copied),
        (linked, 0) => words::linked(linked),
        (linked, copied) => format!("{}, {}", words::linked(linked), words::copied(copied)),
    }
}

/// A line of the table: the fold of a name in several places, its icon, path and notes, its size, and its buttons.
pub(super) fn line_row(drawn: &Drawn<'_, '_>, index: usize, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let Some(line) = drawn.lines.get(index) else {
        return Vec::new();
    };
    let state = drawn.state;
    let theme = *ui.theme();
    let mut intents = Vec::new();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.large);
    let gets = state.chosen.gets(line);
    let paths = line.paths();
    let buttons = Buttons {
        key: ("line", index, 0),
        gets,
        proposed: line.proposed,
        is_folder: line.is_folder,
    };
    if let Some(chosen) = buttons.draw(ui, parts.right(gets_width(ui))) {
        let patterns = choose(&ChooseInput {
            patterns: state.chosen.patterns,
            line: line.chosen(&paths),
            gets: chosen,
        });
        intents.push(page(SetupIntent::SetPatterns(patterns)));
    }
    let size = match line.places.as_slice() {
        [one] => size_of(one),
        _ => words::bytes(line.bytes()),
    };
    let _ = ui.label(parts.right(SIZE_WIDTH), &Label::new(&size).tone(Tone::Muted));
    let mut path = Area::with_gap(parts.rest(), theme.sizes.spacing.medium);
    let start = path.left(theme.sizes.spacing.large);
    if line.is_name() {
        let is_unfolded = state.unfolded.contains(&line.pattern);
        let arrow = if is_unfolded { icons::UNFOLDED } else { icons::FOLDED };
        let fold = Button::icon(arrow, &line.pattern).kind(ButtonKind::Icon);
        if ui.button(("fold name", index), start, &fold).clicked {
            intents.push(page(SetupIntent::Unfold(line.pattern.clone())));
        }
    }
    let last = line.path().trim_end_matches('/').rsplit('/').next().unwrap_or_default();
    let asked = if line.is_folder {
        ui::FileOrFolder::Folder(last)
    } else {
        ui::FileOrFolder::File(last)
    };
    ui.file_icon_in(path.left(theme.sizes.icon), asked, Tone::Muted);
    let name_width = ui.measure(line.path(), ui.text_style(TextKind::Body));
    let _ = ui.label(path.left(name_width), &Label::new(line.path()));
    let notes = notes(drawn, line, gets);
    if !notes.is_empty() {
        let tone = if gets.is_none() { Tone::Warning } else { Tone::Muted };
        let _ = ui.label(path.rest(), &Label::new(&notes).kind(TextKind::Detail).tone(tone));
    }
    intents
}

/// The notes after the path of a line.
fn notes(drawn: &Drawn<'_, '_>, line: &SetupLine<'_>, gets: Option<Gets>) -> String {
    let mut said = Vec::new();
    if line.is_name() {
        said.push(words::places(line.places.len()));
        if gets.is_none() {
            said.push(words::NOT_THE_SAME.to_owned());
        }
    }
    if line.places.iter().any(|place| place.kind == IgnoredKind::Link) {
        said.push(words::LINK_ALREADY.to_owned());
    }
    match drawn.state.chosen.part_taken(line) {
        Some(Gets::Link) => said.push(words::only_part("linked")),
        Some(Gets::Copy) => said.push(words::only_part("copied")),
        _ => {}
    }
    if gets == Some(Gets::Copy) && line.bytes() > LARGE_COPY {
        said.push(words::LARGE_TO_COPY.to_owned());
    }
    said.join(" \u{b7} ")
}

/// A place of a name in several places: its path, its size, and buttons that choose for it alone.
pub(super) fn place_row(
    drawn: &Drawn<'_, '_>,
    (index, place): (usize, usize),
    ui: &mut Ui<'_>,
    rect: Rect,
) -> Vec<Intent> {
    let Some(line) = drawn.lines.get(index) else {
        return Vec::new();
    };
    let Some(entry) = line.places.get(place) else {
        return Vec::new();
    };
    let state = drawn.state;
    let theme = *ui.theme();
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.large);
    let mut intents = Vec::new();
    let buttons = Buttons {
        key: ("place", index, place),
        gets: Some(state.chosen.place_gets(entry)),
        proposed: line.proposed,
        is_folder: is_folder(entry),
    };
    if let Some(chosen) = buttons.draw(ui, parts.right(gets_width(ui))) {
        let others: Vec<String> = line
            .places
            .iter()
            .filter(|other| other.path != entry.path)
            .map(|other| other.path.clone())
            .collect();
        let patterns = choose(&ChooseInput {
            patterns: state.chosen.patterns,
            line: ChosenLine::Place {
                path: &entry.path,
                is_folder: is_folder(entry),
                others: &others,
            },
            gets: chosen,
        });
        intents.push(page(SetupIntent::SetPatterns(patterns)));
    }
    let _ = ui.label(parts.right(SIZE_WIDTH), &Label::new(&size_of(entry)).tone(Tone::Muted));
    let mut path = Area::with_gap(parts.rest(), theme.sizes.spacing.medium);
    let _indent = path.left(theme.sizes.spacing.large * 2.0 + theme.sizes.icon);
    let _ = ui.label(path.rest(), &Label::new(&entry.path).tone(Tone::Muted));
    intents
}

/// The size of an entry: **12 MB · 1,234 files**, **over 1.2 GB · 5,000+ files**, **a link**, or a file's bytes.
fn size_of(entry: &IgnoredEntry) -> String {
    match &entry.kind {
        IgnoredKind::File { bytes } => words::bytes(*bytes),
        IgnoredKind::Folder(size) => words::folder_size(bytes_of(entry), size.files, size.has_more),
        IgnoredKind::Link => words::A_LINK.to_owned(),
    }
}

/// **Show the other `<n>`** of a section.
pub(super) fn show_others(section: Section, others: usize, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let said = words::show_others(others);
    let button = Button::secondary(&said).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width.min(rect.width), height), (Align::Start, Align::Center));
    if ui.button(("show others", section), cell, &button).clicked {
        return vec![page(SetupIntent::ShowAll(section))];
    }
    debug_assert!(others > 0 || SECTION_CAP > 0);
    Vec::new()
}

/// The three buttons of a line or a place: **Link**, **Copy**, **Nothing**, the one it gets pressed and the one proposed
/// said in its tooltip; **Link** is disabled on a file.
struct Buttons {
    /// The key of the line.
    key: (&'static str, usize, usize),
    /// What it gets; none when its places do not get the same.
    gets: Option<Gets>,
    /// What is proposed for it.
    proposed: Gets,
    /// Whether it is a folder.
    is_folder: bool,
}

impl Buttons {
    /// Draws the buttons over a rectangle; what is chosen with a click.
    fn draw(&self, ui: &mut Ui<'_>, rect: Rect) -> Option<Gets> {
        let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.small);
        let mut chosen = None;
        for (gets, label) in [
            (Gets::Link, words::LINK),
            (Gets::Copy, words::COPY),
            (Gets::Nothing, words::NOTHING),
        ] {
            let tooltip = if gets == Gets::Link && !self.is_folder {
                words::ONLY_FOLDERS.to_owned()
            } else if gets == self.proposed {
                words::proposed(label)
            } else {
                label.to_owned()
            };
            let mut button = if self.gets == Some(gets) {
                Button::primary(label)
            } else {
                Button::secondary(label).kind(ButtonKind::Outline)
            }
            .with_tooltip(&tooltip);
            if gets == Gets::Link && !self.is_folder {
                button = button.disabled();
            }
            let (width, height) = ui.button_size(&button);
            let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
            if ui.button((self.key, label), cell, &button).clicked && self.gets != Some(gets) {
                chosen = Some(gets);
            }
        }
        chosen
    }
}
