//! What the blame of the File page in front is drawn with: what is said beside each line (`blame.format-line`), the
//! color of the bar of each line (`blame.age-color`) and what the end of the line under the cursor says, made once for
//! each state of the window, from the moment the dates are said from.

use model::Timestamp;
use model::blame::{AgeInput, BlameLine, FormatInput, NOT_COMMITTED, age_color, ago, format_line};
use model::graph::DateTime;
use ui::Color;
use views::blame::{FileBlame, LineNote, PageBlame};
use views::file_view::FilePageId;
use views::text::blame as words;

use crate::store::state::State;

/// What the blame of the File page in front is drawn with.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BlameData {
    /// The page.
    page: Option<FilePageId>,
    /// What is said beside each line.
    texts: Vec<String>,
    /// The color of the bar of each line.
    bars: Vec<Option<Color>>,
    /// How many columns the longest text takes.
    width: usize,
    /// What the end of the line under the cursor says.
    note: Option<String>,
}

impl BlameData {
    /// What the blame of the page in front is drawn with, the dates said from a moment, in seconds since 1970.
    #[must_use]
    pub fn new(state: &State, now: i64) -> BlameData {
        let Some(id) = state.file_in_front() else {
            return BlameData::default();
        };
        let Some(blame) = state.blames.pages.get(&id) else {
            return BlameData::default();
        };
        let look = &state.settings.file_blame;
        let (texts, bars) = blame.lines.as_deref().map_or_else(Default::default, |lines| {
            let texts = format_line(&FormatInput {
                format: &look.format,
                lines,
                now: DateTime {
                    moment: Timestamp(now),
                    utc_offset: 0,
                },
                group_lines: look.group_lines,
            });
            let bars = lines
                .iter()
                .map(|line| {
                    let color = age_color(&AgeInput {
                        line: line.as_ref(),
                        now: Timestamp(now),
                        is_enabled: look.age_enabled,
                        recent: &look.recent_color,
                        old: &look.old_color,
                        old_after_days: look.old_after_days,
                    });
                    color.as_deref().and_then(color_of)
                })
                .collect();
            (texts, bars)
        });
        let note = blame
            .line
            .as_ref()
            .and_then(|(_, line)| line.as_ref())
            .map(|line| match line {
                BlameLine::NotCommitted => String::from(NOT_COMMITTED),
                BlameLine::Committed(commit) => {
                    words::line_note(&commit.author, &ago(now - commit.author_time.0), &commit.summary)
                }
            });
        BlameData {
            page: Some(id),
            width: texts.iter().map(|text| text.chars().count()).max().unwrap_or(0),
            texts,
            bars,
            note,
        }
    }

    /// The blame a page shows.
    #[must_use]
    pub fn page_blame<'a>(&'a self, state: &'a State, id: FilePageId) -> PageBlame<'a> {
        let settings = &state.settings.features.blame;
        let can_blame = state
            .file_pages
            .pages
            .iter()
            .any(|page| page.id == id && page.document().is_some());
        let mut shown = PageBlame {
            can_blame,
            ignores_whitespace: settings.ignores_whitespace,
            is_on_every_file: settings.is_on_every_file,
            hovered: state
                .blames
                .hovered
                .filter(|(page, _)| *page == id)
                .map(|(_, line)| line),
            ..PageBlame::default()
        };
        let Some(blame) = state.blames.pages.get(&id).filter(|_| self.page == Some(id)) else {
            return shown;
        };
        shown.file = blame.lines.as_deref().map(|lines| FileBlame {
            lines,
            texts: &self.texts,
            bars: &self.bars,
            width: self.width,
        });
        shown.note = match (&blame.line, &self.note) {
            (Some((line, Some(blamed))), Some(text)) => Some(LineNote {
                line: *line,
                text,
                blame: blamed,
            }),
            _ => None,
        };
        shown
    }
}

/// A color written `#rrggbb`.
fn color_of(written: &str) -> Option<Color> {
    let digits = written.strip_prefix('#').filter(|digits| digits.len() == 6)?;
    u32::from_str_radix(digits, 16).ok().map(Color::hex)
}
