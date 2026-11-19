//! The words of the inspector (`details.show`, `details.stash`) and of the page of a commit (`commit-page.view`).

/// What the inspector says while no row of the timeline is selected (Q65).
pub const NOTHING_SELECTED: &str = "Select a commit to see its message and the files it changed.";
/// What the inspector and the page say while the commit is read.
pub const LOADING: &str = "Loading\u{2026}";
/// What the inspector and the page say of a commit that is not in the repository any more.
pub const NOT_IN_REPOSITORY: &str = "This commit is not in the repository any more.";
/// What the inspector and the page say when git could not read the commit.
pub const COULD_NOT_READ: &str = "Git could not read this commit.";
/// The button that closes the inspector.
pub const CLOSE_INSPECTOR: &str = "Close the inspector";
/// The button beside it that opens the commit as a page.
pub const OPEN_AS_PAGE: &str = "Open as a page, with its changes";
/// The author of a commit.
pub const AUTHOR: &str = "Author";
/// The date of a commit.
pub const DATE: &str = "Date";
/// The committer of a commit, when it is not its author.
pub const COMMITTER: &str = "Committer";
/// The hash of a commit.
pub const COMMIT: &str = "Commit";
/// The parent of a commit.
pub const PARENT: &str = "Parent";
/// The parents of a merge.
pub const PARENTS: &str = "Parents";
/// What a click on the short hash in the inspector does.
pub const COPY_FULL_HASH: &str = "Copy full hash";
/// What a click on the hash in the page does.
pub const COPY_THE_HASH: &str = "Copy the hash";
/// What a click on a parent does in the inspector.
pub const GO_TO_PARENT: &str = "Select it in the timeline";
/// What a click on a parent does in the page.
pub const OPEN_PARENT: &str = "Open its page";
/// After the count of the files of a merge.
pub const AGAINST_FIRST_PARENT: &str = "against the first parent";
/// The button that lists the files one after the other.
pub const AS_LIST: &str = "One after the other, each with its directory";
/// The button that lists the files under their directories.
pub const AS_TREE: &str = "Under their directories";
/// The button of the inspector that opens all the files in the Diff page.
pub const OPEN_ALL_CHANGES: &str = "Open all changes in one Diff page";
/// The button of a file that shows its history.
pub const FILE_HISTORY: &str = "Show the history of this file";
/// The entry of the context menu of a file that compares it.
pub const COMPARE_WITH: &str = "Compare with\u{2026}";
/// The entry of the context menu of a file of a commit that opens it in the File page as the commit has it.
pub const OPEN_AT_REVISION: &str = "Open the file at this revision";
/// The link under a long message that shows it whole.
pub const SHOW_MORE: &str = "Show more";
/// The link under a long message shown whole.
pub const SHOW_LESS: &str = "Show less";
/// The list of the files, for the screen reader.
pub const FILES: &str = "Files";
/// The page of a commit, for the screen reader.
pub const PAGE: &str = "Commit";
/// The button of the page that brings the timeline to the commit.
pub const SHOW_IN_TIMELINE: &str = "Show in the timeline";
/// The button of the page, and the links of its notes, that open the changes in the Diff page.
pub const OPEN_IN_DIFF_PAGE: &str = "Open in the Diff page";
/// The link of a diff that could not be shown with the others.
pub const OPEN_IT_IN_DIFF_PAGE: &str = "Open it in the Diff page";
/// The heading of the changes of the page.
pub const CHANGES: &str = "Changes";
/// The button that puts the diffs in the page.
pub const SHOW_THE_CHANGES: &str = "Show the changes";
/// The button that takes them away.
pub const HIDE_THE_CHANGES: &str = "Hide the changes";
/// The button that folds every diff.
pub const FOLD_ALL: &str = "Fold all";
/// The button that unfolds every diff.
pub const UNFOLD_ALL: &str = "Unfold all";
/// The eye of a file in the page.
pub const SHOW_ITS_CHANGES: &str = "Show its changes in this page";
/// The arrow of a diff, folded.
pub const UNFOLD_DIFF: &str = "Unfold";
/// The arrow of a diff, unfolded.
pub const FOLD_DIFF: &str = "Fold";
/// A binary file in the list of the page.
pub const BINARY: &str = "binary";
/// The note of a binary file.
pub const BINARY_FILE: &str = "Binary file.";
/// The note of a file past the cap of lines.
pub const TOO_LONG: &str = "Too long to show with the others.";
/// The note of a file whose text did not change.
pub const NO_CHANGE_TO_TEXT: &str = "No change to its text.";
/// The note of a file renamed with no change to its text.
pub const RENAMED_NO_CHANGE: &str = "Renamed, with no change to its text.";
/// What the page says when the lines cannot be matched to the files.
pub const UNMATCHED: &str = "Git lists these changes in a way that cannot be shown here.";
/// The round button that takes the page back to its top.
pub const BACK_TO_TOP: &str = "Back to the top";

/// How many files a commit changed: **3 changed files**.
#[must_use]
pub fn changed_files(count: usize) -> String {
    if count == 1 {
        String::from("1 changed file")
    } else {
        format!("{count} changed files")
    }
}

/// The untracked files a stash took: **2 untracked files**.
#[must_use]
pub fn untracked_files(count: usize) -> String {
    if count == 1 {
        String::from("1 untracked file")
    } else {
        format!("{count} untracked files")
    }
}

/// The link at the end of a list cut at 300 files: **Show all 301 files…**.
#[must_use]
pub fn show_all_files(count: usize) -> String {
    format!("Show all {count} files\u{2026}")
}

/// The lines a file or a commit gained and lost: `+120 −45`.
#[must_use]
pub fn lines_changed(added: usize, removed: usize) -> String {
    format!("+{added} \u{2212}{removed}")
}

/// A person with the address: `Dana Ruiz <dana@example.com>`.
#[must_use]
pub fn person(name: &str, email: &str) -> String {
    format!("{name} <{email}>")
}

/// A renamed file: `old → new`.
#[must_use]
pub fn renamed(original: &str, path: &str) -> String {
    format!("{original} \u{2192} {path}")
}

/// The title of the page of a commit in its trail: the short hash and the subject, cut after 32 characters with `…`.
#[must_use]
pub fn page_title(short_hash: &str, subject: &str) -> String {
    const LONGEST: usize = 32;
    let cut: String = subject.chars().take(LONGEST).collect();
    if subject.chars().count() > LONGEST {
        format!("{short_hash} {}\u{2026}", cut.trim_end())
    } else {
        format!("{short_hash} {subject}")
    }
}

/// The line of the signature of a commit (`details.signature`).
pub const SIGNATURE: &str = "Signature";
/// What the line of the signature says while it is read.
pub const CHECKING_SIGNATURE: &str = "Checking the signature\u{2026}";

/// What the line of the signature says of what git said of it: `Good, Ada <ada@example.com>`, `Good, expired key`,
/// `Bad`; nothing for a commit not signed, which has no line.
#[must_use]
pub fn signature_words(signature: &model::details::CommitSignature) -> String {
    use model::details::SignatureVerdict;
    let said = match signature.verdict {
        SignatureVerdict::Good if signature.signer.is_empty() => "Good",
        SignatureVerdict::Good => return format!("Good, {}", signature.signer),
        SignatureVerdict::GoodUnknownValidity => "Good, from a key whose validity is not known",
        SignatureVerdict::GoodExpired => "Good, expired",
        SignatureVerdict::GoodExpiredKey => "Good, expired key",
        SignatureVerdict::GoodRevokedKey => "Good, revoked key",
        SignatureVerdict::Bad => "Bad",
        SignatureVerdict::CannotCheck if signature.key.is_empty() => "Cannot be checked",
        SignatureVerdict::CannotCheck => return format!("Cannot be checked: no key {} here", signature.key),
        SignatureVerdict::SignedNotChecked if signature.kind == Some(model::details::SignatureKind::Ssh) => {
            "Signed, not checked: no allowed signers are set"
        }
        SignatureVerdict::SignedNotChecked => "Signed, not checked",
        SignatureVerdict::NotSigned => "",
    };
    said.to_owned()
}

/// The tooltip of the line of a signature: its kind, the key, its fingerprint and the trust git gave it, each when
/// git said it.
#[must_use]
pub fn signature_tooltip(signature: &model::details::CommitSignature) -> String {
    use model::details::SignatureKind;
    let kind = match signature.kind {
        Some(SignatureKind::OpenPgp) => "OpenPGP signature",
        Some(SignatureKind::X509) => "X.509 signature",
        Some(SignatureKind::Ssh) => "SSH signature",
        None => "Signature",
    };
    let mut lines = vec![kind.to_owned()];
    for (name, value) in [
        ("Key", &signature.key),
        ("Fingerprint", &signature.fingerprint),
        ("Trust", &signature.trust),
    ] {
        if !value.is_empty() {
            lines.push(format!("{name}: {value}"));
        }
    }
    lines.join("\n")
}
