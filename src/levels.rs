use bevy::asset::AssetLoader;
use bevy::prelude::*;
use bevy::utils::hashbrown::HashMap;
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

#[derive(Default, Resource)]
pub struct LevelManager {
    pub levels: HashMap<usize, LevelConfig>,
    pub available_levels: Vec<(usize, String)>, // (level number, name)
    loading: bool,
    #[allow(dead_code)]
    handles: Vec<Handle<RonAsset>>, // Keep handles alive
}

// Asset loader for RON files
#[derive(Asset, TypePath, Debug)]
pub struct RonAsset(pub String);

#[derive(Default)]
pub struct RonAssetLoader;

impl AssetLoader for RonAssetLoader {
    type Asset = RonAsset;
    type Settings = ();
    type Error = std::io::Error;

    fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let content = String::from_utf8_lossy(&bytes).to_string();
            Ok(RonAsset(content))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

impl LevelManager {
    pub fn new() -> Self {
        Self {
            levels: HashMap::new(),
            available_levels: Vec::new(),
            loading: true,
            handles: Vec::new(),
        }
    }

    pub fn process_level(&mut self, level_num: usize, content: &str) {
        if let Ok(config) = ron::de::from_str::<LevelConfig>(content) {
            self.available_levels.push((level_num, config.name.clone()));
            self.levels.insert(level_num, config);
        }
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

// System to load level files
pub fn load_levels(
    mut level_manager: ResMut<LevelManager>,
    asset_server: Res<AssetServer>,
    ron_assets: Res<Assets<RonAsset>>,
    mut ev_asset: EventReader<AssetEvent<RonAsset>>,
) {
    // Process any asset events
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::LoadedWithDependencies { id } => {
                if let Some(asset) = ron_assets.get(*id) {
                    // Extract level number from handle
                    if let Some(path) = asset_server.get_path(*id) {
                        let path_str = path.path().to_string_lossy();
                        // Look for "levelX.ron" in the path
                        if let Some(file_name) = path_str.split('/').last() {
                            if file_name.starts_with("level") && file_name.ends_with(".ron") {
                                // Extract number between "level" and ".ron"
                                if let Ok(num) = file_name[5..file_name.len() - 4].parse::<usize>()
                                {
                                    level_manager.process_level(num, &asset.0);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Check if all expected levels are loaded
    if level_manager.is_loading() && !level_manager.available_levels.is_empty() {
        level_manager.mark_loaded();
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
    // Create level manager with handles
    let mut level_manager = LevelManager::new();

    // Load all potential level files
    for i in 0..=10 {
        let path = format!("levels/level{}.ron", i);
        let handle = asset_server.load::<RonAsset>(path);
        level_manager.handles.push(handle);
    }

    commands.insert_resource(level_manager);
}
