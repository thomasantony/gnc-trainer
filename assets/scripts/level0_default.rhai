// Example hover script
fn control(state) {
    // Simple vertical-only control script
    let error = state["vy"];

    console("error = " + error + " m/s");
    // P controller
    let k_p = 10.0;
    0.5 - k_p * error
}
