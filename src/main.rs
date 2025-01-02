use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod constants;
mod levels;
mod particles; // New module
mod persistence;
mod rhai_api;
mod simulation;
mod ui;
mod visualization;

use bevy_persistent::Persistent;
use levels::{CurrentLevel, LevelManager};
use particles::{particle_system, ParticleSpawnTimer};
use persistence::setup_persistence;
use rhai_api::ScriptEngine;
use simulation::{reset_simulation, simulation_system, LanderState};
use ui::{
    level_complete_popup, level_select_ui, ui_system, EditorState, GameState, LevelCompletePopup,
    SimulationState,
};
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
        )))
        .add_systems(Startup, (spawn_visualization, setup, setup_persistence))
        .add_systems(Update, level_completion_check)
        .init_state::<GameState>()
        .insert_resource(State::new(GameState::LevelSelect))
        .insert_resource(LevelCompletePopup::default())
        .add_systems(
            Update,
            (
                level_select_ui.run_if(in_state(GameState::LevelSelect)),
                level_complete_popup,
                (
                    ui_system,
                    simulation_system.run_if(run_simulation),
                    update_visualization,
                    update_grid_lines,
                    particle_system,
                    reset_lander_visibility,
                    visualization::reset_visualization_system,
                )
                    .run_if(in_state(GameState::Playing)),
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
    commands.spawn(Camera2d);
    reset_simulation(&mut lander_state, &level, &mut camera_state);
}

fn setup_game(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::LevelSelect);
}

fn run_simulation(state: Res<EditorState>, lander_state: Res<LanderState>) -> bool {
    state.simulation_state == SimulationState::Running
        && !lander_state.landed
        && !lander_state.crashed
}

fn level_completion_check(
    editor_state: Res<EditorState>,
    lander_state: Res<LanderState>,
    mut progress: ResMut<Persistent<persistence::LevelProgress>>,
    current_level: Res<CurrentLevel>,
    level_manager: Res<LevelManager>,
    mut popup: ResMut<LevelCompletePopup>,
) {
    if lander_state.landed && editor_state.simulation_state == SimulationState::Running {
        if let Some((level_num, _)) = level_manager
            .available_levels
            .iter()
            .find(|(_, name)| name == &current_level.config.name)
        {
            let _ = persistence::mark_level_complete(*level_num, progress);
            popup.show = true;
            popup.completed_level = *level_num;
        }
    }
}
