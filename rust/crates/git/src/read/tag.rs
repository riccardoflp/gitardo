//! The tags a remote has, asked of the remote itself, with those of the repository (`tag.read-remote-tags`): a fetch
//! brings the tags of the commits it fetches, but never says which tags of the repository the remote lacks.

use model::tag::{RemoteTags, TagObject};

use crate::{Git, GitError};

/// Which remote to ask.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteTagsRequest {
    /// The remote, by its name: `origin`.
    pub remote: String,
}

/// `git ls-remote --tags --refs <remote>`: one line for each tag of the remote, its object, a tab and its full name.
/// `--refs` leaves out the lines of the commits of annotated tags, so that such a tag names its tag object.
#[must_use]
pub fn remote_tags_arguments(request: &RemoteTagsRequest) -> Vec<String> {
    ["ls-remote", "--tags", "--refs", request.remote.as_str()]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// `git for-each-ref --format=%(objectname)%00%(refname)%00 refs/tags`: each tag of the repository with the object it
/// names, as `ls-remote` names it.
#[must_use]
pub fn local_tags_arguments() -> Vec<&'static str> {
    vec!["for-each-ref", "--format=%(objectname)%00%(refname)%00", "refs/tags"]
}

/// The tags of what [`remote_tags_arguments`] printed. A name holds no tab and no line break, so a line is read as it
/// is.
#[must_use]
pub fn parse_remote_tags(output: &[u8]) -> Vec<TagObject> {
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter_map(|(object, reference)| {
            let name = reference.strip_prefix("refs/tags/")?;
            Some(TagObject {
                name: name.to_owned(),
                object: object.to_owned(),
            })
        })
        .collect()
}

/// The tags of what [`local_tags_arguments`] printed.
#[must_use]
pub fn parse_local_tags(output: &[u8]) -> Vec<TagObject> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text.split('\0').map(|field| field.trim_start_matches('\n'));
    let mut tags = Vec::new();
    while let (Some(object), Some(reference)) = (fields.next(), fields.next()) {
        if let Some(name) = reference.strip_prefix("refs/tags/") {
            tags.push(TagObject {
                name: name.to_owned(),
                object: object.to_owned(),
            });
        }
    }
    tags
}

/// The tags a remote has and those of the repository, each with the object it names. Git is told never to ask in a
/// terminal, so that a remote that wants credentials fails with what git said.
///
/// Feature: tag.read-remote-tags
///
/// # Errors
///
/// When git cannot be started, or fails: a remote that cannot be reached, or that refuses.
pub fn read_remote_tags(git: &Git, request: &RemoteTagsRequest) -> Result<RemoteTags, GitError> {
    let asking = git.clone().with_variable("GIT_TERMINAL_PROMPT", "0");
    let there = parse_remote_tags(&asking.run_read(&remote_tags_arguments(request))?);
    let here = parse_local_tags(&git.run_read(&local_tags_arguments())?);
    Ok(RemoteTags {
        remote: request.remote.clone(),
        there,
        here,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_local_tags, parse_remote_tags};

    #[test]
    fn the_lines_of_ls_remote_and_the_fields_of_for_each_ref_give_each_tag_and_its_object() {
        let remote = parse_remote_tags(b"aaaa\trefs/tags/v1\nbbbb\trefs/tags/release/v2\n");
        let names: Vec<&str> = remote.iter().map(|tag| tag.name.as_str()).collect();
        assert_eq!(names, ["v1", "release/v2"]);
        let here = parse_local_tags(b"cccc\0refs/tags/v1\0\ndddd\0refs/tags/v3\0\n");
        assert_eq!(here.len(), 2);
        assert_eq!((here[1].name.as_str(), here[1].object.as_str()), ("v3", "dddd"));
    }
}
