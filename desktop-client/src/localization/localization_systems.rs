use crate::localization::{
    Language, LanguageChangeEvent, LocalizationBundle, LocalizationConfig,
    LocalizationConfigHelper, LocalizedText, get_localized_text,
};
use bevy::prelude::*;

/// System to setup localization on startup
pub fn setup_localization(
    mut localization_bundle: ResMut<LocalizationBundle>,
    config: Res<LocalizationConfig>,
) {
    info!("Setting up localization system");

    // Load all supported languages
    for language in Language::all() {
        if let Err(e) = localization_bundle.load_language(language) {
            error!("Failed to load language {:?}: {}", language, e);
        }
    }

    info!(
        "Localization setup complete. Current language: {:?}",
        config.current_language
    );
}

/// System to handle language change events
pub fn handle_language_change(
    mut config: ResMut<LocalizationConfig>,
    mut language_change_events: EventReader<LanguageChangeEvent>,
    localized_text_query: Query<Entity, With<LocalizedText>>,
) {
    for event in language_change_events.read() {
        if config.set_language(event.new_language) {
            info!("Language changed to: {:?}", event.new_language);

            // Trigger update of all localized text
            let count = localized_text_query.iter().count();
            info!("Triggering update for {} localized text components", count);
        }
    }
}

/// System to update localized text when language changes
pub fn update_localized_text(
    localization_bundle: Res<LocalizationBundle>,
    config: Res<LocalizationConfig>,
    mut query: Query<(&mut Text, &LocalizedText)>,
) {
    if config.is_changed() {
        for (mut text, localized_text) in query.iter_mut() {
            let new_text = get_localized_text(
                &*localization_bundle,
                &*config,
                &localized_text.key,
                &localized_text.args,
            );
            text.0 = new_text;
        }
    }
}

/// System to initialize localized text components
pub fn initialize_localized_text(
    localization_bundle: Res<LocalizationBundle>,
    config: Res<LocalizationConfig>,
    mut query: Query<(&mut Text, &LocalizedText), Added<LocalizedText>>,
) {
    for (mut text, localized_text) in query.iter_mut() {
        let localized = get_localized_text(
            &*localization_bundle,
            &*config,
            &localized_text.key,
            &localized_text.args,
        );
        text.0 = localized;
    }
}

/// Helper function to create a text bundle with localization
pub fn create_localized_text_bundle(
    key: impl Into<String>,
    font_size: f32,
    color: Color,
) -> (Text, LocalizedText, TextFont, TextColor) {
    (
        Text::new(""), // Will be filled by the localization system
        LocalizedText::new(key),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
    )
}

/// Helper function to create a text bundle with localization and arguments
pub fn create_localized_text_bundle_with_args(
    key: impl Into<String>,
    args: Vec<(String, String)>,
    font_size: f32,
    color: Color,
) -> (Text, LocalizedText, TextFont, TextColor) {
    (
        Text::new(""), // Will be filled by the localization system
        LocalizedText::with_args(key, args),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
    )
}
