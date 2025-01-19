use bevy::prelude::*;
use bevy_egui::egui::Hyperlink;
use bevy_egui::{egui, EguiContexts};
use bevy_persistent::prelude::*;
use egui_extras::syntax_highlighting;

use crate::assets::ScriptAsset;
use crate::levels::{ControlScheme, CurrentLevel, LevelManager};
use crate::persistence::{self, LevelProgress};
use crate::rhai_api::{ControlType, ScriptEngine};
use crate::simulation::{reset_simulation, LanderState};
use crate::visualization::{CameraState, ResetVisibilityFlag, ResetVisualization};

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
    pub last_console_output: Vec<String>,
    pub show_reset_confirmation: bool,
    pub default_script_handle: Handle<ScriptAsset>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            code: include_str!("../assets/scripts/level0_default.rhai").into(),
            simulation_state: SimulationState::Stopped,
            console_height: 150.0,
            last_console_output: Vec::new(),
            show_reset_confirmation: false,
            default_script_handle: Handle::default(),
        }
    }
}

#[derive(Resource, Default)]
pub struct AboutPopupState {
    pub show: bool,
}

#[derive(Resource, Default)]
pub struct HintPopupState {
    pub show: bool,
}

// Native-only imports
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

pub fn ui_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<EditorState>,
    mut script_engine: ResMut<ScriptEngine>,
    mut lander_state: ResMut<LanderState>,
    mut current_level: ResMut<CurrentLevel>,
    mut camera_state: ResMut<CameraState>,
    mut reset_flag: ResMut<ResetVisibilityFlag>,
    mut reset_vis: ResMut<ResetVisualization>,
    level_manager: Res<LevelManager>,
    mut state: ResMut<NextState<GameState>>,
    progress: ResMut<Persistent<LevelProgress>>,
    mut popup: ResMut<LevelCompletePopup>,
    mut about_popup: ResMut<AboutPopupState>,
    mut hint_popup: ResMut<HintPopupState>,
    asset_server: Res<AssetServer>,
    script_assets: Res<Assets<ScriptAsset>>,
) {
    let new_level_number = None;
    let mut reset_requested = false;

    // Top menu bar with level select button
    egui::TopBottomPanel::top("menu_bar").show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            if ui.button("Level Select").clicked() {
                if let Some((level_num, _)) = level_manager
                    .available_levels
                    .iter()
                    .find(|(_, name)| name == &current_level.config.name)
                {
                    let script_handle =
                        asset_server.load(format!("scripts/level{}_default.rhai", level_num));
                    editor_state.default_script_handle = script_handle;

                    let _ = persistence::save_editor_state(
                        *level_num,
                        editor_state.code.clone(),
                        progress,
                    );
                }
                popup.show = false;
                state.set(GameState::LevelSelect);
            }
            if ui.button("About").clicked() {
                about_popup.show = !about_popup.show;
            }
        });
    });

    // Handle level selection

    if let Some(level_num) = new_level_number {
        if let Some(new_config) = level_manager.get_level(level_num) {
            editor_state.simulation_state = SimulationState::Stopped;
            script_engine.error_message = None;
            editor_state.last_console_output.clear();

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
            reset_flag.0 = true; // Reset lander visibility
            reset_vis.0 = true; // Reset visualization
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
            ui.horizontal(|ui| {
                let rhai_link = Hyperlink::from_label_and_url("Rhai script", "https://rhai.rs")
                    .open_in_new_tab(true);
                ui.label("Write your script below to control the spacecraft.");
                ui.label("The script should be written in");
                ui.add(rhai_link);
            });

            ui.add_space(4.0);

            // Available API documentation
            ui.collapsing("Available API", |ui| {
                ui.label("Available state variables:");
                ui.label("• state[\"x\"] - horizontal position (meters)");
                ui.label("• state[\"y\"] - vertical position (meters)");
                ui.label("• state[\"vx\"] - horizontal velocity (m/s)");
                ui.label("• state[\"vy\"] - vertical velocity (m/s)");
                ui.label("• state[\"rotation\"] - rotation angle (radians)");
                ui.label("• state[\"angular_vel\"] - angular velocity (rad/s)");
                ui.label("• state[\"fuel\"] - remaining fuel mass (kg)");
                ui.add_space(4.0);

                ui.label("Helper functions:");
                ui.label("• console(value) - print debug output");
                ui.label("• user_state - persistent variable storage");
                ui.add_space(4.0);

                match current_level.config.control_scheme {
                    ControlScheme::VerticalOnly => {
                        ui.label("Control output:");
                        ui.label("Return a single number for thrust (0.0 to 1.0)");
                        ui.code("return 0.5; // 50% thrust");
                    }
                    ControlScheme::ThrustVector => {
                        ui.label("Control output:");
                        ui.label("Return an array: [thrust, gimbal]");
                        ui.label("• thrust: 0.0 to 1.0");
                        ui.label("• gimbal: -0.4 to 0.4 radians");
                        ui.code("return [0.5, 0.1]; // 50% thrust, 0.1 rad gimbal");
                    }
                }
            });

            ui.add_space(8.0);

            // Code editor
            egui::ScrollArea::vertical()
                .max_height(CONSOLE_HEIGHT)
                .show(ui, |ui| {
                    let theme = syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());

                    // Create layouter for syntax highlighting
                    let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                        let mut layout_job = syntax_highlighting::highlight(
                            ui.ctx(),
                            ui.style(),
                            &theme,
                            string,
                            "rs", // Using Rust syntax highlighting
                        );
                        layout_job.wrap.max_width = wrap_width;
                        ui.fonts(|f| f.layout_job(layout_job))
                    };

                    let editor = egui::TextEdit::multiline(&mut editor_state.code)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_rows(20)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter);

                    ui.add(editor);
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
                ui.colored_label(egui::Color32::RED, &current_level.config.failure_message);
            } else if lander_state.landed {
                ui.colored_label(egui::Color32::GREEN, &current_level.config.success_message);
            } else if lander_state.stabilizing {
                let remaining =
                    current_level.config.success.persistence_period - lander_state.success_timer;
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("Conditions met. Waiting to stabilize... {:.1}", remaining),
                );
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

                if ui.button("Reset Simulation").clicked() {
                    reset_requested = true;
                }

                if ui.button("Reset Code").clicked() {
                    editor_state.show_reset_confirmation = true;
                }

                if ui.button("Hint").clicked() {
                    about_popup.show = false; // Hide other popups
                    hint_popup.show = true;
                }

                if ui.button("Export").clicked() {
                    // Find current level number
                    if let Some((level_num, _)) = level_manager
                        .available_levels
                        .iter()
                        .find(|(_, name)| name == &current_level.config.name)
                    {
                        export_code(&editor_state.code, *level_num);
                    }
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
                if let ControlScheme::ThrustVector = current_level.config.control_scheme {
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

    // Add the confirmation dialog for "Reset Code"
    if editor_state.show_reset_confirmation {
        egui::Window::new("Confirm Reset")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(contexts.ctx_mut(), |ui| {
                ui.label("Are you sure you want to reset the code to the default script?");
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        if let Some(script_asset) =
                            script_assets.get(&editor_state.default_script_handle)
                        {
                            editor_state.code = script_asset.0.clone();
                        }
                        editor_state.show_reset_confirmation = false;
                    }
                    if ui.button("No").clicked() {
                        editor_state.show_reset_confirmation = false;
                    }
                });
            });
    }

    // Handle reset request
    if reset_requested {
        editor_state.simulation_state = SimulationState::Stopped;
        script_engine.error_message = None;
        editor_state.last_console_output.clear(); // Clear console history on reset
        reset_simulation(&mut lander_state, &current_level, &mut camera_state);
        reset_flag.0 = true; // Set the flag to trigger visibility reset
    }
}

// Level selection UI

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    LevelSelect,
    Playing,
    #[default]
    ThreeDViz, // New state for 3D visualization
}

#[derive(Resource)]
pub struct LevelCompletePopup {
    pub show: bool,
    pub completed_level: usize,
}

impl Default for LevelCompletePopup {
    fn default() -> Self {
        Self {
            show: false,
            completed_level: 0,
        }
    }
}

pub fn level_select_ui(
    mut contexts: EguiContexts,
    level_manager: Res<LevelManager>,
    progress: Res<Persistent<LevelProgress>>,
    mut editor_state: ResMut<EditorState>,
    mut current_level: ResMut<CurrentLevel>,
    mut state: ResMut<NextState<GameState>>,
    mut camera_state: ResMut<CameraState>,
    mut lander_state: ResMut<LanderState>,
    mut reset_flag: ResMut<ResetVisibilityFlag>,
    mut reset_vis: ResMut<ResetVisualization>,
    mut script_engine: ResMut<ScriptEngine>,
    mut about_popup: ResMut<AboutPopupState>,
    asset_server: Res<AssetServer>,
    script_assets: Res<Assets<ScriptAsset>>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut(), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("Level Select");
            ui.add_space(20.0);

            for (number, name) in &level_manager.available_levels {
                let available = persistence::is_level_available(*number, &progress);
                let completed = persistence::is_level_completed(*number, &progress);

                let text = format!("Level {}: {}", number, name);
                let mut button = egui::Button::new(text);

                if !available {
                    button = button.fill(egui::Color32::DARK_GRAY);
                } else if completed {
                    button = button.fill(egui::Color32::DARK_GREEN);
                }

                if available && ui.add(button).clicked() {
                    if let Some(new_config) = level_manager.get_level(*number) {
                        editor_state.simulation_state = SimulationState::Stopped;
                        current_level.config = new_config.clone();

                        // Update script engine control type
                        match new_config.control_scheme {
                            ControlScheme::VerticalOnly => {
                                script_engine.set_control_type(ControlType::Simple)
                            }
                            ControlScheme::ThrustVector => {
                                script_engine.set_control_type(ControlType::Vectored)
                            }
                        }

                        // Load default script for this level
                        let default_script_handle =
                            asset_server.load(format!("scripts/level{}_default.rhai", number));
                        editor_state.default_script_handle = default_script_handle.clone();

                        // Try to get saved code or use default
                        if let Some(saved_code) = persistence::get_editor_state(*number, &progress)
                        {
                            editor_state.code = saved_code;
                        } else if let Some(script_asset) = script_assets.get(&default_script_handle)
                        {
                            editor_state.code = script_asset.0.clone();
                        }

                        reset_simulation(&mut lander_state, &current_level, &mut camera_state);
                        reset_flag.0 = true;
                        reset_vis.0 = true;
                        state.set(GameState::Playing);
                    }
                }
            }

            ui.add_space(30.0);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.horizontal(|ui| {
                        if ui.button("Exit").clicked() {
                            std::process::exit(0);
                        }
                        ui.add_space(20.0);
                        if ui.button("About").clicked() {
                            about_popup.show = !about_popup.show;
                        }
                    });
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if ui.button("About").clicked() {
                        about_popup.show = true;
                    }
                }
            });
        });
    });
}

pub fn level_complete_popup(
    mut contexts: EguiContexts,
    mut popup: ResMut<LevelCompletePopup>,
    mut editor_state: ResMut<EditorState>,
    mut state: ResMut<NextState<GameState>>,
) {
    if popup.show {
        editor_state.simulation_state = SimulationState::Paused;

        egui::Window::new("Level Complete!")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(contexts.ctx_mut(), |ui| {
                ui.label("Congratulations! You've completed this level!");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Continue").clicked() {
                        popup.show = false;
                        state.set(GameState::LevelSelect);
                    }
                    if ui.button("Go back").clicked() {
                        popup.show = false;
                    }
                });
            });
    }
}

pub(crate) fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<NextState<GameState>>,
    mut about_popup: ResMut<AboutPopupState>,
    editor_state: Res<EditorState>,
    progress: ResMut<Persistent<LevelProgress>>,
    current_level: Res<CurrentLevel>,
    level_manager: Res<LevelManager>,
    mut popup: ResMut<LevelCompletePopup>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if about_popup.show {
            about_popup.show = false;
        } else if let Some((level_num, _)) = level_manager
            .available_levels
            .iter()
            .find(|(_, name)| name == &current_level.config.name)
        {
            // Save current editor state before switching
            let _ = persistence::save_editor_state(*level_num, editor_state.code.clone(), progress);
        }
        popup.show = false;
        state.set(GameState::LevelSelect);
    }
}

pub fn about_popup(
    mut contexts: EguiContexts,
    mut popup: ResMut<AboutPopupState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if popup.show {
        egui::Window::new("About GNC Trainer")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(contexts.ctx_mut(), |ui| {
                ui.vertical_centered(|ui| {
                    // Make everything centered
                    ui.add_space(16.0);
                    ui.heading("GNC Trainer");
                    ui.add_space(8.0);

                    ui.label("Thanks for playing!");
                    ui.add_space(4.0);
                    ui.label("\u{00A9} Thomas Antony. 2025");

                    ui.add_space(8.0);
                    let website_link = egui::widgets::Hyperlink::from_label_and_url(
                        "Project Homepage",
                        "https://www.thomasantony.com/gnc-trainer",
                    )
                    .open_in_new_tab(true);
                    ui.add(website_link);
                    ui.add_space(2.0);

                    let github_link = egui::widgets::Hyperlink::from_label_and_url(
                        "Source Code",
                        "https://www.github.com/thomasantony/gnc-trainer",
                    )
                    .open_in_new_tab(true);
                    ui.add(github_link);

                    ui.add_space(16.0);
                    if ui.button("Close").clicked() || keys.just_pressed(KeyCode::Escape) {
                        popup.show = false;
                    }
                });
            });
    }
}

pub fn export_code(code: &str, level_num: usize) {
    let filename = format!("level{}_solution.rhai", level_num);

    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Array;
        use wasm_bindgen::JsCast;
        use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

        // Create a Blob containing the code
        let properties = BlobPropertyBag::new();
        properties.set_type("text/plain");
        let blob_parts = Array::new();
        blob_parts.push(&js_sys::JsString::from(code));
        let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &properties)
            .expect("Failed to create blob");

        // Create download URL
        let url = Url::create_object_url_with_blob(&blob).expect("Failed to create object URL");

        // Create and click a temporary download link
        let document = web_sys::window()
            .expect("Failed to get window")
            .document()
            .expect("Failed to get document");

        let anchor = document
            .create_element("a")
            .expect("Failed to create anchor")
            .dyn_into::<HtmlAnchorElement>()
            .expect("Failed to convert to anchor");

        anchor.set_href(&url);
        anchor.set_download(&filename);
        anchor.click();

        // Clean up
        Url::revoke_object_url(&url).expect("Failed to revoke object URL");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(path) = FileDialog::new().set_file_name(&filename).save_file() {
            std::fs::write(path, code).expect("Failed to write file");
        }
    }
}

pub fn handle_script_loading(
    mut editor_state: ResMut<EditorState>,
    script_assets: Res<Assets<ScriptAsset>>,
    progress: Res<Persistent<LevelProgress>>,
    level_manager: Res<LevelManager>,
    current_level: Res<CurrentLevel>,
) {
    // Only try to load if we have an empty code buffer
    if editor_state.code.is_empty() {
        // Get current level number
        if let Some((level_num, _)) = level_manager
            .available_levels
            .iter()
            .find(|(_, name)| name == &current_level.config.name)
        {
            // First try to get the saved code
            if let Some(saved_code) = persistence::get_editor_state(*level_num, &progress) {
                editor_state.code = saved_code;
            }
            // If no saved code and the asset is loaded, use the default
            else if let Some(script_asset) =
                script_assets.get(&editor_state.default_script_handle)
            {
                editor_state.code = script_asset.0.clone();
            }
        }
    }
}

pub fn hint_popup(
    mut contexts: EguiContexts,
    mut popup: ResMut<HintPopupState>,
    level: Res<CurrentLevel>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if popup.show {
        egui::Window::new("Level Hint")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(contexts.ctx_mut(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(&level.config.hint);
                    ui.add_space(16.0);
                    if ui.button("Close").clicked() || keys.just_pressed(KeyCode::Escape) {
                        popup.show = false;
                    }
                });
            });
    }
}
