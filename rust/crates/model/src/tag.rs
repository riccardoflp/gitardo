//! The tags of a repository against those of a remote (`tag.read-remote-tags`, `tag.compare-remote`): which the remote
//! has as here, under another object, or not at all; and the marks the last check left (`tag.remote-marks`).

use crate::CommitId;
use crate::state::Tag;

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

/// What an operation of Gitardo changed of the tags on a remote, without asking it (`tag.remote-marks`): the tags
/// it pushed, and those it made here or deleted from the remote, which the remote does not have after it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagsOnRemote {
    /// The remote; none for a tag made here, which no remote has.
    pub remote: Option<String>,
    /// The tags pushed to it.
    pub pushed: Vec<String>,
    /// The tags it does not have after the operation.
    pub not_pushed: Vec<String>,
}

impl TagsOnRemote {
    /// Tags pushed to a remote.
    #[must_use]
    pub fn pushed(remote: &str, names: Vec<String>) -> Self {
        Self {
            remote: Some(remote.to_owned()),
            pushed: names,
            not_pushed: Vec::new(),
        }
    }

    /// Tags a remote does not have after the operation: made here when the remote is none, else deleted from it.
    #[must_use]
    pub fn not_pushed(remote: Option<&str>, names: Vec<String>) -> Self {
        Self {
            remote: remote.map(str::to_owned),
            pushed: Vec::new(),
            not_pushed: names,
        }
    }
}

/// What the last check of a remote said of each tag of the repository, kept while the window is open
/// (`tag.remote-marks`): where it stood and the commit it was at here then, changed by Gitardo's own operations on tags
/// since, never by a fetch.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagMarks {
    /// The remote checked.
    pub remote: String,
    /// Each tag known to the check or made since.
    pub tags: Vec<TagMark>,
}

/// What is known of a tag against the remote checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagMark {
    /// Its name.
    pub name: String,
    /// Where it stood.
    pub on_remote: TagOnRemote,
    /// The commit it was at here when that was known; none when it was not here.
    pub commit: Option<CommitId>,
}

impl TagMarks {
    /// The marks of a check: each tag of the comparison, at the commit the tags of the repository have it at.
    #[must_use]
    pub fn checked(comparison: &TagComparison, tags: &[Tag]) -> Self {
        let commit_of = |name: &str| tags.iter().find(|tag| tag.name == name).map(|tag| tag.commit.clone());
        Self {
            remote: comparison.remote.clone(),
            tags: comparison
                .here
                .iter()
                .map(|(name, on_remote)| TagMark {
                    name: name.clone(),
                    on_remote: *on_remote,
                    commit: commit_of(name),
                })
                .collect(),
        }
    }

    /// Where a tag of the repository stands on the remote: not pushed when the check did not know it, made since;
    /// different there when it was pushed at another commit than the one it is at now, moved here since.
    #[must_use]
    pub fn of(&self, tag: &Tag) -> TagOnRemote {
        self.at(&tag.name, &tag.commit)
    }

    /// Where the tag of this name, at this commit here, stands on the remote, as [`TagMarks::of`] says.
    #[must_use]
    pub fn at(&self, name: &str, commit: &CommitId) -> TagOnRemote {
        match self.tags.iter().find(|mark| mark.name == name) {
            None => TagOnRemote::NotPushed,
            Some(mark)
                if mark.on_remote == TagOnRemote::Pushed
                    && mark.commit.as_ref().is_some_and(|known| known != commit) =>
            {
                TagOnRemote::DifferentThere
            }
            Some(mark) => mark.on_remote,
        }
    }

    /// How many tags of the repository the remote does not have.
    #[must_use]
    pub fn not_pushed(&self, tags: &[Tag]) -> usize {
        tags.iter().filter(|tag| self.of(tag) == TagOnRemote::NotPushed).count()
    }

    /// Follows what an operation of Gitardo changed of the tags on a remote, when it is the remote checked or any
    /// remote: the tags pushed at the commit the tags of the repository have them at, the others not pushed.
    pub fn follow(&mut self, changed: &TagsOnRemote, tags: &[Tag]) {
        if changed.remote.as_ref().is_some_and(|remote| *remote != self.remote) {
            return;
        }
        for name in &changed.pushed {
            match tags.iter().find(|tag| tag.name == *name) {
                Some(tag) => self.pushed(name, &tag.commit),
                None => self.set(name, TagOnRemote::Pushed, None),
            }
        }
        for name in &changed.not_pushed {
            self.deleted_there(name);
        }
    }

    /// A tag made here by Gitardo: not on the remote.
    pub fn made(&mut self, name: &str) {
        self.set(name, TagOnRemote::NotPushed, None);
    }

    /// A tag pushed by Gitardo, at the commit it is at here: on the remote as here.
    pub fn pushed(&mut self, name: &str, commit: &CommitId) {
        self.set(name, TagOnRemote::Pushed, Some(commit.clone()));
    }

    /// A tag deleted from the remote by Gitardo: not on it any more.
    pub fn deleted_there(&mut self, name: &str) {
        self.set(name, TagOnRemote::NotPushed, None);
    }

    /// Sets what is known of a tag, adding it when the check did not know it.
    fn set(&mut self, name: &str, on_remote: TagOnRemote, commit: Option<CommitId>) {
        let mark = TagMark {
            name: name.to_owned(),
            on_remote,
            commit,
        };
        match self.tags.iter_mut().find(|known| known.name == name) {
            Some(known) => *known = mark,
            None => self.tags.push(mark),
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{RemoteTags, TagMarks, TagObject, TagOnRemote, TagsOnRemote, compare_remote};
    use crate::state::Tag;
    use crate::{CommitId, Timestamp};

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

    fn here(name: &str, commit: char) -> Tag {
        Tag {
            name: name.to_owned(),
            commit: CommitId::parse(&commit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash")),
            date: Timestamp::default(),
        }
    }

    /// The marks of a check of `origin` that has `v1` as here and lacks `v2`.
    fn checked(tags: &[Tag]) -> TagMarks {
        let compared = compare_remote(&[tag("v1", 'a'), tag("v2", 'b')], &remote(vec![tag("v1", 'a')]));
        TagMarks::checked(&compared, tags)
    }

    #[test]
    fn tag_remote_marks__the_check_marks_the_tag_the_remote_lacks_alone() {
        let tags = [here("v1", '1'), here("v2", '2')];
        let marks = checked(&tags);
        assert_eq!(marks.of(&tags[0]), TagOnRemote::Pushed);
        assert_eq!(marks.of(&tags[1]), TagOnRemote::NotPushed);
        assert_eq!(marks.not_pushed(&tags), 1);
    }

    #[test]
    fn tag_remote_marks__a_tag_pushed_loses_its_mark_without_asking_the_remote() {
        let tags = [here("v1", '1'), here("v2", '2')];
        let mut marks = checked(&tags);
        marks.pushed("v2", &tags[1].commit);
        assert_eq!(marks.of(&tags[1]), TagOnRemote::Pushed);
        assert_eq!(marks.not_pushed(&tags), 0);
        marks.deleted_there("v1");
        assert_eq!(marks.of(&tags[0]), TagOnRemote::NotPushed);
    }

    #[test]
    fn tag_remote_marks__a_tag_made_after_the_check_is_not_pushed() {
        let mut marks = checked(&[here("v1", '1'), here("v2", '2')]);
        let made = here("v3", '3');
        assert_eq!(marks.of(&made), TagOnRemote::NotPushed);
        marks.made("v3");
        assert_eq!(marks.of(&made), TagOnRemote::NotPushed);
    }

    #[test]
    fn tag_remote_marks__the_operations_of_gitardo_change_the_marks_of_the_remote_checked_alone() {
        let tags = [here("v1", '1'), here("v2", '2')];
        let mut marks = checked(&tags);
        marks.follow(&TagsOnRemote::pushed("upstream", vec![String::from("v2")]), &tags);
        assert_eq!(marks.of(&tags[1]), TagOnRemote::NotPushed);
        marks.follow(&TagsOnRemote::pushed("origin", vec![String::from("v2")]), &tags);
        assert_eq!(marks.of(&tags[1]), TagOnRemote::Pushed);
        marks.follow(&TagsOnRemote::not_pushed(None, vec![String::from("v1")]), &tags);
        assert_eq!(marks.of(&tags[0]), TagOnRemote::NotPushed);
    }

    #[test]
    fn tag_remote_marks__a_tag_moved_here_after_it_was_pushed_is_different_there() {
        let marks = checked(&[here("v1", '1'), here("v2", '2')]);
        assert_eq!(marks.of(&here("v1", '9')), TagOnRemote::DifferentThere);
    }
}
