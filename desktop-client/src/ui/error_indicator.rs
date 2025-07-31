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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::ResMut;

    #[test]
    fn test_error_indicator_behavior() {
        let mut error_resource = ErrorResource::default();

        // Initially, the indicator should be off
        assert!(!error_resource.indicator_on);
        assert_eq!(error_resource.messages.len(), 0);

        // Trigger an error
        error_resource.indicator_on = true;
        error_resource.messages.push("Test error".into());

        // Check that indicator reflects error
        assert!(error_resource.indicator_on);
        assert_eq!(error_resource.messages.len(), 1);
        assert_eq!(error_resource.messages[0], "Test error");
    }

    #[test]
    fn test_should_show_error() {
        // Error should show when indicator is on and time is within 5 seconds
        assert!(should_show_error(true, 2.0));
        assert!(should_show_error(true, 4.9));

        // Error should not show when time exceeds 5 seconds
        assert!(!should_show_error(true, 5.0));
        assert!(!should_show_error(true, 10.0));

        // Error should not show when indicator is off
        assert!(!should_show_error(false, 2.0));
        assert!(!should_show_error(false, 10.0));
    }

    #[test]
    fn test_get_error_display_text() {
        assert_eq!(get_error_display_text(true), "ERROR");
        assert_eq!(get_error_display_text(false), "");
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

/// Pure function to determine if error should be shown based on time elapsed
pub fn should_show_error(indicator_on: bool, time_since_last_error: f32) -> bool {
    indicator_on && time_since_last_error < 5.0
}

/// Pure function to get error display text
pub fn get_error_display_text(should_show: bool) -> String {
    if should_show {
        "ERROR".to_string()
    } else {
        "".to_string()
    }
}
