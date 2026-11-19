//! The tags a remote has, asked of the remote over the network for an operation that needs them before its pick
//! (`tag.read-remote-tags`, for `tag.push-all`): read as the other queries are, and tracked as an activity, which says
//! **Asking `<remote>` for its tags** while the remote takes its time.

use std::time::Instant;

use model::contract::{Answer, FeatureId, Query, Text};

use super::Engine;
use crate::queries;

impl Engine {
    /// The tags of a remote and of the repository, read now, the read logged with how long it took.
    pub(super) fn remote_tags(&mut self, feature: FeatureId, query: &Query) -> Result<Answer, git::GitError> {
        let description = match query {
            Query::RemoteTags { remote } => format!("Asking {remote} for its tags"),
            _ => String::from("Asking a remote for its tags"), // Only the tags of a remote come here.
        };
        let _asking = self
            .activities
            .track_read(Text::from(description), Text::from(feature.0), self.clock.now());
        let started = Instant::now();
        let answer = queries::answer(&self.git, query)?;
        self.log
            .record_read(self.clock.now(), queries::describe(query), started.elapsed());
        Ok(answer)
    }
}
