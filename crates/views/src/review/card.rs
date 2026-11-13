//! One conflict as a card (`review.card`): its title, the heading that says what the proposal does with the two sides
//! and the counts of where its lines come from, what looks wrong in it, the lines proposed between the lines around
//! them, the choices (`review.choose`), the sides on demand, and at the bottom where the proposal comes from beside
//! **Resolve by hand** and **Accept** (`review.accept`).

use model::conflicts::SideNames;
use model::review::{Origin, ProposalSource, SideRow, Which};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Icon, Label, Rect, Sense, TextKind, Tone, Ui};

use super::decide::{bottom_and_folded, choose_view};
use super::lines::{Look, code_line, gutter_width, origin_bars};
use super::{CardAi, CardProposal, ConflictCard, Layout, NumberedLine, ProposedLine, ReviewIntent};
use crate::intent::Intent;
use crate::text::review as words;

/// What a card shows.
#[derive(Clone, Copy, Debug)]
pub struct CardState<'a> {
    /// The conflict.
    pub card: &'a ConflictCard,
    /// What the two sides are.
    pub sides: &'a SideNames,
    /// Cards or Columns.
    pub layout: Layout,
    /// The widths of the current and incoming columns, in thousandths.
    pub widths: (u32, u32),
    /// Whether an operation runs, which nothing on the card may change meanwhile.
    pub is_running: bool,
    /// Whether the AI provider may be asked about it (`conflicts.ai`).
    pub can_ask: bool,
}

impl CardState<'_> {
    /// The names of the two sides in a sentence.
    pub(super) fn names(&self) -> (String, String) {
        (
            words::side_name(&self.sides.current),
            words::side_name(&self.sides.incoming),
        )
    }
}

/// A part of a card, one under the other.
#[derive(Clone, Copy, Debug)]
enum Piece<'a> {
    /// The arrow that folds it and **Conflict at line N**.
    Title,
    /// What the proposal does with the two sides.
    Heading,
    /// The bar and the counts of where its lines come from.
    Counts(&'a CardProposal),
    /// What looks wrong in it.
    Warnings(&'a CardProposal),
    /// Where the AI stands on it: reading, writing, or why it failed.
    Ai(&'a CardAi),
    /// A line of the answer of the AI so far.
    Streamed(&'a str),
    /// A line of the file around it, faded.
    Around(&'a NumberedLine),
    /// A line proposed.
    Proposed(&'a ProposedLine),
    /// A proposal of nothing.
    DeletesLines,
    /// Nothing is proposed.
    NothingProposed,
    /// The three columns.
    Columns,
    /// The choices.
    Choices,
    /// The heading of a side.
    SideHeading(Which),
    /// A line of a side around its changes.
    SideAround(&'a (usize, String)),
    /// A row of what a side changed.
    SideRow(&'a SideRow),
    /// The heading of the base.
    BaseHeading,
    /// A line of the base.
    BaseLine(&'a str),
    /// Where the proposal comes from, and the buttons.
    Bottom,
    /// The card folded to one line.
    Folded,
}

/// The parts of a card, in order.
fn pieces<'a>(state: &CardState<'a>) -> Vec<Piece<'a>> {
    let card = state.card;
    if card.shown.is_folded {
        return vec![Piece::Folded];
    }
    let mut pieces = vec![Piece::Title, Piece::Heading];
    if card.ai != CardAi::Idle {
        pieces.push(Piece::Ai(&card.ai));
    }
    if let CardAi::Writing(lines) = &card.ai {
        pieces.extend(lines.iter().map(|line| Piece::Streamed(line)));
    }
    if let Some(proposal) = &card.proposal {
        pieces.push(Piece::Counts(proposal));
        if !proposal.warnings.is_empty() {
            pieces.push(Piece::Warnings(proposal));
        }
    }
    if state.layout == Layout::Columns {
        pieces.push(Piece::Columns);
    } else if let Some(proposal) = &card.proposal {
        pieces.extend(card.before.iter().map(Piece::Around));
        pieces.extend(proposal.lines.iter().map(Piece::Proposed));
        if proposal.lines.is_empty() {
            pieces.push(Piece::DeletesLines);
        }
        pieces.extend(card.after.iter().map(Piece::Around));
    }
    if card.proposal.is_none() && card.ai == CardAi::Idle {
        pieces.push(Piece::NothingProposed);
    }
    pieces.push(Piece::Choices);
    if state.layout == Layout::Cards && (card.shown.is_sides_shown || card.proposal.is_none()) {
        for (which, diff) in [
            (Which::Current, &card.side_diffs.0),
            (Which::Incoming, &card.side_diffs.1),
        ] {
            pieces.push(Piece::SideHeading(which));
            pieces.extend(diff.before.iter().map(Piece::SideAround));
            pieces.extend(diff.rows.iter().map(Piece::SideRow));
            pieces.extend(diff.after.iter().map(Piece::SideAround));
        }
        if card.shown.is_base_shown
            && let Some(base) = &card.base
        {
            pieces.push(Piece::BaseHeading);
            pieces.extend(base.iter().map(|line| Piece::BaseLine(line)));
        }
    }
    pieces.push(Piece::Bottom);
    pieces
}

/// The height of a part.
fn piece_height(state: &CardState<'_>, ui: &Ui<'_>, (piece, width): (Piece<'_>, f32)) -> f32 {
    let sizes = ui.theme().sizes;
    match piece {
        Piece::Choices | Piece::Bottom | Piece::Folded => sizes.control_height,
        Piece::Columns => super::columns::height(state, ui, width),
        _ => sizes.row_height,
    }
}

/// The height of a card in a width.
pub(super) fn card_height(state: &CardState<'_>, ui: &Ui<'_>, width: f32) -> f32 {
    let padding = ui.theme().sizes.spacing.medium;
    pieces(state)
        .into_iter()
        .map(|piece| piece_height(state, ui, (piece, width - 2.0 * padding)))
        .sum::<f32>()
        + 2.0 * padding
}

/// A conflict, as a card, over the rectangle its caller clipped it to.
///
/// Feature: review.card
pub fn card_view(state: &CardState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.card_background);
    let padding = theme.sizes.spacing.medium;
    let inner = ui::layout::inset(rect, ui::layout::Insets::symmetric(padding, padding));
    let gutter = gutter_width(ui, largest_number(state.card));
    let mut area = Area::new(inner);
    let mut intents = Vec::new();
    ui.scope(&state.card.key, |ui| {
        for piece in pieces(state) {
            let height = piece_height(state, ui, (piece, inner.width));
            let row = area.top(height);
            draw_piece(state, ui, (piece, row, gutter), &mut intents);
        }
    });
    intents
}

/// The largest number a card shows, for the width of its numbers.
fn largest_number(card: &ConflictCard) -> usize {
    let numbers = card.before.iter().chain(&card.after).map(|line| line.number);
    let proposed = card
        .proposal
        .iter()
        .flat_map(|proposal| proposal.lines.iter().map(|line| line.line.number));
    let sides = [&card.side_diffs.0, &card.side_diffs.1]
        .into_iter()
        .flat_map(|diff| diff.before.iter().chain(&diff.after).map(|(number, _)| *number));
    let columns = [&card.current, &card.incoming]
        .into_iter()
        .flat_map(|column| column.lines.iter().chain(&column.after).map(|line| line.number));
    numbers.chain(proposed).chain(sides).chain(columns).max().unwrap_or(1)
}

/// Draws a part of a card.
fn draw_piece(
    state: &CardState<'_>,
    ui: &mut Ui<'_>,
    (piece, row, gutter): (Piece<'_>, Rect, f32),
    intents: &mut Vec<Intent>,
) {
    let colors = ui.theme().colors;
    match piece {
        Piece::Title => title(state, ui, row, intents),
        Piece::Heading => {
            let said = heading(state);
            let _ = ui.label(row, &Label::new(&said).kind(TextKind::Strong));
        }
        Piece::Counts(proposal) => counts(state, ui, row, proposal),
        Piece::Warnings(proposal) => warnings(ui, row, proposal),
        Piece::Ai(ai) => {
            let (said, tone) = match ai {
                CardAi::Failed(why) => (why.as_str(), Tone::Warning),
                CardAi::Writing(_) => (words::AI_WRITING, Tone::Muted),
                CardAi::Reading | CardAi::Idle => (words::AI_READING, Tone::Muted),
            };
            let _ = ui.label(row, &Label::new(said).tone(tone));
        }
        Piece::Around(line) => plain_line(
            ui,
            (row, gutter),
            (Some(line.number), &line.text, colors.muted_foreground),
        ),
        Piece::Proposed(line) => proposed_line(ui, row, (line, gutter), &state.names()),
        Piece::DeletesLines => {
            let _ = ui.label(row, &Label::new(words::DELETES_LINES).tone(Tone::Muted));
        }
        Piece::NothingProposed => {
            let _ = ui.label(row, &Label::new(words::NOTHING_IS_PROPOSED).tone(Tone::Warning));
        }
        Piece::Columns => intents.extend(ui.clip(row, |ui| super::columns::columns_view(state, ui))),
        Piece::Choices => intents.extend(ui.clip(row, |ui| choose_view(state, ui))),
        Piece::SideHeading(which) => {
            let side = match which {
                Which::Current => &state.sides.current,
                Which::Incoming => &state.sides.incoming,
            };
            let said = words::side_heading(which == Which::Current, side);
            let _ = ui.label(row, &Label::new(&said).kind(TextKind::SmallStrong));
        }
        Piece::SideAround((number, text)) => {
            plain_line(ui, (row, gutter), (Some(*number), text, colors.muted_foreground));
        }
        Piece::SideRow(side_row) => side_line(ui, row, (side_row, gutter)),
        Piece::BaseHeading => {
            let _ = ui.label(row, &Label::new(words::BASE).kind(TextKind::SmallStrong));
        }
        Piece::Streamed(text) | Piece::BaseLine(text) => plain_line(ui, (row, gutter), (None, text, colors.foreground)),
        Piece::Bottom => bottom_and_folded(state, ui, (row, false), intents),
        Piece::Folded => bottom_and_folded(state, ui, (row, true), intents),
    }
}

/// A line of code with no bar, numbered or not.
fn plain_line(ui: &mut Ui<'_>, (row, gutter): (Rect, f32), (number, text, color): (Option<usize>, &str, ui::Color)) {
    let number = number.map(|number| number.to_string()).unwrap_or_default();
    code_line(
        ui,
        row,
        (text, gutter),
        &Look {
            gutter: &number,
            color,
            bars: [None, None],
            tint: None,
        },
    );
}

/// The arrow that folds the card, and **Conflict at line N**, which opens the file at that line
/// (`conflicts.open-file`).
fn title(state: &CardState<'_>, ui: &mut Ui<'_>, row: Rect, intents: &mut Vec<Intent>) {
    let mut area = Area::with_gap(row, ui.theme().sizes.spacing.small);
    if fold_arrow(ui, &mut area, ("fold", Icon::ChevronDown, words::FOLD)) {
        intents.push(Intent::Review(ReviewIntent::Fold(state.card.key.clone(), true)));
    }
    let said = words::conflict_at(state.card.line);
    let button = Button::secondary(&said)
        .kind(ButtonKind::Icon)
        .with_tooltip(words::OPEN_AT_LINE);
    let (width, height) = ui.button_size(&button);
    let cell = place(area.left(width), (width, height), (Align::Start, Align::Center));
    if ui.button("open at line", cell, &button).clicked {
        let path = state.card.key.path.clone();
        let open = crate::conflicts::ConflictsIntent::OpenAtLine(path, state.card.line);
        intents.push(Intent::Conflicts(open));
    }
}

/// An arrow that folds or unfolds, at the left of an area; whether it was clicked.
pub(super) fn fold_arrow(ui: &mut Ui<'_>, area: &mut Area, (key, icon, said): (&str, Icon, &str)) -> bool {
    let side = ui.theme().sizes.control_height;
    let cell = place(area.left(side), (side, side), (Align::Start, Align::Center));
    ui.button(key, cell, &Button::icon(icon, said).kind(ButtonKind::Icon))
        .clicked
}

/// What the heading says.
fn heading(state: &CardState<'_>) -> String {
    let (current, incoming) = state.names();
    match &state.card.proposal {
        Some(proposal) => words::heading(Some(proposal.shape), &proposal.counts, (&current, &incoming)),
        None => words::heading(None, &model::review::OriginCounts::default(), (&current, &incoming)),
    }
}

/// The bar of where the lines come from, and their counts.
fn counts(state: &CardState<'_>, ui: &mut Ui<'_>, row: Rect, proposal: &CardProposal) {
    let theme = *ui.theme();
    let (current, incoming) = state.names();
    let is_from_ai = matches!(proposal.source, ProposalSource::Ai { .. });
    let said = words::origin_counts(&proposal.counts, (&current, &incoming), is_from_ai);
    let bar_width = 80.0_f32.min(row.width * 0.25);
    let mut area = Area::with_gap(row, theme.sizes.spacing.small);
    let bar = place(
        area.left(bar_width),
        (bar_width, theme.sizes.border * 4.0),
        (Align::Start, Align::Center),
    );
    let counts = proposal.counts;
    let total = counts.current + counts.incoming + counts.both + counts.base + counts.new;
    if total > 0 {
        let mut left = bar.x;
        for (count, color) in [
            (counts.current, theme.colors.success),
            (counts.both, theme.colors.accent),
            (counts.incoming, theme.colors.info),
            (counts.base, theme.colors.muted_foreground),
            (counts.new, super::lines::new_color(ui)),
        ] {
            let width = bar.width * crate::number::float_of_count(count) / crate::number::float_of_count(total);
            ui.fill_rect(Rect::new(left, bar.y, width, bar.height), color);
            left += width;
        }
    }
    let _ = ui.label(area.rest(), &Label::new(&said).kind(TextKind::Small).tone(Tone::Muted));
}

/// What looks wrong in a proposal, the whole of each in the tooltip.
fn warnings(ui: &mut Ui<'_>, row: Rect, proposal: &CardProposal) {
    let Some(said) = words::warnings(&proposal.warnings) else {
        return;
    };
    let mut area = Area::with_gap(row, ui.theme().sizes.spacing.small);
    let side = ui.theme().sizes.row_height;
    ui.icon_in(area.left(side), Icon::Warning, Tone::Warning);
    let cell = area.rest();
    let _ = ui.label(cell, &Label::new(&said).tone(Tone::Warning));
    let whole: Vec<String> = proposal.warnings.iter().map(words::warning).collect();
    let id = ui.id("warnings");
    let response = ui.interact(id, cell, Sense::HOVER);
    ui.tooltip(&response, &whole.join("\n"));
}

/// A line proposed, with the bar of where it comes from, and the pointer on it saying so.
fn proposed_line(ui: &mut Ui<'_>, row: Rect, (line, gutter): (&ProposedLine, f32), names: &(String, String)) {
    let colors = ui.theme().colors;
    let (bars, tint) = origin_bars(ui, line.origin);
    let tint = if line.is_warned {
        Some(colors.warning.with_alpha(40))
    } else {
        tint
    };
    code_line(
        ui,
        row,
        (&line.line.text, gutter),
        &Look {
            gutter: &line.line.number.to_string(),
            color: colors.foreground,
            bars,
            tint,
        },
    );
    if let Some(said) = origin_words(line.origin, names) {
        let id = ui.id(("line", line.line.number));
        let response = ui.interact(id, row, Sense::HOVER);
        ui.tooltip(&response, &said);
    }
}

/// What the pointer on a line proposed says.
pub(super) fn origin_words(origin: Origin, (current, incoming): &(String, String)) -> Option<String> {
    Some(match origin {
        Origin::Current => words::from_side(current),
        Origin::Incoming => words::from_side(incoming),
        Origin::Both => String::from(words::ON_BOTH_SIDES),
        Origin::Base => String::from(words::AS_BEFORE),
        Origin::New => String::from(words::NEW_LINE),
        Origin::Blank => return None,
    })
}

/// A row of what a side changed: the lines alike numbered, those it took out with `-`, those it wrote with `+`.
fn side_line(ui: &mut Ui<'_>, row: Rect, (side_row, gutter): (&SideRow, f32)) {
    let colors = ui.theme().colors;
    let plain = Look {
        gutter: "",
        color: colors.foreground,
        bars: [None, None],
        tint: None,
    };
    match side_row {
        SideRow::Same { number, text } => {
            let number = number.to_string();
            code_line(
                ui,
                row,
                (text, gutter),
                &Look {
                    gutter: &number,
                    ..plain
                },
            );
        }
        SideRow::Removed { text, .. } => code_line(
            ui,
            row,
            (text, gutter),
            &Look {
                gutter: "-",
                tint: Some(colors.removed_background),
                ..plain
            },
        ),
        SideRow::Added { text, .. } => code_line(
            ui,
            row,
            (text, gutter),
            &Look {
                gutter: "+",
                tint: Some(colors.added_background),
                ..plain
            },
        ),
    }
}
