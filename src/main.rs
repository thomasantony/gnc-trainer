use bevy::{asset::AssetMetaCheck, log::LogPlugin, prelude::*};
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
use levels::{CurrentLevel, GameLoadState, LevelManager, LevelPlugin};
use particles::{particle_system, ParticleSpawnTimer};
use persistence::{setup_persistence, LevelProgress};
use rhai_api::ScriptEngine;
use simulation::{reset_simulation, simulation_system, LanderState};
use ui::{
    about_popup, handle_escape, level_complete_popup, level_select_ui, ui_system, AboutPopupState,
    EditorState, GameState, LevelCompletePopup, SimulationState,
};
use visualization::{
    reset_lander_visibility, spawn_visualization, update_grid_lines, update_visualization,
    CameraState, MainCamera, ResetVisibilityFlag,
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "GNC Trainer".into(),
                        resolution: (1280., 720.).into(),
                        // You may want this set to `true` if you need virtual keyboard work in mobile browsers.
                        prevent_default_event_handling: false,
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    level: bevy::log::Level::DEBUG,
                    filter: "info,wgpu_core=warn,wgpu_hal=warn,bevy_persistent::persistent=warn"
                        .into(),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin)
        .add_plugins(LevelPlugin)
        .insert_resource(EditorState::default())
        .insert_resource(LanderState::default())
        .insert_resource(ScriptEngine::default())
        .insert_resource(visualization::CameraState::default())
        .insert_resource(ResetVisibilityFlag::default())
        .insert_resource(visualization::ResetVisualization::default())
        .insert_resource(ParticleSpawnTimer(Timer::from_seconds(
            0.05,
            TimerMode::Repeating,
        )))
        .insert_resource(AboutPopupState::default())
        .init_state::<GameState>()
        .insert_resource(State::new(GameState::LevelSelect))
        .insert_resource(LevelCompletePopup::default())
        .add_systems(
            OnEnter(GameLoadState::Ready),
            (setup, setup_persistence, spawn_visualization),
        )
        .add_systems(
            Update,
            (
                level_select_ui.run_if(in_state(GameState::LevelSelect)),
                level_complete_popup,
                about_popup,
                (
                    ui_system,
                    simulation_system.run_if(run_simulation),
                    update_visualization,
                    update_grid_lines,
                    particle_system,
                    reset_lander_visibility,
                    visualization::reset_visualization_system,
                    (level_completion_check, save_current_editor_state).chain(),
                    handle_escape,
                )
                    .run_if(in_state(GameState::Playing)),
            )
                .run_if(in_state(GameLoadState::Ready)),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut lander_state: ResMut<LanderState>,
    current_level: Res<CurrentLevel>,
    mut camera_state: ResMut<CameraState>,
) {
    commands.spawn((Camera2d, MainCamera));
    reset_simulation(&mut lander_state, &current_level, &mut camera_state);
}

fn run_simulation(state: Res<EditorState>, lander_state: Res<LanderState>) -> bool {
    state.simulation_state == SimulationState::Running
        && !lander_state.landed
        && !lander_state.crashed
}

pub fn save_current_editor_state(
    editor_state: Res<EditorState>,
    current_level: Res<CurrentLevel>,
    level_manager: Res<LevelManager>,
    progress: ResMut<Persistent<LevelProgress>>,
) {
    if let Some((level_num, _)) = level_manager
        .available_levels
        .iter()
        .find(|(_, name)| name == &current_level.config.name)
    {
        let _ = persistence::save_editor_state(*level_num, editor_state.code.clone(), progress);
    }
}

fn level_completion_check(
    editor_state: Res<EditorState>,
    lander_state: Res<LanderState>,
    progress: ResMut<Persistent<persistence::LevelProgress>>,
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
