//! The timeline and the header above it: the commits (`graph.view`) with their graph, labels, columns, draft, stashes,
//! folds, sticky names, marks and edge; moving through them with the keyboard; the header (`graph.toolbar`), its
//! menu of the view (`graph.view-menu`), its **…** menu (`graph.more-menu`) and the panes beside the timeline
//! (`graph.resize-panes`).

mod cell;
mod columns;
pub(crate) mod commit_menu;
mod compact;
mod draft;
mod edge;
mod fold;
mod hover;
mod intent;
mod keyboard;
mod label_overflow;
mod labels;
mod layout;
mod load_more;
mod location;
mod marks;
mod more_menu;
mod panes;
mod pull_request_badge;
mod row;
mod stashes;
mod state;
mod sticky;
mod switches;
mod sync_menu;
#[cfg(test)]
mod tests;
mod timeline;
mod toggle;
mod toolbar;
mod view_menu;

pub use columns::{ColumnsState, columns_view};
pub(crate) use commit_menu::with_compare;
pub use compact::{CompactRow, CompactWidths, RowLines, compact_rows_view};
pub use draft::{
    DraftEntry, UncommittedRow, draft_entries, draft_entry_intent, draft_menu, entry_operation, uncommitted_row_view,
};
pub use edge::{GraphEdgeState, resize_graph_view};
pub use fold::{FoldAllEntry, FoldAllState, FoldsState, MergeEntry, fold_all_view, fold_merge_view, merge_menu};
pub use hover::{HoverState, LINGER, REST, hover_history_view, hover_intent};
pub use intent::{GraphIntent, HoverTarget, HoverWay, Pane, Selected, SideSection, Switches, TimelineSwitch};
pub use keyboard::{key_intent, keyboard_view};
pub use label_overflow::{LabelList, LabelOverflow, OPEN_AFTER, label_overflow_view};
pub(crate) use labels::draw_labels;
pub use layout::{LayoutEnd, LayoutInput, TimelineLayout};
pub use load_more::{AHEAD, LoadMoreState, load_more_view, wants_more};
pub use location::{WhereState, WhereWidths, fit_where, where_view};
pub use marks::{ScrollMarks, mark_positions, scroll_marks_view};
pub use more_menu::{MainRemote, MoreMenuState, OperationsMenu, more_menu_view};
pub use panes::{
    INSPECTOR_LEAST, INSPECTOR_MOST, INSPECTOR_START, PaneLayout, PaneRects, PaneSizes, RAIL_LEAST, RAIL_MOST,
    RAIL_START, STEP, pane_rects, resize_panes_view,
};
pub use pull_request_badge::{
    PullRequestBadge, draw_pull_request_badge, pull_request_badge_tone, pull_request_badge_view,
    pull_request_badge_width,
};
pub use stashes::{StashEntry, StashRow, stash_intent, stash_menu, stashes_view};
pub use state::{DraftState, Folds, GraphState, HistoryState, Reading, TraceShown, TraceStrength};
pub use sticky::{StickyName, StickyNames, sticky_branch_names_view, sticky_names};
pub use switches::{FirstParentState, SelectedTrace, first_parent_view, trace_selected_view};
pub use sync_menu::{PULL_WAYS, SYNC_FEATURES};
pub use timeline::view;
pub use toggle::{ReferencesShown, toggle_references_view};
pub(crate) use toggle::{side_button, top_row_height};
pub use toolbar::{HeadName, RepositoryEntry, SyncCounts, SyncMenu, ToolbarState, toolbar_view};
#[cfg(test)]
pub(crate) use view_menu::kind_entries;
pub use view_menu::{GraphDrawn, TimelineMenu, ViewMenuState, view_menu_view};
