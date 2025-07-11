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
use bevy::time::{Timer, TimerMode};
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::{StandardMaterial, Assets};
use bevy_console::{ConsolePlugin, ConsoleCommand, reply, AddConsoleCommand, ConsoleConfiguration, ConsoleOpen, ConsoleSet};
use clap::Parser;

// Console commands for bevy_console
#[derive(Parser, ConsoleCommand)]
#[command(name = "blink")]
struct BlinkCommand {
    /// Action to perform: start or stop
    action: String,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "mqtt")]
struct MqttCommand {
    /// MQTT action: status or reconnect
    action: String,
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

// ConsoleUi component is no longer needed with bevy_console

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
        let (client, mut connection) = Client::new(mqttoptions, 10);
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
        .add_plugins(ConsolePlugin)
        .insert_resource(ConsoleConfiguration {
            keys: vec![KeyCode::F12],
            left_pos: 200.0,
            top_pos: 100.0,
            height: 400.0,
            width: 800.0,
            ..default()
        })
        .add_console_command::<BlinkCommand, _>(handle_blink_command)
        .add_console_command::<MqttCommand, _>(handle_mqtt_command)
        .insert_resource(BlinkState::default())
        .insert_resource(TemperatureResource::default())
        .add_systems(Startup, setup)
        .add_systems(Update, draw_cursor)
        .add_systems(Startup, spawn_mqtt_subscriber)
        .add_systems(Update, blinking_system)
        .add_systems(Update, (blink_publisher_system, rotate_logo_system))
        .add_systems(Update, (update_thermometer_material, update_temperature, update_thermometer_scale))
        .add_systems(Update, manage_camera_controller)
        .add_systems(Update, handle_console_escape.after(ConsoleSet::Commands))
        .add_systems(Update, handle_console_t_key.after(ConsoleSet::Commands))
        .run();
}

fn draw_cursor(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    ground: Single<&GlobalTransform, With<Ground>>,
    windows: Query<&Window>,
    mut gizmos: Gizmos,
    console_open: Res<ConsoleOpen>,
) {
    if console_open.open { return; }

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

    // bevy_console handles the console UI now

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

// Console command handlers
fn handle_blink_command(
    mut log: ConsoleCommand<BlinkCommand>,
    mut blink_state: ResMut<BlinkState>,
) {
    if let Some(Ok(BlinkCommand { action })) = log.take() {
        match action.as_str() {
            "start" => {
                blink_state.blinking = true;
                reply!(log, "Blink started");
                info!("Blink started via console");
            }
            "stop" => {
                blink_state.blinking = false;
                reply!(log, "Blink stopped");
                info!("Blink stopped via console");
            }
            _ => {
                reply!(log, "Usage: blink [start|stop]");
            }
        }
    }
}

fn handle_mqtt_command(
    mut log: ConsoleCommand<MqttCommand>,
    temperature: Res<TemperatureResource>,
) {
    if let Some(Ok(MqttCommand { action })) = log.take() {
        match action.as_str() {
            "status" => {
                let status = if temperature.value.is_some() {
                    "Connected to MQTT broker"
                } else {
                    "Connecting to MQTT broker..."
                };
                reply!(log, "{}", status);
                info!("MQTT status requested via console");
            }
            "temp" => {
                let temp_msg = if let Some(val) = temperature.value {
                    format!("Current temperature: {:.1}°C", val)
                } else {
                    "No temperature data available".to_string()
                };
                reply!(log, "{}", temp_msg);
            }
            _ => {
                reply!(log, "Usage: mqtt [status|temp]");
            }
        }
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
    if let Ok(rx) = receiver.0.lock() {
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

// System to manage camera controller state based on console state
fn manage_camera_controller(
    console_open: Res<ConsoleOpen>,
    mut camera_query: Query<&mut CameraController, With<Camera>>,
) {
    if let Ok(mut camera_controller) = camera_query.single_mut() {
        // Disable camera controller when console is open
        camera_controller.enabled = !console_open.open;
    }
}

// System to handle ESC key to close console
fn handle_console_escape(
    mut console_open: ResMut<ConsoleOpen>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) && console_open.open {
        console_open.open = false;
    }
}

// System to handle 't' key to open console (only when closed)
fn handle_console_t_key(
    mut console_open: ResMut<ConsoleOpen>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Only open console with 't' when it's currently closed
    if keyboard_input.just_pressed(KeyCode::KeyT) && !console_open.open {
        console_open.open = true;
    }
}
