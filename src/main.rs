use bevy::{asset::AssetMetaCheck, log::LogPlugin, prelude::*};
use bevy_egui::EguiPlugin;

mod assets;
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
    about_popup, handle_escape, handle_script_loading, hint_popup, level_complete_popup,
    level_select_ui, ui_system, AboutPopupState, EditorState, GameState, HintPopupState,
    LevelCompletePopup, SimulationState,
};
use visualization::{
    reset_lander_visibility, spawn_visualization, update_grid_lines, update_visualization,
    CameraState, MainCamera, ResetVisibilityFlag,
};

#[cfg(target_arch = "wasm32")]
fn is_mobile() -> bool {
    let window = web_sys::window().expect("should have window");
    let navigator = window.navigator();
    let user_agent = navigator.user_agent().expect("should have user agent");

    // Common mobile platform keywords
    let mobile_keywords = [
        "Android",
        "iPhone",
        "iPad",
        "iPod",
        "webOS",
        "BlackBerry",
        "Windows Phone",
    ];

    mobile_keywords
        .iter()
        .any(|&keyword| user_agent.contains(keyword))
}

#[cfg(not(target_arch = "wasm32"))]
fn is_mobile() -> bool {
    false
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "GNC Trainer".into(),
                        resolution: (1280., 720.).into(),
                        // Enable default event handling on mobile for virtual keyboard
                        // Disable on desktop for copy-paste shortcuts
                        prevent_default_event_handling: is_mobile(),
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
        .insert_resource(HintPopupState::default())
        .init_state::<GameState>()
        .insert_resource(State::new(GameState::LevelSelect))
        .insert_resource(LevelCompletePopup::default())
        .init_asset::<assets::ScriptAsset>()
        .init_asset_loader::<assets::ScriptAssetLoader>()
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
                    handle_script_loading,
                    hint_popup,
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
