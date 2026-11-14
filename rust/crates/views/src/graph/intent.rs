//! What the user asks of the timeline: what its views return inside [`crate::Intent::Graph`].

use model::CommitId;

/// Something the user asked of the timeline.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphIntent {
    /// Select a row, by a click or a key, and bring it into sight (`graph.view`, `graph.keyboard`); it empties the
    /// selection of several.
    Select(Selected),
    /// Select several commits at once, by `Ctrl`+click or `Shift`+click (`selection.select`).
    SelectSeveral {
        /// The commits, the anchor of `Shift`+click first.
        commits: Vec<CommitId>,
        /// The one the inspector shows: the one clicked last that is in the selection.
        shown: CommitId,
    },
    /// Empty the selection of several, keeping the commit shown selected alone (`selection.bar`).
    ClearSelection,
    /// Copy the full hashes of commits, one per line, to the clipboard (`selection.menu`).
    CopyHashes(Vec<CommitId>),
    /// Open a commit as a page, by a double click or `Enter` (`commit-page.open`).
    Open(CommitId),
    /// Read the next page: the commits loaded when it was asked for (`graph.load-more`).
    LoadMore {
        /// How many commits were loaded.
        loaded: usize,
    },
    /// Fold the commits a merge brought in (`graph.fold-merge`).
    Fold(CommitId),
    /// Show the commits of a folded merge again.
    Unfold(CommitId),
    /// **Fold all merges** (`graph.fold-all`).
    FoldAll,
    /// **Unfold all merges**.
    UnfoldAll,
    /// The pointer rests on a label, or left the labels (`graph.hover-history`).
    Hover(Option<HoverTarget>),
    /// The pointer left the labels at a time of the frames: the history goes 100 ms later.
    HoverLeft(std::time::Duration),
    /// Scroll to a commit, centered, and select it: a sticky branch name clicked (`graph.sticky-branch-names`).
    GoTo(CommitId),
    /// The width the graph is drawn in, dragged; none brings back the one the graph chooses (`graph.resize-graph`).
    SetGraphWidth(Option<f32>),
    /// The size of a pane, dragged; none brings back where it starts (`graph.resize-panes`).
    SetPane(Pane, Option<f32>),
    /// Put the branch rail away, or bring it back (`graph.toggle-references`).
    ToggleReferences,
    /// Turn a switch of the timeline on or off, from the **…** menu (`graph.more-menu`).
    Switch(TimelineSwitch, bool),
    /// Something was dropped on a row, which opened the menu of the drop (`drag.drop`).
    Dropped(crate::drag::Dropped),
}

/// A row that can be selected.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Selected {
    /// The draft of the uncommitted changes.
    Draft,
    /// A commit or a stash, by its hash.
    Commit(CommitId),
}

/// A label the pointer rests on, and which of its history stands out.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HoverTarget {
    /// The commit of the label.
    pub commit: CommitId,
    /// Which of its history.
    pub way: HoverWay,
}

/// Which history of a label stands out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HoverWay {
    /// What it reaches.
    Reaches,
    /// With `Shift`: what it has that the commit checked out has not.
    NotCheckedOut,
}

/// A pane whose size is dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pane {
    /// The width of the branch rail.
    Rail,
    /// The width of the inspector beside the commits.
    Inspector,
    /// The height of the inspector over the commits, in the narrow layout.
    InspectorOver,
}

/// A switch of the timeline in the **…** menu, each written in the user's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimelineSwitch {
    /// `graph.showStashes`.
    ShowStashes,
    /// `graph.firstParent`.
    FirstParent,
    /// `graph.traceSelected`.
    TraceSelected,
    /// `author` in `graph.columns`.
    Author,
    /// `date` in `graph.columns`.
    Date,
    /// `hash` in `graph.columns`.
    Hash,
    /// `graph.dateSeparators`.
    DateSeparators,
    /// `graph.combineUpstream`.
    CombineUpstream,
}

impl TimelineSwitch {
    /// Every switch, in the order of the menu.
    pub const ALL: [TimelineSwitch; 8] = [
        TimelineSwitch::ShowStashes,
        TimelineSwitch::FirstParent,
        TimelineSwitch::TraceSelected,
        TimelineSwitch::Author,
        TimelineSwitch::Date,
        TimelineSwitch::Hash,
        TimelineSwitch::DateSeparators,
        TimelineSwitch::CombineUpstream,
    ];
}

/// The switches of the timeline that are on, as one value that is copied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Switches(u16);

impl Switches {
    /// The switches on by default: all but `graph.firstParent`.
    pub const DEFAULT: Switches = Switches(0b1111_1101);

    /// Whether a switch is on.
    #[must_use]
    pub fn is_on(self, switch: TimelineSwitch) -> bool {
        self.0 & Switches::bit(switch) != 0
    }

    /// The switches with one turned on or off.
    #[must_use]
    pub fn with(self, switch: TimelineSwitch, is_on: bool) -> Switches {
        if is_on {
            Switches(self.0 | Switches::bit(switch))
        } else {
            Switches(self.0 & !Switches::bit(switch))
        }
    }

    /// The bit of a switch.
    fn bit(switch: TimelineSwitch) -> u16 {
        1 << (switch as u16)
    }
}
