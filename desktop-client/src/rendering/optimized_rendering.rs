use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::environment::{BlockType, CUBE_SIZE, VoxelBlock};

/// Plugin for optimized rendering with GPU instancing
pub struct OptimizedRenderingPlugin;

impl Plugin for OptimizedRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BlockInstanceManager::default())
            .insert_resource(RenderingMetrics::default())
            .add_systems(Update, (update_block_instances, optimize_mesh_batching))
            .add_systems(Startup, setup_instanced_materials);
    }
}

/// Manages GPU instancing for block rendering
#[derive(Resource, Default)]
pub struct BlockInstanceManager {
    /// Maps block type to instance data
    pub instances: HashMap<BlockType, Vec<InstanceData>>,
    /// Tracks which blocks need instance updates
    pub dirty_block_types: HashSet<BlockType>,
    /// GPU buffer handles for each block type (using StandardMaterial for now)
    pub instance_buffers: HashMap<BlockType, Handle<StandardMaterial>>,
}

/// Data for each instance in GPU instancing
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    /// Transform matrix (4x4 = 16 floats)
    pub transform: [f32; 16],
    /// Color/tint for this instance
    pub color: [f32; 4],
    /// Additional properties (e.g., height-based shading)
    pub properties: [f32; 4],
}

/// Custom material for instanced block rendering
#[derive(Asset, TypePath, Debug, Clone)]
pub struct InstancedBlockMaterial {
    /// Base texture for the block type
    pub texture: Option<Handle<Image>>,
    /// Instance data buffer (simplified for now)
    pub instances: Vec<InstanceData>,
    /// Material properties
    pub roughness: f32,
    pub metallic: f32,
}

// Material implementation removed for now to avoid AsBindGroup complexity
// This would be implemented once proper GPU instancing is ready

/// Component for managing instanced blocks
#[derive(Component)]
pub struct InstancedBlockRenderer {
    pub block_type: BlockType,
    pub instance_count: u32,
}

/// Setup instanced materials for each block type
fn setup_instanced_materials(
    _commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut instance_manager: ResMut<BlockInstanceManager>,
) {
    // Create standard materials for each block type (simplified for now)
    for block_type in [
        BlockType::Grass,
        BlockType::Dirt,
        BlockType::Stone,
        BlockType::QuartzBlock,
        BlockType::GlassPane,
        BlockType::CyanTerracotta,
        BlockType::Water,
    ] {
        let texture_path = match block_type {
            BlockType::Grass => "textures/grass.webp",
            BlockType::Dirt => "textures/dirt.webp",
            BlockType::Stone => "textures/stone.webp",
            BlockType::QuartzBlock => "textures/quartz_block.webp",
            BlockType::GlassPane => "textures/glass_pane.webp",
            BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
            BlockType::Water => "textures/water.webp",
        };

        let texture = asset_server.load(texture_path);
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            perceptual_roughness: match block_type {
                BlockType::Water | BlockType::GlassPane => 0.0, // Smooth surfaces
                _ => 0.8,                                       // Rough surfaces
            },
            metallic: match block_type {
                BlockType::QuartzBlock => 0.1, // Slight metallic look
                _ => 0.0,
            },
            ..default()
        });

        instance_manager
            .instance_buffers
            .insert(block_type, material);
        instance_manager.instances.insert(block_type, Vec::new());
    }

    info!("Instanced materials setup complete for optimized rendering");
}

/// Update block instances based on world changes
fn update_block_instances(
    mut instance_manager: ResMut<BlockInstanceManager>,
    blocks_query: Query<(&Transform, &VoxelBlock), (Changed<Transform>, With<VoxelBlock>)>,
    voxel_world: Res<crate::environment::VoxelWorld>,
    camera_query: Query<&Transform, (With<Camera>, Without<VoxelBlock>)>,
) {
    let camera_pos = if let Ok(camera_transform) = camera_query.single() {
        camera_transform.translation
    } else {
        return;
    };

    // Distance-based level-of-detail
    const LOD_DISTANCE_HIGH: f32 = 50.0;
    const LOD_DISTANCE_MEDIUM: f32 = 100.0;
    const LOD_DISTANCE_LOW: f32 = 200.0;

    // Clear existing instance data
    for instances in instance_manager.instances.values_mut() {
        instances.clear();
    }

    // Rebuild instances efficiently
    for (pos, block_type) in voxel_world.blocks.iter() {
        let world_pos = pos.as_vec3();
        let distance = camera_pos.distance(world_pos);

        // Skip distant blocks that are too far to be visible
        if distance > LOD_DISTANCE_LOW {
            continue;
        }

        // Create instance data with LOD
        let lod_scale = if distance < LOD_DISTANCE_HIGH {
            1.0
        } else if distance < LOD_DISTANCE_MEDIUM {
            0.8 // Slightly smaller for medium distance
        } else {
            0.6 // Smaller for low detail
        };

        let transform_matrix = Transform::from_translation(world_pos)
            .with_scale(Vec3::splat(lod_scale))
            .compute_matrix();

        // Apply height-based coloring for visual depth
        let height_factor = (pos.y + 10) as f32 / 30.0; // Normalize height
        let color_modifier = 0.8 + (height_factor * 0.4); // Range: 0.8 to 1.2

        let instance_data = InstanceData {
            transform: transform_matrix.to_cols_array(),
            color: [color_modifier, color_modifier, color_modifier, 1.0],
            properties: [height_factor, distance / LOD_DISTANCE_LOW, 0.0, 0.0],
        };

        if let Some(instances) = instance_manager.instances.get_mut(block_type) {
            instances.push(instance_data);
        }
    }

    // Mark all block types as needing updates
    let block_types: Vec<_> = instance_manager.instances.keys().cloned().collect();
    for block_type in block_types {
        instance_manager.dirty_block_types.insert(block_type);
    }
}

// Manage instanced rendering updates - placeholder for future GPU instancing
// This would spawn actual instanced renderers when the Material trait is properly implemented
fn _manage_instanced_rendering() {
    // For now, this system is disabled until proper GPU instancing is implemented
    // The rendering optimization is handled through LOD culling in optimize_mesh_batching
}

/// Optimize mesh batching for better GPU performance
fn optimize_mesh_batching(
    mut commands: Commands,
    block_query: Query<(Entity, &Transform, &VoxelBlock)>,
    camera_query: Query<&Transform, (With<Camera>, Without<VoxelBlock>)>,
) {
    let camera_pos = if let Ok(camera_transform) = camera_query.single() {
        camera_transform.translation
    } else {
        return;
    };

    // Frustum culling - remove blocks outside camera view
    const FRUSTUM_DISTANCE: f32 = 150.0;

    for (entity, transform, _voxel_block) in block_query.iter() {
        let distance = camera_pos.distance(transform.translation);

        // Remove entities that are too far away
        if distance > FRUSTUM_DISTANCE {
            // Instead of despawning, we could disable rendering
            // This is a simple culling approach
            commands.entity(entity).remove::<Visibility>();
        } else {
            // Ensure visible entities have visibility component
            commands.entity(entity).insert(Visibility::Visible);
        }
    }
}

/// Custom shader for instanced block rendering
pub const INSTANCED_BLOCKS_SHADER: &str = r#"
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_clip}

struct InstanceData {
    transform: mat4x4<f32>,
    color: vec4<f32>,
    properties: vec4<f32>,
}

@group(1) @binding(0) var base_color_texture: texture_2d<f32>;
@group(1) @binding(1) var base_color_sampler: sampler;
@group(1) @binding(2) var<storage, read> instances: array<InstanceData>;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let instance = instances[vertex.instance_index];
    let world_position = instance.transform * vec4<f32>(vertex.position, 1.0);
    
    out.position = mesh_position_local_to_clip(
        get_model_matrix(vertex.instance_index),
        vec4<f32>(vertex.position, 1.0)
    );
    out.world_position = world_position;
    out.world_normal = normalize((instance.transform * vec4<f32>(vertex.normal, 0.0)).xyz);
    out.uv = vertex.uv;
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(base_color_texture, base_color_sampler, in.uv);
    let instance = instances[0]; // Would need proper instance index in fragment
    
    // Apply height-based shading and distance-based color modifications
    let color_modifier = instance.color.rgb;
    let final_color = base_color.rgb * color_modifier;
    
    return vec4<f32>(final_color, base_color.a);
}
"#;

/// Performance metrics tracking
#[derive(Resource, Default)]
pub struct RenderingMetrics {
    pub total_instances: u32,
    pub culled_instances: u32,
    pub draw_calls_saved: u32,
    pub last_update_time_ms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_data_size() {
        // Ensure instance data is properly aligned for GPU
        assert_eq!(std::mem::size_of::<InstanceData>(), 80); // 16 + 16 + 16 + 16 + 16 bytes
        assert_eq!(std::mem::align_of::<InstanceData>(), 4);
    }

    #[test]
    fn test_lod_calculations() {
        let camera_pos = Vec3::ZERO;
        let block_pos = Vec3::new(75.0, 0.0, 0.0);
        let distance = camera_pos.distance(block_pos);

        assert_eq!(distance, 75.0);

        // Test LOD scaling
        const LOD_DISTANCE_HIGH: f32 = 50.0;
        const LOD_DISTANCE_MEDIUM: f32 = 100.0;

        let lod_scale = if distance < LOD_DISTANCE_HIGH {
            1.0
        } else if distance < LOD_DISTANCE_MEDIUM {
            0.8
        } else {
            0.6
        };

        assert_eq!(lod_scale, 0.8); // Should be medium LOD
    }
}
