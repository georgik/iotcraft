use crate::fonts::Fonts;
use crate::localization::{
    Language, LanguageChangeEvent, LocalizationBundle, LocalizationConfig, LocalizedText,
    get_localized_text,
};
use crate::world::{
    CreateWorldEvent, DeleteWorldEvent, DiscoveredWorlds, LoadWorldEvent, SaveWorldEvent,
};
use bevy::{app::AppExit, prelude::*};

/// Web-compatible stubs for desktop multiplayer types
#[derive(Event, BufferedEvent, Clone, Debug)]
pub struct JoinSharedWorldEvent(pub String);

impl Default for JoinSharedWorldEvent {
    fn default() -> Self {
        Self(String::new())
    }
}

pub struct OnlineWorlds {
    pub discovered_worlds: Vec<String>,
}

#[derive(Event, BufferedEvent, Clone, Debug, Default)]
pub struct PublishWorldEvent;

#[derive(Event, BufferedEvent, Clone, Debug, Default)]
pub struct RefreshOnlineWorldsEvent;

/// Plugin for the main menu (web-compatible version)
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            // Add stub events for web compatibility
            .add_event::<JoinSharedWorldEvent>()
            .add_event::<PublishWorldEvent>()
            .add_event::<RefreshOnlineWorldsEvent>()
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
            .add_systems(
                OnEnter(GameState::WorldSelection),
                setup_world_selection_menu,
            )
            .add_systems(
                OnExit(GameState::WorldSelection),
                despawn_world_selection_menu,
            )
            .add_systems(OnEnter(GameState::Settings), setup_settings_menu)
            .add_systems(OnExit(GameState::Settings), despawn_settings_menu)
            .add_systems(OnEnter(GameState::GameplayMenu), setup_gameplay_menu)
            .add_systems(OnExit(GameState::GameplayMenu), despawn_gameplay_menu)
            .add_systems(OnEnter(GameState::InGame), grab_cursor_on_game_start)
            .add_systems(OnEnter(GameState::ConsoleOpen), release_cursor_for_console)
            .add_systems(OnExit(GameState::ConsoleOpen), grab_cursor_after_console)
            .add_systems(
                Update,
                (
                    main_menu_interaction.run_if(in_state(GameState::MainMenu)),
                    settings_menu_interaction.run_if(in_state(GameState::Settings)),
                    language_button_interaction.run_if(in_state(GameState::Settings)),
                    world_selection_interaction.run_if(in_state(GameState::WorldSelection)),
                    delete_world_interaction.run_if(in_state(GameState::WorldSelection)),
                    gameplay_menu_interaction.run_if(in_state(GameState::GameplayMenu)),
                    handle_escape_key,
                ),
            );
    }
}

/// System to ensure cursor is grabbed when entering the game
fn grab_cursor_on_game_start(
    mut windows: Query<&mut Window>,
    mut cursor_options_query: Query<&mut bevy::window::CursorOptions>,
    mut camera_controller_query: Query<&mut crate::camera_controllers::CameraController>,
) {
    for mut window in &mut windows {
        info!("Grabbing cursor on game start - setting to Locked");

        // Center the cursor before grabbing it to ensure raycasting starts from screen center
        let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(screen_center));
    }

    // Update cursor options using the new component system in Bevy 0.17
    if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
        cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
        cursor_options.visible = false;
        info!(
            "Cursor grab mode set to: {:?}, visible: {}",
            cursor_options.grab_mode, cursor_options.visible
        );
    }

    // Set flag to ignore the next mouse delta to prevent camera jump
    if let Ok(mut controller) = camera_controller_query.single_mut() {
        controller.ignore_next_mouse_delta = true;
        info!("Set ignore_next_mouse_delta flag to prevent camera jump after cursor re-grab");
    }
}

/// System to release cursor when console is opened
fn release_cursor_for_console(mut cursor_options_query: Query<&mut bevy::window::CursorOptions>) {
    if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
        info!("Releasing cursor for console - setting to None");
        cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
        cursor_options.visible = true;
    }
}

/// System to grab cursor when console is closed (returning to game)
fn grab_cursor_after_console(
    mut cursor_options_query: Query<&mut bevy::window::CursorOptions>,
    mut camera_controller_query: Query<&mut crate::camera_controllers::CameraController>,
) {
    if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
        info!("Grabbing cursor after console - setting to Locked");
        cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
        cursor_options.visible = false;
    }

    // Set flag to ignore the next mouse delta to prevent camera jump
    if let Ok(mut controller) = camera_controller_query.single_mut() {
        controller.ignore_next_mouse_delta = true;
        info!("Set ignore_next_mouse_delta flag to prevent camera jump after console close");
    }
}

/// Component to mark the main menu UI
#[derive(Component)]
pub struct MainMenu;

/// Component to mark the world selection UI
#[derive(Component)]
pub struct WorldSelectionMenu;

/// Component to mark the gameplay menu UI
#[derive(Component)]
pub struct GameplayMenu;

/// Component for the Enter World button
#[derive(Component)]
pub struct EnterWorldButton;

/// Component for the Quit button
#[derive(Component)]
pub struct QuitButton;

/// Component for the Create World button
#[derive(Component)]
pub struct CreateWorldButton;

/// Component for a button that selects a world
#[derive(Component)]
pub struct SelectWorldButton(pub String);

/// Component for a button that deletes a world
#[derive(Component)]
pub struct DeleteWorldButton(pub String);

/// Component for the Return to Game button
#[derive(Component)]
pub struct ReturnToGameButton;

/// Component for the Save and Quit button
#[derive(Component)]
pub struct SaveAndQuitButton;

/// Component for the Quit to Menu button
#[derive(Component)]
pub struct QuitToMenuButton;

/// Component for language selector buttons
#[derive(Component)]
pub struct LanguageButton(pub Language);

/// Component for the Settings button
#[derive(Component)]
pub struct SettingsButton;

/// Component to mark the settings menu UI
#[derive(Component)]
pub struct SettingsMenu;

/// Component for the Back to Main Menu button in settings
#[derive(Component)]
pub struct BackToMainMenuButton;

/// Game state enum (same as desktop)
#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Settings,
    WorldSelection,
    GameplayMenu,
    InGame,
    ConsoleOpen,
}

// Re-export the GameState for compatibility

fn setup_main_menu(
    mut commands: Commands,
    _localization_bundle: Option<Res<LocalizationBundle>>,
    _localization_config: Option<Res<LocalizationConfig>>,
    fonts: Option<Res<Fonts>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            MainMenu,
        ))
        .with_children(|parent| {
            // Container for buttons (vertical layout)
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(20.0),
                    ..default()
                })
                .with_children(|parent| {
                    // Enter World button
                    parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(300.0),
                                height: Val::Px(50.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                            EnterWorldButton,
                        ))
                        .with_children(|parent| {
                            let font_handle = fonts
                                .as_ref()
                                .map(|f| f.regular.clone())
                                .unwrap_or_else(|| asset_server.load("fonts/FiraSans-Bold.ttf"));
                            parent.spawn((
                                Text::new("Play Game"), // Simplified for web
                                TextFont {
                                    font: font_handle,
                                    font_size: 20.0,
                                    font_smoothing: bevy::text::FontSmoothing::default(),
                                    line_height: bevy::text::LineHeight::default(),
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Settings button
                    parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(300.0),
                                height: Val::Px(50.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                            SettingsButton,
                        ))
                        .with_children(|parent| {
                            let font_handle = fonts
                                .as_ref()
                                .map(|f| f.regular.clone())
                                .unwrap_or_else(|| asset_server.load("fonts/FiraSans-Bold.ttf"));
                            parent.spawn((
                                Text::new("Settings"), // Simplified for web
                                TextFont {
                                    font: font_handle,
                                    font_size: 20.0,
                                    font_smoothing: bevy::text::FontSmoothing::default(),
                                    line_height: bevy::text::LineHeight::default(),
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Quit button (for web, this might just reload the page)
                    parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(300.0),
                                height: Val::Px(50.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.1, 0.1)),
                            QuitButton,
                        ))
                        .with_children(|parent| {
                            let font_handle = fonts
                                .as_ref()
                                .map(|f| f.regular.clone())
                                .unwrap_or_else(|| asset_server.load("fonts/FiraSans-Bold.ttf"));
                            parent.spawn((
                                Text::new("Quit"),
                                TextFont {
                                    font: font_handle,
                                    font_size: 20.0,
                                    font_smoothing: bevy::text::FontSmoothing::default(),
                                    line_height: bevy::text::LineHeight::default(),
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });
}

// Stub implementations for the rest of the UI systems
fn setup_world_selection_menu(_commands: Commands) {
    info!("World selection menu setup (web version - simplified)");
}

fn setup_settings_menu(_commands: Commands) {
    info!("Settings menu setup (web version - simplified)");
}

fn setup_gameplay_menu(_commands: Commands) {
    info!("Gameplay menu setup (web version - simplified)");
}

fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn despawn_world_selection_menu(
    mut commands: Commands,
    query: Query<Entity, With<WorldSelectionMenu>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn despawn_settings_menu(mut commands: Commands, query: Query<Entity, With<SettingsMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn despawn_gameplay_menu(mut commands: Commands, query: Query<Entity, With<GameplayMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn main_menu_interaction(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&EnterWorldButton>,
            Option<&SettingsButton>,
            Option<&QuitButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut color, enter_world, settings, quit) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                if enter_world.is_some() {
                    next_state.set(GameState::InGame);
                } else if settings.is_some() {
                    next_state.set(GameState::Settings);
                } else if quit.is_some() {
                    // For web, we could reload the page or show an exit message
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().reload();
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    exit.send(AppExit);
                }
            }
            Interaction::Hovered => *color = Color::srgb(0.25, 0.25, 0.25).into(),
            Interaction::None => *color = Color::srgb(0.15, 0.15, 0.15).into(),
        }
    }
}

fn settings_menu_interaction(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&BackToMainMenuButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, back_to_main) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                if back_to_main.is_some() {
                    next_state.set(GameState::MainMenu);
                }
            }
            Interaction::Hovered => *color = Color::srgb(0.25, 0.25, 0.25).into(),
            Interaction::None => *color = Color::srgb(0.15, 0.15, 0.15).into(),
        }
    }
}

fn language_button_interaction() {
    // Stub for web - language switching simplified
}

fn world_selection_interaction() {
    // Stub for web - world selection simplified
}

fn delete_world_interaction() {
    // Stub for web - world deletion simplified
}

fn gameplay_menu_interaction(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&ReturnToGameButton>,
            Option<&QuitToMenuButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, return_to_game, quit_to_menu) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                if return_to_game.is_some() {
                    next_state.set(GameState::InGame);
                } else if quit_to_menu.is_some() {
                    next_state.set(GameState::MainMenu);
                }
            }
            Interaction::Hovered => *color = Color::srgb(0.25, 0.25, 0.25).into(),
            Interaction::None => *color = Color::srgb(0.15, 0.15, 0.15).into(),
        }
    }
}

fn handle_escape_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match current_state.get() {
            GameState::InGame => next_state.set(GameState::GameplayMenu),
            GameState::GameplayMenu => next_state.set(GameState::InGame),
            GameState::Settings => next_state.set(GameState::MainMenu),
            GameState::WorldSelection => next_state.set(GameState::MainMenu),
            _ => {}
        }
    }
}
