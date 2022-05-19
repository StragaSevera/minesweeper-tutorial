use bevy::prelude::*;
#[cfg(feature = "debug")]
use bevy_inspector_egui::WorldInspectorPlugin;

use board_plugin::resources::BoardOptions;
use board_plugin::BoardPlugin;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    InGame,
    Out,
}

fn main() {
    let mut app = App::new();

    app.insert_resource(WindowDescriptor {
        title: "Mine Sweeper!".to_string(),
        width: 700.,
        height: 800.,
        ..Default::default()
    })
    .add_plugins(DefaultPlugins);

    #[cfg(feature = "debug")]
    app.add_plugin(WorldInspectorPlugin::new());

    // Board plugin options
    app.insert_resource(BoardOptions {
        map_size: (20, 20),
        bomb_count: 40,
        tile_padding: 3.0,
        safe_start: true,
        ..Default::default()
    })
    .add_plugin(BoardPlugin { running_state: AppState::InGame })
    .add_startup_system(camera_setup)
    .add_system(state_handler)
    .run();
}

fn camera_setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn state_handler(mut state: ResMut<State<AppState>>, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::C) {
        debug!("clearing detected");
        if state.current() == &AppState::InGame {
            info!("clearing game");
            state.set(AppState::Out).unwrap();
        }
    }
    if keys.just_pressed(KeyCode::G) {
        debug!("loading detected");
        if state.current() == &AppState::Out {
            info!("loading game");
            state.set(AppState::InGame).unwrap();
        }
    }
}
