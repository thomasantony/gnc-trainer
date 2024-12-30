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
fn calculate_control(state) {
    // TODO: Write your landing algorithm
    // Available in state:
    //   state.position.y  -> altitude in meters
    //   state.velocity.y  -> vertical velocity in m/s
    //   state.fuel       -> remaining fuel in kg
    
    // Return thrust level (0.0 to 1.0)
    0.5  // constant thrust for now
}"#
            .into(),
            is_running: false,
        }
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut lander_state: ResMut<LanderState>,
    params: Res<SimulationParams>,
) {
    let mut reset_requested = false;
    let mut start_stop_requested = false;

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

            // Status message
            if lander_state.crashed {
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
                    start_stop_requested = true;
                }

                if ui.button("Reset").clicked() {
                    reset_requested = true;
                }
            });

            // Manual thrust control (temporary until RHAI is implemented)
            if editor_state.is_running && !lander_state.crashed && !lander_state.landed {
                ui.add(egui::Slider::new(&mut lander_state.thrust_level, 0.0..=1.0).text("Thrust"));
            }
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
        reset_simulation(&mut lander_state, &params);
    }

    if start_stop_requested {
        editor_state.is_running = !editor_state.is_running;
        if editor_state.is_running {
            lander_state.thrust_level = 0.5; // Default thrust level until RHAI is implemented
        }
    }
}
