//! Which of Lucide's icons each [`Icon`] is, and the file its outline is kept in.

use super::Icon;

/// Every icon, in the order of the enumeration.
pub(super) const ALL: [Icon; 92] = [
    Icon::ChevronRight,
    Icon::ChevronDown,
    Icon::ChevronUp,
    Icon::ChevronsUpDown,
    Icon::Close,
    Icon::Check,
    Icon::CheckAll,
    Icon::Plus,
    Icon::Minus,
    Icon::Undo,
    Icon::Delete,
    Icon::Save,
    Icon::Search,
    Icon::Filter,
    Icon::Eye,
    Icon::EyeOff,
    Icon::Branch,
    Icon::CreateBranch,
    Icon::Tag,
    Icon::Cloud,
    Icon::Commit,
    Icon::OpenAsPage,
    Icon::Merge,
    Icon::CherryPick,
    Icon::Compare,
    Icon::Swap,
    Icon::PullRequest,
    Icon::DraftPullRequest,
    Icon::Fetch,
    Icon::Pull,
    Icon::Push,
    Icon::Sync,
    Icon::ArrowDown,
    Icon::ArrowUp,
    Icon::Refresh,
    Icon::ToTop,
    Icon::MoveUp,
    Icon::MoveDown,
    Icon::Timeline,
    Icon::FileDiff,
    Icon::FolderTree,
    Icon::Terminal,
    Icon::Activity,
    Icon::Gear,
    Icon::CommandLog,
    Icon::Folder,
    Icon::File,
    Icon::FileText,
    Icon::Files,
    Icon::Worktree,
    Icon::Submodule,
    Icon::Stash,
    Icon::Apply,
    Icon::Pop,
    Icon::Star,
    Icon::Clock,
    Icon::History,
    Icon::Blame,
    Icon::Pin,
    Icon::Sparkles,
    Icon::Wand,
    Icon::Pencil,
    Icon::Hourglass,
    Icon::Skipped,
    Icon::Warning,
    Icon::Error,
    Icon::Info,
    Icon::More,
    Icon::List,
    Icon::ListTree,
    Icon::FoldUnchanged,
    Icon::Stop,
    Icon::External,
    Icon::TakeIncoming,
    Icon::TakeCurrent,
    Icon::Grip,
    Icon::Bisect,
    Icon::NotInitialized,
    Icon::Base,
    Icon::Palette,
    Icon::Flask,
    Icon::Copy,
    Icon::Cut,
    Icon::Paste,
    Icon::Rename,
    Icon::CheckOut,
    Icon::NewWindow,
    Icon::Reveal,
    Icon::Unfold,
    Icon::Sliders,
    Icon::Shield,
    Icon::Users,
];

/// Each icon with its name in Lucide's set and its SVG file as Lucide publishes it, in the order of the enumeration.
const OUTLINES: [(Icon, &str, &str); 92] = [
    (
        Icon::ChevronRight,
        "chevron-right",
        include_str!("../../icons/lucide/chevron-right.svg"),
    ),
    (
        Icon::ChevronDown,
        "chevron-down",
        include_str!("../../icons/lucide/chevron-down.svg"),
    ),
    (
        Icon::ChevronUp,
        "chevron-up",
        include_str!("../../icons/lucide/chevron-up.svg"),
    ),
    (
        Icon::ChevronsUpDown,
        "chevrons-up-down",
        include_str!("../../icons/lucide/chevrons-up-down.svg"),
    ),
    (Icon::Close, "x", include_str!("../../icons/lucide/x.svg")),
    (Icon::Check, "check", include_str!("../../icons/lucide/check.svg")),
    (
        Icon::CheckAll,
        "check-check",
        include_str!("../../icons/lucide/check-check.svg"),
    ),
    (Icon::Plus, "plus", include_str!("../../icons/lucide/plus.svg")),
    (Icon::Minus, "minus", include_str!("../../icons/lucide/minus.svg")),
    (Icon::Undo, "undo-2", include_str!("../../icons/lucide/undo-2.svg")),
    (Icon::Delete, "trash-2", include_str!("../../icons/lucide/trash-2.svg")),
    (Icon::Save, "save", include_str!("../../icons/lucide/save.svg")),
    (Icon::Search, "search", include_str!("../../icons/lucide/search.svg")),
    (Icon::Filter, "funnel", include_str!("../../icons/lucide/funnel.svg")),
    (Icon::Eye, "eye", include_str!("../../icons/lucide/eye.svg")),
    (Icon::EyeOff, "eye-off", include_str!("../../icons/lucide/eye-off.svg")),
    (
        Icon::Branch,
        "git-branch",
        include_str!("../../icons/lucide/git-branch.svg"),
    ),
    (
        Icon::CreateBranch,
        "git-branch-plus",
        include_str!("../../icons/lucide/git-branch-plus.svg"),
    ),
    (Icon::Tag, "tag", include_str!("../../icons/lucide/tag.svg")),
    (Icon::Cloud, "cloud", include_str!("../../icons/lucide/cloud.svg")),
    (
        Icon::Commit,
        "git-commit-horizontal",
        include_str!("../../icons/lucide/git-commit-horizontal.svg"),
    ),
    (
        Icon::OpenAsPage,
        "maximize-2",
        include_str!("../../icons/lucide/maximize-2.svg"),
    ),
    (
        Icon::Merge,
        "git-merge",
        include_str!("../../icons/lucide/git-merge.svg"),
    ),
    (
        Icon::CherryPick,
        "cherry",
        include_str!("../../icons/lucide/cherry.svg"),
    ),
    (
        Icon::Compare,
        "git-compare-arrows",
        include_str!("../../icons/lucide/git-compare-arrows.svg"),
    ),
    (
        Icon::Swap,
        "arrow-left-right",
        include_str!("../../icons/lucide/arrow-left-right.svg"),
    ),
    (
        Icon::PullRequest,
        "git-pull-request",
        include_str!("../../icons/lucide/git-pull-request.svg"),
    ),
    (
        Icon::DraftPullRequest,
        "git-pull-request-draft",
        include_str!("../../icons/lucide/git-pull-request-draft.svg"),
    ),
    (
        Icon::Fetch,
        "cloud-download",
        include_str!("../../icons/lucide/cloud-download.svg"),
    ),
    (
        Icon::Pull,
        "arrow-down-to-line",
        include_str!("../../icons/lucide/arrow-down-to-line.svg"),
    ),
    (
        Icon::Push,
        "arrow-up-from-line",
        include_str!("../../icons/lucide/arrow-up-from-line.svg"),
    ),
    (
        Icon::Sync,
        "arrow-down-up",
        include_str!("../../icons/lucide/arrow-down-up.svg"),
    ),
    (
        Icon::ArrowDown,
        "arrow-down",
        include_str!("../../icons/lucide/arrow-down.svg"),
    ),
    (
        Icon::ArrowUp,
        "arrow-up",
        include_str!("../../icons/lucide/arrow-up.svg"),
    ),
    (
        Icon::Refresh,
        "refresh-cw",
        include_str!("../../icons/lucide/refresh-cw.svg"),
    ),
    (
        Icon::ToTop,
        "arrow-up-to-line",
        include_str!("../../icons/lucide/arrow-up-to-line.svg"),
    ),
    (Icon::MoveUp, "move-up", include_str!("../../icons/lucide/move-up.svg")),
    (
        Icon::MoveDown,
        "move-down",
        include_str!("../../icons/lucide/move-down.svg"),
    ),
    (
        Icon::Timeline,
        "git-graph",
        include_str!("../../icons/lucide/git-graph.svg"),
    ),
    (
        Icon::FileDiff,
        "file-diff",
        include_str!("../../icons/lucide/file-diff.svg"),
    ),
    (
        Icon::FolderTree,
        "folder-tree",
        include_str!("../../icons/lucide/folder-tree.svg"),
    ),
    (
        Icon::Terminal,
        "square-terminal",
        include_str!("../../icons/lucide/square-terminal.svg"),
    ),
    (
        Icon::Activity,
        "activity",
        include_str!("../../icons/lucide/activity.svg"),
    ),
    (Icon::Gear, "settings", include_str!("../../icons/lucide/settings.svg")),
    (
        Icon::CommandLog,
        "scroll-text",
        include_str!("../../icons/lucide/scroll-text.svg"),
    ),
    (Icon::Folder, "folder", include_str!("../../icons/lucide/folder.svg")),
    (Icon::File, "file", include_str!("../../icons/lucide/file.svg")),
    (
        Icon::FileText,
        "file-text",
        include_str!("../../icons/lucide/file-text.svg"),
    ),
    (Icon::Files, "files", include_str!("../../icons/lucide/files.svg")),
    (
        Icon::Worktree,
        "folder-git-2",
        include_str!("../../icons/lucide/folder-git-2.svg"),
    ),
    (Icon::Submodule, "boxes", include_str!("../../icons/lucide/boxes.svg")),
    (Icon::Stash, "archive", include_str!("../../icons/lucide/archive.svg")),
    (
        Icon::Apply,
        "archive-restore",
        include_str!("../../icons/lucide/archive-restore.svg"),
    ),
    (
        Icon::Pop,
        "package-open",
        include_str!("../../icons/lucide/package-open.svg"),
    ),
    (Icon::Star, "star", include_str!("../../icons/lucide/star.svg")),
    (Icon::Clock, "clock", include_str!("../../icons/lucide/clock.svg")),
    (Icon::History, "history", include_str!("../../icons/lucide/history.svg")),
    (Icon::Blame, "user-pen", include_str!("../../icons/lucide/user-pen.svg")),
    (Icon::Pin, "pin", include_str!("../../icons/lucide/pin.svg")),
    (
        Icon::Sparkles,
        "sparkles",
        include_str!("../../icons/lucide/sparkles.svg"),
    ),
    (
        Icon::Wand,
        "wand-sparkles",
        include_str!("../../icons/lucide/wand-sparkles.svg"),
    ),
    (Icon::Pencil, "pencil", include_str!("../../icons/lucide/pencil.svg")),
    (
        Icon::Hourglass,
        "hourglass",
        include_str!("../../icons/lucide/hourglass.svg"),
    ),
    (
        Icon::Skipped,
        "circle-slash",
        include_str!("../../icons/lucide/circle-slash.svg"),
    ),
    (
        Icon::Warning,
        "triangle-alert",
        include_str!("../../icons/lucide/triangle-alert.svg"),
    ),
    (Icon::Error, "circle-x", include_str!("../../icons/lucide/circle-x.svg")),
    (Icon::Info, "info", include_str!("../../icons/lucide/info.svg")),
    (Icon::More, "ellipsis", include_str!("../../icons/lucide/ellipsis.svg")),
    (Icon::List, "list", include_str!("../../icons/lucide/list.svg")),
    (
        Icon::ListTree,
        "list-tree",
        include_str!("../../icons/lucide/list-tree.svg"),
    ),
    (
        Icon::FoldUnchanged,
        "fold-vertical",
        include_str!("../../icons/lucide/fold-vertical.svg"),
    ),
    (
        Icon::Stop,
        "circle-stop",
        include_str!("../../icons/lucide/circle-stop.svg"),
    ),
    (
        Icon::External,
        "external-link",
        include_str!("../../icons/lucide/external-link.svg"),
    ),
    (
        Icon::TakeIncoming,
        "circle-arrow-down",
        include_str!("../../icons/lucide/circle-arrow-down.svg"),
    ),
    (
        Icon::TakeCurrent,
        "circle-arrow-up",
        include_str!("../../icons/lucide/circle-arrow-up.svg"),
    ),
    (
        Icon::Grip,
        "grip-vertical",
        include_str!("../../icons/lucide/grip-vertical.svg"),
    ),
    (
        Icon::Bisect,
        "binoculars",
        include_str!("../../icons/lucide/binoculars.svg"),
    ),
    (
        Icon::NotInitialized,
        "circle-dashed",
        include_str!("../../icons/lucide/circle-dashed.svg"),
    ),
    (Icon::Base, "git-fork", include_str!("../../icons/lucide/git-fork.svg")),
    (Icon::Palette, "palette", include_str!("../../icons/lucide/palette.svg")),
    (
        Icon::Flask,
        "flask-conical",
        include_str!("../../icons/lucide/flask-conical.svg"),
    ),
    (Icon::Copy, "copy", include_str!("../../icons/lucide/copy.svg")),
    (Icon::Cut, "scissors", include_str!("../../icons/lucide/scissors.svg")),
    (
        Icon::Paste,
        "clipboard-paste",
        include_str!("../../icons/lucide/clipboard-paste.svg"),
    ),
    (
        Icon::Rename,
        "pen-line",
        include_str!("../../icons/lucide/pen-line.svg"),
    ),
    (Icon::CheckOut, "log-in", include_str!("../../icons/lucide/log-in.svg")),
    (
        Icon::NewWindow,
        "app-window",
        include_str!("../../icons/lucide/app-window.svg"),
    ),
    (
        Icon::Reveal,
        "folder-open",
        include_str!("../../icons/lucide/folder-open.svg"),
    ),
    (
        Icon::Unfold,
        "unfold-vertical",
        include_str!("../../icons/lucide/unfold-vertical.svg"),
    ),
    (
        Icon::Sliders,
        "sliders-horizontal",
        include_str!("../../icons/lucide/sliders-horizontal.svg"),
    ),
    (Icon::Shield, "shield", include_str!("../../icons/lucide/shield.svg")),
    (Icon::Users, "users", include_str!("../../icons/lucide/users.svg")),
];

/// The place of an icon in [`OUTLINES`], which is that of its variant.
fn outline(icon: Icon) -> Option<&'static (Icon, &'static str, &'static str)> {
    OUTLINES.get(icon as usize).filter(|(listed, _, _)| *listed == icon)
}

/// The name of an icon in Lucide's set, which is also the name of the file of its outline.
pub(super) fn name(icon: Icon) -> &'static str {
    outline(icon).map_or("", |(_, name, _)| name)
}

/// The SVG file of an icon, as Lucide publishes it.
pub(super) fn source(icon: Icon) -> &'static str {
    outline(icon).map_or("", |(_, _, source)| source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_its_outline_at_its_place() {
        for icon in ALL {
            assert!(!name(icon).is_empty() && !source(icon).is_empty(), "{icon:?}");
        }
    }
}
