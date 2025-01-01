use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

use crate::{
    levels::{ControlScheme, CurrentLevel, LevelManager},
    rhai_api::{ControlType, ScriptEngine},
    simulation::{reset_simulation, LanderState},
    visualization::{CameraState, ResetVisibilityFlag},
};

const CONSOLE_HEIGHT: f32 = 500.0;

#[derive(Default, PartialEq)]
pub enum SimulationState {
    #[default]
    Stopped,
    Running,
    Paused,
}

#[derive(Resource)]
pub struct EditorState {
    pub code: String,
    pub simulation_state: SimulationState,
    pub console_height: f32,
    pub last_console_output: Vec<String>, // Store persistent console history
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            code: include_str!("../assets/scripts/hover.rhai").into(),
            simulation_state: SimulationState::Stopped,
            console_height: 150.0,
            last_console_output: Vec::new(),
        }
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut script_engine: ResMut<ScriptEngine>,
    mut lander_state: ResMut<LanderState>,
    mut current_level: ResMut<CurrentLevel>,
    mut camera_state: ResMut<CameraState>,
    mut reset_flag: ResMut<ResetVisibilityFlag>,
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
            editor_state.simulation_state = SimulationState::Stopped;
            script_engine.error_message = None;
            editor_state.last_console_output.clear(); // Clear console history on level change

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

            reset_simulation(&mut lander_state, &current_level, &mut camera_state);
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
                    // Only update the console output if we get new messages
                    if editor_state.simulation_state == SimulationState::Running {
                        let new_output = script_engine.take_console_output();
                        if !new_output.is_empty() {
                            editor_state.last_console_output = new_output;
                        }
                    }

                    // Display the last set of messages
                    for line in &editor_state.last_console_output {
                        ui.colored_label(egui::Color32::GREEN, line);
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
                let (button_text, next_state) = match editor_state.simulation_state {
                    SimulationState::Stopped => ("Run", SimulationState::Running),
                    SimulationState::Running => ("Pause", SimulationState::Paused),
                    SimulationState::Paused => ("Resume", SimulationState::Running),
                };

                if ui.button(button_text).clicked() {
                    match editor_state.simulation_state {
                        SimulationState::Stopped => {
                            // Starting from stopped state - compile and reset
                            if script_engine.compile_script(&editor_state.code).is_ok() {
                                reset_simulation(
                                    &mut lander_state,
                                    &current_level,
                                    &mut camera_state,
                                );
                                editor_state.simulation_state = next_state;
                            }
                        }
                        SimulationState::Running => {
                            // Pause the simulation
                            editor_state.simulation_state = next_state;
                        }
                        SimulationState::Paused => {
                            // Resume from pause - recompile script but don't reset
                            if script_engine.compile_script(&editor_state.code).is_ok() {
                                editor_state.simulation_state = next_state;
                            }
                        }
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
        editor_state.simulation_state = SimulationState::Stopped;
        script_engine.error_message = None;
        editor_state.last_console_output.clear(); // Clear console history on reset
        reset_simulation(&mut lander_state, &current_level, &mut camera_state);
        reset_flag.0 = true; // Set the flag to trigger visibility reset
    }
}
