use bevy::{prelude::Resource, utils::hashbrown::HashMap};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum ControlScheme {
    VerticalOnly,
    ThrustVector,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Physics {
    pub gravity: f32,    // gravity acceleration (m/s²)
    pub dry_mass: f32,   // dry mass of the lander (kg)
    pub max_thrust: f32, // maximum thrust force (N)
    pub isp: f32,        // specific impulse (s)
}

#[derive(Debug, Deserialize, Clone)]
pub struct InitialState {
    pub x0: f32,            // initial horizontal position
    pub y0: f32,            // initial altitude
    pub vx0: f32,           // initial horizontal velocity
    pub vy0: f32,           // initial vertical velocity
    pub initial_angle: f32, // initial rotation (radians)
    pub initial_fuel: f32,  // initial fuel mass (kg)
}

#[derive(Debug, Deserialize, Clone)]
pub enum Reference {
    Absolute, // Compare against absolute coordinates
    Initial,  // Compare against initial state
}

#[derive(Debug, Deserialize, Clone)]
pub struct BoundingBox {
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
    pub reference: Reference,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SuccessCriteria {
    pub vx_max: f32,               // max horizontal velocity
    pub vy_max: f32,               // max vertical velocity
    pub position_box: BoundingBox, // Box defining valid positions
    pub final_angle: f32,          // desired final angle (radians)
    pub angle_tolerance: f32,      // acceptable deviation from final angle (radians)
    pub persistence_period: f32,   // time criteria must be met (seconds)
}

#[derive(Debug, Deserialize, Clone)]
pub struct FailureCriteria {
    pub ground_collision: bool, // whether ground collision is an instant fail
    pub bounds: Option<BoundingBox>, // Optional out-of-bounds box that causes failure
}

#[derive(Debug, Deserialize, Clone)]
pub struct LevelConfig {
    pub name: String,
    pub description: String,
    pub physics: Physics,
    pub initial: InitialState,
    pub success: SuccessCriteria,
    pub failure: FailureCriteria,
    pub control_scheme: ControlScheme,
    pub success_message: String,
    pub failure_message: String,
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

        // Try loading levels starting from 0
        for i in 0..=10 {
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
