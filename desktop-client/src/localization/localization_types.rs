use bevy::prelude::*;
use unic_langid::LanguageIdentifier;

/// Supported languages in the application using BCP 47 language tags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// English (United States)
    EnglishUS,
    /// Spanish (Spain)
    SpanishES,
    /// German (Germany)
    GermanDE,
}

impl Language {
    /// Get the BCP 47 language tag for this language
    pub fn language_tag(self) -> &'static str {
        match self {
            Language::EnglishUS => "en-US",
            Language::SpanishES => "es-ES",
            Language::GermanDE => "de-DE",
        }
    }

    /// Get the language identifier for this language
    pub fn language_id(self) -> LanguageIdentifier {
        self.language_tag().parse().unwrap()
    }

    /// Get the directory name for localization files (using BCP 47 language tags)
    pub fn directory_name(self) -> &'static str {
        self.language_tag()
    }

    /// Get a human-readable name for the language
    pub fn display_name(self) -> &'static str {
        match self {
            Language::EnglishUS => "English (United States)",
            Language::SpanishES => "Español (España)",
            Language::GermanDE => "Deutsch (Deutschland)",
        }
    }

    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![Language::EnglishUS, Language::SpanishES, Language::GermanDE]
    }

    /// Parse a BCP 47 language tag into a Language enum variant
    pub fn from_language_tag(tag: &str) -> Option<Language> {
        match tag {
            "en-US" | "en" => Some(Language::EnglishUS),
            "es-ES" | "es" => Some(Language::SpanishES),
            "de-DE" | "de" => Some(Language::GermanDE),
            _ => None,
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::EnglishUS
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
            current_language: Language::EnglishUS,
            fallback_language: Language::EnglishUS,
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
