//! The words of the Welcome page (`welcome.page`).

/// The title of the page.
pub const TITLE: &str = "Get started with Gitardo";
/// The button that opens a folder.
pub const OPEN_FOLDER: &str = "Open folder\u{2026}";
/// The title of the recent workspaces.
pub const RECENT: &str = "Recent";
/// Said of a step that is done, for the screen reader.
pub const DONE: &str = "done";

/// A step of the page: its title, what it says, and the words of its buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepWords {
    /// Its title.
    pub title: &'static str,
    /// What it says.
    pub says: &'static str,
    /// The words of its buttons.
    pub buttons: &'static [&'static str],
}

/// The five steps, in their order.
pub const STEPS: [StepWords; 5] = [
    StepWords {
        title: "Open the timeline",
        says: "The branches on the left, the commits of every branch in the middle, the selected commit on the right",
        buttons: &["Show the timeline"],
    },
    StepWords {
        title: "Find the branches",
        says: "The branch rail lists the branches in the colors of their lanes, and keeps the stashes, the worktrees \
               and the tags aside; a branch picked there narrows the timeline to its history",
        buttons: &["Show the commits"],
    },
    StepWords {
        title: "Open a commit, compare two branches",
        says: "A double click or Enter on a commit reads it whole on a page; a comparison says what one side has that \
               the other has not, and what merging them would do",
        buttons: &["Open a commit\u{2026}", "Compare\u{2026}"],
    },
    StepWords {
        title: "See who changed each line",
        says: "The File page says who last changed each line",
        buttons: &["Open a file\u{2026}"],
    },
    StepWords {
        title: "Work in another worktree",
        says: "A worktree is another folder of the same repository on a branch of its own",
        buttons: &["Create a worktree\u{2026}"],
    },
];
