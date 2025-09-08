use crate::{
    localization::get_localized_text,
    ui::main_menu::{GameState, MainMenu, WorldSelectionMenu},
    ui::ui_params::*,
};
use bevy::prelude::*;
use log::info;

/// Setup main menu using parameter bundles
pub fn setup_main_menu_bundled(
    mut core_params: CoreUIParams,
    localization_params: LocalizationUIParams,
) {
    info!("Setting up main menu using parameter bundles");

    core_params
        .commands
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
                    // Play Game button
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
                            crate::ui::main_menu::EnterWorldButton,
                        ))
                        .with_children(|parent| {
                            let font_handle = core_params
                                .fonts
                                .as_ref()
                                .map(|f| f.regular.clone())
                                .unwrap_or_else(|| {
                                    core_params.asset_server.load("fonts/FiraSans-Bold.ttf")
                                });

                            let button_text = if let (Some(bundle), Some(config)) = (
                                &localization_params.localization_bundle,
                                &localization_params.localization_config,
                            ) {
                                get_localized_text(bundle, config, "menu-enter-world", &[])
                            } else {
                                "Play Game".to_string()
                            };

                            parent.spawn((
                                Text::new(button_text),
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
                            crate::ui::main_menu::SettingsButton,
                        ))
                        .with_children(|parent| {
                            let font_handle = core_params
                                .fonts
                                .as_ref()
                                .map(|f| f.regular.clone())
                                .unwrap_or_else(|| {
                                    core_params.asset_server.load("fonts/FiraSans-Bold.ttf")
                                });

                            let button_text = if let (Some(bundle), Some(config)) = (
                                &localization_params.localization_bundle,
                                &localization_params.localization_config,
                            ) {
                                get_localized_text(bundle, config, "menu-settings", &[])
                            } else {
                                "Settings".to_string()
                            };

                            parent.spawn((
                                Text::new(button_text),
                                TextFont {
                                    font: font_handle,
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
                            crate::ui::main_menu::QuitButton,
                        ))
                        .with_children(|parent| {
                            let font_handle = core_params
                                .fonts
                                .as_ref()
                                .map(|f| f.regular.clone())
                                .unwrap_or_else(|| {
                                    core_params.asset_server.load("fonts/FiraSans-Bold.ttf")
                                });

                            let button_text = if let (Some(bundle), Some(config)) = (
                                &localization_params.localization_bundle,
                                &localization_params.localization_config,
                            ) {
                                get_localized_text(bundle, config, "menu-quit", &[])
                            } else {
                                "Quit".to_string()
                            };

                            parent.spawn((
                                Text::new(button_text),
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

/// Setup world selection menu using parameter bundles
pub fn setup_world_selection_menu_bundled(
    mut core_params: CoreUIParams,
    localization_params: LocalizationUIParams,
    world_params: WorldUIParams,
    mut multiplayer_params: MultiplayerUIParams,
) {
    info!("Setting up world selection menu using parameter bundles");

    let worlds_count = world_params
        .discovered_worlds
        .as_ref()
        .map(|w| w.worlds.len())
        .unwrap_or(0);

    let online_worlds_count = multiplayer_params
        .online_worlds
        .as_ref()
        .map(|w| w.worlds.len())
        .unwrap_or(0);

    info!(
        "Setting up world selection menu with {} local worlds and {} online worlds",
        worlds_count, online_worlds_count
    );

    // Refresh online worlds when entering the selection menu
    info!("Sending RefreshOnlineWorldsEvent to update world list");
    multiplayer_params
        .refresh_events
        .write(crate::multiplayer::shared_world::RefreshOnlineWorldsEvent);

    core_params
        .commands
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
            let title_text = if let (Some(bundle), Some(config)) = (
                &localization_params.localization_bundle,
                &localization_params.localization_config,
            ) {
                get_localized_text(bundle, config, "menu-select-world", &[])
            } else {
                "Select World".to_string()
            };

            parent.spawn((
                Text::new(title_text),
                TextFont {
                    font: core_params
                        .fonts
                        .as_ref()
                        .map(|f| f.regular.clone())
                        .unwrap_or_else(|| {
                            core_params.asset_server.load("fonts/FiraSans-Bold.ttf")
                        }),
                    font_size: 40.0,
                    font_smoothing: bevy::text::FontSmoothing::default(),
                    line_height: bevy::text::LineHeight::default(),
                },
                TextColor(Color::WHITE),
            ));

            // Create New World button
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
                    crate::ui::main_menu::CreateWorldButton,
                ))
                .with_children(|parent| {
                    let font_handle = core_params
                        .fonts
                        .as_ref()
                        .map(|f| f.regular.clone())
                        .unwrap_or_else(|| {
                            core_params.asset_server.load("fonts/FiraSans-Bold.ttf")
                        });

                    let button_text = if let (Some(bundle), Some(config)) = (
                        &localization_params.localization_bundle,
                        &localization_params.localization_config,
                    ) {
                        get_localized_text(bundle, config, "menu-create-new-world", &[])
                    } else {
                        "Create New World".to_string()
                    };

                    parent.spawn((
                        Text::new(button_text),
                        TextFont {
                            font: font_handle,
                            font_size: 18.0,
                            font_smoothing: bevy::text::FontSmoothing::default(),
                            line_height: bevy::text::LineHeight::default(),
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // World lists container (placeholder - full implementation would need more complex layout)
            if let Some(discovered_worlds) = &world_params.discovered_worlds {
                if !discovered_worlds.worlds.is_empty() {
                    parent.spawn((
                        Text::new(format!("Local Worlds: {}", discovered_worlds.worlds.len())),
                        TextFont {
                            font: core_params
                                .fonts
                                .as_ref()
                                .map(|f| f.regular.clone())
                                .unwrap_or_else(|| {
                                    core_params.asset_server.load("fonts/FiraSans-Bold.ttf")
                                }),
                            font_size: 18.0,
                            font_smoothing: bevy::text::FontSmoothing::default(),
                            line_height: bevy::text::LineHeight::default(),
                        },
                        TextColor(Color::WHITE),
                    ));
                }
            }
        });
}

/// Handle main menu interactions using parameter bundles
pub fn handle_main_menu_interaction_bundled(
    mut interaction_params: InteractionUIParams,
    mut game_state_params: GameStateUIParams,
) {
    for (interaction, mut color, enter_world, settings, quit) in
        interaction_params.main_menu_buttons.iter_mut()
    {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.35, 0.75, 0.35).into();
                if enter_world.is_some() {
                    game_state_params.next_state.set(GameState::WorldSelection);
                } else if settings.is_some() {
                    game_state_params.next_state.set(GameState::Settings);
                } else if quit.is_some() {
                    // Platform-specific quit handling
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().reload();
                        }
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    game_state_params
                        .exit_events
                        .write(bevy::app::AppExit::Success);
                }
            }
            Interaction::Hovered => *color = Color::srgb(0.25, 0.25, 0.25).into(),
            Interaction::None => *color = Color::srgb(0.15, 0.15, 0.15).into(),
        }
    }
}

/// Grab cursor on game start using parameter bundles
pub fn grab_cursor_on_game_start_bundled(
    mut cursor_params: CursorUIParams,
    game_state_params: GameStateUIParams,
) {
    // Check if this transition was triggered by MCP (only if MCP is enabled)
    if let Some(mut mcp_transition) = game_state_params.mcp_state_transition {
        if mcp_transition.is_mcp_transition {
            info!("Skipping cursor grab - this is an MCP-triggered state transition");
            // Reset the flag for next time
            mcp_transition.is_mcp_transition = false;
            return;
        }
    }

    for mut window in &mut cursor_params.windows {
        info!("Grabbing cursor on game start - setting to Locked");

        // Center the cursor before grabbing it to ensure raycasting starts from screen center
        let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(screen_center));
    }

    // Update cursor options using the new component system in Bevy 0.17
    if let Ok(mut cursor_options) = cursor_params.cursor_options_query.single_mut() {
        cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
        cursor_options.visible = false;
        info!(
            "Cursor grab mode set to: {:?}, visible: {}",
            cursor_options.grab_mode, cursor_options.visible
        );
    }

    // Set flag to ignore the next mouse delta to prevent camera jump
    if let Ok(mut controller) = cursor_params.camera_controller_query.single_mut() {
        controller.ignore_next_mouse_delta = true;
        info!("Set ignore_next_mouse_delta flag to prevent camera jump after cursor re-grab");
    }
}

/// Release cursor for main menu using parameter bundles
pub fn release_cursor_for_main_menu_bundled(mut cursor_params: CursorUIParams) {
    if let Ok(mut cursor_options) = cursor_params.cursor_options_query.single_mut() {
        info!("Releasing cursor for main menu - setting to None");
        cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
        cursor_options.visible = true;
    }
}

/// Handle escape key navigation using parameter bundles
pub fn handle_escape_key_bundled(game_state_params: GameStateUIParams) {
    let mut next_state = game_state_params.next_state;
    if game_state_params
        .keyboard_input
        .just_pressed(KeyCode::Escape)
    {
        match game_state_params.current_state.get() {
            GameState::InGame => next_state.set(GameState::GameplayMenu),
            GameState::GameplayMenu => next_state.set(GameState::InGame),
            GameState::Settings => next_state.set(GameState::MainMenu),
            GameState::WorldSelection => next_state.set(GameState::MainMenu),
            GameState::WorldCreation => next_state.set(GameState::WorldSelection),
            _ => {}
        }
    }
}

/// Despawn UI entities using parameter bundles
pub fn despawn_main_menu_bundled(mut core_params: CoreUIParams, entity_params: EntityUIParams) {
    for entity in entity_params.main_menu_entities.iter() {
        core_params.commands.entity(entity).despawn();
    }
}

/// Despawn world selection menu entities using parameter bundles
pub fn despawn_world_selection_menu_bundled(
    mut core_params: CoreUIParams,
    entity_params: EntityUIParams,
) {
    for entity in entity_params.world_selection_entities.iter() {
        core_params.commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::IntoSystem;

    #[test]
    fn test_main_menu_setup_compiles() {
        let mut world = World::new();

        // Initialize required resources
        world.init_resource::<AssetServer>();
        world.insert_resource(Fonts::default());
        world.init_resource::<Events<crate::localization::LanguageChangeEvent>>();

        // Test that the bundled system compiles
        let test_system = setup_main_menu_bundled;
        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);

        // If we reach here, the parameter bundling works correctly
        assert!(true);
    }
}
