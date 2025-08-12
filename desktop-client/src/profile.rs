use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
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
            player_name: whoami::username(),
        }
    }
}

fn uuid_like() -> String {
    use rand::{RngCore, SeedableRng, rngs::StdRng};
    let mut rng = StdRng::from_entropy();
    let mut bytes = [0u8; 8];
    rng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn profile_path() -> PathBuf {
    // Store in user's config dir: ~/.config/iotcraft/profile.json (macOS/Linux), %APPDATA%/iotcraft/profile.json (Windows)
    let mut dir = dirs::config_dir().unwrap_or(std::env::current_dir().unwrap());
    dir.push("iotcraft");
    fs::create_dir_all(&dir).ok();
    dir.push("profile.json");
    dir
}

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
