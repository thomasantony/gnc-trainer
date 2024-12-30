use crate::rhai_api::ScriptEngine;
use crate::simulation::{reset_simulation, LanderState, SimulationParams};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

#[derive(Resource)]
pub struct EditorState {
    pub code: String,
    pub is_running: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            code: r#"// Function to calculate thrust based on current lander state
// Returns a value between 0.0 (no thrust) and 1.0 (full thrust)
//
// Available state variables:
//   state["altitude"]  -> altitude in meters
//   state["velocity"]  -> vertical velocity in m/s (positive is upward)
//   state["fuel"]      -> remaining fuel in kg

// Simple example: more thrust when going down, less when going up
let target_velocity = if state["altitude"] > 10.0 { -5.0 } else { -1.0 };
let error = state["velocity"] - target_velocity;

// P controller
let k_p = 10.0;
let thrust = 0.5 - k_p * error;

// Ensure we don't waste fuel if we're going up too fast
if state["velocity"] > 2.0 {
    0.0
} else {
    thrust
}"#
            .into(),
            is_running: false,
        }
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut script_engine: ResMut<ScriptEngine>,
    mut lander_state: ResMut<LanderState>,
    params: Res<SimulationParams>,
) {
    let mut reset_requested = false;
    let start_stop_requested = false;

    egui::SidePanel::right("code_panel")
        .default_width(600.0)
        .show(contexts.ctx_mut(), |ui| {
            // Code editor
            CodeEditor::default()
                .id_source("code_editor")
                .with_rows(20)
                .with_fontsize(14.0)
                .with_theme(ColorTheme::GRUVBOX)
                .with_syntax(Syntax::rust())
                .with_numlines(true)
                .show(ui, &mut editor_state.code);

            ui.add_space(8.0);

            // Status and error messages
            if let Some(error) = &script_engine.error_message {
                ui.colored_label(egui::Color32::RED, error);
            } else if lander_state.crashed {
                ui.colored_label(egui::Color32::RED, "Crashed!");
            } else if lander_state.landed {
                ui.colored_label(egui::Color32::GREEN, "Landed successfully!");
            }

            // Control buttons
            ui.horizontal(|ui| {
                if ui
                    .button(if editor_state.is_running {
                        "Stop"
                    } else {
                        "Run"
                    })
                    .clicked()
                {
                    if !editor_state.is_running {
                        // Compile script before starting
                        if script_engine.compile_script(&editor_state.code).is_ok() {
                            editor_state.is_running = true;
                            reset_simulation(&mut lander_state, &params);
                        }
                    } else {
                        editor_state.is_running = false;
                    }
                }

                if ui.button("Reset").clicked() {
                    reset_requested = true;
                }
            });
        });

    // Bottom panel for telemetry
    egui::TopBottomPanel::bottom("telemetry")
        .min_height(50.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Altitude: {:.1} m", lander_state.position.y));
                ui.add_space(20.0);
                ui.label(format!("Velocity: {:.1} m/s", lander_state.velocity.y));
                ui.add_space(20.0);
                ui.label(format!("Fuel: {:.1} kg", lander_state.fuel));
                ui.add_space(20.0);
                ui.label(format!(
                    "Thrust: {}%",
                    (lander_state.thrust_level * 100.0) as i32
                ));
            });
        });

    // Handle state changes outside of the UI closure
    if reset_requested {
        editor_state.is_running = false;
        script_engine.error_message = None;
        reset_simulation(&mut lander_state, &params);
    }

    if start_stop_requested {
        editor_state.is_running = !editor_state.is_running;
        if editor_state.is_running {
            reset_simulation(&mut lander_state, &params);
        }
    }
}
