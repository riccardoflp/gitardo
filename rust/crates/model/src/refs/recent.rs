//! The local branches checked out last, for **Recent**.

/// How many branches **Recent** lists at most.
const MOST: usize = 5;

/// The local branches checked out last, the latest first and up to five, from the moves of `HEAD` (the subjects of its
/// reflog, the latest first), the local branches that exist and the one checked out now. Each move
/// `checkout: moving from <a> to <b>` gives `<b>` then `<a>`; a name is kept when it is a local branch that still
/// exists, is not the one checked out, and is not already kept. Other moves are passed over.
///
/// Feature: refs.recent
#[must_use]
pub fn recent(moves: &[String], branches: &[String], current: Option<&str>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::with_capacity(MOST);
    let names = moves.iter().filter_map(|subject| {
        subject
            .strip_prefix("checkout: moving from ")
            .and_then(|rest| rest.split_once(" to "))
    });
    for (left, arrived) in names {
        for name in [arrived, left] {
            let is_wanted = Some(name) != current
                && branches.iter().any(|branch| branch == name)
                && !kept.iter().any(|known| known == name);
            if is_wanted {
                kept.push(name.to_owned());
            }
            if kept.len() == MOST {
                return kept;
            }
        }
    }
    kept
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::recent;

    /// The subjects of the reflog of a series of checkouts, the latest first.
    fn moves(path: &[&str]) -> Vec<String> {
        path.windows(2)
            .rev()
            .map(|pair| format!("checkout: moving from {} to {}", pair[0], pair[1]))
            .collect()
    }

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn refs_recent__after_a_then_b_then_c_lists_b_then_a() {
        let found = recent(&moves(&["a", "b", "c"]), &owned(&["main", "a", "b", "c"]), Some("c"));
        assert_eq!(found, ["b", "a"]);
    }

    #[test]
    fn refs_recent__a_branch_deleted_since_is_not_listed() {
        let found = recent(&moves(&["main", "a", "b"]), &owned(&["main", "b"]), Some("b"));
        assert_eq!(found, ["main"]);
    }

    #[test]
    fn refs_recent__moves_to_a_commit_are_passed_over() {
        let mut subjects = moves(&["main", "3f2a1c9d0e4b5a6978877665544332211aabbccd", "feature"]);
        subjects.insert(0, String::from("commit: Add the parser"));
        let found = recent(&subjects, &owned(&["main", "feature"]), None);
        assert_eq!(found, ["feature", "main"]);
    }

    #[test]
    fn refs_recent__after_eight_branches_five_are_listed() {
        let path = ["b1", "b2", "b3", "b4", "b5", "b6", "b7", "b8"];
        let found = recent(&moves(&path), &owned(&path), Some("b8"));
        assert_eq!(found, ["b7", "b6", "b5", "b4", "b3"]);
    }
}
