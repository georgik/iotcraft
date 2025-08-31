use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub player_id: String,
    pub player_name: String,
}

impl PlayerProfile {
    /// Create a new PlayerProfile with the given name
    /// This method works on both desktop and WASM targets
    pub fn new(name: String) -> Self {
        let player_id = format!("player-{}", uuid_like());
        Self {
            player_id,
            player_name: name,
        }
    }
}

impl Default for PlayerProfile {
    fn default() -> Self {
        let player_id = format!("player-{}", uuid_like());
        Self {
            player_id,
            player_name: get_default_username(),
        }
    }
}

fn uuid_like() -> String {
    use rand::{RngCore, rng};
    let mut rng = rng();
    let mut bytes = [0u8; 8];
    rng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_default_username() -> String {
    whoami::username()
}

#[cfg(target_arch = "wasm32")]
fn get_default_username() -> String {
    "WebPlayer".to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn profile_path() -> PathBuf {
    // Store in user's config dir: ~/.config/iotcraft/profile.json (macOS/Linux), %APPDATA%/iotcraft/profile.json (Windows)
    let mut dir = dirs::config_dir().unwrap_or(std::env::current_dir().unwrap());
    dir.push("iotcraft");
    fs::create_dir_all(&dir).ok();
    dir.push("profile.json");
    dir
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_or_create_profile_with_override(player_id_override: Option<String>) -> PlayerProfile {
    let path = profile_path();
    let mut profile = if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(profile) = serde_json::from_str::<PlayerProfile>(&content) {
            profile
        } else {
            PlayerProfile::default()
        }
    } else {
        PlayerProfile::default()
    };

    // Override player ID if provided
    if let Some(id) = player_id_override {
        profile.player_id = format!("player-{}", id); // Ensure it's properly formatted
        // Don't save overridden profile to disk to avoid conflicts
        log::info!(
            "Player profile override active: player_id={}, player_name={}",
            profile.player_id,
            profile.player_name
        );
        return profile;
    }

    log::info!(
        "Loaded player profile from disk: player_id={}, player_name={}",
        profile.player_id,
        profile.player_name
    );

    // Save profile to disk only if not using override
    if let Ok(json) = serde_json::to_string_pretty(&profile) {
        let _ = fs::write(path, json);
    }
    profile
}

#[cfg(target_arch = "wasm32")]
pub fn load_or_create_profile_with_override(player_id_override: Option<String>) -> PlayerProfile {
    // For web, we'll use localStorage in the future, but for now just create a default profile
    let mut profile = PlayerProfile::default();

    // Try to load from localStorage if available
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(local_storage)) = window.local_storage() {
                let storage_key = "iotcraft_player_profile";
                if let Ok(Some(stored_data)) = local_storage.get_item(storage_key) {
                    if let Ok(stored_profile) = serde_json::from_str::<PlayerProfile>(&stored_data)
                    {
                        profile = stored_profile;
                    }
                }
            }
        }
    }

    // Track if we used an override
    let used_override = if let Some(id) = player_id_override {
        profile.player_id = format!("player-{}", id); // Ensure consistent formatting
        log::info!(
            "Player profile override active (web): player_id={}, player_name={}",
            profile.player_id,
            profile.player_name
        );
        true
    } else {
        false
    };

    // Save profile back to localStorage (only if not using override)
    #[cfg(target_arch = "wasm32")]
    {
        if !used_override {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    let storage_key = "iotcraft_player_profile";
                    if let Ok(json) = serde_json::to_string(&profile) {
                        let _ = local_storage.set_item(storage_key, &json);
                    }
                }
            }
        }
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_profile_default_generation() {
        let profile = PlayerProfile::default();
        // Should have a player ID that starts with "player-" and is followed by hex
        assert!(profile.player_id.starts_with("player-"));
        assert!(profile.player_id.len() > 7); // "player-" + some hex
        // Should have a username
        assert!(!profile.player_name.is_empty());
    }

    #[test]
    fn test_player_profile_override_native() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let profile = load_or_create_profile_with_override(Some("test-123".to_string()));
            assert_eq!(profile.player_id, "player-test-123");
        }
    }

    #[test]
    fn test_player_profile_override_formats_correctly() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Test various input formats
            let test_cases = [
                ("2", "player-2"),
                ("test-player", "player-test-player"),
                ("abc123", "player-abc123"),
            ];

            for (input, expected) in test_cases {
                let profile = load_or_create_profile_with_override(Some(input.to_string()));
                assert_eq!(profile.player_id, expected, "Failed for input: {}", input);
            }
        }
    }

    #[test]
    fn test_different_player_ids_are_unique() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let profile1 = load_or_create_profile_with_override(Some("1".to_string()));
            let profile2 = load_or_create_profile_with_override(Some("2".to_string()));

            assert_eq!(profile1.player_id, "player-1");
            assert_eq!(profile2.player_id, "player-2");
            assert_ne!(profile1.player_id, profile2.player_id);
        }
    }
}
