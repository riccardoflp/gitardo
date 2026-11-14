//! Secrets on Linux, until Gitardo speaks to the Secret Service: a file `credentials` in Gitardo's folder of the
//! user's configuration (`$XDG_CONFIG_HOME/gitardo-desktop`, or `~/.config/gitardo-desktop`), which only the user may
//! read or write (mode `0600`), in a folder only the user may open when Gitardo makes it (mode `0700`).
//!
//! A line of the file is a name, a tab and its secret; a backslash, a tab and the line breaks are written `\\`, `\t`,
//! `\n` and `\r`. A line that starts with `#` says what the file is. The file is written whole into another file
//! beside it, which then takes its place, so that a secret is never half written.

use std::fs::{self, DirBuilder, OpenOptions, Permissions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::error::SystemError;

/// The name of Gitardo's folder under the configuration folder of the user, as the settings name it.
const FOLDER_NAME: &str = "gitardo-desktop";
/// The name of the file of the secrets.
const FILE_NAME: &str = "credentials";
/// What the first line of the file says.
const HEADER: &str = "# The secrets of Gitardo Desktop, readable only by this user. Do not share this file.";

/// The secret kept under a name; none when there is none.
pub(crate) fn read_secret(name: &str) -> Result<Option<String>, SystemError> {
    read_from(&file()?, name)
}

/// Keeps a secret under a name, in place of what was kept there.
pub(crate) fn write_secret(name: &str, secret: &str) -> Result<(), SystemError> {
    write_to(&file()?, name, Some(secret))
}

/// Takes the secret kept under a name out of the file; nothing happens when there is none.
pub(crate) fn delete_secret(name: &str) -> Result<(), SystemError> {
    write_to(&file()?, name, None)
}

/// The file of the secrets, from the environment.
fn file() -> Result<PathBuf, SystemError> {
    let variable = |name: &str| {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    variable("XDG_CONFIG_HOME")
        .or_else(|| variable("HOME").map(|home| home.join(".config")))
        .map(|folder| folder.join(FOLDER_NAME).join(FILE_NAME))
        .ok_or(SystemError::Unsupported {
            what: "say where the user's configuration folder is: neither XDG_CONFIG_HOME nor HOME is set",
        })
}

/// The secret kept under a name in a file of secrets.
fn read_from(path: &Path, name: &str) -> Result<Option<String>, SystemError> {
    Ok(entries(path)?
        .into_iter()
        .find(|(kept, _)| kept == name)
        .map(|(_, secret)| secret))
}

/// Keeps a secret under a name in a file of secrets, or takes it out when there is none.
fn write_to(path: &Path, name: &str, secret: Option<&str>) -> Result<(), SystemError> {
    let mut entries = entries(path)?;
    let before = entries.len();
    entries.retain(|(kept, _)| kept != name);
    if secret.is_none() && entries.len() == before {
        return Ok(());
    }
    if let Some(secret) = secret {
        entries.push((name.to_owned(), secret.to_owned()));
    }
    let mut text = format!("{HEADER}\n");
    for (kept, secret) in &entries {
        text.push_str(&escape(kept));
        text.push('\t');
        text.push_str(&escape(secret));
        text.push('\n');
    }
    replace(path, &text).map_err(|error| SystemError::system("writing the file of the secrets", &error))
}

/// The names and secrets of a file of secrets; none when it is not there.
fn entries(path: &Path) -> Result<Vec<(String, String)>, SystemError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(SystemError::system("reading the file of the secrets", &error)),
    };
    Ok(text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('\t'))
        .map(|(name, secret)| (unescape(name), unescape(secret)))
        .collect())
}

/// Writes the whole file beside it, readable only by the user, and puts it in its place.
fn replace(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        DirBuilder::new().recursive(true).mode(0o700).create(folder)?;
    }
    let mut written = path.as_os_str().to_owned();
    written.push(format!(".{}.new", std::process::id()));
    let written = PathBuf::from(written);
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&written)?;
        // A file left from before with other rights gets those of a file of secrets.
        file.set_permissions(Permissions::from_mode(0o600))?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&written, path)
    })();
    if result.is_err() {
        let _removed = fs::remove_file(&written);
    }
    result
}

/// A name or a secret as a line of the file writes it.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// A name or a secret as a line of the file wrote it.
fn unescape(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            plain.push(character);
            continue;
        }
        match characters.next() {
            Some('t') => plain.push('\t'),
            Some('n') => plain.push('\n'),
            Some('r') => plain.push('\r'),
            Some(other) => plain.push(other),
            None => plain.push('\\'),
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::{escape, read_from, unescape, write_to};

    #[test]
    fn keeps_secrets_in_a_file_only_the_user_may_read() {
        let base = std::env::temp_dir().join(format!("gitardo-secrets-{}", std::process::id()));
        let _left = fs::remove_dir_all(&base);
        let path = base.join("gitardo-desktop").join("credentials");
        assert_eq!(read_from(&path, "github.com").unwrap(), None);
        write_to(&path, "github.com", Some("first")).unwrap();
        write_to(&path, "ai.anthropic", Some("a key\twith a tab, a \\ and a\nline break")).unwrap();
        write_to(&path, "github.com", Some("second, kept in its place")).unwrap();
        assert_eq!(
            read_from(&path, "github.com").unwrap().as_deref(),
            Some("second, kept in its place")
        );
        assert_eq!(
            read_from(&path, "ai.anthropic").unwrap().as_deref(),
            Some("a key\twith a tab, a \\ and a\nline break")
        );
        let mode = |path: &std::path::Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700);
        write_to(&path, "github.com", None).unwrap();
        write_to(&path, "github.com", None).unwrap();
        assert_eq!(read_from(&path, "github.com").unwrap(), None);
        assert!(read_from(&path, "ai.anthropic").unwrap().is_some());
        assert_eq!(
            fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1,
            "no file is left beside it"
        );
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn escapes_what_would_break_a_line() {
        for text in ["plain", "a\tb", "a\\tb", "end\\", "\r\n", ""] {
            assert_eq!(unescape(&escape(text)), text);
            assert!(!escape(text).contains(['\t', '\n', '\r']));
        }
    }
}
