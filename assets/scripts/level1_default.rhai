// Function to calculate thrust based on current lander state
// Returns a value between 0.0 (no thrust) and 1.0 (full thrust)
//
// Available state variables:
//   state["altitude"]  -> altitude in meters
//   state["velocity"]  -> vertical velocity in m/s (positive is upward)
//   state["fuel"]      -> remaining fuel in kg

// Simple example: more thrust when going down, less when going up
let target_velocity = if state["altitude"] > 10.0 { -5.0 } else { -1.0 };
let error = state["velocity"] - target_velocity;

// P controller
let k_p = 10.0;
let thrust = 0.5 - k_p * error;

// Ensure we don't waste fuel if we're going up too fast
if state["velocity"] > 2.0 {
    0.0
} else {
    thrust
}
