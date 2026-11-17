//! Where the links and the images of a Markdown file go: a web page, a file of the repository relative to the file, or
//! a heading of the file; and the name of a file of the language of a block of code, which tells its colors.

/// Where a link of a Markdown file goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// A web page, or an address to write to, which the browser opens.
    Web(String),
    /// A file of the repository: its path from the top of the working tree, with `/`, and the line its `#L12` names.
    File {
        /// The path from the top of the working tree.
        path: String,
        /// The line, from 1.
        line: Option<usize>,
    },
    /// A heading of the file, by its anchor.
    Heading(String),
    /// Nowhere Gitardo goes: another scheme, such as `javascript:`, or a path out of the repository.
    Nowhere,
}

/// Where a link of a file goes, the file given by its path from the top of the working tree.
#[must_use]
pub fn link_target(from: &str, href: &str) -> LinkTarget {
    let href = href.trim();
    if let Some(anchor) = href.strip_prefix('#') {
        return LinkTarget::Heading(decoded(anchor).to_lowercase());
    }
    if let Some(scheme) = scheme_of(href) {
        let is_web = ["http", "https", "mailto"]
            .iter()
            .any(|web| scheme.eq_ignore_ascii_case(web));
        return if is_web {
            LinkTarget::Web(href.to_owned())
        } else {
            LinkTarget::Nowhere
        };
    }
    if href.starts_with("//") {
        return LinkTarget::Nowhere;
    }
    let (path, fragment) = href.split_once('#').unwrap_or((href, ""));
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    match resolved(from, &decoded(path)) {
        Some(path) => LinkTarget::File {
            path,
            line: line_of(fragment),
        },
        None => LinkTarget::Nowhere,
    }
}

/// The path from the top of the working tree of an image of a file, when it is one of the repository; none for an image
/// of the web, which is not fetched.
#[must_use]
pub fn image_path(from: &str, source: &str) -> Option<String> {
    match link_target(from, source) {
        LinkTarget::File { path, .. } => Some(path),
        LinkTarget::Web(_) | LinkTarget::Heading(_) | LinkTarget::Nowhere => None,
    }
}

/// The name of a file of the language a fence names, which tells the colors of its code: `rust` is `code.rs`, `ts` is
/// `code.ts`; a name the colors do not know gives a file of no language.
#[must_use]
pub fn code_file_name(language: &str) -> String {
    let language = language.to_ascii_lowercase();
    let extension = match language.as_str() {
        "rust" => "rs",
        "typescript" | "tsx" => "ts",
        "javascript" | "jsx" | "node" => "js",
        "python" | "python3" => "py",
        "ruby" => "rb",
        "kotlin" => "kt",
        "csharp" | "c#" => "cs",
        "c++" => "cpp",
        "golang" => "go",
        "shell" | "bash" | "zsh" | "console" | "shellsession" => "sh",
        "powershell" | "pwsh" | "ps" => "ps1",
        "markdown" => "md",
        "yml" => "yaml",
        "docker" => "dockerfile",
        other => other,
    };
    format!("code.{extension}")
}

/// The scheme of an address, `https` of `https://example.com`, when it has one: letters, digits, `+`, `-` and `.`
/// before a colon, starting with a letter. A letter of a drive of Windows, `C:`, is not one.
fn scheme_of(href: &str) -> Option<&str> {
    let (scheme, _) = href.split_once(':')?;
    let mut characters = scheme.chars();
    let is_scheme = characters.next().is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
        && scheme.len() > 1;
    is_scheme.then_some(scheme)
}

/// A path of a link from the folder of a file, or from the top of the working tree when it starts with `/`, with its
/// `.` and `..` taken away; none when it goes above the top.
fn resolved(from: &str, path: &str) -> Option<String> {
    if path.is_empty() {
        return Some(from.to_owned());
    }
    let mut parts: Vec<&str> = Vec::new();
    if !path.starts_with('/') {
        parts.extend(from.split('/').filter(|part| !part.is_empty()));
        let _file = parts.pop();
    }
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                let _parent = parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The line a fragment such as `L12`, or `L12-L20`, names.
fn line_of(fragment: &str) -> Option<usize> {
    let first = fragment.split('-').next()?;
    first.strip_prefix('L')?.parse().ok().filter(|line| *line > 0)
}

/// A part of an address with its `%20` and the other escapes of bytes read back; an escape that is not one is kept.
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let escaped = (byte == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        if let Some(value) = escaped {
            out.push(value);
            index += 3;
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}
