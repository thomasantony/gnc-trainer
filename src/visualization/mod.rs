mod common;
mod viz_2d;

pub use common::*;

use crate::levels::CurrentLevel;
use bevy::prelude::*;

// Re-export the main types that other modules need
pub use common::{CameraState, ResetVisualization};
pub use viz_2d::components::{MainCamera, ResetVisibilityFlag};

#[derive(Component)]
#[allow(dead_code)]
pub enum VisualizationType {
    Viz2D,
    // Viz3D placeholder for future
}

// Plugin that sets up visualization systems
pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<viz_2d::particles::ParticleSpawnTimer>()
            .add_systems(
                Update,
                (
                    viz_2d::systems::update_visualization,
                    viz_2d::systems::update_grid_lines,
                    viz_2d::systems::reset_lander_visibility,
                    viz_2d::systems::reset_visualization_system,
                    viz_2d::particles::particle_system,
                ),
            );
    }
}

// Common spawn function that delegates to correct visualization
pub fn spawn_visualization(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    level: Res<CurrentLevel>,
) {
    // For now, always use 2D visualization
    viz_2d::systems::spawn_visualization(commands, meshes, materials, level);
}
