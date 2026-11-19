//! The images of the pages of commits as the store keeps them (`commit-page.images`): the two versions of an image
//! file read on a worker once its diff is shown unfolded, never while it is folded, and kept while the page is open;
//! and **Show the images** and **Show the lines** of an SVG.

use git::read::diff::{DiffRequest, FileVersion, VersionRequest};
use model::commit_page::PatchLines;
use model::details::CommitDetail;
use model::state::FileChange;
use views::commit_page::{CommitPageId, PageImage};
use views::diff::{DiffImages, DiffVersion};

use super::commit_page::{CommitPage, PageReadState};
use super::message::{Effect, JobId, JobKind};
use super::state::State;

/// Starts the reads of the image files of a page whose diffs are shown, unfolded, and not read yet.
pub(super) fn refresh(state: &mut State, id: CommitPageId) -> Vec<Effect> {
    let Some(page) = state.commit_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let wanted = to_read(page);
    let folder = page.folder.clone();
    let mut effects = Vec::new();
    for (file, request) in wanted {
        let kind = JobKind::ReadImages {
            folder: folder.clone(),
            request,
        };
        let job = super::workspace::start_job(state, kind);
        if let Some(page) = state.commit_pages.pages.iter_mut().find(|page| page.id == id) {
            let _reading = page.images.files.insert(file, PageImage::Reading);
            let _started = page.image_jobs.insert(file, job.id);
        }
        effects.push(Effect::Start(job));
    }
    effects
}

/// The image files of a page to read now, with the two versions of each.
fn to_read(page: &CommitPage) -> Vec<(usize, DiffRequest)> {
    let PageReadState::Read(data) = &page.read else {
        return Vec::new();
    };
    let PatchLines::Files(patches) = &data.patch else {
        return Vec::new();
    };
    if !page.shows_changes {
        return Vec::new();
    }
    let shown = if page.is_all_files_shown {
        patches.len()
    } else {
        patches.len().min(views::details::FILE_CAP)
    };
    patches
        .iter()
        .enumerate()
        .take(shown)
        .filter(|(index, _)| !page.folds.is_folded(*index) && !page.images.files.contains_key(index))
        .filter_map(|(index, patch)| {
            let file = data.detail.files.get(index)?;
            page.images
                .shows(index, patch, &file.path)
                .then(|| (index, request_of(&data.detail, file)))
        })
        .collect()
}

/// The two versions of a file of a commit: in its first parent and in the commit, none on a side it is not on.
fn request_of(detail: &CommitDetail, file: &FileChange) -> DiffRequest {
    let target = views::diff::commit_file(detail, file);
    let version = |side: &DiffVersion| match side {
        DiffVersion::Commit { commit, .. } => FileVersion::Commit(commit.clone()),
        _ => FileVersion::Absent,
    };
    let paths = target
        .files
        .first()
        .cloned()
        .unwrap_or_else(|| views::diff::DiffFile::same(&file.path));
    DiffRequest {
        left: VersionRequest {
            version: version(&target.left),
            path: paths.left,
        },
        right: VersionRequest {
            version: version(&target.right),
            path: paths.right,
        },
    }
}

/// Takes the two versions of an image file read: shown in its page when it is still open.
pub fn images_read(state: &mut State, job: JobId, images: Option<Box<DiffImages>>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let found = state.commit_pages.pages.iter_mut().find_map(|page| {
        let file = page
            .image_jobs
            .iter()
            .find(|(_, running)| **running == job)
            .map(|(file, _)| *file)?;
        Some((page, file))
    });
    if let Some((page, file)) = found {
        let _done = page.image_jobs.remove(&file);
        let _read = page.images.files.insert(file, PageImage::Read(images));
        page.version += 1;
    }
    Vec::new()
}

/// **Show the images** of an SVG, or **Show the lines** back.
pub(super) fn show_images(page: &mut CommitPage, file: usize, shows: bool) {
    if shows {
        let _shown = page.images.svg_shown.insert(file);
    } else {
        let _hidden = page.images.svg_shown.remove(&file);
    }
}
