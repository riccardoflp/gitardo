//! The picture of an image file in its File page as the store keeps it (`file-view.image`): drawn on a worker from the
//! bytes read, or for an SVG from its text as it is edited, drawn again only when the bytes change; its zoom and where
//! it is moved to, kept while the file is read again; and **Preview** and **Source** of an SVG.

use render::image::{ImageFormat, ImageRead, NotRead, image_format};
use views::file_view::{FilePageId, ImageIntent, ImagePlace, ImageShown};

use super::{FilePage, next_serial, page_mut};
use crate::store::message::{Effect, JobId, JobKind};
use crate::store::state::State;

/// The picture of the page of an image file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImagePage {
    /// The format the file says it is.
    pub format: ImageFormat,
    /// What was drawn, or why it could not be; none while it is drawn the first time.
    pub drawn: Option<Result<ImageRead, NotRead>>,
    /// Where the picture is, kept when the file is read again.
    pub place: ImagePlace,
    /// Whether an SVG shows its picture rather than its text; it opens in **Preview** (Q840).
    pub is_preview: bool,
    /// The bytes last drawn, or being drawn, so that the same bytes are not drawn again.
    bytes: Vec<u8>,
    /// The drawing running.
    job: Option<JobId>,
}

impl ImagePage {
    /// What the page shows of it.
    #[must_use]
    pub fn shown(&self) -> ImageShown<'_> {
        match &self.drawn {
            None => ImageShown::Reading,
            Some(Ok(read)) => ImageShown::Drawn(read),
            Some(Err(not_read)) => ImageShown::NotRead(not_read),
        }
    }

    /// Whether the page shows the picture, rather than the text of an SVG.
    #[must_use]
    pub fn is_picture_shown(&self) -> bool {
        self.format != ImageFormat::Svg || self.is_preview
    }
}

/// Takes the bytes of a file read: an image file gets its picture, drawn when its bytes changed; a file that is not
/// one has none.
pub(super) fn take_bytes(state: &mut State, id: FilePageId, bytes: &[u8]) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let Some(format) = image_format(bytes, &page.path) else {
        page.image = None;
        return Vec::new();
    };
    let image = page.image.get_or_insert_with(|| ImagePage {
        format,
        drawn: None,
        place: ImagePlace::default(),
        is_preview: true,
        bytes: Vec::new(),
        job: None,
    });
    image.format = format;
    if image.is_picture_shown() {
        draw(state, id, bytes.to_vec())
    } else {
        Vec::new()
    }
}

/// Starts the drawing of the bytes of a page, unless they are drawn already.
fn draw(state: &mut State, id: FilePageId, bytes: Vec<u8>) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let Some(image) = page.image.as_mut() else {
        return Vec::new();
    };
    if image.bytes == bytes && (image.drawn.is_some() || image.job.is_some()) {
        return Vec::new();
    }
    image.bytes.clone_from(&bytes);
    let kind = JobKind::DrawImage {
        folder: page.top.clone(),
        path: page.path.clone(),
        bytes,
    };
    let previous = image.job.take();
    let job = crate::store::workspace::start_job(state, kind);
    if let Some(image) = page_mut(state, id).and_then(|page| page.image.as_mut()) {
        image.job = Some(job.id);
    }
    let mut effects = Vec::new();
    if let Some(previous) = previous {
        let _cancelled = state.jobs.remove(&previous);
        effects.push(Effect::Cancel(previous));
    }
    effects.push(Effect::Start(job));
    effects
}

/// **Preview** and **Source** of an SVG, and the picture zoomed or moved.
pub fn reduce(state: &mut State, id: FilePageId, intent: ImageIntent) -> Vec<Effect> {
    let serial = next_serial(state);
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    // The picture is drawn from the text as it is now, edited or not.
    let bytes = page
        .document()
        .map_or_else(|| page.read.clone(), |document| document.buffer().to_bytes());
    let Some(image) = page.image.as_mut() else {
        return Vec::new();
    };
    match intent {
        ImageIntent::Place(place) => {
            image.place = place;
            Vec::new()
        }
        ImageIntent::ShowSource => {
            image.is_preview = false;
            page.focus = Some(serial);
            Vec::new()
        }
        ImageIntent::ShowPreview => {
            image.is_preview = true;
            draw(state, id, bytes)
        }
    }
}

/// Takes a picture drawn: shown when its page still waits for it.
pub fn image_drawn(state: &mut State, job: JobId, result: Result<ImageRead, NotRead>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let found = state
        .file_pages
        .pages
        .iter_mut()
        .filter_map(|page| page.image.as_mut())
        .find(|image| image.job == Some(job));
    if let Some(image) = found {
        image.job = None;
        image.drawn = Some(result);
    }
    Vec::new()
}

/// The drawing of a page, to cancel when it closes.
pub(super) fn job(page: &FilePage) -> Option<JobId> {
    page.image.as_ref().and_then(|image| image.job)
}
