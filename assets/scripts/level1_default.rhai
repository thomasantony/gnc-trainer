// Simple vertical-only control script
let target_velocity = if state["y"] > 10.0 { -5.0 } else { -1.0 };
let error = state["vy"] - target_velocity;

// P controller
let k_p = 10.0;
let thrust = 0.5 - k_p * error;

// Return control struct
simple_control(thrust)
