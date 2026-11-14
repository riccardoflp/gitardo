//! What the operations of the File page are given (`file-view.save`, `file-view.open-external`).

/// A file to save: its path, the text to write, and the bytes it had when it was read, to tell whether another program
/// changed it since.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileSave {
    /// The path from the top of the working tree, with `/`.
    pub path: String,
    /// The bytes to write: the text in the line endings and the encoding of the file.
    pub contents: Vec<u8>,
    /// The bytes on disk when the page read them, or when it last saved them.
    pub read: Vec<u8>,
}

/// A file of the working tree and the line of the cursor in it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileAt {
    /// The path from the top of the working tree, with `/`.
    pub path: String,
    /// The line, from 1.
    pub line: usize,
}
