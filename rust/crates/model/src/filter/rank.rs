//! Which names match what is typed best, as the branch filter and the branch rail list them.

/// How well a name matches, the best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Match {
    /// The whole name: `main`.
    Whole,
    /// Its last part after a slash: `origin/main`.
    LastPart,
    /// Its start: `maintenance`.
    Start,
    /// The start of one of its parts, split at `/`, `.`, `_` and `-`: `release/main-2`.
    PartStart,
    /// Anywhere: `domain`.
    Anywhere,
}

/// The places of the names that contain what is typed, without regard to case, the best matches first and, among names
/// that match as well, in their order. Nothing typed leaves every name in its order.
///
/// Feature: filter.rank-names
#[must_use]
pub fn rank_names(names: &[String], typed: &str) -> Vec<usize> {
    if typed.is_empty() {
        return (0..names.len()).collect();
    }
    let typed = typed.to_lowercase();
    let mut ranked: Vec<(Match, usize)> = names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| how_it_matches(&name.to_lowercase(), &typed).map(|found| (found, index)))
        .collect();
    ranked.sort_unstable();
    ranked.into_iter().map(|(_, index)| index).collect()
}

/// How a name, in lower case, matches what is typed, in lower case; none when it does not contain it.
fn how_it_matches(name: &str, typed: &str) -> Option<Match> {
    if !name.contains(typed) {
        return None;
    }
    let last_part = name.rsplit('/').next().unwrap_or(name);
    let found = if name == typed {
        Match::Whole
    } else if last_part == typed {
        Match::LastPart
    } else if name.starts_with(typed) {
        Match::Start
    } else if name.split(['/', '.', '_', '-']).any(|part| part.starts_with(typed)) {
        Match::PartStart
    } else {
        Match::Anywhere
    };
    Some(found)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::rank_names;

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn ranked(names: &[&str], typed: &str) -> Vec<String> {
        let names = owned(names);
        rank_names(&names, typed)
            .into_iter()
            .map(|index| names[index].clone())
            .collect()
    }

    const NAMES: [&str; 5] = ["domain", "release/main-2", "maintenance", "origin/main", "main"];

    #[test]
    fn filter_rank_names__main_lists_the_whole_name_the_last_part_the_start_a_part_then_anywhere() {
        assert_eq!(
            ranked(&NAMES, "main"),
            ["main", "origin/main", "maintenance", "release/main-2", "domain"]
        );
    }

    #[test]
    fn filter_rank_names__case_does_not_count() {
        assert_eq!(
            ranked(&NAMES, "MAIN"),
            ["main", "origin/main", "maintenance", "release/main-2", "domain"]
        );
    }

    #[test]
    fn filter_rank_names__a_name_that_does_not_contain_it_is_left_out() {
        assert_eq!(ranked(&["feature/x", "fix/parser"], "fix"), ["fix/parser"]);
    }

    #[test]
    fn filter_rank_names__names_that_match_as_well_keep_their_order() {
        assert_eq!(ranked(&["a/main", "b/main"], "main"), ["a/main", "b/main"]);
        assert_eq!(rank_names(&owned(&["b", "a"]), ""), [0, 1]);
    }
}
