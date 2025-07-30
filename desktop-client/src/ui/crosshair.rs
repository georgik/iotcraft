use crate::ui::GameState;
use bevy::prelude::*;

/// Plugin for crosshair functionality
pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_crosshair)
            .add_systems(Update, update_crosshair_visibility);
    }
}

/// Component to mark the crosshair UI element
#[derive(Component)]
pub struct Crosshair;

/// System to set up the crosshair UI
fn setup_crosshair(mut commands: Commands) {
    // Create crosshair container
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            Crosshair,
        ))
        .with_children(|parent| {
            // Horizontal line of crosshair
            parent.spawn((
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)), // Semi-transparent white
            ));

            // Vertical line of crosshair
            parent.spawn((
                Node {
                    width: Val::Px(2.0),
                    height: Val::Px(20.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)), // Semi-transparent white
            ));
        });
}

/// System to update crosshair visibility based on game state
fn update_crosshair_visibility(
    mut crosshair_query: Query<&mut Visibility, With<Crosshair>>,
    game_state: Res<State<GameState>>,
) {
    for mut visibility in crosshair_query.iter_mut() {
        *visibility = match game_state.get() {
            GameState::InGame => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}
