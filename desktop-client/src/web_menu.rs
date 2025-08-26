use bevy::prelude::*;

/// Simplified game states for web version  
#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum WebGameState {
    #[default]
    MainMenu,
    Settings,
    WorldSelection,
    GameplayMenu,
    InGame,
}

/// Plugin for the web menu system
pub struct WebMenuPlugin;

impl Plugin for WebMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<WebGameState>()
            .add_systems(OnEnter(WebGameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(WebGameState::MainMenu), cleanup_main_menu)
            .add_systems(OnEnter(WebGameState::InGame), setup_in_game)
            .add_systems(OnExit(WebGameState::InGame), cleanup_in_game)
            .add_systems(OnEnter(WebGameState::GameplayMenu), setup_gameplay_menu)
            .add_systems(OnExit(WebGameState::GameplayMenu), cleanup_gameplay_menu)
            .add_systems(OnEnter(WebGameState::Settings), setup_settings_menu)
            .add_systems(OnExit(WebGameState::Settings), cleanup_settings_menu)
            .add_systems(OnEnter(WebGameState::WorldSelection), setup_world_selection)
            .add_systems(
                OnExit(WebGameState::WorldSelection),
                cleanup_world_selection,
            )
            .add_systems(
                Update,
                (
                    handle_main_menu_buttons.run_if(in_state(WebGameState::MainMenu)),
                    handle_gameplay_menu_buttons.run_if(in_state(WebGameState::GameplayMenu)),
                    handle_settings_menu_buttons.run_if(in_state(WebGameState::Settings)),
                    handle_world_selection_buttons.run_if(in_state(WebGameState::WorldSelection)),
                    handle_escape_key,
                ),
            );
    }
}

/// Component markers for different UI elements
#[derive(Component)]
struct MainMenuUI;

#[derive(Component)]
struct GameplayMenuUI;

#[derive(Component)]
struct SettingsMenuUI;

#[derive(Component)]
struct WorldSelectionUI;

/// Button component markers
#[derive(Component)]
struct EnterWorldButton;

#[derive(Component)]
struct SettingsButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct ReturnToGameButton;

#[derive(Component)]
struct BackToMainButton;

#[derive(Component)]
struct CreateWorldButton;

#[derive(Component)]
struct SelectWorldButton(String);

/// Text styling constants
const NORMAL_BUTTON_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON_COLOR: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON_COLOR: Color = Color::srgb(0.35, 0.75, 0.35);
const BUTTON_TEXT_COLOR: Color = Color::WHITE;

fn setup_main_menu(mut commands: Commands, _asset_server: Res<AssetServer>) {
    info!("Setting up main menu UI - press Enter to start");

    // Spawn main menu UI
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        MainMenuUI,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("IoTCraft Web\n\nPress Enter or tap anywhere to start\nPress Escape for menu when in-game"),
            TextFont { font_size: 32.0, ..default() },
            TextColor(BUTTON_TEXT_COLOR),
        ));
    });
}

fn setup_world_selection(_commands: Commands) {
    info!("World selection - press Enter to play");
}

fn setup_gameplay_menu(_commands: Commands) {
    info!("Gameplay menu - press Escape to return");
}

fn setup_settings_menu(_commands: Commands) {
    info!("Settings menu");
}

fn setup_in_game(
    mut windows: Query<&mut Window>,
    mut camera_controller: ResMut<crate::CameraController>,
    mut cursor_options_query: Query<&mut bevy::window::CursorOptions>,
) {
    info!("Entering game - enabling camera control");

    // Enable camera controller
    camera_controller.enabled = true;

    // Try to grab mouse for in-game experience (safe for mobile) using lib_gradual helper
    if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
        for mut window in &mut windows {
            crate::lib_gradual::safe_set_cursor_grab_mode(
                &mut window,
                Some(&mut cursor_options),
                bevy::window::CursorGrabMode::Locked,
                false,
            );
        }
    }
}

fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cleanup_world_selection(mut commands: Commands, query: Query<Entity, With<WorldSelectionUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cleanup_gameplay_menu(mut commands: Commands, query: Query<Entity, With<GameplayMenuUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cleanup_settings_menu(mut commands: Commands, query: Query<Entity, With<SettingsMenuUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cleanup_in_game(
    mut windows: Query<&mut Window>,
    mut cursor_options_query: Query<&mut bevy::window::CursorOptions>,
    _camera_controller: ResMut<crate::CameraController>,
) {
    info!("Exiting game - releasing camera control");

    // Release mouse when leaving game
    if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
        for _window in &mut windows {
            cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }
}

/// Handle main menu button interactions
fn handle_main_menu_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    enter_world_query: Query<&Interaction, (With<EnterWorldButton>, Changed<Interaction>)>,
    settings_query: Query<&Interaction, (With<SettingsButton>, Changed<Interaction>)>,
    quit_query: Query<&Interaction, (With<QuitButton>, Changed<Interaction>)>,
    mut next_state: ResMut<NextState<WebGameState>>,
) {
    // Update button colors
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => *color = PRESSED_BUTTON_COLOR.into(),
            Interaction::Hovered => *color = HOVERED_BUTTON_COLOR.into(),
            Interaction::None => *color = NORMAL_BUTTON_COLOR.into(),
        }
    }

    // Handle button clicks
    for interaction in enter_world_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(WebGameState::WorldSelection);
        }
    }

    for interaction in settings_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(WebGameState::Settings);
        }
    }

    for interaction in quit_query.iter() {
        if *interaction == Interaction::Pressed {
            info!("Quit button pressed - close browser tab to exit");
        }
    }
}

/// Handle world selection button interactions
fn handle_world_selection_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    create_world_query: Query<&Interaction, (With<CreateWorldButton>, Changed<Interaction>)>,
    select_world_query: Query<(&Interaction, &SelectWorldButton), Changed<Interaction>>,
    back_query: Query<&Interaction, (With<BackToMainButton>, Changed<Interaction>)>,
    mut next_state: ResMut<NextState<WebGameState>>,
) {
    // Update button colors
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => *color = PRESSED_BUTTON_COLOR.into(),
            Interaction::Hovered => *color = HOVERED_BUTTON_COLOR.into(),
            Interaction::None => *color = NORMAL_BUTTON_COLOR.into(),
        }
    }

    // Handle button clicks
    for interaction in create_world_query.iter() {
        if *interaction == Interaction::Pressed {
            info!("Creating new world...");
            next_state.set(WebGameState::InGame);
        }
    }

    for (interaction, world_button) in select_world_query.iter() {
        if *interaction == Interaction::Pressed {
            info!("Loading world: {}", world_button.0);
            next_state.set(WebGameState::InGame);
        }
    }

    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(WebGameState::MainMenu);
        }
    }
}

/// Handle gameplay menu button interactions
fn handle_gameplay_menu_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    return_to_game_query: Query<&Interaction, (With<ReturnToGameButton>, Changed<Interaction>)>,
    back_query: Query<&Interaction, (With<BackToMainButton>, Changed<Interaction>)>,
    mut next_state: ResMut<NextState<WebGameState>>,
) {
    // Update button colors
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => *color = PRESSED_BUTTON_COLOR.into(),
            Interaction::Hovered => *color = HOVERED_BUTTON_COLOR.into(),
            Interaction::None => *color = NORMAL_BUTTON_COLOR.into(),
        }
    }

    // Handle button clicks
    for interaction in return_to_game_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(WebGameState::InGame);
        }
    }

    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(WebGameState::MainMenu);
        }
    }
}

/// Handle settings menu button interactions
fn handle_settings_menu_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    back_query: Query<&Interaction, (With<BackToMainButton>, Changed<Interaction>)>,
    mut next_state: ResMut<NextState<WebGameState>>,
) {
    // Update button colors
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => *color = PRESSED_BUTTON_COLOR.into(),
            Interaction::Hovered => *color = HOVERED_BUTTON_COLOR.into(),
            Interaction::None => *color = NORMAL_BUTTON_COLOR.into(),
        }
    }

    // Handle button clicks
    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(WebGameState::MainMenu);
        }
    }
}

/// Handle keyboard, touch, and mouse navigation (iPad compatible) - with crash protection
fn handle_escape_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut touch_events: EventReader<TouchInput>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    current_state: Res<State<WebGameState>>,
    mut next_state: ResMut<NextState<WebGameState>>,
) {
    // Handle keyboard input
    if keyboard_input.just_pressed(KeyCode::Enter) {
        match current_state.get() {
            WebGameState::MainMenu => {
                info!("📱 Starting game via Enter key");
                next_state.set(WebGameState::InGame);
            }
            _ => {}
        }
    }

    if keyboard_input.just_pressed(KeyCode::Escape) {
        match current_state.get() {
            WebGameState::InGame => {
                info!("📱 Opening menu via Escape key");
                next_state.set(WebGameState::MainMenu);
            }
            _ => {}
        }
    }

    // Handle touch input - tap anywhere on main menu to start game
    let mut touch_detected = false;
    for touch in touch_events.read() {
        if touch.phase == bevy::input::touch::TouchPhase::Started {
            touch_detected = true;
            match current_state.get() {
                WebGameState::MainMenu => {
                    info!("📱 Starting game via touch input at {:?}", touch.position);
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(&"📱 Touch detected - starting game!".into());
                    next_state.set(WebGameState::InGame);
                }
                _ => {}
            }
        }
    }

    // iPad fallback: Handle mouse clicks as touch events (iPad Safari sometimes treats touches as mouse events)
    if !touch_detected && mouse_button_input.just_pressed(MouseButton::Left) {
        match current_state.get() {
            WebGameState::MainMenu => {
                info!("📱 Starting game via mouse click (iPad fallback)");
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &"📱 Mouse click detected - starting game (iPad fallback)!".into(),
                );
                next_state.set(WebGameState::InGame);
            }
            _ => {}
        }
    }
}
