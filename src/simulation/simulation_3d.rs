// simulation_3d.rs
use super::LanderState;
use crate::constants::LANDER_BASE_OFFSET;
use crate::levels::CurrentLevel;
use crate::rhai_api::ScriptEngine;
use crate::visualization::CameraState;
use bevy::prelude::*;

pub fn update_3d(
    time: &Time,
    state: &mut LanderState,
    level: &CurrentLevel,
    script_engine: &mut ScriptEngine,
) {
    let dt = time.delta_secs();

    // Basic 6DOF implementation for now
    if !state.landed && !state.crashed {
        // Update position
        state.position += state.velocity * dt;

        // Update rotation
        let angle = state.angular_vel.length() * dt;
        if angle > 0.0 {
            let axis = state.angular_vel.normalize();
            let delta_rot = Quat::from_axis_angle(axis, angle);
            state.rotation *= delta_rot;
        }

        // Ground collision check
        if state.position.y <= LANDER_BASE_OFFSET {
            state.position.y = LANDER_BASE_OFFSET;
            state.landed = true;
        }
    }
}

pub fn reset_3d(state: &mut LanderState, level: &CurrentLevel, camera_state: &mut CameraState) {
    let initial_height = level.config.initial.y0;
    // TODO: Change this to eventually use data from the level config
    let initial_radius = 1737.1e3 + initial_height;

    *state = LanderState {
        // Start some distance above moon surface
        position: Vec3::new(initial_radius, 0.0, 0.0),
        // Initial orbital velocity
        velocity: Vec3::new(0.0, level.config.initial.vx0, 0.0),
        // Default orientation pointing along surface normal (radially outward)
        rotation: Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        angular_vel: Vec3::ZERO,
        fuel: level.config.initial.initial_fuel,
        thrust_level: 0.0,
        gimbal_angle: 0.0,
        crashed: false,
        landed: false,
        success_timer: 0.0,
        stabilizing: false,
    };

    // Reset camera
    camera_state.following = true;
    camera_state.target_offset = Vec2::ZERO;
    camera_state.explosion_spawned = false;
}
