use bevy::prelude::*;

/// Component to mark entities as part of a voxel-style avatar
#[derive(Component)]
pub struct PlayerAvatar {
    pub player_id: String,
    pub player_name: String,
}

/// Components for different body parts
#[derive(Component)]
pub struct AvatarHead;

#[derive(Component)]
pub struct AvatarBody;

#[derive(Component)]
pub struct AvatarArm {
    pub is_right: bool,
}

#[derive(Component)]
pub struct AvatarLeg {
    pub is_right: bool,
}

#[derive(Component)]
pub struct AvatarEye {
    pub is_right: bool,
}

#[derive(Component)]
pub struct AvatarHair;

#[derive(Component)]
pub struct AvatarNose;

#[derive(Component)]
pub struct AvatarMouth;

/// Plugin for player avatar functionality
pub struct PlayerAvatarPlugin;

impl Plugin for PlayerAvatarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, animate_avatar_parts);
    }
}

/// Avatar dimensions following voxel game proportions (scaled to fit our world)
pub struct AvatarDimensions;

impl AvatarDimensions {
    // Classic voxel player is 32 pixels tall, we scale to ~1.8 units to match our cube
    const SCALE: f32 = 1.8 / 32.0;

    // Head: 8x8x8 pixels
    pub const HEAD_SIZE: f32 = 8.0 * Self::SCALE;

    // Body: 8x12x4 pixels (width x height x depth)
    pub const BODY_WIDTH: f32 = 8.0 * Self::SCALE;
    pub const BODY_HEIGHT: f32 = 12.0 * Self::SCALE;
    pub const BODY_DEPTH: f32 = 4.0 * Self::SCALE;

    // Arms: 3.8x12x3.8 pixels (slightly smaller to avoid z-fighting)
    pub const ARM_WIDTH: f32 = 3.8 * Self::SCALE;
    pub const ARM_HEIGHT: f32 = 12.0 * Self::SCALE;
    pub const ARM_DEPTH: f32 = 3.8 * Self::SCALE;

    // Legs: 4x12x4 pixels
    pub const LEG_WIDTH: f32 = 4.0 * Self::SCALE;
    pub const LEG_HEIGHT: f32 = 12.0 * Self::SCALE;
    pub const LEG_DEPTH: f32 = 4.0 * Self::SCALE;
}

/// Spawn a complete voxel-style avatar at the given position
pub fn spawn_player_avatar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    player_id: String,
    player_name: String,
) -> Entity {
    // Create materials for different body parts
    let head_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.8, 0.69), // Skin color
        ..default()
    });

    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.8), // Blue shirt
        ..default()
    });

    let arm_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.8, 0.69), // Skin color for arms
        ..default()
    });

    let leg_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.6), // Dark blue pants
        ..default()
    });

    // Eye material (dark for pupils)
    let eye_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.1), // Dark eyes
        ..default()
    });

    // Hair material (brown)
    let hair_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.2, 0.1), // Brown hair
        ..default()
    });

    // Nose material (skin color, slightly darker)
    let nose_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.75, 0.65), // Slightly darker skin tone
        ..default()
    });

    // Mouth material (dark red)
    let mouth_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.2, 0.2), // Dark red mouth
        ..default()
    });

    // Create meshes for different body parts
    let head_mesh = meshes.add(Cuboid::new(
        AvatarDimensions::HEAD_SIZE,
        AvatarDimensions::HEAD_SIZE,
        AvatarDimensions::HEAD_SIZE,
    ));

    let body_mesh = meshes.add(Cuboid::new(
        AvatarDimensions::BODY_WIDTH,
        AvatarDimensions::BODY_HEIGHT,
        AvatarDimensions::BODY_DEPTH,
    ));

    let arm_mesh = meshes.add(Cuboid::new(
        AvatarDimensions::ARM_WIDTH,
        AvatarDimensions::ARM_HEIGHT,
        AvatarDimensions::ARM_DEPTH,
    ));

    let leg_mesh = meshes.add(Cuboid::new(
        AvatarDimensions::LEG_WIDTH,
        AvatarDimensions::LEG_HEIGHT,
        AvatarDimensions::LEG_DEPTH,
    ));

    // Eye mesh (small cubes)
    let eye_size = AvatarDimensions::HEAD_SIZE * 0.15;
    let eye_mesh = meshes.add(Cuboid::new(eye_size, eye_size, eye_size));

    // Hair mesh (wider than head, but shorter to avoid eye overlap)
    let hair_width = AvatarDimensions::HEAD_SIZE * 1.1;
    let hair_height = AvatarDimensions::HEAD_SIZE * 0.3; // Reduced from 0.6 to 0.3
    let hair_depth = AvatarDimensions::HEAD_SIZE * 1.1;
    let hair_mesh = meshes.add(Cuboid::new(hair_width, hair_height, hair_depth));

    // Nose mesh (small and narrow)
    let nose_width = AvatarDimensions::HEAD_SIZE * 0.08;
    let nose_height = AvatarDimensions::HEAD_SIZE * 0.12;
    let nose_depth = AvatarDimensions::HEAD_SIZE * 0.1;
    let nose_mesh = meshes.add(Cuboid::new(nose_width, nose_height, nose_depth));

    // Mouth mesh (small and wide)
    let mouth_width = AvatarDimensions::HEAD_SIZE * 0.25;
    let mouth_height = AvatarDimensions::HEAD_SIZE * 0.08;
    let mouth_depth = AvatarDimensions::HEAD_SIZE * 0.05;
    let mouth_mesh = meshes.add(Cuboid::new(mouth_width, mouth_height, mouth_depth));

    // Spawn the main avatar entity (this will be the parent)
    let avatar_entity = commands
        .spawn((
            Transform::from_translation(position),
            GlobalTransform::default(),
            PlayerAvatar {
                player_id: player_id.clone(),
                player_name: player_name.clone(),
            },
            Name::new(format!("PlayerAvatar-{}", player_name)),
            Visibility::default(),
        ))
        .id();

    // Spawn head (positioned above body center)
    let head_offset = Vec3::new(
        0.0,
        AvatarDimensions::BODY_HEIGHT / 2.0 + AvatarDimensions::HEAD_SIZE / 2.0,
        0.0,
    );
    let head_entity = commands
        .spawn((
            Mesh3d(head_mesh),
            MeshMaterial3d(head_material),
            Transform::from_translation(head_offset),
            AvatarHead,
            Name::new("Head"),
        ))
        .id();

    // Spawn body (centered)
    let body_entity = commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_material),
            Transform::from_translation(Vec3::ZERO),
            AvatarBody,
            Name::new("Body"),
        ))
        .id();

    // Spawn right arm (positioned to the right of the body)
    let right_arm_offset = Vec3::new(
        AvatarDimensions::BODY_WIDTH / 2.0 + AvatarDimensions::ARM_WIDTH / 2.0,
        0.0,
        0.0,
    );
    let right_arm_entity = commands
        .spawn((
            Mesh3d(arm_mesh.clone()),
            MeshMaterial3d(arm_material.clone()),
            Transform::from_translation(right_arm_offset),
            AvatarArm { is_right: true },
            Name::new("RightArm"),
        ))
        .id();

    // Spawn left arm (positioned to the left of the body)
    let left_arm_offset = Vec3::new(
        -(AvatarDimensions::BODY_WIDTH / 2.0 + AvatarDimensions::ARM_WIDTH / 2.0),
        0.0,
        0.0,
    );
    let left_arm_entity = commands
        .spawn((
            Mesh3d(arm_mesh),
            MeshMaterial3d(arm_material),
            Transform::from_translation(left_arm_offset),
            AvatarArm { is_right: false },
            Name::new("LeftArm"),
        ))
        .id();

    // Spawn right leg (positioned below and to the right of body center)
    let right_leg_offset = Vec3::new(
        AvatarDimensions::LEG_WIDTH / 2.0,
        -(AvatarDimensions::BODY_HEIGHT / 2.0 + AvatarDimensions::LEG_HEIGHT / 2.0),
        0.0,
    );
    let right_leg_entity = commands
        .spawn((
            Mesh3d(leg_mesh.clone()),
            MeshMaterial3d(leg_material.clone()),
            Transform::from_translation(right_leg_offset),
            AvatarLeg { is_right: true },
            Name::new("RightLeg"),
        ))
        .id();

    // Spawn left leg (positioned below and to the left of body center)
    let left_leg_offset = Vec3::new(
        -AvatarDimensions::LEG_WIDTH / 2.0,
        -(AvatarDimensions::BODY_HEIGHT / 2.0 + AvatarDimensions::LEG_HEIGHT / 2.0),
        0.0,
    );
    let left_leg_entity = commands
        .spawn((
            Mesh3d(leg_mesh),
            MeshMaterial3d(leg_material),
            Transform::from_translation(left_leg_offset),
            AvatarLeg { is_right: false },
            Name::new("LeftLeg"),
        ))
        .id();

    // Spawn eyes on the head (positioned on the front face of the head)
    let eye_offset_y = AvatarDimensions::HEAD_SIZE * 0.15; // Slightly above center
    let eye_offset_x = AvatarDimensions::HEAD_SIZE * 0.2; // Distance from center
    let eye_offset_z = AvatarDimensions::HEAD_SIZE * 0.48; // Just outside the head surface

    // Right eye
    let right_eye_offset = head_offset + Vec3::new(eye_offset_x, eye_offset_y, eye_offset_z);
    let right_eye_entity = commands
        .spawn((
            Mesh3d(eye_mesh.clone()),
            MeshMaterial3d(eye_material.clone()),
            Transform::from_translation(right_eye_offset),
            AvatarEye { is_right: true },
            Name::new("RightEye"),
        ))
        .id();

    // Left eye
    let left_eye_offset = head_offset + Vec3::new(-eye_offset_x, eye_offset_y, eye_offset_z);
    let left_eye_entity = commands
        .spawn((
            Mesh3d(eye_mesh),
            MeshMaterial3d(eye_material),
            Transform::from_translation(left_eye_offset),
            AvatarEye { is_right: false },
            Name::new("LeftEye"),
        ))
        .id();

    // Spawn hair on top of head (slightly higher to avoid plane overlap)
    let hair_offset = head_offset + Vec3::new(0.0, AvatarDimensions::HEAD_SIZE * 0.42, 0.0);
    let hair_entity = commands
        .spawn((
            Mesh3d(hair_mesh),
            MeshMaterial3d(hair_material),
            Transform::from_translation(hair_offset),
            AvatarHair,
            Name::new("Hair"),
        ))
        .id();

    // Spawn nose in center of face
    let nose_offset = head_offset
        + Vec3::new(
            0.0,
            -AvatarDimensions::HEAD_SIZE * 0.1,
            AvatarDimensions::HEAD_SIZE * 0.48,
        );
    let nose_entity = commands
        .spawn((
            Mesh3d(nose_mesh),
            MeshMaterial3d(nose_material),
            Transform::from_translation(nose_offset),
            AvatarNose,
            Name::new("Nose"),
        ))
        .id();

    // Spawn mouth below nose
    let mouth_offset = head_offset
        + Vec3::new(
            0.0,
            -AvatarDimensions::HEAD_SIZE * 0.25,
            AvatarDimensions::HEAD_SIZE * 0.48,
        );
    let mouth_entity = commands
        .spawn((
            Mesh3d(mouth_mesh),
            MeshMaterial3d(mouth_material),
            Transform::from_translation(mouth_offset),
            AvatarMouth,
            Name::new("Mouth"),
        ))
        .id();

    // Set up parent-child relationships
    commands.entity(avatar_entity).add_children(&[
        head_entity,
        body_entity,
        right_arm_entity,
        left_arm_entity,
        right_leg_entity,
        left_leg_entity,
        right_eye_entity,
        left_eye_entity,
        hair_entity,
        nose_entity,
        mouth_entity,
    ]);

    avatar_entity
}

/// Simple animation system for avatar parts (subtle swaying motion)
fn animate_avatar_parts(
    time: Res<Time>,
    mut arms_query: Query<
        (&mut Transform, &AvatarArm),
        (
            With<AvatarArm>,
            Without<AvatarLeg>,
            Without<AvatarHead>,
            Without<AvatarBody>,
        ),
    >,
    mut legs_query: Query<
        (&mut Transform, &AvatarLeg),
        (
            With<AvatarLeg>,
            Without<AvatarArm>,
            Without<AvatarHead>,
            Without<AvatarBody>,
        ),
    >,
) {
    let time_secs = time.elapsed_secs();

    // Subtle arm swaying front-to-back (X-axis rotation)
    for (mut arm_transform, arm) in arms_query.iter_mut() {
        let phase = if arm.is_right {
            0.0
        } else {
            std::f32::consts::PI
        }; // Opposite phases
        let sway = (time_secs * 2.0 + phase).sin() * 0.3;
        arm_transform.rotation = Quat::from_rotation_x(sway);
    }

    // Subtle leg swaying front-to-back (opposite to arms)
    for (mut leg_transform, leg) in legs_query.iter_mut() {
        let phase = if leg.is_right {
            std::f32::consts::PI
        } else {
            0.0
        }; // Opposite to arms and each other
        let sway = (time_secs * 2.0 + phase).sin() * 0.15;
        leg_transform.rotation = Quat::from_rotation_x(sway);
    }
}

/// Update avatar position and rotation
pub fn update_player_avatar_transform(
    avatar_entity: Entity,
    position: Vec3,
    yaw: f32,
    _pitch: f32, // Not used for now, but available for future head movement
    commands: &mut Commands,
    avatar_query: &Query<&Transform, With<PlayerAvatar>>,
) {
    // Update the main avatar entity's transform
    if let Ok(_current_transform) = avatar_query.get(avatar_entity) {
        commands.entity(avatar_entity).insert(
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
        );
    }
}
