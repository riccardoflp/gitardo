//! The screen reader: what it asks of a window comes back as input, and the window answers with the tree of what is
//! on screen, which `ui` builds with the types of `accesskit`.

use std::sync::Arc;

use accesskit::{ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, TreeUpdate};

use crate::input::InputEvent;
use crate::mailbox::Mailbox;

/// What a screen reader asks of a window.
#[derive(Clone, Debug, PartialEq)]
pub enum AccessibilityRequest {
    /// A screen reader started listening: the window is to give the whole tree of what is on screen, with
    /// `Window::update_accessibility`, and its changes from then on.
    InitialTree,
    /// The person asked, through the screen reader, for an action on a part of the tree: a click, the focus, a scroll.
    Action(ActionRequest),
    /// No screen reader listens any more: the tree need not be kept up to date.
    Deactivated,
}

/// What the adapter of a window calls, from any thread, when a screen reader asks something: the request is handed
/// to the window's event loop as input.
pub(crate) struct Requests {
    inbox: Arc<Mailbox<InputEvent>>,
}

impl Requests {
    /// Requests handed to the inbox of a window.
    pub(crate) fn new(inbox: &Arc<Mailbox<InputEvent>>) -> Requests {
        Requests {
            inbox: Arc::clone(inbox),
        }
    }

    /// Hands a request to the window's loop. A window already closed has no one to answer: the request is dropped.
    fn hand_over(&self, request: AccessibilityRequest) {
        let _ = self.inbox.post(InputEvent::Accessibility(request));
    }
}

impl ActivationHandler for Requests {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // The tree is built on the thread of the loop, which may be busy: it is given later, as the adapter allows.
        self.hand_over(AccessibilityRequest::InitialTree);
        None
    }
}

impl ActionHandler for Requests {
    fn do_action(&mut self, request: ActionRequest) {
        self.hand_over(AccessibilityRequest::Action(request));
    }
}

impl DeactivationHandler for Requests {
    fn deactivate_accessibility(&mut self) {
        self.hand_over(AccessibilityRequest::Deactivated);
    }
}
