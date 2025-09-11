use bevy::prelude::*;
use log::{error, info};
use std::collections::VecDeque;

use crate::environment::{BlockType, VoxelWorld};
use crate::ui::main_menu::GameState;
use crate::world::world_types::WorldMetadata;

/// Maximum number of blocks to process per frame to avoid UI blocking
const BLOCKS_PER_FRAME: usize = 50;

/// Represents a template command to be executed asynchronously
#[derive(Debug, Clone)]
pub enum TemplateCommand {
    Place {
        block_type: BlockType,
        x: i32,
        y: i32,
        z: i32,
    },
    Wall {
        block_type: BlockType,
        x1: i32,
        y1: i32,
        z1: i32,
        x2: i32,
        y2: i32,
        z2: i32,
    },
    Teleport {
        x: f32,
        y: f32,
        z: f32,
    },
    Look {
        yaw: f32,
        pitch: f32,
    },
}

/// Resource to track ongoing world creation tasks
#[derive(Resource, Default)]
pub struct WorldCreationTask {
    pub world_name: String,
    pub description: String,
    pub metadata: Option<WorldMetadata>,
    pub template_commands: VecDeque<TemplateCommand>,
    pub pending_blocks: VecDeque<(bevy::math::IVec3, BlockType)>, // Pre-expanded wall commands
    pub total_commands: usize,
    pub processed_commands: usize,
    pub total_blocks_expected: usize,
    pub blocks_created: usize,
    pub is_active: bool,
    pub should_transition_to_ingame: bool,
    pub mcp_request_id: Option<String>, // Track MCP request if initiated via MCP
}

/// Event fired when world creation is completed
#[derive(Event, BufferedEvent, Debug)]
pub struct WorldCreationCompletedEvent {
    pub world_name: String,
    pub blocks_created: usize,
    pub template_used: String,
    pub mcp_request_id: Option<String>, // Track MCP request if initiated via MCP
}

/// Event to start async world creation
#[derive(Event, BufferedEvent, Debug)]
pub struct StartWorldCreationEvent {
    pub world_name: String,
    pub description: String,
    pub template: String,
    pub should_transition_to_ingame: bool,
    pub mcp_request_id: Option<String>, // Track MCP request if initiated via MCP
}

impl WorldCreationTask {
    pub fn new(
        world_name: String,
        description: String,
        template_commands: Vec<TemplateCommand>,
        metadata: WorldMetadata,
        should_transition_to_ingame: bool,
        mcp_request_id: Option<String>,
    ) -> Self {
        let total_commands = template_commands.len();
        let mut task = Self {
            world_name,
            description,
            metadata: Some(metadata),
            template_commands: template_commands.into(),
            pending_blocks: VecDeque::new(),
            total_commands,
            processed_commands: 0,
            total_blocks_expected: 0,
            blocks_created: 0,
            is_active: true,
            should_transition_to_ingame,
            mcp_request_id,
        };

        // Pre-calculate total blocks for progress tracking
        task.calculate_total_blocks();
        task
    }

    fn calculate_total_blocks(&mut self) {
        let mut total = 0;
        for cmd in &self.template_commands {
            match cmd {
                TemplateCommand::Place { .. } => total += 1,
                TemplateCommand::Wall {
                    x1,
                    y1,
                    z1,
                    x2,
                    y2,
                    z2,
                    ..
                } => {
                    let width = (x1.max(x2) - x1.min(x2) + 1) as usize;
                    let height = (y1.max(y2) - y1.min(y2) + 1) as usize;
                    let depth = (z1.max(z2) - z1.min(z2) + 1) as usize;
                    total += width * height * depth;
                }
                _ => {} // Teleport and Look don't create blocks
            }
        }
        self.total_blocks_expected = total;
    }

    pub fn progress_percentage(&self) -> f32 {
        if self.total_blocks_expected == 0 {
            return 100.0;
        }
        (self.blocks_created as f32 / self.total_blocks_expected as f32) * 100.0
    }
}

pub struct AsyncWorldCreationPlugin;

impl Plugin for AsyncWorldCreationPlugin {
    fn build(&self, app: &mut App) {
        info!("🔧 [DEBUG] AsyncWorldCreationPlugin: Registering systems and resources");

        app.init_resource::<WorldCreationTask>()
            .add_event::<WorldCreationCompletedEvent>()
            .add_event::<StartWorldCreationEvent>()
            .add_systems(
                Update,
                (
                    handle_start_world_creation_events,
                    process_world_creation_chunks,
                    log_world_creation_progress,
                    handle_world_creation_completion,
                ),
            );

        info!("✅ [DEBUG] AsyncWorldCreationPlugin: Systems and resources registered successfully");
    }
}

/// System to handle world creation start events
fn handle_start_world_creation_events(
    mut start_events: EventReader<StartWorldCreationEvent>,
    mut world_creation_task: ResMut<WorldCreationTask>,
    mut voxel_world: ResMut<VoxelWorld>,
) {
    for event in start_events.read() {
        info!(
            "🌍 [DEBUG] Starting async world creation: '{}' with template '{}', MCP ID: {:?}",
            event.world_name, event.template, event.mcp_request_id
        );

        // Clear existing world
        voxel_world.blocks.clear();
        info!("🧹 Cleared existing voxel world for new world creation");

        // Create metadata
        let metadata = WorldMetadata {
            name: event.world_name.clone(),
            description: event.description.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        };

        // Parse template commands
        let template_path = format!("scripts/world_templates/{}.txt", event.template);
        info!(
            "🔍 [DEBUG] Attempting to parse template file: {}",
            template_path
        );
        match parse_template_file(&template_path) {
            Ok(commands) => {
                info!(
                    "📜 [DEBUG] Successfully parsed {} template commands for world '{}'",
                    commands.len(),
                    event.world_name
                );

                // Start the async world creation
                *world_creation_task = WorldCreationTask::new(
                    event.world_name.clone(),
                    event.description.clone(),
                    commands,
                    metadata,
                    event.should_transition_to_ingame,
                    event.mcp_request_id.clone(),
                );

                info!(
                    "🚀 [DEBUG] World creation task started: {} commands, ~{} blocks expected, MCP ID: {:?}",
                    world_creation_task.total_commands,
                    world_creation_task.total_blocks_expected,
                    world_creation_task.mcp_request_id
                );
            }
            Err(e) => {
                error!(
                    "❌ [DEBUG] Failed to parse template '{}': {}",
                    event.template, e
                );
                world_creation_task.is_active = false;
                // Still need to send MCP response even on failure
                if let Some(mcp_id) = &event.mcp_request_id {
                    error!(
                        "📤 [DEBUG] Should send error response for MCP ID: {}",
                        mcp_id
                    );
                }
            }
        }
    }
}

/// System to process world creation in chunks to avoid UI blocking
fn process_world_creation_chunks(
    mut world_creation_task: ResMut<WorldCreationTask>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut camera_query: Query<&mut Transform, With<crate::camera_controllers::CameraController>>,
    mut completion_events: EventWriter<WorldCreationCompletedEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !world_creation_task.is_active {
        return;
    }

    // Debug logging every few frames to track progress
    use std::sync::atomic::{AtomicU32, Ordering};
    static DEBUG_COUNTER: AtomicU32 = AtomicU32::new(0);

    let counter = DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
    if counter % 60 == 0 {
        // Log every second at 60fps
        info!(
            "🏗️ [DEBUG] Processing world creation chunks - active: {}, commands: {}/{}, blocks: {}/{}, pending blocks: {}",
            world_creation_task.is_active,
            world_creation_task.processed_commands,
            world_creation_task.total_commands,
            world_creation_task.blocks_created,
            world_creation_task.total_blocks_expected,
            world_creation_task.pending_blocks.len()
        );
    }

    let mut blocks_processed_this_frame = 0;

    // Process pending blocks first (from expanded wall commands)
    while let Some((pos, block_type)) = world_creation_task.pending_blocks.pop_front() {
        voxel_world.blocks.insert(pos, block_type);
        world_creation_task.blocks_created += 1;
        blocks_processed_this_frame += 1;

        if blocks_processed_this_frame >= BLOCKS_PER_FRAME {
            return; // Yield control to avoid blocking
        }
    }

    // Process template commands
    while let Some(command) = world_creation_task.template_commands.pop_front() {
        match command {
            TemplateCommand::Place {
                block_type,
                x,
                y,
                z,
            } => {
                let pos = bevy::math::IVec3::new(x, y, z);
                voxel_world.blocks.insert(pos, block_type);
                world_creation_task.blocks_created += 1;
                blocks_processed_this_frame += 1;
                if world_creation_task.blocks_created % 100 == 0 {
                    info!(
                        "🧱 [DEBUG] Placed block #{} at ({}, {}, {})",
                        world_creation_task.blocks_created, x, y, z
                    );
                }
            }
            TemplateCommand::Wall {
                block_type,
                x1,
                y1,
                z1,
                x2,
                y2,
                z2,
            } => {
                // Expand wall command into individual blocks and add to pending_blocks
                let min_x = x1.min(x2);
                let max_x = x1.max(x2);
                let min_y = y1.min(y2);
                let max_y = y1.max(y2);
                let min_z = z1.min(z2);
                let max_z = z1.max(z2);

                for x in min_x..=max_x {
                    for y in min_y..=max_y {
                        for z in min_z..=max_z {
                            let pos = bevy::math::IVec3::new(x, y, z);
                            world_creation_task
                                .pending_blocks
                                .push_back((pos, block_type));
                        }
                    }
                }
                info!(
                    "🧱 Expanded wall command: {} blocks queued",
                    world_creation_task.pending_blocks.len()
                );
            }
            TemplateCommand::Teleport { x, y, z } => {
                // Set camera position
                for mut camera_transform in camera_query.iter_mut() {
                    camera_transform.translation = bevy::math::Vec3::new(x, y, z);
                    info!("📍 Teleported camera to ({}, {}, {})", x, y, z);
                    break;
                }
            }
            TemplateCommand::Look { yaw, pitch } => {
                // Set camera rotation
                for mut camera_transform in camera_query.iter_mut() {
                    let yaw_rad = yaw.to_radians();
                    let pitch_rad = pitch.to_radians();
                    camera_transform.rotation = bevy::math::Quat::from_euler(
                        bevy::math::EulerRot::YXZ,
                        yaw_rad,
                        pitch_rad,
                        0.0,
                    );
                    info!("👀 Set camera rotation: yaw={}, pitch={}", yaw, pitch);
                    break;
                }
            }
        }

        world_creation_task.processed_commands += 1;

        if blocks_processed_this_frame >= BLOCKS_PER_FRAME {
            return; // Yield control to avoid blocking
        }
    }

    // Check if world creation is complete
    if world_creation_task.template_commands.is_empty()
        && world_creation_task.pending_blocks.is_empty()
    {
        info!(
            "🎉 [DEBUG] World creation completed: '{}' with {} blocks, MCP ID: {:?}",
            world_creation_task.world_name,
            world_creation_task.blocks_created,
            world_creation_task.mcp_request_id
        );

        // Transition to InGame if requested
        if world_creation_task.should_transition_to_ingame {
            next_state.set(GameState::InGame);
            info!("🎮 [DEBUG] Transitioned to InGame state after world creation");
        }

        // Fire completion event
        let completion_event = WorldCreationCompletedEvent {
            world_name: world_creation_task.world_name.clone(),
            blocks_created: world_creation_task.blocks_created,
            template_used: "async".to_string(), // Could track this better
            mcp_request_id: world_creation_task.mcp_request_id.clone(),
        };
        info!(
            "📤 [DEBUG] About to write WorldCreationCompletedEvent: {:?}",
            completion_event.mcp_request_id
        );
        completion_events.write(completion_event);

        // Mark task as inactive
        world_creation_task.is_active = false;
        info!("🛝 [DEBUG] World creation task marked as inactive");
    }
}

/// System to log world creation progress periodically
fn log_world_creation_progress(
    world_creation_task: Res<WorldCreationTask>,
    mut last_log_time: Local<f32>,
    time: Res<Time>,
) {
    if !world_creation_task.is_active {
        return;
    }

    *last_log_time += time.delta_secs();
    if *last_log_time >= 2.0 {
        // Log every 2 seconds
        let progress = world_creation_task.progress_percentage();
        info!(
            "🏗️ World creation progress: {:.1}% ({}/{} blocks, {}/{} commands)",
            progress,
            world_creation_task.blocks_created,
            world_creation_task.total_blocks_expected,
            world_creation_task.processed_commands,
            world_creation_task.total_commands
        );
        *last_log_time = 0.0;
    }
}

/// Parse a template file into commands
fn parse_template_file(template_path: &str) -> Result<Vec<TemplateCommand>, String> {
    let content = std::fs::read_to_string(template_path)
        .map_err(|e| format!("Failed to read template file: {}", e))?;

    let mut commands = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "place" => {
                if parts.len() == 5 {
                    if let (Some(block_type), Ok(x), Ok(y), Ok(z)) = (
                        parse_block_type(parts[1]),
                        parts[2].parse::<i32>(),
                        parts[3].parse::<i32>(),
                        parts[4].parse::<i32>(),
                    ) {
                        commands.push(TemplateCommand::Place {
                            block_type,
                            x,
                            y,
                            z,
                        });
                    } else {
                        return Err(format!("Line {}: Invalid place command", line_num + 1));
                    }
                } else {
                    return Err(format!("Line {}: place requires 4 arguments", line_num + 1));
                }
            }
            "wall" => {
                if parts.len() == 8 {
                    if let (Some(block_type), Ok(x1), Ok(y1), Ok(z1), Ok(x2), Ok(y2), Ok(z2)) = (
                        parse_block_type(parts[1]),
                        parts[2].parse::<i32>(),
                        parts[3].parse::<i32>(),
                        parts[4].parse::<i32>(),
                        parts[5].parse::<i32>(),
                        parts[6].parse::<i32>(),
                        parts[7].parse::<i32>(),
                    ) {
                        commands.push(TemplateCommand::Wall {
                            block_type,
                            x1,
                            y1,
                            z1,
                            x2,
                            y2,
                            z2,
                        });
                    } else {
                        return Err(format!("Line {}: Invalid wall command", line_num + 1));
                    }
                } else {
                    return Err(format!("Line {}: wall requires 7 arguments", line_num + 1));
                }
            }
            "tp" => {
                if parts.len() == 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        commands.push(TemplateCommand::Teleport { x, y, z });
                    } else {
                        return Err(format!("Line {}: Invalid tp command", line_num + 1));
                    }
                } else {
                    return Err(format!("Line {}: tp requires 3 arguments", line_num + 1));
                }
            }
            "look" => {
                if parts.len() == 3 {
                    if let (Ok(yaw), Ok(pitch)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>())
                    {
                        commands.push(TemplateCommand::Look { yaw, pitch });
                    } else {
                        return Err(format!("Line {}: Invalid look command", line_num + 1));
                    }
                } else {
                    return Err(format!("Line {}: look requires 2 arguments", line_num + 1));
                }
            }
            _ => {
                info!(
                    "Unknown template command: {} (line {})",
                    parts[0],
                    line_num + 1
                );
            }
        }
    }

    Ok(commands)
}

/// System to handle world creation completion events and send MCP responses
/// The PendingToolExecutions resource is optional to support non-MCP usage
fn handle_world_creation_completion(
    mut completion_events: EventReader<WorldCreationCompletedEvent>,
    mut pending_executions: Option<ResMut<crate::mcp::mcp_types::PendingToolExecutions>>,
) {
    // Only log when there are actual events to process
    for event in completion_events.read() {
        info!(
            "📬 [DEBUG] Received WorldCreationCompletedEvent: world='{}', blocks={}, mcp_id={:?}",
            event.world_name, event.blocks_created, event.mcp_request_id
        );

        if let Some(mcp_request_id) = &event.mcp_request_id {
            // Only try to send MCP response if MCP is enabled (resource exists)
            if let Some(ref mut pending_executions) = pending_executions {
                let response_message = format!(
                    "World '{}' created successfully with {} blocks using async template system",
                    event.world_name, event.blocks_created
                );

                info!(
                    "📤 [DEBUG] Sending MCP completion response for request {}: {}",
                    mcp_request_id, response_message
                );

                // Check if the request ID exists in pending executions
                if pending_executions.executions.contains_key(mcp_request_id) {
                    info!(
                        "✅ [DEBUG] Found pending execution for request {}",
                        mcp_request_id
                    );
                } else {
                    error!(
                        "❌ [DEBUG] No pending execution found for request {}! Available keys: {:?}",
                        mcp_request_id,
                        pending_executions.executions.keys().collect::<Vec<_>>()
                    );
                }

                // Send the completion response via the pending executions system
                pending_executions.complete_execution(mcp_request_id.clone(), response_message);
                info!(
                    "✅ [DEBUG] Called complete_execution for request {}",
                    mcp_request_id
                );
            } else {
                warn!(
                    "🎯 [DEBUG] MCP world creation completed but MCP plugin not enabled: world='{}', blocks={}",
                    event.world_name, event.blocks_created
                );
            }
        } else {
            info!(
                "🎉 [DEBUG] Non-MCP world creation completed: '{}' with {} blocks",
                event.world_name, event.blocks_created
            );
        }
    }
}

/// Parse block type from string
fn parse_block_type(block_type_str: &str) -> Option<BlockType> {
    match block_type_str.to_lowercase().as_str() {
        "grass" => Some(BlockType::Grass),
        "dirt" => Some(BlockType::Dirt),
        "stone" => Some(BlockType::Stone),
        "quartz_block" => Some(BlockType::QuartzBlock),
        "glass_pane" => Some(BlockType::GlassPane),
        "cyan_terracotta" => Some(BlockType::CyanTerracotta),
        "water" => Some(BlockType::Water),
        _ => None,
    }
}
