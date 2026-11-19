//! The marks of the tags a remote lacks (`tag.remote-marks`): after the name of a tag in the branch rail, among the
//! icons of its label on the timeline, and after the count of the **Tags** list, once the remote was checked.

use model::CommitId;
use model::state::Tag;
use model::tag::{TagMarks, TagOnRemote};
use ui::Icon;

use crate::icons;
use crate::text::rail as words;

#[cfg(test)]
mod tests;

/// The mark of a tag against the remote last checked: its icon and its tooltip, `Not on origin` with `cloud-off` or
/// `origin has another tag of this name` with `cloud-alert`; none for a tag the remote has as here, and none for any tag
/// before **Check tags on `<remote>`** was chosen.
///
/// Feature: tag.remote-marks
#[must_use]
pub fn remote_marks_view(state: Option<&TagMarks>, (name, commit): (&str, &CommitId)) -> Option<(Icon, String)> {
    let marks = state?;
    match marks.at(name, commit) {
        TagOnRemote::Pushed => None,
        TagOnRemote::NotPushed => Some((icons::NOT_ON_REMOTE, words::not_on(&marks.remote))),
        TagOnRemote::DifferentThere => Some((icons::DIFFERENT_ON_REMOTE, words::other_tag_on(&marks.remote))),
    }
}

/// The icons of the label of a tag: the tag, then the mark it has.
#[must_use]
pub(crate) fn label_icons(mark: Option<Icon>) -> &'static [Icon] {
    match mark {
        Some(icons::NOT_ON_REMOTE) => &[icons::TAG, icons::NOT_ON_REMOTE],
        Some(icons::DIFFERENT_ON_REMOTE) => &[icons::TAG, icons::DIFFERENT_ON_REMOTE],
        _ => &[icons::TAG],
    }
}

/// What follows the count of the **Tags** list: `· 2 not on origin` while the remote checked lacks some tags.
#[must_use]
pub(crate) fn not_pushed_words(state: Option<&TagMarks>, tags: &[Tag]) -> Option<String> {
    let marks = state?;
    let count = marks.not_pushed(tags);
    (count > 0).then(|| words::not_on_count(count, &marks.remote))
}
