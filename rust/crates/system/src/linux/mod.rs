//! Linux: the secrets, kept in a file only the user may read until Gitardo speaks to the Secret Service.

mod credentials;

pub(crate) use credentials::{delete_secret, read_secret, write_secret};
