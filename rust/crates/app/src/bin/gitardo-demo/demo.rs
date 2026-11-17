//! The demo window: the gallery of the toolkit, every widget of it live, its menus, dialogs, pickers, find bar and
//! drag and drop included, in the theme chosen in its menu or the system's.

use app::AppError;
use app::window::{Step, Surface, SurfaceOptions};
use platform::{Context, Event, LogicalSize, TimerId, Visibility};
use render::ThemeName;
use ui::gallery::{self, GalleryIntent, GalleryState};

/// The timer of the frames of the demo window.
const FRAME_TIMER: TimerId = TimerId(1);

/// Whether the loop goes on after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// It goes on.
    Continue,
    /// The window was closed: the loop is to stop.
    Quit,
}

/// The demo window and what the gallery in it shows.
pub struct Demo {
    /// The window.
    surface: Surface,
    /// The state of the gallery.
    state: GalleryState,
}

impl Demo {
    /// Opens the demo window, in the theme of the system, with the gallery in its fixed starting state, once the loop
    /// runs.
    ///
    /// # Errors
    ///
    /// The errors of [`Surface::open`].
    pub fn open(context: &Context<'_>, visibility: Visibility) -> Result<Demo, AppError> {
        let options = SurfaceOptions {
            title: String::from("Gitardo Desktop widgets"),
            size: LogicalSize {
                width: 1280.0,
                height: 800.0,
            },
            visibility,
            theme: ThemeName::System,
            frame_timer: FRAME_TIMER,
            title_bar: views::window::TitleBar::System,
            position: None,
            is_maximized: false,
        };
        let mut surface = Surface::open(context, options)?;
        let theme = *surface.theme();
        gallery::prepare(surface.memory_mut(), &theme);
        Ok(Demo {
            surface,
            state: GalleryState::default(),
        })
    }

    /// Takes an event of the loop: draws a frame when one is due, and says whether to stop.
    ///
    /// # Errors
    ///
    /// The errors of [`Surface::draw`].
    pub fn handle(&mut self, context: &Context<'_>, event: &Event<()>) -> Result<Flow, AppError> {
        match self.surface.handle(event) {
            Step::Draw => self.draw(context)?,
            Step::Close => return Ok(Flow::Quit),
            Step::Handled | Step::Ignored => {}
        }
        Ok(Flow::Continue)
    }

    /// The window.
    #[allow(
        dead_code,
        reason = "the tests of the demo look at the window, the demo itself does not"
    )]
    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    /// What the gallery shows.
    #[allow(
        dead_code,
        reason = "the tests of the demo look at the gallery, the demo itself does not"
    )]
    pub fn state(&self) -> &GalleryState {
        &self.state
    }

    /// Draws a frame of the gallery and applies what the person asked for in it, a theme chosen included.
    fn draw(&mut self, context: &Context<'_>) -> Result<(), AppError> {
        let state = self.state.clone();
        let intents = self.surface.draw(context, |ui| gallery::view(&state, ui))?;
        for intent in &intents {
            if let GalleryIntent::ChoseTheme(theme) = intent {
                self.surface.set_theme(*theme);
            }
            self.state.apply(intent);
        }
        if !intents.is_empty() {
            self.surface.window().request_redraw();
        }
        Ok(())
    }
}
