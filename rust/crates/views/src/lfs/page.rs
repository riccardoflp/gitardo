//! The Large files page (`lfs.page`): whether Git LFS is there and set up, the patterns, and the files with their sizes,
//! whether they are downloaded and who locked them, each with its menu; rows one under the other in a scroll area,
//! those in sight drawn.

use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use model::lfs::{LfsDownload, LfsFile, LfsLock, LfsLocks, LfsPath, LfsStatus};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Label, MenuEntry, Rect, Sense, TextKind, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::file_view::{FileTarget, FileViewIntent};
use crate::icons;
use crate::intent::Intent;
use crate::text::lfs as words;

/// The widest the page is drawn.
const PAGE_WIDTH: f32 = 960.0;
/// The width of the column of the sizes.
const SIZE_WIDTH: f32 = 120.0;
/// The width of the column of whether a file is downloaded.
const STATE_WIDTH: f32 = 160.0;
/// The width of the column of who locked a file.
const OWNER_WIDTH: f32 = 140.0;
/// The answer of `lfs.untrack` that names the pattern.
const PATTERN: AnswerKey = AnswerKey("pattern");

/// What the page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LfsPageState<'a> {
    /// Whether Git LFS is installed, set up, and the patterns; none before the first read.
    pub status: Option<&'a LfsStatus>,
    /// The files stored with Git LFS in the commit checked out.
    pub files: &'a [LfsFile],
    /// Whether a read is under way.
    pub is_reading: bool,
    /// What git said when the read failed.
    pub failed: Option<&'a str>,
    /// The locks of the server, or that it has none; none before they are read.
    pub locks: Option<&'a LfsLocks>,
    /// Whether the server is being asked for the locks.
    pub is_reading_locks: bool,
    /// What git said when the locks could not be read.
    pub locks_failed: Option<&'a str>,
    /// The remote Git LFS asks for the locks.
    pub remote: Option<&'a str>,
}

/// Something asked of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LargeFilesIntent {
    /// Open the page for the repository shown.
    Open,
    /// **Large files…** of the menu of the row of a repository: show it, then open the page.
    OpenForTab(usize),
    /// **Refresh**: read the files and the locks again.
    Refresh,
    /// **Lock** or **Unlock** was chosen: read the locks again once the operation ended.
    ReadLocksAfterOperation,
}

/// A row of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    /// **Git LFS `<version>`**, or **Git LFS is not installed** with **Open git-lfs.com**.
    Head,
    /// The repository has patterns and is not set up, with **Set up**.
    NotSetUp,
    /// A row left empty between two parts.
    Gap,
    /// The repository stores nothing with Git LFS, with **Track a pattern…**.
    Nothing,
    /// **Patterns** with **Track a pattern…**.
    PatternsTitle,
    /// A pattern, by its place.
    Pattern(usize),
    /// **Files** with their count and size, **Download all** and **Refresh**.
    FilesTitle,
    /// What the page says of the read: reading, or what git said.
    Note,
    /// What the page says of the locks: reading them, a server without them, or what git said.
    LocksNote,
    /// A file, by its place.
    File(usize),
}

/// The choices of the menu of a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PatternChoice {
    /// **Stop storing with Git LFS**.
    Untrack,
    /// **Store them with Git LFS in the next commit**.
    StoreCommitted,
    /// **Open `<file>`**.
    Open,
}

/// The choices of the menu of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileChoice {
    /// **Download**.
    Download,
    /// **Lock**.
    Lock,
    /// **Unlock**.
    Unlock,
    /// **Open file**.
    Open,
    /// **Show in the Files page**.
    ShowInFiles,
}

/// The Large files page.
///
/// Feature: lfs.page
pub fn page(state: &LfsPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let clip = ui.current_clip();
    let spacing = ui.theme().sizes.spacing;
    let area = inset(
        Rect::new(clip.x, clip.y, PAGE_WIDTH.min(clip.width), clip.height),
        Insets::all(spacing.large + spacing.small),
    );
    let rows = rows(state);
    let height = ui.theme().sizes.control_height + spacing.small;
    let content = (area.width, height * count(rows.len()));
    let mut intents = Vec::new();
    ui.region("large files", (area, AccessRole::Region, words::PAGE), |ui| {
        let (_, drawn) = ui.scroll_area("large files rows", area, content, |ui, inside| {
            let mut intents = Vec::new();
            for (index, row) in rows.iter().enumerate() {
                let top = inside.y + height * count(index);
                if top + height < area.y || top > area.bottom() {
                    continue;
                }
                let gutter = ui.theme().sizes.spacing.large * 1.5;
                let rect = Rect::new(inside.x, top, (inside.width - gutter).max(0.0), height);
                intents.extend(draw_row(state, *row, ui, rect));
            }
            intents
        });
        intents = drawn;
    });
    intents
}

/// A count of rows as a length.
fn count(rows: usize) -> f32 {
    f32::from(u16::try_from(rows).unwrap_or(u16::MAX))
}

/// The rows of the page, in order.
fn rows(state: &LfsPageState<'_>) -> Vec<Row> {
    let mut rows = vec![Row::Head];
    let Some(status) = state.status else {
        rows.push(Row::Note);
        return rows;
    };
    if status.version.is_some() && !status.is_set_up && !status.patterns.is_empty() {
        rows.push(Row::NotSetUp);
    }
    rows.push(Row::Gap);
    if status.patterns.is_empty() && state.files.is_empty() && state.failed.is_none() {
        rows.push(Row::Nothing);
        return rows;
    }
    rows.push(Row::PatternsTitle);
    rows.extend((0..status.patterns.len()).map(Row::Pattern));
    rows.extend([Row::Gap, Row::FilesTitle]);
    if state.failed.is_some() || (state.is_reading && state.files.is_empty()) {
        rows.push(Row::Note);
    }
    if locks_note(state).is_some() {
        rows.push(Row::LocksNote);
    }
    rows.extend((0..state.files.len()).map(Row::File));
    rows
}

/// One row.
fn draw_row(state: &LfsPageState<'_>, row: Row, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    match row {
        Row::Head => head(state, ui, rect),
        Row::NotSetUp => message_with_button(ui, rect, (words::NOT_SET_UP, words::SET_UP), run("lfs.set-up")),
        Row::Nothing => message_with_button(ui, rect, (words::NOTHING_STORED, words::TRACK), run("lfs.track")),
        Row::PatternsTitle => title_with_button(ui, rect, words::PATTERNS, words::TRACK, run("lfs.track")),
        Row::Pattern(index) => state
            .status
            .and_then(|status| status.patterns.get(index))
            .and_then(|pattern| pattern_row(ui, rect, (index, &pattern.pattern, &pattern.file)))
            .into_iter()
            .collect(),
        Row::FilesTitle => files_title(state, ui, rect),
        Row::Note => {
            let (said, tone) = match state.failed {
                Some(failed) => (failed, Tone::Error),
                None => (words::READING, Tone::Muted),
            };
            let _ = ui.label(rect, &Label::new(said).tone(tone));
            Vec::new()
        }
        Row::LocksNote => {
            if let Some((said, tone)) = locks_note(state) {
                let _ = ui.label(rect, &Label::new(&said).tone(tone));
            }
            Vec::new()
        }
        Row::File(index) => state
            .files
            .get(index)
            .map(|file| file_row(state, ui, rect, (index, file)))
            .unwrap_or_default(),
        Row::Gap => Vec::new(),
    }
}

/// An operation run on what is checked out.
fn run(feature: &'static str) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head))
}

/// A button at the right of a row, drawn; whether it was clicked.
fn button_right(ui: &mut Ui<'_>, parts: &mut Area, (key, button): (&str, &Button<'_>)) -> bool {
    let (width, height) = ui.button_size(button);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    ui.button(key, cell, button).clicked
}

/// The head: the version of Git LFS, or that it is not installed with **Open git-lfs.com**.
fn head(state: &LfsPageState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let version = state.status.and_then(|status| status.version.as_deref());
    let said = match (state.status, version) {
        (_, Some(version)) => words::version(version),
        (Some(_), None) => String::from(words::NOT_INSTALLED),
        (None, None) => String::from(words::PAGE),
    };
    let mut intents = Vec::new();
    if state.status.is_some() && version.is_none() {
        let button = Button::secondary(words::OPEN_SITE).with_icon(icons::OPEN_EXTERNAL);
        if button_right(ui, &mut parts, ("open site", &button)) {
            intents.push(Intent::OpenUrl(String::from(words::SITE)));
        }
    }
    let _ = ui.label(parts.rest(), &Label::new(&said).kind(TextKind::Heading));
    intents
}

/// A message with a button after it.
fn message_with_button(ui: &mut Ui<'_>, rect: Rect, (said, label): (&str, &str), intent: Intent) -> Vec<Intent> {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let button = Button::secondary(label).kind(ButtonKind::Outline);
    let width = ui.measure(said, ui.text_style(TextKind::Body));
    let _ = ui.label(parts.left(width), &Label::new(said));
    let (button_width, height) = ui.button_size(&button);
    let cell = place(
        parts.left(button_width),
        (button_width, height),
        (Align::Start, Align::Center),
    );
    if ui.button(("message button", label), cell, &button).clicked {
        return vec![intent];
    }
    Vec::new()
}

/// The title of a part with a button at its right.
fn title_with_button(ui: &mut Ui<'_>, rect: Rect, title: &str, label: &str, intent: Intent) -> Vec<Intent> {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let button = Button::secondary(label).with_icon(icons::ADD);
    let clicked = button_right(ui, &mut parts, (label, &button));
    let _ = ui.label(parts.rest(), &Label::new(title).kind(TextKind::Strong));
    if clicked { vec![intent] } else { Vec::new() }
}

/// **Files** with their count and size, **Download all** with those not downloaded, and **Refresh**.
fn files_title(state: &LfsPageState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.medium);
    let mut intents = Vec::new();
    let refresh = Button::secondary(words::REFRESH).with_icon(icons::READ_AGAIN);
    let refresh = if state.is_reading { refresh.disabled() } else { refresh };
    if button_right(ui, &mut parts, ("refresh", &refresh)) {
        intents.push(Intent::LargeFiles(LargeFilesIntent::Refresh));
    }
    let missing: Vec<&LfsFile> = state.files.iter().filter(|file| !is_downloaded(file)).collect();
    let size = missing.iter().map(|file| file.size).sum();
    let label = words::download_all(missing.len(), size);
    let can_download = !missing.is_empty() && state.status.is_some_and(|status| status.version.is_some());
    let download = Button::secondary(&label).kind(ButtonKind::Outline);
    let download = if can_download { download } else { download.disabled() };
    if button_right(ui, &mut parts, ("download all", &download)) {
        let every = Target::LfsFiles(Box::default());
        intents.push(Intent::Feature(FeatureIntent::Run(FeatureId("lfs.download"), every)));
    }
    let total = state.files.iter().map(|file| file.size).sum();
    let summary = words::files(state.files.len(), total);
    let width = ui.measure(words::FILES, ui.text_style(TextKind::Strong));
    let _ = ui.label(parts.left(width), &Label::new(words::FILES).kind(TextKind::Strong));
    let _ = ui.label(parts.rest(), &Label::new(&summary).tone(Tone::Muted));
    intents
}

/// What the page says of the locks: **Reading the locks…** while none were read yet, that the server has none, or
/// what git said; none once they are read.
fn locks_note(state: &LfsPageState<'_>) -> Option<(String, Tone)> {
    if let Some(failed) = state.locks_failed {
        return Some((words::locks_failed(failed), Tone::Error));
    }
    match state.locks {
        Some(LfsLocks::Unsupported(_)) => Some((words::no_locks(state.remote), Tone::Muted)),
        None if state.is_reading_locks => Some((String::from(words::READING_LOCKS), Tone::Muted)),
        _ => None,
    }
}

/// The lock of a file, when the server has one.
fn lock_of<'a>(state: &LfsPageState<'a>, file: &LfsFile) -> Option<&'a LfsLock> {
    match state.locks? {
        LfsLocks::Read(locks) => locks.iter().find(|lock| lock.path == file.path),
        LfsLocks::Unsupported(_) => None,
    }
}

/// Whether a file not locked can be locked: Git LFS is there and the server answered with its locks.
fn can_lock(state: &LfsPageState<'_>) -> bool {
    state.status.is_some_and(|status| status.version.is_some()) && matches!(state.locks, Some(LfsLocks::Read(_)))
}

/// Whether the content of a file is on this computer and in the working tree.
fn is_downloaded(file: &LfsFile) -> bool {
    file.is_downloaded && file.is_checked_out
}

/// A pattern, with its file of attributes at the right, and its menu.
fn pattern_row(ui: &mut Ui<'_>, rect: Rect, (index, pattern, file): (usize, &str, &str)) -> Option<Intent> {
    let response = ui.interact(ui.id(("pattern", index)), rect, Sense::CLICK | Sense::HOVER);
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.large);
    let file_width = ui.measure(file, ui.text_style(TextKind::Body));
    let _ = ui.label(parts.right(file_width), &Label::new(file).tone(Tone::Muted));
    let _ = ui.label(parts.rest(), &Label::new(pattern));
    let open = words::open_attributes(file);
    let mut entries = Vec::new();
    if file == ".gitattributes" {
        entries.push(MenuEntry::Item(icons::action(
            icons::DELETE,
            words::STOP_STORING,
            PatternChoice::Untrack,
        )));
        entries.push(MenuEntry::Item(icons::action(
            icons::ADD,
            words::STORE_COMMITTED,
            PatternChoice::StoreCommitted,
        )));
        entries.push(MenuEntry::Separator);
    }
    entries.push(MenuEntry::Item(icons::action(
        icons::OPEN_FILE,
        &open,
        PatternChoice::Open,
    )));
    match ui.context_menu(("pattern menu", index), &response, &entries)? {
        PatternChoice::Untrack => {
            let answers = Answers::new().with(PATTERN, Answer::Text(pattern.to_owned()));
            Some(Intent::Feature(FeatureIntent::RunWith(
                FeatureId("lfs.untrack"),
                Target::Head,
                answers,
            )))
        }
        PatternChoice::StoreCommitted => Some(run("lfs.store-committed")),
        PatternChoice::Open => Some(open_file(file)),
    }
}

/// The intent that opens the File page of a file of the working tree.
fn open_file(path: &str) -> Intent {
    Intent::FileView(FileViewIntent::Open(FileTarget::working_tree(path)))
}

/// A file, with whether it is downloaded, its size and who locked it, and its menu.
fn file_row(state: &LfsPageState<'_>, ui: &mut Ui<'_>, rect: Rect, (index, file): (usize, &LfsFile)) -> Vec<Intent> {
    let response = ui.interact(ui.id(("file", index)), rect, Sense::CLICK | Sense::HOVER);
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.large);
    let (downloaded, tone) = if is_downloaded(file) {
        (words::DOWNLOADED, Tone::Muted)
    } else {
        (words::NOT_DOWNLOADED, Tone::Warning)
    };
    let _ = ui.label(parts.right(STATE_WIDTH), &Label::new(downloaded).tone(tone));
    let size = crate::text::worktree_setup::bytes(file.size);
    let _ = ui.label(parts.right(SIZE_WIDTH), &Label::new(&size).tone(Tone::Muted));
    let lock = lock_of(state, file);
    let owner_cell = parts.right(OWNER_WIDTH);
    if let Some(lock) = lock {
        let owner = if lock.is_ours == Some(true) {
            words::YOU
        } else {
            lock.owner.as_str()
        };
        let _ = ui.label(owner_cell, &Label::new(owner).tone(Tone::Muted));
    }
    let theme = *ui.theme();
    let last = file.path.rsplit('/').next().unwrap_or_default();
    ui.file_icon_in(parts.left(theme.sizes.icon), ui::FileOrFolder::File(last), Tone::Muted);
    let _ = ui.label(parts.rest(), &Label::new(&file.path));
    let entries = file_entries(file, lock.is_some(), can_lock(state));
    let Some(choice) = ui.context_menu(("file menu", index), &response, &entries) else {
        return Vec::new();
    };
    file_choice(choice, file, lock)
}

/// The menu of a file: **Download** when it is not, **Lock** or **Unlock**, **Open file**, **Show in the Files page**.
fn file_entries(file: &LfsFile, is_locked: bool, can_lock: bool) -> Vec<MenuEntry<'static, FileChoice>> {
    let mut entries = Vec::new();
    if !is_downloaded(file) {
        entries.push(MenuEntry::Item(icons::action(
            icons::PULL,
            words::DOWNLOAD,
            FileChoice::Download,
        )));
    }
    if is_locked {
        entries.push(MenuEntry::Item(icons::without_icon(words::UNLOCK, FileChoice::Unlock)));
    } else if can_lock {
        entries.push(MenuEntry::Item(icons::without_icon(words::LOCK, FileChoice::Lock)));
    }
    if !entries.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.push(MenuEntry::Item(icons::action(
        icons::OPEN_FILE,
        words::OPEN_FILE,
        FileChoice::Open,
    )));
    entries.push(MenuEntry::Item(icons::action(
        icons::FILES,
        words::SHOW_IN_FILES,
        FileChoice::ShowInFiles,
    )));
    entries
}

/// What a choice of the menu of a file asks; **Lock** and **Unlock** read the locks again once they ended.
fn file_choice(choice: FileChoice, file: &LfsFile, lock: Option<&LfsLock>) -> Vec<Intent> {
    let operation = |feature: &'static str, target: Target| {
        vec![
            Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)),
            Intent::LargeFiles(LargeFilesIntent::ReadLocksAfterOperation),
        ]
    };
    match choice {
        FileChoice::Download => {
            let download = LfsDownload {
                paths: vec![LfsPath {
                    path: file.path.clone(),
                    is_folder: false,
                }],
                commit: None,
            };
            let target = Target::LfsFiles(Box::new(download));
            vec![Intent::Feature(FeatureIntent::Run(FeatureId("lfs.download"), target))]
        }
        FileChoice::Lock => operation("lfs.lock", Target::File(file.path.clone())),
        FileChoice::Unlock => lock
            .map(|lock| operation("lfs.unlock", Target::LfsLock(Box::new(lock.clone()))))
            .unwrap_or_default(),
        FileChoice::Open => vec![open_file(&file.path)],
        FileChoice::ShowInFiles => vec![Intent::Files(crate::files::FilesPageIntent::OpenFile(
            file.path.clone(),
        ))],
    }
}
