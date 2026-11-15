//! The settings of the features from what the extension sends: the object of `initialize` and `settings/changed`,
//! whose keys are those of Gitardo Desktop's `settings.json`, read with the code that reads that file (crate
//! `settings`), as `docs/transition/02-protocol.md` says under Settings.

use model::contract::FeatureSettings;
use serde_json::{Map, Value};

/// The settings of the features, with the offset of the local time the extension says (`utcOffsetMinutes`). A value of
/// the wrong kind is left for its default, as in the desktop's file.
pub fn feature_settings(settings: &Map<String, Value>) -> FeatureSettings {
    let text = serde_json::to_string(settings).unwrap_or_default();
    let mut lookup = ::settings::Lookup::parse(Some(&text), None);
    let mut features = ::settings::feature_settings(&mut lookup);
    features.utc_offset = settings
        .get("utcOffsetMinutes")
        .and_then(Value::as_i64)
        .map_or(0, |minutes| minutes.saturating_mul(60));
    features
}

#[cfg(test)]
mod tests {
    use super::feature_settings;
    use model::contract::CommandPreview;
    use serde_json::json;

    #[test]
    fn the_settings_of_the_extension_are_read_by_the_desktops_names() {
        let Some(settings) = json!({
            "branches.format": "${input:ticket}-${base}",
            "submodules.updateOnCheckout": false,
            "commandPreview": "all",
            "host": "vscode",
            "ai.provider": "vscode",
            "utcOffsetMinutes": 120
        })
        .as_object()
        .cloned() else {
            panic!("an object")
        };
        let features = feature_settings(&settings);
        assert_eq!(features.branch_format, "${input:ticket}-${base}");
        assert!(!features.update_submodules_on_checkout);
        assert_eq!(features.command_preview, CommandPreview::All);
        assert_eq!(features.utc_offset, 7_200);
    }

    #[test]
    fn no_settings_are_the_defaults() {
        let features = feature_settings(&serde_json::Map::new());
        assert!(features.update_submodules_on_checkout);
        assert_eq!(features.branch_format, "");
        assert_eq!(features.utc_offset, 0);
    }
}
