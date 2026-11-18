//! The rows the timeline listed before a read of the same history found other commits (D72): kept aside with their
//! layout, to be drawn dimmed under the rows read until the read reaches them or ends.

use std::sync::Arc;

use model::graph::Row;
use views::graph::TimelineLayout;

use super::Timeline;

/// The rows a timeline listed before a read of the same history found other commits, after a commit, a fetch or
/// **Refresh**: the list keeps their length, so that it stays where it is scrolled to, and the graph its width (D72).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Awaited {
    /// The rows, the draft first when there was one.
    pub rows: Vec<Row>,
    /// Their layout, without the labels: the references are those of the read.
    pub layout: Arc<TimelineLayout>,
}

/// Keeps aside the rows listed, and their layout without its labels, before a page of a read of the same history takes
/// out those from `at` on (D72); the rows read stay, the draft taken out of them. The rows of an earlier read that has
/// not reached them yet stay those kept aside.
pub(super) fn hold(timeline: &mut Timeline, at: usize, draft: Option<&Row>) {
    let listed = std::mem::take(&mut timeline.rows);
    timeline.rows = listed.get(..at).map(<[Row]>::to_vec).unwrap_or_default();
    let mut layout = std::mem::take(&mut timeline.layout);
    if timeline.awaited.is_some() {
        timeline.layout = layout;
        return;
    }
    Arc::make_mut(&mut layout).labels.clear();
    let rows = draft.cloned().into_iter().chain(listed).collect();
    timeline.awaited = Some(Awaited { rows, layout });
}
