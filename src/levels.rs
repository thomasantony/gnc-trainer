use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
use serde::Deserialize;

use crate::assets::{RonAsset, RonAssetLoader};

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
    pub hint: String, // Add this line
    pub physics: Physics,
    pub initial: InitialState,
    pub success: SuccessCriteria,
    pub failure: FailureCriteria,
    pub control_scheme: ControlScheme,
    pub success_message: String,
    pub failure_message: String,
}

#[derive(Debug, Deserialize)]
pub struct LevelList {
    pub levels: Vec<String>, // List of level file names without extension
}

#[derive(Default, Resource)]
pub struct LevelManager {
    pub levels: HashMap<usize, LevelConfig>,
    pub available_levels: Vec<(usize, String)>, // (level number, name)
    loading: bool,
    #[allow(dead_code)]
    level_handles: Vec<Handle<RonAsset>>,
    level_list: Option<LevelList>,
    loaded_configs: Vec<(usize, LevelConfig)>, // Temporary storage for loaded configs
}
impl LevelManager {
    pub fn new() -> Self {
        Self {
            levels: HashMap::new(),
            available_levels: Vec::new(),
            loading: true,
            level_handles: Vec::new(),
            level_list: None,
            loaded_configs: Vec::new(),
        }
    }

    // Sort and finalize loaded levels
    fn finalize_loading(&mut self) {
        // Sort loaded configs by index
        self.loaded_configs.sort_by_key(|(idx, _)| *idx);

        // Clear existing data
        self.levels.clear();
        self.available_levels.clear();

        // Insert in correct order
        for (idx, config) in self.loaded_configs.drain(..) {
            self.available_levels.push((idx, config.name.clone()));
            self.levels.insert(idx, config);
        }

        self.loading = false;
    }

    pub fn get_level(&self, number: usize) -> Option<LevelConfig> {
        self.levels.get(&number).cloned()
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn mark_loaded(&mut self) {
        self.loading = false;
    }
}

#[derive(Resource)]
pub struct CurrentLevel {
    pub config: LevelConfig,
}
pub fn load_levels(
    mut level_manager: ResMut<LevelManager>,
    asset_server: Res<AssetServer>,
    ron_assets: Res<Assets<RonAsset>>,
    mut ev_asset: EventReader<AssetEvent<RonAsset>>,
) {
    for ev in ev_asset.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev {
            if let Some(asset) = ron_assets.get(*id) {
                if let Some(path) = asset_server.get_path(*id) {
                    let path_str = path.path().to_string_lossy();

                    // Check if this is the level list
                    if path_str.ends_with("level_list.ron") {
                        if let Ok(list) = ron::de::from_str::<LevelList>(&asset.0) {
                            // Load all levels from the list
                            for level_file in &list.levels {
                                let handle = asset_server
                                    .load::<RonAsset>(format!("levels/{}.ron", level_file));
                                level_manager.level_handles.push(handle);
                            }
                            level_manager.level_list = Some(list);
                        }
                    } else if path_str.contains("level") && path_str.ends_with(".ron") {
                        // Process individual level file
                        if let Some(file_name) = path_str.split('/').last() {
                            if let Ok(config) = ron::de::from_str::<LevelConfig>(&asset.0) {
                                // Get level index from level list
                                if let Some(list) = &level_manager.level_list {
                                    if let Some(index) = list
                                        .levels
                                        .iter()
                                        .position(|name| format!("{}.ron", name) == file_name)
                                    {
                                        // Store in temporary vector instead of inserting directly
                                        level_manager.loaded_configs.push((index, config));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check if loading is complete
    if level_manager.is_loading()
        && level_manager.level_list.is_some()
        && !level_manager.loaded_configs.is_empty()
        && level_manager.loaded_configs.len()
            == level_manager.level_list.as_ref().unwrap().levels.len()
    {
        // Finalize loading by sorting and inserting in correct order
        level_manager.finalize_loading();
    }
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameLoadState {
    #[default]
    Loading,
    Ready,
}

// Plugin to set up the level system
pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameLoadState>()
            .init_asset::<RonAsset>()
            .init_asset_loader::<RonAssetLoader>()
            .init_resource::<LevelManager>()
            .add_systems(Startup, setup_levels)
            .add_systems(Update, load_levels)
            .add_systems(
                Update,
                check_loading_complete.run_if(in_state(GameLoadState::Loading)),
            );
    }
}

// System to check if loading is complete and transition state
fn check_loading_complete(
    level_manager: Res<LevelManager>,
    mut next_state: ResMut<NextState<GameLoadState>>,
    mut commands: Commands,
) {
    if !level_manager.is_loading() {
        if let Some(config) = level_manager.get_level(0) {
            // Create CurrentLevel resource once we have the data
            commands.insert_resource(CurrentLevel { config });
            next_state.set(GameLoadState::Ready);
        }
    }
}

fn setup_levels(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut level_manager = LevelManager::new();

    // First load the level list
    let list_handle = asset_server.load::<RonAsset>("levels/level_list.ron");
    level_manager.level_handles.push(list_handle);

    commands.insert_resource(level_manager);
}
