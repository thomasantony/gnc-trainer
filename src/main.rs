use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod constants;
mod rhai_api;
mod simulation;
mod ui;
mod visualization;

use rhai_api::ScriptEngine;
use simulation::{reset_simulation, simulation_system, LanderState, SimulationParams};
use ui::{ui_system, EditorState};
use visualization::{particle_system, spawn_visualization, update_visualization};

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
        .insert_resource(ScriptEngine::default())
        .add_systems(Startup, (setup, spawn_visualization))
        .add_systems(
            Update,
            (
                ui_system,
                simulation_system.run_if(|state: Res<EditorState>| state.is_running),
                update_visualization,
                particle_system,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut lander_state: ResMut<LanderState>,
    params: Res<SimulationParams>,
) {
    // Camera setup
    commands.spawn((Camera2d::default(), Transform::from_xyz(0.0, 0.0, 1000.0)));

    // Initialize lander state
    reset_simulation(&mut lander_state, &params);
}
