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
    
    // DONUT MODE!!
    let gimbal = 0.1;
    let thrust = 0.5;
    
    [thrust, gimbal]
}
