//! The fonts that come with the program (D18): Inter for the interface and `JetBrains Mono` for hashes and code, both
//! under the SIL Open Font License, whose text is kept beside them in `crates/render/fonts/`. They are part of the
//! program itself, so that text looks and measures the same on every system and no file is read to draw it.

use std::sync::Arc;

use cosmic_text::fontdb;

/// The name of the family of the interface font.
pub(crate) const INTERFACE_FAMILY: &str = "Inter";

/// The name of the family of the font for hashes and code.
pub(crate) const CODE_FAMILY: &str = "JetBrains Mono";

/// The faces of the two families, as their files.
const FACES: [&[u8]; 6] = [
    include_bytes!("../../fonts/Inter-Regular.ttf"),
    include_bytes!("../../fonts/Inter-Medium.ttf"),
    include_bytes!("../../fonts/Inter-SemiBold.ttf"),
    include_bytes!("../../fonts/Inter-Bold.ttf"),
    include_bytes!("../../fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../../fonts/JetBrainsMono-Bold.ttf"),
];

/// A database of fonts holding the faces that come with the program, and nothing of the system.
pub(crate) fn database() -> fontdb::Database {
    let mut database = fontdb::Database::new();
    for face in FACES {
        let _ = database.load_font_source(fontdb::Source::Binary(Arc::new(face)));
    }
    database.set_sans_serif_family(INTERFACE_FAMILY);
    database.set_monospace_family(CODE_FAMILY);
    database
}
