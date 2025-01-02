use bevy::prelude::*;
use rhai::{Dynamic, Engine, Map as RhaiMap, Scope, AST};
use std::sync::Arc;

#[derive(Clone)]
pub struct SimpleControl {
    pub thrust: f32,
}

#[derive(Clone)]
pub struct VectoredControl {
    pub thrust: f32,
    pub gimbal: f32,
}

#[derive(Clone)]
pub enum ControlOutput {
    Simple(SimpleControl),
    Vectored(VectoredControl),
}

#[derive(Clone)]
pub struct LanderState {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub rotation: f32,
    pub angular_vel: f32,
    pub fuel: f32,
}

#[derive(Resource)]
pub struct ScriptEngine {
    engine: Arc<Engine>,
    compiled_script: Option<Arc<AST>>,
    pub error_message: Option<String>,
    pub control_type: ControlType,
    pub user_state: RhaiMap,
    pub console_buffer: Vec<String>,
}

#[derive(Clone)]
pub enum ControlType {
    Simple,
    Vectored,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        let mut engine = Engine::new();

        // Register console function that can handle any type
        let console_fn = move |x: Dynamic| {
            let text = x.to_string();
            // Add text directly to console buffer
            CONSOLE_BUFFER.with(|buffer| {
                buffer.borrow_mut().push(text);
            });
            // Return () to satisfy Rhai
            Dynamic::UNIT
        };
        engine.register_fn("console", console_fn);

        // Disable unsafe operations
        engine.set_max_expr_depths(64, 64);
        engine.set_max_operations(100_000);
        engine.set_max_modules(0);
        engine.set_max_string_size(1_000_000);
        engine.disable_symbol("eval");

        Self {
            engine: Arc::new(engine),
            compiled_script: None,
            error_message: None,
            control_type: ControlType::Simple,
            user_state: RhaiMap::new(),
            console_buffer: Vec::new(),
        }
    }
}

thread_local! {
    static CONSOLE_BUFFER: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
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
            // Clear console buffer for this execution
            CONSOLE_BUFFER.with(|buffer| {
                buffer.borrow_mut().clear();
            });
            self.console_buffer.clear(); // Also clear the engine's buffer

            // Create state map
            let mut map = RhaiMap::new();
            map.insert("x".into(), Dynamic::from_float(state.x as f64));
            map.insert("y".into(), Dynamic::from_float(state.y as f64));
            map.insert("vx".into(), Dynamic::from_float(state.vx as f64));
            map.insert("vy".into(), Dynamic::from_float(state.vy as f64));
            map.insert(
                "rotation".into(),
                Dynamic::from_float(state.rotation as f64),
            );
            map.insert(
                "angular_vel".into(),
                Dynamic::from_float(state.angular_vel as f64),
            );
            map.insert("fuel".into(), Dynamic::from_float(state.fuel as f64));

            // Create scope with state and user_state
            let mut scope = Scope::new();
            scope.push("state", map.clone());
            scope.push_dynamic("user_state", Dynamic::from(self.user_state.clone()));

            // First evaluate script to define functions
            match self.engine.eval_ast_with_scope::<Dynamic>(&mut scope, ast) {
                Ok(_) => {
                    // Now call the control function
                    match self
                        .engine
                        .call_fn::<Dynamic>(&mut scope, ast, "control", (map,))
                    {
                        Ok(result) => {
                            // Get console output and clear thread local buffer
                            CONSOLE_BUFFER.with(|buffer| {
                                let mut buffer = buffer.borrow_mut();
                                self.console_buffer.extend(buffer.drain(..));
                            });

                            // Extract updated user_state
                            if let Some(new_state) = scope.get_value::<RhaiMap>("user_state") {
                                self.user_state = new_state;
                            }

                            // Convert result to control output
                            match self.control_type {
                                ControlType::Simple => match result.as_float() {
                                    Ok(thrust) => Some(ControlOutput::Simple(SimpleControl {
                                        thrust: thrust as f32,
                                    })),
                                    Err(_) => {
                                        self.error_message = Some(
                                            "Control function must return a number (thrust)".into(),
                                        );
                                        None
                                    }
                                },
                                ControlType::Vectored => match result.into_array() {
                                    Ok(array) if array.len() == 2 => {
                                        match (array[0].as_float(), array[1].as_float()) {
                                            (Ok(thrust), Ok(gimbal)) => {
                                                Some(ControlOutput::Vectored(VectoredControl {
                                                    thrust: thrust as f32,
                                                    gimbal: gimbal as f32,
                                                }))
                                            }
                                            _ => {
                                                self.error_message = Some("Control function must return [thrust, gimbal] as numbers".into());
                                                None
                                            }
                                        }
                                    }
                                    _ => {
                                        self.error_message = Some(
                                            "Control function must return [thrust, gimbal]".into(),
                                        );
                                        None
                                    }
                                },
                            }
                        }
                        Err(e) => {
                            let error = format!("Runtime error: {}", e);
                            self.error_message = Some(error);
                            None
                        }
                    }
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

    pub fn take_console_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.console_buffer)
    }
}
