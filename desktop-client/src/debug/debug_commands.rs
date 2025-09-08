// Debug and diagnostics system implementations using parameter bundles
// Refactored from main.rs to use parameter bundles for Bevy compliance

use bevy::prelude::*;
use log::info;
use std::collections::HashMap;

use crate::environment::BlockType;
use crate::inventory::ItemType;

use super::debug_params::{
    ComprehensiveDebugParams, CoreDebugParams, DebugToggleParams, DiagnosticsOverlay,
    DiagnosticsText,
};

/// System to setup diagnostics UI using parameter bundles
pub fn setup_diagnostics_ui_bundled(mut params: CoreDebugParams) {
    // Create a full-width diagnostics panel at the top
    params
        .commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Auto,
                height: Val::Px(480.0), // Fixed height to ensure proper display
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)), // Dark semi-transparent background
            Visibility::Hidden,                                 // Start hidden
            DiagnosticsOverlay,                                 // Add the component for toggling
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("IoTCraft Debug Information (Press F3 to toggle)\\n\\nLoading..."),
                TextFont {
                    font: params.fonts.regular.clone(),
                    font_size: 16.0,
                    font_smoothing: bevy::text::FontSmoothing::default(),
                    line_height: bevy::text::LineHeight::default(),
                },
                TextColor(Color::WHITE),
                DiagnosticsText, // Component for text updates
            ));
        });
}

/// System to handle F3 key toggle using parameter bundles
pub fn handle_diagnostics_toggle_bundled(mut params: DebugToggleParams) {
    if params.keyboard_input.just_pressed(KeyCode::F3) {
        params.diagnostics_visible.visible = !params.diagnostics_visible.visible;

        if let Ok(mut visibility) = params.diagnostics_query.single_mut() {
            *visibility = if params.diagnostics_visible.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        info!(
            "Diagnostics screen toggled: {}",
            params.diagnostics_visible.visible
        );
    }
}

/// System to update diagnostics content using comprehensive parameter bundles
pub fn update_diagnostics_content_bundled(mut params: ComprehensiveDebugParams) {
    if !params.display.diagnostics_visible.visible {
        return;
    }

    if let Ok(mut text) = params.display.diagnostics_text_query.single_mut() {
        if let Ok((transform, camera_controller)) = params.player.camera_query.single() {
            let translation = transform.translation;
            let yaw_degrees = camera_controller.yaw.to_degrees();
            let pitch_degrees = camera_controller.pitch.to_degrees();

            // Calculate additional useful information
            #[cfg(not(target_arch = "wasm32"))]
            let device_count = params.game_state.device_query.iter().count();
            #[cfg(target_arch = "wasm32")]
            let device_count = 0;
            let block_count = params.game_state.voxel_world.blocks.len();
            let selected_slot = params.game_state.inventory.selected_slot + 1; // 1-indexed for display

            // Get selected item info
            let selected_item = if let Some(item_stack) = params
                .game_state
                .inventory
                .slots
                .get(params.game_state.inventory.selected_slot)
                .and_then(|slot| slot.as_ref())
            {
                if item_stack.count > 0 {
                    format!(
                        "{} x {}",
                        item_stack.count,
                        match item_stack.item_type {
                            ItemType::Block(block_type) => match block_type {
                                BlockType::Grass => "Grass",
                                BlockType::Dirt => "Dirt",
                                BlockType::Stone => "Stone",
                                BlockType::QuartzBlock => "Quartz Block",
                                BlockType::GlassPane => "Glass Pane",
                                BlockType::CyanTerracotta => "Cyan Terracotta",
                                BlockType::Water => "Water",
                            },
                        }
                    )
                } else {
                    "Empty".to_string()
                }
            } else {
                "Empty".to_string()
            };

            // Get multiplayer player information
            let remote_player_count = params.player.player_avatar_query.iter().count();
            let mut player_list = Vec::new();
            player_list.push(format!(
                "  {} (Local): X={:.1} Y={:.1} Z={:.1}",
                params.player.local_profile.player_name,
                translation.x,
                translation.y,
                translation.z
            ));

            for (player_transform, player_avatar) in params.player.player_avatar_query.iter() {
                let pos = player_transform.translation;
                player_list.push(format!(
                    "  {} (Remote): X={:.1} Y={:.1} Z={:.1}",
                    player_avatar.player_id, pos.x, pos.y, pos.z
                ));
            }

            let players_text = if player_list.len() <= 1 {
                "  No other players connected".to_string()
            } else {
                player_list.join("\\n")
            };

            // Get MQTT broker connection status using temperature resource as indicator
            #[cfg(not(target_arch = "wasm32"))]
            let mqtt_connection_status = if (*params.multiplayer.temperature).value.is_some() {
                "✅ Connected (MQTT broker available)"
            } else {
                "🔄 Connecting to MQTT broker..."
            };

            #[cfg(target_arch = "wasm32")]
            let mqtt_connection_status = "🌐 Web MQTT (WebSocket)";

            // Get multiplayer mode information and world ID
            #[cfg(not(target_arch = "wasm32"))]
            let (multiplayer_mode_text, current_world_id) =
                match &*params.multiplayer.multiplayer_mode {
                    crate::multiplayer::MultiplayerMode::SinglePlayer => {
                        ("🚫 SinglePlayer".to_string(), "None".to_string())
                    }
                    crate::multiplayer::MultiplayerMode::HostingWorld {
                        world_id,
                        is_published,
                    } => {
                        let mode_text = if *is_published {
                            "🏠 Hosting (Public)"
                        } else {
                            "🏠 Hosting (Private)"
                        };
                        (mode_text.to_string(), world_id.clone())
                    }
                    crate::multiplayer::MultiplayerMode::JoinedWorld {
                        world_id,
                        host_player: _,
                    } => ("👥 Joined World".to_string(), world_id.clone()),
                };

            #[cfg(target_arch = "wasm32")]
            let (multiplayer_mode_text, current_world_id) =
                ("🌐 WASM Mode".to_string(), "Web".to_string());

            #[cfg(not(target_arch = "wasm32"))]
            let multiplayer_enabled = if params.multiplayer.multiplayer_status.connection_available
            {
                "✅ Enabled"
            } else {
                "❌ Disabled"
            };

            #[cfg(target_arch = "wasm32")]
            let multiplayer_enabled = "🌐 Web Mode";

            // Get MQTT subscription information from WorldDiscovery resource
            let subscribed_topics = vec![
                "iotcraft/worlds/+/info".to_string(),
                "iotcraft/worlds/+/data".to_string(),
                "iotcraft/worlds/+/data/chunk".to_string(),
                "iotcraft/worlds/+/changes".to_string(),
                "iotcraft/worlds/+/state/blocks/placed".to_string(),
                "iotcraft/worlds/+/state/blocks/removed".to_string(),
            ];

            // Get last messages from WorldDiscovery resource
            #[cfg(not(target_arch = "wasm32"))]
            let last_messages =
                if let Ok(messages) = params.multiplayer.world_discovery.last_messages.try_lock() {
                    messages.clone()
                } else {
                    HashMap::new()
                };

            #[cfg(target_arch = "wasm32")]
            let last_messages: std::collections::HashMap<
                String,
                crate::multiplayer::world_discovery::LastMessage,
            > = HashMap::new();

            let topics_text = subscribed_topics
                .iter()
                .map(|topic| {
                    // Find matching topic in last_messages (handling wildcards)
                    let _pattern = topic.replace("+", "[^/]+");
                    let matching_message = last_messages.iter().find(|(msg_topic, _)| {
                        // Simple pattern matching for wildcard topics
                        if topic.contains("+") {
                            // Create a basic regex-like match
                            let pattern_parts: Vec<&str> = topic.split("+").collect();
                            if pattern_parts.len() == 2 {
                                msg_topic.starts_with(pattern_parts[0])
                                    && msg_topic.ends_with(pattern_parts[1])
                            } else {
                                false
                            }
                        } else {
                            *msg_topic == topic
                        }
                    });

                    if let Some((_, last_msg)) = matching_message {
                        format!("  • {}: {}", topic, last_msg)
                    } else {
                        format!("  • {}: (no messages)", topic)
                    }
                })
                .collect::<Vec<_>>()
                .join("\\n");

            let uptime = params.game_state.time.elapsed_secs();
            let minutes = (uptime / 60.0) as u32;
            let seconds = (uptime % 60.0) as u32;

            text.0 = format!(
                "IoTCraft Debug Information (Press F3 to toggle)                        MQTT SUBSCRIPTIONS\\n\\
                -------------------------------------------------  |  --------------------------------------\\n\\
                                                               |\\n\\
                - PLAYER INFORMATION                           |  Current World Filter: {}\\n\\
                Position: X={:.2}  Y={:.2}  Z={:.2}               |\\n\\
                Rotation: Yaw={:.1}°  Pitch={:.1}°                    |  Subscribed Topics:\\n\\
                Selected Slot: {} ({})                        |  {}\\n\\
                                                               |\\n\\
                - MULTIPLAYER INFORMATION                      |\\n\\
                MQTT Broker: {}                               |\\n\\
                Multiplayer Status: {}                        |\\n\\
                Multiplayer Mode: {}                          |\\n\\
                Current World ID: {}                          |\\n\\
                Connected Players: {} (1 local + {} remote)   |\\n\\
                {}                                             |\\n\\
                                                               |\\n\\
                - WORLD INFORMATION                            |\\n\\
                Total Blocks: {}                              |\\n\\
                IoT Devices: {}                               |\\n\\
                Session Time: {}m {}s                         |\\n\\
                                                               |\\n\\
                - SCRIPT COMMANDS                              |\\n\\
                Teleport: tp {:.2} {:.2} {:.2}                    |\\n\\
                Look Direction: look {:.1} {:.1}                  |\\n\\
                                                               |\\n\\
                - CONTROLS                                     |\\n\\
                F3: Toggle this debug screen                  |\\n\\
                T: Open console                               |\\n\\
                1-9: Select inventory slot                    |\\n\\
                Mouse Wheel: Scroll inventory slots           |",
                current_world_id, // Used for the filter line
                translation.x,
                translation.y,
                translation.z,
                yaw_degrees,
                pitch_degrees,
                selected_slot,
                selected_item,
                topics_text,
                mqtt_connection_status,
                multiplayer_enabled,
                multiplayer_mode_text,
                current_world_id,
                remote_player_count + 1,
                remote_player_count,
                players_text,
                block_count,
                device_count,
                minutes,
                seconds,
                translation.x,
                translation.y,
                translation.z,
                yaw_degrees,
                pitch_degrees
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{MinimalPlugins, app::App};

    #[test]
    fn test_setup_diagnostics_ui_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_systems(Update, setup_diagnostics_ui_bundled);
        // Compilation test only
    }

    #[test]
    fn test_handle_diagnostics_toggle_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DiagnosticsVisible>();

        app.add_systems(Update, handle_diagnostics_toggle_bundled);
        // Compilation test only
    }

    #[test]
    fn test_update_diagnostics_content_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DiagnosticsVisible>();

        app.add_systems(Update, update_diagnostics_content_bundled);
        // Compilation test only
    }
}
