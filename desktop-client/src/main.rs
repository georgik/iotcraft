use log::info;
use bevy::prelude::*;
use std::thread;
use std::time::Duration;
use rumqttd::{Broker, Config};
use rumqttc::{Client, MqttOptions, QoS, Event, Outgoing};
use bevy::ui::{Node, PositionType, Val, BackgroundColor};
use bevy::render::view::Visibility;
use bevy::text::{TextFont, TextColor, TextLayout, JustifyText};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::time::{Timer, TimerMode};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::{StandardMaterial, Assets};

#[derive(Resource, Default)]
struct ConsoleState {
    active: bool,
    input: String,
}

#[derive(Resource)]
struct BlinkState {
    blinking: bool,
    timer: Timer,
    last_sent: bool,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            blinking: false,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            last_sent: false,
        }
    }
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
        .insert_resource(BlinkState::default())
        .add_systems(Update, toggle_console)
        .add_systems(Update, console_input)
        .add_systems(Update, command_execution)
        .add_systems(Update, update_console_ui)
        .add_systems(Update, blinking_system)
        .add_systems(Update, blink_publisher_system)
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
        Visibility::default(),
    ));

    // console UI
    commands.spawn((
        Text::new(""),
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(JustifyText::Left),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        Visibility::Hidden,
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
    mut keyboard_input: ResMut<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::F12) && !console_state.active {
        console_state.active = true;
        keyboard_input.clear_just_pressed(KeyCode::F12);
    }
}

fn console_input(
    mut console_state: ResMut<ConsoleState>,
    mut key_evr: EventReader<KeyboardInput>,
) {
    if !console_state.active {
        return;
    }
    for ev in key_evr.read() {
        if ev.state == ButtonState::Pressed {
            if let Some(text) = &ev.text {
                console_state.input.push_str(text);
            }
            if ev.key_code == KeyCode::Backspace {
                console_state.input.pop();
            }
        }
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
    if keyboard_input.just_pressed(KeyCode::Enter) {
        let cmd = console_state.input.trim();
        match cmd {
            "blink start" => {
                blink_state.blinking = true;
                info!("Command executed: {}", cmd);
            }
            "blink stop" => {
                blink_state.blinking = false;
                info!("Command executed: {}", cmd);
            }
            "help" => {
                console_state.input = "Commands:\nhelp\nblink start\nblink stop".to_string();
                // keep console open to show help
                return;
            }
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
        text.0 = console_state.input.clone();
        *visibility = if console_state.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn blinking_system(
    time: Res<Time>,
    mut blink_state: ResMut<BlinkState>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<BlinkCube>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if blink_state.blinking {
        blink_state.timer.tick(time.delta());
        if blink_state.timer.just_finished() {
            for mesh_material in &query {
                // Deref gives us &Handle<StandardMaterial>
                let handle = mesh_material.clone();
                if let Some(mat) = materials.get_mut(&handle) {
                    // toggle between white and red each second
                    mat.base_color = if mat.base_color == Color::WHITE {
                        Color::BLACK
                    } else {
                        Color::WHITE
                    };
                }
            }
        }
    }
}
fn blink_publisher_system(mut blink_state: ResMut<BlinkState>) {
    if blink_state.blinking != blink_state.last_sent {
        let payload = if blink_state.blinking { "ON" } else { "OFF" };
        // sync MQTT client
        let mut opts = MqttOptions::new("bevy_client", "127.0.0.1", 1883);
        opts.set_keep_alive(Duration::from_secs(5));
        let (mut client, mut connection) = Client::new(opts, 10);
        client
            .publish("home/cube/light", QoS::AtMostOnce, false, payload.as_bytes())
            .unwrap();
        // drive until publish is sent
        for notification in connection.iter() {
            if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                break;
            }
        }
        // give broker time
        std::thread::sleep(Duration::from_millis(100));
        blink_state.last_sent = blink_state.blinking;
    }
}