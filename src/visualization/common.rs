use bevy::prelude::*;

// Constants moved from original visualization.rs
pub const WORLD_TO_SCREEN_SCALE: f32 = 10.0;
pub const RIGHT_PANEL_WIDTH: f32 = 600.0;
pub const GROUND_OFFSET: f32 = -200.0;
pub const MIN_VIEW_HEIGHT: f32 = 30.0;

#[derive(Resource)]
pub struct CameraState {
    pub following: bool,
    pub target_offset: Vec2,
    pub explosion_spawned: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            following: true,
            target_offset: Vec2::ZERO,
            explosion_spawned: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct ResetVisualization(pub bool);

// Common utility functions
pub fn world_to_screen(pos: Vec2, camera_offset: Vec2) -> Vec2 {
    let center_offset = -(RIGHT_PANEL_WIDTH / 2.0);

    Vec2::new(
        pos.x * WORLD_TO_SCREEN_SCALE + center_offset - camera_offset.x,
        pos.y * WORLD_TO_SCREEN_SCALE + GROUND_OFFSET - camera_offset.y,
    )
}
