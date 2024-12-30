use bevy::prelude::*;

use crate::{
    rhai_api::{LanderControlState, ScriptEngine},
    ui::EditorState,
};

#[derive(Resource, Default)]
pub struct LanderState {
    pub position: Vec2,    // meters
    pub velocity: Vec2,    // m/s
    pub fuel: f32,         // kg
    pub thrust_level: f32, // 0.0 to 1.0
    pub crashed: bool,
    pub landed: bool,
}

#[derive(Resource)]
pub struct SimulationParams {
    pub dry_mass: f32,              // kg
    pub max_thrust: f32,            // Newtons
    pub isp: f32,                   // seconds
    pub gravity: f32,               // m/s²
    pub initial_altitude: f32,      // meters
    pub initial_fuel: f32,          // kg
    pub safe_landing_velocity: f32, // m/s
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            dry_mass: 300.0,            // kg
            max_thrust: 3.0 * 463.0,    // N (3x 463N vernier engines)
            isp: 326.0,                 // seconds
            gravity: 1.62,              // m/s² (lunar gravity)
            initial_altitude: 50.0,     // meters
            initial_fuel: 70.98,        // kg
            safe_landing_velocity: 2.0, // m/s
        }
    }
}

// Helper function to calculate mass flow rate based on thrust
fn calculate_mass_flow(thrust: f32, isp: f32) -> f32 {
    thrust / (isp * 9.81) // 9.81 is standard gravity for Isp calculations
}

pub fn simulation_system(
    time: Res<Time>,
    mut state: ResMut<LanderState>,
    params: Res<SimulationParams>,
    editor_state: Res<EditorState>,
    mut script_engine: ResMut<ScriptEngine>,
) {
    if !state.landed && !state.crashed && editor_state.is_running {
        // Create control state for script
        let control_state = LanderControlState {
            altitude: state.position.y,
            velocity: state.velocity.y,
            fuel: state.fuel,
        };

        // Get thrust from script
        if let Some(thrust) = script_engine.calculate_thrust(control_state) {
            state.thrust_level = thrust as f32;
        } else {
            // Script error occurred - stop simulation
            state.thrust_level = 0.0;
            return;
        }

        let dt = time.delta_secs();

        // Calculate current mass
        let total_mass = params.dry_mass + state.fuel;

        // Calculate thrust force (vertical only for now)
        let thrust_force = Vec2::new(0.0, state.thrust_level * params.max_thrust);

        // Calculate gravity force
        let gravity_force = Vec2::new(0.0, -params.gravity * total_mass);

        // Sum forces and calculate acceleration
        let total_force = thrust_force + gravity_force;
        let acceleration = total_force / total_mass;

        // Update velocity and position using simple Euler integration
        let velocity = state.velocity;
        state.velocity += acceleration * dt;
        state.position += velocity * dt;

        // Calculate fuel consumption
        let thrust_magnitude = thrust_force.length();
        let fuel_flow = calculate_mass_flow(thrust_magnitude, params.isp);
        state.fuel = (state.fuel - fuel_flow * dt).max(0.0);

        // Ground collision detection (using the base of the triangle, which is 7 units below center)
        use crate::constants::LANDER_BASE_OFFSET;
        if state.position.y <= LANDER_BASE_OFFSET {
            state.position.y = LANDER_BASE_OFFSET; // Keep the base exactly at ground level
                                                   // Check landing conditions
            if state.velocity.y.abs() <= params.safe_landing_velocity {
                state.landed = true;
            } else {
                state.crashed = true;
            }
            state.velocity.y = 0.0;
            // Cut off thrusters on touchdown
            state.thrust_level = 0.0;
        }
    }
}

pub fn reset_simulation(state: &mut LanderState, params: &SimulationParams) {
    *state = LanderState {
        position: Vec2::new(0.0, params.initial_altitude),
        velocity: Vec2::ZERO,
        fuel: params.initial_fuel,
        thrust_level: 0.0,
        crashed: false,
        landed: false,
    };
}
