use rumqttc::Incoming;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
mod camera_controllers;
use bevy::prelude::ClearColor;
use camera_controllers::{CameraController, CameraControllerPlugin};
use log::info;
use bevy::prelude::*;
use bevy::asset::Handle;
use bevy::image::Image;
use std::time::Duration;
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
    light_state: bool,
    last_sent: bool,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            blinking: false,
            light_state: false,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            last_sent: false,
        }
    }
}

#[derive(Component)]
struct BlinkCube;

#[derive(Component)]
struct LogoCube;

#[derive(Component)]
struct ConsoleUi;

#[derive(Resource)]
struct TemperatureResource {
    value: Option<f32>,
}

impl Default for TemperatureResource {
    fn default() -> Self {
        Self { value: None }
    }
}

#[derive(Resource)]
struct TemperatureReceiver(Mutex<Receiver<f32>>);

#[derive(Component)]
struct Thermometer;

#[derive(Resource)]
struct ThermometerMaterial(pub Handle<StandardMaterial>);

/// Spawn a background thread to subscribe to the temperature topic and feed readings into the channel.
fn spawn_mqtt_subscriber(mut commands: Commands) {
    let (tx, rx) = mpsc::channel::<f32>();
    thread::spawn(move || {
        let mut mqttoptions = MqttOptions::new("desktop-subscriber", "127.0.0.1", 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (mut client, mut connection) = Client::new(mqttoptions, 10);
        client.subscribe("home/sensor/temperature", QoS::AtMostOnce).unwrap();
        for notification in connection.iter() {
            if let Ok(Event::Incoming(Incoming::Publish(p))) = notification {
                if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                    if let Ok(val) = s.parse::<f32>() {
                        let _ = tx.send(val);
                    }
                }
            }
        }
    });
    commands.insert_resource(TemperatureReceiver(Mutex::new(rx)));
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_cursor)
        .insert_resource(ConsoleState::default())
        .insert_resource(BlinkState::default())
        .insert_resource(TemperatureResource::default())
        .add_systems(Update, toggle_console)
        .add_systems(Startup, spawn_mqtt_subscriber)
        .add_systems(Update, console_input)
        .add_systems(Update, command_execution)
        .add_systems(Update, update_console_ui)
        .add_systems(Update, blinking_system)
        .add_systems(Update, (blink_publisher_system, rotate_logo_system))
        .add_systems(Update, (update_thermometer_material, update_temperature, update_thermometer_scale))
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
    // grass texture for ground
    let grass_texture: Handle<Image> = asset_server.load("textures/grass.png");
    let grass_material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(grass_texture),
        ..default()
    });

    // ground plane with grass texture
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(grass_material_handle.clone()),
        Ground,
    ));

    // cube with lamp texture
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let lamp_texture: Handle<Image> = asset_server.load("textures/lamp.png");
    let lamp_material = materials.add(StandardMaterial {
        base_color_texture: Some(lamp_texture),
        base_color: Color::srgb(0.2, 0.2, 0.2),
        ..default()
    });
    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(lamp_material),
        Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        BlinkCube,
        Visibility::default(),
    ));

    // block with Espressif logo texture
    let block_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let esp_logo_texture: Handle<Image> = asset_server.load("textures/espressif.png");
    let esp_logo_material = materials.add(StandardMaterial {
        base_color_texture: Some(esp_logo_texture),
        base_color: Color::WHITE,
        ..default()
    });
    commands.spawn((
        Mesh3d(block_mesh),
        MeshMaterial3d(esp_logo_material),
        Transform::from_translation(Vec3::new(3.0, 6.5, 2.0)),
        Visibility::default(),
        LogoCube,
    ));

    // thermometer 3D indicator
    let thermo_mesh = meshes.add(Cuboid::new(0.2, 5.0, 0.2));
    let thermo_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        ..default()
    });
    commands.spawn((
        Mesh3d(thermo_mesh),
        MeshMaterial3d(thermo_handle.clone()),
        Transform::from_translation(Vec3::new(-3.0, 2.5, 2.0)),
        Thermometer,
    ));
    commands.insert_resource(ThermometerMaterial(thermo_handle));

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
        CameraController::default(),
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
                    // toggle between dimmed and bright lamp each second
                    if mat.base_color == Color::WHITE {
                        blink_state.light_state = false;
                        // dim the lamp
                        mat.base_color = Color::srgb(0.2, 0.2, 0.2);
                    } else {
                        blink_state.light_state = true;
                        // full brightness
                        mat.base_color = Color::WHITE;
                    }
                }
            }
        }
    }
}
fn blink_publisher_system(mut blink_state: ResMut<BlinkState>) {
    if blink_state.light_state != blink_state.last_sent {
        let payload = if blink_state.light_state { "ON" } else { "OFF" };
        // sync MQTT client
        let mut opts = MqttOptions::new("bevy_client", "127.0.0.1", 1883);
        opts.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(opts, 10);
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
        blink_state.last_sent = blink_state.light_state;
    }
}

fn rotate_logo_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<LogoCube>>,
) {
    for mut transform in &mut query {
        // rotate slowly around the Y axis
        transform.rotate_y(time.delta_secs() * 0.5);
        transform.rotate_x(time.delta_secs() * 0.5);
    }
}

fn update_temperature(
    mut temp_res: ResMut<TemperatureResource>,
    receiver: Res<TemperatureReceiver>,
) {
    if let Ok(mut rx) = receiver.0.lock() {
        if let Ok(val) = rx.try_recv() {
            temp_res.value = Some(val);
        }
    }
}

fn update_thermometer_material(
    temp: Res<TemperatureResource>,
    thermo: Res<ThermometerMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(mat) = materials.get_mut(&thermo.0) {
        if temp.value.is_some() {
            // reading received: red
            mat.base_color = Color::srgb(1.0, 0.0, 0.0);
        } else {
            // no reading yet: gray
            mat.base_color = Color::srgb(0.2, 0.2, 0.2);
        }
    }
}

fn update_thermometer_scale(
    temp: Res<TemperatureResource>,
    mut query: Query<&mut Transform, With<Thermometer>>,
) {
    if let Some(value) = temp.value {
        for mut transform in &mut query {
            // Scale Y axis proportional to temperature value, clamped to a reasonable range
            let scale_y = (value / 100.0).clamp(0.1, 2.0);
            transform.scale = Vec3::new(1.0, scale_y, 1.0);
        }
    }
}