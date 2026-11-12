//! Wrapping text by words: as many words on each line as fit the width, the spaces where a line breaks left out.

use std::ops::Range;

use super::layout::{self, Placed};
use super::{Fonts, TextStyle};

/// The lines of a text wrapped to a width, as ranges of its bytes.
pub(crate) fn wrap(fonts: &Fonts, text: &str, style: TextStyle, width: f32) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut paragraph_start = 0;
    for paragraph in text.split('\n') {
        let mut placements = Vec::new();
        let _ = layout::place(fonts, paragraph, style, |placed| placements.push(placed));
        for line in wrap_paragraph(&placements, width) {
            lines.push(paragraph_start + line.start..paragraph_start + line.end);
        }
        paragraph_start += paragraph.len() + 1;
    }
    lines
}

/// Where a line may break: after the last character of a word, and before the first character of the next word.
#[derive(Clone, Copy, Debug)]
struct Break {
    /// The byte after the last character before the spaces.
    end: usize,
    /// Which placed character starts the next line: the first after the spaces.
    next: usize,
}

/// The lines of one paragraph, its characters already placed, as ranges of the bytes of the paragraph.
fn wrap_paragraph(placements: &[Placed], width: f32) -> Vec<Range<usize>> {
    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut last_break: Option<Break> = None;
    let mut index = 0;
    while let Some(placed) = placements.get(index) {
        let is_space = is_space_at(placements, index);
        if is_space && index > line_start && !is_space_at(placements, index - 1) {
            let next = (index..placements.len())
                .find(|later| !is_space_at(placements, *later))
                .unwrap_or(placements.len());
            last_break = Some(Break {
                end: placed.start,
                next,
            });
        }
        let line_x = placements.get(line_start).map_or(0.0, |first| first.x);
        let overflows = !is_space && index > line_start && placed.x + placed.advance - line_x > width;
        if overflows {
            let (end, next) = match last_break {
                Some(found) => (found.end, found.next),
                None => (placed.start, index),
            };
            lines.push(byte_at(placements, line_start)..end);
            line_start = next;
            last_break = None;
            index = next;
            continue;
        }
        index += 1;
    }
    let end = placements.last().map_or(0, |last| last.end);
    let start = byte_at(placements, line_start).min(end);
    let trimmed_end = (line_start..placements.len())
        .rev()
        .find(|at| !is_space_at(placements, *at))
        .and_then(|at| placements.get(at))
        .map_or(start, |last| last.end);
    lines.push(start..trimmed_end.max(start));
    lines
}

/// Whether the placed character at an index is a space a line may break at.
fn is_space_at(placements: &[Placed], index: usize) -> bool {
    placements.get(index).is_some_and(|placed| placed.is_space)
}

/// The byte where the placed character at an index starts, or the end of the paragraph after its last one.
fn byte_at(placements: &[Placed], index: usize) -> usize {
    placements
        .get(index)
        .map_or_else(|| placements.last().map_or(0, |last| last.end), |placed| placed.start)
}
