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
    /// Czech (Czechia)
    CzechCZ,
    /// Slovak (Slovakia)
    SlovakSK,
    /// Polish (Poland)
    PolishPL,
    /// Hungarian (Hungary)
    HungarianHU,
    /// French (France)
    FrenchFR,
    /// Italian (Italy)
    ItalianIT,
    /// Portuguese (Brazil)
    PortugueseBR,
    /// Chinese (China)
    ChineseCN,
    /// Japanese (Japan)
    JapaneseJP,
    /// Slovenian (Slovenia)
    SlovenianSI,
    /// Croatian (Croatia)
    CroatianHR,
    /// Romanian (Romania)
    RomanianRO,
    /// Bulgarian (Bulgaria)
    BulgarianBG,
}

impl Language {
    /// Get the BCP 47 language tag for this language
    pub fn language_tag(self) -> &'static str {
        match self {
            Language::EnglishUS => "en-US",
            Language::SpanishES => "es-ES",
            Language::GermanDE => "de-DE",
            Language::CzechCZ => "cs-CZ",
            Language::SlovakSK => "sk-SK",
            Language::PolishPL => "pl-PL",
            Language::HungarianHU => "hu-HU",
            Language::FrenchFR => "fr-FR",
            Language::ItalianIT => "it-IT",
            Language::PortugueseBR => "pt-BR",
            Language::ChineseCN => "zh-CN",
            Language::JapaneseJP => "ja-JP",
            Language::SlovenianSI => "sl-SI",
            Language::CroatianHR => "hr-HR",
            Language::RomanianRO => "ro-RO",
            Language::BulgarianBG => "bg-BG",
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
            Language::CzechCZ => "Čeština (Česká republika)",
            Language::SlovakSK => "Slovenčina (Slovensko)",
            Language::PolishPL => "Polski (Polska)",
            Language::HungarianHU => "Magyar (Magyarország)",
            Language::FrenchFR => "Français (France)",
            Language::ItalianIT => "Italiano (Italia)",
            Language::PortugueseBR => "Português (Brasil)",
            Language::ChineseCN => "中文 (中国)",
            Language::JapaneseJP => "日本語 (日本)",
            Language::SlovenianSI => "Slovenščina (Slovenija)",
            Language::CroatianHR => "Hrvatski (Hrvatska)",
            Language::RomanianRO => "Română (România)",
            Language::BulgarianBG => "Български (България)",
        }
    }

    /// Get all supported languages
    pub fn all() -> Vec<Language> {
        vec![
            Language::EnglishUS,
            Language::SpanishES,
            Language::GermanDE,
            Language::CzechCZ,
            Language::SlovakSK,
            Language::PolishPL,
            Language::HungarianHU,
            Language::FrenchFR,
            Language::ItalianIT,
            Language::PortugueseBR,
            Language::ChineseCN,
            Language::JapaneseJP,
            Language::SlovenianSI,
            Language::CroatianHR,
            Language::RomanianRO,
            Language::BulgarianBG,
        ]
    }

    /// Parse a BCP 47 language tag into a Language enum variant
    pub fn from_language_tag(tag: &str) -> Option<Language> {
        match tag {
            "en-US" | "en" => Some(Language::EnglishUS),
            "es-ES" | "es" => Some(Language::SpanishES),
            "de-DE" | "de" => Some(Language::GermanDE),
            "cs-CZ" | "cs" => Some(Language::CzechCZ),
            "sk-SK" | "sk" => Some(Language::SlovakSK),
            "pl-PL" | "pl" => Some(Language::PolishPL),
            "hu-HU" | "hu" => Some(Language::HungarianHU),
            "fr-FR" | "fr" => Some(Language::FrenchFR),
            "it-IT" | "it" => Some(Language::ItalianIT),
            "pt-BR" => Some(Language::PortugueseBR),
            "zh-CN" | "zh" => Some(Language::ChineseCN),
            "ja-JP" | "ja" => Some(Language::JapaneseJP),
            "sl-SI" | "sl" => Some(Language::SlovenianSI),
            "hr-HR" | "hr" => Some(Language::CroatianHR),
            "ro-RO" | "ro" => Some(Language::RomanianRO),
            "bg-BG" | "bg" => Some(Language::BulgarianBG),
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

impl LocalizationConfig {
    /// Create a new LocalizationConfig with optional CLI language override
    pub fn new(cli_language_override: Option<String>) -> Self {
        let current_language = if let Some(lang_tag) = cli_language_override {
            // CLI language takes precedence
            Language::from_language_tag(&lang_tag).unwrap_or_else(|| {
                warn!(
                    "Invalid language tag from CLI: '{}', falling back to environment detection",
                    lang_tag
                );
                Self::detect_system_language()
            })
        } else {
            // Detect from environment
            Self::detect_system_language()
        };

        Self {
            current_language,
            fallback_language: Language::EnglishUS,
        }
    }

    /// Detect language from environment variables
    fn detect_system_language() -> Language {
        // Try LANG environment variable first (Unix/Linux/macOS)
        if let Ok(lang_env) = std::env::var("LANG") {
            // LANG format: "en_US.UTF-8" -> extract "en_US" -> convert to "en-US"
            let lang_part = lang_env.split('.').next().unwrap_or("en_US");
            let normalized = lang_part.replace('_', "-");
            if let Some(language) = Language::from_language_tag(&normalized) {
                info!(
                    "Detected language from LANG environment variable: {}",
                    normalized
                );
                return language;
            }
        }

        // Try LC_ALL environment variable
        if let Ok(lc_all) = std::env::var("LC_ALL") {
            let lang_part = lc_all.split('.').next().unwrap_or("en_US");
            let normalized = lang_part.replace('_', "-");
            if let Some(language) = Language::from_language_tag(&normalized) {
                info!(
                    "Detected language from LC_ALL environment variable: {}",
                    normalized
                );
                return language;
            }
        }

        // Try LANGUAGE environment variable
        if let Ok(lang_env) = std::env::var("LANGUAGE") {
            // LANGUAGE can contain multiple languages separated by colon
            let first_lang = lang_env.split(':').next().unwrap_or("en_US");
            let normalized = first_lang.replace('_', "-");
            if let Some(language) = Language::from_language_tag(&normalized) {
                info!(
                    "Detected language from LANGUAGE environment variable: {}",
                    normalized
                );
                return language;
            }
        }

        info!("No supported language detected from environment, using English (US) as default");
        Language::EnglishUS
    }
}

impl Default for LocalizationConfig {
    fn default() -> Self {
        Self::new(None)
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
