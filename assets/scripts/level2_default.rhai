fn normalize_angle(angle)
{
    return atan(sin(angle), cos(angle));
}

fn control(state) {
    let x = state["x"];
    let y = state["y"];
    let vx = state["vx"];
    let vy = state["vy"];
    let ang = normalize_angle(state["rotation"]);
    let omega = state["angular_vel"];
    
    // Constants for the controller
    let GRAVITY = 1.62;      // Lunar gravity (m/s^2)
    let MAX_DESCENT = 4.0;   // Maximum descent rate (m/s)
    let MIN_DESCENT = 1.0;   // Minimum descent rate near ground (m/s)
    let GAIN_P = 10.0;        // Proportional gain for velocity control
    let GAIN_D = 10.0;        // Derivative gain for angular velocity
	let GAIN_P_GIMBAL = 8.0;
    
    // Calculate desired vertical velocity based on height
    // Linear relationship between height and descent rate
    let descent_rate = if y > 30.0 {
        MAX_DESCENT
    } else {
        MIN_DESCENT + (MAX_DESCENT - MIN_DESCENT) * (y / 20.0)
    };
    
    // Thrust control based on vertical velocity error
    let vel_error = -vy - descent_rate;  // Convert vy to positive descent rate
    let thrust = 0.5 + GAIN_P * vel_error / GRAVITY;
    
    // Gravity turn: point thrust vector opposite to velocity vector
    let desired_angle = atan(vx, -vy);  // Target angle for velocity vector
    desired_angle = normalize_angle(desired_angle);
    let angle_error = desired_angle - ang;
    
    // Normalize angle error to [-π, π]
    let angle_error = normalize_angle(angle_error);
    // Gimbal control with PD controller
    let gimbal = -GAIN_P_GIMBAL * angle_error + GAIN_D * omega;
    
    // Clamp controls to allowable ranges
    let thrust = max(0.0, min(1.0, thrust));
    let gimbal = max(-0.1, min(0.1, gimbal));
    
    [thrust, gimbal]
}
