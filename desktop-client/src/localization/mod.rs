pub mod localization_bundle;
pub mod localization_resource;
pub mod localization_systems;
pub mod localization_types;

pub use localization_bundle::*;
pub use localization_resource::*;
pub use localization_systems::*;
pub use localization_types::*;

use bevy::prelude::*;

/// Plugin for localization support in the application
pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalizationConfig>()
            .init_resource::<LocalizationBundle>()
            .add_event::<LanguageChangeEvent>()
            .add_systems(
                Startup,
                setup_localization.in_set(LocalizationSystem::Setup),
            )
            .add_systems(
                Update,
                (
                    handle_language_change,
                    update_localized_text,
                    initialize_localized_text,
                ),
            );
    }
}
/// System set for localization setup
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum LocalizationSystem {
    Setup,
}
