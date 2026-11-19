//! The upkeep of a repository: the room its objects take (`upkeep.read-size`), said in words for **Clean up…**
//! (`upkeep.clean-up`), and whether git's scheduled maintenance looks after it (`upkeep.read-background`).

use std::path::Path;

/// How much room the objects of a repository take, as `git count-objects -v` says, in KiB.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RepositorySize {
    /// The loose objects (`count`).
    pub loose: u64,
    /// The room they take (`size`).
    pub loose_kib: u64,
    /// The objects packed (`in-pack`).
    pub packed: u64,
    /// The packs (`packs`).
    pub packs: u64,
    /// The room the packs take (`size-pack`).
    pub pack_kib: u64,
    /// The loose objects that are in a pack too (`prune-packable`).
    pub prune_packable: u64,
    /// The room of the files of the object folder that are not objects (`size-garbage`).
    pub garbage_kib: u64,
}

impl RepositorySize {
    /// The room the objects take in all, in KiB.
    #[must_use]
    pub fn total_kib(&self) -> u64 {
        self.loose_kib + self.pack_kib + self.garbage_kib
    }

    /// The numbers in the order an answer carries them, and back (`Query::RepositorySize`).
    #[must_use]
    pub fn to_names(&self) -> Vec<String> {
        [
            self.loose,
            self.loose_kib,
            self.packed,
            self.packs,
            self.pack_kib,
            self.prune_packable,
            self.garbage_kib,
        ]
        .iter()
        .map(u64::to_string)
        .collect()
    }

    /// The size from the numbers of [`RepositorySize::to_names`]; a number missing or not one reads as 0.
    #[must_use]
    pub fn from_names(names: &[String]) -> RepositorySize {
        let number = |at: usize| names.get(at).and_then(|name| name.parse().ok()).unwrap_or(0);
        RepositorySize {
            loose: number(0),
            loose_kib: number(1),
            packed: number(2),
            packs: number(3),
            pack_kib: number(4),
            prune_packable: number(5),
            garbage_kib: number(6),
        }
    }
}

/// A size in KiB for people: `380 KB`, `830 MB`, `1.2 GB`, in powers of 1,024.
#[must_use]
pub fn size_text(kib: u64) -> String {
    #[allow(clippy::cast_precision_loss, reason = "a size is said with one decimal at most")]
    let mut value = kib as f64;
    for unit in ["KB", "MB", "GB"] {
        if value < 1024.0 || unit == "GB" {
            return if value < 10.0 && unit != "KB" {
                format!("{value:.1} {unit}")
            } else {
                format!("{value:.0} {unit}")
            };
        }
        value /= 1024.0;
    }
    format!("{value:.1} GB")
}

/// A whole number with a comma every three digits: `45,210`.
#[must_use]
pub fn count_text(count: u64) -> String {
    let digits = count.to_string();
    let mut said = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            said.push(',');
        }
        said.push(digit);
    }
    said
}

/// What **Clean up…** says above its question: **`<repository>` takes 1.2 GB: 830 MB in 3 packs, and 45,210 loose
/// objects in 380 MB.**
#[must_use]
pub fn size_sentence(repository: &str, size: &RepositorySize) -> String {
    let packs = match size.packs {
        1 => String::from("1 pack"),
        packs => format!("{} packs", count_text(packs)),
    };
    let loose = match size.loose {
        1 => String::from("1 loose object"),
        loose => format!("{} loose objects", count_text(loose)),
    };
    format!(
        "{repository} takes {}: {} in {packs}, and {loose} in {}.",
        size_text(size.total_kib()),
        size_text(size.pack_kib),
        size_text(size.loose_kib)
    )
}

/// Whether a folder is among those `maintenance.repo` lists, each compared as git writes it, absolute with `/`, and
/// whatever the case on Windows and macOS, whose file systems ignore it.
#[must_use]
pub fn is_registered(listed: &[String], folder: &Path) -> bool {
    let normal = |path: &str| {
        let path = path.trim().replace('\\', "/");
        let path = path.trim_end_matches('/').to_owned();
        if cfg!(any(windows, target_os = "macos")) {
            path.to_lowercase()
        } else {
            path
        }
    };
    let folder = normal(&folder.to_string_lossy());
    listed.iter().any(|listed| normal(listed) == folder)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::Path;

    use super::{RepositorySize, is_registered, size_sentence, size_text};

    #[test]
    fn upkeep_read_size__the_sizes_are_said_for_people() {
        assert_eq!(size_text(1), "1 KB");
        assert_eq!(size_text(5120), "5.0 MB");
        assert_eq!(size_text(850_000), "830 MB");
        assert_eq!(size_text(1_258_291), "1.2 GB");
    }

    #[test]
    fn upkeep_clean_up__the_question_says_the_room_the_repository_takes() {
        let size = RepositorySize {
            loose: 45_210,
            loose_kib: 389_120,
            packed: 1200,
            packs: 3,
            pack_kib: 849_920,
            ..RepositorySize::default()
        };
        assert_eq!(
            size_sentence("app", &size),
            "app takes 1.2 GB: 830 MB in 3 packs, and 45,210 loose objects in 380 MB."
        );
        assert_eq!(RepositorySize::from_names(&size.to_names()), size);
    }

    #[test]
    fn upkeep_read_background__a_folder_written_with_backslashes_is_the_same_folder() {
        let listed = vec![String::from("C:\\code\\app"), String::from("/srv/other")];
        assert!(is_registered(&listed, Path::new("C:/code/app")));
        assert!(!is_registered(&listed, Path::new("C:/code/app2")));
    }
}
