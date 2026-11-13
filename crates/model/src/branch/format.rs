//! `branch.format-name`: the name of a new branch made from `branches.format` and the values of its parts, and what is
//! wrong with a format, said before anything is asked.

use std::collections::BTreeMap;

use crate::Timestamp;
use crate::contract::Text;
use crate::state::RepositoryState;

use super::local_name;

/// An input of `branches.inputs`: what a `${input:<id>}` of the format asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInput {
    /// Its `id`, as the format names it.
    pub id: String,
    /// Its `type`: typed or picked.
    pub kind: InputKind,
    /// Its `description`, the prompt of its question.
    pub description: Option<String>,
    /// Its `default`, proposed before any value was given.
    pub default: Option<String>,
    /// Its `pattern`, a regular expression what is typed must match.
    pub pattern: Option<String>,
    /// Its `options`, for a pick.
    pub options: Vec<String>,
    /// Its `allowCustom`: whether a pick also takes a value typed.
    pub allow_custom: bool,
}

/// How an input is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// `promptString`: an input box.
    PromptString,
    /// `pickString`: a pick of its options.
    PickString,
}

/// The values the parts of a format stand for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormatValues {
    /// The value given for each input, by its id.
    pub inputs: BTreeMap<String, String>,
    /// What `${base}` stands for: the branch the new one starts from, a remote branch with its remote.
    pub base: String,
    /// The remotes of the repository, which `${base}` loses.
    pub remotes: Vec<String>,
    /// What `${repo}` stands for: the name of the folder of the main worktree.
    pub repository: String,
    /// What `${date}` stands for: today, as `2026-10-15`.
    pub date: String,
}

/// A part of a format between `${` and `}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part<'a> {
    /// Text kept as it is written.
    Kept(&'a str),
    /// `${…}`, by what is between the braces.
    Placeholder(&'a str),
}

/// The parts of a format, in order. A `${` with no `}` after it is kept as it is written.
fn parts(format: &str) -> Vec<Part<'_>> {
    let mut parts = Vec::new();
    let mut rest = format;
    while let Some(start) = rest.find("${") {
        let (before, from) = rest.split_at(start);
        let Some(end) = from.find('}') else {
            break;
        };
        if !before.is_empty() {
            parts.push(Part::Kept(before));
        }
        parts.push(Part::Placeholder(from.get(2..end).unwrap_or_default()));
        rest = from.get(end + 1..).unwrap_or_default();
    }
    if !rest.is_empty() {
        parts.push(Part::Kept(rest));
    }
    parts
}

/// The name a format makes with the values of its parts: `${input:<id>}` is the value of that input wherever it
/// appears, `${base}` the branch it starts from without its remote, `${repo}` the folder of the main worktree,
/// `${date}` today; the rest is kept as it is written.
///
/// Feature: branch.format-name
#[must_use]
pub fn format_name(format: &str, values: &FormatValues) -> String {
    let mut name = String::new();
    for part in parts(format) {
        match part {
            Part::Kept(text) => name.push_str(text),
            Part::Placeholder("base") => name.push_str(local_name(&values.base, &values.remotes)),
            Part::Placeholder("repo") => name.push_str(&values.repository),
            Part::Placeholder("date") => name.push_str(&values.date),
            Part::Placeholder(other) => {
                let value = other.strip_prefix("input:").and_then(|id| values.inputs.get(id));
                name.push_str(&value.map_or_else(|| format!("${{{other}}}"), String::clone));
            }
        }
    }
    name
}

/// The ids of the inputs a format uses, in the order they first appear, each once.
pub(super) fn input_ids(format: &str) -> Vec<&str> {
    let mut ids: Vec<&str> = Vec::new();
    for part in parts(format) {
        if let Part::Placeholder(name) = part
            && let Some(id) = name.strip_prefix("input:")
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    ids
}

/// Whether a format uses `${base}`.
pub(super) fn has_base(format: &str) -> bool {
    parts(format).contains(&Part::Placeholder("base"))
}

/// What is wrong with a format, said before anything is asked: a `${…}` that is not one of the parts, or an input
/// that `branches.inputs` does not define.
#[must_use]
pub fn format_problem(format: &str, inputs: &[BranchInput]) -> Option<Text> {
    parts(format).into_iter().find_map(|part| {
        let Part::Placeholder(name) = part else {
            return None;
        };
        let problem = match name.strip_prefix("input:") {
            Some(id) if inputs.iter().any(|input| input.id == id) => return None,
            Some(id) => format!("The format uses `${{input:{id}}}`, which `branches.inputs` does not define."),
            None if ["base", "repo", "date"].contains(&name) => return None,
            None => format!(
                "The format uses `${{{name}}}`, which is not one of `${{base}}`, `${{repo}}`, `${{date}}` or \
                 `${{input:<id>}}`."
            ),
        };
        Some(Text::from(format!("{problem} Change it in the settings.")))
    })
}

/// The name of the folder of the main worktree of a repository, whichever worktree was read.
#[must_use]
pub fn repository_name(state: &RepositoryState) -> String {
    let main = state
        .worktrees
        .iter()
        .find(|worktree| worktree.is_main)
        .map_or(state.folder.as_path(), |worktree| worktree.folder.as_path());
    main.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A day as `${date}` writes it, `2026-10-15`, from a moment seen `utc_offset` seconds from UTC.
///
/// Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms", `civil_from_days`.
#[must_use]
pub fn day_of(moment: Timestamp, utc_offset: i64) -> String {
    let days = (moment.0 + utc_offset).div_euclid(86_400);
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;

    use super::{BranchInput, FormatValues, InputKind, day_of, format_name, format_problem, repository_name};
    use crate::Timestamp;
    use crate::state::{RepositoryState, Worktree};

    fn input(id: &str) -> BranchInput {
        BranchInput {
            id: id.to_owned(),
            kind: InputKind::PromptString,
            description: None,
            default: None,
            pattern: None,
            options: Vec::new(),
            allow_custom: false,
        }
    }

    #[test]
    fn branch_format_name__fills_the_inputs_and_the_base_without_its_remote() {
        let values = FormatValues {
            inputs: [("ticket", "123"), ("kind", "fix")]
                .into_iter()
                .map(|(id, value)| (id.to_owned(), value.to_owned()))
                .collect(),
            base: String::from("origin/feature/login"),
            remotes: vec![String::from("origin")],
            ..FormatValues::default()
        };
        assert_eq!(
            format_name("PRJ-${input:ticket}-${input:kind}-${base}", &values),
            "PRJ-123-fix-feature/login"
        );
    }

    #[test]
    fn branch_format_name__names_the_repository_by_its_main_worktree_and_the_day() {
        let worktree = |folder: &str, is_main, is_current| Worktree {
            folder: PathBuf::from(folder),
            commit: None,
            branch: None,
            is_main,
            is_current,
            is_missing: false,
            uncommitted: None,
        };
        let state = RepositoryState {
            folder: PathBuf::from("projects/app.worktrees/x"),
            worktrees: vec![
                worktree("projects/app", true, false),
                worktree("projects/app.worktrees/x", false, true),
            ],
            ..RepositoryState::default()
        };
        // 15 October 2026, at noon UTC.
        let noon = Timestamp(1_792_022_400 + 43_200);
        let values = FormatValues {
            repository: repository_name(&state),
            date: day_of(noon, 0),
            ..FormatValues::default()
        };
        assert_eq!(format_name("${repo}/${date}", &values), "app/2026-10-15");
        assert_eq!(day_of(Timestamp(0), 0), "1970-01-01");
        assert_eq!(day_of(Timestamp(1_792_022_400 - 1), 3_600), "2026-10-15");
    }

    #[test]
    fn branch_format_name__says_what_a_format_uses_that_it_cannot_fill() {
        let problem = format_problem("${user}-${input:ticket}", &[input("ticket")]).map(|text| text.to_string());
        assert_eq!(
            problem.as_deref(),
            Some(
                "The format uses `${user}`, which is not one of `${base}`, `${repo}`, `${date}` or \
                 `${input:<id>}`. Change it in the settings."
            )
        );
        let problem = format_problem("PRJ-${input:ticket}", &[]).map(|text| text.to_string());
        assert_eq!(
            problem.as_deref(),
            Some(
                "The format uses `${input:ticket}`, which `branches.inputs` does not define. Change it in the settings."
            )
        );
        assert_eq!(
            format_problem("${base}-${repo}-${date}-${input:ticket}", &[input("ticket")]),
            None
        );
    }

    #[test]
    fn branch_format_name__a_format_with_no_parts_makes_itself() {
        assert_eq!(format_name("release/next", &FormatValues::default()), "release/next");
        assert_eq!(format_name("odd-${", &FormatValues::default()), "odd-${");
    }
}
