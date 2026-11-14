//! The folders the user trusts: what such a folder, and every folder in it, asks to run is run (`trust.is-trusted`).

use std::path::{Path, PathBuf};

/// Whether the user has trusted the folder of the window, so that the commands it asks to run are run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Trust {
    /// Not trusted: nothing it asks to run is run.
    #[default]
    NotTrusted,
    /// Trusted.
    Trusted,
}

/// Whether a folder is trusted: it is one of the folders trusted, or inside one. The folder that holds a trusted folder
/// is not. Names are compared whole, as the system compares them: on Windows the case and the slashes do not count.
///
/// Feature: trust.is-trusted
#[must_use]
pub fn is_trusted(folder: &Path, trusted: &[PathBuf]) -> Trust {
    if trusted
        .iter()
        .any(|holding| crate::repo::path_in_folder(folder, holding).is_some())
    {
        Trust::Trusted
    } else {
        Trust::NotTrusted
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Trust, is_trusted};

    fn trusted(folders: &[&str]) -> Vec<PathBuf> {
        folders.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn trust_is_trusted__a_folder_trusted_is_trusted_and_one_never_trusted_is_not() {
        let list = trusted(&["/work/app", "/work/tools"]);
        assert_eq!(is_trusted(Path::new("/work/app"), &list), Trust::Trusted);
        assert_eq!(is_trusted(Path::new("/work/tools/"), &list), Trust::Trusted);
        assert_eq!(is_trusted(Path::new("/home/downloads/app"), &list), Trust::NotTrusted);
        assert_eq!(is_trusted(Path::new("/work/app"), &[]), Trust::NotTrusted);
    }

    #[test]
    fn trust_is_trusted__a_folder_inside_is_trusted_and_the_folder_holding_it_is_not() {
        let list = trusted(&["/work/app"]);
        assert_eq!(is_trusted(Path::new("/work/app/lib/core"), &list), Trust::Trusted);
        assert_eq!(is_trusted(Path::new("/work"), &list), Trust::NotTrusted);
    }

    #[test]
    fn trust_is_trusted__trusting_a_folder_does_not_trust_one_whose_name_starts_like_it() {
        let list = trusted(&["/work/app"]);
        assert_eq!(is_trusted(Path::new("/work/app-old"), &list), Trust::NotTrusted);
        assert_eq!(is_trusted(Path::new("/work/app-old/src"), &list), Trust::NotTrusted);
    }

    #[test]
    fn trust_is_trusted__on_windows_the_case_and_the_slashes_do_not_count() {
        let list = trusted(&["c:/work/app"]);
        let expected = if cfg!(windows) {
            Trust::Trusted
        } else {
            Trust::NotTrusted
        };
        assert_eq!(is_trusted(Path::new(r"C:\Work\App"), &list), expected);
        assert_eq!(is_trusted(Path::new(r"c:\work\app\src"), &list), Trust::Trusted);
    }
}
