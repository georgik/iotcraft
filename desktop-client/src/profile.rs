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
        profile.player_id = id;
        // Don't save overridden profile to disk to avoid conflicts
        return profile;
    }

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

    // Override player ID if provided
    if let Some(id) = player_id_override {
        profile.player_id = id;
    }

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
                        if let Some(id) = player_id_override {
                            profile.player_id = id;
                        }
                    }
                }

                // Save profile back to localStorage
                if let Ok(json) = serde_json::to_string(&profile) {
                    let _ = local_storage.set_item(storage_key, &json);
                }
            }
        }
    }

    profile
}
