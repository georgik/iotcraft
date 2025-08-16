use crate::environment::BlockType;
use bevy::prelude::*;
use std::collections::HashMap;

/// Resource that holds shared materials for all block types
/// This prevents creating a new material for every block, which is a major performance bottleneck
#[derive(Resource)]
pub struct SharedBlockMaterials {
    pub materials: HashMap<BlockType, Handle<StandardMaterial>>,
    pub shared_mesh: Handle<Mesh>,
}

impl Default for SharedBlockMaterials {
    fn default() -> Self {
        Self {
            materials: HashMap::new(),
            shared_mesh: Handle::default(),
        }
    }
}

impl SharedBlockMaterials {
    /// Get the shared material for a block type
    pub fn get_material(&self, block_type: BlockType) -> Option<Handle<StandardMaterial>> {
        self.materials.get(&block_type).cloned()
    }

    /// Get the shared mesh for blocks
    pub fn get_mesh(&self) -> Handle<Mesh> {
        self.shared_mesh.clone()
    }

    /// Initialize all block materials and the shared mesh
    pub fn initialize(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        meshes: &mut Assets<Mesh>,
        asset_server: &AssetServer,
    ) {
        // Create shared mesh for all blocks
        self.shared_mesh = meshes.add(Cuboid::new(
            crate::environment::CUBE_SIZE,
            crate::environment::CUBE_SIZE,
            crate::environment::CUBE_SIZE,
        ));

        // Create materials for each block type
        let block_configs = [
            (BlockType::Grass, "textures/grass.webp", 0.8, 0.0),
            (BlockType::Dirt, "textures/dirt.webp", 0.8, 0.0),
            (BlockType::Stone, "textures/stone.webp", 0.8, 0.0),
            (
                BlockType::QuartzBlock,
                "textures/quartz_block.webp",
                0.6,
                0.1,
            ), // Slightly smoother and metallic
            (BlockType::GlassPane, "textures/glass_pane.webp", 0.0, 0.0), // Smooth glass
            (
                BlockType::CyanTerracotta,
                "textures/cyan_terracotta.webp",
                0.8,
                0.0,
            ),
            (BlockType::Water, "textures/water.webp", 0.0, 0.0), // Smooth water
        ];

        for (block_type, texture_path, roughness, metallic) in block_configs {
            let texture = asset_server.load(texture_path);
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                perceptual_roughness: roughness,
                metallic,
                ..default()
            });
            self.materials.insert(block_type, material);
        }

        info!(
            "SharedBlockMaterials initialized with {} materials",
            self.materials.len()
        );
    }

    /// Get material with fallback to default if not found
    pub fn get_material_or_default(&self, block_type: BlockType) -> Handle<StandardMaterial> {
        self.materials.get(&block_type).cloned().unwrap_or_else(|| {
            warn!(
                "No material found for block type {:?}, using grass as fallback",
                block_type
            );
            self.materials
                .get(&BlockType::Grass)
                .cloned()
                .unwrap_or_default()
        })
    }
}

/// Plugin to setup shared materials
pub struct SharedMaterialsPlugin;

impl Plugin for SharedMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SharedBlockMaterials::default())
            .add_systems(Startup, setup_shared_materials);
    }
}

/// Setup system that initializes all shared materials
fn setup_shared_materials(
    mut shared_materials: ResMut<SharedBlockMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    shared_materials.initialize(&mut materials, &mut meshes, &asset_server);
}
