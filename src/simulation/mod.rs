// simulation/mod.rs
use crate::{
    constants::LANDER_BASE_OFFSET,
    levels::{CurrentLevel, DynamicsType, Reference},
    rhai_api::ScriptEngine,
    visualization::CameraState,
};
use bevy::prelude::*;

mod simulation_2d;
mod simulation_3d;

// Common state that works for both 2D/3D
#[derive(Resource)]
pub struct LanderState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub angular_vel: Vec3,
    pub fuel: f32,
    pub thrust_level: f32,
    pub gimbal_angle: f32,
    pub crashed: bool,
    pub landed: bool,
    pub success_timer: f32,
    pub stabilizing: bool,
}

impl Default for LanderState {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_vel: Vec3::ZERO,
            fuel: 0.0,
            thrust_level: 0.0,
            gimbal_angle: 0.0,
            crashed: false,
            landed: false,
            success_timer: 0.0,
            stabilizing: false,
        }
    }
}

pub fn check_success_conditions(state: &LanderState, level: &CurrentLevel) -> bool {
    let config = &level.config;

    // Check velocity constraints
    let speed_ok = state.velocity.x.abs() <= config.success.vx_max
        && state.velocity.y.abs() <= config.success.vy_max;

    // Check angle constraints - extract 2D angle from quaternion for 2D case
    let current_angle = match level.config.dynamics_type {
        DynamicsType::Dynamics2D => state.rotation.to_euler(EulerRot::XYZ).2,
        DynamicsType::Dynamics3D => {
            // TODO: For 3D, we'll need different angle success criteria
            // For now just check Z rotation
            state.rotation.to_euler(EulerRot::XYZ).2
        }
    };

    let angle_ok =
        (current_angle - config.success.final_angle).abs() <= config.success.angle_tolerance;

    // Position checks remain the same since we're only using x,y components
    let position_ok = match config.success.position_box.reference {
        Reference::Initial => {
            let initial_pos = Vec2::new(config.initial.x0, config.initial.y0);
            let rel_pos = Vec2::new(state.position.x, state.position.y) - initial_pos;
            rel_pos.x >= config.success.position_box.x_min
                && rel_pos.x <= config.success.position_box.x_max
                && rel_pos.y >= config.success.position_box.y_min
                && rel_pos.y <= config.success.position_box.y_max
        }
        Reference::Absolute => {
            if state.position.y <= LANDER_BASE_OFFSET + 0.1 {
                state.position.x >= config.success.position_box.x_min
                    && state.position.x <= config.success.position_box.x_max
                    && state.position.y >= config.success.position_box.y_min
                    && state.position.y <= config.success.position_box.y_max
            } else {
                false
            }
        }
    };

    speed_ok && position_ok && angle_ok
}

fn check_failure_conditions(state: &LanderState, level: &CurrentLevel) -> bool {
    let config = &level.config;

    // Check ground collision based on the flag
    if state.position.y <= LANDER_BASE_OFFSET {
        if config.failure.ground_collision {
            // If ground_collision flag is true, any contact is failure
            return true;
        } else {
            // Otherwise, check if landing was too hard
            let hard_landing = state.velocity.x.abs() > config.success.vx_max * 1.5
                || state.velocity.y.abs() > config.success.vy_max * 1.5;
            if hard_landing {
                return true;
            }
        }
    }

    // Check out-of-bounds if defined
    if let Some(bounds) = &config.failure.bounds {
        let reference_pos = match bounds.reference {
            Reference::Absolute => Vec2::ZERO,
            Reference::Initial => Vec2::new(config.initial.x0, config.initial.y0),
        };

        let rel_pos = Vec2::new(state.position.x, state.position.y) - reference_pos;
        if rel_pos.x < bounds.x_min
            || rel_pos.x > bounds.x_max
            || rel_pos.y < bounds.y_min
            || rel_pos.y > bounds.y_max
        {
            return true;
        }
    }

    false
}

pub fn reset_simulation(
    state: &mut LanderState,
    level: &CurrentLevel,
    camera_state: &mut CameraState,
) {
    match level.config.dynamics_type {
        DynamicsType::Dynamics2D => simulation_2d::reset_2d(state, level, camera_state),
        DynamicsType::Dynamics3D => simulation_3d::reset_3d(state, level, camera_state),
    }
}

// Helper function to calculate mass flow rate based on thrust
fn calculate_mass_flow(thrust: f32, isp: f32) -> f32 {
    thrust / (isp * 9.81) // 9.81 is standard gravity for Isp calculations
}

// System dispatcher
pub fn simulation_system(
    time: Res<Time>,
    mut state: ResMut<LanderState>,
    level: Res<CurrentLevel>,
    mut script_engine: ResMut<ScriptEngine>,
) {
    match level.config.dynamics_type {
        DynamicsType::Dynamics2D => {
            simulation_2d::update_2d(&time, &mut state, &level, &mut script_engine)
        }
        DynamicsType::Dynamics3D => {
            simulation_3d::update_3d(&time, &mut state, &level, &mut script_engine)
        }
    }
}
