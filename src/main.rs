use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod rhai_api;
mod simulation;
mod ui;

use simulation::{reset_simulation, simulation_system, LanderState, SimulationParams};
use ui::{ui_system, EditorState};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Lander Simulator".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(EditorState::default())
        .insert_resource(LanderState::default())
        .insert_resource(SimulationParams::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                ui_system,
                simulation_system.run_if(|state: Res<EditorState>| state.is_running),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut lander_state: ResMut<LanderState>,
    params: Res<SimulationParams>,
) {
    // Camera
    commands.spawn(Camera2d::default());

    // Initialize lander state
    reset_simulation(&mut lander_state, &params);
}
