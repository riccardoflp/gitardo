//! The words of the Changes page (`changes.pane`), of its commit box (`commit.create`, `commit.amend`) and of the
//! identity under it (`identity.switch`).

/// The Changes page in its trail, and for the screen reader.
pub const PAGE: &str = "Changes";
/// The button that commits.
pub const COMMIT: &str = "Commit";
/// What **Commit** reads while **Amend last commit** is ticked.
pub const AMEND: &str = "Amend";
/// The box that makes **Commit** amend the last commit.
pub const AMEND_LAST_COMMIT: &str = "Amend last commit";
/// The tooltip of **Amend last commit**.
pub const AMEND_TOOLTIP: &str =
    "Replace the last commit instead of adding a new one. Leave the message empty to keep it.";
/// The tooltip of the sparkle button, which drafts the message with the AI provider (`commit.draft-message`).
pub const DRAFT_MESSAGE: &str = "Draft a message from the changes with the AI provider";
/// The tooltip of the button in place of the sparkle while the message is drafted.
pub const STOP_DRAFT: &str = "Stop drafting the message: the box is left as it was";
/// What the sparkle says with nothing changed.
pub const NOTHING_TO_DESCRIBE: &str = "There are no changes to describe.";
/// The tooltip of what comes in a later milestone.
pub const NOT_YET: &str = "Not yet";
/// **Switch identity…** in the **…** menu of the header (`identity.switch`).
pub const SWITCH_IDENTITY: &str = "Switch identity\u{2026}";
/// The identity line when git has none.
pub const NO_IDENTITY: &str = "No identity configured";
/// The heading of the files in conflict.
pub const CONFLICTS: &str = "Conflicts";
/// The Changes page as a column beside the file it opened, for the screen reader (`changes.beside-file`).
pub const COLUMN: &str = "List of changes";
/// What the **Changes** section of the side column shows, for the screen reader (`changes.section`).
pub const SECTION: &str = "Changes of the repository shown";
/// The heading of the files changed.
pub const CHANGES: &str = "Changes";
/// The box on the header of the list, for the screen reader.
pub const ALL_FILES: &str = "All files";
/// **View as tree**.
pub const VIEW_AS_TREE: &str = "View as tree";
/// **View as list**.
pub const VIEW_AS_LIST: &str = "View as list";
/// **Open all changes in one Diff page**.
pub const OPEN_ALL_CHANGES: &str = "Open all changes in one Diff page";
/// **Stash only the staged changes…**.
pub const STASH_STAGED: &str = "Stash only the staged changes\u{2026}";
/// **Stash all changes…**.
pub const STASH_ALL: &str = "Stash all changes\u{2026}";
/// **Save all**.
pub const SAVE_ALL: &str = "Save all";
/// **Discard all changes**.
pub const DISCARD_ALL: &str = "Discard all changes";
/// **…** at the right of the heading of the list, which has the buttons the heading has no room for.
pub const MORE_FOR_ALL: &str = "More for all changes";
/// **Stage**, in the menu of a file.
pub const STAGE: &str = "Stage";
/// **Unstage**, in the menu of a file wholly staged.
pub const UNSTAGE: &str = "Unstage";
/// **Open file**.
pub const OPEN_FILE: &str = "Open file";
/// **Open changes**.
pub const OPEN_CHANGES: &str = "Open changes";
/// **Save**.
pub const SAVE: &str = "Save";
/// **Stash this file…**.
pub const STASH_FILE: &str = "Stash this file\u{2026}";
/// **Discard changes**.
pub const DISCARD: &str = "Discard changes";
/// **Compare with the saved file**.
pub const COMPARE_WITH_SAVED: &str = "Compare with the saved file";
/// The tooltip of the box of a file only an editor has changed.
pub const NOT_SAVED: &str = "Not saved: save it to stage it";
/// **Mark as resolved**.
pub const MARK_AS_RESOLVED: &str = "Mark as resolved";
/// **Take the current side for the whole file**.
pub const TAKE_CURRENT: &str = "Take the current side for the whole file";
/// **Take the incoming side for the whole file**.
pub const TAKE_INCOMING: &str = "Take the incoming side for the whole file";
/// The wand on a file in conflict, which opens the review of its conflicts (`review.page`).
pub const RESOLVE_AUTOMATICALLY: &str = "Resolve its conflicts automatically, after looking at what is proposed";
/// The sparkle on a file in conflict, with `conflicts.ai` (`ai.resolve-conflicts`).
pub const RESOLVE_WITH_AI: &str = "Resolve with AI, after looking at what is proposed";
/// What the page says when there is nothing in either list.
pub const NO_CHANGES: &str = "No changes";
/// The letter of an untracked file.
pub const UNTRACKED: &str = "?";
/// The letter of a file in conflict.
pub const CONFLICTED: &str = "!";
/// The letter of a file only an editor has changed.
pub const UNSAVED: &str = "\u{25cf}";

/// What the message box shows while it is empty: `Message (Ctrl+Enter to commit on main)`, or `detached HEAD` for the
/// branch.
#[must_use]
pub fn message_placeholder(branch: Option<&str>) -> String {
    format!(
        "Message (Ctrl+Enter to commit on {})",
        branch.unwrap_or("detached HEAD")
    )
}

/// The identity commits are made with: `Ada <ada@example.com>`.
#[must_use]
pub fn identity(name: &str, email: &str) -> String {
    format!("{name} <{email}>")
}

/// The identity commits are made with, from what of it is set: `Ada <ada@example.com>`, the name or the email alone, or
/// **No identity configured**.
#[must_use]
pub fn identity_said(name: Option<&str>, email: Option<&str>) -> String {
    match (name, email) {
        (Some(name), Some(email)) => identity(name, email),
        (Some(name), None) => name.to_owned(),
        (None, Some(email)) => email.to_owned(),
        (None, None) => NO_IDENTITY.to_owned(),
    }
}

/// A heading with its count: `Changes 3`.
#[must_use]
pub fn with_count(heading: &str, count: usize) -> String {
    format!("{heading} {count}")
}

/// The name of a renamed file with its old name beside it: `b.txt ← a.txt`.
#[must_use]
pub fn renamed_from(original: &str) -> String {
    format!("\u{2190} {original}")
}

/// The button beside **Commit** that opens the options of the next commit (`commit.options`).
pub const COMMIT_OPTIONS: &str = "Commit options";
/// The option that signs the next commit.
pub const SIGN_THIS_COMMIT: &str = "Sign this commit";
/// What **Sign this commit** says when git has no key to sign with.
pub const NO_SIGNING_KEY: &str = "No key to sign with: set one with Switch identity\u{2026}";
/// The option that adds `Signed-off-by` to the next commit.
pub const ADD_SIGNED_OFF_BY: &str = "Add Signed-off-by";
/// The entry that opens the picker of the co-authors (`commit.co-authors`).
pub const CO_AUTHORS: &str = "Co-authors\u{2026}";
/// The tooltip of a co-author listed in the menu.
pub const REMOVE_CO_AUTHOR: &str = "Remove from the co-authors";
/// The option that skips the hooks of the next commit.
pub const SKIP_THE_HOOKS: &str = "Skip the hooks";
/// What **Skip the hooks** does.
pub const SKIP_THE_HOOKS_TOOLTIP: &str = "Commit without running the pre-commit and commit-msg hooks";

/// What **Sign this commit** signs with: `With the SSH key ~/.ssh/id_ed25519.pub`, `With the OpenPGP key
/// 3AA5C34371567BD2`, or the key of the email of the identity when none is set.
#[must_use]
pub fn signs_with(signing: &model::commit::SigningConfig) -> String {
    use model::commit::SigningKind;
    let kind = match signing.kind {
        SigningKind::OpenPgp => "OpenPGP",
        SigningKind::X509 => "X.509",
        SigningKind::Ssh => "SSH",
    };
    match signing.key.as_deref() {
        Some(key) => format!("With the {kind} key {key}"),
        None => format!("With the {kind} key of your email"),
    }
}

/// The parts of the line under the commit box, in the order of the menu: whether the commit is signed, `Signed-off-by`,
/// the co-authors and the hooks skipped; each part only while its option differs from where it starts.
#[must_use]
pub fn options_said(parts: &OptionsSaid<'_>) -> String {
    let mut said: Vec<String> = Vec::new();
    if let Some(is_signed) = parts.sign {
        said.push(String::from(if is_signed { "Signed" } else { "Not signed" }));
    }
    if let Some(is_signed_off) = parts.sign_off {
        said.push(String::from(if is_signed_off {
            "Signed-off-by"
        } else {
            "No Signed-off-by"
        }));
    }
    match parts.co_authors {
        [] => {}
        [one] => said.push(format!("With {one}")),
        [first, rest @ ..] => said.push(format!("With {first}, +{}", rest.len())),
    }
    if parts.skips_hooks {
        said.push(String::from("Hooks skipped"));
    }
    said.join(" \u{b7} ")
}

/// What the line under the commit box says, each part none while it is as it starts.
#[derive(Clone, Copy, Debug, Default)]
pub struct OptionsSaid<'a> {
    /// Whether the commit is signed, when it differs from git's configuration.
    pub sign: Option<bool>,
    /// Whether `Signed-off-by` is added, when it differs from `commit.signOff`.
    pub sign_off: Option<bool>,
    /// The names of the co-authors chosen.
    pub co_authors: &'a [&'a str],
    /// Whether the hooks are skipped.
    pub skips_hooks: bool,
}
