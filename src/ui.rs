use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

use crate::{
    levels::{ControlScheme, CurrentLevel},
    rhai_api::{ControlType, ScriptEngine},
    simulation::{reset_simulation, LanderState},
};

#[derive(Resource)]
pub struct EditorState {
    pub code: String,
    pub is_running: bool,
    pub current_level: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            code: include_str!("../assets/scripts/level1_default.rhai").into(),
            is_running: false,
            current_level: 1,
        }
    }
}

pub fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut script_engine: ResMut<ScriptEngine>,
    mut lander_state: ResMut<LanderState>,
    mut level: ResMut<CurrentLevel>,
) {
    let mut reset_requested = false;
    let mut level_changed = false;

    // Top menu bar
    egui::TopBottomPanel::top("menu_bar").show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Levels", |ui| {
                if ui.button("Level 1: Vertical Landing").clicked() {
                    editor_state.current_level = 1;
                    level_changed = true;
                    ui.close_menu();
                }
                if ui.button("Level 2: Precision Landing").clicked() {
                    editor_state.current_level = 2;
                    level_changed = true;
                    ui.close_menu();
                }
            });
        });
    });

    // Code editor panel
    egui::SidePanel::right("code_panel")
        .default_width(600.0)
        .show(contexts.ctx_mut(), |ui| {
            // Level description
            ui.heading(&level.config.name);
            ui.label(&level.config.description);
            ui.add_space(8.0);

            // Available API documentation
            ui.collapsing("Available API", |ui| {
                ui.label("Script state variables:");
                for var in &level.config.script_api {
                    ui.label(format!("• state[\"{}\"]", var));
                }
                ui.add_space(4.0);

                match level.config.control_scheme {
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
                            reset_simulation(&mut lander_state, &level);
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
                if matches!(level.config.control_scheme, ControlScheme::ThrustVector) {
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

    // Handle level changes
    if level_changed {
        editor_state.is_running = false;
        script_engine.error_message = None;

        // Load new level
        *level = CurrentLevel::load(editor_state.current_level);

        // Update script engine control type
        match level.config.control_scheme {
            ControlScheme::VerticalOnly => script_engine.set_control_type(ControlType::Simple),
            ControlScheme::ThrustVector => script_engine.set_control_type(ControlType::Vectored),
        }

        // Load default script for level
        editor_state.code = std::fs::read_to_string(format!(
            "assets/scripts/level{}_default.rhai",
            editor_state.current_level
        ))
        .unwrap_or_else(|_| String::new());

        reset_simulation(&mut lander_state, &level);
    }

    // Handle reset request
    if reset_requested {
        editor_state.is_running = false;
        script_engine.error_message = None;
        reset_simulation(&mut lander_state, &level);
    }
}
