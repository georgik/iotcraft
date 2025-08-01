use crate::localization::Language;
use bevy::prelude::*;
use std::collections::HashMap;

/// Resource that manages localization strings for different languages
#[derive(Resource, Default)]
pub struct LocalizationBundle {
    translations: HashMap<Language, HashMap<String, String>>,
}

impl LocalizationBundle {
    /// Load localization files for a specific language
    pub fn load_language(&mut self, language: Language) -> Result<(), Box<dyn std::error::Error>> {
        let mut translations = HashMap::new();

        // Parse the simple key=value format from our localization files
        let content = self.get_localization_content(language);
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(eq_pos) = line.find(" = ") {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 3..].trim().to_string();
                translations.insert(key, value);
            }
        }

        self.translations.insert(language, translations);
        info!(
            "Loaded {} translations for {:?}",
            self.translations
                .get(&language)
                .map(|t| t.len())
                .unwrap_or(0),
            language
        );

        Ok(())
    }

    /// Get localized text for a key
    pub fn get_text(&self, language: Language, key: &str, args: &[(String, String)]) -> String {
        if let Some(translations) = self.translations.get(&language) {
            if let Some(template) = translations.get(key) {
                // Simple template substitution for arguments
                let mut result = template.clone();
                for (arg_key, arg_value) in args {
                    let placeholder = format!("{{${}}}", arg_key);
                    result = result.replace(&placeholder, arg_value);
                }
                return result;
            }
        }

        // Fallback: return the key itself if translation is not found
        warn!("Missing translation for key: {}", key);
        key.to_string()
    }

    /// Get the default localization content for each language
    fn get_localization_content(&self, language: Language) -> String {
        match language {
            Language::EnglishUS => include_str!("../../localization/en-US/main.ftl").to_string(),
            Language::SpanishES => include_str!("../../localization/es-ES/main.ftl").to_string(),
            Language::GermanDE => include_str!("../../localization/de-DE/main.ftl").to_string(),
            Language::CzechCZ => include_str!("../../localization/cs-CZ/main.ftl").to_string(),
            Language::SlovakSK => include_str!("../../localization/sk-SK/main.ftl").to_string(),
            Language::PolishPL => include_str!("../../localization/pl-PL/main.ftl").to_string(),
            Language::HungarianHU => include_str!("../../localization/hu-HU/main.ftl").to_string(),
            Language::FrenchFR => include_str!("../../localization/fr-FR/main.ftl").to_string(),
            Language::ItalianIT => include_str!("../../localization/it-IT/main.ftl").to_string(),
            Language::PortugueseBR => include_str!("../../localization/pt-BR/main.ftl").to_string(),
            Language::SlovenianSI => include_str!("../../localization/sl-SI/main.ftl").to_string(),
            Language::CroatianHR => include_str!("../../localization/hr-HR/main.ftl").to_string(),
            Language::RomanianRO => include_str!("../../localization/ro-RO/main.ftl").to_string(),
            Language::BulgarianBG => include_str!("../../localization/bg-BG/main.ftl").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localization_bundle_loads_english() {
        let mut bundle = LocalizationBundle::default();
        bundle
            .load_language(Language::EnglishUS)
            .expect("Failed to load English");

        // Test basic key retrieval
        let text = bundle.get_text(Language::EnglishUS, "menu-enter-world", &[]);
        assert_eq!(text, "Enter the world");

        let text = bundle.get_text(Language::EnglishUS, "menu-quit-application", &[]);
        assert_eq!(text, "Quit Application");
    }

    #[test]
    fn test_localization_bundle_with_args() {
        let mut bundle = LocalizationBundle::default();
        bundle
            .load_language(Language::EnglishUS)
            .expect("Failed to load English");

        // Test with arguments
        let args = vec![("time".to_string(), "2023-12-25 15:30".to_string())];
        let text = bundle.get_text(Language::EnglishUS, "world-last-played", &args);
        assert_eq!(text, "Last played: 2023-12-25 15:30");
    }

    #[test]
    fn test_localization_bundle_fallback() {
        let bundle = LocalizationBundle::default();

        // Should return the key when translation is not found
        let text = bundle.get_text(Language::EnglishUS, "non-existent-key", &[]);
        assert_eq!(text, "non-existent-key");
    }
}
