use bevy::prelude::*;

use crate::{
    constants::LANDER_BASE_OFFSET,
    levels::{CurrentLevel, Reference},
    rhai_api::{ControlOutput, LanderState as ScriptLanderState, ScriptEngine},
    visualization::CameraState,
};

use super::{calculate_mass_flow, check_failure_conditions, check_success_conditions, LanderState};

// Control limits
const MAX_GIMBAL_ANGLE: f32 = 0.4; // radians (~23 degrees)
const MIN_GIMBAL_ANGLE: f32 = -0.4; // radians
const MAX_THRUST: f32 = 1.0;
const MIN_THRUST: f32 = 0.0;
const MAX_THRUST_CHANGE_RATE: f32 = 2.0; // Maximum thrust change per second
const MAX_GIMBAL_RATE: f32 = 1.0; // Maximum gimbal angle change per second

// Constants for rotational dynamics
const MOMENT_OF_INERTIA: f32 = 100.0; // kg·m²
const ANGULAR_DAMPING: f32 = 0.0; // artificial damping coefficient

pub fn update_2d(
    time: &Res<Time>,
    state: &mut ResMut<LanderState>,
    level: &Res<CurrentLevel>,
    script_engine: &mut ResMut<ScriptEngine>,
) {
    // Only run simulation if we have a level config
    if !state.landed && !state.crashed {
        let dt = time.delta_secs();

        // Create control state for script
        let script_state = ScriptLanderState {
            x: state.position.x,
            y: state.position.y,
            vx: state.velocity.x,
            vy: state.velocity.y,
            rotation: state.rotation.to_euler(EulerRot::XYZ).2,
            angular_vel: state.angular_vel.z,
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
        let thrust_direction = -state.rotation.to_euler(EulerRot::XYZ).2 - state.gimbal_angle;

        let thrust_force = Vec3::new(
            thrust_direction.sin() * state.thrust_level * config.physics.max_thrust,
            thrust_direction.cos() * state.thrust_level * config.physics.max_thrust,
            0.0,
        );

        // Calculate gravity force (y-axis only)
        let gravity_force = Vec3::new(0.0, config.physics.gravity * total_mass, 0.0);

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
        let damping_torque = -state.angular_vel.z * ANGULAR_DAMPING;
        let total_torque = thrust_torque + damping_torque;

        // Update angular velocity and rotation
        let angular_acc = total_torque / MOMENT_OF_INERTIA;
        state.angular_vel.z += angular_acc * dt;

        // Convert 2D rotation to quaternion
        let new_angle = state.rotation.to_euler(EulerRot::XYZ).2 + state.angular_vel.z * dt;
        state.rotation = Quat::from_rotation_z(new_angle);

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
                state.velocity = Vec3::ZERO;
                state.angular_vel = Vec3::ZERO;
                state.thrust_level = 0.0;
                state.gimbal_angle = 0.0;
                return;
            }

            // Not a crash, normal ground contact
            state.position.y = LANDER_BASE_OFFSET;
            state.velocity = Vec3::ZERO;
            state.angular_vel = Vec3::ZERO;
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

pub fn reset_2d(state: &mut LanderState, level: &CurrentLevel, camera_state: &mut CameraState) {
    *state = LanderState {
        position: Vec3::new(level.config.initial.x0, level.config.initial.y0, 0.0),
        velocity: Vec3::new(level.config.initial.vx0, level.config.initial.vy0, 0.0),
        rotation: Quat::from_rotation_z(level.config.initial.initial_angle),
        angular_vel: Vec3::new(0.0, 0.0, 0.0),
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
