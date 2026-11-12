//! The reference an operation on branches or tags was invoked on, found in the repository as last read.

use model::contract::{Context, Outcome, Refusal, Target};
use model::state::{Branch, RemoteBranch, Tag};

/// The local branch the operation was invoked on, or the refusal to return when it is not one the repository has.
pub(crate) fn local_branch(context: &Context) -> Result<&Branch, Box<Outcome>> {
    let Target::Branch(name) = &context.target else {
        return Err(Box::new(refuse("Choose a branch first.")));
    };
    context
        .repository
        .branches
        .iter()
        .find(|branch| branch.name == *name)
        .ok_or_else(|| Box::new(refuse(&format!("There is no branch {name}."))))
}

/// The remote branch the operation was invoked on, or the refusal to return.
pub(crate) fn remote_branch(context: &Context) -> Result<&RemoteBranch, Box<Outcome>> {
    let Target::RemoteBranch(name) = &context.target else {
        return Err(Box::new(refuse("Choose a remote branch first.")));
    };
    context
        .repository
        .remote_branches
        .iter()
        .find(|branch| branch.name == *name)
        .ok_or_else(|| Box::new(refuse(&format!("There is no remote branch {name}."))))
}

/// The tag the operation was invoked on, or the refusal to return.
pub(crate) fn tag(context: &Context) -> Result<&Tag, Box<Outcome>> {
    let Target::Tag(name) = &context.target else {
        return Err(Box::new(refuse("Choose a tag first.")));
    };
    context
        .repository
        .tags
        .iter()
        .find(|tag| tag.name == *name)
        .ok_or_else(|| Box::new(refuse(&format!("There is no tag {name}."))))
}

/// An outcome that refuses, saying why.
pub(crate) fn refuse(message: &str) -> Outcome {
    Outcome::Refuse(Refusal::saying(message))
}
