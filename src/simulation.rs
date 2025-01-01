use bevy::prelude::*;

use crate::{
    constants::LANDER_BASE_OFFSET,
    levels::{CurrentLevel, Reference},
    rhai_api::{ControlOutput, LanderState as ScriptLanderState, ScriptEngine},
    visualization::CameraState,
};

// Control limits
const MAX_GIMBAL_ANGLE: f32 = 0.4; // radians (~23 degrees)
const MIN_GIMBAL_ANGLE: f32 = -0.4; // radians
const MAX_THRUST: f32 = 1.0;
const MIN_THRUST: f32 = 0.0;
const MAX_THRUST_CHANGE_RATE: f32 = 2.0; // Maximum thrust change per second
const MAX_GIMBAL_RATE: f32 = 1.0; // Maximum gimbal angle change per second

#[derive(Resource, Default)]
pub struct LanderState {
    pub position: Vec2,    // (x, y) position in meters
    pub velocity: Vec2,    // (vx, vy) velocity in m/s
    pub rotation: f32,     // rotation in radians
    pub angular_vel: f32,  // angular velocity in rad/s
    pub fuel: f32,         // kg
    pub thrust_level: f32, // 0.0 to 1.0
    pub gimbal_angle: f32, // radians
    pub crashed: bool,
    pub landed: bool,
    pub success_timer: f32, // Time spent meeting success criteria
    pub stabilizing: bool,  // True when meeting conditions but not yet complete
}

// Constants for rotational dynamics
const MOMENT_OF_INERTIA: f32 = 100.0; // kg·m²
const ANGULAR_DAMPING: f32 = 0.0; // artificial damping coefficient

fn check_success_conditions(state: &LanderState, level: &CurrentLevel) -> bool {
    let config = &level.config;

    // Check velocity constraints
    let speed_ok = state.velocity.x.abs() <= config.success.vx_max
        && state.velocity.y.abs() <= config.success.vy_max;

    // Check angle constraints
    let angle_ok =
        (state.rotation - config.success.final_angle).abs() <= config.success.angle_tolerance;

    // Check position constraints
    let position_ok = match config.success.position_box.reference {
        Reference::Initial => {
            // For initial-reference boxes (like hover), always check position
            let initial_pos = Vec2::new(config.initial.x0, config.initial.y0);
            let rel_pos = state.position - initial_pos;
            rel_pos.x >= config.success.position_box.x_min
                && rel_pos.x <= config.success.position_box.x_max
                && rel_pos.y >= config.success.position_box.y_min
                && rel_pos.y <= config.success.position_box.y_max
        }
        Reference::Absolute => {
            if state.position.y <= LANDER_BASE_OFFSET + 0.1 {
                // Only check absolute position constraints when on/near ground
                state.position.x >= config.success.position_box.x_min
                    && state.position.x <= config.success.position_box.x_max
                    && state.position.y >= config.success.position_box.y_min
                    && state.position.y <= config.success.position_box.y_max
            } else {
                // When in air, only check speed and angle
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

        let rel_pos = state.position - reference_pos;
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

pub fn simulation_system(
    time: Res<Time>,
    mut state: ResMut<LanderState>,
    level: Res<CurrentLevel>,
    mut script_engine: ResMut<ScriptEngine>,
) {
    if !state.landed && !state.crashed {
        let dt = time.delta_secs();

        // Create control state for script
        let script_state = ScriptLanderState {
            x: state.position.x,
            y: state.position.y,
            vx: state.velocity.x,
            vy: state.velocity.y,
            rotation: state.rotation,
            angular_vel: state.angular_vel,
            fuel: state.fuel,
        };

        // Get thrust and gimbal commands from script
        let mut new_thrust;
        let mut new_gimbal;

        if let Some(control) = script_engine.calculate_control(script_state) {
            match control {
                ControlOutput::Simple(simple) => {
                    new_thrust = simple.thrust;
                    new_gimbal = 0.0;
                }
                ControlOutput::Vectored(vectored) => {
                    new_thrust = vectored.thrust;
                    new_gimbal = vectored.gimbal;
                }
            }
        } else {
            // Script error occurred - maintain current values
            return;
        }

        // Apply rate limits and clamps to controls
        new_thrust = new_thrust.clamp(MIN_THRUST, MAX_THRUST);
        new_gimbal = new_gimbal.clamp(MIN_GIMBAL_ANGLE, MAX_GIMBAL_ANGLE);

        // Rate limit the thrust changes
        let max_thrust_delta = MAX_THRUST_CHANGE_RATE * dt;
        new_thrust = if new_thrust > state.thrust_level {
            (state.thrust_level + max_thrust_delta).min(new_thrust)
        } else {
            (state.thrust_level - max_thrust_delta).max(new_thrust)
        };

        // Rate limit the gimbal changes
        let max_gimbal_delta = MAX_GIMBAL_RATE * dt;
        new_gimbal = if new_gimbal > state.gimbal_angle {
            (state.gimbal_angle + max_gimbal_delta).min(new_gimbal)
        } else {
            (state.gimbal_angle - max_gimbal_delta).max(new_gimbal)
        };

        // Update control state
        state.thrust_level = new_thrust;
        state.gimbal_angle = new_gimbal;

        // Force thrust to 0 if out of fuel
        if state.fuel <= 0.0 {
            state.thrust_level = 0.0;
            state.gimbal_angle = 0.0;
        }

        let config = &level.config;

        // Calculate current mass
        let total_mass = config.physics.dry_mass + state.fuel;

        // When rotation is 0 (pointing up):
        //   - thrust should be upward
        //   - gimbal rotates this direction
        let thrust_direction = -state.rotation - state.gimbal_angle;

        let thrust_force = Vec2::new(
            thrust_direction.sin() * state.thrust_level * config.physics.max_thrust,
            thrust_direction.cos() * state.thrust_level * config.physics.max_thrust,
        );

        // Calculate gravity force (y-axis only)
        let gravity_force = Vec2::new(0.0, config.physics.gravity * total_mass);

        // Sum forces and calculate linear acceleration
        let total_force = thrust_force + gravity_force;
        let acceleration = total_force / total_mass;

        // Calculate torque from offset thrust
        let thrust_torque = if state.thrust_level > 0.0 {
            -state.gimbal_angle.sin()
                * state.thrust_level
                * config.physics.max_thrust
                * LANDER_BASE_OFFSET
        } else {
            0.0
        };

        // Add artificial angular damping
        let damping_torque = -state.angular_vel * ANGULAR_DAMPING;
        let total_torque = thrust_torque + damping_torque;

        // Update angular velocity and rotation
        let angular_acc = total_torque / MOMENT_OF_INERTIA;
        state.angular_vel += angular_acc * dt;
        state.rotation += state.angular_vel * dt;

        // Update linear velocity and position using simple Euler integration
        let velocity = state.velocity;
        state.velocity += acceleration * dt;
        state.position += velocity * dt;

        // Ground collision check - check failure first
        if state.position.y <= LANDER_BASE_OFFSET {
            // Check for crash before zeroing velocity
            if check_failure_conditions(&state, &level) {
                state.crashed = true;
                state.position.y = LANDER_BASE_OFFSET;
                state.velocity = Vec2::ZERO;
                state.angular_vel = 0.0;
                state.thrust_level = 0.0;
                state.gimbal_angle = 0.0;
                return;
            }

            // Not a crash, normal ground contact
            state.position.y = LANDER_BASE_OFFSET;
            state.velocity = Vec2::ZERO;
            state.angular_vel = 0.0;
            state.thrust_level = 0.0;
            state.gimbal_angle = 0.0;
        }

        // Calculate fuel consumption
        let thrust_magnitude = thrust_force.length();
        let fuel_flow = calculate_mass_flow(thrust_magnitude, config.physics.isp);
        state.fuel = (state.fuel - fuel_flow * dt).max(0.0);

        // Check success/failure conditions
        if check_failure_conditions(&state, &level) {
            state.crashed = true;
            return;
        }

        // Check for success conditions
        if check_success_conditions(&state, &level) {
            state.success_timer += dt;
            state.stabilizing = true;

            // Check if we've met the persistence requirement
            if state.success_timer >= config.success.persistence_period {
                state.landed = true;
                state.stabilizing = false;
            }
        } else {
            // Reset the timer if any condition is not met
            state.success_timer = 0.0;
            state.stabilizing = false;
        }
    }
}

pub fn reset_simulation(
    state: &mut LanderState,
    level: &CurrentLevel,
    camera_state: &mut CameraState,
) {
    *state = LanderState {
        position: Vec2::new(level.config.initial.x0, level.config.initial.y0),
        velocity: Vec2::new(level.config.initial.vx0, level.config.initial.vy0),
        rotation: level.config.initial.initial_angle,
        angular_vel: 0.0,
        fuel: level.config.initial.initial_fuel,
        thrust_level: 0.0,
        gimbal_angle: 0.0,
        crashed: false,
        landed: false,
        success_timer: 0.0,
        stabilizing: false,
    };

    // Reset camera to following state
    camera_state.following = true;
    camera_state.target_offset.x = 0.0;
    camera_state.target_offset.y = 0.0;
    camera_state.explosion_spawned = false;
}

// Helper function to calculate mass flow rate based on thrust
fn calculate_mass_flow(thrust: f32, isp: f32) -> f32 {
    thrust / (isp * 9.81) // 9.81 is standard gravity for Isp calculations
}
