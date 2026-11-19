//! The files a repository stores with Git LFS (D84): a file left as a pointer shown in the pages, with **Download**
//! (`lfs.show-in-pages`).

mod stored;

pub use stored::{StoredWithLfsState, download_intent, stored_height, stored_with_lfs};
