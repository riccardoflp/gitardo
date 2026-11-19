//! Ask Gitardo (decision D76): the guide of the features the model is told of (`help.guide`), the two requests of a
//! question (`help.choose-cards`, `help.question`), the answer read (`help.read-answer`), and what **Show me** does
//! with each feature it names.

mod answer;
mod guide;
mod requests;
mod show_me;

pub use answer::{HelpAnswer, SHOW_ME_LINK, read_answer, shown_feature};
pub use guide::{App, Guide, GuideCard, Show, guide};
pub use requests::{Exchange, HelpInput, MOST_CARDS, choose_cards, question, read_chosen};
pub use show_me::{MOST_BUTTONS, Place, ShowMe, Showing, buttons, show_me};

/// What Ask Gitardo says when no AI provider is set up.
pub const NO_PROVIDER: &str = "Set up an AI provider to ask how to do something.";
