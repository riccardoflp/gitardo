//! Patches: how many commits a patch made by `git format-patch` has.

/// The number of commits of a patch made by `git format-patch`; 0 for any other text.
///
/// Such a patch starts, after any blank space, with a line `From <hash> `, the hash of 40 to 64 lower-case hexadecimal
/// characters; its commits are its lines that start so.
///
/// Feature: patch.count-commits
#[must_use]
pub fn count_commits(input: &str) -> usize {
    if !starts_a_commit(input.trim_start()) {
        return 0;
    }
    input.lines().filter(|line| starts_a_commit(line)).count()
}

/// Whether a line starts a commit of `git format-patch`: `From `, a hash, and a space.
fn starts_a_commit(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("From ") else {
        return false;
    };
    let Some((hash, _)) = rest.split_once(' ') else {
        return false;
    };
    (40..=64).contains(&hash.len()) && hash.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::count_commits;

    /// A commit as `git format-patch --stdout` writes it, with a hash of some length.
    fn commit(hash: &str, subject: &str) -> String {
        format!(
            "From {hash} Mon Sep 17 00:00:00 2001\nFrom: A Writer <writer@example.com>\nDate: Thu, 12 Nov 2026 \
             10:00:00 +0100\nSubject: [PATCH] {subject}\n\n---\n a.txt | 1 +\n\ndiff --git a/a.txt b/a.txt\n\
             --- a/a.txt\n+++ b/a.txt\n@@ -1 +1,2 @@\n a\n+b\n-- \n2.47.0\n\n"
        )
    }

    #[test]
    fn patch_count_commits__five_commits_of_format_patch_count_5() {
        let patch: String = (0..5)
            .map(|index| commit(&format!("{index}").repeat(40), "Add b"))
            .collect();
        assert_eq!(count_commits(&patch), 5);
        assert_eq!(count_commits(&format!("\n  \n{patch}")), 5, "after blank space");
    }

    #[test]
    fn patch_count_commits__sha256_hashes_are_counted_the_same() {
        let patch: String = (0..2)
            .map(|index| commit(&format!("{index}").repeat(64), "Add b"))
            .collect();
        assert_eq!(count_commits(&patch), 2);
    }

    #[test]
    fn patch_count_commits__a_diff_counts_0_even_with_a_line_from() {
        let diff = format!(
            "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1,2 @@\n a\n+{}",
            commit(&"a".repeat(40), "Add b")
        );
        assert_eq!(count_commits(&diff), 0);
        assert_eq!(count_commits("From someone who wrote this\nHello\n"), 0);
    }
}
