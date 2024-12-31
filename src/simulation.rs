use bevy::prelude::*;

use crate::{
    constants::LANDER_BASE_OFFSET,
    levels::CurrentLevel,
    rhai_api::{ControlOutput, LanderState as ScriptLanderState, ScriptEngine},
};

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
}

// Constants for rotational dynamics
const MOMENT_OF_INERTIA: f32 = 100.0; // kg·m²
const MAX_GIMBAL_ANGLE: f32 = 0.4; // radians (~23 degrees)
const ANGULAR_DAMPING: f32 = 0.2; // artificial damping coefficient

pub fn simulation_system(
    time: Res<Time>,
    mut state: ResMut<LanderState>,
    level: Res<CurrentLevel>,
    mut script_engine: ResMut<ScriptEngine>,
) {
    if !state.landed && !state.crashed {
        // Create control state for script
        let script_state = ScriptLanderState {
            x: state.position.x,
            y: state.position.y,
            vx: state.velocity.x,
            vy: state.velocity.y,
            rotation: state.rotation,
            fuel: state.fuel,
        };

        // Get control output from script
        if let Some(control) = script_engine.calculate_control(script_state) {
            match control {
                ControlOutput::Simple(simple) => {
                    state.thrust_level = simple.thrust.clamp(0.0, 1.0);
                    state.gimbal_angle = 0.0;
                }
                ControlOutput::Vectored(vectored) => {
                    state.thrust_level = vectored.thrust.clamp(0.0, 1.0);
                    state.gimbal_angle = vectored.gimbal.clamp(-MAX_GIMBAL_ANGLE, MAX_GIMBAL_ANGLE);
                }
            }
        } else {
            // Script error occurred - stop thrusting
            state.thrust_level = 0.0;
            state.gimbal_angle = 0.0;
            return;
        }

        let dt = time.delta_secs();
        let config = &level.config;

        // Calculate current mass
        let total_mass = config.physics.m + state.fuel;

        // Calculate thrust vector based on rotation and gimbal
        let thrust_angle = state.rotation + state.gimbal_angle;
        let thrust_force = Vec2::new(
            thrust_angle.sin() * state.thrust_level * config.physics.t,
            thrust_angle.cos() * state.thrust_level * config.physics.t,
        );

        // Calculate gravity force (y-axis only)
        let gravity_force = Vec2::new(0.0, config.physics.g * total_mass);

        // Sum forces and calculate linear acceleration
        let total_force = thrust_force + gravity_force;
        let acceleration = total_force / total_mass;

        // Calculate torque from offset thrust
        let thrust_torque = if state.thrust_level > 0.0 {
            -state.gimbal_angle * state.thrust_level * config.physics.t * LANDER_BASE_OFFSET
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

        // Calculate fuel consumption
        let thrust_magnitude = thrust_force.length();
        let fuel_flow = calculate_mass_flow(thrust_magnitude, config.physics.isp);
        state.fuel = (state.fuel - fuel_flow * dt).max(0.0);

        // Ground collision detection
        if state.position.y <= LANDER_BASE_OFFSET {
            state.position.y = LANDER_BASE_OFFSET;

            // Check landing conditions
            let landing_speed_ok = state.velocity.y.abs() <= config.success.vy_max
                && state.velocity.x.abs() <= config.success.vx_max;
            let position_ok = state.position.x >= config.success.x_min
                && state.position.x <= config.success.x_max;
            let rotation_ok = state.rotation.abs() <= 0.1; // Allow slight tilt

            if landing_speed_ok && position_ok && rotation_ok {
                state.landed = true;
            } else {
                state.crashed = true;
            }

            state.velocity = Vec2::ZERO;
            state.angular_vel = 0.0;
            state.thrust_level = 0.0;
            state.gimbal_angle = 0.0;
        }
    }
}

pub fn reset_simulation(state: &mut LanderState, level: &CurrentLevel) {
    *state = LanderState {
        position: Vec2::new(level.config.initial.x0, level.config.initial.y0),
        velocity: Vec2::new(level.config.initial.vx0, level.config.initial.vy0),
        rotation: level.config.initial.r0,
        angular_vel: 0.0,
        fuel: level.config.initial.f0,
        thrust_level: 0.0,
        gimbal_angle: 0.0,
        crashed: false,
        landed: false,
    };
}

// Helper function to calculate mass flow rate based on thrust
fn calculate_mass_flow(thrust: f32, isp: f32) -> f32 {
    thrust / (isp * 9.81) // 9.81 is standard gravity for Isp calculations
}
