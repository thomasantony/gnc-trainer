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

pub fn ui_system(mut contexts: EguiContexts, mut editor_state: Local<EditorState>) {
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
                    editor_state.is_running = !editor_state.is_running;
                }
                if ui.button("Reset").clicked() {
                    // TODO: Reset simulation state
                }
            });
        });

    // Bottom panel for telemetry
    egui::TopBottomPanel::bottom("telemetry")
        .min_height(50.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Altitude: 0.0 m");
                ui.add_space(20.0);
                ui.label("Velocity: 0.0 m/s");
                ui.add_space(20.0);
                ui.label("Fuel: 100.0 kg");
                ui.add_space(20.0);
                ui.label("Thrust: 0%");
            });
        });
}
