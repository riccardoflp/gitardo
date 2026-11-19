//! The people who made commits in a repository (`commit.read-authors`), among whom the co-authors of a commit are
//! chosen (`commit.co-authors`).

/// Someone who made commits in the repository.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Author {
    /// The name of the latest of their commits.
    pub name: String,
    /// The email, as their latest commit has it.
    pub email: String,
    /// How many commits they made, among those read.
    pub commits: usize,
}

impl Author {
    /// Whether the author has an email, without regard to case.
    #[must_use]
    pub fn has_email(&self, email: &str) -> bool {
        self.email.eq_ignore_ascii_case(email)
    }

    /// Whether each word typed is in the name or in the email, without regard to case, as Quick search matches.
    #[must_use]
    pub fn matches(&self, typed: &str) -> bool {
        let name = self.name.to_lowercase();
        let email = self.email.to_lowercase();
        typed
            .to_lowercase()
            .split_whitespace()
            .all(|word| name.contains(word) || email.contains(word))
    }
}

/// The authors of commits given newest first, each as its name and email: one author for each email, without regard to
/// case, named as their latest commit says, the one with the most commits first, and by name among those with as many.
#[must_use]
pub fn authors_of<'a>(commits: impl IntoIterator<Item = (&'a str, &'a str)>) -> Vec<Author> {
    let mut authors: Vec<Author> = Vec::new();
    for (name, email) in commits {
        match authors.iter_mut().find(|author| author.has_email(email)) {
            Some(author) => author.commits += 1,
            None => authors.push(Author {
                name: name.to_owned(),
                email: email.to_owned(),
                commits: 1,
            }),
        }
    }
    authors.sort_by(|left, right| {
        right
            .commits
            .cmp(&left.commits)
            .then_with(|| left.name.cmp(&right.name))
    });
    authors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authors_of_counts_each_email_once_without_regard_to_case_with_the_latest_name() {
        let authors = authors_of([
            ("Ben Moss", "Ben@Example.com"),
            ("Cy", "cy@example.com"),
            ("Ben", "ben@example.com"),
        ]);
        assert_eq!(authors.len(), 2);
        assert_eq!((authors[0].name.as_str(), authors[0].commits), ("Ben Moss", 2));
        assert_eq!((authors[1].name.as_str(), authors[1].commits), ("Cy", 1));
    }

    #[test]
    fn an_author_matches_each_word_typed_in_the_name_or_the_email() {
        let author = Author {
            name: String::from("Ben Moss"),
            email: String::from("ben@example.com"),
            commits: 1,
        };
        assert!(author.matches("ben"));
        assert!(author.matches("MOSS example"));
        assert!(!author.matches("cy"));
    }
}
