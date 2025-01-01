fn control(state) {
    // Simple vertical-only control script
    let target_velocity = if state["y"] > 10.0 { -5.0 } else { -1.0 };
    let error = state["vy"] - target_velocity;

    // P controller
    console("Target velocity: " + target_velocity);
    console("Current velocity: " + state["vy"]);
    console("Error: " + error);

    let k_p = 10.0;
    0.5 - k_p * error
}
