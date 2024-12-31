use bevy::prelude::Resource;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum ControlScheme {
    VerticalOnly,
    ThrustVector,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Physics {
    pub g: f32,   // gravity
    pub m: f32,   // dry mass
    pub t: f32,   // max thrust
    pub isp: f32, // specific impulse
}

#[derive(Debug, Deserialize, Clone)]
pub struct InitialState {
    pub x0: f32,  // initial horizontal position
    pub y0: f32,  // initial altitude
    pub vx0: f32, // initial horizontal velocity
    pub vy0: f32, // initial vertical velocity
    pub r0: f32,  // initial rotation (radians)
    pub f0: f32,  // initial fuel
}

#[derive(Debug, Deserialize, Clone)]
pub struct SuccessCriteria {
    pub vx_max: f32, // max horizontal velocity
    pub vy_max: f32, // max vertical velocity
    pub x_min: f32,  // landing zone left boundary
    pub x_max: f32,  // landing zone right boundary
}

#[derive(Debug, Deserialize, Clone)]
pub struct LevelConfig {
    pub name: String,
    pub description: String,
    pub physics: Physics,
    pub initial: InitialState,
    pub success: SuccessCriteria,
    pub control_scheme: ControlScheme,
    pub script_api: Vec<String>,
}

#[derive(Resource)]
pub struct CurrentLevel {
    pub config: LevelConfig,
}

impl CurrentLevel {
    pub fn load(level_number: usize) -> Self {
        let file_content =
            std::fs::read_to_string(format!("assets/levels/level{}.ron", level_number))
                .expect("Failed to read level file");

        let config: LevelConfig =
            ron::de::from_str(&file_content).expect("Failed to parse level configuration");

        Self { config }
    }
}
