use bevy::{prelude::Resource, utils::hashbrown::HashMap};
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
}

#[derive(Resource)]
pub struct LevelManager {
    pub levels: HashMap<usize, LevelConfig>,
    pub available_levels: Vec<(usize, String)>, // (level number, name)
}

impl LevelManager {
    pub fn load() -> Self {
        let mut levels = HashMap::new();
        let mut available_levels = Vec::new();

        // Try loading levels from 1 to 10 (arbitrary limit)
        for i in 1..=10 {
            if let Ok(content) = std::fs::read_to_string(format!("assets/levels/level{}.ron", i)) {
                if let Ok(config) = ron::de::from_str::<LevelConfig>(&content) {
                    available_levels.push((i, config.name.clone()));
                    levels.insert(i, config);
                }
            }
        }

        Self {
            levels,
            available_levels,
        }
    }

    pub fn get_level(&self, number: usize) -> Option<LevelConfig> {
        self.levels.get(&number).cloned()
    }
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
