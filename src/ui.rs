use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

use crate::{
    levels::{ControlScheme, CurrentLevel, LevelManager},
    rhai_api::{ControlType, ScriptEngine},
    simulation::{reset_simulation, LanderState},
};

const CONSOLE_HEIGHT: f32 = 500.0;

#[derive(Resource)]
pub struct EditorState {
    pub code: String,
    pub is_running: bool,
    pub console_height: f32, // Height of console panel
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            // Default script now includes function definition
            code: r#"// Example hover script
fn control(state) {
    // Simple vertical-only control script
    let target_velocity = if state["y"] > 10.0 { -5.0 } else { -1.0 };
    let error = state["vy"] - target_velocity;

    // P controller
    let k_p = 10.0;
    0.5 - k_p * error
}
}"#
            .into(),
            is_running: false,
            console_height: 150.0, // Default console height
        }
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut script_engine: ResMut<ScriptEngine>,
    mut lander_state: ResMut<LanderState>,
    mut current_level: ResMut<CurrentLevel>,
    level_manager: Res<LevelManager>,
) {
    let mut reset_requested = false;
    let mut new_level_number = None;

    // Top menu bar
    egui::TopBottomPanel::top("menu_bar").show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Levels", |ui| {
                for (number, name) in &level_manager.available_levels {
                    if ui.button(format!("Level {}: {}", number, name)).clicked() {
                        new_level_number = Some(*number);
                        ui.close_menu();
                    }
                }
            });
        });
    });

    // Handle level selection
    if let Some(level_num) = new_level_number {
        if let Some(new_config) = level_manager.get_level(level_num) {
            editor_state.is_running = false;
            script_engine.error_message = None;

            // Update current level
            current_level.config = new_config.clone();

            // Update script engine control type
            match new_config.control_scheme {
                ControlScheme::VerticalOnly => script_engine.set_control_type(ControlType::Simple),
                ControlScheme::ThrustVector => {
                    script_engine.set_control_type(ControlType::Vectored)
                }
            }

            // Load default script for level
            if let Ok(script) =
                std::fs::read_to_string(format!("assets/scripts/level{}_default.rhai", level_num))
            {
                editor_state.code = script;
            }

            reset_simulation(&mut lander_state, &current_level);
        }
    }

    // Right panel with code editor
    egui::SidePanel::right("code_panel")
        .default_width(600.0)
        .show(contexts.ctx_mut(), |ui| {
            // Level info
            ui.heading(&current_level.config.name);
            ui.label(&current_level.config.description);
            ui.add_space(8.0);

            // Available API documentation
            ui.collapsing("Available API", |ui| {
                ui.label("Script state variables:");
                for var in &current_level.config.script_api {
                    ui.label(format!("• state[\"{}\"]", var));
                }
                ui.add_space(4.0);

                match current_level.config.control_scheme {
                    ControlScheme::VerticalOnly => {
                        ui.label("Return control with:");
                        ui.code("simple_control(thrust)");
                        ui.label("where thrust is between 0.0 and 1.0");
                    }
                    ControlScheme::ThrustVector => {
                        ui.label("Return control with:");
                        ui.code("vector_control(thrust, gimbal)");
                        ui.label("where:");
                        ui.label("• thrust is between 0.0 and 1.0");
                        ui.label("• gimbal is between -0.4 and 0.4 radians");
                    }
                }
            });

            ui.add_space(8.0);

            // Code editor
            egui::ScrollArea::vertical()
                .max_height(CONSOLE_HEIGHT)
                .show(ui, |ui| {
                    CodeEditor::default()
                        .id_source("code_editor")
                        .with_rows(20)
                        .with_fontsize(14.0)
                        .with_theme(ColorTheme::GRUVBOX)
                        .with_syntax(Syntax::rust())
                        .with_numlines(true)
                        .vscroll(true)
                        .stick_to_bottom(true)
                        .show(ui, &mut editor_state.code);
                    ui.add_space(8.0);
                });

            // Console output
            ui.label("Console Output");
            egui::ScrollArea::vertical()
                .id_salt(1234)
                .max_height(editor_state.console_height)
                .show(ui, |ui| {
                    let console_output = script_engine.take_console_output();
                    for line in console_output {
                        ui.colored_label(egui::Color32::GREEN, line.clone());
                    }
                });

            // Status messages
            if let Some(error) = &script_engine.error_message {
                ui.colored_label(egui::Color32::RED, error);
            } else if lander_state.crashed {
                ui.colored_label(egui::Color32::RED, "Crashed!");
            } else if lander_state.landed {
                ui.colored_label(egui::Color32::GREEN, "Landed successfully!");
            }

            // Control buttons
            ui.horizontal(|ui| {
                let run_text = if editor_state.is_running {
                    "Stop"
                } else {
                    "Run"
                };

                if ui.button(run_text).clicked() {
                    if !editor_state.is_running {
                        // Compile script before starting
                        if script_engine.compile_script(&editor_state.code).is_ok() {
                            editor_state.is_running = true;
                            reset_simulation(&mut lander_state, &current_level);
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

    // Bottom telemetry panel
    egui::TopBottomPanel::bottom("telemetry")
        .min_height(80.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                // Position
                ui.vertical(|ui| {
                    ui.label("Position:");
                    ui.label(format!("X: {:.1} m", lander_state.position.x));
                    ui.label(format!("Y: {:.1} m", lander_state.position.y));
                });

                ui.add_space(20.0);

                // Velocity
                ui.vertical(|ui| {
                    ui.label("Velocity:");
                    ui.label(format!("VX: {:.1} m/s", lander_state.velocity.x));
                    ui.label(format!("VY: {:.1} m/s", lander_state.velocity.y));
                });

                ui.add_space(20.0);

                // Rotation (only show for thrust vector control)
                match current_level.config.control_scheme {
                    ControlScheme::ThrustVector => {
                        ui.vertical(|ui| {
                            ui.label("Rotation:");
                            ui.label(format!("Angle: {:.1}°", lander_state.rotation.to_degrees()));
                            ui.label(format!(
                                "Gimbal: {:.1}°",
                                lander_state.gimbal_angle.to_degrees()
                            ));
                        });
                        ui.add_space(20.0);
                    }
                    _ => {}
                }

                // Thrust and fuel
                ui.vertical(|ui| {
                    ui.label("Engine:");
                    ui.label(format!(
                        "Thrust: {}%",
                        (lander_state.thrust_level * 100.0) as i32
                    ));
                    ui.label(format!("Fuel: {:.1} kg", lander_state.fuel));
                });
            });
        });

    // Handle reset request
    if reset_requested {
        editor_state.is_running = false;
        script_engine.error_message = None;
        reset_simulation(&mut lander_state, &current_level);
    }
}
