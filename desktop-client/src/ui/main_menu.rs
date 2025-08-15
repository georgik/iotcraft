use crate::fonts::Fonts;
use crate::localization::{
    Language, LanguageChangeEvent, LocalizationBundle, LocalizationConfig, LocalizedText,
    get_localized_text,
};
use crate::world::{
    CreateWorldEvent, DeleteWorldEvent, DiscoveredWorlds, LoadWorldEvent, SaveWorldEvent,
};
use bevy::{app::AppExit, prelude::*};

/// Plugin for the main menu
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
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
    mut camera_controller_query: Query<&mut crate::camera_controllers::CameraController>,
) {
    for mut window in &mut windows {
        info!("Grabbing cursor on game start - setting to Locked");

        // Center the cursor before grabbing it to ensure raycasting starts from screen center
        let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(screen_center));

        window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
        window.cursor_options.visible = false;
        info!(
            "Cursor grab mode after setting: {:?}, cursor positioned at screen center",
            window.cursor_options.grab_mode
        );
    }

    // Set flag to ignore the next mouse delta to prevent camera jump
    if let Ok(mut controller) = camera_controller_query.single_mut() {
        controller.ignore_next_mouse_delta = true;
        info!("Set ignore_next_mouse_delta flag to prevent camera jump after cursor re-grab");
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

/// Game state enum
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

fn setup_main_menu(
    mut commands: Commands,
    _localization_bundle: Res<LocalizationBundle>,
    _localization_config: Res<LocalizationConfig>,
    fonts: Res<Fonts>,
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
                            parent.spawn((
                                Text::new(""), // Will be filled by localization system
                                LocalizedText::new("menu-enter-world"),
                                TextFont {
                                    font: fonts.regular.clone(),
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
                            parent.spawn((
                                Text::new(""), // Will be filled by localization system
                                LocalizedText::new("menu-settings"),
                                TextFont {
                                    font: fonts.regular.clone(),
                                    font_size: 20.0,
                                    font_smoothing: bevy::text::FontSmoothing::default(),
                                    line_height: bevy::text::LineHeight::default(),
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    // Quit button
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
                            parent.spawn((
                                Text::new(""), // Will be filled by localization system
                                LocalizedText::new("menu-quit-application"),
                                TextFont {
                                    font: fonts.regular.clone(),
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

fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn main_menu_interaction(
    mut enter_world_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<EnterWorldButton>),
    >,
    mut settings_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<SettingsButton>,
            Without<EnterWorldButton>,
        ),
    >,
    mut quit_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<QuitButton>,
            Without<EnterWorldButton>,
            Without<SettingsButton>,
        ),
    >,
    mut game_state: ResMut<NextState<GameState>>,
    _windows: Query<&mut Window>,
    mut exit: EventWriter<AppExit>,
) {
    // Handle Enter World button
    for (interaction, mut color) in enter_world_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                game_state.set(GameState::WorldSelection);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }

    // Handle Settings button
    for (interaction, mut color) in settings_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                game_state.set(GameState::Settings);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }

    // Handle Quit button
    for (interaction, mut color) in quit_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.6, 0.2, 0.2).into();
                exit.write(AppExit::Success);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.3, 0.15, 0.15).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.2, 0.1, 0.1).into();
            }
        }
    }
}

fn language_button_interaction(
    mut language_button_query: Query<
        (&Interaction, &mut BackgroundColor, &LanguageButton),
        (Changed<Interaction>, With<LanguageButton>),
    >,
    mut localization_config: ResMut<LocalizationConfig>,
    mut language_change_events: EventWriter<LanguageChangeEvent>,
) {
    for (interaction, mut color, language_button) in language_button_query.iter_mut() {
        let is_current = language_button.0 == localization_config.current_language;

        match *interaction {
            Interaction::Pressed => {
                if !is_current {
                    info!("Language changed to: {:?}", language_button.0);
                    localization_config.current_language = language_button.0;
                    language_change_events.write(LanguageChangeEvent {
                        new_language: language_button.0,
                    });
                }
                *color = Color::srgb(0.2, 0.4, 0.6).into(); // Pressed/selected color
            }
            Interaction::Hovered => {
                if is_current {
                    *color = Color::srgb(0.3, 0.5, 0.7).into(); // Lighter highlight for current
                } else {
                    *color = Color::srgb(0.2, 0.2, 0.2).into(); // Hover color for others
                }
            }
            Interaction::None => {
                if is_current {
                    *color = Color::srgb(0.2, 0.4, 0.6).into(); // Current language color
                } else {
                    *color = Color::srgb(0.1, 0.1, 0.1).into(); // Normal color
                }
            }
        }
    }
}

/// Get display code for a language (e.g., "en-US", "de-DE", "fr-FR")
fn get_language_display_code(language: Language) -> String {
    match language {
        Language::EnglishUS => "en-US".to_string(),
        Language::SpanishES => "es-ES".to_string(),
        Language::GermanDE => "de-DE".to_string(),
        Language::CzechCZ => "cs-CZ".to_string(),
        Language::SlovakSK => "sk-SK".to_string(),
        Language::PolishPL => "pl-PL".to_string(),
        Language::HungarianHU => "hu-HU".to_string(),
        Language::FrenchFR => "fr-FR".to_string(),
        Language::ItalianIT => "it-IT".to_string(),
        Language::PortugueseBR => "pt-BR".to_string(),
        Language::SlovenianSI => "sl-SI".to_string(),
        Language::CroatianHR => "hr-HR".to_string(),
        Language::RomanianRO => "ro-RO".to_string(),
        Language::BulgarianBG => "bg-BG".to_string(),
    }
}

fn setup_world_selection_menu(
    mut commands: Commands,
    discovered_worlds: Res<DiscoveredWorlds>,
    localization_bundle: Res<LocalizationBundle>,
    localization_config: Res<LocalizationConfig>,
    fonts: Res<Fonts>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            WorldSelectionMenu,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new(get_localized_text(
                    &localization_bundle,
                    &localization_config,
                    "menu-select-world",
                    &[],
                )),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 40.0,
                    font_smoothing: bevy::text::FontSmoothing::default(),
                    line_height: bevy::text::LineHeight::default(),
                },
                TextColor(Color::WHITE),
            ));

            // Create New World button (moved to top)
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::bottom(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.5, 0.2)),
                    CreateWorldButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(get_localized_text(
                            &localization_bundle,
                            &localization_config,
                            "menu-create-new-world",
                            &[],
                        )),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 18.0,
                            font_smoothing: bevy::text::FontSmoothing::default(),
                            line_height: bevy::text::LineHeight::default(),
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Scrollable worlds list container
            parent
                .spawn(Node {
                    width: Val::Px(500.0),
                    height: Val::Px(400.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    border: UiRect::all(Val::Px(2.0)),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                })
                .insert(BorderColor(Color::srgb(0.3, 0.3, 0.3)))
                .insert(BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)))
                .with_children(|parent| {
                    // Worlds list
                    for world_info in &discovered_worlds.worlds {
                        // Format the last played time nicely
                        let last_played = if let Ok(datetime) =
                            chrono::DateTime::parse_from_rfc3339(&world_info.metadata.last_played)
                        {
                            datetime.format("%Y-%m-%d %H:%M").to_string()
                        } else {
                            "Unknown".to_string()
                        };

                        // World entry container
                        parent
                            .spawn(Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(60.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                margin: UiRect::bottom(Val::Px(5.0)),
                                ..default()
                            })
                            .with_children(|parent| {
                                // Main world selection button (takes most space)
                                parent
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(85.0),
                                            height: Val::Percent(100.0),
                                            justify_content: JustifyContent::SpaceBetween,
                                            align_items: AlignItems::Center,
                                            padding: UiRect::all(Val::Px(10.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                                        SelectWorldButton(world_info.name.clone()),
                                    ))
                                    .with_children(|parent| {
                                        // Left side - world name
                                        parent.spawn((
                                            Text::new(world_info.name.clone()),
                                            TextFont {
                                                font: fonts.regular.clone(),
                                                font_size: 18.0,
                                                font_smoothing: bevy::text::FontSmoothing::default(
                                                ),
                                                line_height: bevy::text::LineHeight::default(),
                                            },
                                            TextColor(Color::WHITE),
                                        ));

                                        // Right side - last played time
                                        parent.spawn((
                                            Text::new(get_localized_text(
                                                &localization_bundle,
                                                &localization_config,
                                                "world-last-played",
                                                &[("time".to_string(), last_played)],
                                            )),
                                            TextFont {
                                                font: fonts.regular.clone(),
                                                font_size: 12.0,
                                                font_smoothing: bevy::text::FontSmoothing::default(
                                                ),
                                                line_height: bevy::text::LineHeight::default(),
                                            },
                                            TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                        ));
                                    });

                                // Delete button (small)
                                parent
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(30.0),
                                            height: Val::Px(30.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            margin: UiRect::left(Val::Px(5.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.6, 0.2, 0.2)),
                                        DeleteWorldButton(world_info.name.clone()),
                                    ))
                                    .with_children(|parent| {
                                        parent.spawn((
                                            Text::new("×"),
                                            TextFont {
                                                font: fonts.regular.clone(),
                                                font_size: 20.0,
                                                font_smoothing: bevy::text::FontSmoothing::default(
                                                ),
                                                line_height: bevy::text::LineHeight::default(),
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });
                            });
                    }
                });
        });
}

fn despawn_world_selection_menu(
    mut commands: Commands,
    query: Query<Entity, With<WorldSelectionMenu>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn world_selection_interaction(
    mut select_world_query: Query<
        (&Interaction, &mut BackgroundColor, &SelectWorldButton),
        (Changed<Interaction>, With<SelectWorldButton>),
    >,
    mut create_world_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<CreateWorldButton>,
            Without<SelectWorldButton>,
        ),
    >,
    mut game_state: ResMut<NextState<GameState>>,
    mut load_world_events: EventWriter<LoadWorldEvent>,
    mut create_world_events: EventWriter<CreateWorldEvent>,
) {
    for (interaction, mut color, select_button) in select_world_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                load_world_events.write(LoadWorldEvent {
                    world_name: select_button.0.clone(),
                });
                game_state.set(GameState::InGame);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }

    for (interaction, mut color) in create_world_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                // For simplicity, we'll create a new world with a default name
                let new_world_name = format!("NewWorld-{}", chrono::Utc::now().timestamp());
                create_world_events.write(CreateWorldEvent {
                    world_name: new_world_name.clone(),
                    description: "A new world".to_string(),
                });
                // No need to send LoadWorldEvent - create_empty_world already sets up CurrentWorld
                game_state.set(GameState::InGame);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }
}

fn delete_world_interaction(
    mut delete_world_query: Query<
        (&Interaction, &mut BackgroundColor, &DeleteWorldButton),
        (Changed<Interaction>, With<DeleteWorldButton>),
    >,
    mut delete_world_events: EventWriter<DeleteWorldEvent>,
) {
    for (interaction, mut color, delete_button) in delete_world_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.8, 0.3, 0.3).into();
                // For now, delete immediately. In a full implementation, you'd want a confirmation dialog.
                info!("Deleting world: {}", delete_button.0);
                delete_world_events.write(DeleteWorldEvent {
                    world_name: delete_button.0.clone(),
                });
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.8, 0.4, 0.4).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.6, 0.2, 0.2).into();
            }
        }
    }
}

fn setup_gameplay_menu(
    mut commands: Commands,
    mut windows: Query<&mut Window>,
    localization_bundle: Res<LocalizationBundle>,
    localization_config: Res<LocalizationConfig>,
) {
    // Release cursor when entering gameplay menu to allow UI interaction
    for mut window in &mut windows {
        info!("Releasing cursor for gameplay menu - setting to None");
        window.cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            GameplayMenu,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(350.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    ReturnToGameButton,
                ))
                .with_children(|parent| {
                    parent.spawn(Text::new(get_localized_text(
                        &localization_bundle,
                        &localization_config,
                        "menu-return-to-game",
                        &[],
                    )));
                });

            // Save and Quit to Main Menu
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(350.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    SaveAndQuitButton,
                ))
                .with_children(|parent| {
                    parent.spawn(Text::new(get_localized_text(
                        &localization_bundle,
                        &localization_config,
                        "menu-save-and-quit",
                        &[],
                    )));
                });

            // Quit to Main Menu (without saving)
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(350.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    QuitToMenuButton,
                ))
                .with_children(|parent| {
                    parent.spawn(Text::new(get_localized_text(
                        &localization_bundle,
                        &localization_config,
                        "menu-quit-no-save",
                        &[],
                    )));
                });
        });
}

fn despawn_gameplay_menu(mut commands: Commands, query: Query<Entity, With<GameplayMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn gameplay_menu_interaction(
    mut return_to_game_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ReturnToGameButton>),
    >,
    mut save_and_quit_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<SaveAndQuitButton>,
            Without<ReturnToGameButton>,
        ),
    >,
    mut quit_to_menu_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<QuitToMenuButton>,
            Without<SaveAndQuitButton>,
            Without<ReturnToGameButton>,
        ),
    >,
    mut game_state: ResMut<NextState<GameState>>,
    mut save_world_events: EventWriter<SaveWorldEvent>,
    current_world: Option<Res<crate::world::CurrentWorld>>,
) {
    for (interaction, mut color) in return_to_game_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                game_state.set(GameState::InGame);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }

    for (interaction, mut color) in save_and_quit_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                if let Some(current_world) = current_world.as_ref() {
                    info!("Queueing save for world: {}", current_world.name);
                    save_world_events.write(SaveWorldEvent {
                        world_name: current_world.name.clone(),
                    });
                }
                game_state.set(GameState::MainMenu);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }

    for (interaction, mut color) in quit_to_menu_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.75, 0.35, 0.35).into();
                info!("Quitting to main menu without saving");
                game_state.set(GameState::MainMenu);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }
}

fn setup_settings_menu(
    mut commands: Commands,
    localization_bundle: Res<LocalizationBundle>,
    localization_config: Res<LocalizationConfig>,
    fonts: Res<Fonts>,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            SettingsMenu,
        ))
        .with_children(|parent| {
            // Settings title
            parent.spawn((
                Text::new(get_localized_text(
                    &localization_bundle,
                    &localization_config,
                    "menu-settings",
                    &[],
                )),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 40.0,
                    font_smoothing: bevy::text::FontSmoothing::default(),
                    line_height: bevy::text::LineHeight::default(),
                },
                TextColor(Color::WHITE),
            ));

            // Language label
            parent.spawn((
                Text::new(get_localized_text(
                    &localization_bundle,
                    &localization_config,
                    "menu-language",
                    &[],
                )),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 20.0,
                    font_smoothing: bevy::text::FontSmoothing::default(),
                    line_height: bevy::text::LineHeight::default(),
                },
                TextColor(Color::WHITE),
            ));

            // Language buttons container
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    row_gap: Val::Px(10.0),
                    flex_wrap: FlexWrap::Wrap,
                    max_width: Val::Px(600.0),
                    ..default()
                })
                .with_children(|parent| {
                    // Create language buttons for all supported languages
                    let languages = [
                        Language::EnglishUS,
                        Language::SpanishES,
                        Language::GermanDE,
                        Language::CzechCZ,
                        Language::SlovakSK,
                        Language::PolishPL,
                        Language::HungarianHU,
                        Language::FrenchFR,
                        Language::ItalianIT,
                        Language::PortugueseBR,
                        Language::SlovenianSI,
                        Language::CroatianHR,
                        Language::RomanianRO,
                        Language::BulgarianBG,
                    ];

                    for language in languages {
                        let is_current = language == localization_config.current_language;
                        parent
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(80.0),
                                    height: Val::Px(40.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(if is_current {
                                    Color::srgb(0.2, 0.4, 0.6) // Current language color
                                } else {
                                    Color::srgb(0.1, 0.1, 0.1) // Normal color
                                }),
                                LanguageButton(language),
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    Text::new(get_language_display_code(language)),
                                    TextFont {
                                        font: fonts.regular.clone(),
                                        font_size: 14.0,
                                        font_smoothing: bevy::text::FontSmoothing::default(),
                                        line_height: bevy::text::LineHeight::default(),
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                    }
                });

            // Back to Main Menu button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(40.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                    BackToMainMenuButton,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(get_localized_text(
                            &localization_bundle,
                            &localization_config,
                            "menu-back-to-main",
                            &[],
                        )),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: 20.0,
                            font_smoothing: bevy::text::FontSmoothing::default(),
                            line_height: bevy::text::LineHeight::default(),
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn despawn_settings_menu(mut commands: Commands, query: Query<Entity, With<SettingsMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn settings_menu_interaction(
    mut back_to_main_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<BackToMainMenuButton>),
    >,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color) in back_to_main_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                game_state.set(GameState::MainMenu);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.25, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.15, 0.15, 0.15).into();
            }
        }
    }
}

fn handle_escape_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match **current_state {
            GameState::InGame => game_state.set(GameState::GameplayMenu),
            GameState::GameplayMenu => game_state.set(GameState::InGame),
            GameState::Settings => game_state.set(GameState::MainMenu),
            _ => (),
        }
    }
}

/// Pure function to format timestamp for world selection display
#[allow(dead_code)]
pub fn format_last_played_time(rfc3339_timestamp: &str) -> String {
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(rfc3339_timestamp) {
        datetime.format("%Y-%m-%d %H:%M").to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Pure function to generate new world name with timestamp
#[allow(dead_code)]
pub fn generate_new_world_name() -> String {
    format!("NewWorld-{}", chrono::Utc::now().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_last_played_time() {
        // Valid RFC3339 timestamp
        let valid_timestamp = "2023-12-25T15:30:00Z";
        let formatted = format_last_played_time(valid_timestamp);
        assert_eq!(formatted, "2023-12-25 15:30");

        // Invalid timestamp should return "Unknown"
        let invalid_timestamp = "invalid-date";
        let formatted = format_last_played_time(invalid_timestamp);
        assert_eq!(formatted, "Unknown");

        // Empty string should return "Unknown"
        let empty_timestamp = "";
        let formatted = format_last_played_time(empty_timestamp);
        assert_eq!(formatted, "Unknown");
    }

    #[test]
    fn test_generate_new_world_name() {
        let name1 = generate_new_world_name();

        // Name should start with "NewWorld-"
        assert!(name1.starts_with("NewWorld-"));

        // Name should contain a timestamp (numeric after "NewWorld-")
        let timestamp_part = &name1["NewWorld-".len()..];
        assert!(!timestamp_part.is_empty());
        assert!(timestamp_part.chars().all(|c| c.is_ascii_digit()));

        // Should generate a valid world name format
        assert!(name1.len() > "NewWorld-".len());
    }
}
