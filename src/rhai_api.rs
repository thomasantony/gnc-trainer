use bevy::prelude::*;
use rhai::{Dynamic, Engine, Map as RhaiMap, Scope, AST};
use std::sync::Arc;

#[derive(Clone)]
pub struct SimpleControl {
    pub thrust: f32, // 0.0 to 1.0
}

#[derive(Clone)]
pub struct VectoredControl {
    pub thrust: f32, // 0.0 to 1.0
    pub gimbal: f32, // gimbal angle in radians
}

#[derive(Clone)]
pub enum ControlOutput {
    Simple(SimpleControl),
    Vectored(VectoredControl),
}

#[derive(Clone)]
pub struct LanderState {
    pub x: f32,        // horizontal position
    pub y: f32,        // altitude
    pub vx: f32,       // horizontal velocity
    pub vy: f32,       // vertical velocity
    pub rotation: f32, // current rotation in radians
    pub fuel: f32,     // remaining fuel
}

#[derive(Resource)]
pub struct ScriptEngine {
    engine: Arc<Engine>,
    compiled_script: Option<Arc<AST>>,
    pub error_message: Option<String>,
    pub control_type: ControlType,
}

#[derive(Clone)]
pub enum ControlType {
    Simple,
    Vectored,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        let mut engine = Engine::new();

        // Register the control output types
        engine.register_type::<SimpleControl>();
        engine.register_type::<VectoredControl>();

        engine.register_fn("simple_control", |thrust: f64| -> Dynamic {
            let control = SimpleControl {
                thrust: thrust as f32,
            };
            Dynamic::from(control)
        });

        engine.register_fn("vector_control", |thrust: f64, gimbal: f64| -> Dynamic {
            let control = VectoredControl {
                thrust: thrust as f32,
                gimbal: gimbal as f32,
            };
            Dynamic::from(control)
        });

        // Disable operations we don't want to allow
        engine.set_max_expr_depths(64, 64);
        engine.set_max_operations(100_000);
        engine.set_max_modules(0);
        engine.set_max_string_size(0);
        engine.disable_symbol("eval");

        Self {
            engine: Arc::new(engine),
            compiled_script: None,
            error_message: None,
            control_type: ControlType::Simple,
        }
    }
}

impl ScriptEngine {
    pub fn set_control_type(&mut self, control_type: ControlType) {
        self.control_type = control_type;
    }

    pub fn compile_script(&mut self, script: &str) -> Result<(), String> {
        self.error_message = None;
        match self.engine.compile(script) {
            Ok(ast) => {
                self.compiled_script = Some(Arc::new(ast));
                Ok(())
            }
            Err(e) => {
                let error = format!("Compilation error: {}", e);
                self.error_message = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn calculate_control(&mut self, state: LanderState) -> Option<ControlOutput> {
        if let Some(ast) = &self.compiled_script {
            // Create a map to hold our state values
            let mut map = RhaiMap::new();
            map.insert("x".into(), Dynamic::from_float(state.x as f64));
            map.insert("y".into(), Dynamic::from_float(state.y as f64));
            map.insert("vx".into(), Dynamic::from_float(state.vx as f64));
            map.insert("vy".into(), Dynamic::from_float(state.vy as f64));
            map.insert(
                "rotation".into(),
                Dynamic::from_float(state.rotation as f64),
            );
            map.insert("fuel".into(), Dynamic::from_float(state.fuel as f64));

            // Create a new scope for this execution
            let mut scope = Scope::new();
            scope.push("state", map);

            match self.engine.eval_ast_with_scope::<Dynamic>(&mut scope, ast) {
                Ok(result) => match self.control_type {
                    ControlType::Simple => {
                        let control: SimpleControl = result.cast();
                        Some(ControlOutput::Simple(control))
                    }
                    ControlType::Vectored => {
                        let control: VectoredControl = result.cast();
                        Some(ControlOutput::Vectored(control))
                    }
                },
                Err(e) => {
                    let error = format!("Runtime error: {}", e);
                    self.error_message = Some(error);
                    None
                }
            }
        } else {
            None
        }
    }
}
