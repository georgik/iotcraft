use crate::localization::{Language, LocalizationBundle, LocalizationConfig};
use bevy::prelude::*;

/// Helper functions for working with localization resources
impl LocalizationBundle {
    /// Get text using the current language from config with fallback support
    pub fn get_current_text(
        &self,
        config: &LocalizationConfig,
        key: &str,
        args: &[(String, String)],
    ) -> String {
        self.get_text_with_fallback(config.current_language, config.fallback_language, key, args)
    }
}

/// System helper to get localized text from components
pub fn get_localized_text(
    bundle: &LocalizationBundle,
    config: &LocalizationConfig,
    key: &str,
    args: &[(String, String)],
) -> String {
    bundle.get_current_text(config, key, args)
}

/// Trait to extend LocalizationConfig with helper methods
pub trait LocalizationConfigHelper {
    /// Set the current language and return true if it changed
    fn set_language(&mut self, language: Language) -> bool;

    /// Get the current language
    #[allow(dead_code)]
    fn current_language(&self) -> Language;

    /// Get the fallback language
    #[allow(dead_code)]
    fn fallback_language(&self) -> Language;
}

impl LocalizationConfigHelper for LocalizationConfig {
    fn set_language(&mut self, language: Language) -> bool {
        if self.current_language != language {
            self.current_language = language;
            true
        } else {
            false
        }
    }

    fn current_language(&self) -> Language {
        self.current_language
    }

    fn fallback_language(&self) -> Language {
        self.fallback_language
    }
}
