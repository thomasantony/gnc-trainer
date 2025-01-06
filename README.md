# GNC Trainer

A spacecraft landing simulator for learning Guidance and Control systems. Built with Bevy 0.15.

## Features

- Progressive levels teaching different control schemes
- Real-time script editor with syntax highlighting
- Physics simulation with thrust vectoring and fuel consumption
- Persistent progress tracking
- WebAssembly support

## Core Mechanics

- Vertical-only and thrust vectoring control modes
- Dynamic success criteria including:
  - Position constraints
  - Velocity limits
  - Attitude requirements
  - Hover capabilities
- Real-time telemetry display
- Particle effects for engine exhaust and crashes

## Getting Started

```bash
# Run locally
cargo run --release

# Build for web (using trunk - https://trunkrs.dev)
trunk serve --release
```

## Writing Control Scripts

Scripts are written in [RHAI](https://rhai.rs) and have access to:

```javascript
// State variables
state['x'] // Horizontal position (m)
state['y'] // Vertical position (m)
state['vx'] // Horizontal velocity (m/s)
state['vy'] // Vertical velocity (m/s)
state['rotation'] // Attitude angle (rad)
state['angular_vel'] // Angular velocity (rad/s)
state['fuel'] // Remaining fuel mass (kg)

// Helper functions
console(value) // Debug output

// Return format depends on control mode:
return 0.5 // Vertical-only: thrust 0.0-1.0
return [0.5, 0.1] // Thrust vectoring: [thrust, gimbal_angle]
```

## Dependencies

- Bevy 0.15
- egui + egui_code_editor for UI
- [RHAI](https://rhai.rs) for scripting
- bevy_persistent for save data

## License

MIT
