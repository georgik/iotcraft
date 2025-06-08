use bevy::prelude::*;
use bevy::ui::{PositionType, UiRect, Val};
use bevy::input::ButtonInput;
use bevy::time::{Timer, TimerMode};

#[derive(Resource, Default)]
struct ConsoleState {
    active: bool,
    input: String,
}

#[derive(Resource)]
struct BlinkState {
    blinking: bool,
    timer: Timer,
}

#[derive(Component)]
struct BlinkCube;

#[derive(Component)]
struct ConsoleUi;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_cursor)
        .insert_resource(ConsoleState::default())
        .insert_resource(BlinkState {
            blinking: false,
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        })
        .add_systems(Update, toggle_console)
        .add_systems(Update, console_input)
        .add_systems(Update, command_execution)
        .add_systems(Update, update_console_ui)
        .add_systems(Update, blinking_system)
        .run();
}

fn draw_cursor(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    ground: Single<&GlobalTransform, With<Ground>>,
    windows: Query<&Window>,
    mut gizmos: Gizmos,
) {
    let Ok(windows) = windows.single() else {
        return;
    };

    let (camera, camera_transform) = *camera_query;

    let Some(cursor_position) = windows.cursor_position() else {
        return;
    };

    // Calculate a ray pointing from the camera into the world based on the cursor's position.
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Calculate if and where the ray is hitting the ground plane.
    let Some(distance) =
        ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up()))
    else {
        return;
    };
    let point = ray.get_point(distance);

    // Draw a circle just above the ground plane at that position.
    gizmos.circle(
        Isometry3d::new(
            point + ground.up() * 0.01,
            Quat::from_rotation_arc(Vec3::Z, ground.up().as_vec3()),
        ),
        0.2,
        Color::WHITE,
    );
}

#[derive(Component)]
struct Ground;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20., 20.))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Ground,
    ));

    // cube
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        BlinkCube,
    ));

    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 20.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            },
            ..default()
        }),
        ConsoleUi,
    ));

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(15.0, 5.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn toggle_console(
    mut console_state: ResMut<ConsoleState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::T) {
        console_state.active = !console_state.active;
        if !console_state.active {
            console_state.input.clear();
        }
    }
}

fn console_input(
    mut console_state: ResMut<ConsoleState>,
    mut char_evr: EventReader<ReceivedCharacter>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if !console_state.active {
        return;
    }
    for ev in char_evr.iter() {
        let c = ev.char;
        if !c.is_control() {
            console_state.input.push(c);
        }
    }
    if keyboard_input.just_pressed(KeyCode::Back) {
        console_state.input.pop();
    }
}

fn command_execution(
    mut console_state: ResMut<ConsoleState>,
    mut blink_state: ResMut<BlinkState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if !console_state.active {
        return;
    }
    if keyboard_input.just_pressed(KeyCode::Return) {
        match console_state.input.trim() {
            "blink start" => blink_state.blinking = true,
            "blink stop" => blink_state.blinking = false,
            _ => {}
        }
        console_state.input.clear();
        console_state.active = false;
    }
}

fn update_console_ui(
    console_state: Res<ConsoleState>,
    mut query: Query<(&mut Text, &mut Visibility), With<ConsoleUi>>,
) {
    for (mut text, mut visibility) in query.iter_mut() {
        text.sections[0].value = console_state.input.clone();
        visibility.is_visible = console_state.active;
    }
}

fn blinking_system(
    time: Res<Time>,
    mut blink_state: ResMut<BlinkState>,
    mut query: Query<&mut Visibility, With<BlinkCube>>,
) {
    if blink_state.blinking {
        blink_state.timer.tick(time.delta());
        if blink_state.timer.just_finished() {
            for mut vis in query.iter_mut() {
                vis.is_visible = !vis.is_visible;
            }
        }
    }
}