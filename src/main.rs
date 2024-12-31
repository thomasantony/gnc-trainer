use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod constants;
mod levels;
mod rhai_api;
mod simulation;
mod ui;
mod visualization;

use levels::{CurrentLevel, LevelManager};
use rhai_api::ScriptEngine;
use simulation::{reset_simulation, simulation_system, LanderState};
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
        .insert_resource(LevelManager::load()) // Load all available levels
        .insert_resource(CurrentLevel::load(1)) // Start with level 1
        .insert_resource(ScriptEngine::default())
        .add_systems(Startup, spawn_visualization)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                ui_system,
                simulation_system.run_if(run_simulation),
                update_visualization,
                particle_system,
            ),
        )
        .run();
}

fn setup(mut commands: Commands, mut lander_state: ResMut<LanderState>, level: Res<CurrentLevel>) {
    commands.spawn(Camera2d::default());
    reset_simulation(&mut lander_state, &level);
}

fn run_simulation(state: Res<EditorState>, lander_state: Res<LanderState>) -> bool {
    state.is_running && !lander_state.landed && !lander_state.crashed
}
