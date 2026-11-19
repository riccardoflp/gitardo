//! Ask Gitardo (decision D76): the guide of the features the model is told of (`help.guide`), the two requests of a
//! question, the answer read, and what **Show me** does with each feature it names.

mod guide;

pub use guide::{App, Guide, GuideCard, Show, guide};
