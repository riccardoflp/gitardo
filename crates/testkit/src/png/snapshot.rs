//! Comparing a drawn image with its reference image, for the snapshots of the renderer, the toolkit and the views.

use std::path::{Path, PathBuf};

use super::image::Image;
use super::read::read_file;
use super::write::write_file;

/// The environment variable that records the actual image as the reference, as for golden files.
const RECORD_VARIABLE: &str = "GITARDO_GOLDEN_RECORD";

/// Checks that an image has the pixels of its reference, a PNG file. When it does not, writes the actual image as
/// `<name>.actual.png` and the differences as `<name>.difference.png` beside the reference, and fails. With
/// `GITARDO_GOLDEN_RECORD=1` set, writes the actual image as the reference instead, making its folders.
///
/// # Panics
///
/// When the image differs from its reference, when there is no reference, or when an image cannot be read or written.
pub fn assert_snapshot(reference: impl AsRef<Path>, actual: &Image) {
    let reference = reference.as_ref();
    let beside = |suffix: &str| -> PathBuf {
        let name = reference.file_stem().unwrap_or_default().to_string_lossy();
        reference.with_file_name(format!("{name}.{suffix}.png"))
    };
    let (actual_path, difference_path) = (beside("actual"), beside("difference"));
    if std::env::var_os(RECORD_VARIABLE).is_some_and(|value| value == "1") {
        write_file(reference, actual).unwrap_or_else(|error| panic!("{error}"));
        remove(&[actual_path, difference_path]);
        return;
    }
    let expected = read_file(reference).unwrap_or_else(|error| {
        panic!(
            "{error}\nRun the test with {RECORD_VARIABLE}=1 to record {}",
            reference.display()
        )
    });
    let Some(difference) = actual.difference_from(&expected) else {
        remove(&[actual_path, difference_path]);
        return;
    };
    write_file(&actual_path, actual).unwrap_or_else(|error| panic!("{error}"));
    write_file(&difference_path, &difference.image).unwrap_or_else(|error| panic!("{error}"));
    panic!(
        "{} pixels differ from {}; the actual image is {} and the differences are {}",
        difference.differing_pixels,
        reference.display(),
        actual_path.display(),
        difference_path.display()
    );
}

/// Removes the images a mismatch left, when there are any.
fn remove(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}
