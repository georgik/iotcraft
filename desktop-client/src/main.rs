use bevy::prelude::*;
use camera_controllers::{CameraController, CameraControllerPlugin};
use bevy_console::{ConsolePlugin, ConsoleCommand, reply, AddConsoleCommand, ConsoleConfiguration, ConsoleOpen, ConsoleSet, PrintConsoleLine};
use clap::Parser;
use log::{info, error};
use serde_json::json;
use std::time::Duration;
use std::fs;
use rumqttc::{Client, MqttOptions, QoS, Event, Outgoing};
use rumqttc::Incoming;
use std::sync::mpsc::Receiver;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::collections::HashSet;
use bevy::pbr::MeshMaterial3d;

mod camera_controllers;

// CLI arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Script file to execute on startup
    #[arg(short, long)]
    script: Option<String>,
}

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

#[derive(Parser, ConsoleCommand)]
#[command(name = "load")]
struct LoadCommand {
    /// Script file to load and execute
    filename: String,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "spawn")]
struct SpawnCommand {
    /// Device ID
    device_id: String,
    /// X coordinate
    x: f32,
    /// Y coordinate
    y: f32,
    /// Z coordinate
    z: f32,
}

#[derive(Resource)]
struct DevicesTracker {
    spawned_devices: HashSet<String>,
}

#[derive(Resource)]
struct DeviceAnnouncementReceiver(Mutex<Receiver<String>>);

#[derive(Component)]
struct DeviceEntity {
    device_id: String,
    device_type: String,
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

    // Create device announcement channel
    let (device_tx, device_rx) = mpsc::channel::<String>();
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));
    commands.insert_resource(DevicesTracker { spawned_devices: HashSet::new() });
    
    // Subscribe to device announcements
    thread::spawn(move || {
        let mut mqttoptions = MqttOptions::new("device-subscriber", "127.0.0.1", 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 10);
        client.subscribe("devices/announce", QoS::AtMostOnce).unwrap();
        
        for notification in connection.iter() {
            if let Ok(Event::Incoming(Incoming::Publish(p))) = notification {
                if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                    let _ = device_tx.send(s);
                }
            }
        }
    });
}

// Script execution system
#[derive(Resource)]
struct ScriptExecutor {
    commands: Vec<String>,
    current_index: usize,
    delay_timer: Timer,
    startup_script: Option<String>,
    execute_startup: bool,
}

#[derive(Resource)]
struct PendingCommands {
    commands: Vec<String>,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            current_index: 0,
            delay_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            startup_script: None,
            execute_startup: false,
        }
    }
}

fn execute_script(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

fn handle_load_command(
    mut log: ConsoleCommand<LoadCommand>,
    mut script_executor: ResMut<ScriptExecutor>,
) {
    if let Some(Ok(LoadCommand { filename })) = log.take() {
        info!("Console command: load {}", filename);
        match fs::read_to_string(&filename) {
            Ok(content) => {
                let commands = execute_script(&content);
                script_executor.commands = commands;
                script_executor.current_index = 0;
                reply!(log, "Loaded {} commands from {}", script_executor.commands.len(), filename);
                info!("Loaded script file: {}", filename);
            }
            Err(e) => {
                reply!(log, "Error loading script {}: {}", filename, e);
            }
        }
    }
}

fn script_execution_system(
    mut script_executor: ResMut<ScriptExecutor>,
    time: Res<Time>,
    mut pending_commands: ResMut<PendingCommands>,
) {
    // Handle startup script execution
    if script_executor.execute_startup {
        if let Some(ref startup_script) = script_executor.startup_script.clone() {
            match fs::read_to_string(startup_script) {
                Ok(content) => {
                    let commands = execute_script(&content);
                    script_executor.commands = commands;
                    script_executor.current_index = 0;
                    info!("Loaded startup script: {}", startup_script);
                }
                Err(e) => {
                    error!("Error loading startup script {}: {}", startup_script, e);
                }
            }
        }
        script_executor.execute_startup = false;
    }
    
    // Execute commands from script
    if !script_executor.commands.is_empty() && script_executor.current_index < script_executor.commands.len() {
        script_executor.delay_timer.tick(time.delta());
        
        if script_executor.delay_timer.just_finished() {
            let command = &script_executor.commands[script_executor.current_index];
            
            // Log the command execution
            info!("Executing script command: {}", command);
            
            // Queue the command for execution
            pending_commands.commands.push(command.clone());
            
            script_executor.current_index += 1;
            
            // Check if we've finished executing all commands
            if script_executor.current_index >= script_executor.commands.len() {
                script_executor.commands.clear();
                script_executor.current_index = 0;
                info!("Script execution completed");
            }
        }
    }
}

fn execute_pending_commands(
    mut pending_commands: ResMut<PendingCommands>,
    mut print_console_line: EventWriter<PrintConsoleLine>,
    mut blink_state: ResMut<BlinkState>,
    temperature: Res<TemperatureResource>,
) {
    for command in pending_commands.commands.drain(..) {
        info!("Executing queued command: {}", command);

        // Parse command string and dispatch to appropriate handler
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "blink" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "start" => {
                            blink_state.blinking = true;
                            print_console_line.write(PrintConsoleLine::new("Blink started".to_string()));
                            info!("Blink started via script");
                        }
                        "stop" => {
                            blink_state.blinking = false;
                            print_console_line.write(PrintConsoleLine::new("Blink stopped".to_string()));
                            info!("Blink stopped via script");
                        }
                        _ => {
                            print_console_line.write(PrintConsoleLine::new("Usage: blink [start|stop]".to_string()));
                        }
                    }
                }
            }
            "mqtt" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "status" => {
                            let status = if temperature.value.is_some() {
                                "Connected to MQTT broker"
                            } else {
                                "Connecting to MQTT broker..."
                            };
                            print_console_line.write(PrintConsoleLine::new(status.to_string()));
                            info!("MQTT status requested via script");
                        }
                        "temp" => {
                            let temp_msg = if let Some(val) = temperature.value {
                                format!("Current temperature: {:.1}°C", val)
                            } else {
                                "No temperature data available".to_string()
                            };
                            print_console_line.write(PrintConsoleLine::new(temp_msg));
                        }
                        _ => {
                            print_console_line.write(PrintConsoleLine::new("Usage: mqtt [status|temp]".to_string()));
                        }
                    }
                }
            }
            "spawn" => {
                if parts.len() == 5 {
                    if let Ok(x) = parts[2].parse::<f32>() {
                        if let Ok(y) = parts[3].parse::<f32>() {
                            if let Ok(z) = parts[4].parse::<f32>() {
                                let device_id = parts[1].to_string();
                                
                                // Create spawn command payload
                                let payload = json!({
                                    "device_id": device_id,
                                    "device_type": "lamp",
                                    "state": "online",
                                    "location": { "x": x, "y": y, "z": z }
                                }).to_string();

                                // Create a temporary client for simulation
                                let mut mqtt_options = MqttOptions::new("spawn-client", "127.0.0.1", 1883);
                                mqtt_options.set_keep_alive(Duration::from_secs(5));
                                let (client, mut connection) = Client::new(mqtt_options, 10);
                                
                                client
                                    .publish("devices/announce", QoS::AtMostOnce, false, payload.as_bytes())
                                    .unwrap();

                                // Drive the event loop to ensure the message is sent
                                for notification in connection.iter() {
                                    if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                                        break;
                                    }
                                }
                                
                                print_console_line.write(PrintConsoleLine::new(format!("Spawn command sent for device {}", device_id)));
                            }
                        }
                    }
                }
            }
            _ => {
                print_console_line.write(PrintConsoleLine::new(format!("Unknown command: {}", command)));
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    let mut script_executor = ScriptExecutor::default();

    // Set up startup script if provided
    if let Some(script_file) = args.script {
        script_executor.startup_script = Some(script_file);
        script_executor.execute_startup = true;
    }

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
        .add_console_command::<SpawnCommand, _>(handle_spawn_command)
        .add_console_command::<LoadCommand, _>(handle_load_command)
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
        .add_systems(Update, listen_for_device_announcements)
        .add_systems(Update, handle_console_t_key.after(ConsoleSet::Commands))
        .insert_resource(script_executor)
        .insert_resource(PendingCommands { commands: Vec::new() })
        .add_systems(Update, script_execution_system)
        .add_systems(Update, execute_pending_commands)
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
        info!("Console command: blink {}", action);
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
        info!("Console command: mqtt {}", action);
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

fn handle_spawn_command(
    mut log: ConsoleCommand<SpawnCommand>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut devices_tracker: ResMut<DevicesTracker>,
) {
    if let Some(Ok(SpawnCommand { device_id, x, y, z })) = log.take() {
        info!("Console command: spawn {} {} {} {}", device_id, x, y, z);
        
        // Check if device is already spawned
        if devices_tracker.spawned_devices.contains(&device_id) {
            reply!(log, "Device {} already spawned", device_id);
            return;
        }
        
        // Create device entity
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
            Transform::from_translation(Vec3::new(x, y, z)),
            DeviceEntity {
                device_id: device_id.clone(),
                device_type: "lamp".to_string(),
            },
            Visibility::default(),
        ));
        
        // Track the spawned device
        devices_tracker.spawned_devices.insert(device_id.clone());
        
        reply!(log, "Device {} spawned at ({}, {}, {})", device_id, x, y, z);
        info!("Device {} spawned via console", device_id);
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

// System to listen for device announcements via MQTT
fn listen_for_device_announcements(
    receiver: Res<DeviceAnnouncementReceiver>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut devices_tracker: ResMut<DevicesTracker>,
) {
    if let Ok(rx) = receiver.0.lock() {
        while let Ok(announcement) = rx.try_recv() {
            // Parse the JSON announcement
            if let Ok(device_info) = serde_json::from_str::<serde_json::Value>(&announcement) {
                if let (Some(device_id), Some(location)) = (
                    device_info.get("device_id").and_then(|v| v.as_str()),
                    device_info.get("location")
                ) {
                    if let (Some(x), Some(y), Some(z)) = (
                        location.get("x").and_then(|v| v.as_f64()),
                        location.get("y").and_then(|v| v.as_f64()),
                        location.get("z").and_then(|v| v.as_f64())
                    ) {
                        // Check if device is already spawned
                        if !devices_tracker.spawned_devices.contains(device_id) {
                            // Create device entity
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
                                Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                                DeviceEntity {
                                    device_id: device_id.to_string(),
                                    device_type: device_info.get("device_type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                },
                                Visibility::default(),
                            ));
                            
                            // Track the spawned device
                            devices_tracker.spawned_devices.insert(device_id.to_string());
                            
                            info!("Device {} spawned at ({}, {}, {}) via MQTT", device_id, x, y, z);
                        }
                    }
                }
            }
        }
    }
}

