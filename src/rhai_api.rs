use bevy::prelude::*;
use rhai::{Dynamic, Engine, Map as RhaiMap, Scope, AST};
use std::sync::Arc;

#[derive(Resource)]
pub struct ScriptEngine {
    engine: Arc<Engine>,
    compiled_script: Option<Arc<AST>>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub struct LanderControlState {
    pub altitude: f32, // meters
    pub velocity: f32, // m/s
    pub fuel: f32,     // kg
}

impl Default for ScriptEngine {
    fn default() -> Self {
        let mut engine = Engine::new();

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
        }
    }
}

impl ScriptEngine {
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

    pub fn calculate_thrust(&mut self, state: LanderControlState) -> Option<f64> {
        if let Some(ast) = &self.compiled_script {
            // Create a map to hold our state values
            let mut map = RhaiMap::new();
            map.insert(
                "altitude".into(),
                Dynamic::from_float(state.altitude as f64),
            );
            map.insert(
                "velocity".into(),
                Dynamic::from_float(state.velocity as f64),
            );
            map.insert("fuel".into(), Dynamic::from_float(state.fuel as f64));

            // Create a new scope for this execution
            let mut scope = Scope::new();
            scope.push("state", map);

            match self.engine.eval_ast_with_scope::<f64>(&mut scope, ast) {
                Ok(thrust) => {
                    // Clamp thrust between 0 and 1
                    let thrust = thrust.clamp(0.0, 1.0);
                    self.error_message = None;
                    Some(thrust)
                }
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
