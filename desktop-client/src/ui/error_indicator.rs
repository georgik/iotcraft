use bevy::prelude::*;
use bevy_console::PrintConsoleLine;

#[derive(Component)]
pub struct ErrorIndicator;

#[derive(Resource)]
pub struct ErrorResource {
    pub indicator_on: bool,
    pub messages: Vec<String>,
    pub last_error_time: f32,
}

impl Default for ErrorResource {
    fn default() -> Self {
        Self {
            indicator_on: false,
            messages: Vec::new(),
            last_error_time: 0.0,
        }
    }
}

pub struct ErrorIndicatorPlugin;

impl Plugin for ErrorIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ErrorResource::default())
            .add_systems(Startup, setup_error_indicator)
            .add_systems(Update, (update_error_indicator, capture_error_logs));
    }
}

fn setup_error_indicator(mut commands: Commands) {
    // Create error indicator UI in top-right corner
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(100.0),
                height: Val::Px(30.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ZIndex(1000),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.2, 0.2)), // Red color for errors
                ErrorIndicator,
            ));
        });
}

fn update_error_indicator(
    error_resource: Res<ErrorResource>,
    mut query: Query<&mut Text, With<ErrorIndicator>>,
    time: Res<Time>,
) {
    if let Ok(mut text) = query.single_mut() {
        if error_resource.indicator_on {
            let time_since_last_error = time.elapsed_secs() - error_resource.last_error_time;
            if time_since_last_error < 5.0 {
                // Show indicator for 5 seconds
                text.0 = "ERROR".to_string();
            } else {
                text.0 = "".to_string();
            }
        } else {
            text.0 = "".to_string();
        }
    }
}

fn capture_error_logs(
    _error_resource: ResMut<ErrorResource>,
    _time: Res<Time>,
    _print_console_line: EventWriter<PrintConsoleLine>,
) {
    // This is a placeholder - in a real implementation you would hook into the logging system
    // For now, we'll just trigger the error indicator when certain conditions are met

    // You could add specific error detection logic here
    // For example, checking for file not found errors, network errors, etc.
}

// Function to trigger an error (can be called from other systems)
#[allow(dead_code)]
pub fn trigger_error(
    mut error_resource: ResMut<ErrorResource>,
    time: Res<Time>,
    mut print_console_line: EventWriter<PrintConsoleLine>,
    message: String,
) {
    error_resource.indicator_on = true;
    error_resource.last_error_time = time.elapsed_secs();
    error_resource.messages.push(message.clone());

    // Send to console
    print_console_line.write(PrintConsoleLine::new(format!("ERROR: {}", message)));
}
