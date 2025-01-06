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
    
    // Something to start with:
    // let target_angle = 0.0;  // First try to get upright
    // let angle_error = normalize_angle(target_angle - ang);
    // let kp = 1.0;  // tune this value
    // let kd = 1.0;  // tune this value
    // Check the signs on your gain ... maybe you are going the wrong way?
    // let gimbal = kp * angle_error + kd * omega; // also try to minimize angular velocity
    // let thrust = 0.5;

    // Yoou can also limit the controls to a certain range if you want
    // let thrust = max(0.0, min(1.0, thrust));
    // let gimbal = max(-0.1, min(0.1, gimbal));

    // Until you do all that ...
    // DONUT MODE!!
    let gimbal = 0.1;
    let thrust = 0.5;
    
    [thrust, gimbal]
}
