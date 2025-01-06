fn control(state) {
    // Constant 50% throttle
    console("y = " + state["y"] + " m");
    console("vy = " + state["vy"] + " m/s");
    
    // Hint: Use a Proportional controller to adjust the thrust based on state["vy"]
    // E.g. 
    // let kp = 1.0;  // tune this value
	// thrust = kp * (target_vy - state["vy"]);
    // Can you further improve this to also reposition the spacecraft if it drifts too far from the starting position?
    let thrust = 0.5;
    thrust
}
