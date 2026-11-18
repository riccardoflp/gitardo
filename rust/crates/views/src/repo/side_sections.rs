//! Where the sections of the side column go (`repo.side-column`): one under the other, each heading but the first under
//! a line that is there whatever is folded; a folded section is its heading alone, and the open ones share what is left
//! of the height, each at the height it wants but the last open one, which takes the rest. The line under an open
//! section that has another open section under it is the one that is dragged.

use ui::Rect;
use ui::layout::Area;

/// The least height of a section while another is open with it, its heading included.
pub const SECTION_LEAST: f32 = 64.0;

/// What the layout is told of a section.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionAsk {
    /// Whether it is folded to its heading.
    pub is_folded: bool,
    /// The height it wants while a section under it is open too, its heading included: as dragged, or where it starts.
    pub wanted: f32,
}

/// Where a section goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionPlace {
    /// The line above its heading, which every section but the first has, open or folded.
    pub line: Option<Rect>,
    /// Its heading.
    pub heading: Rect,
    /// What it shows under its heading, while it is open.
    pub body: Option<Rect>,
    /// The line under it, while that line is dragged: the section is open and so is one under it.
    pub dragged: Option<DraggedLine>,
}

/// A line under a section that is dragged to change the height of the section.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DraggedLine {
    /// The top of the section, which the height is counted from.
    pub top: f32,
    /// The height of the section, its heading included: the line is right under it.
    pub height: f32,
    /// The height the section shares with the open sections under it.
    pub room: f32,
    /// The least of that height the open sections under it keep.
    pub least_after: f32,
}

/// Where the sections go in a column, from the heights of a heading and of a line: the headings one under the other,
/// each but the first under its line; an open section as high as it wants, never less than [`SECTION_LEAST`] nor so
/// high that an open one under it has less, while there is room for them; the last open section takes what is left, so
/// that the headings of the folded ones under it are at the foot of the column.
#[must_use]
pub fn section_places<const N: usize>(
    column: Rect,
    asks: [SectionAsk; N],
    (heading, line): (f32, f32),
) -> [SectionPlace; N] {
    let folded = asks.iter().filter(|ask| ask.is_folded).count();
    let mut open_left = N - folded;
    let fixed = count_height(N.saturating_sub(1), line) + count_height(folded, heading);
    let mut room = (column.height - fixed).max(0.0);
    let mut area = Area::new(column);
    let mut is_first = true;
    asks.map(|ask| {
        let rule = if is_first { None } else { Some(area.top(line)) };
        is_first = false;
        if ask.is_folded {
            return SectionPlace {
                line: rule,
                heading: area.top(heading),
                body: None,
                dragged: None,
            };
        }
        open_left = open_left.saturating_sub(1);
        let least_after = count_height(open_left, SECTION_LEAST);
        let height = if open_left == 0 {
            room
        } else {
            ask.wanted
                .clamp(SECTION_LEAST, (room - least_after).max(SECTION_LEAST))
                .min(room)
        };
        let dragged = (open_left > 0).then_some(DraggedLine {
            top: area.rest().y,
            height,
            room,
            least_after,
        });
        room = (room - height).max(0.0);
        let heading = area.top(heading);
        SectionPlace {
            line: rule,
            heading,
            body: Some(area.top((height - heading.height).max(0.0))),
            dragged,
        }
    })
}

/// The height of a count of headings, lines or least sections of one height.
fn count_height(count: usize, height: f32) -> f32 {
    #[allow(clippy::cast_precision_loss, reason = "a side column has three sections")]
    let count = count as f32;
    count * height
}
