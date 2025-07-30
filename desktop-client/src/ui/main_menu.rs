use crate::world::{CreateWorldEvent, DiscoveredWorlds, LoadWorldEvent, SaveWorldEvent};
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
            .add_systems(OnEnter(GameState::GameplayMenu), setup_gameplay_menu)
            .add_systems(OnExit(GameState::GameplayMenu), despawn_gameplay_menu)
            .add_systems(OnEnter(GameState::InGame), grab_cursor_on_game_start)
            .add_systems(
                Update,
                (
                    main_menu_interaction.run_if(in_state(GameState::MainMenu)),
                    world_selection_interaction.run_if(in_state(GameState::WorldSelection)),
                    gameplay_menu_interaction.run_if(in_state(GameState::GameplayMenu)),
                    handle_escape_key,
                ),
            );
    }
}

/// System to ensure cursor is grabbed when entering the game
fn grab_cursor_on_game_start(mut windows: Query<&mut Window>) {
    for mut window in &mut windows {
        info!("Grabbing cursor on game start - setting to Locked");
        window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
        window.cursor_options.visible = false;
        info!(
            "Cursor grab mode after setting: {:?}",
            window.cursor_options.grab_mode
        );
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

/// Component for the Return to Game button
#[derive(Component)]
pub struct ReturnToGameButton;

/// Component for the Save and Quit button
#[derive(Component)]
pub struct SaveAndQuitButton;

/// Component for the Quit to Menu button
#[derive(Component)]
pub struct QuitToMenuButton;

/// Game state enum
#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    WorldSelection,
    GameplayMenu,
    InGame,
    ConsoleOpen,
}

fn setup_main_menu(mut commands: Commands) {
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
                                Text::new("Enter the world"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
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
                                Text::new("Quit Application"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
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
    mut quit_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<QuitButton>,
            Without<EnterWorldButton>,
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

fn setup_world_selection_menu(mut commands: Commands, discovered_worlds: Res<DiscoveredWorlds>) {
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
                Text::new("Select a World"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

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

                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(400.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                        SelectWorldButton(world_info.name.clone()),
                    ))
                    .with_children(|parent| {
                        // World name on the left
                        parent.spawn((
                            Text::new(world_info.name.clone()),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));

                        // Last played time on the right
                        parent.spawn((
                            Text::new(format!("Last played: {}", last_played)),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                    });
            }

            // Create New World button
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
                    CreateWorldButton,
                ))
                .with_children(|parent| {
                    parent.spawn(Text::new("Create New World"));
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

fn setup_gameplay_menu(mut commands: Commands, mut windows: Query<&mut Window>) {
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
                    parent.spawn(Text::new("Return to Game"));
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
                    parent.spawn(Text::new("Save and Quit to Main Menu"));
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
                    parent.spawn(Text::new("Quit to Main Menu (No Save)"));
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

fn handle_escape_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match **current_state {
            GameState::InGame => game_state.set(GameState::GameplayMenu),
            GameState::GameplayMenu => game_state.set(GameState::InGame),
            _ => (),
        }
    }
}
