//! The made-up repository the gallery shows: its commits, which repeat in blocks of ten down a history of a million,
//! the people who wrote them, its branches, remote branches and what is kept aside, and the files of a commit.

use std::hash::Hasher;

use crate::id::StableHasher;

/// A person who wrote commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Author {
    /// The name.
    pub(super) name: &'static str,
    /// The letters of the round picture beside the name.
    pub(super) initials: &'static str,
    /// The e-mail address.
    pub(super) email: &'static str,
}

/// The people who wrote the commits.
const AUTHORS: [Author; 3] = [
    Author {
        name: "Dana Ruiz",
        initials: "DR",
        email: "dana@example.com",
    },
    Author {
        name: "Ken Ito",
        initials: "KI",
        email: "ken@example.com",
    },
    Author {
        name: "Mara Lind",
        initials: "ML",
        email: "mara@example.com",
    },
];

/// A commit of a block of ten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Commit {
    /// The first line of its message.
    subject: &'static str,
    /// The rest of its message.
    body: &'static str,
    /// Who wrote it, by its place among the authors.
    author: usize,
    /// Its hash, in the first block.
    hash: &'static str,
    /// When it was written, in the first block.
    date: &'static str,
}

/// The commits of a block, newest first.
const COMMITS: [Commit; BLOCK] = [
    Commit {
        subject: "Add the login page",
        body: "The page asks for the e-mail and the password, and says what is wrong under each field rather than in a \
               dialog.",
        author: 0,
        hash: "4b830a6",
        date: "2 hours ago",
    },
    Commit {
        subject: "Validate the e-mail before sending it",
        body: "An address with no @ or no dot after it is refused before the form is sent.",
        author: 0,
        hash: "2c53511",
        date: "3 hours ago",
    },
    Commit {
        subject: "Merge branch 'fix/parser' into main",
        body: "Brings the reading of sections that have no name.",
        author: 1,
        hash: "bd04041",
        date: "yesterday",
    },
    Commit {
        subject: "Read a section that has no name",
        body: "A section with empty brackets was read as the end of the file, and what followed it was lost.",
        author: 1,
        hash: "cd6fa2f",
        date: "yesterday",
    },
    Commit {
        subject: "Speed up the reading of large histories",
        body: "The commits are read in pages of five hundred, and the next page while the first is drawn.",
        author: 2,
        hash: "711656c",
        date: "2 days ago",
    },
    Commit {
        subject: "Add a test for empty sections",
        body: "The test reads a file whose only section has no name.",
        author: 1,
        hash: "64abc8a",
        date: "2 days ago",
    },
    Commit {
        subject: "Keep the selection when the list is read again",
        body: "The rows selected stay selected when a fetch brings new commits above them.",
        author: 2,
        hash: "1a18795",
        date: "3 days ago",
    },
    Commit {
        subject: "Draw the lines of the graph in the colors of the theme",
        body: "Each lane takes the next of the four colors of the theme.",
        author: 0,
        hash: "00a00b0",
        date: "4 days ago",
    },
    Commit {
        subject: "Cancel a fetch that takes too long",
        body: "A fetch that has received nothing for a minute stops, and says so.",
        author: 2,
        hash: "be627f3",
        date: "5 days ago",
    },
    Commit {
        subject: "Explain the shortcuts in the README",
        body: "The shortcuts of the timeline are listed with what each does.",
        author: 1,
        hash: "c1c03de",
        date: "last week",
    },
];

/// How many commits a block has.
pub(super) const BLOCK: usize = 10;

/// The commit of a block a row shows.
fn commit(row: usize) -> &'static Commit {
    let [first, ..] = &COMMITS;
    COMMITS.get(row % BLOCK).unwrap_or(first)
}

/// The first line of the message of the commit of a row.
pub(super) fn subject(row: usize) -> &'static str {
    commit(row).subject
}

/// The rest of the message of the commit of a row.
pub(super) fn body(row: usize) -> &'static str {
    commit(row).body
}

/// Who wrote the commit of a row.
pub(super) fn author(row: usize) -> Author {
    let [first, ..] = AUTHORS;
    AUTHORS.get(commit(row).author).copied().unwrap_or(first)
}

/// The short hash of the commit of a row: that of the design in the first block, seven hexadecimal digits made from
/// the number of the row below it.
pub(super) fn short_hash(row: usize) -> String {
    if row < BLOCK {
        return commit(row).hash.to_owned();
    }
    let mut hasher = StableHasher::new();
    hasher.write_usize(row);
    format!("{:07x}", hasher.finish() & 0x0FFF_FFFF)
}

/// When the commit of a row was written, older down the history.
pub(super) fn date(row: usize) -> String {
    if row < BLOCK {
        return commit(row).date.to_owned();
    }
    let days = row / 2 + 7;
    let (count, unit) = match days {
        0..30 => (days / 7, "week"),
        30..365 => (days / 30, "month"),
        _ => (days / 365, "year"),
    };
    if count == 1 {
        format!("a {unit} ago")
    } else {
        format!("{count} {unit}s ago")
    }
}

/// The text find in page sees in a row, which also names it for the screen reader: its hash and its subject.
pub(super) fn row_text(row: usize) -> String {
    format!("{} {}", short_hash(row), subject(row))
}

/// A branch, a remote branch or a tag shown on a commit, in the color of its lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reference {
    /// Its name.
    pub(super) name: &'static str,
    /// The lane whose color it takes.
    pub(super) lane: usize,
}

/// The references on the commit of a row: those of the design, on the first block.
pub(super) fn references(row: usize) -> &'static [Reference] {
    const LOGIN: [Reference; 1] = [Reference {
        name: "feature/login-page",
        lane: 1,
    }];
    const MAIN: [Reference; 1] = [Reference { name: "main", lane: 0 }];
    const PARSER: [Reference; 1] = [Reference {
        name: "fix/parser",
        lane: 2,
    }];
    const TAG: [Reference; 1] = [Reference {
        name: "v2.4.0",
        lane: 3,
    }];
    match row {
        0 => &LOGIN,
        2 => &MAIN,
        3 => &PARSER,
        7 => &TAG,
        _ => &[],
    }
}

/// A branch of the branch rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RailBranch {
    /// Its name.
    pub(super) name: &'static str,
    /// The lane of the graph it is drawn in.
    pub(super) lane: usize,
    /// How it stands: how far ahead of its upstream, or merged.
    pub(super) state: &'static str,
}

/// The branches of the repository, the one checked out first.
pub(super) const BRANCHES: [RailBranch; 4] = [
    RailBranch {
        name: "feature/login-page",
        lane: 1,
        state: "\u{2191}2",
    },
    RailBranch {
        name: "main",
        lane: 0,
        state: "",
    },
    RailBranch {
        name: "fix/parser",
        lane: 2,
        state: "merged",
    },
    RailBranch {
        name: "spike/graph-colors",
        lane: 3,
        state: "\u{2191}5",
    },
];

/// The remote branches of origin, with the lane of the branch each is the upstream of; none for one with no branch.
pub(super) const REMOTES: [(&str, Option<usize>); 3] = [
    ("origin/main", Some(0)),
    ("origin/feature/login-page", Some(1)),
    ("origin/release/2.4", None),
];

/// What is kept aside, in the last section of the rail: its name, and what it holds.
pub(super) const KEPT: [(&str, &[&str]); 3] = [
    (
        "Stashes",
        &[
            "WIP on feature/login-page",
            "Before the rebase onto main",
            "Try the new parser",
        ],
    ),
    (
        "Worktrees",
        &["gitardo-review  fix/parser", "gitardo-spike  spike/graph-colors"],
    ),
    (
        "Tags",
        &[
            "v2.4.0", "v2.3.1", "v2.3.0", "v2.2.0", "v2.1.2", "v2.1.1", "v2.1.0", "v2.0.0", "v1.9.0", "v1.8.1",
            "v1.8.0", "v1.7.0", "v1.6.0", "v1.0.0",
        ],
    ),
];

/// A file a commit changed, with the lines it added and removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChangedFile {
    /// Its path.
    pub(super) path: &'static str,
    /// The lines added.
    pub(super) added: u32,
    /// The lines removed.
    pub(super) removed: u32,
}

/// The files of the commits: those of the design for the first, and a few others in turn for the rest.
pub(super) fn files(row: usize) -> &'static [ChangedFile] {
    const LOGIN: [ChangedFile; 4] = [
        ChangedFile {
            path: "src/login/page.rs",
            added: 84,
            removed: 0,
        },
        ChangedFile {
            path: "src/login/form.rs",
            added: 41,
            removed: 6,
        },
        ChangedFile {
            path: "src/router.rs",
            added: 3,
            removed: 1,
        },
        ChangedFile {
            path: "tests/login.rs",
            added: 52,
            removed: 0,
        },
    ];
    const PARSER: [ChangedFile; 2] = [
        ChangedFile {
            path: "src/config/parser.rs",
            added: 12,
            removed: 4,
        },
        ChangedFile {
            path: "tests/config.rs",
            added: 27,
            removed: 0,
        },
    ];
    const README: [ChangedFile; 1] = [ChangedFile {
        path: "README.md",
        added: 18,
        removed: 3,
    }];
    match row % BLOCK {
        0 => &LOGIN,
        9 => &README,
        _ => &PARSER,
    }
}
