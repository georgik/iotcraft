use bevy::{app::AppExit, prelude::*};

/// Plugin for the main menu
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
            .add_systems(OnEnter(GameState::InGame), grab_cursor_on_game_start)
            .add_systems(
                Update,
                main_menu_interaction.run_if(in_state(GameState::MainMenu)),
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

/// Component for the Enter World button
#[derive(Component)]
pub struct EnterWorldButton;

/// Component for the Quit button
#[derive(Component)]
pub struct QuitButton;

/// Game state enum
#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
pub enum GameState {
    #[default]
    MainMenu,
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
                                width: Val::Px(200.0),
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
                                width: Val::Px(200.0),
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
                                Text::new("Quit"),
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
    mut windows: Query<&mut Window>,
    mut exit: EventWriter<AppExit>,
) {
    // Handle Enter World button
    for (interaction, mut color) in enter_world_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                game_state.set(GameState::InGame);

                // Grab cursor immediately
                for mut window in &mut windows {
                    window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
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
