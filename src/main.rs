use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod constants;
mod levels;
mod particles; // New module
mod rhai_api;
mod simulation;
mod ui;
mod visualization;

use levels::{CurrentLevel, LevelManager};
use particles::{particle_system, ParticleSpawnTimer}; // New imports
use rhai_api::ScriptEngine;
use simulation::{reset_simulation, simulation_system, LanderState};
use ui::{ui_system, EditorState, SimulationState};
use visualization::{
    reset_lander_visibility, spawn_visualization, update_grid_lines, update_visualization,
    CameraState, ResetVisibilityFlag,
};

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
        .insert_resource(LevelManager::load())
        .insert_resource(CurrentLevel::load(0))
        .insert_resource(ScriptEngine::default())
        .insert_resource(visualization::CameraState::default())
        .insert_resource(ResetVisibilityFlag::default())
        .insert_resource(visualization::ResetVisualization::default())
        .insert_resource(ParticleSpawnTimer(Timer::from_seconds(
            0.05,
            TimerMode::Repeating,
        ))) // Add particle timer
        .add_systems(Startup, spawn_visualization)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                ui_system,
                simulation_system.run_if(run_simulation),
                (
                    update_visualization,
                    update_grid_lines,
                    particle_system,
                    reset_lander_visibility,
                    visualization::reset_visualization_system,
                ),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut lander_state: ResMut<LanderState>,
    level: Res<CurrentLevel>,
    mut camera_state: ResMut<CameraState>,
) {
    commands.spawn(Camera2d::default());
    reset_simulation(&mut lander_state, &level, &mut camera_state);
}

fn run_simulation(state: Res<EditorState>, lander_state: Res<LanderState>) -> bool {
    state.simulation_state == SimulationState::Running
        && !lander_state.landed
        && !lander_state.crashed
}
