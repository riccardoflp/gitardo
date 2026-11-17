//! The preview of a Markdown file as the store keeps it (`file-view.markdown-preview`): whether the page shows it,
//! its blocks laid out again from the text each time it is shown or the file is read again, the colors of its code, the
//! images of the repository it shows, read on a worker, and where its links go.

use model::markdown::{self, LinkTarget};
use views::file_view::{FilePageId, FileTarget, MarkdownContent, MarkdownIntent, MarkdownShown};

use super::{FilePage, colors_of, next_serial, page_mut};
use crate::store::message::{Effect, JobId, JobKind};
use crate::store::state::State;
use crate::work::file_view::FileJob;

/// The preview of the page of a Markdown file.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Preview {
    /// Whether the page shows it, rather than the text.
    pub is_shown: bool,
    /// What it lays out.
    pub content: MarkdownContent,
    /// The images asked for, by their source as the file writes it: read, being read, or not of the repository.
    asked: Vec<String>,
    /// The reads of images running, each with the source it is for.
    reading: Vec<(JobId, String)>,
}

/// The preview a page of a file opens with: shown when `markdown.openAsPreview` is on, none for a file that is not
/// Markdown.
#[must_use]
pub fn preview_of(path: &str, is_shown: bool) -> Option<Preview> {
    markdown::is_markdown(path).then(|| Preview {
        is_shown,
        ..Preview::default()
    })
}

/// What the page shows of its file: none for a file that is not Markdown.
#[must_use]
pub fn shown(page: &FilePage) -> Option<MarkdownShown<'_>> {
    let preview = page.markdown.as_ref()?;
    Some(if preview.is_shown {
        MarkdownShown::Preview(&preview.content)
    } else {
        MarkdownShown::Source
    })
}

/// **Preview**, **Source**, or a click on a link of the preview.
pub fn reduce(state: &mut State, id: FilePageId, intent: MarkdownIntent) -> Vec<Effect> {
    let serial = next_serial(state);
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    match intent {
        MarkdownIntent::ShowPreview => {
            if let Some(preview) = page.markdown.as_mut() {
                preview.is_shown = true;
            }
            refresh(state, id)
        }
        MarkdownIntent::ShowSource => {
            if let Some(preview) = page.markdown.as_mut() {
                preview.is_shown = false;
            }
            page.focus = Some(serial);
            Vec::new()
        }
        MarkdownIntent::OpenLink(href) => open_link(state, id, &href, serial),
    }
}

/// Goes where a link says: the browser opens a web page, a file of the repository opens in its page, at the revision
/// the page shows when it shows one, and a heading of the file is scrolled to.
fn open_link(state: &mut State, id: FilePageId, href: &str, serial: u64) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    match markdown::link_target(&page.path, href) {
        LinkTarget::Web(url) => vec![Effect::OpenUrl(url)],
        LinkTarget::File { path, line } => {
            let target = FileTarget {
                path,
                revision: page.revision.clone(),
                line,
            };
            super::open(state, target)
        }
        LinkTarget::Heading(anchor) => {
            if let Some(preview) = page.markdown.as_mut() {
                preview.content.reveal = Some((serial, anchor));
            }
            Vec::new()
        }
        LinkTarget::Nowhere => Vec::new(),
    }
}

/// Lays out again what the preview of a page shows from its text, while it is shown, and starts the reads of the
/// images of the repository it has not asked for yet.
pub fn refresh(state: &mut State, id: FilePageId) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let Some(text) = page.document().map(|document| document.buffer().text().to_owned()) else {
        return Vec::new();
    };
    let (path, top, revision) = (page.path.clone(), page.top.clone(), page.revision.clone());
    let Some(preview) = page.markdown.as_mut().filter(|preview| preview.is_shown) else {
        return Vec::new();
    };
    let blocks = markdown::blocks(&text);
    preview.content.code_colors = markdown::code_blocks(&blocks)
        .into_iter()
        .map(|code| colors_of(&markdown::code_file_name(&code.language), &code.text))
        .collect();
    let mut to_read = Vec::new();
    for source in markdown::image_sources(&blocks) {
        if preview.asked.iter().any(|asked| asked == source) {
            continue;
        }
        preview.asked.push(source.to_owned());
        if let Some(image) = markdown::image_path(&path, source) {
            to_read.push((source.to_owned(), image));
        }
    }
    preview.content.blocks = blocks;
    to_read
        .into_iter()
        .map(|(source, image)| {
            let job = FileJob {
                page: id,
                path: image,
                revision: revision.clone(),
            };
            start_image_read(state, (id, source), (top.clone(), job))
        })
        .collect()
}

/// Starts the read of an image of a page, kept with the source it is for.
fn start_image_read(
    state: &mut State,
    (id, source): (FilePageId, String),
    (folder, job): (std::path::PathBuf, FileJob),
) -> Effect {
    let job = crate::store::workspace::start_job(state, JobKind::ReadImage { folder, job });
    if let Some(preview) = page_mut(state, id).and_then(|page| page.markdown.as_mut()) {
        preview.reading.push((job.id, source));
    }
    Effect::Start(job)
}

/// Takes an image read: drawn when it could be read, else its text stays in its place.
pub fn image_read(state: &mut State, job: JobId, image: Option<render::Image>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    for page in &mut state.file_pages.pages {
        let Some(preview) = page.markdown.as_mut() else {
            continue;
        };
        let Some(index) = preview.reading.iter().position(|(reading, _)| *reading == job) else {
            continue;
        };
        let (_, source) = preview.reading.remove(index);
        if let Some(image) = image {
            preview.content.images.push((source, image));
        }
        break;
    }
    Vec::new()
}

/// The reads of images of a page, to cancel when it closes.
pub(super) fn image_jobs(page: &FilePage) -> Vec<JobId> {
    page.markdown
        .as_ref()
        .map(|preview| preview.reading.iter().map(|(job, _)| *job).collect())
        .unwrap_or_default()
}
