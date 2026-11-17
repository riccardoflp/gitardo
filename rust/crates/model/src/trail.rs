//! The way to the page shown, which its title row names (`ui.page-trail`, D57): the pages it was reached through, each
//! opened from the one before, the timeline first and never listed.

use std::ops::Range;

#[cfg(test)]
mod tests;

/// The most names the title row of a page shows, **Timeline** and the page shown among them; those in between are
/// folded into **…**.
pub const MOST_NAMES: usize = 4;

/// How the page shown changed, for the way it was reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WayChange<P> {
    /// A page opened from the page shown, the last of the way: it is added after it, or gone back to when it is on the
    /// way already, so that the way has no loops.
    Opened(P),
    /// A page opened from the bar of views, Quick search, the side column or a key: the way starts again from the
    /// timeline, with that page alone.
    StartedAgain(P),
    /// Back to the page at a place of the way: what came after it is dropped.
    Back(usize),
    /// The timeline shown: the way is empty.
    Timeline,
}

/// The way after a change of the page shown: the pages before the timeline is reached again, the page shown last.
///
/// Feature: ui.page-trail
#[must_use]
pub fn page_trail<P: PartialEq + Clone>(way: &[P], change: WayChange<P>) -> Vec<P> {
    match change {
        WayChange::Opened(page) => match way.iter().position(|visited| *visited == page) {
            Some(at) => way.get(..=at).unwrap_or(way).to_vec(),
            None => way.iter().cloned().chain([page]).collect(),
        },
        WayChange::StartedAgain(page) => vec![page],
        WayChange::Back(at) => way.get(..=at).unwrap_or(way).to_vec(),
        WayChange::Timeline => Vec::new(),
    }
}

/// The way as the title row of the page shown names it: the pages of the way still open, when the last of them is the
/// page shown; otherwise the page shown alone, as one reached from the timeline.
#[must_use]
pub fn way_to<P: PartialEq + Clone>(way: &[P], open: &[P], shown: &P) -> Vec<P> {
    let kept: Vec<P> = way.iter().filter(|page| open.contains(page)).cloned().collect();
    if kept.last() == Some(shown) {
        kept
    } else {
        vec![shown.clone()]
    }
}

/// Which of so many names of a title row, **Timeline** first, are folded into **…**: those between **Timeline** and the
/// last names that fit, none when all of them fit.
#[must_use]
pub fn folded(count: usize) -> Option<Range<usize>> {
    (count > MOST_NAMES).then(|| 1..count - (MOST_NAMES - 1))
}
