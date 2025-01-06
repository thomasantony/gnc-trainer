fn control(state) {
    // Constant 50% throttle
    console("y = " + state["y"] + " m");
    console("vy = " + state["vy"] + " m/s");
    
    // let target_vy = ??;  // Maybe this should vary by altitude?
    // let kp = 1.0;  // tune this value
    // let thrust = kp * (target_vy - state["vy"]);
    let thrust = 0.5;
    thrust
}
