//! The tags of a repository against those of a remote (`tag.read-remote-tags`, `tag.compare-remote`): which the remote
//! has as here, under another object, or not at all.

/// A tag and the object it names: a commit for a lightweight tag, the tag object for an annotated one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagObject {
    /// Its name without `refs/tags/`: `v1.0.0`.
    pub name: String,
    /// The full hash of the object it names.
    pub object: String,
}

/// The tags a remote has, and those of the repository, each with its object, read together.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteTags {
    /// The remote asked: `origin`.
    pub remote: String,
    /// The tags of the remote.
    pub there: Vec<TagObject>,
    /// The tags of the repository.
    pub here: Vec<TagObject>,
}

/// Where a tag of the repository stands on a remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagOnRemote {
    /// The remote has a tag of its name naming the same object.
    Pushed,
    /// The remote has a tag of its name naming another object.
    DifferentThere,
    /// The remote has no tag of its name.
    NotPushed,
}

/// The tags of the repository sorted against those of a remote.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagComparison {
    /// The remote compared with.
    pub remote: String,
    /// Each tag of the repository, in the order given, with where it stands on the remote.
    pub here: Vec<(String, TagOnRemote)>,
    /// The tags of the remote with no tag of their name here.
    pub only_there: Vec<String>,
}

impl TagComparison {
    /// Where a tag of the repository stands on the remote; none for a tag the comparison does not know.
    #[must_use]
    pub fn of(&self, name: &str) -> Option<TagOnRemote> {
        self.here.iter().find(|(tag, _)| tag == name).map(|(_, state)| *state)
    }

    /// The tags of the repository the remote does not have.
    #[must_use]
    pub fn not_pushed(&self) -> Vec<&str> {
        self.named(TagOnRemote::NotPushed)
    }

    /// The tags of the repository the remote has under another object.
    #[must_use]
    pub fn different_there(&self) -> Vec<&str> {
        self.named(TagOnRemote::DifferentThere)
    }

    /// The tags of the repository that stand so on the remote.
    fn named(&self, wanted: TagOnRemote) -> Vec<&str> {
        self.here
            .iter()
            .filter(|(_, state)| *state == wanted)
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

/// Sorts the tags of the repository against those of a remote: pushed when the remote has a tag of the name naming the
/// same object, different there when it names another, not pushed when the remote has none of that name; and the tags
/// only the remote has. Names are compared as they are, case included.
///
/// Feature: tag.compare-remote
#[must_use]
pub fn compare_remote(here: &[TagObject], there: &RemoteTags) -> TagComparison {
    let state_of = |tag: &TagObject| match there.there.iter().find(|remote| remote.name == tag.name) {
        Some(remote) if remote.object == tag.object => TagOnRemote::Pushed,
        Some(_) => TagOnRemote::DifferentThere,
        None => TagOnRemote::NotPushed,
    };
    TagComparison {
        remote: there.remote.clone(),
        here: here.iter().map(|tag| (tag.name.clone(), state_of(tag))).collect(),
        only_there: there
            .there
            .iter()
            .filter(|remote| !here.iter().any(|tag| tag.name == remote.name))
            .map(|remote| remote.name.clone())
            .collect(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{RemoteTags, TagObject, TagOnRemote, compare_remote};

    fn tag(name: &str, object: char) -> TagObject {
        TagObject {
            name: name.to_owned(),
            object: object.to_string().repeat(40),
        }
    }

    fn remote(there: Vec<TagObject>) -> RemoteTags {
        RemoteTags {
            remote: String::from("origin"),
            there,
            here: Vec::new(),
        }
    }

    #[test]
    fn tag_compare_remote__pushed_not_pushed_and_only_on_the_remote() {
        let compared = compare_remote(
            &[tag("v1", 'a'), tag("v2", 'b')],
            &remote(vec![tag("v1", 'a'), tag("v3", 'c')]),
        );
        assert_eq!(compared.of("v1"), Some(TagOnRemote::Pushed));
        assert_eq!(compared.of("v2"), Some(TagOnRemote::NotPushed));
        assert_eq!(compared.only_there, ["v3"]);
        assert_eq!(compared.not_pushed(), ["v2"]);
    }

    #[test]
    fn tag_compare_remote__a_tag_made_again_on_another_commit_is_different_there() {
        let compared = compare_remote(&[tag("v1", 'd')], &remote(vec![tag("v1", 'a')]));
        assert_eq!(compared.of("v1"), Some(TagOnRemote::DifferentThere));
        assert_eq!(compared.different_there(), ["v1"]);
    }

    #[test]
    fn tag_compare_remote__an_annotated_tag_made_again_with_another_message_is_different_there() {
        // Made again on the same commit with another message, the annotated tag is another tag object.
        let compared = compare_remote(&[tag("v2", 'e')], &remote(vec![tag("v2", 'f')]));
        assert_eq!(compared.of("v2"), Some(TagOnRemote::DifferentThere));
    }

    #[test]
    fn tag_compare_remote__names_are_compared_with_their_case() {
        let compared = compare_remote(&[tag("V1", 'a')], &remote(vec![tag("v1", 'a')]));
        assert_eq!(compared.of("V1"), Some(TagOnRemote::NotPushed));
        assert_eq!(compared.only_there, ["v1"]);
    }
}
