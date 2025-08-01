use bevy::prelude::*;
use unic_langid::LanguageIdentifier;

/// Supported languages in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Spanish,
    German,
}

impl Language {
    /// Get the language identifier for this language
    pub fn language_id(self) -> LanguageIdentifier {
        match self {
            Language::English => "en-US".parse().unwrap(),
            Language::Spanish => "es-ES".parse().unwrap(),
            Language::German => "de-DE".parse().unwrap(),
        }
    }

    /// Get the directory name for localization files
    pub fn directory_name(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
            Language::German => "de",
        }
    }

    /// Get a human-readable name for the language
    pub fn display_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
            Language::German => "Deutsch",
        }
    }

    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![Language::English, Language::Spanish, Language::German]
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

/// Configuration for localization
#[derive(Resource, Debug, Clone)]
pub struct LocalizationConfig {
    pub current_language: Language,
    pub fallback_language: Language,
}

impl Default for LocalizationConfig {
    fn default() -> Self {
        Self {
            current_language: Language::English,
            fallback_language: Language::English,
        }
    }
}

/// Event sent when the language changes
#[derive(Event, Debug, Clone)]
pub struct LanguageChangeEvent {
    pub new_language: Language,
}

/// Component to mark UI text that should be localized
#[derive(Component, Debug, Clone)]
pub struct LocalizedText {
    pub key: String,
    pub args: Vec<(String, String)>,
}

impl LocalizedText {
    /// Create a new localized text component with a key
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: Vec::new(),
        }
    }

    /// Create a new localized text component with a key and arguments
    pub fn with_args(key: impl Into<String>, args: Vec<(String, String)>) -> Self {
        Self {
            key: key.into(),
            args,
        }
    }

    /// Add an argument to the localized text
    pub fn with_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((key.into(), value.into()));
        self
    }
}
