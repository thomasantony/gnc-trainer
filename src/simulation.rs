use bevy::prelude::*;

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
            initial_altitude: 100.0,    // meters
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
) {
    if !state.landed && !state.crashed {
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

        // Ground collision detection
        if state.position.y <= 0.0 {
            state.position.y = 0.0;
            // Check landing conditions
            if state.velocity.y.abs() <= params.safe_landing_velocity {
                state.landed = true;
            } else {
                state.crashed = true;
            }
            state.velocity.y = 0.0;
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
