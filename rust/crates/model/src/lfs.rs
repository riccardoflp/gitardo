//! Files a repository stores with Git LFS (D84): whether Git LFS is there and what the repository stores with it, the
//! files with their sizes, a pointer told apart from a file, and the content a pointer stands for.

#[cfg(test)]
mod tests;

/// The most bytes a pointer has.
const MOST_POINTER_BYTES: usize = 1024;

/// The versions of the specification a pointer names on its first line.
const VERSIONS: [&str; 2] = [
    "https://git-lfs.github.com/spec/v1",
    "https://hawser.github.com/spec/v1",
];

/// Whether Git LFS is installed and set up for a repository, and the patterns of the files it stores with it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfsStatus {
    /// The version of Git LFS, `3.7.1`; none when it is not installed.
    pub version: Option<String>,
    /// Whether the configuration of the repository runs the filter of Git LFS (`filter.lfs.process` or
    /// `filter.lfs.smudge`).
    pub is_set_up: bool,
    /// The patterns whose attributes say `filter=lfs`.
    pub patterns: Vec<LfsPattern>,
}

/// A pattern of the files stored with Git LFS, where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfsPattern {
    /// The pattern, as written: `*.psd`.
    pub pattern: String,
    /// The file of attributes it is in, from the top of the working tree with `/`: `assets/.gitattributes`, or
    /// `.git/info/attributes`.
    pub file: String,
    /// Its line in that file, from 1.
    pub line: usize,
}

/// What reading the state of Git LFS is asked to read again.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfsStatusRequest {
    /// The version read before, once for the engine: none to run `git lfs version`.
    pub version: Option<Option<String>>,
    /// The patterns read before, kept when no `.gitattributes` changed since: none to read the files of attributes.
    pub patterns: Option<Vec<LfsPattern>>,
}

/// What reading the files stored with Git LFS needs: the version of Git LFS, which decides how they are listed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfsFilesRequest {
    /// The version of Git LFS, `3.7.1`; empty when it is not installed, and nothing is run.
    pub version: String,
}

/// A file of the commit checked out stored with Git LFS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfsFile {
    /// Its path from the top of the working tree, with `/`.
    pub path: String,
    /// The size of its content, in bytes.
    pub size: u64,
    /// The id of its content: 64 hexadecimal digits.
    pub oid: String,
    /// Whether its content is in the store of the repository.
    pub is_downloaded: bool,
    /// Whether the file on disk is its content rather than its pointer.
    pub is_checked_out: bool,
}

/// What reading the content of a pointer needs: the pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfsContentRequest {
    /// The pointer.
    pub pointer: LfsPointer,
}

/// The content a pointer stands for, as the store of the repository on this computer has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LfsContent {
    /// Its bytes.
    Downloaded(Vec<u8>),
    /// Not in the store, or not whole.
    NotDownloaded,
}

/// A pointer of Git LFS: the id and the size of the content it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfsPointer {
    /// The id of the content, 64 hexadecimal digits in lower case, without `sha256:`.
    pub oid: String,
    /// The size of the content, in bytes.
    pub size: u64,
    /// The other keys, with their values, in their order.
    pub others: Vec<(String, String)>,
}

/// Tells whether some bytes are a pointer of Git LFS: text of less than 1,024 bytes, lines `<key> <value>` each ended
/// by a line feed alone, `version` first with a version of the specification, the other keys in the order of their
/// names, among them `oid` (`sha256:` and 64 hexadecimal digits in lower case) and `size`.
///
/// Feature: lfs.pointer
#[must_use]
pub fn pointer(bytes: &[u8]) -> Option<LfsPointer> {
    if bytes.is_empty() || bytes.len() >= MOST_POINTER_BYTES || !bytes.ends_with(b"\n") || bytes.contains(&b'\r') {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    let version = lines.next()?.strip_prefix("version ")?;
    if !VERSIONS.contains(&version) {
        return None;
    }
    let mut keys: Vec<(&str, &str)> = Vec::new();
    for line in lines {
        let (key, value) = line.split_once(' ')?;
        if key.is_empty() || keys.last().is_some_and(|(before, _)| *before >= key) {
            return None;
        }
        keys.push((key, value));
    }
    let oid = keys.iter().find(|(key, _)| *key == "oid")?.1.strip_prefix("sha256:")?;
    let is_oid = oid.len() == 64
        && oid
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    let size = keys.iter().find(|(key, _)| *key == "size")?.1;
    if !is_oid || size.is_empty() || !size.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(LfsPointer {
        oid: oid.to_owned(),
        size: size.parse().ok()?,
        others: keys
            .iter()
            .filter(|(key, _)| !matches!(*key, "oid" | "size"))
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    })
}
