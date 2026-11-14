//! The words of the page that chooses what new worktrees get (`worktree-setup.page`).

use model::worktree_setup::{Section, SettingSource, SettingsPlace};

/// The page in the trail: **New Worktrees · `<repository>`**.
#[must_use]
pub fn title(repository: &str) -> String {
    format!("New Worktrees \u{b7} {repository}")
}

/// The page, for the screen reader.
pub const PAGE: &str = "New Worktrees";
/// **Choose What New Worktrees Get…** in Quick search.
pub const COMMAND: &str = "Choose What New Worktrees Get\u{2026}";
/// **Choose what new worktrees get…** in the **…** of the Worktrees list.
pub const MENU_ENTRY: &str = "Choose what new worktrees get\u{2026}";
/// The header.
pub const HEADER: &str = "What new worktrees get";
/// What the header says a new worktree has and gets.
pub const HEADER_DETAIL: &str = "A new worktree has the files git tracks. Of what git ignores here, it gets the folders \
                                 linked and the files and folders copied below, then runs the commands.";
/// **Linked now**.
pub const LINKED_NOW: &str = "Linked now";
/// **Copied now**.
pub const COPIED_NOW: &str = "Copied now";
/// **Run now**.
pub const RUN_NOW: &str = "Run now";

/// Where what applies comes from, for the line of the header of one setting.
#[must_use]
pub fn source(source: SettingSource, repository: &str, vs_code_key: &str) -> String {
    match source {
        SettingSource::Repository => format!("the settings of {repository}"),
        SettingSource::User => String::from("your user settings"),
        SettingSource::VsCode => format!("VS Code's {vs_code_key}"),
        SettingSource::Nothing => String::from("nothing"),
    }
}

/// A line of the header: **Linked now: `node_modules/` · from the settings of `<repository>`**.
#[must_use]
pub fn now(what: &str, values: &[String], from: &str) -> String {
    if values.is_empty() {
        format!("{what}: nothing \u{b7} from {from}")
    } else {
        format!("{what}: {} \u{b7} from {from}", values.join(", "))
    }
}

/// **What git ignores here**, with how many.
#[must_use]
pub fn ignored_here(count: usize) -> String {
    format!("What git ignores here \u{b7} {count}")
}

/// **Use what is proposed**.
pub const USE_PROPOSED: &str = "Use what is proposed";
/// **Look again**.
pub const LOOK_AGAIN: &str = "Look again";
/// **Looking…** while git is asked again.
pub const LOOKING: &str = "Looking\u{2026}";
/// While what git ignores is first read.
pub const LOOKING_FIRST: &str = "Looking at what git ignores\u{2026}";
/// With nothing ignored.
pub const NOTHING_IGNORED: &str = "Git ignores nothing here, so new worktrees have all there is.";
/// After a change, until **Look again**.
pub const CHANGED_SINCE: &str = "Changed since Gitardo last looked: each line shows what is chosen for it, and Look \
                                 again checks what the patterns take as a whole.";
/// When git cannot say what the patterns take, before its message.
pub const COULD_NOT_SAY: &str = "Git could not say what the patterns take:";
/// When git could not list what it ignores, before its message.
pub const COULD_NOT_LIST: &str = "Git could not say what it ignores here:";

/// The columns of the table.
pub const PATH: &str = "Path";
/// The column of the sizes.
pub const SIZE: &str = "Size";
/// The column of the buttons.
pub const GETS: &str = "New worktrees get";

/// The title of a section.
#[must_use]
pub fn section_title(section: Section) -> &'static str {
    match section {
        Section::NotKnown => "Not one Gitardo knows",
        Section::Dependencies => "Dependencies",
        Section::Secrets => "Settings and secrets of this machine",
        Section::Editor => "Settings of the editor",
        Section::BuildOutput => "Build output",
        Section::OtherWorktrees => "Other worktrees",
        Section::Logs => "Logs and files the system leaves",
    }
}

/// Why a section gets what is proposed for it.
#[must_use]
pub fn section_why(section: Section) -> &'static str {
    match section {
        Section::NotKnown => "Nothing is proposed for them",
        Section::Dependencies => "The same in every worktree, as long as the branches agree on them",
        Section::Secrets => "Kept out of commits, and needed in each worktree",
        Section::Editor => "Each worktree starts with the same",
        Section::BuildOutput => "Each worktree makes its own",
        Section::OtherWorktrees => "Not to be inside one another",
        Section::Logs => "No worktree needs them",
    }
}

/// **Link**.
pub const LINK: &str = "Link";
/// **Copy**.
pub const COPY: &str = "Copy";
/// **Nothing**.
pub const NOTHING: &str = "Nothing";
/// The tooltip of the button of what is proposed.
#[must_use]
pub fn proposed(button: &str) -> String {
    format!("{button} (proposed)")
}
/// The tooltip of **Link** on a file.
pub const ONLY_FOLDERS: &str = "Only folders are linked";
/// What a section's lines get, in short: **all linked**.
pub const ALL_LINKED: &str = "all linked";
/// **all copied**.
pub const ALL_COPIED: &str = "all copied";
/// What none of the lines of a section get.
pub const GET_NOTHING: &str = "nothing";
/// **2 linked**.
#[must_use]
pub fn linked(count: usize) -> String {
    format!("{count} linked")
}
/// **2 copied**.
#[must_use]
pub fn copied(count: usize) -> String {
    format!("{count} copied")
}

/// The note of a link in the checkout.
pub const LINK_ALREADY: &str = "A link already, in this checkout";
/// The note of a folder of which a pattern takes only a part.
#[must_use]
pub fn only_part(gets: &str) -> String {
    format!("Only part of it is {gets}, by a pattern below")
}
/// The note of a copy of more than 100 MB.
pub const LARGE_TO_COPY: &str = "Large to copy into each worktree: better linked, or made again";
/// The size of a link.
pub const A_LINK: &str = "a link";
/// **`<n>` places**.
#[must_use]
pub fn places(count: usize) -> String {
    format!("{count} places")
}
/// The note of a name whose places get different things.
pub const NOT_THE_SAME: &str = "Not the same in each place";
/// **Show the other `<n>`**.
#[must_use]
pub fn show_others(count: usize) -> String {
    format!("Show the other {count}")
}

/// **Patterns**.
pub const PATTERNS: &str = "Patterns";
/// **Folders to link**.
pub const FOLDERS_TO_LINK: &str = "Folders to link";
/// What the folders to link are, beside their setting.
pub const FOLDERS_TO_LINK_DETAIL: &str = "worktrees.linkFolders \u{b7} On Windows, junctions.";
/// **Files and folders to copy**.
pub const FILES_TO_COPY: &str = "Files and folders to copy";
/// What the files to copy are, beside their setting.
pub const FILES_TO_COPY_DETAIL: &str = "worktrees.copyFiles \u{b7} Not what is inside a linked folder.";
/// What a link pattern takes: **`<n>` folders**.
#[must_use]
pub fn folders(count: usize) -> String {
    if count == 1 {
        String::from("1 folder")
    } else {
        format!("{count} folders")
    }
}
/// What a copy pattern takes: **`<n>` files or folders**.
#[must_use]
pub fn files_or_folders(count: usize) -> String {
    if count == 1 {
        String::from("1 file or folder")
    } else {
        format!("{count} files or folders")
    }
}
/// The warning of a pattern that takes nothing.
pub const TAKES_NOTHING: &str = "takes nothing here";
/// The button that removes a pattern or a command.
pub const REMOVE: &str = "Remove";
/// **Add**.
pub const ADD: &str = "Add";
/// What the box of the link patterns says before anything is typed.
pub const LINK_PLACEHOLDER: &str = "node_modules/, .venv/";
/// What the box of the copy patterns says before anything is typed.
pub const COPY_PLACEHOLDER: &str = "/.env, *.local";

/// **Commands to run**.
pub const COMMANDS: &str = "Commands to run";
/// What the commands are, beside their setting.
pub const COMMANDS_DETAIL: &str = "worktrees.setupCommands \u{b7} In Tasks, in the worktree, in a trusted folder only.";
/// What the box of the commands says before anything is typed.
pub const COMMAND_PLACEHOLDER: &str = "npm ci";
/// The button that moves a command up.
pub const MOVE_UP: &str = "Move up";
/// The button that moves a command down.
pub const MOVE_DOWN: &str = "Move down";
/// Before the proposals.
pub const PROPOSED_HERE: &str = "Proposed from what is committed here:";

/// **Save in**.
pub const SAVE_IN: &str = "Save in";
/// The repository as the place to save in.
pub const REPOSITORY: &str = "the repository (.gitardo/settings.json, for everyone who works on it)";
/// The user as the place to save in.
pub const USER: &str = "User (for every repository)";
/// The box of VS Code's settings.
pub const ALSO_VS_CODE: &str = "Also in VS Code's git.worktreeSymlinkFolders and git.worktreeIncludeFiles, for the \
                                worktrees VS Code creates";
/// **Save**.
pub const SAVE: &str = "Save";
/// **Discard changes**.
pub const DISCARD: &str = "Discard changes";
/// **Bring into the other worktree**, or **the other `<n>` worktrees**; **the other worktrees** while there is none.
#[must_use]
pub fn bring_into(others: usize) -> String {
    match others {
        0 => String::from("Bring into the other worktrees"),
        1 => String::from("Bring into the other worktree"),
        _ => format!("Bring into the other {others} worktrees"),
    }
}
/// The tooltip of **Bring into…** while there are changes not saved.
pub const SAVE_FIRST: &str = "Save first: the worktrees get what is saved";
/// **Remove from `<place>`**.
#[must_use]
pub fn remove_from(place: SettingsPlace) -> &'static str {
    match place {
        SettingsPlace::Repository => "Remove from the repository",
        SettingsPlace::User => "Remove from User",
    }
}
/// The tooltip of **Remove from**.
pub const REMOVE_FROM_TOOLTIP: &str = "What is set elsewhere applies again";
/// **Edit in settings file**.
pub const EDIT_IN_FILE: &str = "Edit in settings file";

/// A size in bytes, in the unit that reads best: `512 bytes`, `12 KB`, `1.2 GB`, one decimal below 10.
#[must_use]
pub fn bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return if bytes == 1 {
            String::from("1 byte")
        } else {
            format!("{bytes} bytes")
        };
    }
    // Tenths of the unit, in whole numbers, so that no float rounds the size.
    let mut tenths = bytes.saturating_mul(10) / 1024;
    let mut unit = 0;
    while tenths >= 10_240 && unit + 1 < UNITS.len() {
        tenths /= 1024;
        unit += 1;
    }
    let name = UNITS.get(unit).copied().unwrap_or("TB");
    if tenths < 100 {
        format!("{}.{} {name}", tenths / 10, tenths % 10)
    } else {
        format!("{} {name}", tenths / 10)
    }
}

/// A count with a comma between thousands: `1,234`.
#[must_use]
pub fn thousands(count: usize) -> String {
    let digits = count.to_string();
    let mut said = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            said.push(',');
        }
        said.push(digit);
    }
    said
}

/// The size of a folder: **12 MB · 1,234 files**, **over 1.2 GB · 5,000+ files** past what is counted.
#[must_use]
pub fn folder_size(bytes_counted: u64, files: usize, has_more: bool) -> String {
    if has_more {
        return format!("over {} \u{b7} {}+ files", bytes(bytes_counted), thousands(files));
    }
    let files_said = if files == 1 {
        String::from("1 file")
    } else {
        format!("{} files", thousands(files))
    };
    format!("{} \u{b7} {files_said}", bytes(bytes_counted))
}
