// Vectored thrust control script
// First calculate desired horizontal velocity based on position
let target_x = 0.0;  // target landing position
let x_error = state["x"] - target_x;
let desired_vx = -0.3 * x_error;  // P controller for horizontal position

// Calculate velocity errors
let vx_error = state["vx"] - desired_vx;
let vy_error = state["vy"] - if state["y"] > 10.0 { -5.0 } else { -1.0 };

// P controllers for thrust and gimbal
let kp_y = 10.0;
// let thrust = 0.5 - kp_y * vy_error;
let thrust = 1.0;

let kp_x = 5.0;
let kd_x = 1.0;
// let gimbal = -kp_x * vx_error - kd_x * state["rotation"];  // Also try to stay vertical
let gimbal = 0.0;

// Return control struct
vector_control(thrust, gimbal)
