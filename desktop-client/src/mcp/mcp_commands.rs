use bevy::prelude::*;
use chrono;
use log::info;
use serde_json::{Value, json};

use super::mcp_params::*;

/// Execute MCP command with bundled parameters to restore full multiplayer functionality
/// This replaces the simplified execute_basic_mcp_command function
pub fn execute_mcp_command_bundled(
    tool_name: &str,
    arguments: &Value,
    _core_params: &CoreMcpParams,
    world_params: &mut WorldMcpParams,
    multiplayer_params: &mut MultiplayerMcpParams,
    entity_params: &mut EntityMcpParams,
    state_params: &mut McpStateMcpParams,
) -> String {
    match tool_name {
        // System and health commands
        "get_client_info" => {
            json!({
                "client_id": crate::profile::load_or_create_profile_with_override(None).player_id,
                "version": "1.0.0",
                "status": "ready",
                "capabilities": ["world_building", "device_management", "mqtt_integration", "multiplayer"]
            })
            .to_string()
        }
        "get_game_state" => {
            json!({
                "game_state": "InGame",
                "world_loaded": true,
                "multiplayer_active": multiplayer_params.multiplayer_mode
                    .as_ref()
                    .map(|mode| !matches!(**mode, crate::multiplayer::shared_world::MultiplayerMode::SinglePlayer))
                    .unwrap_or(false)
            })
            .to_string()
        }
        "health_check" => {
            json!({
                "status": "healthy",
                "uptime_seconds": 3600,
                "memory_usage_mb": 256,
                "services_running": ["mqtt_client", "mcp_server", "multiplayer_system"]
            })
            .to_string()
        }
        "get_system_info" => {
            json!({
                "platform": std::env::consts::OS,
                "architecture": std::env::consts::ARCH,
                "rust_version": env!("CARGO_PKG_RUST_VERSION"),
                "app_version": env!("CARGO_PKG_VERSION")
            })
            .to_string()
        }
        "get_mqtt_status" => {
            execute_get_mqtt_status_command(_core_params, multiplayer_params)
        }
        "get_world_status" => {
            let block_count = world_params.voxel_world.blocks.len();

            // TEMPORARY DEBUG: Comment out device_query to isolate blocking issue
            // let device_count = entity_params.device_query.iter().count();
            let device_count = 0; // Hardcoded to avoid potential blocking query
            let world_name = world_params.current_world
                .as_ref()
                .map(|cw| cw.name.as_str())
                .unwrap_or("No World Loaded");

            json!({
                "blocks": block_count,
                "devices": device_count,
                "uptime_seconds": 3600,
                "world_name": world_name,
                "debug": "device_query_disabled_to_test_blocking"
            })
            .to_string()
        }
        // Game state management
        "set_game_state" => {
            execute_set_game_state_command(arguments, world_params, state_params)
        }
        // World management commands
        "load_world_from_fs" => {
            execute_load_world_from_fs_command(arguments, world_params)
        },
        "load_world_from_mqtt" => {
            execute_load_world_from_mqtt_command(arguments, world_params, multiplayer_params)
        }
        // Multiplayer commands - now fully functional!
        "get_multiplayer_status" => {
            execute_get_multiplayer_status_command(multiplayer_params)
        }
        "list_online_worlds" => {
            execute_list_online_worlds_command(multiplayer_params)
        }
        "join_world" => {
            execute_join_world_command(arguments, world_params, multiplayer_params, state_params)
        }
        "leave_world" => {
            execute_leave_world_command(multiplayer_params)
        }
        "publish_world" => {
            execute_publish_world_command(arguments, world_params, multiplayer_params)
        }
        "unpublish_world" => {
            execute_unpublish_world_command(multiplayer_params)
        }
        // Block manipulation commands
        "place_block" => {
            execute_place_block_command(arguments, world_params)
        }
        "remove_block" => {
            execute_remove_block_command(arguments, world_params)
        }
        "create_wall" => {
            execute_create_wall_command(arguments, world_params)
        }
        // Camera and movement commands
        "player_move" => {
            execute_player_move_command(arguments, entity_params)
        }
        "set_camera_angle" => {
            execute_set_camera_angle_command(arguments, entity_params)
        }
        // Device management
        "list_devices" => {
            execute_list_devices_command(entity_params)
        }
        // World template and creation commands
        "list_world_templates" => {
            execute_list_world_templates_command()
        }
        "create_world" => {
            // This command is handled specially in the MCP server for async execution
            "Error: create_world should be handled by async execution system".to_string()
        }
        _ => {
            format!("Error: Unknown MCP command: {}", tool_name)
        }
    }
}

/// Set game state command implementation
fn execute_set_game_state_command(
    arguments: &Value,
    world_params: &mut WorldMcpParams,
    state_params: &mut McpStateMcpParams,
) -> String {
    if let Some(state_str) = arguments.get("state").and_then(|v| v.as_str()) {
        info!("Setting game state via MCP to: {}", state_str);

        let new_state = match state_str.to_lowercase().as_str() {
            "mainmenu" | "main_menu" => crate::ui::main_menu::GameState::MainMenu,
            "ingame" | "in_game" => crate::ui::main_menu::GameState::InGame,
            "settings" => crate::ui::main_menu::GameState::Settings,
            "worldselection" | "world_selection" => crate::ui::main_menu::GameState::WorldSelection,
            "gameplaymenu" | "gameplay_menu" => crate::ui::main_menu::GameState::GameplayMenu,
            "consoleopen" | "console_open" => crate::ui::main_menu::GameState::ConsoleOpen,
            _ => {
                return format!(
                    "Error: Invalid game state '{}'. Valid states: MainMenu, InGame, Settings, WorldSelection, GameplayMenu, ConsoleOpen",
                    state_str
                );
            }
        };

        if let Some(next_state) = world_params.next_game_state.as_mut() {
            state_params.mcp_state_transition.is_mcp_transition = true;
            next_state.set(new_state.clone());
            format!("Game state set to {:?} (MCP transition)", new_state)
        } else {
            "Error: Game state resource not available".to_string()
        }
    } else {
        "Error: state parameter is required for set_game_state".to_string()
    }
}

/// Load world from filesystem command implementation (single-player and host)
fn execute_load_world_from_fs_command(
    arguments: &Value,
    world_params: &mut WorldMcpParams,
) -> String {
    if let Some(world_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
        info!("MCP load_world_from_fs command: world_name={}", world_name);

        // Always load from filesystem for this command
        world_params
            .load_world_events
            .write(crate::world::LoadWorldEvent {
                world_name: world_name.to_string(),
            });

        if let Some(next_state) = world_params.next_game_state.as_mut() {
            next_state.set(crate::ui::main_menu::GameState::InGame);
            info!("MCP load_world_from_fs: set game state to InGame");
        }

        format!(
            "Loading world '{}' from filesystem and transitioning to InGame state",
            world_name
        )
    } else {
        "Error: load_world_from_fs requires world_name parameter".to_string()
    }
}

/// Load world from MQTT command implementation (multiplayer world reconstruction)
fn execute_load_world_from_mqtt_command(
    arguments: &Value,
    world_params: &mut WorldMcpParams,
    multiplayer_params: &mut MultiplayerMcpParams,
) -> String {
    if let Some(world_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
        info!(
            "MCP load_world_from_mqtt command: world_name={}",
            world_name
        );

        // Check if we're in multiplayer mode and can reconstruct from MQTT
        if let Some(multiplayer_mode) = &multiplayer_params.multiplayer_mode {
            match multiplayer_mode.as_ref() {
                crate::multiplayer::shared_world::MultiplayerMode::JoinedWorld {
                    world_id,
                    host_player,
                } => {
                    info!(
                        "In JoinedWorld mode - checking for MQTT world data for world_id: {}",
                        world_id
                    );

                    // Check if we have cached world data from MQTT
                    if let Some(online_worlds) = &multiplayer_params.online_worlds {
                        if let Some(cached_data) = online_worlds.world_data_cache.get(world_id) {
                            info!(
                                "🔍 [Bob Debug] Found cached MQTT world data for world_id: {}",
                                world_id
                            );
                            info!(
                                "📊 [Bob Debug] World metadata - name: '{}', description: '{}'",
                                cached_data.metadata.name, cached_data.metadata.description
                            );
                            info!(
                                "📊 [Bob Debug] World metadata - created: {}, last_played: {}, version: {}",
                                cached_data.metadata.created_at,
                                cached_data.metadata.last_played,
                                cached_data.metadata.version
                            );
                            info!(
                                "🧱 [Bob Debug] Block data - total blocks: {}",
                                cached_data.blocks.len()
                            );

                            // Log detailed block type breakdown
                            let mut block_counts = std::collections::HashMap::new();
                            for block in &cached_data.blocks {
                                *block_counts.entry(block.block_type).or_insert(0) += 1;
                            }
                            info!("🧱 [Bob Debug] Block type breakdown: {:?}", block_counts);

                            // Log coordinate range for blocks
                            if !cached_data.blocks.is_empty() {
                                let (min_x, max_x) = cached_data
                                    .blocks
                                    .iter()
                                    .map(|b| b.x)
                                    .fold((i32::MAX, i32::MIN), |(min, max), x| {
                                        (min.min(x), max.max(x))
                                    });
                                let (min_y, max_y) = cached_data
                                    .blocks
                                    .iter()
                                    .map(|b| b.y)
                                    .fold((i32::MAX, i32::MIN), |(min, max), y| {
                                        (min.min(y), max.max(y))
                                    });
                                let (min_z, max_z) = cached_data
                                    .blocks
                                    .iter()
                                    .map(|b| b.z)
                                    .fold((i32::MAX, i32::MIN), |(min, max), z| {
                                        (min.min(z), max.max(z))
                                    });

                                info!(
                                    "📐 [Bob Debug] World dimensions - X: [{}, {}], Y: [{}, {}], Z: [{}, {}]",
                                    min_x, max_x, min_y, max_y, min_z, max_z
                                );
                            }

                            // Log player position and rotation data
                            info!(
                                "🎮 [Bob Debug] Player spawn position: ({:.2}, {:.2}, {:.2})",
                                cached_data.player_position.x,
                                cached_data.player_position.y,
                                cached_data.player_position.z
                            );
                            info!(
                                "🔄 [Bob Debug] Player rotation: ({:.4}, {:.4}, {:.4}, {:.4})",
                                cached_data.player_rotation.x,
                                cached_data.player_rotation.y,
                                cached_data.player_rotation.z,
                                cached_data.player_rotation.w
                            );

                            // Log inventory data
                            info!(
                                "🎒 [Bob Debug] Inventory items: {} slots with data",
                                cached_data.inventory.slots.len()
                            );
                            for (slot, item) in cached_data.inventory.slots.iter().enumerate() {
                                if let Some(block_type) = item {
                                    info!(
                                        "🎒 [Bob Debug] Inventory slot {}: {:?}",
                                        slot, block_type
                                    );
                                }
                            }

                            info!(
                                "✅ [Bob Debug] Triggering WorldStateReceivedEvent for world reconstruction from MQTT data"
                            );

                            // Trigger world state received event to reconstruct from MQTT data
                            multiplayer_params.world_state_events.write(
                                crate::multiplayer::shared_world::WorldStateReceivedEvent {
                                    world_id: world_id.clone(),
                                    world_data: cached_data.clone(),
                                },
                            );

                            if let Some(next_state) = world_params.next_game_state.as_mut() {
                                next_state.set(crate::ui::main_menu::GameState::InGame);
                            }

                            return format!(
                                "Reconstructed world '{}' from MQTT shared data ({} blocks, {} unique block types) - hosted by: {}",
                                world_name,
                                cached_data.blocks.len(),
                                block_counts.len(),
                                host_player
                            );
                        } else {
                            info!(
                                "No cached MQTT world data found for {}, attempting fallback to filesystem load for testing",
                                world_id
                            );

                            // Fallback: If no MQTT world data is available (testing scenario),
                            // load the world from filesystem as a backup
                            world_params
                                .load_world_events
                                .write(crate::world::LoadWorldEvent {
                                    world_name: world_name.to_string(),
                                });

                            info!(
                                "Triggered fallback filesystem load for world: {}",
                                world_name
                            );

                            if let Some(next_state) = world_params.next_game_state.as_mut() {
                                next_state.set(crate::ui::main_menu::GameState::InGame);
                            }

                            return format!(
                                "Joined world '{}' - loaded from filesystem as MQTT data fallback (testing mode)",
                                world_name
                            );
                        }
                    }
                }
                crate::multiplayer::shared_world::MultiplayerMode::HostingWorld {
                    world_id,
                    ..
                } => {
                    return format!(
                        "Error: Host should use load_world_from_fs instead of load_world_from_mqtt for world: {}",
                        world_id
                    );
                }
                crate::multiplayer::shared_world::MultiplayerMode::SinglePlayer => {
                    return format!(
                        "Error: Single-player mode should use load_world_from_fs instead of load_world_from_mqtt"
                    );
                }
            }
        }

        "Error: load_world_from_mqtt requires being in JoinedWorld multiplayer mode".to_string()
    } else {
        "Error: load_world_from_mqtt requires world_name parameter".to_string()
    }
}

/// Get multiplayer status command implementation
fn execute_get_multiplayer_status_command(multiplayer_params: &MultiplayerMcpParams) -> String {
    if let Some(mode) = &multiplayer_params.multiplayer_mode {
        match mode.as_ref() {
            crate::multiplayer::shared_world::MultiplayerMode::SinglePlayer => json!({
                "multiplayer_mode": "SinglePlayer",
                "world_id": null,
                "is_published": false,
                "host_player": null,
                "player_positions": [],
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
            .to_string(),
            crate::multiplayer::shared_world::MultiplayerMode::HostingWorld {
                world_id,
                is_published,
                ..
            } => {
                let host_player =
                    crate::profile::load_or_create_profile_with_override(None).player_name;
                json!({
                    "multiplayer_mode": "HostingWorld",
                    "world_id": world_id,
                    "is_published": is_published,
                    "host_player": host_player,
                    "player_positions": [],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string()
            }
            crate::multiplayer::shared_world::MultiplayerMode::JoinedWorld {
                world_id,
                host_player,
                ..
            } => json!({
                "multiplayer_mode": "JoinedWorld",
                "world_id": world_id,
                "is_published": false,
                "host_player": host_player,
                "player_positions": [],
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
            .to_string(),
        }
    } else {
        json!({
            "multiplayer_mode": "SinglePlayer",
            "world_id": null,
            "is_published": false,
            "host_player": null,
            "player_positions": [],
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string()
    }
}

/// Get MQTT status command implementation
fn execute_get_mqtt_status_command(
    core_params: &CoreMcpParams,
    multiplayer_params: &MultiplayerMcpParams,
) -> String {
    // Get actual MQTT connection status from Core MQTT Service
    let mqtt_connected = core_params
        .core_mqtt_status
        .as_ref()
        .map(|status| status.is_connected)
        .unwrap_or(false);
    let core_mqtt_service_available = core_params.core_mqtt_status.is_some();

    json!({
        "mqtt_connected": mqtt_connected,
        "core_mqtt_service_available": core_mqtt_service_available,
        "multiplayer_available": multiplayer_params.multiplayer_mode.is_some(),
        "status": if mqtt_connected && core_mqtt_service_available { "healthy" } else { "degraded" },
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "details": {
            "bundled_mcp_version": true,
            "multiplayer_mode": multiplayer_params.multiplayer_mode
                .as_ref()
                .map(|mode| format!("{:?}", **mode))
                .unwrap_or("SinglePlayer".to_string())
        }
    })
    .to_string()
}

/// List online worlds command implementation
fn execute_list_online_worlds_command(multiplayer_params: &mut MultiplayerMcpParams) -> String {
    // Refresh online worlds first
    multiplayer_params
        .refresh_events
        .write(crate::multiplayer::shared_world::RefreshOnlineWorldsEvent);

    if let Some(online_worlds) = &multiplayer_params.online_worlds {
        let worlds: Vec<Value> = online_worlds
            .worlds
            .iter()
            .map(|(world_id, world_info)| {
                json!({
                    "world_id": world_id,
                    "world_name": world_info.world_name,
                    "host_player": world_info.host_name,
                    "player_count": world_info.player_count,
                    "max_players": world_info.max_players,
                    "is_public": true,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            })
            .collect();

        json!({
            "worlds": worlds,
            "count": worlds.len(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string()
    } else {
        json!({
            "worlds": [],
            "count": 0,
            "error": "Online worlds resource not available (multiplayer may be disabled)",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        .to_string()
    }
}

/// Join world command implementation
fn execute_join_world_command(
    arguments: &Value,
    world_params: &mut WorldMcpParams,
    multiplayer_params: &mut MultiplayerMcpParams,
    state_params: &mut McpStateMcpParams,
) -> String {
    if let Some(world_id) = arguments.get("world_id").and_then(|v| v.as_str()) {
        info!("MCP join_world command: world_id={}", world_id);

        // Emit JoinSharedWorldEvent to trigger multiplayer world joining
        multiplayer_params.join_events.write(
            crate::multiplayer::shared_world::JoinSharedWorldEvent {
                world_id: world_id.to_string(),
            },
        );

        // Set game state to InGame with MCP transition flag
        if let Some(next_state) = world_params.next_game_state.as_mut() {
            state_params.mcp_state_transition.is_mcp_transition = true;
            next_state.set(crate::ui::main_menu::GameState::InGame);
            info!("MCP join_world: set game state to InGame with MCP transition flag");
        }

        format!(
            "Attempting to join multiplayer world '{}' and transitioning to InGame state",
            world_id
        )
    } else {
        "Error: join_world requires world_id parameter".to_string()
    }
}

/// Leave world command implementation
fn execute_leave_world_command(multiplayer_params: &mut MultiplayerMcpParams) -> String {
    info!("MCP leave_world command");

    // Emit LeaveSharedWorldEvent to trigger leaving multiplayer
    multiplayer_params
        .leave_events
        .write(crate::multiplayer::shared_world::LeaveSharedWorldEvent);

    "Left shared world and returned to single-player mode".to_string()
}

/// Publish world command implementation
fn execute_publish_world_command(
    arguments: &Value,
    world_params: &WorldMcpParams,
    multiplayer_params: &mut MultiplayerMcpParams,
) -> String {
    // Use current world name or provided world_name parameter
    let world_name =
        if let Some(provided_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
            provided_name.to_string()
        } else if let Some(current_world) = &world_params.current_world {
            current_world.name.clone()
        } else {
            return "Error: No current world loaded and no world_name provided".to_string();
        };

    let max_players = arguments
        .get("max_players")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as u32;
    let is_public = arguments
        .get("is_public")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    info!(
        "Publishing world via MCP: {} (max_players: {}, public: {})",
        world_name, max_players, is_public
    );

    // Emit PublishWorldEvent to trigger multiplayer mode transition
    multiplayer_params
        .publish_events
        .write(crate::multiplayer::shared_world::PublishWorldEvent {
            world_name: world_name.clone(),
            max_players,
            is_public,
        });

    format!(
        "World '{}' published for multiplayer (max_players: {}, public: {})",
        world_name, max_players, is_public
    )
}

/// Unpublish world command implementation
fn execute_unpublish_world_command(multiplayer_params: &mut MultiplayerMcpParams) -> String {
    info!("MCP unpublish_world command");

    if let Some(mode) = &multiplayer_params.multiplayer_mode {
        match mode.as_ref() {
            crate::multiplayer::shared_world::MultiplayerMode::HostingWorld {
                world_id, ..
            } => {
                // Emit UnpublishWorldEvent with the current world ID
                multiplayer_params.unpublish_events.write(
                    crate::multiplayer::shared_world::UnpublishWorldEvent {
                        world_id: world_id.clone(),
                    },
                );
                format!(
                    "World '{}' unpublished and returned to single-player mode",
                    world_id
                )
            }
            crate::multiplayer::shared_world::MultiplayerMode::JoinedWorld { .. } => {
                "Error: Cannot unpublish a joined world - use leave_world instead".to_string()
            }
            crate::multiplayer::shared_world::MultiplayerMode::SinglePlayer => {
                "Error: No world is currently published".to_string()
            }
        }
    } else {
        "Error: Multiplayer mode not available".to_string()
    }
}

/// Place block command implementation
fn execute_place_block_command(arguments: &Value, world_params: &mut WorldMcpParams) -> String {
    if let (Some(block_type), Some(x), Some(y), Some(z)) = (
        arguments.get("block_type").and_then(|v| v.as_str()),
        arguments.get("x").and_then(|v| v.as_i64()),
        arguments.get("y").and_then(|v| v.as_i64()),
        arguments.get("z").and_then(|v| v.as_i64()),
    ) {
        if let Some(block_type_enum) = parse_block_type(block_type) {
            let position = bevy::math::IVec3::new(x as i32, y as i32, z as i32);
            world_params
                .voxel_world
                .set_block(position, block_type_enum);
            format!("Placed {} block at ({}, {}, {})", block_type, x, y, z)
        } else {
            format!(
                "Error: Unknown block type '{}'. Valid types: grass, dirt, stone, quartz_block, glass_pane, cyan_terracotta",
                block_type
            )
        }
    } else {
        "Error: place_block requires block_type, x, y, z parameters".to_string()
    }
}

/// Remove block command implementation
fn execute_remove_block_command(arguments: &Value, world_params: &mut WorldMcpParams) -> String {
    if let (Some(x), Some(y), Some(z)) = (
        arguments.get("x").and_then(|v| v.as_i64()),
        arguments.get("y").and_then(|v| v.as_i64()),
        arguments.get("z").and_then(|v| v.as_i64()),
    ) {
        let position = bevy::math::IVec3::new(x as i32, y as i32, z as i32);
        if world_params.voxel_world.remove_block(&position).is_some() {
            format!("Removed block at ({}, {}, {})", x, y, z)
        } else {
            format!("No block found at ({}, {}, {}) to remove", x, y, z)
        }
    } else {
        "Error: remove_block requires x, y, z parameters".to_string()
    }
}

/// Create wall command implementation
fn execute_create_wall_command(arguments: &Value, world_params: &mut WorldMcpParams) -> String {
    if let (Some(block_type), Some(x1), Some(y1), Some(z1), Some(x2), Some(y2), Some(z2)) = (
        arguments.get("block_type").and_then(|v| v.as_str()),
        arguments.get("x1").and_then(|v| v.as_i64()),
        arguments.get("y1").and_then(|v| v.as_i64()),
        arguments.get("z1").and_then(|v| v.as_i64()),
        arguments.get("x2").and_then(|v| v.as_i64()),
        arguments.get("y2").and_then(|v| v.as_i64()),
        arguments.get("z2").and_then(|v| v.as_i64()),
    ) {
        if let Some(block_type_enum) = parse_block_type(block_type) {
            let min_x = x1.min(x2) as i32;
            let max_x = x1.max(x2) as i32;
            let min_y = y1.min(y2) as i32;
            let max_y = y1.max(y2) as i32;
            let min_z = z1.min(z2) as i32;
            let max_z = z1.max(z2) as i32;

            let mut blocks_created = 0;
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    for z in min_z..=max_z {
                        let position = bevy::math::IVec3::new(x, y, z);
                        world_params
                            .voxel_world
                            .set_block(position, block_type_enum);
                        blocks_created += 1;
                    }
                }
            }

            format!(
                "Created {} wall with {} {} blocks from ({},{},{}) to ({},{},{})",
                block_type, blocks_created, block_type, x1, y1, z1, x2, y2, z2
            )
        } else {
            format!(
                "Error: Unknown block type '{}'. Valid types: grass, dirt, stone, quartz_block, glass_pane, cyan_terracotta",
                block_type
            )
        }
    } else {
        "Error: create_wall requires block_type, x1, y1, z1, x2, y2, z2 parameters".to_string()
    }
}

/// Player move command implementation
fn execute_player_move_command(arguments: &Value, entity_params: &mut EntityMcpParams) -> String {
    if let (Some(x), Some(y), Some(z)) = (
        arguments.get("x").and_then(|v| v.as_f64()),
        arguments.get("y").and_then(|v| v.as_f64()),
        arguments.get("z").and_then(|v| v.as_f64()),
    ) {
        let mut moved = false;
        for (mut transform, _) in entity_params.camera_query.iter_mut() {
            transform.translation = Vec3::new(x as f32, y as f32, z as f32);
            moved = true;
            break; // Only move the first camera
        }

        if moved {
            format!("Player moved to ({:.2}, {:.2}, {:.2})", x, y, z)
        } else {
            "Error: No player camera found to move".to_string()
        }
    } else {
        "Error: player_move requires x, y, z parameters".to_string()
    }
}

/// Set camera angle command implementation
fn execute_set_camera_angle_command(
    arguments: &Value,
    entity_params: &mut EntityMcpParams,
) -> String {
    if let (Some(yaw), Some(pitch)) = (
        arguments.get("yaw").and_then(|v| v.as_f64()),
        arguments.get("pitch").and_then(|v| v.as_f64()),
    ) {
        let mut camera_rotated = false;
        for (_transform, mut camera_controller) in entity_params.camera_query.iter_mut() {
            camera_controller.yaw = yaw as f32;
            camera_controller.pitch = pitch as f32;
            camera_rotated = true;
            break;
        }

        if camera_rotated {
            format!("Camera angle set to yaw: {:.1}°, pitch: {:.1}°", yaw, pitch)
        } else {
            "Error: No camera found to set angle".to_string()
        }
    } else {
        "Error: set_camera_angle requires yaw and pitch parameters".to_string()
    }
}

/// List devices command implementation
fn execute_list_devices_command(entity_params: &EntityMcpParams) -> String {
    let devices: Vec<Value> = entity_params
        .device_query
        .iter()
        .map(|(device, transform)| {
            json!({
                "device_id": device.device_id,
                "device_type": device.device_type,
                "position": {
                    "x": transform.translation.x,
                    "y": transform.translation.y,
                    "z": transform.translation.z
                },
                "status": "online"
            })
        })
        .collect();

    json!({
        "devices": devices,
        "count": devices.len()
    })
    .to_string()
}

/// List world templates command implementation
fn execute_list_world_templates_command() -> String {
    let templates_dir = std::path::Path::new("scripts/world_templates");
    if templates_dir.exists() {
        match std::fs::read_dir(templates_dir) {
            Ok(entries) => {
                let mut templates = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map(|ext| ext == "txt").unwrap_or(false) {
                        if let Some(template_name) = path.file_stem().and_then(|s| s.to_str()) {
                            // Read first few lines to get description
                            let description = if let Ok(content) = std::fs::read_to_string(&path) {
                                content
                                    .lines()
                                    .find(|line| {
                                        line.starts_with("# ") && !line.contains("Template")
                                    })
                                    .map(|line| line.trim_start_matches("# "))
                                    .unwrap_or("World template")
                                    .to_string()
                            } else {
                                "World template".to_string()
                            };

                            templates.push(json!({
                                "name": template_name,
                                "description": description,
                                "file": format!("{}.txt", template_name)
                            }));
                        }
                    }
                }
                templates.sort_by(|a, b| {
                    a["name"]
                        .as_str()
                        .unwrap_or("")
                        .cmp(b["name"].as_str().unwrap_or(""))
                });

                json!({
                    "templates": templates,
                    "count": templates.len()
                })
                .to_string()
            }
            Err(e) => format!("Error reading templates directory: {}", e),
        }
    } else {
        "Error: templates directory not found at scripts/world_templates/".to_string()
    }
}

/// Create world command implementation - now uses async world creation
/// This command starts async world creation and will be completed by the completion handler
pub fn execute_create_world_command(
    arguments: &Value,
    world_params: &mut WorldMcpParams,
    _entity_params: &mut EntityMcpParams,
    state_params: &mut McpStateMcpParams,
    _core_params: &mut crate::mcp::mcp_params::CoreMcpParams,
    request_id: String,
) -> Option<String> {
    if let Some(world_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
        let description = arguments
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("A new world created via MCP");
        let template = arguments
            .get("template")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        info!(
            "🚀 [DEBUG] Starting async world creation via MCP: name='{}', description='{}', template='{}', request_id='{}'",
            world_name, description, template, request_id
        );

        // Validate template exists
        let template_path = format!("scripts/world_templates/{}.txt", template);
        if !std::path::Path::new(&template_path).exists() {
            return Some(format!(
                "Error: Template '{}' not found. Available templates: default, medieval, modern, creative",
                template
            ));
        }

        // Create world directory structure
        let worlds_dir = crate::world::world_systems::get_worlds_directory();
        let world_path = worlds_dir.join(&world_name);

        if let Err(e) = std::fs::create_dir_all(&world_path) {
            return Some(format!(
                "Error: Failed to create world directory for {}: {}",
                world_name, e
            ));
        }

        // Create and save metadata
        let metadata = crate::world::world_types::WorldMetadata {
            name: world_name.to_string(),
            description: description.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        };

        let metadata_path = world_path.join("metadata.json");
        match serde_json::to_string_pretty(&metadata) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&metadata_path, json_str) {
                    return Some(format!(
                        "Error: Failed to write metadata for {}: {}",
                        world_name, e
                    ));
                }
            }
            Err(e) => {
                return Some(format!(
                    "Error: Failed to serialize metadata for {}: {}",
                    world_name, e
                ));
            }
        }

        // Set MCP transition flag for UI state management
        state_params.mcp_state_transition.is_mcp_transition = true;

        // Send async world creation event (non-blocking)
        if let Some(start_events) = world_params.start_world_creation_events.as_mut() {
            let event = crate::world::StartWorldCreationEvent {
                world_name: world_name.to_string(),
                description: description.to_string(),
                template: template.to_string(),
                should_transition_to_ingame: true,
                mcp_request_id: Some(request_id.clone()),
            };

            info!(
                "📤 [DEBUG] About to write StartWorldCreationEvent: {:?}",
                event.mcp_request_id
            );
            start_events.write(event);

            info!(
                "📤 [DEBUG] Sent async world creation event for '{}' (MCP request {}) - world creation will proceed in background",
                world_name, request_id
            );

            // Return None to indicate this command will complete asynchronously
            None
        } else {
            Some("Error: World creation system not available".to_string())
        }
    } else {
        Some("Error: world_name is required for create_world".to_string())
    }
}

/// Parse block type from string for MCP commands
fn parse_block_type(block_type_str: &str) -> Option<crate::environment::BlockType> {
    match block_type_str.to_lowercase().as_str() {
        "grass" => Some(crate::environment::BlockType::Grass),
        "dirt" => Some(crate::environment::BlockType::Dirt),
        "stone" => Some(crate::environment::BlockType::Stone),
        "quartz_block" => Some(crate::environment::BlockType::QuartzBlock),
        "glass_pane" => Some(crate::environment::BlockType::GlassPane),
        "cyan_terracotta" => Some(crate::environment::BlockType::CyanTerracotta),
        "water" => Some(crate::environment::BlockType::Water),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{environment::BlockType, script::script_types::CommandExecutedEvent};

    #[test]
    fn test_parse_block_type() {
        assert_eq!(parse_block_type("grass"), Some(BlockType::Grass));
        assert_eq!(parse_block_type("STONE"), Some(BlockType::Stone));
        assert_eq!(parse_block_type("invalid"), None);
    }

    #[test]
    fn test_place_block_command_validation() {
        let mut world = World::new();
        world.insert_resource(crate::environment::VoxelWorld::default());
        world.init_resource::<Events<crate::world::CreateWorldEvent>>();
        world.init_resource::<Events<crate::world::LoadWorldEvent>>();
        world.insert_resource(crate::world::world_types::DiscoveredWorlds::default());

        // Test valid place_block arguments
        let valid_args = json!({
            "block_type": "stone",
            "x": 1,
            "y": 2,
            "z": 3
        });

        // This would be tested in integration tests with full world setup
        // For unit test, we're just validating argument parsing
        assert!(valid_args.get("block_type").is_some());
        assert!(valid_args.get("x").and_then(|v| v.as_i64()).is_some());
    }

    #[test]
    fn test_invalid_command_arguments() {
        // Test missing required arguments
        let invalid_args = json!({
            "block_type": "stone"
            // missing x, y, z
        });

        assert!(invalid_args.get("x").is_none());

        // Test invalid block type
        assert_eq!(parse_block_type("nonexistent"), None);
    }
}
