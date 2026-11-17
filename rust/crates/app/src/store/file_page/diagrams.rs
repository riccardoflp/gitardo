//! The diagrams of Mermaid of the preview of a Markdown file as the store keeps them (`file-view.mermaid`): asked in
//! the theme the window draws the preview in, drawn on a worker, and kept for their text and their theme while the page
//! is open, so that laying the preview out again, or coming back to a theme, draws nothing again.

use render::ThemeName;
use views::file_view::{DiagramFailure, DiagramShown, DrawnDiagram, FilePageId};

use super::{FilePage, page_mut};
use crate::store::message::{Effect, JobId, JobKind};
use crate::store::state::State;
use crate::work::diagram::DiagramJob;

/// The diagrams of a preview: those drawn, or not, and those being drawn.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Diagrams {
    /// What came of each diagram drawn, by its text and its theme.
    kept: Vec<(DiagramJob, DiagramShown)>,
    /// The drawings running, each with what it draws.
    drawing: Vec<(JobId, DiagramJob)>,
}

/// Starts the drawing of the diagrams of the preview of a page that are neither kept nor being drawn in the theme it
/// asked them in, and shows those kept.
pub(super) fn draw(state: &mut State, id: FilePageId) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let folder = page.top.clone();
    let Some(preview) = page.markdown.as_mut() else {
        return Vec::new();
    };
    let Some(theme) = preview.content.diagram_theme else {
        return Vec::new();
    };
    let wanted: Vec<DiagramJob> = model::markdown::mermaid_sources(&preview.content.blocks)
        .into_iter()
        .map(|source| DiagramJob {
            source: source.to_owned(),
            theme,
        })
        .filter(|job| {
            let diagrams = &preview.diagrams;
            !diagrams.kept.iter().any(|(kept, _)| kept == job)
                && !diagrams.drawing.iter().any(|(_, drawing)| drawing == job)
        })
        .collect();
    show(preview);
    wanted
        .into_iter()
        .map(|job| {
            let started = crate::store::workspace::start_job(
                state,
                JobKind::DrawDiagram {
                    folder: folder.clone(),
                    job: job.clone(),
                },
            );
            if let Some(preview) = page_mut(state, id).and_then(|page| page.markdown.as_mut()) {
                preview.diagrams.drawing.push((started.id, job));
            }
            Effect::Start(started)
        })
        .collect()
}

/// Takes a diagram drawn, or why it is not, keeps it, and shows it when its preview asked for its theme.
pub(super) fn drawn(state: &mut State, job: JobId, diagram: Result<DrawnDiagram, DiagramFailure>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    for page in &mut state.file_pages.pages {
        let Some(preview) = page.markdown.as_mut() else {
            continue;
        };
        let Some(index) = preview.diagrams.drawing.iter().position(|(drawing, _)| *drawing == job) else {
            continue;
        };
        let (_, asked) = preview.diagrams.drawing.remove(index);
        let shown = match diagram {
            Ok(drawn) => DiagramShown::Drawn(drawn),
            Err(failure) => DiagramShown::NotDrawn(failure),
        };
        preview.diagrams.kept.push((asked, shown));
        show(preview);
        break;
    }
    Vec::new()
}

/// The drawings of the diagrams of a page, to cancel when it closes.
pub(super) fn jobs(page: &FilePage) -> Vec<JobId> {
    page.markdown
        .as_ref()
        .map(|preview| preview.diagrams.drawing.iter().map(|(job, _)| *job).collect())
        .unwrap_or_default()
}

/// What the preview shows of its diagrams: those kept in the theme it asked for, of the texts it has.
fn show(preview: &mut super::Preview) {
    let theme: Option<ThemeName> = preview.content.diagram_theme;
    let sources = model::markdown::mermaid_sources(&preview.content.blocks);
    preview.content.diagrams = preview
        .diagrams
        .kept
        .iter()
        .filter(|(kept, _)| Some(kept.theme) == theme && sources.contains(&kept.source.as_str()))
        .map(|(kept, shown)| (kept.source.clone(), shown.clone()))
        .collect();
}
