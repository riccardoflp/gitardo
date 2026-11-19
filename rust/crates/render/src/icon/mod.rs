//! Icons: [Lucide](https://lucide.dev)'s, from the outlines of its SVG files kept in `icons/lucide/` with its licence
//! (ISC), and the logo of Gitardo, from `icons/gitardo.svg`, the extension's own. Each outline is read into paths the
//! first time its icon is drawn, and each icon is rasterized once for each size and kept (`graph.icons`,
//! `window.brand`).

mod lucide;
mod outline;

use std::collections::HashMap;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::geometry::{PixelRect, Point, Rect, Transform};
use crate::mask::{Mask, Outline, rasterize};
use crate::number;
use crate::path::{FillRule, LineCap, Path};

/// The side icons are drawn at unless said otherwise, in logical pixels.
pub const GRID: f32 = 16.0;

/// The side of the grid Lucide draws its icons on, in the units of its files.
const LUCIDE_GRID: f32 = 24.0;

/// The width of the lines of an icon drawn at [`GRID`], in logical pixels: wider than Lucide's 2 units of 24 would be
/// at that size (1.33), so that the icons keep the weight of the text beside them (Q420). It grows with the size.
const LINE_WIDTH: f32 = 1.5;

/// An icon of the window, each one of [Lucide](https://lucide.dev)'s, named in its doc comment (`graph.icons`), but
/// the logo of Gitardo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Icon {
    /// A chevron pointing right: folded, or a menu that opens to the side. Lucide's `chevron-right`.
    ChevronRight,
    /// A chevron pointing down: unfolded, or the next change or match. Lucide's `chevron-down`.
    ChevronDown,
    /// A chevron pointing up: the previous change or match. Lucide's `chevron-up`.
    ChevronUp,
    /// Two chevrons, up and down: a button that opens a list of choices below it. Lucide's `chevrons-up-down`.
    ChevronsUpDown,
    /// A cross: close, or empty a box. Lucide's `x`.
    Close,
    /// A check mark: chosen, or done. Lucide's `check`.
    Check,
    /// Two check marks: accept every change that needs no choice. Lucide's `check-check`.
    CheckAll,
    /// A plus: add, or stage. Lucide's `plus`.
    Plus,
    /// A minus: unstage. Lucide's `minus`.
    Minus,
    /// An arrow turning back: back to how it was, as discarding or reverting. Lucide's `undo-2`.
    Undo,
    /// A bin: delete. Lucide's `trash-2`.
    Delete,
    /// A floppy disk: save. Lucide's `save`.
    Save,
    /// A magnifying glass: search. Lucide's `search`.
    Search,
    /// A funnel: filter what is shown or listed. Lucide's `funnel`.
    Filter,
    /// An eye: show. Lucide's `eye`.
    Eye,
    /// An eye struck through: hide. Lucide's `eye-off`.
    EyeOff,
    /// A branch leaving a line of commits. Lucide's `git-branch`.
    Branch,
    /// A branch with a plus: create a branch. Lucide's `git-branch-plus`.
    CreateBranch,
    /// A label with a hole: a tag. Lucide's `tag`.
    Tag,
    /// A cloud: a remote. Lucide's `cloud`.
    Cloud,
    /// A ring on a line: a commit. Lucide's `git-commit-horizontal`.
    Commit,
    /// Two arrows to opposite corners: open as a page. Lucide's `maximize-2`.
    OpenAsPage,
    /// Two lines joining: merge. Lucide's `git-merge`.
    Merge,
    /// Two cherries: cherry-pick. Lucide's `cherry`.
    CherryPick,
    /// Two commits with arrows between them: compare. Lucide's `git-compare-arrows`.
    Compare,
    /// Arrows to the left and to the right: swap the two sides. Lucide's `arrow-left-right`.
    Swap,
    /// A branch coming back to another with an arrow: a pull request. Lucide's `git-pull-request`.
    PullRequest,
    /// A pull request whose second branch is dotted: a draft. Lucide's `git-pull-request-draft`.
    DraftPullRequest,
    /// A cloud with an arrow down: fetch. Lucide's `cloud-download`.
    Fetch,
    /// An arrow down to a line: pull. Lucide's `arrow-down-to-line`.
    Pull,
    /// An arrow up from a line: push. Lucide's `arrow-up-from-line`.
    Push,
    /// An arrow down beside an arrow up: sync. Lucide's `arrow-down-up`.
    Sync,
    /// An arrow down: commits to pull, beside their count. Lucide's `arrow-down`.
    ArrowDown,
    /// An arrow up: commits to push, beside their count. Lucide's `arrow-up`.
    ArrowUp,
    /// Two arrows in a circle: read again. Lucide's `refresh-cw`.
    Refresh,
    /// An arrow up to a line: back to the top. Lucide's `arrow-up-to-line`.
    ToTop,
    /// An arrow up with its tail: move up in a list. Lucide's `move-up`.
    MoveUp,
    /// An arrow down with its tail: move down in a list. Lucide's `move-down`.
    MoveDown,
    /// Commits on two lanes: the timeline. Lucide's `git-graph`.
    Timeline,
    /// A file with a plus and a minus: changes. Lucide's `file-diff`.
    FileDiff,
    /// Folders on a tree: the files. Lucide's `folder-tree`.
    FolderTree,
    /// A prompt in a square: the terminal. Lucide's `square-terminal`.
    Terminal,
    /// A line with a beat in it: what is running. Lucide's `activity`.
    Activity,
    /// A gear: settings. Lucide's `settings`.
    Gear,
    /// A scroll with lines of text: the command log. Lucide's `scroll-text`.
    CommandLog,
    /// A folder. Lucide's `folder`.
    Folder,
    /// A file with a folded corner. Lucide's `file`.
    File,
    /// A file with lines of text: open the file. Lucide's `file-text`.
    FileText,
    /// Two files: open all the changes. Lucide's `files`.
    Files,
    /// A folder with a branch on it: a worktree. Lucide's `folder-git-2`.
    Worktree,
    /// Boxes stacked: a submodule. Lucide's `boxes`.
    Submodule,
    /// A box with a lid: a stash. Lucide's `archive`.
    Stash,
    /// A box with an arrow out of it: apply a stash. Lucide's `archive-restore`.
    Apply,
    /// An open box: pop a stash. Lucide's `package-open`.
    Pop,
    /// A star with five points: a favorite. Lucide's `star`.
    Star,
    /// A clock: recent. Lucide's `clock`.
    Clock,
    /// A clock with an arrow turning back: the history of a file or a folder. Lucide's `history`.
    History,
    /// A person with a pen: who changed each line. Lucide's `user-pen`.
    Blame,
    /// A pin: kept on every file. Lucide's `pin`.
    Pin,
    /// Sparkles: drafted by the AI, or proposed for a conflict. Lucide's `sparkles`.
    Sparkles,
    /// A wand with sparkles: resolved by the rules. Lucide's `wand-sparkles`.
    Wand,
    /// A pencil: edited, not saved or not committed. Lucide's `pencil`.
    Pencil,
    /// An hourglass: waits. Lucide's `hourglass`.
    Hourglass,
    /// A circle struck through: did not run, or skipped. Lucide's `circle-slash`.
    Skipped,
    /// A triangle with an exclamation mark: a warning. Lucide's `triangle-alert`.
    Warning,
    /// A circle with a cross: an error, or a bad commit of a bisect. Lucide's `circle-x`.
    Error,
    /// A circle with an `i`: information. Lucide's `info`.
    Info,
    /// Three dots: more actions. Lucide's `ellipsis`.
    More,
    /// Lines one under the other: a list. Lucide's `list`.
    List,
    /// Lines indented under each other: a tree of folders. Lucide's `list-tree`.
    ListTree,
    /// Arrows toward a line: fold the lines that did not change. Lucide's `fold-vertical`.
    FoldUnchanged,
    /// A square in a circle: stop what is running. Lucide's `circle-stop`.
    Stop,
    /// An arrow out of a square: open in another program. Lucide's `external-link`.
    External,
    /// An arrow down in a circle: take the incoming version. Lucide's `circle-arrow-down`.
    TakeIncoming,
    /// An arrow up in a circle: keep the current version. Lucide's `circle-arrow-up`.
    TakeCurrent,
    /// Two columns of dots: drag to move. Lucide's `grip-vertical`.
    Grip,
    /// Binoculars: a bisect looking for the commit that broke something. Lucide's `binoculars`.
    Bisect,
    /// A dashed circle: a submodule not initialized. Lucide's `circle-dashed`.
    NotInitialized,
    /// A line that forks in two: the base of a merge. Lucide's `git-fork`.
    Base,
    /// A palette: the look of the window, the section **Appearance** of the settings. Lucide's `palette`.
    Palette,
    /// A flask: experimental, the list of the experimental settings. Lucide's `flask-conical`.
    Flask,
    /// Two sheets one over the other: copy. Lucide's `copy`.
    Copy,
    /// Scissors: cut. Lucide's `scissors`.
    Cut,
    /// A clipboard with an arrow: paste. Lucide's `clipboard-paste`.
    Paste,
    /// A pen over a line: rename, or edit the message of a commit. Lucide's `pen-line`.
    Rename,
    /// An arrow going in through a door: check out. Lucide's `log-in`.
    CheckOut,
    /// A window with its bar: open in a new window. Lucide's `app-window`.
    NewWindow,
    /// An open folder: reveal in the file manager of the system. Lucide's `folder-open`.
    Reveal,
    /// Arrows going away from a line: unfold, the commits a merge brought in shown again. Lucide's `unfold-vertical`.
    Unfold,
    /// Three sliders on their rails: how the timeline is shown, the menu of the view of the header. Lucide's
    /// `sliders-horizontal`.
    Sliders,
    /// A padlock: a worktree git keeps whatever happens to its folder (`worktree.locked`). Lucide's `lock`.
    Lock,
    /// The logo of Gitardo: a branch leaving a line of two commits, as the extension draws it (`window.brand`). Not one
    /// of Lucide's, so not in [`Icon::ALL`].
    Logo,
}

/// The outline of the logo of Gitardo, a copy of the extension's `media/gitardo.svg`, drawn on the same grid of 24 as
/// Lucide's icons.
const LOGO: &str = include_str!("../../icons/gitardo.svg");

impl Icon {
    /// Every icon of Lucide's, in the order of the enumeration: all but the logo.
    pub const ALL: [Icon; lucide::ALL.len()] = lucide::ALL;

    /// The name of the icon in Lucide's set, such as `git-branch`; nothing for the logo, which is not Lucide's.
    #[must_use]
    pub fn lucide_name(self) -> &'static str {
        lucide::name(self)
    }
}

/// The paths of an icon on Lucide's grid: lines to stroke, and shapes to fill.
#[derive(Clone, Debug, Default)]
pub(crate) struct Design {
    /// The lines, stroked with round ends and round joins.
    pub(crate) strokes: Path,
    /// The shapes filled too, such as the dot of the tag.
    pub(crate) fills: Path,
}

/// An icon at a size in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct IconKey {
    /// The icon.
    icon: Icon,
    /// Its side, in 16ths of a physical pixel.
    size: u32,
}

/// The icons read and rasterized so far, each over pixels counted from its top left corner.
#[derive(Clone, Debug, Default)]
pub struct Icons {
    /// The paths of each icon read so far.
    designs: HashMap<Icon, Design>,
    /// The coverage of each icon at each size.
    masks: HashMap<IconKey, Mask>,
}

impl Icons {
    /// The coverage of an icon at a size in physical pixels, rasterized the first time it is asked for.
    fn mask(&mut self, icon: Icon, size: f32) -> &Mask {
        let key = IconKey {
            icon,
            size: number::key(size * 16.0),
        };
        let design = self.designs.entry(icon).or_insert_with(|| design(icon));
        self.masks.entry(key).or_insert_with(|| rasterize_icon(design, size))
    }
}

/// The paths of an icon, read from its outline. The outlines are part of the program and a test reads every one of
/// them, so an outline that cannot be read is a mistake of the build: the icon is left empty rather than stopping.
fn design(icon: Icon) -> Design {
    outline::outline(source(icon)).unwrap_or_default()
}

/// The SVG file of an icon: Lucide's, or the logo's.
fn source(icon: Icon) -> &'static str {
    if icon == Icon::Logo { LOGO } else { lucide::source(icon) }
}

/// The coverage of an icon whose grid is scaled to a side in physical pixels; nothing covered for a size of nothing.
fn rasterize_icon(design: &Design, size: f32) -> Mask {
    let factor = size / LUCIDE_GRID;
    let transform = Transform::scale(factor, factor);
    let strokes = Outline::stroke(
        &design.strokes.transformed(transform),
        LINE_WIDTH * size / GRID,
        LineCap::Round,
    );
    let fills = Outline::fill(&design.fills.transformed(transform), FillRule::NonZero);
    let outlines: Vec<Outline> = strokes.into_iter().chain(fills).collect();
    let pixels = number::ceil(size);
    rasterize(&outlines, PixelRect::new(0, 0, pixels, pixels)).unwrap_or_default()
}

impl Canvas<'_> {
    /// Draws an icon in a color, its top left corner at a point snapped to a whole physical pixel, its side `size`
    /// logical pixels (16 unless said otherwise), its lines as wide as the size asks.
    ///
    /// Feature: graph.icons
    pub fn draw_icon(&mut self, icon: Icon, origin: Point, size: f32, color: Color) {
        let corner = self.to_physical_point(origin);
        let (column, row) = (number::round(corner.x), number::round(corner.y));
        let clip = self.physical_clip();
        let physical_size = size * self.scale();
        let bounds = PixelRect::enclosing(Rect::new(
            number::float(column),
            number::float(row),
            physical_size,
            physical_size,
        ));
        if bounds.intersection(clip).is_empty() {
            return;
        }
        let mask = self.assets.icons.mask(icon, physical_size);
        self.frame.fill_mask(mask.view_at(column, row), color, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_outline_is_read() {
        for icon in Icon::ALL {
            let read = outline::outline(lucide::source(icon));
            assert!(read.is_ok(), "{icon:?}: {read:?}");
            assert!(
                read.is_ok_and(|design| !design.strokes.is_empty()),
                "{icon:?} draws nothing"
            );
        }
    }

    #[test]
    fn every_icon_draws_something_inside_its_square() {
        for icon in Icon::ALL {
            for size in [16.0, 32.0] {
                let mask = rasterize_icon(&design(icon), size);
                let covered = mask.coverage.iter().filter(|amount| **amount > 128).count();
                assert!(covered > 8, "{icon:?} at {size} covers {covered} pixels");
            }
        }
    }

    #[test]
    fn the_logo_is_read_and_draws_something_inside_its_square() {
        let read = outline::outline(LOGO);
        assert!(read.as_ref().is_ok_and(|design| !design.strokes.is_empty()), "{read:?}");
        let mask = rasterize_icon(&design(Icon::Logo), 16.0);
        assert!(mask.coverage.iter().filter(|amount| **amount > 128).count() > 8);
        assert!(!Icon::ALL.contains(&Icon::Logo));
    }

    #[test]
    fn the_logo_is_the_one_of_the_extension() {
        let extension = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../extension/media/gitardo.svg");
        let source = std::fs::read_to_string(extension).unwrap_or_default();
        assert_eq!(source, LOGO, "the copy kept here is the logo of the extension");
    }

    #[test]
    fn every_icon_is_a_different_outline() {
        let mut names: Vec<&str> = Icon::ALL.iter().map(|icon| icon.lucide_name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Icon::ALL.len());
    }
}
