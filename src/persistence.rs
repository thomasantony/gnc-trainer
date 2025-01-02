use bevy::prelude::*;
use bevy_persistent::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Resource, Serialize, Deserialize, Clone, Default)]
pub struct LevelProgress {
    pub completed_levels: Vec<usize>,
    pub max_level_reached: usize,
    pub editor_states: HashMap<usize, String>,
}

pub fn setup_persistence(mut commands: Commands) {
    let config_dir = dirs::config_dir()
        .map(|native_config_dir| native_config_dir.join("lander-game"))
        .unwrap_or(PathBuf::from("local/configuration"));

    commands.insert_resource(
        Persistent::<LevelProgress>::builder()
            .name("level_progress")
            .format(StorageFormat::Json)
            .path(config_dir.join("progress.json"))
            .default(LevelProgress::default())
            .build()
            .expect("Failed to initialize level progress"),
    );
}

pub fn mark_level_complete(
    level: usize,
    mut progress: ResMut<Persistent<LevelProgress>>,
) -> Result<(), String> {
    progress
        .update(|progress| {
            if !progress.completed_levels.contains(&level) {
                progress.completed_levels.push(level);
                progress.completed_levels.sort();
            }
            progress.max_level_reached = progress.max_level_reached.max(level);
        })
        .map_err(|e| e.to_string())
}

pub fn save_editor_state(
    level: usize,
    code: String,
    mut progress: ResMut<Persistent<LevelProgress>>,
) -> Result<(), String> {
    progress
        .update(|progress| {
            progress.editor_states.insert(level, code.clone());
        })
        .map_err(|e| e.to_string())
}

pub fn is_level_available(level: usize, progress: &Persistent<LevelProgress>) -> bool {
    level == 0 || progress.completed_levels.contains(&(level - 1))
}

pub fn is_level_completed(level: usize, progress: &Persistent<LevelProgress>) -> bool {
    progress.completed_levels.contains(&level)
}

pub fn get_editor_state(level: usize, progress: &Persistent<LevelProgress>) -> Option<String> {
    progress.editor_states.get(&level).cloned()
}
